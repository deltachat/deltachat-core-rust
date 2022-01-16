//! # SQLite wrapper.

use async_std::path::Path;
use async_std::sync::RwLock;

use std::collections::HashSet;
use std::convert::TryFrom;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use async_std::path::PathBuf;
use async_std::prelude::*;
use rusqlite::{config::DbConfig, Connection, OpenFlags};

use crate::blob::BlobObject;
use crate::chat::{add_device_msg, update_device_icon, update_saved_messages_icon};
use crate::config::Config;
use crate::constants::{Viewtype, DC_CHAT_ID_TRASH};
use crate::context::Context;
use crate::dc_tools::{dc_delete_file, time};
use crate::ephemeral::start_ephemeral_timers;
use crate::message::Message;
use crate::param::{Param, Params};
use crate::peerstate::{deduplicate_peerstates, Peerstate};
use crate::stock_str;

#[macro_export]
macro_rules! paramsv {
    () => {
        rusqlite::params_from_iter(Vec::<&dyn $crate::ToSql>::new())
    };
    ($($param:expr),+ $(,)?) => {
        rusqlite::params_from_iter(vec![$(&$param as &dyn $crate::ToSql),+])
    };
}

mod migrations;

/// A wrapper around the underlying Sqlite3 object.
#[derive(Debug)]
pub struct Sql {
    /// Database file path
    pub(crate) dbfile: PathBuf,

    pool: RwLock<Option<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>,
}

impl Sql {
    pub fn new(dbfile: PathBuf) -> Sql {
        Self {
            dbfile,
            pool: Default::default(),
        }
    }

    /// Tests SQLCipher passphrase.
    ///
    /// Returns true if passphrase is correct, i.e. the database is new or can be unlocked with
    /// this passphrase, and false if the database is already encrypted with another passphrase or
    /// corrupted.
    ///
    /// Fails if database is already open.
    pub async fn check_passphrase(&self, passphrase: String) -> Result<bool> {
        if self.is_open().await {
            bail!("Database is already opened.");
        }

        // Hold the lock to prevent other thread from opening the database.
        let _lock = self.pool.write().await;

        // Test that the key is correct using a single connection.
        let connection = Connection::open(&self.dbfile)?;
        connection
            .pragma_update(None, "key", &passphrase)
            .context("failed to set PRAGMA key")?;
        let key_is_correct = connection
            .query_row("SELECT count(*) FROM sqlite_master", [], |_row| Ok(()))
            .is_ok();

        Ok(key_is_correct)
    }

    /// Checks if there is currently a connection to the underlying Sqlite database.
    pub async fn is_open(&self) -> bool {
        self.pool.read().await.is_some()
    }

    /// Closes all underlying Sqlite connections.
    async fn close(&self) {
        let _ = self.pool.write().await.take();
        // drop closes the connection
    }

    /// Exports the database to a separate file with the given passphrase.
    ///
    /// Set passphrase to empty string to export the database unencrypted.
    pub(crate) async fn export(&self, path: &Path, passphrase: String) -> Result<()> {
        let path_str = path
            .to_str()
            .with_context(|| format!("path {:?} is not valid unicode", path))?;
        let conn = self.get_conn().await?;
        conn.execute(
            "ATTACH DATABASE ? AS backup KEY ?",
            paramsv![path_str, passphrase],
        )
        .context("failed to attach backup database")?;
        let res = conn
            .query_row("SELECT sqlcipher_export('backup')", [], |_row| Ok(()))
            .context("failed to export to attached backup database");
        conn.execute("DETACH DATABASE backup", [])
            .context("failed to detach backup database")?;
        res?;
        Ok(())
    }

    /// Imports the database from a separate file with the given passphrase.
    pub(crate) async fn import(&self, path: &Path, passphrase: String) -> Result<()> {
        let path_str = path
            .to_str()
            .with_context(|| format!("path {:?} is not valid unicode", path))?;
        let conn = self.get_conn().await?;

        // Reset the database without reopening it. We don't want to reopen the database because we
        // don't have main database passphrase at this point.
        // See <https://sqlite.org/c3ref/c_dbconfig_enable_fkey.html> for documentation.
        // Without resetting import may fail due to existing tables.
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)
            .context("failed to set SQLITE_DBCONFIG_RESET_DATABASE")?;
        conn.execute("VACUUM", [])
            .context("failed to vacuum the database")?;
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)
            .context("failed to unset SQLITE_DBCONFIG_RESET_DATABASE")?;

        conn.execute(
            "ATTACH DATABASE ? AS backup KEY ?",
            paramsv![path_str, passphrase],
        )
        .context("failed to attach backup database")?;
        let res = conn
            .query_row("SELECT sqlcipher_export('main', 'backup')", [], |_row| {
                Ok(())
            })
            .context("failed to import from attached backup database");
        conn.execute("DETACH DATABASE backup", [])
            .context("failed to detach backup database")?;
        res?;
        Ok(())
    }

    fn new_pool(
        dbfile: &Path,
        passphrase: String,
    ) -> Result<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>> {
        let mut open_flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;
        open_flags.insert(OpenFlags::SQLITE_OPEN_READ_WRITE);
        open_flags.insert(OpenFlags::SQLITE_OPEN_CREATE);

        // this actually creates min_idle database handles just now.
        // therefore, with_init() must not try to modify the database as otherwise
        // we easily get busy-errors (eg. table-creation, journal_mode etc. should be done on only one handle)
        let mgr = r2d2_sqlite::SqliteConnectionManager::file(dbfile)
            .with_flags(open_flags)
            .with_init(move |c| {
                c.execute_batch(&format!(
                    "PRAGMA cipher_memory_security = OFF; -- Too slow on Android
                     PRAGMA secure_delete=on;
                     PRAGMA busy_timeout = {};
                     PRAGMA temp_store=memory; -- Avoid SQLITE_IOERR_GETTEMPPATH errors on Android
                     ",
                    Duration::from_secs(10).as_millis()
                ))?;
                c.pragma_update(None, "key", passphrase.clone())?;
                Ok(())
            });

        let pool = r2d2::Pool::builder()
            .min_idle(Some(2))
            .max_size(10)
            .connection_timeout(Duration::from_secs(60))
            .build(mgr)
            .context("Can't build SQL connection pool")?;
        Ok(pool)
    }

    async fn try_open(&self, context: &Context, dbfile: &Path, passphrase: String) -> Result<()> {
        *self.pool.write().await = Some(Self::new_pool(dbfile, passphrase.to_string())?);

        {
            let conn = self.get_conn().await?;

            // Try to enable auto_vacuum. This will only be
            // applied if the database is new or after successful
            // VACUUM, which usually happens before backup export.
            // When auto_vacuum is INCREMENTAL, it is possible to
            // use PRAGMA incremental_vacuum to return unused
            // database pages to the filesystem.
            conn.pragma_update(None, "auto_vacuum", &"INCREMENTAL".to_string())?;

            // journal_mode is persisted, it is sufficient to change it only for one handle.
            conn.pragma_update(None, "journal_mode", &"WAL".to_string())?;

            // Default synchronous=FULL is much slower. NORMAL is sufficient for WAL mode.
            conn.pragma_update(None, "synchronous", &"NORMAL".to_string())?;
        }

        // (1) update low-level database structure.
        // this should be done before updates that use high-level objects that
        // rely themselves on the low-level structure.

        let (recalc_fingerprints, update_icons, disable_server_delete, recode_avatar) =
            migrations::run(context, self)
                .await
                .context("failed to run migrations")?;

        // (2) updates that require high-level objects
        // the structure is complete now and all objects are usable

        if recalc_fingerprints {
            info!(context, "[migration] recalc fingerprints");
            let addrs = self
                .query_map(
                    "SELECT addr FROM acpeerstates;",
                    paramsv![],
                    |row| row.get::<_, String>(0),
                    |addrs| {
                        addrs
                            .collect::<std::result::Result<Vec<_>, _>>()
                            .map_err(Into::into)
                    },
                )
                .await?;
            for addr in &addrs {
                if let Some(ref mut peerstate) = Peerstate::from_addr(context, addr).await? {
                    peerstate.recalc_fingerprint();
                    peerstate.save_to_db(self, false).await?;
                }
            }
        }

        if update_icons {
            update_saved_messages_icon(context).await?;
            update_device_icon(context).await?;
        }

        if disable_server_delete {
            // We now always watch all folders and delete messages there if delete_server is enabled.
            // So, for people who have delete_server enabled, disable it and add a hint to the devicechat:
            if context.get_config_delete_server_after().await?.is_some() {
                let mut msg = Message::new(Viewtype::Text);
                msg.text = Some(stock_str::delete_server_turned_off(context).await);
                add_device_msg(context, None, Some(&mut msg)).await?;
                context
                    .set_config(Config::DeleteServerAfter, Some("0"))
                    .await?;
            }
        }

        if recode_avatar {
            if let Some(avatar) = context.get_config(Config::Selfavatar).await? {
                let mut blob = BlobObject::new_from_path(context, avatar.as_ref()).await?;
                match blob.recode_to_avatar_size(context).await {
                    Ok(()) => {
                        context
                            .set_config(Config::Selfavatar, Some(&avatar))
                            .await?
                    }
                    Err(e) => {
                        warn!(context, "Migrations can't recode avatar, removing. {:#}", e);
                        context.set_config(Config::Selfavatar, None).await?
                    }
                }
            }
        }

        Ok(())
    }

    /// Opens the provided database and runs any necessary migrations.
    /// If a database is already open, this will return an error.
    pub async fn open(&self, context: &Context, passphrase: String) -> Result<()> {
        if self.is_open().await {
            error!(
                context,
                "Cannot open, database \"{:?}\" already opened.", self.dbfile,
            );
            bail!("SQL database is already opened.");
        }

        if let Err(err) = self.try_open(context, &self.dbfile, passphrase).await {
            self.close().await;
            Err(err)
        } else {
            info!(context, "Opened database {:?}.", self.dbfile);
            Ok(())
        }
    }

    /// Execute the given query, returning the number of affected rows.
    pub async fn execute(
        &self,
        query: impl AsRef<str>,
        params: impl rusqlite::Params,
    ) -> Result<usize> {
        let conn = self.get_conn().await?;
        let res = conn.execute(query.as_ref(), params)?;
        Ok(res)
    }

    /// Executes the given query, returning the last inserted row ID.
    pub async fn insert(
        &self,
        query: impl AsRef<str>,
        params: impl rusqlite::Params,
    ) -> Result<i64> {
        let conn = self.get_conn().await?;
        conn.execute(query.as_ref(), params)?;
        Ok(conn.last_insert_rowid())
    }

    /// Prepares and executes the statement and maps a function over the resulting rows.
    /// Then executes the second function over the returned iterator and returns the
    /// result of that function.
    pub async fn query_map<T, F, G, H>(
        &self,
        sql: impl AsRef<str>,
        params: impl rusqlite::Params,
        f: F,
        mut g: G,
    ) -> Result<H>
    where
        F: FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
        G: FnMut(rusqlite::MappedRows<F>) -> Result<H>,
    {
        let sql = sql.as_ref();

        let conn = self.get_conn().await?;
        let mut stmt = conn.prepare(sql)?;
        let res = stmt.query_map(params, f)?;
        g(res)
    }

    pub async fn get_conn(
        &self,
    ) -> Result<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>> {
        let lock = self.pool.read().await;
        let pool = lock.as_ref().context("no SQL connection")?;
        let conn = pool.get()?;

        Ok(conn)
    }

    /// Used for executing `SELECT COUNT` statements only. Returns the resulting count.
    pub async fn count(
        &self,
        query: impl AsRef<str>,
        params: impl rusqlite::Params,
    ) -> anyhow::Result<usize> {
        let count: isize = self.query_row(query, params, |row| row.get(0)).await?;
        Ok(usize::try_from(count)?)
    }

    /// Used for executing `SELECT COUNT` statements only. Returns `true`, if the count is at least
    /// one, `false` otherwise.
    pub async fn exists(&self, sql: &str, params: impl rusqlite::Params) -> Result<bool> {
        let count = self.count(sql, params).await?;
        Ok(count > 0)
    }

    /// Execute a query which is expected to return one row.
    pub async fn query_row<T, F>(
        &self,
        query: impl AsRef<str>,
        params: impl rusqlite::Params,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let conn = self.get_conn().await?;
        let res = conn.query_row(query.as_ref(), params, f)?;
        Ok(res)
    }

    /// Execute the function inside a transaction.
    ///
    /// If the function returns an error, the transaction will be rolled back. If it does not return an
    /// error, the transaction will be committed.
    pub async fn transaction<G, H>(&self, callback: G) -> anyhow::Result<H>
    where
        H: Send + 'static,
        G: Send + 'static + FnOnce(&mut rusqlite::Transaction<'_>) -> anyhow::Result<H>,
    {
        let mut conn = self.get_conn().await?;
        let mut transaction = conn.transaction()?;
        let ret = callback(&mut transaction);

        match ret {
            Ok(ret) => {
                transaction.commit()?;
                Ok(ret)
            }
            Err(err) => {
                transaction.rollback()?;
                Err(err)
            }
        }
    }

    /// Query the database if the requested table already exists.
    pub async fn table_exists(&self, name: &str) -> anyhow::Result<bool> {
        let conn = self.get_conn().await?;
        let mut exists = false;
        conn.pragma(None, "table_info", &name.to_string(), |_row| {
            // will only be executed if the info was found
            exists = true;
            Ok(())
        })?;

        Ok(exists)
    }

    /// Check if a column exists in a given table.
    pub async fn col_exists(&self, table_name: &str, col_name: &str) -> anyhow::Result<bool> {
        let conn = self.get_conn().await?;
        let mut exists = false;
        // `PRAGMA table_info` returns one row per column,
        // each row containing 0=cid, 1=name, 2=type, 3=notnull, 4=dflt_value
        conn.pragma(None, "table_info", &table_name.to_string(), |row| {
            let curr_name: String = row.get(1)?;
            if col_name == curr_name {
                exists = true;
            }
            Ok(())
        })?;

        Ok(exists)
    }

    /// Execute a query which is expected to return zero or one row.
    pub async fn query_row_optional<T, F>(
        &self,
        sql: impl AsRef<str>,
        params: impl rusqlite::Params,
        f: F,
    ) -> anyhow::Result<Option<T>>
    where
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let conn = self.get_conn().await?;
        let res = match conn.query_row(sql.as_ref(), params, f) {
            Ok(res) => Ok(Some(res)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(rusqlite::Error::InvalidColumnType(_, _, rusqlite::types::Type::Null)) => Ok(None),
            Err(err) => Err(err),
        }?;
        Ok(res)
    }

    /// Executes a query which is expected to return one row and one
    /// column. If the query does not return a value or returns SQL
    /// `NULL`, returns `Ok(None)`.
    pub async fn query_get_value<T>(
        &self,
        query: &str,
        params: impl rusqlite::Params,
    ) -> anyhow::Result<Option<T>>
    where
        T: rusqlite::types::FromSql,
    {
        self.query_row_optional(query, params, |row| row.get::<_, T>(0))
            .await
    }

    /// Set private configuration options.
    ///
    /// Setting `None` deletes the value.  On failure an error message
    /// will already have been logged.
    pub async fn set_raw_config(&self, key: impl AsRef<str>, value: Option<&str>) -> Result<()> {
        let key = key.as_ref();
        if let Some(value) = value {
            let exists = self
                .exists(
                    "SELECT COUNT(*) FROM config WHERE keyname=?;",
                    paramsv![key],
                )
                .await?;

            if exists {
                self.execute(
                    "UPDATE config SET value=? WHERE keyname=?;",
                    paramsv![value, key],
                )
                .await?;
            } else {
                self.execute(
                    "INSERT INTO config (keyname, value) VALUES (?, ?);",
                    paramsv![key, value],
                )
                .await?;
            }
        } else {
            self.execute("DELETE FROM config WHERE keyname=?;", paramsv![key])
                .await?;
        }

        Ok(())
    }

    /// Get configuration options from the database.
    pub async fn get_raw_config(&self, key: impl AsRef<str>) -> Result<Option<String>> {
        let value = self
            .query_get_value(
                "SELECT value FROM config WHERE keyname=?;",
                paramsv![key.as_ref()],
            )
            .await
            .context(format!("failed to fetch raw config: {}", key.as_ref()))?;

        Ok(value)
    }

    pub async fn set_raw_config_int(&self, key: impl AsRef<str>, value: i32) -> Result<()> {
        self.set_raw_config(key, Some(&format!("{}", value))).await
    }

    pub async fn get_raw_config_int(&self, key: impl AsRef<str>) -> Result<Option<i32>> {
        self.get_raw_config(key)
            .await
            .map(|s| s.and_then(|s| s.parse().ok()))
    }

    pub async fn get_raw_config_bool(&self, key: impl AsRef<str>) -> Result<bool> {
        // Not the most obvious way to encode bool as string, but it is matter
        // of backward compatibility.
        let res = self.get_raw_config_int(key).await?;
        Ok(res.unwrap_or_default() > 0)
    }

    pub async fn set_raw_config_bool<T>(&self, key: T, value: bool) -> Result<()>
    where
        T: AsRef<str>,
    {
        let value = if value { Some("1") } else { None };
        self.set_raw_config(key, value).await
    }

    pub async fn set_raw_config_int64(&self, key: impl AsRef<str>, value: i64) -> Result<()> {
        self.set_raw_config(key, Some(&format!("{}", value))).await
    }

    pub async fn get_raw_config_int64(&self, key: impl AsRef<str>) -> Result<Option<i64>> {
        self.get_raw_config(key)
            .await
            .map(|s| s.and_then(|r| r.parse().ok()))
    }
}

pub async fn housekeeping(context: &Context) -> Result<()> {
    if let Err(err) = crate::ephemeral::delete_expired_messages(context).await {
        warn!(context, "Failed to delete expired messages: {}", err);
    }

    let mut files_in_use = HashSet::new();
    let mut unreferenced_count = 0;

    info!(context, "Start housekeeping...");
    maybe_add_from_param(
        &context.sql,
        &mut files_in_use,
        "SELECT param FROM msgs  WHERE chat_id!=3   AND type!=10;",
        Param::File,
    )
    .await?;
    maybe_add_from_param(
        &context.sql,
        &mut files_in_use,
        "SELECT param FROM jobs;",
        Param::File,
    )
    .await?;
    maybe_add_from_param(
        &context.sql,
        &mut files_in_use,
        "SELECT param FROM chats;",
        Param::ProfileImage,
    )
    .await?;
    maybe_add_from_param(
        &context.sql,
        &mut files_in_use,
        "SELECT param FROM contacts;",
        Param::ProfileImage,
    )
    .await?;

    context
        .sql
        .query_map(
            "SELECT value FROM config;",
            paramsv![],
            |row| row.get::<_, String>(0),
            |rows| {
                for row in rows {
                    maybe_add_file(&mut files_in_use, row?);
                }
                Ok(())
            },
        )
        .await
        .context("housekeeping: failed to SELECT value FROM config")?;

    info!(context, "{} files in use.", files_in_use.len(),);
    /* go through directory and delete unused files */
    let p = context.get_blobdir();
    match async_std::fs::read_dir(p).await {
        Ok(mut dir_handle) => {
            /* avoid deletion of files that are just created to build a message object */
            let diff = std::time::Duration::from_secs(60 * 60);
            let keep_files_newer_than = std::time::SystemTime::now().checked_sub(diff).unwrap();

            while let Some(entry) = dir_handle.next().await {
                if entry.is_err() {
                    break;
                }
                let entry = entry.unwrap();
                let name_f = entry.file_name();
                let name_s = name_f.to_string_lossy();

                if is_file_in_use(&files_in_use, None, &name_s)
                    || is_file_in_use(&files_in_use, Some(".increation"), &name_s)
                    || is_file_in_use(&files_in_use, Some(".waveform"), &name_s)
                    || is_file_in_use(&files_in_use, Some("-preview.jpg"), &name_s)
                {
                    continue;
                }

                unreferenced_count += 1;

                if let Ok(stats) = async_std::fs::metadata(entry.path()).await {
                    let recently_created =
                        stats.created().is_ok() && stats.created().unwrap() > keep_files_newer_than;
                    let recently_modified = stats.modified().is_ok()
                        && stats.modified().unwrap() > keep_files_newer_than;
                    let recently_accessed = stats.accessed().is_ok()
                        && stats.accessed().unwrap() > keep_files_newer_than;

                    if recently_created || recently_modified || recently_accessed {
                        info!(
                            context,
                            "Housekeeping: Keeping new unreferenced file #{}: {:?}",
                            unreferenced_count,
                            entry.file_name(),
                        );
                        continue;
                    }
                }
                info!(
                    context,
                    "Housekeeping: Deleting unreferenced file #{}: {:?}",
                    unreferenced_count,
                    entry.file_name()
                );
                let path = entry.path();
                dc_delete_file(context, path).await;
            }
        }
        Err(err) => {
            warn!(
                context,
                "Housekeeping: Cannot open {}. ({})",
                context.get_blobdir().display(),
                err
            );
        }
    }

    if let Err(err) = start_ephemeral_timers(context).await {
        warn!(
            context,
            "Housekeeping: cannot start ephemeral timers: {}", err
        );
    }

    if let Err(err) = prune_tombstones(&context.sql).await {
        warn!(
            context,
            "Housekeeping: Cannot prune message tombstones: {}", err
        );
    }

    if let Err(err) = deduplicate_peerstates(&context.sql).await {
        warn!(context, "Failed to deduplicate peerstates: {}", err)
    }

    context.schedule_quota_update().await?;

    // Try to clear the freelist to free some space on the disk. This
    // only works if auto_vacuum is enabled.
    if let Err(err) = context
        .sql
        .execute("PRAGMA incremental_vacuum", paramsv![])
        .await
    {
        warn!(context, "Failed to run incremental vacuum: {}", err);
    }

    if let Err(e) = context
        .set_config(Config::LastHousekeeping, Some(&time().to_string()))
        .await
    {
        warn!(context, "Can't set config: {}", e);
    }

    info!(context, "Housekeeping done.");
    Ok(())
}

#[allow(clippy::indexing_slicing)]
fn is_file_in_use(files_in_use: &HashSet<String>, namespc_opt: Option<&str>, name: &str) -> bool {
    let name_to_check = if let Some(namespc) = namespc_opt {
        let name_len = name.len();
        let namespc_len = namespc.len();
        if name_len <= namespc_len || !name.ends_with(namespc) {
            return false;
        }
        &name[..name_len - namespc_len]
    } else {
        name
    };
    files_in_use.contains(name_to_check)
}

fn maybe_add_file(files_in_use: &mut HashSet<String>, file: impl AsRef<str>) {
    if let Some(file) = file.as_ref().strip_prefix("$BLOBDIR/") {
        files_in_use.insert(file.to_string());
    }
}

async fn maybe_add_from_param(
    sql: &Sql,
    files_in_use: &mut HashSet<String>,
    query: &str,
    param_id: Param,
) -> Result<()> {
    sql.query_map(
        query,
        paramsv![],
        |row| row.get::<_, String>(0),
        |rows| {
            for row in rows {
                let param: Params = row?.parse().unwrap_or_default();
                if let Some(file) = param.get(param_id) {
                    maybe_add_file(files_in_use, file);
                }
            }
            Ok(())
        },
    )
    .await
    .context(format!("housekeeping: failed to add_from_param {}", query))?;

    Ok(())
}

/// Removes from the database locally deleted messages that also don't
/// have a server UID.
async fn prune_tombstones(sql: &Sql) -> Result<()> {
    sql.execute(
        "DELETE FROM msgs
         WHERE (chat_id=? OR hidden)
         AND NOT EXISTS (
         SELECT * FROM imap WHERE msgs.rfc724_mid=rfc724_mid AND target!=''
         )",
        paramsv![DC_CHAT_ID_TRASH],
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use async_std::channel;
    use async_std::fs::File;

    use crate::config::Config;
    use crate::{test_utils::TestContext, EventType};

    use super::*;

    #[test]
    fn test_maybe_add_file() {
        let mut files = Default::default();
        maybe_add_file(&mut files, "$BLOBDIR/hello");
        maybe_add_file(&mut files, "$BLOBDIR/world.txt");
        maybe_add_file(&mut files, "world2.txt");
        maybe_add_file(&mut files, "$BLOBDIR");

        assert!(files.contains("hello"));
        assert!(files.contains("world.txt"));
        assert!(!files.contains("world2.txt"));
        assert!(!files.contains("$BLOBDIR"));
    }

    #[test]
    fn test_is_file_in_use() {
        let mut files = Default::default();
        maybe_add_file(&mut files, "$BLOBDIR/hello");
        maybe_add_file(&mut files, "$BLOBDIR/world.txt");
        maybe_add_file(&mut files, "world2.txt");

        assert!(is_file_in_use(&files, None, "hello"));
        assert!(!is_file_in_use(&files, Some(".txt"), "hello"));
        assert!(is_file_in_use(&files, Some("-suffix"), "world.txt-suffix"));
    }

    #[async_std::test]
    async fn test_table_exists() {
        let t = TestContext::new().await;
        assert!(t.ctx.sql.table_exists("msgs").await.unwrap());
        assert!(!t.ctx.sql.table_exists("foobar").await.unwrap());
    }

    #[async_std::test]
    async fn test_col_exists() {
        let t = TestContext::new().await;
        assert!(t.ctx.sql.col_exists("msgs", "mime_modified").await.unwrap());
        assert!(!t.ctx.sql.col_exists("msgs", "foobar").await.unwrap());
        assert!(!t.ctx.sql.col_exists("foobar", "foobar").await.unwrap());
    }

    /// Tests that auto_vacuum is enabled for new databases.
    #[async_std::test]
    async fn test_auto_vacuum() -> Result<()> {
        let t = TestContext::new().await;

        let conn = t.sql.get_conn().await?;
        let auto_vacuum = conn.pragma_query_value(None, "auto_vacuum", |row| {
            let auto_vacuum: i32 = row.get(0)?;
            Ok(auto_vacuum)
        })?;

        // auto_vacuum=2 is the same as auto_vacuum=INCREMENTAL
        assert_eq!(auto_vacuum, 2);
        Ok(())
    }

    #[async_std::test]
    async fn test_housekeeping_db_closed() {
        let t = TestContext::new().await;

        let avatar_src = t.dir.path().join("avatar.png");
        let avatar_bytes = include_bytes!("../test-data/image/avatar64x64.png");
        File::create(&avatar_src)
            .await
            .unwrap()
            .write_all(avatar_bytes)
            .await
            .unwrap();
        t.set_config(Config::Selfavatar, Some(avatar_src.to_str().unwrap()))
            .await
            .unwrap();

        let (event_sink, event_source) = channel::unbounded();
        t.add_event_sender(event_sink).await;

        let a = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        assert_eq!(avatar_bytes, &async_std::fs::read(&a).await.unwrap()[..]);

        t.sql.close().await;
        housekeeping(&t).await.unwrap_err(); // housekeeping should fail as the db is closed
        t.sql.open(&t, "".to_string()).await.unwrap();

        let a = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        assert_eq!(avatar_bytes, &async_std::fs::read(&a).await.unwrap()[..]);

        while let Ok(event) = event_source.try_recv() {
            match event.typ {
                EventType::Info(s) => assert!(
                    !s.contains("Keeping new unreferenced file"),
                    "File {} was almost deleted, only reason it was kept is that it was created recently (as the tests don't run for a long time)",
                    s
                ),
                EventType::Error(s) => panic!("{}", s),
                _ => {}
            }
        }
    }

    /// Regression test.
    ///
    /// Previously the code checking for existence of `config` table
    /// checked it with `PRAGMA table_info("config")` but did not
    /// drain `SqlitePool.fetch` result, only using the first row
    /// returned. As a result, prepared statement for `PRAGMA` was not
    /// finalized early enough, leaving reader connection in a broken
    /// state after reopening the database, when `config` table
    /// existed and `PRAGMA` returned non-empty result.
    ///
    /// Statements were not finalized due to a bug in sqlx:
    /// <https://github.com/launchbadge/sqlx/issues/1147>
    #[async_std::test]
    async fn test_db_reopen() -> Result<()> {
        use tempfile::tempdir;

        // The context is used only for logging.
        let t = TestContext::new().await;

        // Create a separate empty database for testing.
        let dir = tempdir()?;
        let dbfile = dir.path().join("testdb.sqlite");
        let sql = Sql::new(dbfile.into());

        // Create database with all the tables.
        sql.open(&t, "".to_string()).await.unwrap();
        sql.close().await;

        // Reopen the database
        sql.open(&t, "".to_string()).await?;
        sql.execute(
            "INSERT INTO config (keyname, value) VALUES (?, ?);",
            paramsv!("foo", "bar"),
        )
        .await?;

        let value: Option<String> = sql
            .query_get_value("SELECT value FROM config WHERE keyname=?;", paramsv!("foo"))
            .await?;
        assert_eq!(value.unwrap(), "bar");

        Ok(())
    }

    #[async_std::test]
    async fn test_migration_flags() -> Result<()> {
        let t = TestContext::new().await;
        t.evtracker.get_info_contains("Opened database").await;

        // as migrations::run() was already executed on context creation,
        // another call should not result in any action needed.
        // this test catches some bugs where dbversion was forgotten to be persisted.
        let (recalc_fingerprints, update_icons, disable_server_delete, recode_avatar) =
            migrations::run(&t, &t.sql).await?;
        assert!(!recalc_fingerprints);
        assert!(!update_icons);
        assert!(!disable_server_delete);
        assert!(!recode_avatar);

        info!(&t, "test_migration_flags: XXX END MARKER");

        loop {
            let evt = t
                .evtracker
                .get_matching(|evt| matches!(evt, EventType::Info(_)))
                .await;
            match evt {
                EventType::Info(msg) => {
                    assert!(
                        !msg.contains("[migration]"),
                        "Migrations were run twice, you probably forgot to update the db version"
                    );
                    if msg.contains("test_migration_flags: XXX END MARKER") {
                        break;
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_check_passphrase() -> Result<()> {
        use tempfile::tempdir;

        // The context is used only for logging.
        let t = TestContext::new().await;

        // Create a separate empty database for testing.
        let dir = tempdir()?;
        let dbfile = dir.path().join("testdb.sqlite");
        let sql = Sql::new(dbfile.clone().into());

        sql.check_passphrase("foo".to_string()).await?;
        sql.open(&t, "foo".to_string())
            .await
            .context("failed to open the database first time")?;
        sql.close().await;

        // Reopen the database
        let sql = Sql::new(dbfile.into());

        // Test that we can't open encrypted database without a passphrase.
        assert!(sql.open(&t, "".to_string()).await.is_err());

        // Now open the database with passpharse, it should succeed.
        sql.check_passphrase("foo".to_string()).await?;
        sql.open(&t, "foo".to_string())
            .await
            .context("failed to open the database second time")?;
        Ok(())
    }
}

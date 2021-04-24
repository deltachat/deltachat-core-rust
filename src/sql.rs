//! # SQLite wrapper

use std::collections::HashSet;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use anyhow::Context as _;
use async_std::prelude::*;
use async_std::sync::RwLock;
use sqlx::{
    pool::PoolOptions,
    query::Query,
    sqlite::{Sqlite, SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous},
    Executor, IntoArguments, Row,
};

use crate::chat::{add_device_msg, update_device_icon, update_saved_messages_icon};
use crate::config::Config;
use crate::constants::{Viewtype, DC_CHAT_ID_TRASH};
use crate::context::Context;
use crate::dc_tools::{dc_delete_file, time};
use crate::ephemeral::start_ephemeral_timers;
use crate::message::Message;
use crate::param::{Param, Params};
use crate::peerstate::Peerstate;
use crate::stock_str;

mod error;
mod migrations;

pub use self::error::*;

/// A wrapper around the underlying Sqlite3 object.
///
/// We maintain two different pools to sqlite, on for reading, one for writing.
/// This can go away once https://github.com/launchbadge/sqlx/issues/459 is implemented.
#[derive(Debug)]
pub struct Sql {
    /// Writer pool, must only have 1 connection in it.
    writer: RwLock<Option<SqlitePool>>,
    /// Reader pool, maintains multiple connections for reading data.
    reader: RwLock<Option<SqlitePool>>,
}

impl Default for Sql {
    fn default() -> Self {
        Self {
            writer: RwLock::new(None),
            reader: RwLock::new(None),
        }
    }
}

impl Drop for Sql {
    fn drop(&mut self) {
        async_std::task::block_on(self.close());
    }
}

impl Sql {
    pub fn new() -> Sql {
        Self::default()
    }

    /// Checks if there is currently a connection to the underlying Sqlite database.
    pub async fn is_open(&self) -> bool {
        // in read only mode the writer does not exists
        self.reader.read().await.is_some()
    }

    /// Closes all underlying Sqlite connections.
    pub async fn close(&self) {
        if let Some(sql) = self.writer.write().await.take() {
            sql.close().await;
        }
        if let Some(sql) = self.reader.write().await.take() {
            sql.close().await;
        }
    }

    async fn new_writer_pool(dbfile: impl AsRef<Path>) -> sqlx::Result<SqlitePool> {
        let config = SqliteConnectOptions::new()
            .journal_mode(SqliteJournalMode::Wal)
            .filename(dbfile.as_ref())
            .read_only(false)
            .busy_timeout(Duration::from_secs(100))
            .create_if_missing(true)
            .shared_cache(true)
            .synchronous(SqliteSynchronous::Normal);

        PoolOptions::<Sqlite>::new()
            .max_connections(1)
            .after_connect(|conn| {
                Box::pin(async move {
                    let q = r#"
PRAGMA secure_delete=on;
PRAGMA temp_store=memory; -- Avoid SQLITE_IOERR_GETTEMPPATH errors on Android
"#;

                    conn.execute_many(sqlx::query(q))
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .await?;
                    Ok(())
                })
            })
            .connect_with(config)
            .await
    }

    async fn new_reader_pool(dbfile: impl AsRef<Path>, readonly: bool) -> sqlx::Result<SqlitePool> {
        let config = SqliteConnectOptions::new()
            .journal_mode(SqliteJournalMode::Wal)
            .filename(dbfile.as_ref())
            .read_only(readonly)
            .shared_cache(true)
            .busy_timeout(Duration::from_secs(100))
            .synchronous(SqliteSynchronous::Normal);

        PoolOptions::<Sqlite>::new()
            .max_connections(10)
            .after_connect(|conn| {
                Box::pin(async move {
                    let q = r#"
PRAGMA temp_store=memory; -- Avoid SQLITE_IOERR_GETTEMPPATH errors on Android
PRAGMA query_only=1; -- Protect against writes even in read-write mode
PRAGMA read_uncommitted=1; -- This helps avoid "table locked" errors in shared cache mode
"#;

                    conn.execute_many(sqlx::query(q))
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .await?;
                    Ok(())
                })
            })
            .connect_with(config)
            .await
    }

    /// Opens the provided database and runs any necessary migrations.
    /// If a database is already open, this will return an error.
    pub async fn open(
        &self,
        context: &Context,
        dbfile: impl AsRef<Path>,
        readonly: bool,
    ) -> anyhow::Result<()> {
        if self.is_open().await {
            error!(
                context,
                "Cannot open, database \"{:?}\" already opened.",
                dbfile.as_ref(),
            );
            return Err(Error::SqlAlreadyOpen.into());
        }

        // Open write pool
        if !readonly {
            *self.writer.write().await = Some(Self::new_writer_pool(&dbfile).await?);
        }

        // Open read pool
        *self.reader.write().await = Some(Self::new_reader_pool(&dbfile, readonly).await?);

        if !readonly {
            // (1) update low-level database structure.
            // this should be done before updates that use high-level objects that
            // rely themselves on the low-level structure.

            let (recalc_fingerprints, update_icons, disable_server_delete) =
                migrations::run(context, self).await?;

            // (2) updates that require high-level objects
            // the structure is complete now and all objects are usable

            if recalc_fingerprints {
                info!(context, "[migration] recalc fingerprints");
                let mut rows = self
                    .fetch(sqlx::query("SELECT addr FROM acpeerstates;"))
                    .await?;

                while let Some(row) = rows.next().await {
                    let row = row?;
                    let addr = row.try_get(0)?;
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
        }

        info!(context, "Opened {:?}.", dbfile.as_ref());

        Ok(())
    }

    /// Execute the given query, returning the number of affected rows.
    pub async fn execute<'q, E>(&self, query: Query<'q, Sqlite, E>) -> Result<u64>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        let lock = self.writer.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let rows = pool.execute(query).await?;
        Ok(rows.rows_affected())
    }

    /// Executes the given query, returning the last inserted row ID.
    pub async fn insert<'q, E>(&self, query: Query<'q, Sqlite, E>) -> Result<i64>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        let lock = self.writer.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let rows = pool.execute(query).await?;
        Ok(rows.last_insert_rowid())
    }

    /// Execute many queries.
    pub async fn execute_many<'q, E>(&self, query: Query<'q, Sqlite, E>) -> Result<()>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        let lock = self.writer.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        pool.execute_many(query)
            .collect::<sqlx::Result<Vec<_>>>()
            .await?;
        Ok(())
    }

    /// Fetch the given query.
    pub async fn fetch<'q, E>(
        &self,
        query: Query<'q, Sqlite, E>,
    ) -> Result<impl Stream<Item = sqlx::Result<<Sqlite as sqlx::Database>::Row>> + Send + 'q>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        let lock = self.reader.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let rows = pool.fetch(query);
        Ok(rows)
    }

    /// Fetch exactly one row, errors if no row is found.
    pub async fn fetch_one<'q, E>(
        &self,
        query: Query<'q, Sqlite, E>,
    ) -> Result<<Sqlite as sqlx::Database>::Row>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        let lock = self.reader.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let row = pool.fetch_one(query).await?;
        Ok(row)
    }

    /// Fetches at most one row.
    pub async fn fetch_optional<'e, 'q, E>(
        &self,
        query: Query<'q, Sqlite, E>,
    ) -> Result<Option<<Sqlite as sqlx::Database>::Row>>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        let lock = self.reader.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let row = pool.fetch_optional(query).await?;
        Ok(row)
    }

    /// Used for executing `SELECT COUNT` statements only. Returns the resulting count.
    pub async fn count<'e, 'q, E>(&self, query: Query<'q, Sqlite, E>) -> Result<usize>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        use std::convert::TryFrom;

        let row = self.fetch_one(query).await?;
        let count: i64 = row.try_get(0)?;

        Ok(usize::try_from(count).map_err::<anyhow::Error, _>(Into::into)?)
    }

    /// Used for executing `SELECT COUNT` statements only. Returns `true`, if the count is at least
    /// one, `false` otherwise.
    pub async fn exists<'e, 'q, E>(&self, query: Query<'q, Sqlite, E>) -> Result<bool>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
    {
        let count = self.count(query).await?;
        Ok(count > 0)
    }

    /// Execute the function inside a transaction.
    ///
    /// If the function returns an error, the transaction will be rolled back. If it does not return an
    /// error, the transaction will be committed.
    pub async fn transaction<F, R>(&self, callback: F) -> Result<R>
    where
        F: for<'c> FnOnce(
                &'c mut sqlx::Transaction<'_, Sqlite>,
            ) -> Pin<Box<dyn Future<Output = Result<R>> + 'c + Send>>
            + 'static
            + Send
            + Sync,
        R: Send,
    {
        let lock = self.writer.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let mut transaction = pool.begin().await?;
        let ret = callback(&mut transaction).await;

        match ret {
            Ok(ret) => {
                transaction.commit().await?;

                Ok(ret)
            }
            Err(err) => {
                transaction.rollback().await?;

                Err(err)
            }
        }
    }

    /// Query the database if the requested table already exists.
    pub async fn table_exists(&self, name: impl AsRef<str>) -> Result<bool> {
        let q = format!("PRAGMA table_info(\"{}\")", name.as_ref());

        let lock = self.reader.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let mut rows = pool.fetch(sqlx::query(&q));
        if let Some(first_row) = rows.next().await {
            Ok(first_row.is_ok())
        } else {
            Ok(false)
        }
    }

    /// Check if a column exists in a given table.
    pub async fn col_exists(
        &self,
        table_name: impl AsRef<str>,
        col_name: impl AsRef<str>,
    ) -> Result<bool> {
        let q = format!("PRAGMA table_info(\"{}\")", table_name.as_ref());
        let lock = self.reader.read().await;
        let pool = lock.as_ref().ok_or(Error::SqlNoConnection)?;

        let mut rows = pool.fetch(sqlx::query(&q));
        while let Some(row) = rows.next().await {
            let row = row?;

            // `PRAGMA table_info` returns one row per column,
            // each row containing 0=cid, 1=name, 2=type, 3=notnull, 4=dflt_value

            let curr_name: &str = row.try_get(1)?;
            if col_name.as_ref() == curr_name {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Executes a query which is expected to return one row and one
    /// column. If the query does not return a value or returns SQL
    /// `NULL`, returns `Ok(None)`.
    pub async fn query_get_value<'e, 'q, E, T>(
        &self,
        query: Query<'q, Sqlite, E>,
    ) -> Result<Option<T>>
    where
        E: 'q + IntoArguments<'q, Sqlite>,
        T: for<'r> sqlx::Decode<'r, Sqlite> + sqlx::Type<Sqlite>,
    {
        let res = self
            .fetch_optional(query)
            .await?
            .map(|row| row.get::<T, _>(0));
        Ok(res)
    }

    /// Set private configuration options.
    ///
    /// Setting `None` deletes the value.  On failure an error message
    /// will already have been logged.
    pub async fn set_raw_config(&self, key: impl AsRef<str>, value: Option<&str>) -> Result<()> {
        if !self.is_open().await {
            return Err(Error::SqlNoConnection);
        }

        let key = key.as_ref();
        if let Some(value) = value {
            let exists = self
                .exists(sqlx::query("SELECT COUNT(*) FROM config WHERE keyname=?;").bind(key))
                .await?;

            if exists {
                self.execute(
                    sqlx::query("UPDATE config SET value=? WHERE keyname=?;")
                        .bind(value)
                        .bind(key),
                )
                .await?;
            } else {
                self.execute(
                    sqlx::query("INSERT INTO config (keyname, value) VALUES (?, ?);")
                        .bind(key)
                        .bind(value),
                )
                .await?;
            }
        } else {
            self.execute(sqlx::query("DELETE FROM config WHERE keyname=?;").bind(key))
                .await?;
        }

        Ok(())
    }

    /// Get configuration options from the database.
    pub async fn get_raw_config(&self, key: impl AsRef<str>) -> Result<Option<String>> {
        if !self.is_open().await || key.as_ref().is_empty() {
            return Err(Error::SqlNoConnection);
        }
        let value = self
            .query_get_value(
                sqlx::query("SELECT value FROM config WHERE keyname=?;").bind(key.as_ref()),
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

    let mut rows = context
        .sql
        .fetch(sqlx::query("SELECT value FROM config;"))
        .await?;
    while let Some(row) = rows.next().await {
        let row: String = row?.try_get(0)?;
        maybe_add_file(&mut files_in_use, row);
    }

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
    let mut rows = sql.fetch(sqlx::query(query)).await?;
    while let Some(row) = rows.next().await {
        let row: String = row?.try_get(0)?;
        let param: Params = row.parse().unwrap_or_default();
        if let Some(file) = param.get(param_id) {
            maybe_add_file(files_in_use, file);
        }
    }

    Ok(())
}

/// Removes from the database locally deleted messages that also don't
/// have a server UID.
async fn prune_tombstones(sql: &Sql) -> Result<()> {
    sql.execute(
        sqlx::query(
            "DELETE FROM msgs \
         WHERE (chat_id = ? OR hidden) \
         AND server_uid = 0",
        )
        .bind(DC_CHAT_ID_TRASH),
    )
    .await?;
    Ok(())
}

/// Returns the SQLite version as a string; e.g., `"3.16.2"` for version 3.16.2.
pub fn version() -> &'static str {
    #[allow(unsafe_code)]
    let cstr = unsafe { std::ffi::CStr::from_ptr(libsqlite3_sys::sqlite3_libversion()) };
    cstr.to_str()
        .expect("SQLite version string is not valid UTF8 ?!")
}

#[cfg(test)]
mod test {
    use async_std::fs::File;

    use crate::config::Config;
    use crate::{test_utils::TestContext, Event, EventType};

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

        t.add_event_sink(move |event: Event| async move {
            match event.typ {
                EventType::Info(s) => assert!(
                    !s.contains("Keeping new unreferenced file"),
                    "File {} was almost deleted, only reason it was kept is that it was created recently (as the tests don't run for a long time)",
                    s
                ),
                EventType::Error(s) => panic!("{}", s),
                _ => {}
            }
        })
        .await;

        let a = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        assert_eq!(avatar_bytes, &async_std::fs::read(&a).await.unwrap()[..]);

        t.sql.close().await;
        housekeeping(&t).await.unwrap_err(); // housekeeping should fail as the db is closed
        t.sql.open(&t, &t.get_dbfile(), false).await.unwrap();

        let a = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        assert_eq!(avatar_bytes, &async_std::fs::read(&a).await.unwrap()[..]);
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
    /// https://github.com/launchbadge/sqlx/issues/1147
    #[async_std::test]
    async fn test_db_reopen() -> Result<()> {
        use tempfile::tempdir;

        // The context is used only for logging.
        let t = TestContext::new().await;

        // Create a separate empty database for testing.
        let dir = tempdir()?;
        let dbfile = dir.path().join("testdb.sqlite");
        let sql = Sql::new();

        // Create database with all the tables.
        sql.open(&t, &dbfile, false).await.unwrap();
        sql.close().await;

        // Reopen the database
        sql.open(&t, &dbfile, false).await?;
        sql.execute(
            sqlx::query("INSERT INTO config (keyname, value) VALUES (?, ?);")
                .bind("foo")
                .bind("bar"),
        )
        .await?;

        let value: Option<String> = sql
            .query_get_value(sqlx::query("SELECT value FROM config WHERE keyname=?;").bind("foo"))
            .await?;
        assert_eq!(value.unwrap(), "bar");

        Ok(())
    }
}

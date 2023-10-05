//! # SQLite wrapper.

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use rusqlite::{self, config::DbConfig, types::ValueRef, Connection, OpenFlags, Row};
use tokio::sync::{Mutex, MutexGuard, RwLock};

use crate::blob::BlobObject;
use crate::chat::{add_device_msg, update_device_icon, update_saved_messages_icon};
use crate::config::Config;
use crate::constants::DC_CHAT_ID_TRASH;
use crate::contact::Origin;
use crate::context::Context;
use crate::debug_logging::set_debug_logging_xdc;
use crate::ephemeral::start_ephemeral_timers;
use crate::imex::BLOBS_BACKUP_NAME;
use crate::log::LogExt;
use crate::message::{Message, MsgId, Viewtype};
use crate::param::{Param, Params};
use crate::peerstate::{deduplicate_peerstates, Peerstate};
use crate::stock_str;
use crate::tools::{delete_file, time};

/// Extension to [`rusqlite::ToSql`] trait
/// which also includes [`Send`] and [`Sync`].
pub trait ToSql: rusqlite::ToSql + Send + Sync {}

impl<T: rusqlite::ToSql + Send + Sync> ToSql for T {}

/// Constructs a slice of trait object references `&dyn ToSql`.
///
/// One of the uses is passing more than 16 parameters
/// to a query, because [`rusqlite::Params`] is only implemented
/// for tuples of up to 16 elements.
#[macro_export]
macro_rules! params_slice {
    ($($param:expr),+) => {
        [$(&$param as &dyn $crate::sql::ToSql),+]
    };
}

pub(crate) fn params_iter(
    iter: &[impl crate::sql::ToSql],
) -> impl Iterator<Item = &dyn crate::sql::ToSql> {
    iter.iter().map(|item| item as &dyn crate::sql::ToSql)
}

mod migrations;
mod pool;

use pool::Pool;

/// A wrapper around the underlying Sqlite3 object.
#[derive(Debug)]
pub struct Sql {
    /// Database file path
    pub(crate) dbfile: PathBuf,

    /// Write transactions mutex.
    ///
    /// See [`Self::write_lock`].
    write_mtx: Mutex<()>,

    /// SQL connection pool.
    pool: RwLock<Option<Pool>>,

    /// None if the database is not open, true if it is open with passphrase and false if it is
    /// open without a passphrase.
    is_encrypted: RwLock<Option<bool>>,

    /// Cache of `config` table.
    pub(crate) config_cache: RwLock<HashMap<String, Option<String>>>,
}

impl Sql {
    /// Creates new SQL database.
    pub fn new(dbfile: PathBuf) -> Sql {
        Self {
            dbfile,
            write_mtx: Mutex::new(()),
            pool: Default::default(),
            is_encrypted: Default::default(),
            config_cache: Default::default(),
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

    /// Returns true if the database is encrypted.
    ///
    /// If database is not open, returns `None`.
    pub(crate) async fn is_encrypted(&self) -> Option<bool> {
        *self.is_encrypted.read().await
    }

    /// Closes all underlying Sqlite connections.
    async fn close(&self) {
        let _ = self.pool.write().await.take();
        // drop closes the connection
    }

    /// Imports the database from a separate file with the given passphrase.
    pub(crate) async fn import(&self, path: &Path, passphrase: String) -> Result<()> {
        let path_str = path
            .to_str()
            .with_context(|| format!("path {path:?} is not valid unicode"))?
            .to_string();
        let res = self
            .call_write(move |conn| {
                // Check that backup passphrase is correct before resetting our database.
                conn.execute("ATTACH DATABASE ? AS backup KEY ?", (path_str, passphrase))
                    .context("failed to attach backup database")?;
                if let Err(err) = conn
                    .query_row("SELECT count(*) FROM sqlite_master", [], |_row| Ok(()))
                    .context("backup passphrase is not correct")
                {
                    conn.execute("DETACH DATABASE backup", [])
                        .context("failed to detach backup database")?;
                    return Err(err);
                }

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
                let res = conn
                    .query_row("SELECT sqlcipher_export('main', 'backup')", [], |_row| {
                        Ok(())
                    })
                    .context("failed to import from attached backup database");
                conn.execute("DETACH DATABASE backup", [])
                    .context("failed to detach backup database")?;
                res?;
                Ok(())
            })
            .await;

        // The config cache is wrong now that we have a different database
        self.config_cache.write().await.clear();

        res
    }

    /// Creates a new connection pool.
    fn new_pool(dbfile: &Path, passphrase: String) -> Result<Pool> {
        let mut connections = Vec::new();
        for _ in 0..3 {
            let connection = new_connection(dbfile, &passphrase)?;
            connections.push(connection);
        }

        let pool = Pool::new(connections);
        Ok(pool)
    }

    async fn try_open(&self, context: &Context, dbfile: &Path, passphrase: String) -> Result<()> {
        *self.pool.write().await = Some(Self::new_pool(dbfile, passphrase.to_string())?);

        self.run_migrations(context).await?;

        Ok(())
    }

    /// Updates SQL schema to the latest version.
    pub async fn run_migrations(&self, context: &Context) -> Result<()> {
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
                    (),
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
                    peerstate.save_to_db(self).await?;
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
                msg.set_text(stock_str::delete_server_turned_off(context).await);
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

        let passphrase_nonempty = !passphrase.is_empty();
        if let Err(err) = self.try_open(context, &self.dbfile, passphrase).await {
            self.close().await;
            Err(err)
        } else {
            info!(context, "Opened database {:?}.", self.dbfile);
            *self.is_encrypted.write().await = Some(passphrase_nonempty);

            // setup debug logging if there is an entry containing its id
            if let Some(xdc_id) = self
                .get_raw_config_u32(Config::DebugLogging.as_ref())
                .await?
            {
                set_debug_logging_xdc(context, Some(MsgId::new(xdc_id))).await?;
            }

            Ok(())
        }
    }

    /// Changes the passphrase of encrypted database.
    ///
    /// The database must already be encrypted and the passphrase cannot be empty.
    /// It is impossible to turn encrypted database into unencrypted
    /// and vice versa this way, use import/export for this.
    pub async fn change_passphrase(&self, passphrase: String) -> Result<()> {
        let mut lock = self.pool.write().await;

        let pool = lock.take().context("SQL connection pool is not open")?;
        let conn = pool.get().await?;
        conn.pragma_update(None, "rekey", passphrase.clone())
            .context("failed to set PRAGMA rekey")?;
        drop(pool);

        *lock = Some(Self::new_pool(&self.dbfile, passphrase.to_string())?);

        Ok(())
    }

    /// Locks the write transactions mutex in order to make sure that there never are
    /// multiple write transactions at once.
    ///
    /// Doing the locking ourselves instead of relying on SQLite has these reasons:
    ///
    /// - SQLite's locking mechanism is non-async, blocking a thread
    /// - SQLite's locking mechanism just sleeps in a loop, which is really inefficient
    ///
    /// ---
    ///
    /// More considerations on alternatives to the current approach:
    ///
    /// We use [DEFERRED](https://www.sqlite.org/lang_transaction.html#deferred_immediate_and_exclusive_transactions) transactions.
    ///
    /// In order to never get concurrency issues, we could make all transactions IMMEDIATE,
    /// but this would mean that there can never be two simultaneous transactions.
    ///
    /// Read transactions can simply be made DEFERRED to run in parallel w/o any drawbacks.
    ///
    /// DEFERRED write transactions without doing the locking ourselves would have these drawbacks:
    ///
    /// 1. As mentioned above, SQLite's locking mechanism is non-async and sleeps in a loop.
    /// 2. If there are other write transactions, we block the db connection until
    ///   upgraded. If some reader comes then, it has to get the next, less used connection with a
    ///   worse per-connection page cache (SQLite allows one write and any number of reads in parallel).
    /// 3. If a transaction is blocked for more than `busy_timeout`, it fails with SQLITE_BUSY.
    /// 4. If upon a successful upgrade to a write transaction the db has been modified,
    ///   the transaction has to be rolled back and retried, which means extra work in terms of
    ///   CPU/battery.
    ///
    /// The only pro of making write transactions DEFERRED w/o the external locking would be some
    /// parallelism between them.
    ///
    /// Another option would be to make write transactions IMMEDIATE, also
    /// w/o the external locking. But then cons 1. - 3. above would still be valid.
    pub async fn write_lock(&self) -> MutexGuard<'_, ()> {
        self.write_mtx.lock().await
    }

    /// Allocates a connection and calls `function` with the connection. If `function` does write
    /// queries,
    /// - either first take a lock using `write_lock()`
    /// - or use `call_write()` instead.
    ///
    /// Returns the result of the function.
    async fn call<'a, F, R>(&'a self, function: F) -> Result<R>
    where
        F: 'a + FnOnce(&mut Connection) -> Result<R> + Send,
        R: Send + 'static,
    {
        let lock = self.pool.read().await;
        let pool = lock.as_ref().context("no SQL connection")?;
        let mut conn = pool.get().await?;
        let res = tokio::task::block_in_place(move || function(&mut conn))?;
        Ok(res)
    }

    /// Allocates a connection and calls given function, assuming it does write queries, with the
    /// connection.
    ///
    /// Returns the result of the function.
    pub async fn call_write<'a, F, R>(&'a self, function: F) -> Result<R>
    where
        F: 'a + FnOnce(&mut Connection) -> Result<R> + Send,
        R: Send + 'static,
    {
        let _lock = self.write_lock().await;
        self.call(function).await
    }

    /// Execute `query` assuming it is a write query, returning the number of affected rows.
    pub async fn execute(
        &self,
        query: &str,
        params: impl rusqlite::Params + Send,
    ) -> Result<usize> {
        self.call_write(move |conn| {
            let res = conn.execute(query, params)?;
            Ok(res)
        })
        .await
    }

    /// Executes the given query, returning the last inserted row ID.
    pub async fn insert(&self, query: &str, params: impl rusqlite::Params + Send) -> Result<i64> {
        self.call_write(move |conn| {
            conn.execute(query, params)?;
            Ok(conn.last_insert_rowid())
        })
        .await
    }

    /// Prepares and executes the statement and maps a function over the resulting rows.
    /// Then executes the second function over the returned iterator and returns the
    /// result of that function.
    pub async fn query_map<T, F, G, H>(
        &self,
        sql: &str,
        params: impl rusqlite::Params + Send,
        f: F,
        mut g: G,
    ) -> Result<H>
    where
        F: Send + FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
        G: Send + FnMut(rusqlite::MappedRows<F>) -> Result<H>,
        H: Send + 'static,
    {
        self.call(move |conn| {
            let mut stmt = conn.prepare(sql)?;
            let res = stmt.query_map(params, f)?;
            g(res)
        })
        .await
    }

    /// Used for executing `SELECT COUNT` statements only. Returns the resulting count.
    pub async fn count(&self, query: &str, params: impl rusqlite::Params + Send) -> Result<usize> {
        let count: isize = self.query_row(query, params, |row| row.get(0)).await?;
        Ok(usize::try_from(count)?)
    }

    /// Used for executing `SELECT COUNT` statements only. Returns `true`, if the count is at least
    /// one, `false` otherwise.
    pub async fn exists(&self, sql: &str, params: impl rusqlite::Params + Send) -> Result<bool> {
        let count = self.count(sql, params).await?;
        Ok(count > 0)
    }

    /// Execute a query which is expected to return one row.
    pub async fn query_row<T, F>(
        &self,
        query: &str,
        params: impl rusqlite::Params + Send,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T> + Send,
        T: Send + 'static,
    {
        self.call(move |conn| {
            let res = conn.query_row(query, params, f)?;
            Ok(res)
        })
        .await
    }

    /// Execute the function inside a transaction assuming that it does write queries.
    ///
    /// If the function returns an error, the transaction will be rolled back. If it does not return an
    /// error, the transaction will be committed.
    pub async fn transaction<G, H>(&self, callback: G) -> Result<H>
    where
        H: Send + 'static,
        G: Send + FnOnce(&mut rusqlite::Transaction<'_>) -> Result<H>,
    {
        self.call_write(move |conn| {
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
        })
        .await
    }

    /// Query the database if the requested table already exists.
    pub async fn table_exists(&self, name: &str) -> Result<bool> {
        self.call(move |conn| {
            let mut exists = false;
            conn.pragma(None, "table_info", name.to_string(), |_row| {
                // will only be executed if the info was found
                exists = true;
                Ok(())
            })?;

            Ok(exists)
        })
        .await
    }

    /// Check if a column exists in a given table.
    pub async fn col_exists(&self, table_name: &str, col_name: &str) -> Result<bool> {
        self.call(move |conn| {
            let mut exists = false;
            // `PRAGMA table_info` returns one row per column,
            // each row containing 0=cid, 1=name, 2=type, 3=notnull, 4=dflt_value
            conn.pragma(None, "table_info", table_name.to_string(), |row| {
                let curr_name: String = row.get(1)?;
                if col_name == curr_name {
                    exists = true;
                }
                Ok(())
            })?;

            Ok(exists)
        })
        .await
    }

    /// Execute a query which is expected to return zero or one row.
    pub async fn query_row_optional<T, F>(
        &self,
        sql: &str,
        params: impl rusqlite::Params + Send,
        f: F,
    ) -> Result<Option<T>>
    where
        F: Send + FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
        T: Send + 'static,
    {
        self.call(move |conn| match conn.query_row(sql.as_ref(), params, f) {
            Ok(res) => Ok(Some(res)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(rusqlite::Error::InvalidColumnType(_, _, rusqlite::types::Type::Null)) => Ok(None),
            Err(err) => Err(err.into()),
        })
        .await
    }

    /// Executes a query which is expected to return one row and one
    /// column. If the query does not return a value or returns SQL
    /// `NULL`, returns `Ok(None)`.
    pub async fn query_get_value<T>(
        &self,
        query: &str,
        params: impl rusqlite::Params + Send,
    ) -> Result<Option<T>>
    where
        T: rusqlite::types::FromSql + Send + 'static,
    {
        self.query_row_optional(query, params, |row| row.get::<_, T>(0))
            .await
    }

    /// Set private configuration options.
    ///
    /// Setting `None` deletes the value.  On failure an error message
    /// will already have been logged.
    pub async fn set_raw_config(&self, key: &str, value: Option<&str>) -> Result<()> {
        let mut lock = self.config_cache.write().await;
        if let Some(value) = value {
            let exists = self
                .exists("SELECT COUNT(*) FROM config WHERE keyname=?;", (key,))
                .await?;

            if exists {
                self.execute("UPDATE config SET value=? WHERE keyname=?;", (value, key))
                    .await?;
            } else {
                self.execute(
                    "INSERT INTO config (keyname, value) VALUES (?, ?);",
                    (key, value),
                )
                .await?;
            }
        } else {
            self.execute("DELETE FROM config WHERE keyname=?;", (key,))
                .await?;
        }
        lock.insert(key.to_string(), value.map(|s| s.to_string()));
        drop(lock);

        Ok(())
    }

    /// Get configuration options from the database.
    pub async fn get_raw_config(&self, key: &str) -> Result<Option<String>> {
        let lock = self.config_cache.read().await;
        let cached = lock.get(key).cloned();
        drop(lock);

        if let Some(c) = cached {
            return Ok(c);
        }

        let mut lock = self.config_cache.write().await;
        let value = self
            .query_get_value("SELECT value FROM config WHERE keyname=?;", (key,))
            .await
            .context(format!("failed to fetch raw config: {key}"))?;
        lock.insert(key.to_string(), value.clone());
        drop(lock);

        Ok(value)
    }

    /// Sets configuration for the given key to 32-bit signed integer value.
    pub async fn set_raw_config_int(&self, key: &str, value: i32) -> Result<()> {
        self.set_raw_config(key, Some(&format!("{value}"))).await
    }

    /// Returns 32-bit signed integer configuration value for the given key.
    pub async fn get_raw_config_int(&self, key: &str) -> Result<Option<i32>> {
        self.get_raw_config(key)
            .await
            .map(|s| s.and_then(|s| s.parse().ok()))
    }

    /// Returns 32-bit unsigned integer configuration value for the given key.
    pub async fn get_raw_config_u32(&self, key: &str) -> Result<Option<u32>> {
        self.get_raw_config(key)
            .await
            .map(|s| s.and_then(|s| s.parse().ok()))
    }

    /// Returns boolean configuration value for the given key.
    pub async fn get_raw_config_bool(&self, key: &str) -> Result<bool> {
        // Not the most obvious way to encode bool as string, but it is matter
        // of backward compatibility.
        let res = self.get_raw_config_int(key).await?;
        Ok(res.unwrap_or_default() > 0)
    }

    /// Sets configuration for the given key to boolean value.
    pub async fn set_raw_config_bool(&self, key: &str, value: bool) -> Result<()> {
        let value = if value { Some("1") } else { None };
        self.set_raw_config(key, value).await
    }

    /// Sets configuration for the given key to 64-bit signed integer value.
    pub async fn set_raw_config_int64(&self, key: &str, value: i64) -> Result<()> {
        self.set_raw_config(key, Some(&format!("{value}"))).await
    }

    /// Returns 64-bit signed integer configuration value for the given key.
    pub async fn get_raw_config_int64(&self, key: &str) -> Result<Option<i64>> {
        self.get_raw_config(key)
            .await
            .map(|s| s.and_then(|r| r.parse().ok()))
    }

    /// Returns configuration cache.
    #[cfg(feature = "internals")]
    pub fn config_cache(&self) -> &RwLock<HashMap<String, Option<String>>> {
        &self.config_cache
    }
}

/// Creates a new SQLite connection.
///
/// `path` is the database path.
///
/// `passphrase` is the SQLCipher database passphrase.
/// Empty string if database is not encrypted.
fn new_connection(path: &Path, passphrase: &str) -> Result<Connection> {
    let mut flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;
    flags.insert(OpenFlags::SQLITE_OPEN_READ_WRITE);
    flags.insert(OpenFlags::SQLITE_OPEN_CREATE);

    let conn = Connection::open_with_flags(path, flags)?;
    conn.execute_batch(
        "PRAGMA cipher_memory_security = OFF; -- Too slow on Android
         PRAGMA secure_delete=on;
         PRAGMA busy_timeout = 0; -- fail immediately
         PRAGMA temp_store=memory; -- Avoid SQLITE_IOERR_GETTEMPPATH errors on Android
         PRAGMA foreign_keys=on;
         ",
    )?;
    conn.pragma_update(None, "key", passphrase)?;
    // Try to enable auto_vacuum. This will only be
    // applied if the database is new or after successful
    // VACUUM, which usually happens before backup export.
    // When auto_vacuum is INCREMENTAL, it is possible to
    // use PRAGMA incremental_vacuum to return unused
    // database pages to the filesystem.
    conn.pragma_update(None, "auto_vacuum", "INCREMENTAL".to_string())?;

    conn.pragma_update(None, "journal_mode", "WAL".to_string())?;
    // Default synchronous=FULL is much slower. NORMAL is sufficient for WAL mode.
    conn.pragma_update(None, "synchronous", "NORMAL".to_string())?;

    Ok(conn)
}

/// Cleanup the account to restore some storage and optimize the database.
pub async fn housekeeping(context: &Context) -> Result<()> {
    // Setting `Config::LastHousekeeping` at the beginning avoids endless loops when things do not
    // work out for whatever reason or are interrupted by the OS.
    if let Err(e) = context
        .set_config(Config::LastHousekeeping, Some(&time().to_string()))
        .await
    {
        warn!(context, "Can't set config: {e:#}.");
    }

    if let Err(err) = remove_unused_files(context).await {
        warn!(
            context,
            "Housekeeping: cannot remove unused files: {:#}.", err
        );
    }

    if let Err(err) = start_ephemeral_timers(context).await {
        warn!(
            context,
            "Housekeeping: cannot start ephemeral timers: {:#}.", err
        );
    }

    if let Err(err) = prune_tombstones(&context.sql).await {
        warn!(
            context,
            "Housekeeping: Cannot prune message tombstones: {:#}.", err
        );
    }

    if let Err(err) = deduplicate_peerstates(&context.sql).await {
        warn!(context, "Failed to deduplicate peerstates: {:#}.", err)
    }

    context.schedule_quota_update().await?;

    // Try to clear the freelist to free some space on the disk. This
    // only works if auto_vacuum is enabled.
    match context
        .sql
        .query_row_optional("PRAGMA incremental_vacuum", (), |_row| Ok(()))
        .await
    {
        Err(err) => {
            warn!(context, "Failed to run incremental vacuum: {err:#}.");
        }
        Ok(Some(())) => {
            // Incremental vacuum returns a zero-column result if it did anything.
            info!(context, "Successfully ran incremental vacuum.");
        }
        Ok(None) => {
            // Incremental vacuum returned `SQLITE_DONE` immediately,
            // there were no pages to remove.
        }
    }

    context
        .sql
        .execute(
            "DELETE FROM msgs_mdns WHERE msg_id NOT IN (SELECT id FROM msgs)",
            (),
        )
        .await
        .context("failed to remove old MDNs")
        .log_err(context)
        .ok();

    context
        .sql
        .execute(
            "DELETE FROM msgs_status_updates WHERE msg_id NOT IN (SELECT id FROM msgs)",
            (),
        )
        .await
        .context("failed to remove old webxdc status updates")
        .log_err(context)
        .ok();

    context
        .sql
        .execute(
            "DELETE FROM contacts WHERE origin=? AND id NOT IN (SELECT contact_id FROM chats_contacts);",
            (Origin::Hidden,),
        )
        .await
        .context("Failed to remove hidden contacts with no chats")
        .log_err(context)
        .ok();

    info!(context, "Housekeeping done.");
    Ok(())
}

/// Get the value of a column `idx` of the `row` as `Vec<u8>`.
pub fn row_get_vec(row: &Row, idx: usize) -> rusqlite::Result<Vec<u8>> {
    row.get(idx).or_else(|err| match row.get_ref(idx)? {
        ValueRef::Null => Ok(Vec::new()),
        ValueRef::Text(text) => Ok(text.to_vec()),
        ValueRef::Blob(blob) => Ok(blob.to_vec()),
        ValueRef::Integer(_) | ValueRef::Real(_) => Err(err),
    })
}

/// Enumerates used files in the blobdir and removes unused ones.
pub async fn remove_unused_files(context: &Context) -> Result<()> {
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
            (),
            |row| row.get::<_, String>(0),
            |rows| {
                for row in rows {
                    maybe_add_file(&mut files_in_use, &row?);
                }
                Ok(())
            },
        )
        .await
        .context("housekeeping: failed to SELECT value FROM config")?;

    info!(context, "{} files in use.", files_in_use.len());
    /* go through directories and delete unused files */
    let blobdir = context.get_blobdir();
    for p in [&blobdir.join(BLOBS_BACKUP_NAME), blobdir] {
        match tokio::fs::read_dir(p).await {
            Ok(mut dir_handle) => {
                /* avoid deletion of files that are just created to build a message object */
                let diff = std::time::Duration::from_secs(60 * 60);
                let keep_files_newer_than = std::time::SystemTime::now()
                    .checked_sub(diff)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                while let Ok(Some(entry)) = dir_handle.next_entry().await {
                    let name_f = entry.file_name();
                    let name_s = name_f.to_string_lossy();

                    if p == blobdir
                        && (is_file_in_use(&files_in_use, None, &name_s)
                            || is_file_in_use(&files_in_use, Some(".increation"), &name_s)
                            || is_file_in_use(&files_in_use, Some(".waveform"), &name_s)
                            || is_file_in_use(&files_in_use, Some("-preview.jpg"), &name_s))
                    {
                        continue;
                    }

                    if let Ok(stats) = tokio::fs::metadata(entry.path()).await {
                        if stats.is_dir() {
                            if let Err(e) = tokio::fs::remove_dir(entry.path()).await {
                                // The dir could be created not by a user, but by a desktop
                                // environment f.e. So, no warning.
                                info!(
                                    context,
                                    "Housekeeping: Cannot rmdir {}: {:#}.",
                                    entry.path().display(),
                                    e
                                );
                            }
                            continue;
                        }
                        unreferenced_count += 1;
                        let recently_created =
                            stats.created().map_or(false, |t| t > keep_files_newer_than);
                        let recently_modified = stats
                            .modified()
                            .map_or(false, |t| t > keep_files_newer_than);
                        let recently_accessed = stats
                            .accessed()
                            .map_or(false, |t| t > keep_files_newer_than);

                        if p == blobdir
                            && (recently_created || recently_modified || recently_accessed)
                        {
                            info!(
                                context,
                                "Housekeeping: Keeping new unreferenced file #{}: {:?}.",
                                unreferenced_count,
                                entry.file_name(),
                            );
                            continue;
                        }
                    } else {
                        unreferenced_count += 1;
                    }
                    info!(
                        context,
                        "Housekeeping: Deleting unreferenced file #{}: {:?}.",
                        unreferenced_count,
                        entry.file_name()
                    );
                    let path = entry.path();
                    if let Err(err) = delete_file(context, &path).await {
                        error!(
                            context,
                            "Failed to delete unused file {}: {:#}.",
                            path.display(),
                            err
                        );
                    }
                }
            }
            Err(err) => {
                if !p.ends_with(BLOBS_BACKUP_NAME) {
                    warn!(
                        context,
                        "Housekeeping: Cannot read dir {}: {:#}.",
                        p.display(),
                        err
                    );
                }
            }
        }
    }

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

fn maybe_add_file(files_in_use: &mut HashSet<String>, file: &str) {
    if let Some(file) = file.strip_prefix("$BLOBDIR/") {
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
        (),
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
    .context(format!("housekeeping: failed to add_from_param {query}"))?;

    Ok(())
}

/// Removes from the database locally deleted messages that also don't
/// have a server UID.
async fn prune_tombstones(sql: &Sql) -> Result<()> {
    sql.execute(
        "DELETE FROM msgs
         WHERE chat_id=?
         AND NOT EXISTS (
         SELECT * FROM imap WHERE msgs.rfc724_mid=rfc724_mid AND target!=''
         )",
        (DC_CHAT_ID_TRASH,),
    )
    .await?;
    Ok(())
}

/// Helper function to return comma-separated sequence of `?` chars.
///
/// Use this together with [`rusqlite::ParamsFromIter`] to use dynamically generated
/// parameter lists.
pub fn repeat_vars(count: usize) -> String {
    let mut s = "?,".repeat(count);
    s.pop(); // Remove trailing comma
    s
}

#[cfg(test)]
mod tests {
    use async_channel as channel;

    use super::*;
    use crate::config::Config;
    use crate::{test_utils::TestContext, EventType};

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_table_exists() {
        let t = TestContext::new().await;
        assert!(t.ctx.sql.table_exists("msgs").await.unwrap());
        assert!(!t.ctx.sql.table_exists("foobar").await.unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_col_exists() {
        let t = TestContext::new().await;
        assert!(t.ctx.sql.col_exists("msgs", "mime_modified").await.unwrap());
        assert!(!t.ctx.sql.col_exists("msgs", "foobar").await.unwrap());
        assert!(!t.ctx.sql.col_exists("foobar", "foobar").await.unwrap());
    }

    /// Tests that auto_vacuum is enabled for new databases.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_auto_vacuum() -> Result<()> {
        let t = TestContext::new().await;

        let auto_vacuum = t
            .sql
            .call(|conn| {
                let auto_vacuum = conn.pragma_query_value(None, "auto_vacuum", |row| {
                    let auto_vacuum: i32 = row.get(0)?;
                    Ok(auto_vacuum)
                })?;
                Ok(auto_vacuum)
            })
            .await?;

        // auto_vacuum=2 is the same as auto_vacuum=INCREMENTAL
        assert_eq!(auto_vacuum, 2);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_housekeeping_db_closed() {
        let t = TestContext::new().await;

        let avatar_src = t.dir.path().join("avatar.png");
        let avatar_bytes = include_bytes!("../test-data/image/avatar64x64.png");
        tokio::fs::write(&avatar_src, avatar_bytes).await.unwrap();
        t.set_config(Config::Selfavatar, Some(avatar_src.to_str().unwrap()))
            .await
            .unwrap();

        let (event_sink, event_source) = channel::unbounded();
        t.add_event_sender(event_sink).await;

        let a = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        assert_eq!(avatar_bytes, &tokio::fs::read(&a).await.unwrap()[..]);

        t.sql.close().await;
        housekeeping(&t).await.unwrap(); // housekeeping should emit warnings but not fail
        t.sql.open(&t, "".to_string()).await.unwrap();

        let a = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        assert_eq!(avatar_bytes, &tokio::fs::read(&a).await.unwrap()[..]);

        while let Ok(event) = event_source.try_recv() {
            match event.typ {
                EventType::Info(s) => assert!(
                    !s.contains("Keeping new unreferenced file"),
                    "File {s} was almost deleted, only reason it was kept is that it was created recently (as the tests don't run for a long time)"
                ),
                EventType::Error(s) => panic!("{}", s),
                _ => {}
            }
        }
    }

    /// Regression test for a bug where housekeeping deleted drafts since their
    /// `hidden` flag is set.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_housekeeping_dont_delete_drafts() {
        let t = TestContext::new_alice().await;

        let chat = t.create_chat_with_contact("bob", "bob@example.com").await;
        let mut new_draft = Message::new(Viewtype::Text);
        new_draft.set_text("This is my draft".to_string());
        chat.id.set_draft(&t, Some(&mut new_draft)).await.unwrap();

        housekeeping(&t).await.unwrap();

        let loaded_draft = chat.id.get_draft(&t).await.unwrap();
        assert_eq!(loaded_draft.unwrap().text, "This is my draft");
    }

    /// Tests that `housekeeping` deletes the blobs backup dir which is created normally by
    /// `imex::import_backup`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_housekeeping_delete_blobs_backup_dir() {
        let t = TestContext::new_alice().await;
        let dir = t.get_blobdir().join(BLOBS_BACKUP_NAME);
        tokio::fs::create_dir(&dir).await.unwrap();
        tokio::fs::write(dir.join("f"), "").await.unwrap();
        housekeeping(&t).await.unwrap();
        tokio::fs::create_dir(&dir).await.unwrap();
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
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_db_reopen() -> Result<()> {
        use tempfile::tempdir;

        // The context is used only for logging.
        let t = TestContext::new().await;

        // Create a separate empty database for testing.
        let dir = tempdir()?;
        let dbfile = dir.path().join("testdb.sqlite");
        let sql = Sql::new(dbfile);

        // Create database with all the tables.
        sql.open(&t, "".to_string()).await.unwrap();
        sql.close().await;

        // Reopen the database
        sql.open(&t, "".to_string()).await?;
        sql.execute(
            "INSERT INTO config (keyname, value) VALUES (?, ?);",
            ("foo", "bar"),
        )
        .await?;

        let value: Option<String> = sql
            .query_get_value("SELECT value FROM config WHERE keyname=?;", ("foo",))
            .await?;
        assert_eq!(value.unwrap(), "bar");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_check_passphrase() -> Result<()> {
        use tempfile::tempdir;

        // The context is used only for logging.
        let t = TestContext::new().await;

        // Create a separate empty database for testing.
        let dir = tempdir()?;
        let dbfile = dir.path().join("testdb.sqlite");
        let sql = Sql::new(dbfile.clone());

        sql.check_passphrase("foo".to_string()).await?;
        sql.open(&t, "foo".to_string())
            .await
            .context("failed to open the database first time")?;
        sql.close().await;

        // Reopen the database
        let sql = Sql::new(dbfile);

        // Test that we can't open encrypted database without a passphrase.
        assert!(sql.open(&t, "".to_string()).await.is_err());

        // Now open the database with passpharse, it should succeed.
        sql.check_passphrase("foo".to_string()).await?;
        sql.open(&t, "foo".to_string())
            .await
            .context("failed to open the database second time")?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sql_change_passphrase() -> Result<()> {
        use tempfile::tempdir;

        // The context is used only for logging.
        let t = TestContext::new().await;

        // Create a separate empty database for testing.
        let dir = tempdir()?;
        let dbfile = dir.path().join("testdb.sqlite");
        let sql = Sql::new(dbfile.clone());

        sql.open(&t, "foo".to_string())
            .await
            .context("failed to open the database first time")?;
        sql.close().await;

        // Change the passphrase from "foo" to "bar".
        let sql = Sql::new(dbfile.clone());
        sql.open(&t, "foo".to_string())
            .await
            .context("failed to open the database second time")?;
        sql.change_passphrase("bar".to_string())
            .await
            .context("failed to change passphrase")?;

        // Test that at least two connections are still working.
        // This ensures that not only the connection which changed the password is working,
        // but other connections as well.
        {
            let lock = sql.pool.read().await;
            let pool = lock.as_ref().unwrap();
            let conn1 = pool.get().await?;
            let conn2 = pool.get().await?;
            conn1
                .query_row("SELECT count(*) FROM sqlite_master", [], |_row| Ok(()))
                .unwrap();
            conn2
                .query_row("SELECT count(*) FROM sqlite_master", [], |_row| Ok(()))
                .unwrap();
        }

        sql.close().await;

        let sql = Sql::new(dbfile);

        // Test that old passphrase is not working.
        assert!(sql.open(&t, "foo".to_string()).await.is_err());

        // Open the database with the new passphrase.
        sql.check_passphrase("bar".to_string()).await?;
        sql.open(&t, "bar".to_string())
            .await
            .context("failed to open the database third time")?;
        sql.close().await;

        Ok(())
    }
}

//! # SQLite wrapper

use async_std::prelude::*;
use async_std::sync::RwLock;

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, Error as SqlError, OpenFlags};

use crate::chat::{update_device_icon, update_saved_messages_icon};
use crate::constants::DC_CHAT_ID_TRASH;
use crate::context::Context;
use crate::dc_tools::*;
use crate::param::*;
use crate::peerstate::*;

#[macro_export]
macro_rules! paramsv {
    () => {
        Vec::new()
    };
    ($($param:expr),+ $(,)?) => {
        vec![$(&$param as &dyn $crate::ToSql),+]
    };
}

mod migrations;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sqlite Error: {0:?}")]
    Sql(#[from] rusqlite::Error),
    #[error("Sqlite Connection Pool Error: {0:?}")]
    ConnectionPool(#[from] r2d2::Error),
    #[error("Sqlite: Connection closed")]
    SqlNoConnection,
    #[error("Sqlite: Already open")]
    SqlAlreadyOpen,
    #[error("Sqlite: Failed to open")]
    SqlFailedToOpen,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0:?}")]
    BlobError(#[from] crate::blob::BlobError),
    #[error("{0}")]
    Other(#[from] crate::error::Error),
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A wrapper around the underlying Sqlite3 object.
#[derive(Debug)]
pub struct Sql {
    pool: RwLock<Option<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>,
    xpool: RwLock<Option<sqlx::SqlitePool>>,
}

impl Default for Sql {
    fn default() -> Self {
        Self {
            pool: RwLock::new(None),
            xpool: RwLock::new(None),
        }
    }
}

impl Sql {
    pub fn new() -> Sql {
        Self::default()
    }

    pub async fn is_open(&self) -> bool {
        self.pool.read().await.is_some() && self.xpool.read().await.is_some()
    }

    pub async fn close(&self) {
        let _ = self.pool.write().await.take();
        let _ = self.xpool.write().await.take();
        // drop closes the connection
    }

    pub async fn open<T: AsRef<Path>>(
        &self,
        context: &Context,
        dbfile: T,
        readonly: bool,
    ) -> crate::error::Result<()> {
        if let Err(err) = open(context, self, dbfile, readonly).await {
            return match err.downcast_ref::<Error>() {
                Some(Error::SqlAlreadyOpen) => Err(err),
                _ => {
                    self.close().await;
                    Err(err)
                }
            };
        }

        Ok(())
    }

    pub async fn execute<S: AsRef<str>>(
        &self,
        sql: S,
        params: Vec<&dyn crate::ToSql>,
    ) -> Result<usize> {
        let res = {
            let conn = self.get_conn().await?;
            conn.execute(sql.as_ref(), params)
        };

        res.map_err(Into::into)
    }

    pub async fn xexecute<S: AsRef<str>>(
        &self,
        statement: S,
        params: sqlx::sqlite::SqliteArguments,
    ) -> Result<()> {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;

        sqlx::query(statement.as_ref())
            .bind_all(params)
            .execute(xpool)
            .await?;

        Ok(())
    }

    /// Execute a list of statements, without any bindings
    pub async fn xexecute_batch<S: AsRef<str>>(&self, statement: S) -> Result<()> {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;

        sqlx::query(statement.as_ref()).execute(xpool).await?;

        Ok(())
    }

    pub async fn execute_batch<S: AsRef<str>>(&self, sql: S) -> Result<()> {
        let res = {
            let conn = self.get_conn().await?;
            conn.execute_batch(sql.as_ref())
        };

        res.map_err(Into::into)
    }
    /// Prepares and executes the statement and maps a function over the resulting rows.
    /// Then executes the second function over the returned iterator and returns the
    /// result of that function.
    pub async fn query_map<T, F, G, H>(
        &self,
        sql: impl AsRef<str>,
        params: Vec<&dyn crate::ToSql>,
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
        let res = stmt.query_map(&params, f)?;
        g(res)
    }

    pub async fn get_conn(
        &self,
    ) -> Result<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>> {
        let lock = self.pool.read().await;
        let pool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;
        let conn = pool.get()?;

        Ok(conn)
    }

    pub async fn with_conn<G, H>(&self, g: G) -> Result<H>
    where
        H: Send + 'static,
        G: Send
            + 'static
            + FnOnce(r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>) -> Result<H>,
    {
        let lock = self.pool.read().await;
        let pool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;
        let conn = pool.get()?;

        g(conn)
    }

    pub async fn with_conn_async<G, H, Fut>(&self, mut g: G) -> Result<H>
    where
        G: FnMut(r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>) -> Fut,
        Fut: Future<Output = Result<H>> + Send,
    {
        let lock = self.pool.read().await;
        let pool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;

        let conn = pool.get()?;
        g(conn).await
    }

    /// Return `true` if a query in the SQL statement it executes returns one or more
    /// rows and false if the SQL returns an empty set.
    pub async fn exists(&self, sql: &str, params: Vec<&dyn crate::ToSql>) -> Result<bool> {
        let res = {
            let conn = self.get_conn().await?;
            let mut stmt = conn.prepare(sql)?;
            stmt.exists(&params)
        };

        res.map_err(Into::into)
    }

    /// Execute a query which is expected to return one row.
    pub async fn query_row<T, F>(
        &self,
        sql: impl AsRef<str>,
        params: Vec<&dyn crate::ToSql>,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let sql = sql.as_ref();
        let res = {
            let conn = self.get_conn().await?;
            conn.query_row(sql, params, f)
        };

        res.map_err(Into::into)
    }

    pub async fn table_exists(&self, name: impl AsRef<str>) -> Result<bool> {
        let name = name.as_ref().to_string();
        self.with_conn(move |conn| {
            let mut exists = false;
            conn.pragma(None, "table_info", &name, |_row| {
                // will only be executed if the info was found
                exists = true;
                Ok(())
            })?;

            Ok(exists)
        })
        .await
    }

    /// Execute a query which is expected to return zero or one row.
    pub async fn query_row_optional<T, F>(
        &self,
        sql: impl AsRef<str>,
        params: Vec<&dyn crate::ToSql>,
        f: F,
    ) -> Result<Option<T>>
    where
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        match self.query_row(sql, params, f).await {
            Ok(res) => Ok(Some(res)),
            Err(Error::Sql(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
            Err(Error::Sql(rusqlite::Error::InvalidColumnType(
                _,
                _,
                rusqlite::types::Type::Null,
            ))) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Executes a query which is expected to return one row and one
    /// column. If the query does not return a value or returns SQL
    /// `NULL`, returns `Ok(None)`.
    pub async fn query_get_value_result<T>(
        &self,
        query: &str,
        params: Vec<&dyn crate::ToSql>,
    ) -> Result<Option<T>>
    where
        T: rusqlite::types::FromSql,
    {
        self.query_row_optional(query, params, |row| row.get::<_, T>(0))
            .await
    }

    /// Not resultified version of `query_get_value_result`. Returns
    /// `None` on error.
    pub async fn query_get_value<T>(
        &self,
        context: &Context,
        query: &str,
        params: Vec<&dyn crate::ToSql>,
    ) -> Option<T>
    where
        T: rusqlite::types::FromSql,
    {
        match self.query_get_value_result(query, params).await {
            Ok(res) => res,
            Err(err) => {
                warn!(context, "sql: Failed query_row: {}", err);
                None
            }
        }
    }

    /// Set private configuration options.
    ///
    /// Setting `None` deletes the value.  On failure an error message
    /// will already have been logged.
    pub async fn set_raw_config(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: Option<&str>,
    ) -> Result<()> {
        if !self.is_open().await {
            error!(context, "set_raw_config(): Database not ready.");
            return Err(Error::SqlNoConnection);
        }

        let key = key.as_ref();
        let res = if let Some(ref value) = value {
            let exists = self
                .exists("SELECT value FROM config WHERE keyname=?;", paramsv![key])
                .await?;
            if exists {
                self.execute(
                    "UPDATE config SET value=? WHERE keyname=?;",
                    paramsv![(*value).to_string(), key.to_string()],
                )
                .await
            } else {
                self.execute(
                    "INSERT INTO config (keyname, value) VALUES (?, ?);",
                    paramsv![key.to_string(), (*value).to_string()],
                )
                .await
            }
        } else {
            self.execute("DELETE FROM config WHERE keyname=?;", paramsv![key])
                .await
        };

        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(context, "set_raw_config(): Cannot change value. {:?}", &err);
                Err(err)
            }
        }
    }

    /// Get configuration options from the database.
    pub async fn get_raw_config(&self, context: &Context, key: impl AsRef<str>) -> Option<String> {
        if !self.is_open().await || key.as_ref().is_empty() {
            return None;
        }
        self.query_get_value(
            context,
            "SELECT value FROM config WHERE keyname=?;",
            paramsv![key.as_ref().to_string()],
        )
        .await
    }

    pub async fn set_raw_config_int(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: i32,
    ) -> Result<()> {
        self.set_raw_config(context, key, Some(&format!("{}", value)))
            .await
    }

    pub async fn get_raw_config_int(&self, context: &Context, key: impl AsRef<str>) -> Option<i32> {
        self.get_raw_config(context, key)
            .await
            .and_then(|s| s.parse().ok())
    }

    pub async fn get_raw_config_bool(&self, context: &Context, key: impl AsRef<str>) -> bool {
        // Not the most obvious way to encode bool as string, but it is matter
        // of backward compatibility.
        let res = self.get_raw_config_int(context, key).await;
        res.unwrap_or_default() > 0
    }

    pub async fn set_raw_config_bool<T>(&self, context: &Context, key: T, value: bool) -> Result<()>
    where
        T: AsRef<str>,
    {
        let value = if value { Some("1") } else { None };
        self.set_raw_config(context, key, value).await
    }

    pub async fn set_raw_config_int64(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: i64,
    ) -> Result<()> {
        self.set_raw_config(context, key, Some(&format!("{}", value)))
            .await
    }

    pub async fn get_raw_config_int64(
        &self,
        context: &Context,
        key: impl AsRef<str>,
    ) -> Option<i64> {
        self.get_raw_config(context, key)
            .await
            .and_then(|r| r.parse().ok())
    }

    /// Alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    /// the ORDER BY ensures, this function always returns the most recent id,
    /// eg. if a Message-ID is split into different messages.
    pub async fn get_rowid(
        &self,
        _context: &Context,
        table: impl AsRef<str>,
        field: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<u32> {
        let res = {
            let mut conn = self.get_conn().await?;
            get_rowid(&mut conn, table, field, value)
        };

        res.map_err(Into::into)
    }

    pub async fn get_rowid2(
        &self,
        _context: &Context,
        table: impl AsRef<str>,
        field: impl AsRef<str>,
        value: i64,
        field2: impl AsRef<str>,
        value2: i32,
    ) -> Result<u32> {
        let res = {
            let mut conn = self.get_conn().await?;
            get_rowid2(&mut conn, table, field, value, field2, value2)
        };

        res.map_err(Into::into)
    }
}

pub fn get_rowid(
    conn: &mut Connection,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: impl AsRef<str>,
) -> std::result::Result<u32, SqlError> {
    // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    // the ORDER BY ensures, this function always returns the most recent id,
    // eg. if a Message-ID is split into different messages.
    let query = format!(
        "SELECT id FROM {} WHERE {}=? ORDER BY id DESC",
        table.as_ref(),
        field.as_ref(),
    );

    conn.query_row(&query, params![value.as_ref()], |row| row.get::<_, u32>(0))
}

pub fn get_rowid2(
    conn: &mut Connection,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: i64,
    field2: impl AsRef<str>,
    value2: i32,
) -> std::result::Result<u32, SqlError> {
    conn.query_row(
        &format!(
            "SELECT id FROM {} WHERE {}={} AND {}={} ORDER BY id DESC",
            table.as_ref(),
            field.as_ref(),
            value,
            field2.as_ref(),
            value2,
        ),
        params![],
        |row| row.get::<_, u32>(0),
    )
}

pub async fn housekeeping(context: &Context) {
    let mut files_in_use = HashSet::new();
    let mut unreferenced_count = 0;

    info!(context, "Start housekeeping...");
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM msgs  WHERE chat_id!=3   AND type!=10;",
        Param::File,
    )
    .await;
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM jobs;",
        Param::File,
    )
    .await;
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM chats;",
        Param::ProfileImage,
    )
    .await;
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM contacts;",
        Param::ProfileImage,
    )
    .await;

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
        .unwrap_or_else(|err| {
            warn!(context, "sql: failed query: {}", err);
        });

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

    if let Err(err) = prune_tombstones(context).await {
        warn!(
            context,
            "Houskeeping: Cannot prune message tombstones: {}", err
        );
    }

    info!(context, "Housekeeping done.",);
}

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
    if !file.as_ref().starts_with("$BLOBDIR/") {
        return;
    }

    files_in_use.insert(file.as_ref()[9..].into());
}

async fn maybe_add_from_param(
    context: &Context,
    files_in_use: &mut HashSet<String>,
    query: &str,
    param_id: Param,
) {
    context
        .sql
        .query_map(
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
        .unwrap_or_else(|err| {
            warn!(context, "sql: failed to add_from_param: {}", err);
        });
}

#[allow(clippy::cognitive_complexity)]
async fn open(
    context: &Context,
    sql: &Sql,
    dbfile: impl AsRef<Path>,
    readonly: bool,
) -> crate::error::Result<()> {
    if sql.is_open().await {
        error!(
            context,
            "Cannot open, database \"{:?}\" already opened.",
            dbfile.as_ref(),
        );
        return Err(Error::SqlAlreadyOpen.into());
    }

    let mut open_flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if readonly {
        open_flags.insert(OpenFlags::SQLITE_OPEN_READ_ONLY);
    } else {
        open_flags.insert(OpenFlags::SQLITE_OPEN_READ_WRITE);
        open_flags.insert(OpenFlags::SQLITE_OPEN_CREATE);
    }

    // this actually creates min_idle database handles just now.
    // therefore, with_init() must not try to modify the database as otherwise
    // we easily get busy-errors (eg. table-creation, journal_mode etc. should be done on only one handle)
    let mgr = r2d2_sqlite::SqliteConnectionManager::file(dbfile.as_ref())
        .with_flags(open_flags)
        .with_init(|c| {
            c.execute_batch(&format!(
                "PRAGMA secure_delete=on; PRAGMA busy_timeout = {};",
                Duration::from_secs(10).as_millis()
            ))?;
            Ok(())
        });
    let pool = r2d2::Pool::builder()
        .min_idle(Some(2))
        .max_size(10)
        .connection_timeout(Duration::from_secs(60))
        .build(mgr)
        .map_err(Error::ConnectionPool)?;

    {
        *sql.pool.write().await = Some(pool);
    }

    let xpool = sqlx::SqlitePool::builder()
        .min_size(1)
        .max_size(1)
        .build(&format!("sqlite://{}", dbfile.as_ref().to_string_lossy()))
        .await?;

    {
        *sql.xpool.write().await = Some(xpool)
    }

    if !readonly {
        // journal_mode is persisted, it is sufficient to change it only for one handle.
        // (nb: execute() always returns errors for this PRAGMA call, just discard it.
        // but even if execute() would handle errors more gracefully, we should continue on errors -
        // systems might not be able to handle WAL, in which case the standard-journal is used.
        // that may be not optimal, but better than not working at all :)
        sql.execute("PRAGMA journal_mode=WAL;", paramsv![])
            .await
            .ok();

        let mut exists_before_update = false;
        let mut dbversion_before_update: i32 = -1;

        if sql.table_exists("config").await? {
            exists_before_update = true;
            dbversion_before_update = sql
                .get_raw_config_int(context, "dbversion")
                .await
                .unwrap_or_default();
        }

        // (1) update low-level database structure.
        // this should be done before updates that use high-level objects that
        // rely themselves on the low-level structure.
        // --------------------------------------------------------------------

        migrations::run(context, &sql, dbversion_before_update, exists_before_update).await?;

        // general updates
        // (2) updates that require high-level objects
        // (the structure is complete now and all objects are usable)
        // --------------------------------------------------------------------
        let mut recalc_fingerprints = false;
        let mut update_icons = false;

        if dbversion_before_update < 34 {
            recalc_fingerprints = true;
        }

        if dbversion_before_update < 61 {
            update_icons = true;
        }

        if recalc_fingerprints {
            info!(context, "[migration] recalc fingerprints");
            let addrs = sql
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
                if let Some(ref mut peerstate) = Peerstate::from_addr(context, addr).await {
                    peerstate.recalc_fingerprint();
                    peerstate.save_to_db(sql, false).await?;
                }
            }
        }
        if update_icons {
            update_saved_messages_icon(context).await?;
            update_device_icon(context).await?;
        }
    }

    info!(context, "Opened {:?}.", dbfile.as_ref(),);

    Ok(())
}

/// Removes from the database locally deleted messages that also don't
/// have a server UID.
async fn prune_tombstones(context: &Context) -> Result<()> {
    context
        .sql
        .execute(
            "DELETE FROM msgs \
         WHERE (chat_id = ? OR hidden) \
         AND server_uid = 0",
            paramsv![DC_CHAT_ID_TRASH],
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
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
}

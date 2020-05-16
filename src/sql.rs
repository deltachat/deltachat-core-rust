//! # SQLite wrapper

use async_std::prelude::*;
use async_std::sync::RwLock;

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, Error as SqlError, OpenFlags};

use crate::chat::{update_device_icon, update_saved_messages_icon};
use crate::constants::{ShowEmails, DC_CHAT_ID_TRASH};
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
}

pub type Result<T> = std::result::Result<T, Error>;

/// A wrapper around the underlying Sqlite3 object.
#[derive(Debug)]
pub struct Sql {
    pool: RwLock<Option<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>,
}

impl Default for Sql {
    fn default() -> Self {
        Self {
            pool: RwLock::new(None),
        }
    }
}

impl Sql {
    pub fn new() -> Sql {
        Self::default()
    }

    pub async fn is_open(&self) -> bool {
        self.pool.read().await.is_some()
    }

    pub async fn close(&self) {
        let _ = self.pool.write().await.take();
        // drop closes the connection
    }

    // return true on success, false on failure
    pub async fn open<T: AsRef<Path>>(&self, context: &Context, dbfile: T, readonly: bool) -> bool {
        match open(context, self, dbfile, readonly).await {
            Ok(_) => true,
            Err(err) => match err.downcast_ref::<Error>() {
                Some(Error::SqlAlreadyOpen) => false,
                _ => {
                    self.close().await;
                    false
                }
            },
        }
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
    if !file.as_ref().starts_with("$BLOBDIR") {
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
        let mut dbversion_before_update: i32 = 0;
        /* Init tables to dbversion=0 */
        if !sql.table_exists("config").await? {
            info!(
                context,
                "First time init: creating tables in {:?}.",
                dbfile.as_ref(),
            );
            sql.execute(
                "CREATE TABLE config (id INTEGER PRIMARY KEY, keyname TEXT, value TEXT);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX config_index1 ON config (keyname);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE TABLE contacts (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 name TEXT DEFAULT '', \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 origin INTEGER DEFAULT 0, \
                 blocked INTEGER DEFAULT 0, \
                 last_seen INTEGER DEFAULT 0, \
                 param TEXT DEFAULT '');",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX contacts_index1 ON contacts (name COLLATE NOCASE);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX contacts_index2 ON contacts (addr COLLATE NOCASE);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "INSERT INTO contacts (id,name,origin) VALUES \
                 (1,'self',262144), (2,'info',262144), (3,'rsvd',262144), \
                 (4,'rsvd',262144), (5,'device',262144), (6,'rsvd',262144), \
                 (7,'rsvd',262144), (8,'rsvd',262144), (9,'rsvd',262144);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE TABLE chats (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT,  \
                 type INTEGER DEFAULT 0, \
                 name TEXT DEFAULT '', \
                 draft_timestamp INTEGER DEFAULT 0, \
                 draft_txt TEXT DEFAULT '', \
                 blocked INTEGER DEFAULT 0, \
                 grpid TEXT DEFAULT '', \
                 param TEXT DEFAULT '');",
                paramsv![],
            )
            .await?;
            sql.execute("CREATE INDEX chats_index1 ON chats (grpid);", paramsv![])
                .await?;
            sql.execute(
                "CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "INSERT INTO chats (id,type,name) VALUES \
                 (1,120,'deaddrop'), (2,120,'rsvd'), (3,120,'trash'), \
                 (4,120,'msgs_in_creation'), (5,120,'starred'), (6,120,'archivedlink'), \
                 (7,100,'rsvd'), (8,100,'rsvd'), (9,100,'rsvd');",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE TABLE msgs (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 rfc724_mid TEXT DEFAULT '', \
                 server_folder TEXT DEFAULT '', \
                 server_uid INTEGER DEFAULT 0, \
                 chat_id INTEGER DEFAULT 0, \
                 from_id INTEGER DEFAULT 0, \
                 to_id INTEGER DEFAULT 0, \
                 timestamp INTEGER DEFAULT 0, \
                 type INTEGER DEFAULT 0, \
                 state INTEGER DEFAULT 0, \
                 msgrmsg INTEGER DEFAULT 1, \
                 bytes INTEGER DEFAULT 0, \
                 txt TEXT DEFAULT '', \
                 txt_raw TEXT DEFAULT '', \
                 param TEXT DEFAULT '');",
                paramsv![],
            )
            .await?;
            sql.execute("CREATE INDEX msgs_index1 ON msgs (rfc724_mid);", paramsv![])
                .await?;
            sql.execute("CREATE INDEX msgs_index2 ON msgs (chat_id);", paramsv![])
                .await?;
            sql.execute("CREATE INDEX msgs_index3 ON msgs (timestamp);", paramsv![])
                .await?;
            sql.execute("CREATE INDEX msgs_index4 ON msgs (state);", paramsv![])
                .await?;
            sql.execute(
                "INSERT INTO msgs (id,msgrmsg,txt) VALUES \
                 (1,0,'marker1'), (2,0,'rsvd'), (3,0,'rsvd'), \
                 (4,0,'rsvd'), (5,0,'rsvd'), (6,0,'rsvd'), (7,0,'rsvd'), \
                 (8,0,'rsvd'), (9,0,'daymarker');",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE TABLE jobs (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 added_timestamp INTEGER, \
                 desired_timestamp INTEGER DEFAULT 0, \
                 action INTEGER, \
                 foreign_id INTEGER, \
                 param TEXT DEFAULT '');",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX jobs_index1 ON jobs (desired_timestamp);",
                paramsv![],
            )
            .await?;
            if !sql.table_exists("config").await?
                || !sql.table_exists("contacts").await?
                || !sql.table_exists("chats").await?
                || !sql.table_exists("chats_contacts").await?
                || !sql.table_exists("msgs").await?
                || !sql.table_exists("jobs").await?
            {
                error!(
                    context,
                    "Cannot create tables in new database \"{:?}\".",
                    dbfile.as_ref(),
                );
                // cannot create the tables - maybe we cannot write?
                return Err(Error::SqlFailedToOpen.into());
            } else {
                sql.set_raw_config_int(context, "dbversion", 0).await?;
            }
        } else {
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

        let mut dbversion = dbversion_before_update;
        let mut recalc_fingerprints = false;
        let mut update_icons = false;

        if dbversion < 1 {
            info!(context, "[migration] v1");
            sql.execute(
                "CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT '');",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX leftgrps_index1 ON leftgrps (grpid);",
                paramsv![],
            )
            .await?;
            dbversion = 1;
            sql.set_raw_config_int(context, "dbversion", 1).await?;
        }
        if dbversion < 2 {
            info!(context, "[migration] v2");
            sql.execute(
                "ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT '';",
                paramsv![],
            )
            .await?;
            dbversion = 2;
            sql.set_raw_config_int(context, "dbversion", 2).await?;
        }
        if dbversion < 7 {
            info!(context, "[migration] v7");
            sql.execute(
                "CREATE TABLE keypairs (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 is_default INTEGER DEFAULT 0, \
                 private_key, \
                 public_key, \
                 created INTEGER DEFAULT 0);",
                paramsv![],
            )
            .await?;
            dbversion = 7;
            sql.set_raw_config_int(context, "dbversion", 7).await?;
        }
        if dbversion < 10 {
            info!(context, "[migration] v10");
            sql.execute(
                "CREATE TABLE acpeerstates (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 last_seen INTEGER DEFAULT 0, \
                 last_seen_autocrypt INTEGER DEFAULT 0, \
                 public_key, \
                 prefer_encrypted INTEGER DEFAULT 0);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);",
                paramsv![],
            )
            .await?;
            dbversion = 10;
            sql.set_raw_config_int(context, "dbversion", 10).await?;
        }
        if dbversion < 12 {
            info!(context, "[migration] v12");
            sql.execute(
                "CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);",
                paramsv![],
            )
            .await?;
            dbversion = 12;
            sql.set_raw_config_int(context, "dbversion", 12).await?;
        }
        if dbversion < 17 {
            info!(context, "[migration] v17");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute("CREATE INDEX chats_index2 ON chats (archived);", paramsv![])
                .await?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute("CREATE INDEX msgs_index5 ON msgs (starred);", paramsv![])
                .await?;
            dbversion = 17;
            sql.set_raw_config_int(context, "dbversion", 17).await?;
        }
        if dbversion < 18 {
            info!(context, "[migration] v18");
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN gossip_key;",
                paramsv![],
            )
            .await?;
            dbversion = 18;
            sql.set_raw_config_int(context, "dbversion", 18).await?;
        }
        if dbversion < 27 {
            info!(context, "[migration] v27");
            // chat.id=1 and chat.id=2 are the old deaddrops,
            // the current ones are defined by chats.blocked=2
            sql.execute("DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;", paramsv![])
                .await?;
            sql.execute(
                "CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            dbversion = 27;
            sql.set_raw_config_int(context, "dbversion", 27).await?;
        }
        if dbversion < 34 {
            info!(context, "[migration] v34");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT '';",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT '';",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);",
                paramsv![],
            )
            .await?;
            recalc_fingerprints = true;
            dbversion = 34;
            sql.set_raw_config_int(context, "dbversion", 34).await?;
        }
        if dbversion < 39 {
            info!(context, "[migration] v39");
            sql.execute(
                "CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT '', timestamp INTEGER DEFAULT 0);",
                paramsv![]
            ).await?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN verified_key;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT '';",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);",
                paramsv![],
            )
            .await?;
            dbversion = 39;
            sql.set_raw_config_int(context, "dbversion", 39).await?;
        }
        if dbversion < 40 {
            info!(context, "[migration] v40");
            sql.execute(
                "ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            dbversion = 40;
            sql.set_raw_config_int(context, "dbversion", 40).await?;
        }
        if dbversion < 44 {
            info!(context, "[migration] v44");
            sql.execute("ALTER TABLE msgs ADD COLUMN mime_headers TEXT;", paramsv![])
                .await?;
            dbversion = 44;
            sql.set_raw_config_int(context, "dbversion", 44).await?;
        }
        if dbversion < 46 {
            info!(context, "[migration] v46");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN mime_references TEXT;",
                paramsv![],
            )
            .await?;
            dbversion = 46;
            sql.set_raw_config_int(context, "dbversion", 46).await?;
        }
        if dbversion < 47 {
            info!(context, "[migration] v47");
            sql.execute(
                "ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            dbversion = 47;
            sql.set_raw_config_int(context, "dbversion", 47).await?;
        }
        if dbversion < 48 {
            info!(context, "[migration] v48");
            // NOTE: move_state is not used anymore
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;",
                paramsv![],
            )
            .await?;

            dbversion = 48;
            sql.set_raw_config_int(context, "dbversion", 48).await?;
        }
        if dbversion < 49 {
            info!(context, "[migration] v49");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            dbversion = 49;
            sql.set_raw_config_int(context, "dbversion", 49).await?;
        }
        if dbversion < 50 {
            info!(context, "[migration] v50");
            // installations <= 0.100.1 used DC_SHOW_EMAILS_ALL implicitly;
            // keep this default and use DC_SHOW_EMAILS_NO
            // only for new installations
            if exists_before_update {
                sql.set_raw_config_int(context, "show_emails", ShowEmails::All as i32)
                    .await?;
            }
            dbversion = 50;
            sql.set_raw_config_int(context, "dbversion", 50).await?;
        }
        if dbversion < 53 {
            info!(context, "[migration] v53");
            // the messages containing _only_ locations
            // are also added to the database as _hidden_.
            sql.execute(
                "CREATE TABLE locations ( id INTEGER PRIMARY KEY AUTOINCREMENT, latitude REAL DEFAULT 0.0, longitude REAL DEFAULT 0.0, accuracy REAL DEFAULT 0.0, timestamp INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0);",
                paramsv![]
            ).await?;
            sql.execute(
                "CREATE INDEX locations_index1 ON locations (from_id);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX locations_index2 ON locations (timestamp);",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX chats_index3 ON chats (locations_send_until);",
                paramsv![],
            )
            .await?;
            dbversion = 53;
            sql.set_raw_config_int(context, "dbversion", 53).await?;
        }
        if dbversion < 54 {
            info!(context, "[migration] v54");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.execute(
                "CREATE INDEX msgs_index6 ON msgs (location_id);",
                paramsv![],
            )
            .await?;
            dbversion = 54;
            sql.set_raw_config_int(context, "dbversion", 54).await?;
        }
        if dbversion < 55 {
            info!(context, "[migration] v55");
            sql.execute(
                "ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.set_raw_config_int(context, "dbversion", 55).await?;
        }
        if dbversion < 59 {
            info!(context, "[migration] v59");
            // records in the devmsglabels are kept when the message is deleted.
            // so, msg_id may or may not exist.
            sql.execute(
                "CREATE TABLE devmsglabels (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT, msg_id INTEGER DEFAULT 0);",
                paramsv![],
            ).await?;
            sql.execute(
                "CREATE INDEX devmsglabels_index1 ON devmsglabels (label);",
                paramsv![],
            )
            .await?;
            if exists_before_update && sql.get_raw_config_int(context, "bcc_self").await.is_none() {
                sql.set_raw_config_int(context, "bcc_self", 1).await?;
            }
            sql.set_raw_config_int(context, "dbversion", 59).await?;
        }
        if dbversion < 60 {
            info!(context, "[migration] v60");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN created_timestamp INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.set_raw_config_int(context, "dbversion", 60).await?;
        }
        if dbversion < 61 {
            info!(context, "[migration] v61");
            sql.execute(
                "ALTER TABLE contacts ADD COLUMN selfavatar_sent INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            update_icons = true;
            sql.set_raw_config_int(context, "dbversion", 61).await?;
        }
        if dbversion < 62 {
            info!(context, "[migration] v62");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN muted_until INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.set_raw_config_int(context, "dbversion", 62).await?;
        }
        if dbversion < 63 {
            info!(context, "[migration] v63");
            sql.execute("UPDATE chats SET grpid='' WHERE type=100", paramsv![])
                .await?;
            sql.set_raw_config_int(context, "dbversion", 63).await?;
        }
        if dbversion < 64 {
            info!(context, "[migration] v64");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN error TEXT DEFAULT '';",
                paramsv![],
            )
            .await?;
            sql.set_raw_config_int(context, "dbversion", 64).await?;
        }
        if dbversion < 65 {
            info!(context, "[migration] v65");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN autodelete_timer INTEGER;",
                paramsv![],
            )
            .await?;
            // Timer value in seconds. For incoming messages this
            // timer starts when message is read, so we want to have
            // the value stored here until the timer starts.
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN autodelete_timer INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            // Timestamp indicating when the message should be
            // deleted. It is convenient to store it here because UI
            // needs this value to display how much time is left until
            // the message is deleted.
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN autodelete_timestamp INTEGER DEFAULT 0;",
                paramsv![],
            )
            .await?;
            sql.set_raw_config_int(context, "dbversion", 65).await?;
        }

        // (2) updates that require high-level objects
        // (the structure is complete now and all objects are usable)
        // --------------------------------------------------------------------

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

        assert!(files.contains("hello"));
        assert!(files.contains("world.txt"));
        assert!(!files.contains("world2.txt"));
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

//! # SQLite wrapper

use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, Statement, NO_PARAMS};
use thread_local_object::ThreadLocal;

use crate::chat::{update_device_icon, update_saved_messages_icon};
use crate::constants::ShowEmails;
use crate::context::Context;
use crate::dc_tools::*;
use crate::param::*;
use crate::peerstate::*;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Sqlite Error: {:?}", _0)]
    Sql(#[cause] rusqlite::Error),
    #[fail(display = "Sqlite Connection Pool Error: {:?}", _0)]
    ConnectionPool(#[cause] r2d2::Error),
    #[fail(display = "Sqlite: Connection closed")]
    SqlNoConnection,
    #[fail(display = "Sqlite: Already open")]
    SqlAlreadyOpen,
    #[fail(display = "Sqlite: Failed to open")]
    SqlFailedToOpen,
    #[fail(display = "{:?}", _0)]
    Io(#[cause] std::io::Error),
    #[fail(display = "{:?}", _0)]
    BlobError(#[cause] crate::blob::BlobError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::Sql(err)
    }
}

impl From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Error {
        Error::ConnectionPool(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<crate::blob::BlobError> for Error {
    fn from(err: crate::blob::BlobError) -> Error {
        Error::BlobError(err)
    }
}

/// A wrapper around the underlying Sqlite3 object.
#[derive(DebugStub)]
pub struct Sql {
    pool: RwLock<Option<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>,
    #[debug_stub = "ThreadLocal<String>"]
    in_use: Arc<ThreadLocal<String>>,
}

impl Default for Sql {
    fn default() -> Self {
        Self {
            pool: RwLock::new(None),
            in_use: Arc::new(ThreadLocal::new()),
        }
    }
}

impl Sql {
    pub fn new() -> Sql {
        Self::default()
    }

    pub fn is_open(&self) -> bool {
        self.pool.read().unwrap().is_some()
    }

    pub fn close(&self, context: &Context) {
        let _ = self.pool.write().unwrap().take();
        self.in_use.remove();
        // drop closes the connection

        info!(context, "Database closed.");
    }

    // return true on success, false on failure
    pub fn open(&self, context: &Context, dbfile: &std::path::Path, readonly: bool) -> bool {
        match open(context, self, dbfile, readonly) {
            Ok(_) => true,
            Err(crate::error::Error::SqlError(Error::SqlAlreadyOpen)) => false,
            Err(_) => {
                self.close(context);
                false
            }
        }
    }

    pub fn execute<P>(&self, sql: &str, params: P) -> Result<usize>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
    {
        self.start_stmt(sql.to_string());
        self.with_conn(|conn| conn.execute(sql, params).map_err(Into::into))
    }

    fn with_conn<T, G>(&self, g: G) -> Result<T>
    where
        G: FnOnce(&Connection) -> Result<T>,
    {
        let res = match &*self.pool.read().unwrap() {
            Some(pool) => {
                let conn = pool.get()?;

                // Only one process can make changes to the database at one time.
                // busy_timeout defines, that if a seconds process wants write access,
                // this second process will wait some milliseconds
                // and try over until it gets write access or the given timeout is elapsed.
                // If the second process does not get write access within the given timeout,
                // sqlite3_step() will return the error SQLITE_BUSY.
                // (without a busy_timeout, sqlite3_step() would return SQLITE_BUSY _at once_)
                conn.busy_timeout(Duration::from_secs(10))?;

                g(&conn)
            }
            None => Err(Error::SqlNoConnection),
        };
        self.in_use.remove();
        res
    }

    pub fn prepare<G, H>(&self, sql: &str, g: G) -> Result<H>
    where
        G: FnOnce(Statement<'_>, &Connection) -> Result<H>,
    {
        self.start_stmt(sql.to_string());
        self.with_conn(|conn| {
            let stmt = conn.prepare(sql)?;
            g(stmt, conn)
        })
    }

    pub fn prepare2<G, H>(&self, sql1: &str, sql2: &str, g: G) -> Result<H>
    where
        G: FnOnce(Statement<'_>, Statement<'_>, &Connection) -> Result<H>,
    {
        self.start_stmt(format!("{} - {}", sql1, sql2));
        self.with_conn(|conn| {
            let stmt1 = conn.prepare(sql1)?;
            let stmt2 = conn.prepare(sql2)?;

            g(stmt1, stmt2, conn)
        })
    }

    /// Prepares and executes the statement and maps a function over the resulting rows.
    /// Then executes the second function over the returned iterator and returns the
    /// result of that function.
    pub fn query_map<T, P, F, G, H>(
        &self,
        sql: impl AsRef<str>,
        params: P,
        f: F,
        mut g: G,
    ) -> Result<H>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        F: FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
        G: FnMut(rusqlite::MappedRows<F>) -> Result<H>,
    {
        self.start_stmt(sql.as_ref().to_string());
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(sql.as_ref())?;
            let res = stmt.query_map(params, f)?;
            g(res)
        })
    }

    /// Return `true` if a query in the SQL statement it executes returns one or more
    /// rows and false if the SQL returns an empty set.
    pub fn exists<P>(&self, sql: &str, params: P) -> Result<bool>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
    {
        self.start_stmt(sql.to_string());
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(sql)?;
            let res = stmt.exists(params)?;
            Ok(res)
        })
    }

    /// Execute a query which is expected to return one row.
    pub fn query_row<T, P, F>(&self, sql: impl AsRef<str>, params: P, f: F) -> Result<T>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        self.start_stmt(sql.as_ref().to_string());
        self.with_conn(|conn| conn.query_row(sql.as_ref(), params, f).map_err(Into::into))
    }

    pub fn table_exists(&self, name: impl AsRef<str>) -> bool {
        self.with_conn(|conn| table_exists(conn, name))
            .unwrap_or_default()
    }

    /// Executes a query which is expected to return one row and one
    /// column. If the query does not return a value or returns SQL
    /// `NULL`, returns `Ok(None)`.
    pub fn query_get_value_result<P, T>(&self, query: &str, params: P) -> Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        T: rusqlite::types::FromSql,
    {
        match self.query_row(query, params, |row| row.get::<_, T>(0)) {
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

    /// Not resultified version of `query_get_value_result`. Returns
    /// `None` on error.
    pub fn query_get_value<P, T>(&self, context: &Context, query: &str, params: P) -> Option<T>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        T: rusqlite::types::FromSql,
    {
        match self.query_get_value_result(query, params) {
            Ok(res) => res,
            Err(err) => {
                error!(context, "sql: Failed query_row: {}", err);
                None
            }
        }
    }

    /// Set private configuration options.
    ///
    /// Setting `None` deletes the value.  On failure an error message
    /// will already have been logged.
    pub fn set_raw_config(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: Option<&str>,
    ) -> Result<()> {
        if !self.is_open() {
            error!(context, "set_raw_config(): Database not ready.");
            return Err(Error::SqlNoConnection);
        }

        let key = key.as_ref();
        let res = if let Some(ref value) = value {
            let exists = self.exists("SELECT value FROM config WHERE keyname=?;", params![key])?;
            if exists {
                execute(
                    context,
                    self,
                    "UPDATE config SET value=? WHERE keyname=?;",
                    params![value, key],
                )
            } else {
                execute(
                    context,
                    self,
                    "INSERT INTO config (keyname, value) VALUES (?, ?);",
                    params![key, value],
                )
            }
        } else {
            execute(
                context,
                self,
                "DELETE FROM config WHERE keyname=?;",
                params![key],
            )
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
    pub fn get_raw_config(&self, context: &Context, key: impl AsRef<str>) -> Option<String> {
        if !self.is_open() || key.as_ref().is_empty() {
            return None;
        }
        self.query_get_value(
            context,
            "SELECT value FROM config WHERE keyname=?;",
            params![key.as_ref()],
        )
    }

    pub fn set_raw_config_int(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: i32,
    ) -> Result<()> {
        self.set_raw_config(context, key, Some(&format!("{}", value)))
    }

    pub fn get_raw_config_int(&self, context: &Context, key: impl AsRef<str>) -> Option<i32> {
        self.get_raw_config(context, key)
            .and_then(|s| s.parse().ok())
    }

    pub fn get_raw_config_bool(&self, context: &Context, key: impl AsRef<str>) -> bool {
        // Not the most obvious way to encode bool as string, but it is matter
        // of backward compatibility.
        self.get_raw_config_int(context, key).unwrap_or_default() > 0
    }

    pub fn set_raw_config_bool<T>(&self, context: &Context, key: T, value: bool) -> Result<()>
    where
        T: AsRef<str>,
    {
        let value = if value { Some("1") } else { None };
        self.set_raw_config(context, key, value)
    }

    pub fn set_raw_config_int64(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: i64,
    ) -> Result<()> {
        self.set_raw_config(context, key, Some(&format!("{}", value)))
    }

    pub fn get_raw_config_int64(&self, context: &Context, key: impl AsRef<str>) -> Option<i64> {
        self.get_raw_config(context, key)
            .and_then(|r| r.parse().ok())
    }

    fn start_stmt(&self, stmt: impl AsRef<str>) {
        if let Some(query) = self.in_use.get_cloned() {
            let bt = backtrace::Backtrace::new();
            eprintln!("old query: {}", query);
            eprintln!("Connection is already used from this thread: {:?}", bt);
            panic!("Connection is already used from this thread");
        }

        self.in_use.set(stmt.as_ref().to_string());
    }
}

fn table_exists(conn: &Connection, name: impl AsRef<str>) -> Result<bool> {
    let mut exists = false;
    conn.pragma(None, "table_info", &name.as_ref().to_string(), |_row| {
        // will only be executed if the info was found
        exists = true;
        Ok(())
    })?;
    Ok(exists)
}

#[allow(clippy::cognitive_complexity)]
fn open(
    context: &Context,
    sql: &Sql,
    dbfile: impl AsRef<std::path::Path>,
    readonly: bool,
) -> crate::error::Result<()> {
    if sql.is_open() {
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
    let mgr = r2d2_sqlite::SqliteConnectionManager::file(dbfile.as_ref())
        .with_flags(open_flags)
        .with_init(|c| c.execute_batch("PRAGMA secure_delete=on;"));
    let pool = r2d2::Pool::builder()
        .min_idle(Some(2))
        .max_size(10)
        .connection_timeout(std::time::Duration::new(60, 0))
        .build(mgr)
        .map_err(Error::ConnectionPool)?;

    {
        *sql.pool.write().unwrap() = Some(pool);
    }

    if !readonly {
        let mut exists_before_update = false;
        let mut dbversion_before_update = 0;
        /* Init tables to dbversion=0 */
        if !sql.table_exists("config") {
            info!(
                context,
                "First time init: creating tables in {:?}.",
                dbfile.as_ref(),
            );
            sql.execute(
                "CREATE TABLE config (id INTEGER PRIMARY KEY, keyname TEXT, value TEXT);",
                NO_PARAMS,
            )?;
            sql.execute("CREATE INDEX config_index1 ON config (keyname);", NO_PARAMS)?;
            sql.execute(
                "CREATE TABLE contacts (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 name TEXT DEFAULT '', \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 origin INTEGER DEFAULT 0, \
                 blocked INTEGER DEFAULT 0, \
                 last_seen INTEGER DEFAULT 0, \
                 param TEXT DEFAULT '');",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX contacts_index1 ON contacts (name COLLATE NOCASE);",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX contacts_index2 ON contacts (addr COLLATE NOCASE);",
                params![],
            )?;
            sql.execute(
                "INSERT INTO contacts (id,name,origin) VALUES \
                 (1,'self',262144), (2,'info',262144), (3,'rsvd',262144), \
                 (4,'rsvd',262144), (5,'device',262144), (6,'rsvd',262144), \
                 (7,'rsvd',262144), (8,'rsvd',262144), (9,'rsvd',262144);",
                params![],
            )?;
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
                params![],
            )?;
            sql.execute("CREATE INDEX chats_index1 ON chats (grpid);", params![])?;
            sql.execute(
                "CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);",
                params![],
            )?;
            sql.execute(
                "INSERT INTO chats (id,type,name) VALUES \
                 (1,120,'deaddrop'), (2,120,'rsvd'), (3,120,'trash'), \
                 (4,120,'msgs_in_creation'), (5,120,'starred'), (6,120,'archivedlink'), \
                 (7,100,'rsvd'), (8,100,'rsvd'), (9,100,'rsvd');",
                params![],
            )?;
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
                params![],
            )?;
            sql.execute("CREATE INDEX msgs_index1 ON msgs (rfc724_mid);", params![])?;
            sql.execute("CREATE INDEX msgs_index2 ON msgs (chat_id);", params![])?;
            sql.execute("CREATE INDEX msgs_index3 ON msgs (timestamp);", params![])?;
            sql.execute("CREATE INDEX msgs_index4 ON msgs (state);", params![])?;
            sql.execute(
                "INSERT INTO msgs (id,msgrmsg,txt) VALUES \
                 (1,0,'marker1'), (2,0,'rsvd'), (3,0,'rsvd'), \
                 (4,0,'rsvd'), (5,0,'rsvd'), (6,0,'rsvd'), (7,0,'rsvd'), \
                 (8,0,'rsvd'), (9,0,'daymarker');",
                params![],
            )?;
            sql.execute(
                "CREATE TABLE jobs (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 added_timestamp INTEGER, \
                 desired_timestamp INTEGER DEFAULT 0, \
                 action INTEGER, \
                 foreign_id INTEGER, \
                 param TEXT DEFAULT '');",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX jobs_index1 ON jobs (desired_timestamp);",
                params![],
            )?;
            if !sql.table_exists("config")
                || !sql.table_exists("contacts")
                || !sql.table_exists("chats")
                || !sql.table_exists("chats_contacts")
                || !sql.table_exists("msgs")
                || !sql.table_exists("jobs")
            {
                error!(
                    context,
                    "Cannot create tables in new database \"{:?}\".",
                    dbfile.as_ref(),
                );
                // cannot create the tables - maybe we cannot write?
                return Err(Error::SqlFailedToOpen.into());
            } else {
                sql.set_raw_config_int(context, "dbversion", 0)?;
            }
        } else {
            exists_before_update = true;
            dbversion_before_update = sql
                .get_raw_config_int(context, "dbversion")
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
                params![],
            )?;
            sql.execute(
                "CREATE INDEX leftgrps_index1 ON leftgrps (grpid);",
                params![],
            )?;
            dbversion = 1;
            sql.set_raw_config_int(context, "dbversion", 1)?;
        }
        if dbversion < 2 {
            info!(context, "[migration] v2");
            sql.execute(
                "ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT '';",
                params![],
            )?;
            dbversion = 2;
            sql.set_raw_config_int(context, "dbversion", 2)?;
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
                params![],
            )?;
            dbversion = 7;
            sql.set_raw_config_int(context, "dbversion", 7)?;
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
                params![],
            )?;
            sql.execute(
                "CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);",
                params![],
            )?;
            dbversion = 10;
            sql.set_raw_config_int(context, "dbversion", 10)?;
        }
        if dbversion < 12 {
            info!(context, "[migration] v12");
            sql.execute(
                "CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);",
                params![],
            )?;
            dbversion = 12;
            sql.set_raw_config_int(context, "dbversion", 12)?;
        }
        if dbversion < 17 {
            info!(context, "[migration] v17");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute("CREATE INDEX chats_index2 ON chats (archived);", params![])?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute("CREATE INDEX msgs_index5 ON msgs (starred);", params![])?;
            dbversion = 17;
            sql.set_raw_config_int(context, "dbversion", 17)?;
        }
        if dbversion < 18 {
            info!(context, "[migration] v18");
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute("ALTER TABLE acpeerstates ADD COLUMN gossip_key;", params![])?;
            dbversion = 18;
            sql.set_raw_config_int(context, "dbversion", 18)?;
        }
        if dbversion < 27 {
            info!(context, "[migration] v27");
            // chat.id=1 and chat.id=2 are the old deaddrops,
            // the current ones are defined by chats.blocked=2
            sql.execute("DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;", params![])?;
            sql.execute(
                "CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;",
                params![],
            )?;
            dbversion = 27;
            sql.set_raw_config_int(context, "dbversion", 27)?;
        }
        if dbversion < 34 {
            info!(context, "[migration] v34");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT '';",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT '';",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);",
                params![],
            )?;
            recalc_fingerprints = true;
            dbversion = 34;
            sql.set_raw_config_int(context, "dbversion", 34)?;
        }
        if dbversion < 39 {
            info!(context, "[migration] v39");
            sql.execute(
                "CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT '', timestamp INTEGER DEFAULT 0);",
                params![]
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN verified_key;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT '';",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);",
                params![],
            )?;
            dbversion = 39;
            sql.set_raw_config_int(context, "dbversion", 39)?;
        }
        if dbversion < 40 {
            info!(context, "[migration] v40");
            sql.execute(
                "ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;",
                params![],
            )?;
            dbversion = 40;
            sql.set_raw_config_int(context, "dbversion", 40)?;
        }
        if dbversion < 44 {
            info!(context, "[migration] v44");
            sql.execute("ALTER TABLE msgs ADD COLUMN mime_headers TEXT;", params![])?;
            dbversion = 44;
            sql.set_raw_config_int(context, "dbversion", 44)?;
        }
        if dbversion < 46 {
            info!(context, "[migration] v46");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN mime_references TEXT;",
                params![],
            )?;
            dbversion = 46;
            sql.set_raw_config_int(context, "dbversion", 46)?;
        }
        if dbversion < 47 {
            info!(context, "[migration] v47");
            sql.execute(
                "ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;",
                params![],
            )?;
            dbversion = 47;
            sql.set_raw_config_int(context, "dbversion", 47)?;
        }
        if dbversion < 48 {
            info!(context, "[migration] v48");
            // NOTE: move_state is not used anymore
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;",
                params![],
            )?;

            dbversion = 48;
            sql.set_raw_config_int(context, "dbversion", 48)?;
        }
        if dbversion < 49 {
            info!(context, "[migration] v49");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;",
                params![],
            )?;
            dbversion = 49;
            sql.set_raw_config_int(context, "dbversion", 49)?;
        }
        if dbversion < 50 {
            info!(context, "[migration] v50");
            // installations <= 0.100.1 used DC_SHOW_EMAILS_ALL implicitly;
            // keep this default and use DC_SHOW_EMAILS_NO
            // only for new installations
            if exists_before_update {
                sql.set_raw_config_int(context, "show_emails", ShowEmails::All as i32)?;
            }
            dbversion = 50;
            sql.set_raw_config_int(context, "dbversion", 50)?;
        }
        if dbversion < 53 {
            info!(context, "[migration] v53");
            // the messages containing _only_ locations
            // are also added to the database as _hidden_.
            sql.execute(
                "CREATE TABLE locations ( id INTEGER PRIMARY KEY AUTOINCREMENT, latitude REAL DEFAULT 0.0, longitude REAL DEFAULT 0.0, accuracy REAL DEFAULT 0.0, timestamp INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0);",
                params![]
            )?;
            sql.execute(
                "CREATE INDEX locations_index1 ON locations (from_id);",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX locations_index2 ON locations (timestamp);",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX chats_index3 ON chats (locations_send_until);",
                params![],
            )?;
            dbversion = 53;
            sql.set_raw_config_int(context, "dbversion", 53)?;
        }
        if dbversion < 54 {
            info!(context, "[migration] v54");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute("CREATE INDEX msgs_index6 ON msgs (location_id);", params![])?;
            dbversion = 54;
            sql.set_raw_config_int(context, "dbversion", 54)?;
        }
        if dbversion < 55 {
            info!(context, "[migration] v55");
            sql.execute(
                "ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.set_raw_config_int(context, "dbversion", 55)?;
        }
        if dbversion < 59 {
            info!(context, "[migration] v59");
            // records in the devmsglabels are kept when the message is deleted.
            // so, msg_id may or may not exist.
            sql.execute(
                "CREATE TABLE devmsglabels (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT, msg_id INTEGER DEFAULT 0);",
                NO_PARAMS,
            )?;
            sql.execute(
                "CREATE INDEX devmsglabels_index1 ON devmsglabels (label);",
                NO_PARAMS,
            )?;
            if exists_before_update && sql.get_raw_config_int(context, "bcc_self").is_none() {
                sql.set_raw_config_int(context, "bcc_self", 1)?;
            }
            sql.set_raw_config_int(context, "dbversion", 59)?;
        }
        if dbversion < 60 {
            info!(context, "[migration] v60");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN created_timestamp INTEGER DEFAULT 0;",
                NO_PARAMS,
            )?;
            sql.set_raw_config_int(context, "dbversion", 60)?;
        }
        if dbversion < 61 {
            info!(context, "[migration] v61");
            sql.execute(
                "ALTER TABLE contacts ADD COLUMN selfavatar_sent INTEGER DEFAULT 0;",
                NO_PARAMS,
            )?;
            update_icons = true;
            sql.set_raw_config_int(context, "dbversion", 61)?;
        }

        // (2) updates that require high-level objects
        // (the structure is complete now and all objects are usable)
        // --------------------------------------------------------------------

        if recalc_fingerprints {
            info!(context, "[migration] recalc fingerprints");
            sql.query_map(
                "SELECT addr FROM acpeerstates;",
                params![],
                |row| row.get::<_, String>(0),
                |addrs| {
                    for addr in addrs {
                        if let Some(ref mut peerstate) = Peerstate::from_addr(context, sql, &addr?)
                        {
                            peerstate.recalc_fingerprint();
                            peerstate.save_to_db(sql, false)?;
                        }
                    }
                    Ok(())
                },
            )?;
        }
        if update_icons {
            update_saved_messages_icon(context)?;
            update_device_icon(context)?;
        }
    }

    info!(context, "Opened {:?}.", dbfile.as_ref(),);

    Ok(())
}

pub fn execute<P>(context: &Context, sql: &Sql, querystr: impl AsRef<str>, params: P) -> Result<()>
where
    P: IntoIterator,
    P::Item: rusqlite::ToSql,
{
    match sql.execute(querystr.as_ref(), params) {
        Ok(_) => Ok(()),
        Err(err) => {
            error!(
                context,
                "execute failed: {:?} for {}",
                &err,
                querystr.as_ref()
            );
            Err(err)
        }
    }
}

pub fn try_execute(context: &Context, sql: &Sql, querystr: impl AsRef<str>) -> Result<()> {
    // same as execute() but does not pass error to ui
    match sql.execute(querystr.as_ref(), params![]) {
        Ok(_) => Ok(()),
        Err(err) => {
            warn!(
                context,
                "Try-execute for \"{}\" failed: {}",
                querystr.as_ref(),
                &err,
            );
            Err(err)
        }
    }
}

pub fn get_rowid(
    context: &Context,
    sql: &Sql,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: impl AsRef<str>,
) -> u32 {
    sql.start_stmt("get rowid".to_string());
    sql.with_conn(|conn| Ok(get_rowid_with_conn(context, conn, table, field, value)))
        .unwrap_or_else(|_| 0)
}

pub fn get_rowid_with_conn(
    context: &Context,
    conn: &Connection,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: impl AsRef<str>,
) -> u32 {
    // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    // the ORDER BY ensures, this function always returns the most recent id,
    // eg. if a Message-ID is split into different messages.
    let query = format!(
        "SELECT id FROM {} WHERE {}=? ORDER BY id DESC",
        table.as_ref(),
        field.as_ref(),
    );

    match conn.query_row(&query, params![value.as_ref()], |row| row.get::<_, u32>(0)) {
        Ok(id) => id,
        Err(err) => {
            error!(
                context,
                "sql: Failed to retrieve rowid: {} in {}", err, query
            );
            0
        }
    }
}
pub fn get_rowid2(
    context: &Context,
    sql: &Sql,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: i64,
    field2: impl AsRef<str>,
    value2: i32,
) -> u32 {
    sql.start_stmt("get rowid2".to_string());
    sql.with_conn(|conn| {
        Ok(get_rowid2_with_conn(
            context, conn, table, field, value, field2, value2,
        ))
    })
    .unwrap_or_else(|_| 0)
}

pub fn get_rowid2_with_conn(
    context: &Context,
    conn: &Connection,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: i64,
    field2: impl AsRef<str>,
    value2: i32,
) -> u32 {
    match conn.query_row(
        &format!(
            "SELECT id FROM {} WHERE {}={} AND {}={} ORDER BY id DESC",
            table.as_ref(),
            field.as_ref(),
            value,
            field2.as_ref(),
            value2,
        ),
        NO_PARAMS,
        |row| row.get::<_, u32>(0),
    ) {
        Ok(id) => id,
        Err(err) => {
            error!(context, "sql: Failed to retrieve rowid2: {}", err);
            0
        }
    }
}

pub fn housekeeping(context: &Context) {
    let mut files_in_use = HashSet::new();
    let mut unreferenced_count = 0;

    info!(context, "Start housekeeping...");
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM msgs  WHERE chat_id!=3   AND type!=10;",
        Param::File,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM jobs;",
        Param::File,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM chats;",
        Param::ProfileImage,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM contacts;",
        Param::ProfileImage,
    );

    context
        .sql
        .query_map(
            "SELECT value FROM config;",
            params![],
            |row| row.get::<_, String>(0),
            |rows| {
                for row in rows {
                    maybe_add_file(&mut files_in_use, row?);
                }
                Ok(())
            },
        )
        .unwrap_or_else(|err| {
            warn!(context, "sql: failed query: {}", err);
        });

    info!(context, "{} files in use.", files_in_use.len(),);
    /* go through directory and delete unused files */
    let p = context.get_blobdir();
    match std::fs::read_dir(p) {
        Ok(dir_handle) => {
            /* avoid deletion of files that are just created to build a message object */
            let diff = std::time::Duration::from_secs(60 * 60);
            let keep_files_newer_than = std::time::SystemTime::now().checked_sub(diff).unwrap();

            for entry in dir_handle {
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

                if let Ok(stats) = std::fs::metadata(entry.path()) {
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
                dc_delete_file(context, path);
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

fn maybe_add_from_param(
    context: &Context,
    files_in_use: &mut HashSet<String>,
    query: &str,
    param_id: Param,
) {
    context
        .sql
        .query_map(
            query,
            NO_PARAMS,
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
        .unwrap_or_else(|err| {
            warn!(context, "sql: failed to add_from_param: {}", err);
        });
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

use std::collections::HashSet;
use std::sync::RwLock;

use rusqlite::{Connection, OpenFlags, Statement, NO_PARAMS};

use crate::constants::*;
use crate::context::Context;
use crate::dc_param::*;
use crate::dc_tools::*;
use crate::error::{Error, Result};
use crate::peerstate::*;
use crate::x::*;

const DC_OPEN_READONLY: usize = 0x01;

/// A wrapper around the underlying Sqlite3 object.
pub struct Sql {
    pool: RwLock<Option<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>,
}

impl Sql {
    pub fn new() -> Sql {
        Sql {
            pool: RwLock::new(None),
        }
    }

    pub fn is_open(&self) -> bool {
        self.pool.read().unwrap().is_some()
    }

    pub fn close(&self, context: &Context) {
        let mut pool = self.pool.write().unwrap();
        if pool.is_some() {
            pool.take();
            // drop closes the connection
        }
        info!(context, 0, "Database closed.");
    }

    // return true on success, false on failure
    pub fn open(&self, context: &Context, dbfile: &std::path::Path, flags: libc::c_int) -> bool {
        match open(context, self, dbfile, flags) {
            Ok(_) => true,
            Err(Error::SqlAlreadyOpen) => false,
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
        self.with_conn(|conn| conn.execute(sql, params).map_err(Into::into))
    }

    fn with_conn<T, G>(&self, g: G) -> Result<T>
    where
        G: FnOnce(&Connection) -> Result<T>,
    {
        match &*self.pool.read().unwrap() {
            Some(pool) => {
                let conn = pool.get()?;
                g(&conn)
            }
            None => Err(Error::SqlNoConnection),
        }
    }

    pub fn prepare<G, H>(&self, sql: &str, g: G) -> Result<H>
    where
        G: FnOnce(Statement<'_>) -> Result<H>,
    {
        self.with_conn(|conn| {
            let stmt = conn.prepare(sql)?;
            let res = g(stmt)?;
            Ok(res)
        })
    }

    pub fn prepare2<G, H>(&self, sql1: &str, sql2: &str, g: G) -> Result<H>
    where
        G: FnOnce(Statement<'_>, Statement<'_>, &Connection) -> Result<H>,
    {
        self.with_conn(|conn| {
            let stmt1 = conn.prepare(sql1)?;
            let stmt2 = conn.prepare(sql2)?;

            let res = g(stmt1, stmt2, conn)?;
            Ok(res)
        })
    }

    /// Prepares and executes the statement and maps a function over the resulting rows.
    /// Then executes the second function over the returned iterator and returns the
    /// result of that function.
    pub fn query_map<T, P, F, G, H>(&self, sql: &str, params: P, f: F, mut g: G) -> Result<H>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        F: FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
        G: FnMut(rusqlite::MappedRows<F>) -> Result<H>,
    {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(sql)?;
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
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(sql)?;
            let res = stmt.exists(params)?;
            Ok(res)
        })
    }

    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        self.with_conn(|conn| conn.query_row(sql, params, f).map_err(Into::into))
    }

    pub fn table_exists(&self, name: impl AsRef<str>) -> bool {
        self.with_conn(|conn| Ok(table_exists(conn, name)))
            .unwrap_or_default()
    }
}

fn table_exists(conn: &Connection, name: impl AsRef<str>) -> bool {
    let mut exists = false;
    conn.pragma(None, "table_info", &format!("{}", name.as_ref()), |_row| {
        // will only be executed if the info was found
        exists = true;
        Ok(())
    })
    .expect("bad sqlite state");

    exists
}

// Return 1 -> success
// Return 0 -> failure
fn open(
    context: &Context,
    sql: &Sql,
    dbfile: impl AsRef<std::path::Path>,
    flags: libc::c_int,
) -> Result<()> {
    if sql.is_open() {
        error!(
            context,
            0,
            "Cannot open, database \"{:?}\" already opened.",
            dbfile.as_ref(),
        );
        return Err(Error::SqlAlreadyOpen);
    }

    let mut open_flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if 0 != (flags & DC_OPEN_READONLY as i32) {
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
        .max_size(4)
        .connection_timeout(std::time::Duration::new(60, 0))
        .build(mgr)?;

    {
        *sql.pool.write().unwrap() = Some(pool);
    }

    if 0 == flags & DC_OPEN_READONLY as i32 {
        let mut exists_before_update = 0;
        let mut dbversion_before_update = 0;
        /* Init tables to dbversion=0 */
        if !sql.table_exists("config") {
            info!(
                context,
                0,
                "First time init: creating tables in \"{:?}\".",
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
                 name TEXT DEFAULT \'\', \
                 addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                 origin INTEGER DEFAULT 0, \
                 blocked INTEGER DEFAULT 0, \
                 last_seen INTEGER DEFAULT 0, \
                 param TEXT DEFAULT \'\');",
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
                 (1,\'self\',262144), (2,\'device\',262144), (3,\'rsvd\',262144), \
                 (4,\'rsvd\',262144), (5,\'rsvd\',262144), (6,\'rsvd\',262144), \
                 (7,\'rsvd\',262144), (8,\'rsvd\',262144), (9,\'rsvd\',262144);",
                params![],
            )?;
            sql.execute(
                "CREATE TABLE chats (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT,  \
                 type INTEGER DEFAULT 0, \
                 name TEXT DEFAULT \'\', \
                 draft_timestamp INTEGER DEFAULT 0, \
                 draft_txt TEXT DEFAULT \'\', \
                 blocked INTEGER DEFAULT 0, \
                 grpid TEXT DEFAULT \'\', \
                 param TEXT DEFAULT \'\');",
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
                 (1,120,\'deaddrop\'), (2,120,\'rsvd\'), (3,120,\'trash\'), \
                 (4,120,\'msgs_in_creation\'), (5,120,\'starred\'), (6,120,\'archivedlink\'), \
                 (7,100,\'rsvd\'), (8,100,\'rsvd\'), (9,100,\'rsvd\');",
                params![],
            )?;
            sql.execute(
                "CREATE TABLE msgs (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 rfc724_mid TEXT DEFAULT \'\', \
                 server_folder TEXT DEFAULT \'\', \
                 server_uid INTEGER DEFAULT 0, \
                 chat_id INTEGER DEFAULT 0, \
                 from_id INTEGER DEFAULT 0, \
                 to_id INTEGER DEFAULT 0, \
                 timestamp INTEGER DEFAULT 0, \
                 type INTEGER DEFAULT 0, \
                 state INTEGER DEFAULT 0, \
                 msgrmsg INTEGER DEFAULT 1, \
                 bytes INTEGER DEFAULT 0, \
                 txt TEXT DEFAULT \'\', \
                 txt_raw TEXT DEFAULT \'\', \
                 param TEXT DEFAULT \'\');",
                params![],
            )?;
            sql.execute("CREATE INDEX msgs_index1 ON msgs (rfc724_mid);", params![])?;
            sql.execute("CREATE INDEX msgs_index2 ON msgs (chat_id);", params![])?;
            sql.execute("CREATE INDEX msgs_index3 ON msgs (timestamp);", params![])?;
            sql.execute("CREATE INDEX msgs_index4 ON msgs (state);", params![])?;
            sql.execute(
                "INSERT INTO msgs (id,msgrmsg,txt) VALUES \
                 (1,0,\'marker1\'), (2,0,\'rsvd\'), (3,0,\'rsvd\'), \
                 (4,0,\'rsvd\'), (5,0,\'rsvd\'), (6,0,\'rsvd\'), (7,0,\'rsvd\'), \
                 (8,0,\'rsvd\'), (9,0,\'daymarker\');",
                params![],
            )?;
            sql.execute(
                "CREATE TABLE jobs (\
                 id INTEGER PRIMARY KEY AUTOINCREMENT, \
                 added_timestamp INTEGER, \
                 desired_timestamp INTEGER DEFAULT 0, \
                 action INTEGER, \
                 foreign_id INTEGER, \
                 param TEXT DEFAULT \'\');",
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
                    0,
                    "Cannot create tables in new database \"{:?}\".",
                    dbfile.as_ref(),
                );
                // cannot create the tables - maybe we cannot write?
                return Err(Error::SqlFailedToOpen);
            } else {
                set_config_int(context, sql, "dbversion", 0);
            }
        } else {
            exists_before_update = 1;
            dbversion_before_update = get_config_int(context, sql, "dbversion", 0);
        }

        // (1) update low-level database structure.
        // this should be done before updates that use high-level objects that
        // rely themselves on the low-level structure.
        // --------------------------------------------------------------------
        let mut dbversion = dbversion_before_update;
        let mut recalc_fingerprints = 0;
        let mut update_file_paths = 0;

        if dbversion < 1 {
            sql.execute(
                "CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT \'\');",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX leftgrps_index1 ON leftgrps (grpid);",
                params![],
            )?;
            dbversion = 1;
            set_config_int(context, sql, "dbversion", 1);
        }
        if dbversion < 2 {
            sql.execute(
                "ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT \'\';",
                params![],
            )?;
            dbversion = 2;
            set_config_int(context, sql, "dbversion", 2);
        }
        if dbversion < 7 {
            sql.execute(
                "CREATE TABLE keypairs (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                 is_default INTEGER DEFAULT 0, \
                 private_key, \
                 public_key, \
                 created INTEGER DEFAULT 0);",
                params![],
            )?;
            dbversion = 7;
            set_config_int(context, sql, "dbversion", 7);
        }
        if dbversion < 10 {
            sql.execute(
                "CREATE TABLE acpeerstates (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT \'\' COLLATE NOCASE, \
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
            set_config_int(context, sql, "dbversion", 10);
        }
        if dbversion < 12 {
            sql.execute(
                "CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);",
                params![],
            )?;
            dbversion = 12;
            set_config_int(context, sql, "dbversion", 12);
        }
        if dbversion < 17 {
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
            set_config_int(context, sql, "dbversion", 17);
        }
        if dbversion < 18 {
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute("ALTER TABLE acpeerstates ADD COLUMN gossip_key;", params![])?;
            dbversion = 18;
            set_config_int(context, sql, "dbversion", 18);
        }
        if dbversion < 27 {
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
            set_config_int(context, sql, "dbversion", 27);
        }
        if dbversion < 34 {
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT \'\';",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT \'\';",
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
            recalc_fingerprints = 1;
            dbversion = 34;
            set_config_int(context, sql, "dbversion", 34);
        }
        if dbversion < 39 {
            sql.execute(
                "CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT \'\', timestamp INTEGER DEFAULT 0);",
                params![]
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN verified_key;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT \'\';",
                params![],
            )?;
            sql.execute(
                "CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);",
                params![],
            )?;
            if dbversion_before_update == 34 {
                sql.execute(
                    "UPDATE acpeerstates SET verified_key=gossip_key, verified_key_fingerprint=gossip_key_fingerprint WHERE gossip_key_verified=2;",
                    params![]
                )?;
                sql.execute(
                    "UPDATE acpeerstates SET verified_key=public_key, verified_key_fingerprint=public_key_fingerprint WHERE public_key_verified=2;",
                    params![]
                )?;
            }
            dbversion = 39;
            set_config_int(context, sql, "dbversion", 39);
        }
        if dbversion < 40 {
            sql.execute(
                "ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;",
                params![],
            )?;
            dbversion = 40;
            set_config_int(context, sql, "dbversion", 40);
        }
        if dbversion < 41 {
            update_file_paths = 1;
            dbversion = 41;
            set_config_int(context, sql, "dbversion", 41);
        }
        if dbversion < 42 {
            sql.execute("UPDATE msgs SET txt=\'\' WHERE type!=10", params![])?;
            dbversion = 42;
            set_config_int(context, sql, "dbversion", 42);
        }
        if dbversion < 44 {
            sql.execute("ALTER TABLE msgs ADD COLUMN mime_headers TEXT;", params![])?;
            dbversion = 44;
            set_config_int(context, sql, "dbversion", 44);
        }
        if dbversion < 46 {
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;",
                params![],
            )?;
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN mime_references TEXT;",
                params![],
            )?;
            dbversion = 46;
            set_config_int(context, sql, "dbversion", 46);
        }
        if dbversion < 47 {
            info!(context, 0, "[migration] v47");
            sql.execute(
                "ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;",
                params![],
            )?;
            dbversion = 47;
            set_config_int(context, sql, "dbversion", 47);
        }
        if dbversion < 48 {
            info!(context, 0, "[migration] v48");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;",
                params![],
            )?;
            assert_eq!(DC_MOVE_STATE_UNDEFINED as libc::c_int, 0);
            assert_eq!(DC_MOVE_STATE_PENDING as libc::c_int, 1);
            assert_eq!(DC_MOVE_STATE_STAY as libc::c_int, 2);
            assert_eq!(DC_MOVE_STATE_MOVING as libc::c_int, 3);

            dbversion = 48;
            set_config_int(context, sql, "dbversion", 48);
        }
        if dbversion < 49 {
            info!(context, 0, "[migration] v49");
            sql.execute(
                "ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;",
                params![],
            )?;
            dbversion = 49;
            set_config_int(context, sql, "dbversion", 49);
        }
        if dbversion < 50 {
            info!(context, 0, "[migration] v50");
            if 0 != exists_before_update {
                set_config_int(context, sql, "show_emails", 2);
            }
            dbversion = 50;
            set_config_int(context, sql, "dbversion", 50);
        }
        if dbversion < 53 {
            info!(context, 0, "[migration] v53");
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
            set_config_int(context, sql, "dbversion", 53);
        }
        if dbversion < 54 {
            info!(context, 0, "[migration] v54");
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;",
                params![],
            )?;
            sql.execute("CREATE INDEX msgs_index6 ON msgs (location_id);", params![])?;
            dbversion = 54;
            set_config_int(context, sql, "dbversion", 54);
        }
        if dbversion < 55 {
            sql.execute(
                "ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;",
                params![],
            )?;

            set_config_int(context, sql, "dbversion", 55);
        }

        if 0 != recalc_fingerprints {
            sql.query_map(
                "SELECT addr FROM acpeerstates;",
                params![],
                |row| row.get::<_, String>(0),
                |addrs| {
                    for addr in addrs {
                        if let Some(ref mut peerstate) = Peerstate::from_addr(context, sql, &addr?)
                        {
                            peerstate.recalc_fingerprint();
                            peerstate.save_to_db(sql, false);
                        }
                    }
                    Ok(())
                },
            )?;
        }
        if 0 != update_file_paths {
            // versions before 2018-08 save the absolute paths in the database files at "param.f=";
            // for newer versions, we copy files always to the blob directory and store relative paths.
            // this snippet converts older databases and can be removed after some time.

            info!(context, 0, "[open] update file paths");

            let repl_from = get_config(
                context,
                sql,
                "backup_for",
                Some(as_str(context.get_blobdir())),
            )
            .unwrap();

            let repl_from = dc_ensure_no_slash_safe(&repl_from);
            sql.execute(
                &format!(
                    "UPDATE msgs SET param=replace(param, \'f={}/\', \'f=$BLOBDIR/\')",
                    repl_from
                ),
                NO_PARAMS,
            )?;

            sql.execute(
                &format!(
                    "UPDATE chats SET param=replace(param, \'i={}/\', \'i=$BLOBDIR/\');",
                    repl_from
                ),
                NO_PARAMS,
            )?;

            set_config(context, sql, "backup_for", None);
        }
    }

    info!(context, 0, "Opened \"{:?}\".", dbfile.as_ref(),);

    Ok(())
}

// handle configurations, private
pub fn set_config(
    context: &Context,
    sql: &Sql,
    key: impl AsRef<str>,
    value: Option<&str>,
) -> libc::c_int {
    if !sql.is_open() {
        error!(context, 0, "set_config(): Database not ready.");
        return 0;
    }

    let key = key.as_ref();
    let good;

    if let Some(ref value) = value {
        let exists = sql
            .exists("SELECT value FROM config WHERE keyname=?;", params![key])
            .unwrap_or_default();
        if exists {
            good = execute(
                context,
                sql,
                "UPDATE config SET value=? WHERE keyname=?;",
                params![value, key],
            );
        } else {
            good = execute(
                context,
                sql,
                "INSERT INTO config (keyname, value) VALUES (?, ?);",
                params![key, value],
            );
        }
    } else {
        good = execute(
            context,
            sql,
            "DELETE FROM config WHERE keyname=?;",
            params![key],
        );
    }

    if !good {
        error!(context, 0, "set_config(): Cannot change value.",);
        return 0;
    }

    1
}

pub fn get_config(
    context: &Context,
    sql: &Sql,
    key: impl AsRef<str>,
    def: Option<&str>,
) -> Option<String> {
    if !sql.is_open() || key.as_ref().is_empty() {
        return None;
    }
    query_row(
        context,
        sql,
        "SELECT value FROM config WHERE keyname=?;",
        params![key.as_ref()],
        0,
    )
    .or_else(|| def.map(|s| s.to_string()))
}

pub fn execute<P>(context: &Context, sql: &Sql, querystr: impl AsRef<str>, params: P) -> bool
where
    P: IntoIterator,
    P::Item: rusqlite::ToSql,
{
    match sql.execute(querystr.as_ref(), params) {
        Ok(_) => true,
        Err(err) => {
            error!(context, 0, "execute failed: {:?}", err);
            false
        }
    }
}

// TODO Remove the Option<> from the return type.
pub fn query_row<P, T>(
    context: &Context,
    sql: &Sql,
    query: &str,
    params: P,
    column: usize,
) -> Option<T>
where
    P: IntoIterator,
    P::Item: rusqlite::ToSql,
    T: rusqlite::types::FromSql,
{
    match sql.query_row(query, params, |row| row.get::<_, T>(column)) {
        Ok(res) => Some(res),
        Err(Error::Sql(rusqlite::Error::QueryReturnedNoRows)) => None,
        Err(Error::Sql(rusqlite::Error::InvalidColumnType(_, _, rusqlite::types::Type::Null))) => {
            None
        }
        Err(err) => {
            error!(context, 0, "sql: Failed query_row: {}", err);
            None
        }
    }
}

pub fn set_config_int(
    context: &Context,
    sql: &Sql,
    key: impl AsRef<str>,
    value: i32,
) -> libc::c_int {
    set_config(context, sql, key, Some(&format!("{}", value)))
}

pub fn get_config_int(context: &Context, sql: &Sql, key: impl AsRef<str>, def: i32) -> i32 {
    get_config(context, sql, key, None)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| def)
}

pub fn set_config_int64(
    context: &Context,
    sql: &Sql,
    key: impl AsRef<str>,
    value: i64,
) -> libc::c_int {
    set_config(context, sql, key, Some(&format!("{}", value)))
}

pub fn get_config_int64(
    context: &Context,
    sql: &Sql,
    key: impl AsRef<str>,
    def: Option<i64>,
) -> i64 {
    let ret = get_config(context, sql, key, None);
    ret.map(|r| r.parse().unwrap_or_default())
        .unwrap_or_else(|| def.unwrap_or_default())
}

pub fn try_execute(context: &Context, sql: &Sql, querystr: impl AsRef<str>) -> libc::c_int {
    // same as execute() but does not pass error to ui
    match sql.execute(querystr.as_ref(), params![]) {
        Ok(_) => 1,
        Err(err) => {
            warn!(
                context,
                0,
                "Try-execute for \"{}\" failed: {}",
                querystr.as_ref(),
                err,
            );
            0
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
    // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    // the ORDER BY ensures, this function always returns the most recent id,
    // eg. if a Message-ID is splitted into different messages.
    let query = format!(
        "SELECT id FROM {} WHERE {}='{}' ORDER BY id DESC",
        table.as_ref(),
        field.as_ref(),
        value.as_ref()
    );

    match sql.query_row(&query, NO_PARAMS, |row| row.get::<_, u32>(0)) {
        Ok(id) => id,
        Err(err) => {
            error!(
                context,
                0, "sql: Failed to retrieve rowid: {} in {}", err, query
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
            error!(context, 0, "sql: Failed to retrieve rowid2: {}", err);
            0
        }
    }
}

pub fn housekeeping(context: &Context) {
    let mut files_in_use = HashSet::new();
    let mut unreferenced_count = 0;

    info!(context, 0, "Start housekeeping...");
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM msgs  WHERE chat_id!=3   AND type!=10;",
        'f' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM jobs;",
        'f' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM chats;",
        'i' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM contacts;",
        'i' as i32,
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
            warn!(context, 0, "sql: failed query: {}", err);
        });

    info!(context, 0, "{} files in use.", files_in_use.len(),);
    /* go through directory and delete unused files */
    let p = std::path::Path::new(as_str(context.get_blobdir()));
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
                let name_c = to_cstring(name_f.to_string_lossy());

                if unsafe {
                    is_file_in_use(&mut files_in_use, 0 as *const libc::c_char, name_c.as_ptr())
                } || unsafe {
                    is_file_in_use(
                        &mut files_in_use,
                        b".increation\x00" as *const u8 as *const libc::c_char,
                        name_c.as_ptr(),
                    )
                } || unsafe {
                    is_file_in_use(
                        &mut files_in_use,
                        b".waveform\x00" as *const u8 as *const libc::c_char,
                        name_c.as_ptr(),
                    )
                } || unsafe {
                    is_file_in_use(
                        &mut files_in_use,
                        b"-preview.jpg\x00" as *const u8 as *const libc::c_char,
                        name_c.as_ptr(),
                    )
                } {
                    continue;
                }
                unreferenced_count += 1;

                match std::fs::metadata(entry.path()) {
                    Ok(stats) => {
                        let created = stats.created().is_ok()
                            && stats.created().unwrap() > keep_files_newer_than;
                        let modified = stats.modified().is_ok()
                            && stats.modified().unwrap() > keep_files_newer_than;
                        let accessed = stats.accessed().is_ok()
                            && stats.accessed().unwrap() > keep_files_newer_than;

                        if created || modified || accessed {
                            info!(
                                context,
                                0,
                                "Housekeeping: Keeping new unreferenced file #{}: {:?}",
                                unreferenced_count,
                                entry.file_name(),
                            );
                            continue;
                        }
                    }
                    Err(_) => {}
                }
                info!(
                    context,
                    0,
                    "Housekeeping: Deleting unreferenced file #{}: {:?}",
                    unreferenced_count,
                    entry.file_name()
                );
                let path = to_cstring(entry.path().to_str().unwrap());
                unsafe { dc_delete_file(context, path.as_ptr()) };
            }
        }
        Err(err) => {
            warn!(
                context,
                0,
                "Housekeeping: Cannot open {}. ({})",
                as_str(context.get_blobdir()),
                err
            );
        }
    }

    info!(context, 0, "Housekeeping done.",);
}

unsafe fn is_file_in_use(
    files_in_use: &HashSet<String>,
    namespc: *const libc::c_char,
    name: *const libc::c_char,
) -> bool {
    let name_to_check = dc_strdup(name);
    if !namespc.is_null() {
        let name_len: libc::c_int = strlen(name) as libc::c_int;
        let namespc_len: libc::c_int = strlen(namespc) as libc::c_int;
        if name_len <= namespc_len
            || strcmp(&*name.offset((name_len - namespc_len) as isize), namespc) != 0
        {
            return false;
        }
        *name_to_check.offset((name_len - namespc_len) as isize) = 0 as libc::c_char
    }

    let contains = files_in_use.contains(as_str(name_to_check));
    free(name_to_check as *mut libc::c_void);
    contains
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
    param_id: libc::c_int,
) {
    let param = unsafe { dc_param_new() };

    context
        .sql
        .query_row(query, NO_PARAMS, |row| {
            let v = to_cstring(row.get::<_, String>(0)?);
            unsafe {
                dc_param_set_packed(param, v.as_ptr() as *const libc::c_char);
                let file = dc_param_get(param, param_id, 0 as *const libc::c_char);
                if !file.is_null() {
                    maybe_add_file(files_in_use, as_str(file));
                    free(file as *mut libc::c_void);
                }
            }
            Ok(())
        })
        .unwrap_or_else(|err| {
            warn!(context, 0, "sql: failed to add_from_param: {}", err);
        });

    unsafe { dc_param_unref(param) };
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

        println!("{:?}", files);
        assert!(unsafe {
            is_file_in_use(
                &mut files,
                std::ptr::null(),
                b"hello\x00" as *const u8 as *const _,
            )
        });
        assert!(!unsafe {
            is_file_in_use(
                &mut files,
                b".txt\x00" as *const u8 as *const _,
                b"hello\x00" as *const u8 as *const _,
            )
        });
        assert!(unsafe {
            is_file_in_use(
                &mut files,
                b"-suffix\x00" as *const u8 as *const _,
                b"world.txt-suffix\x00" as *const u8 as *const _,
            )
        });
    }
}

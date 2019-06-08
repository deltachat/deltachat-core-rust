use std::collections::HashSet;

use rusqlite::{Connection, OpenFlags, Statement, NO_PARAMS};

use crate::constants::*;
use crate::context::Context;
use crate::dc_log::*;
use crate::dc_param::*;
use crate::dc_tools::*;
use crate::peerstate::*;
use crate::types::*;
use crate::x::*;

const DC_OPEN_READONLY: usize = 0x01;

/// A simple wrapper around the underlying Sqlite3 object.
pub struct dc_sqlite3_t {
    connection: Option<Connection>,
}

impl dc_sqlite3_t {
    pub fn conn(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

    pub unsafe fn raw(&self) -> Option<*mut sqlite3> {
        self.connection.as_ref().map(|c| c.handle())
    }
}

pub fn dc_sqlite3_new() -> dc_sqlite3_t {
    dc_sqlite3_t { connection: None }
}

pub fn dc_sqlite3_close(context: &Context, sql: &mut dc_sqlite3_t) {
    if !sql.connection.is_some() {
        let _conn = sql.connection.take().unwrap();
        // drop closes the connection
    }

    info!(context, 0, "Database closed.");
}

pub fn dc_sqlite3_open(
    context: &Context,
    sql: &mut dc_sqlite3_t,
    dbfile: *const libc::c_char,
    flags: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    if 0 != dc_sqlite3_is_open(sql) {
        return 0;
    }
    if !dbfile.is_null() {
        let dbfile = as_str(dbfile);
        if unsafe { sqlite3_threadsafe() } == 0 {
            error!(
                context,
                0, "Sqlite3 compiled thread-unsafe; this is not supported.",
            );
        } else if sql.conn().is_some() {
            error!(
                context,
                0, "Cannot open, database \"{}\" already opened.", dbfile,
            );
        } else {
            let mut open_flags = OpenFlags::SQLITE_OPEN_FULL_MUTEX;
            if 0 != (flags & DC_OPEN_READONLY as i32) {
                open_flags.insert(OpenFlags::SQLITE_OPEN_READ_ONLY);
            } else {
                open_flags.insert(OpenFlags::SQLITE_OPEN_READ_WRITE);
                open_flags.insert(OpenFlags::SQLITE_OPEN_CREATE);
            }

            match Connection::open_with_flags(dbfile, open_flags) {
                Ok(conn) => {
                    sql.connection = Some(conn);
                }
                Err(err) => {
                    error!(context, 0, "Cannot open database: \"{}\".", err);
                    return 0;
                }
            }

            let conn = sql.conn().unwrap();

            conn.pragma_update(None, "secure_delete", "on")
                .expect("failed to enable pragma");
            conn.busy_timeout(std::time::Duration::new(10, 0))
                .expect("failed to set busy timeout");

            if 0 == flags & DC_OPEN_READONLY as i32 {
                let mut exists_before_update = 0;
                let mut dbversion_before_update = 0;
                /* Init tables to dbversion=0 */
                if 0 == dc_sqlite3_table_exists(context, sql, "config") {
                    info!(
                        context,
                        0, "First time init: creating tables in \"{}\".", dbfile,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE TABLE config (id INTEGER PRIMARY KEY, keyname TEXT, value TEXT);",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE INDEX config_index1 ON config (keyname);",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE TABLE contacts (\
                         id INTEGER PRIMARY KEY AUTOINCREMENT, \
                         name TEXT DEFAULT \'\', \
                         addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                         origin INTEGER DEFAULT 0, \
                         blocked INTEGER DEFAULT 0, \
                         last_seen INTEGER DEFAULT 0, \
                         param TEXT DEFAULT \'\');",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE INDEX contacts_index1 ON contacts (name COLLATE NOCASE);",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE INDEX contacts_index2 ON contacts (addr COLLATE NOCASE);",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "INSERT INTO contacts (id,name,origin) VALUES \
                         (1,\'self\',262144), (2,\'device\',262144), (3,\'rsvd\',262144), \
                         (4,\'rsvd\',262144), (5,\'rsvd\',262144), (6,\'rsvd\',262144), \
                         (7,\'rsvd\',262144), (8,\'rsvd\',262144), (9,\'rsvd\',262144);",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE TABLE chats (\
                         id INTEGER PRIMARY KEY AUTOINCREMENT,  \
                         type INTEGER DEFAULT 0, \
                         name TEXT DEFAULT \'\', \
                         draft_timestamp INTEGER DEFAULT 0, \
                         draft_txt TEXT DEFAULT \'\', \
                         blocked INTEGER DEFAULT 0, \
                         grpid TEXT DEFAULT \'\', \
                         param TEXT DEFAULT \'\');",
                    );
                    dc_sqlite3_execute(context, sql, "CREATE INDEX chats_index1 ON chats (grpid);");
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "INSERT INTO chats (id,type,name) VALUES \
                          (1,120,\'deaddrop\'), (2,120,\'rsvd\'), (3,120,\'trash\'), \
                          (4,120,\'msgs_in_creation\'), (5,120,\'starred\'), (6,120,\'archivedlink\'), \
                          (7,100,\'rsvd\'), (8,100,\'rsvd\'), (9,100,\'rsvd\');"

                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
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
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE INDEX msgs_index1 ON msgs (rfc724_mid);",
                    );
                    dc_sqlite3_execute(context, sql, "CREATE INDEX msgs_index2 ON msgs (chat_id);");
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE INDEX msgs_index3 ON msgs (timestamp);",
                    );
                    dc_sqlite3_execute(context, sql, "CREATE INDEX msgs_index4 ON msgs (state);");
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "INSERT INTO msgs (id,msgrmsg,txt) VALUES \
                         (1,0,\'marker1\'), (2,0,\'rsvd\'), (3,0,\'rsvd\'), \
                         (4,0,\'rsvd\'), (5,0,\'rsvd\'), (6,0,\'rsvd\'), (7,0,\'rsvd\'), \
                         (8,0,\'rsvd\'), (9,0,\'daymarker\');",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE TABLE jobs (\
                         id INTEGER PRIMARY KEY AUTOINCREMENT, \
                         added_timestamp INTEGER, \
                         desired_timestamp INTEGER DEFAULT 0, \
                         action INTEGER, \
                         foreign_id INTEGER, \
                         param TEXT DEFAULT \'\');",
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        "CREATE INDEX jobs_index1 ON jobs (desired_timestamp);",
                    );
                    if 0 == dc_sqlite3_table_exists(context, sql, "config")
                        || 0 == dc_sqlite3_table_exists(context, sql, "contacts")
                        || 0 == dc_sqlite3_table_exists(context, sql, "chats")
                        || 0 == dc_sqlite3_table_exists(context, sql, "chats_contacts")
                        || 0 == dc_sqlite3_table_exists(context, sql, "msgs")
                        || 0 == dc_sqlite3_table_exists(context, sql, "jobs")
                    {
                        error!(
                            context,
                            0, "Cannot create tables in new database \"{}\".", dbfile,
                        );
                        // cannot create the tables - maybe we cannot write?
                        current_block = 13628706266672894061;
                    } else {
                        dc_sqlite3_set_config_int(
                            context,
                            sql,
                            b"dbversion\x00" as *const u8 as *const libc::c_char,
                            0,
                        );
                        current_block = 14072441030219150333;
                    }
                } else {
                    exists_before_update = 1;
                    dbversion_before_update = dc_sqlite3_get_config_int(
                        context,
                        sql,
                        b"dbversion\x00" as *const u8 as *const libc::c_char,
                        0,
                    );
                    current_block = 14072441030219150333;
                }
                match current_block {
                    13628706266672894061 => {}
                    _ => {
                        // (1) update low-level database structure.
                        // this should be done before updates that use high-level objects that
                        // rely themselves on the low-level structure.
                        // --------------------------------------------------------------------
                        let mut dbversion: libc::c_int = dbversion_before_update;
                        let mut recalc_fingerprints: libc::c_int = 0;
                        let mut update_file_paths: libc::c_int = 0;
                        if dbversion < 1 {
                            dc_sqlite3_execute(
                                context, sql,
                                "CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT \'\');"

                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX leftgrps_index1 ON leftgrps (grpid);",
                            );
                            dbversion = 1;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                1,
                            );
                        }
                        if dbversion < 2 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT \'\';",
                            );
                            dbversion = 2;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                2,
                            );
                        }
                        if dbversion < 7 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE TABLE keypairs (\
                                 id INTEGER PRIMARY KEY, \
                                 addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                                 is_default INTEGER DEFAULT 0, \
                                 private_key, \
                                 public_key, \
                                 created INTEGER DEFAULT 0);",
                            );
                            dbversion = 7;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                7,
                            );
                        }
                        if dbversion < 10 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE TABLE acpeerstates (\
                                 id INTEGER PRIMARY KEY, \
                                 addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                                 last_seen INTEGER DEFAULT 0, \
                                 last_seen_autocrypt INTEGER DEFAULT 0, \
                                 public_key, \
                                 prefer_encrypted INTEGER DEFAULT 0);",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);",
                            );
                            dbversion = 10;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                10,
                            );
                        }
                        if dbversion < 12 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 12;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                12,
                            );
                        }
                        if dbversion < 17 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX chats_index2 ON chats (archived);",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX msgs_index5 ON msgs (starred);",
                            );
                            dbversion = 17;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                17,
                            );
                        }
                        if dbversion < 18 {
                            dc_sqlite3_execute(
                                context, sql,
                                "ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;"
                                   );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE acpeerstates ADD COLUMN gossip_key;",
                            );
                            dbversion = 18;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                18,
                            );
                        }
                        if dbversion < 27 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;",
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                "CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);"
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;",
                            );
                            dbversion = 27;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                27,
                            );
                        }
                        if dbversion < 34 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;",
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                "ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;"
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                "ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT \'\';"
                                   );
                            dc_sqlite3_execute(
                                context, sql,
                                "ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT \'\';"
                                   );
                            dc_sqlite3_execute(
                                context, sql,
                                "CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);"
                                   );
                            dc_sqlite3_execute(
                                context, sql,
                                "CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);"
                                   );
                            recalc_fingerprints = 1;
                            dbversion = 34;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                34,
                            );
                        }
                        if dbversion < 39 {
                            dc_sqlite3_execute(
                                context, sql,
                                "CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT \'\', timestamp INTEGER DEFAULT 0);"
                                   );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE acpeerstates ADD COLUMN verified_key;",
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                "ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT \'\';"
                                   );
                            dc_sqlite3_execute(
                                context, sql,
                                "CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);"
                                   );
                            if dbversion_before_update == 34 {
                                dc_sqlite3_execute(
                                    context, sql,
                                    "UPDATE acpeerstates SET verified_key=gossip_key, verified_key_fingerprint=gossip_key_fingerprint WHERE gossip_key_verified=2;"
                                       );
                                dc_sqlite3_execute(
                                    context, sql,
                                    "UPDATE acpeerstates SET verified_key=public_key, verified_key_fingerprint=public_key_fingerprint WHERE public_key_verified=2;"
                                       );
                            }
                            dbversion = 39;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                39,
                            );
                        }
                        if dbversion < 40 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;",
                            );
                            dbversion = 40;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                40,
                            );
                        }
                        if dbversion < 41 {
                            update_file_paths = 1;
                            dbversion = 41;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                41,
                            );
                        }
                        if dbversion < 42 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "UPDATE msgs SET txt=\'\' WHERE type!=10",
                            );
                            dbversion = 42;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                42,
                            );
                        }
                        if dbversion < 44 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN mime_headers TEXT;",
                            );
                            dbversion = 44;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                44,
                            );
                        }
                        if dbversion < 46 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN mime_references TEXT;",
                            );
                            dbversion = 46;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                46,
                            );
                        }
                        if dbversion < 47 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;",
                            );
                            dbversion = 47;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                47,
                            );
                        }
                        if dbversion < 48 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;",
                            );
                            assert_eq!(DC_MOVE_STATE_UNDEFINED as libc::c_int, 0);
                            assert_eq!(DC_MOVE_STATE_PENDING as libc::c_int, 1);
                            assert_eq!(DC_MOVE_STATE_STAY as libc::c_int, 2);
                            assert_eq!(DC_MOVE_STATE_MOVING as libc::c_int, 3);

                            dbversion = 48;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                48,
                            );
                        }
                        if dbversion < 49 {
                            dc_sqlite3_execute(
                                context, sql,
                                "ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;"
                                   );
                            dbversion = 49;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                49,
                            );
                        }
                        if dbversion < 50 {
                            if 0 != exists_before_update {
                                dc_sqlite3_set_config_int(
                                    context,
                                    sql,
                                    b"show_emails\x00" as *const u8 as *const libc::c_char,
                                    2,
                                );
                            }
                            dbversion = 50;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                50,
                            );
                        }
                        if dbversion < 53 {
                            dc_sqlite3_execute(
                                context, sql,
                                "CREATE TABLE locations ( id INTEGER PRIMARY KEY AUTOINCREMENT, latitude REAL DEFAULT 0.0, longitude REAL DEFAULT 0.0, accuracy REAL DEFAULT 0.0, timestamp INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0);"
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX locations_index1 ON locations (from_id);",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX locations_index2 ON locations (timestamp);",
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                               "ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;"
                                              );
                            dc_sqlite3_execute(
                                context, sql,
                                               "ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;"
                                              );
                            dc_sqlite3_execute(
                                context, sql,
                                               "ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;"
                                              );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX chats_index3 ON chats (locations_send_until);",
                            );
                            dbversion = 53;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                53,
                            );
                        }
                        if dbversion < 54 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;",
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "CREATE INDEX msgs_index6 ON msgs (location_id);",
                            );
                            dbversion = 54;
                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                54,
                            );
                        }
                        if dbversion < 55 {
                            dc_sqlite3_execute(
                                context,
                                sql,
                                "ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;",
                            );

                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                55,
                            );
                        }

                        if 0 != recalc_fingerprints {
                            let stmt =
                                dc_sqlite3_prepare(context, sql, "SELECT addr FROM acpeerstates;");
                            while sqlite3_step(stmt) == 100 {
                                if let Some(ref mut peerstate) = Peerstate::from_addr(
                                    context,
                                    sql,
                                    as_str(sqlite3_column_text(stmt, 0) as *const libc::c_char),
                                ) {
                                    peerstate.recalc_fingerprint();
                                    peerstate.save_to_db(sql, false);
                                }
                            }
                            sqlite3_finalize(stmt);
                        }
                        if 0 != update_file_paths {
                            let repl_from: *mut libc::c_char = dc_sqlite3_get_config(
                                context,
                                sql,
                                b"backup_for\x00" as *const u8 as *const libc::c_char,
                                context.get_blobdir(),
                            );
                            dc_ensure_no_slash(repl_from);

                            let mut q3: *mut libc::c_char =
                                sqlite3_mprintf(b"UPDATE msgs SET param=replace(param, \'f=%q/\', \'f=$BLOBDIR/\');\x00"
                                                as *const u8 as
                                                *const libc::c_char,
                                                repl_from);
                            dc_sqlite3_execute(context, sql, q3);
                            sqlite3_free(q3 as *mut libc::c_void);
                            q3 =
                                sqlite3_mprintf(b"UPDATE chats SET param=replace(param, \'i=%q/\', \'i=$BLOBDIR/\');\x00"
                                                as *const u8 as
                                                *const libc::c_char,
                                                repl_from);
                            dc_sqlite3_execute(context, sql, q3);
                            sqlite3_free(q3 as *mut libc::c_void);
                            free(repl_from as *mut libc::c_void);
                            dc_sqlite3_set_config(
                                context,
                                sql,
                                b"backup_for\x00" as *const u8 as *const libc::c_char,
                                0 as *const libc::c_char,
                            );
                        }
                        current_block = 12024807525273687499;
                    }
                }
            } else {
                current_block = 12024807525273687499;
            }
            match current_block {
                13628706266672894061 => {}
                _ => {
                    dc_log_info(
                        context,
                        0,
                        b"Opened \"%s\".\x00" as *const u8 as *const libc::c_char,
                        dbfile,
                    );
                    return 1;
                }
            }
        }
    }

    dc_sqlite3_close(context, sql);
    0
}

// handle configurations, private
pub fn dc_sqlite3_set_config(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: impl AsRef<str>,
    value: Option<impl AsRef<str>>,
) -> libc::c_int {
    let key = key.as_ref();
    let mut state;
    if 0 == dc_sqlite3_is_open(sql) {
        dc_log_error(context, 0, "dc_sqlite3_set_config(): Database not ready.");
        return 0;
    }
    if let Some(ref value) = value {
        let mut stmt =
            dc_sqlite3_prepare(context, sql, "SELECT value FROM config WHERE keyname=?;");
        sqlite3_bind_text(stmt, 1, key, -1, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        if state == 101 {
            stmt = dc_sqlite3_prepare(
                context,
                sql,
                "INSERT INTO config (keyname, value) VALUES (?, ?);",
            );
            sqlite3_bind_text(stmt, 1, key, -1, None);
            sqlite3_bind_text(stmt, 2, value, -1, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else if state == 100 {
            stmt = dc_sqlite3_prepare(context, sql, "UPDATE config SET value=? WHERE keyname=?;");
            sqlite3_bind_text(stmt, 1, value, -1, None);
            sqlite3_bind_text(stmt, 2, key, -1, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else {
            error!(context, 0, "dc_sqlite3_set_config(): Cannot read value.",);
            return 0;
        }
    } else {
        let stmt = dc_sqlite3_prepare(context, sql, "DELETE FROM config WHERE keyname=?;");
        sqlite3_bind_text(stmt, 1, key, -1, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    }
    if state != 101 {
        error!(context, 0, "dc_sqlite3_set_config(): Cannot change value.",);
        return 0;
    }

    1
}

/* tools, these functions are compatible to the corresponding sqlite3_* functions */
/* the result mus be freed using sqlite3_finalize() */
pub fn dc_sqlite3_prepare<'a>(
    context: &Context,
    sql: &dc_sqlite3_t,
    querystr: &'a str,
) -> Option<Statement<'a>> {
    if let Some(ref conn) = sql.conn() {
        match conn.prepare(querystr) {
            Ok(s) => Some(s),
            Err(err) => {
                error!(context, 0, "Query failed: {} ({})", querystr.as_ref(), err);
                None
            }
        }
    } else {
        None
    }
}

pub fn dc_sqlite3_is_open(sql: &dc_sqlite3_t) -> libc::c_int {
    sql.raw().is_none() as libc::c_int
}

/* the returned string must be free()'d, returns NULL on errors */
pub fn dc_sqlite3_get_config(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: impl AsRef<str>,
    def: Option<&str>,
) -> Option<String> {
    if 0 == dc_sqlite3_is_open(sql) || key.is_null() {
        return None;
    }
    let stmt = dc_sqlite3_prepare(context, sql, "SELECT value FROM config WHERE keyname=?;");
    sqlite3_bind_text(stmt, 1, key, -1, None);
    if sqlite3_step(stmt) == 100 {
        let ptr: *const libc::c_uchar = sqlite3_column_text(stmt, 0);
        if !ptr.is_null() {
            let ret: *mut libc::c_char = dc_strdup(ptr as *const libc::c_char);
            sqlite3_finalize(stmt);
            return ret;
        }
    }
    sqlite3_finalize(stmt);
    Some(def)
}

pub fn dc_sqlite3_execute(
    context: &Context,
    sql: &dc_sqlite3_t,
    querystr: impl AsRef<str>,
) -> libc::c_int {
    if let Some(stmt) = dc_sqlite3_prepare(context, sql, querystr) {
        match stmt.execute() {
            Ok(_) => 1,
            Err(err) => {
                error!(
                    context,
                    0,
                    "Cannot execute \"{}\". ({})",
                    querystr.as_ref(),
                    err
                );
                0
            }
        }
    } else {
        0
    }
}

pub fn dc_sqlite3_query_row<T>(
    context: &Context,
    sql: &dc_sqlite3_t,
    query: &str,
    column: usize,
) -> Option<T> {
    if let Some(ref conn) = sql.conn() {
        match conn.query_row(query, NO_PARAMS, |row| row.get(column)) {
            Ok(res) => Some(res),
            Err(err) => {
                error!(context, 0, "sql: Failed query_row: {}", err);
                None
            }
        }
    }
}

pub fn dc_sqlite3_set_config_int(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: impl AsRef<str>,
    value: i32,
) -> libc::c_int {
    dc_sqlite3_set_config(context, sql, key, format!("{}", value))
}

pub fn dc_sqlite3_get_config_int(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: impl AsRef<str>,
    def: i32,
) -> i32 {
    let s = dc_sqlite3_get_config(context, sql, key, None);
    s.parse().unwrap_or_else(|_| def)
}

pub fn dc_sqlite3_table_exists(
    context: &Context,
    sql: &dc_sqlite3_t,
    name: impl AsRef<str>,
) -> libc::c_int {
    match sql.conn() {
        Some(ref conn) => {
            conn.pragma(None, "table_info", name.as_ref(), |row| {
                // will only be executed if the info was found
                println!("row: {:?}", row.get(0));
                Ok(())
            })
            .map(|_| 1)
            .unwrap_or_else(|_| 0)
        }
        None => 0,
    }
}

pub fn dc_sqlite3_set_config_int64(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: *const libc::c_char,
    value: i64,
) -> libc::c_int {
    dc_sqlite3_set_config(context, sql, key, format!("{}", value));
}

pub fn dc_sqlite3_get_config_int64(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: impl AsRef<str>,
    def: Option<i64>,
) -> i64 {
    let ret = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    ret.map(|r| r.parse().unwrap_or_default())
        .unwrap_or_else(|_| def.unwrap_or_default())
}

pub fn dc_sqlite3_try_execute(
    context: &Context,
    sql: &dc_sqlite3_t,
    querystr: impl AsRef<str>,
) -> libc::c_int {
    // same as dc_sqlite3_execute() but does not pass error to ui
    if let Some(stmt) = dc_sqlite3_prepare(context, sql, querystr) {
        match stmt.execute() {
            Ok(_) => 1,
            Err(err) => {
                warn!(
                    context,
                    0, "Try-execute for \"{}\" failed: {}", querystr, err,
                );
                0
            }
        }
    } else {
        0
    }
}

pub fn dc_sqlite3_get_rowid(
    context: &Context,
    sql: &dc_sqlite3_t,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: impl AsRef<str>,
) -> uint32_t {
    // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    // the ORDER BY ensures, this function always returns the most recent id,
    // eg. if a Message-ID is splitted into different messages.
    if let Some(ref conn) = sql.conn() {
        match conn.query_row(
            &format!(
                "SELECT id FROM ? WHERE {}=? ORDER BY id DESC",
                field.as_ref()
            ),
            &[table.as_ref(), value.as_ref()],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(err) => {
                error!(context, 0, "sql: Failed to retrieve rowid: {}", err);
                0
            }
        }
    }
}

pub fn dc_sqlite3_get_rowid2(
    context: &Context,
    sql: &dc_sqlite3_t,
    table: impl AsRef<str>,
    field: impl AsRef<str>,
    value: u64,
    field2: impl AsRef<str>,
    value2: u32,
) -> uint32_t {
    // same as dc_sqlite3_get_rowid() with a key over two columns
    if let Some(ref conn) = sql.conn() {
        match conn.query_row(
            &format!(
                "SELECT id FROM ? WHERE {}=? AND {}=? ORDER BY id DESC",
                field.as_ref(),
                field2.as_ref(),
            ),
            &[table.as_ref(), value, value2],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(err) => {
                error!(context, 0, "sql: Failed to retrieve rowid2: {}", err);
                0
            }
        }
    }
}

pub fn dc_housekeeping(context: &Context) {
    let mut files_in_use = HashSet::new();
    let mut path = 0 as *mut libc::c_char;
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
    let mut stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT value FROM config;",
    );
    match stmt.query_map(NO_PARAMS, |row| row.get(0)) {
        Ok(rows) => {
            for row in rows {
                maybe_add_file(&mut files_in_use, row);
            }
        }
        Err(err) => {
            warn!(context, 0, "sql: failed query: {}", err);
        }
    }
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

                if is_file_in_use(&mut files_in_use, 0 as *const libc::c_char, name_c.as_ptr())
                    || is_file_in_use(
                        &mut files_in_use,
                        b".increation\x00" as *const u8 as *const libc::c_char,
                        name_c.as_ptr(),
                    )
                    || is_file_in_use(
                        &mut files_in_use,
                        b".waveform\x00" as *const u8 as *const libc::c_char,
                        name_c.as_ptr(),
                    )
                    || is_file_in_use(
                        &mut files_in_use,
                        b"-preview.jpg\x00" as *const u8 as *const libc::c_char,
                        name_c.as_ptr(),
                    )
                {
                    continue;
                }
                unreferenced_count += 1;
                free(path as *mut libc::c_void);

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
                                "Housekeeping: Keeping new unreferenced file #{}: {}",
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
                    "Housekeeping: Deleting unreferenced file #{}: {}",
                    unreferenced_count,
                    entry.file_name()
                );
                dc_delete_file(context, path);
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

    free(path as *mut libc::c_void);
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

unsafe fn maybe_add_file(files_in_use: &mut HashSet<String>, file: *const libc::c_char) {
    if strncmp(
        file,
        b"$BLOBDIR/\x00" as *const u8 as *const libc::c_char,
        9,
    ) != 0
    {
        return;
    }
    let raw_name = to_string(&*file.offset(9isize) as *const libc::c_char);
    files_in_use.insert(raw_name);
}

fn maybe_add_from_param(
    context: &Context,
    files_in_use: &mut HashSet<String>,
    query: &str,
    param_id: libc::c_int,
) {
    let param = unsafe { dc_param_new() };

    if let Some(ref mut stmt) =
        dc_sqlite3_prepare(context, &context.sql.clone().read().unwrap(), query)
    {
        match stmt.query_row(NO_PARAMS, |row| {
            let v = to_cstring(row.get(0));
            unsafe {
                dc_param_set_packed(param, v.as_ptr() as *const libc::c_char);
                let file = dc_param_get(param, param_id, 0 as *const libc::c_char);
                if !file.is_null() {
                    maybe_add_file(files_in_use, file);
                    free(file as *mut libc::c_void);
                }
            }
            Ok(())
        }) {
            Ok(_) => {}
            Err(err) => {
                warn!(context, 0, "sql: failed to add_from_param: {}", err);
            }
        }
    }
    dc_param_unref(param);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_maybe_add_file() {
        let mut files = Default::default();
        unsafe { maybe_add_file(&mut files, b"$BLOBDIR/hello\x00" as *const u8 as *const _) };
        unsafe {
            maybe_add_file(
                &mut files,
                b"$BLOBDIR/world.txt\x00" as *const u8 as *const _,
            )
        };
        unsafe { maybe_add_file(&mut files, b"world2.txt\x00" as *const u8 as *const _) };

        assert!(files.contains("hello"));
        assert!(files.contains("world.txt"));
        assert!(!files.contains("world2.txt"));
    }

    #[test]
    fn test_is_file_in_use() {
        let mut files = Default::default();
        unsafe { maybe_add_file(&mut files, b"$BLOBDIR/hello\x00" as *const u8 as *const _) };
        unsafe {
            maybe_add_file(
                &mut files,
                b"$BLOBDIR/world.txt\x00" as *const u8 as *const _,
            )
        };
        unsafe { maybe_add_file(&mut files, b"world2.txt\x00" as *const u8 as *const _) };

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

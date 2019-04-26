use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_apeerstate::*;
use crate::dc_context::dc_context_t;
use crate::dc_hash::*;
use crate::dc_imap::dc_imap_t;
use crate::dc_log::*;
use crate::dc_lot::dc_lot_t;
use crate::dc_param::*;
use crate::dc_smtp::dc_smtp_t;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_sqlite3_t {
    pub cobj: *mut sqlite3,
    pub context: *mut dc_context_t,
}

pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;

#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_new(mut context: *mut dc_context_t) -> *mut dc_sqlite3_t {
    let mut sql: *mut dc_sqlite3_t = 0 as *mut dc_sqlite3_t;
    sql = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_sqlite3_t>() as libc::c_ulong,
    ) as *mut dc_sqlite3_t;
    if sql.is_null() {
        exit(24i32);
    }
    (*sql).context = context;
    return sql;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_unref(mut sql: *mut dc_sqlite3_t) {
    if sql.is_null() {
        return;
    }
    if !(*sql).cobj.is_null() {
        dc_sqlite3_close(sql);
    }
    free(sql as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_close(mut sql: *mut dc_sqlite3_t) {
    if sql.is_null() {
        return;
    }
    if !(*sql).cobj.is_null() {
        sqlite3_close((*sql).cobj);
        (*sql).cobj = 0 as *mut sqlite3
    }
    dc_log_info(
        (*sql).context,
        0i32,
        b"Database closed.\x00" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_open(
    mut sql: *mut dc_sqlite3_t,
    mut dbfile: *const libc::c_char,
    mut flags: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    if 0 != dc_sqlite3_is_open(sql) {
        return 0i32;
    }
    if !(sql.is_null() || dbfile.is_null()) {
        if sqlite3_threadsafe() == 0i32 {
            dc_log_error(
                (*sql).context,
                0i32,
                b"Sqlite3 compiled thread-unsafe; this is not supported.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else if !(*sql).cobj.is_null() {
            dc_log_error(
                (*sql).context,
                0i32,
                b"Cannot open, database \"%s\" already opened.\x00" as *const u8
                    as *const libc::c_char,
                dbfile,
            );
        } else if sqlite3_open_v2(
            dbfile,
            &mut (*sql).cobj,
            0x10000i32
                | if 0 != flags & 0x1i32 {
                    0x1i32
                } else {
                    0x2i32 | 0x4i32
                },
            0 as *const libc::c_char,
        ) != 0i32
        {
            dc_sqlite3_log_error(
                sql,
                b"Cannot open database \"%s\".\x00" as *const u8 as *const libc::c_char,
                dbfile,
            );
        } else {
            dc_sqlite3_execute(
                sql,
                b"PRAGMA secure_delete=on;\x00" as *const u8 as *const libc::c_char,
            );
            sqlite3_busy_timeout((*sql).cobj, 10i32 * 1000i32);
            if 0 == flags & 0x1i32 {
                let mut exists_before_update: libc::c_int = 0i32;
                let mut dbversion_before_update: libc::c_int = 0i32;
                /* Init tables to dbversion=0 */
                if 0 == dc_sqlite3_table_exists(
                    sql,
                    b"config\x00" as *const u8 as *const libc::c_char,
                ) {
                    dc_log_info(
                        (*sql).context,
                        0i32,
                        b"First time init: creating tables in \"%s\".\x00" as *const u8
                            as *const libc::c_char,
                        dbfile,
                    );
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE config (id INTEGER PRIMARY KEY, keyname TEXT, value TEXT);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX config_index1 ON config (keyname);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE contacts (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT DEFAULT \'\', addr TEXT DEFAULT \'\' COLLATE NOCASE, origin INTEGER DEFAULT 0, blocked INTEGER DEFAULT 0, last_seen INTEGER DEFAULT 0, param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX contacts_index1 ON contacts (name COLLATE NOCASE);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX contacts_index2 ON contacts (addr COLLATE NOCASE);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"INSERT INTO contacts (id,name,origin) VALUES (1,\'self\',262144), (2,\'device\',262144), (3,\'rsvd\',262144), (4,\'rsvd\',262144), (5,\'rsvd\',262144), (6,\'rsvd\',262144), (7,\'rsvd\',262144), (8,\'rsvd\',262144), (9,\'rsvd\',262144);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE chats (id INTEGER PRIMARY KEY AUTOINCREMENT,  type INTEGER DEFAULT 0, name TEXT DEFAULT \'\', draft_timestamp INTEGER DEFAULT 0, draft_txt TEXT DEFAULT \'\', blocked INTEGER DEFAULT 0, grpid TEXT DEFAULT \'\', param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX chats_index1 ON chats (grpid);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"INSERT INTO chats (id,type,name) VALUES (1,120,\'deaddrop\'), (2,120,\'rsvd\'), (3,120,\'trash\'), (4,120,\'msgs_in_creation\'), (5,120,\'starred\'), (6,120,\'archivedlink\'), (7,100,\'rsvd\'), (8,100,\'rsvd\'), (9,100,\'rsvd\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE msgs (id INTEGER PRIMARY KEY AUTOINCREMENT, rfc724_mid TEXT DEFAULT \'\', server_folder TEXT DEFAULT \'\', server_uid INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0, to_id INTEGER DEFAULT 0, timestamp INTEGER DEFAULT 0, type INTEGER DEFAULT 0, state INTEGER DEFAULT 0, msgrmsg INTEGER DEFAULT 1, bytes INTEGER DEFAULT 0, txt TEXT DEFAULT \'\', txt_raw TEXT DEFAULT \'\', param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index1 ON msgs (rfc724_mid);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index2 ON msgs (chat_id);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index3 ON msgs (timestamp);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index4 ON msgs (state);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"INSERT INTO msgs (id,msgrmsg,txt) VALUES (1,0,\'marker1\'), (2,0,\'rsvd\'), (3,0,\'rsvd\'), (4,0,\'rsvd\'), (5,0,\'rsvd\'), (6,0,\'rsvd\'), (7,0,\'rsvd\'), (8,0,\'rsvd\'), (9,0,\'daymarker\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, added_timestamp INTEGER, desired_timestamp INTEGER DEFAULT 0, action INTEGER, foreign_id INTEGER, param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX jobs_index1 ON jobs (desired_timestamp);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    if 0 == dc_sqlite3_table_exists(
                        sql,
                        b"config\x00" as *const u8 as *const libc::c_char,
                    ) || 0
                        == dc_sqlite3_table_exists(
                            sql,
                            b"contacts\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"chats\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"chats_contacts\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"msgs\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"jobs\x00" as *const u8 as *const libc::c_char,
                        )
                    {
                        dc_sqlite3_log_error(
                            sql,
                            b"Cannot create tables in new database \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            dbfile,
                        );
                        /* cannot create the tables - maybe we cannot write? */
                        current_block = 13628706266672894061;
                    } else {
                        dc_sqlite3_set_config_int(
                            sql,
                            b"dbversion\x00" as *const u8 as *const libc::c_char,
                            0i32,
                        );
                        current_block = 14072441030219150333;
                    }
                } else {
                    exists_before_update = 1i32;
                    dbversion_before_update = dc_sqlite3_get_config_int(
                        sql,
                        b"dbversion\x00" as *const u8 as *const libc::c_char,
                        0i32,
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
                        let mut recalc_fingerprints: libc::c_int = 0i32;
                        let mut update_file_paths: libc::c_int = 0i32;
                        if dbversion < 1i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT \'\');\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX leftgrps_index1 ON leftgrps (grpid);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 1i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                1i32,
                            );
                        }
                        if dbversion < 2i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT \'\';\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 2i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                2i32,
                            );
                        }
                        if dbversion < 7i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE keypairs ( id INTEGER PRIMARY KEY, addr TEXT DEFAULT \'\' COLLATE NOCASE, is_default INTEGER DEFAULT 0, private_key, public_key, created INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dbversion = 7i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                7i32,
                            );
                        }
                        if dbversion < 10i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE acpeerstates ( id INTEGER PRIMARY KEY, addr TEXT DEFAULT \'\' COLLATE NOCASE, last_seen INTEGER DEFAULT 0, last_seen_autocrypt INTEGER DEFAULT 0, public_key, prefer_encrypted INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 10i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                10i32,
                            );
                        }
                        if dbversion < 12i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 12i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                12i32,
                            );
                        }
                        if dbversion < 17i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX chats_index2 ON chats (archived);\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX msgs_index5 ON msgs (starred);\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 17i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                17i32,
                            );
                        }
                        if dbversion < 18i32 {
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN gossip_key;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 18i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                18i32,
                            );
                        }
                        if dbversion < 27i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 27i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                27i32,
                            );
                        }
                        if dbversion < 34i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT \'\';\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT \'\';\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            recalc_fingerprints = 1i32;
                            dbversion = 34i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                34i32,
                            );
                        }
                        if dbversion < 39i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT \'\', timestamp INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN verified_key;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT \'\';\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            if dbversion_before_update == 34i32 {
                                dc_sqlite3_execute(sql,
                                                   b"UPDATE acpeerstates SET verified_key=gossip_key, verified_key_fingerprint=gossip_key_fingerprint WHERE gossip_key_verified=2;\x00"
                                                       as *const u8 as
                                                       *const libc::c_char);
                                dc_sqlite3_execute(sql,
                                                   b"UPDATE acpeerstates SET verified_key=public_key, verified_key_fingerprint=public_key_fingerprint WHERE public_key_verified=2;\x00"
                                                       as *const u8 as
                                                       *const libc::c_char);
                            }
                            dbversion = 39i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                39i32,
                            );
                        }
                        if dbversion < 40i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 40i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                40i32,
                            );
                        }
                        if dbversion < 41i32 {
                            update_file_paths = 1i32;
                            dbversion = 41i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                41i32,
                            );
                        }
                        if dbversion < 42i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"UPDATE msgs SET txt=\'\' WHERE type!=10\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 42i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                42i32,
                            );
                        }
                        if dbversion < 44i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN mime_headers TEXT;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 44i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                44i32,
                            );
                        }
                        if dbversion < 46i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN mime_references TEXT;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 46i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                46i32,
                            );
                        }
                        if dbversion < 47i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 47i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                47i32,
                            );
                        }
                        if dbversion < 48i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            if 0 != !(DC_MOVE_STATE_UNDEFINED as libc::c_int == 0i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    559i32,
                                    b"DC_MOVE_STATE_UNDEFINED == 0\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_PENDING as libc::c_int == 1i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    560i32,
                                    b"DC_MOVE_STATE_PENDING == 1\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_STAY as libc::c_int == 2i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    561i32,
                                    b"DC_MOVE_STATE_STAY == 2\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_MOVING as libc::c_int == 3i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    562i32,
                                    b"DC_MOVE_STATE_MOVING == 3\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            dbversion = 48i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                48i32,
                            );
                        }
                        if dbversion < 49i32 {
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dbversion = 49i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                49i32,
                            );
                        }
                        if dbversion < 50i32 {
                            if 0 != exists_before_update {
                                dc_sqlite3_set_config_int(
                                    sql,
                                    b"show_emails\x00" as *const u8 as *const libc::c_char,
                                    2i32,
                                );
                            }
                            dbversion = 50i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                50i32,
                            );
                        }
                        if dbversion < 53i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE locations ( id INTEGER PRIMARY KEY AUTOINCREMENT, latitude REAL DEFAULT 0.0, longitude REAL DEFAULT 0.0, accuracy REAL DEFAULT 0.0, timestamp INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX locations_index1 ON locations (from_id);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX locations_index2 ON locations (timestamp);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX chats_index3 ON chats (locations_send_until);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 53i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                53i32,
                            );
                        }
                        if dbversion < 54i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX msgs_index6 ON msgs (location_id);\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 54i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                54i32,
                            );
                        }
                        if 0 != recalc_fingerprints {
                            let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
                                sql,
                                b"SELECT addr FROM acpeerstates;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            while sqlite3_step(stmt) == 100i32 {
                                let mut peerstate: *mut dc_apeerstate_t =
                                    dc_apeerstate_new((*sql).context);
                                if 0 != dc_apeerstate_load_by_addr(
                                    peerstate,
                                    sql,
                                    sqlite3_column_text(stmt, 0i32) as *const libc::c_char,
                                ) && 0 != dc_apeerstate_recalc_fingerprint(peerstate)
                                {
                                    dc_apeerstate_save_to_db(peerstate, sql, 0i32);
                                }
                                dc_apeerstate_unref(peerstate);
                            }
                            sqlite3_finalize(stmt);
                        }
                        if 0 != update_file_paths {
                            let mut repl_from: *mut libc::c_char = dc_sqlite3_get_config(
                                sql,
                                b"backup_for\x00" as *const u8 as *const libc::c_char,
                                (*(*sql).context).blobdir,
                            );
                            dc_ensure_no_slash(repl_from);
                            if 0 != !('f' as i32 == 'f' as i32) as libc::c_int as libc::c_long {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    656i32,
                                    b"\'f\'==DC_PARAM_FILE\x00" as *const u8 as *const libc::c_char,
                                );
                            } else {
                            };
                            let mut q3: *mut libc::c_char =
                                sqlite3_mprintf(b"UPDATE msgs SET param=replace(param, \'f=%q/\', \'f=$BLOBDIR/\');\x00"
                                                    as *const u8 as
                                                    *const libc::c_char,
                                                repl_from);
                            dc_sqlite3_execute(sql, q3);
                            sqlite3_free(q3 as *mut libc::c_void);
                            if 0 != !('i' as i32 == 'i' as i32) as libc::c_int as libc::c_long {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    661i32,
                                    b"\'i\'==DC_PARAM_PROFILE_IMAGE\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            q3 =
                                sqlite3_mprintf(b"UPDATE chats SET param=replace(param, \'i=%q/\', \'i=$BLOBDIR/\');\x00"
                                                    as *const u8 as
                                                    *const libc::c_char,
                                                repl_from);
                            dc_sqlite3_execute(sql, q3);
                            sqlite3_free(q3 as *mut libc::c_void);
                            free(repl_from as *mut libc::c_void);
                            dc_sqlite3_set_config(
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
                        (*sql).context,
                        0i32,
                        b"Opened \"%s\".\x00" as *const u8 as *const libc::c_char,
                        dbfile,
                    );
                    return 1i32;
                }
            }
        }
    }
    dc_sqlite3_close(sql);
    return 0i32;
}
/* handle configurations, private */
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_set_config(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut value: *const libc::c_char,
) -> libc::c_int {
    let mut state: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if key.is_null() {
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_set_config(): Bad parameter.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if 0 == dc_sqlite3_is_open(sql) {
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_set_config(): Database not ready.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if !value.is_null() {
        stmt = dc_sqlite3_prepare(
            sql,
            b"SELECT value FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        if state == 101i32 {
            stmt = dc_sqlite3_prepare(
                sql,
                b"INSERT INTO config (keyname, value) VALUES (?, ?);\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
            sqlite3_bind_text(stmt, 2i32, value, -1i32, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else if state == 100i32 {
            stmt = dc_sqlite3_prepare(
                sql,
                b"UPDATE config SET value=? WHERE keyname=?;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_text(stmt, 1i32, value, -1i32, None);
            sqlite3_bind_text(stmt, 2i32, key, -1i32, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else {
            dc_log_error(
                (*sql).context,
                0i32,
                b"dc_sqlite3_set_config(): Cannot read value.\x00" as *const u8
                    as *const libc::c_char,
            );
            return 0i32;
        }
    } else {
        stmt = dc_sqlite3_prepare(
            sql,
            b"DELETE FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    }
    if state != 101i32 {
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_set_config(): Cannot change value.\x00" as *const u8
                as *const libc::c_char,
        );
        return 0i32;
    }
    return 1i32;
}
/* tools, these functions are compatible to the corresponding sqlite3_* functions */
/* the result mus be freed using sqlite3_finalize() */
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_prepare(
    mut sql: *mut dc_sqlite3_t,
    mut querystr: *const libc::c_char,
) -> *mut sqlite3_stmt {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if sql.is_null() || querystr.is_null() || (*sql).cobj.is_null() {
        return 0 as *mut sqlite3_stmt;
    }
    if sqlite3_prepare_v2(
        (*sql).cobj,
        querystr,
        -1i32,
        &mut stmt,
        0 as *mut *const libc::c_char,
    ) != 0i32
    {
        dc_sqlite3_log_error(
            sql,
            b"Query failed: %s\x00" as *const u8 as *const libc::c_char,
            querystr,
        );
        return 0 as *mut sqlite3_stmt;
    }
    return stmt;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_log_error(
    mut sql: *mut dc_sqlite3_t,
    mut msg_format: *const libc::c_char,
    mut va: ...
) {
    let mut msg: *mut libc::c_char = 0 as *mut libc::c_char;
    if sql.is_null() || msg_format.is_null() {
        return;
    }
    msg = sqlite3_vmprintf(msg_format, va);
    dc_log_error(
        (*sql).context,
        0i32,
        b"%s SQLite says: %s\x00" as *const u8 as *const libc::c_char,
        if !msg.is_null() {
            msg
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        if !(*sql).cobj.is_null() {
            sqlite3_errmsg((*sql).cobj)
        } else {
            b"SQLite object not set up.\x00" as *const u8 as *const libc::c_char
        },
    );
    sqlite3_free(msg as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_is_open(mut sql: *const dc_sqlite3_t) -> libc::c_int {
    if sql.is_null() || (*sql).cobj.is_null() {
        return 0i32;
    }
    return 1i32;
}
/* the returned string must be free()'d, returns NULL on errors */
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_config(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut def: *const libc::c_char,
) -> *mut libc::c_char {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if 0 == dc_sqlite3_is_open(sql) || key.is_null() {
        return dc_strdup_keep_null(def);
    }
    stmt = dc_sqlite3_prepare(
        sql,
        b"SELECT value FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
    if sqlite3_step(stmt) == 100i32 {
        let mut ptr: *const libc::c_uchar = sqlite3_column_text(stmt, 0i32);
        if !ptr.is_null() {
            let mut ret: *mut libc::c_char = dc_strdup(ptr as *const libc::c_char);
            sqlite3_finalize(stmt);
            return ret;
        }
    }
    sqlite3_finalize(stmt);
    return dc_strdup_keep_null(def);
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_execute(
    mut sql: *mut dc_sqlite3_t,
    mut querystr: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut sqlState: libc::c_int = 0i32;
    stmt = dc_sqlite3_prepare(sql, querystr);
    if !stmt.is_null() {
        sqlState = sqlite3_step(stmt);
        if sqlState != 101i32 && sqlState != 100i32 {
            dc_sqlite3_log_error(
                sql,
                b"Cannot execute \"%s\".\x00" as *const u8 as *const libc::c_char,
                querystr,
            );
        } else {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_set_config_int(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut value: int32_t,
) -> libc::c_int {
    let mut value_str: *mut libc::c_char = dc_mprintf(
        b"%i\x00" as *const u8 as *const libc::c_char,
        value as libc::c_int,
    );
    if value_str.is_null() {
        return 0i32;
    }
    let mut ret: libc::c_int = dc_sqlite3_set_config(sql, key, value_str);
    free(value_str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_config_int(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut def: int32_t,
) -> int32_t {
    let mut str: *mut libc::c_char = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    if str.is_null() {
        return def;
    }
    let mut ret: int32_t = atol(str) as int32_t;
    free(str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_table_exists(
    mut sql: *mut dc_sqlite3_t,
    mut name: *const libc::c_char,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut querystr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut sqlState: libc::c_int = 0i32;
    querystr = sqlite3_mprintf(
        b"PRAGMA table_info(%s)\x00" as *const u8 as *const libc::c_char,
        name,
    );
    if querystr.is_null() {
        /* this statement cannot be used with binded variables */
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_table_exists_(): Out of memory.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        stmt = dc_sqlite3_prepare(sql, querystr);
        if !stmt.is_null() {
            sqlState = sqlite3_step(stmt);
            if sqlState == 100i32 {
                ret = 1i32
            }
        }
    }
    /* error/cleanup */
    if !stmt.is_null() {
        sqlite3_finalize(stmt);
    }
    if !querystr.is_null() {
        sqlite3_free(querystr as *mut libc::c_void);
    }
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_set_config_int64(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut value: int64_t,
) -> libc::c_int {
    let mut value_str: *mut libc::c_char = dc_mprintf(
        b"%lld\x00" as *const u8 as *const libc::c_char,
        value as libc::c_long,
    );
    if value_str.is_null() {
        return 0i32;
    }
    let mut ret: libc::c_int = dc_sqlite3_set_config(sql, key, value_str);
    free(value_str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_config_int64(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut def: int64_t,
) -> int64_t {
    let mut str: *mut libc::c_char = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    if str.is_null() {
        return def;
    }
    let mut ret: int64_t = 0i32 as int64_t;
    sscanf(
        str,
        b"%lld\x00" as *const u8 as *const libc::c_char,
        &mut ret as *mut int64_t,
    );
    free(str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_try_execute(
    mut sql: *mut dc_sqlite3_t,
    mut querystr: *const libc::c_char,
) -> libc::c_int {
    // same as dc_sqlite3_execute() but does not pass error to ui
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut sql_state: libc::c_int = 0i32;
    stmt = dc_sqlite3_prepare(sql, querystr);
    if !stmt.is_null() {
        sql_state = sqlite3_step(stmt);
        if sql_state != 101i32 && sql_state != 100i32 {
            dc_log_warning(
                (*sql).context,
                0i32,
                b"Try-execute for \"%s\" failed: %s\x00" as *const u8 as *const libc::c_char,
                querystr,
                sqlite3_errmsg((*sql).cobj),
            );
        } else {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_rowid(
    mut sql: *mut dc_sqlite3_t,
    mut table: *const libc::c_char,
    mut field: *const libc::c_char,
    mut value: *const libc::c_char,
) -> uint32_t {
    // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    // the ORDER BY ensures, this function always returns the most recent id,
    // eg. if a Message-ID is splitted into different messages.
    let mut id: uint32_t = 0i32 as uint32_t;
    let mut q3: *mut libc::c_char = sqlite3_mprintf(
        b"SELECT id FROM %s WHERE %s=%Q ORDER BY id DESC;\x00" as *const u8 as *const libc::c_char,
        table,
        field,
        value,
    );
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(sql, q3);
    if 100i32 == sqlite3_step(stmt) {
        id = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    return id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_rowid2(
    mut sql: *mut dc_sqlite3_t,
    mut table: *const libc::c_char,
    mut field: *const libc::c_char,
    mut value: uint64_t,
    mut field2: *const libc::c_char,
    mut value2: uint32_t,
) -> uint32_t {
    // same as dc_sqlite3_get_rowid() with a key over two columns
    let mut id: uint32_t = 0i32 as uint32_t;
    // see https://www.sqlite.org/printf.html for sqlite-printf modifiers
    let mut q3: *mut libc::c_char = sqlite3_mprintf(
        b"SELECT id FROM %s WHERE %s=%lli AND %s=%i ORDER BY id DESC;\x00" as *const u8
            as *const libc::c_char,
        table,
        field,
        value,
        field2,
        value2,
    );
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(sql, q3);
    if 100i32 == sqlite3_step(stmt) {
        id = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    return id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_begin_transaction(mut sql: *mut dc_sqlite3_t) {}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_commit(mut sql: *mut dc_sqlite3_t) {}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_rollback(mut sql: *mut dc_sqlite3_t) {}
/* housekeeping */
#[no_mangle]
pub unsafe extern "C" fn dc_housekeeping(mut context: *mut dc_context_t) {
    let mut keep_files_newer_than: time_t = 0;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut files_in_use: dc_hash_t = dc_hash_t {
        keyClass: 0,
        copyKey: 0,
        count: 0,
        first: 0 as *mut dc_hashelem_t,
        htsize: 0,
        ht: 0 as *mut _ht,
    };
    let mut path: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut unreferenced_count: libc::c_int = 0i32;
    dc_hash_init(&mut files_in_use, 3i32, 1i32);
    dc_log_info(
        context,
        0i32,
        b"Start housekeeping...\x00" as *const u8 as *const libc::c_char,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM msgs  WHERE chat_id!=3   AND type!=10;\x00" as *const u8
            as *const libc::c_char,
        'f' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM jobs;\x00" as *const u8 as *const libc::c_char,
        'f' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM chats;\x00" as *const u8 as *const libc::c_char,
        'i' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM contacts;\x00" as *const u8 as *const libc::c_char,
        'i' as i32,
    );
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT value FROM config;\x00" as *const u8 as *const libc::c_char,
    );
    while sqlite3_step(stmt) == 100i32 {
        maybe_add_file(
            &mut files_in_use,
            sqlite3_column_text(stmt, 0i32) as *const libc::c_char,
        );
    }
    dc_log_info(
        context,
        0i32,
        b"%i files in use.\x00" as *const u8 as *const libc::c_char,
        files_in_use.count as libc::c_int,
    );
    /* go through directory and delete unused files */
    dir_handle = opendir((*context).blobdir);
    if dir_handle.is_null() {
        dc_log_warning(
            context,
            0i32,
            b"Housekeeping: Cannot open %s.\x00" as *const u8 as *const libc::c_char,
            (*context).blobdir,
        );
    } else {
        /* avoid deletion of files that are just created to build a message object */
        keep_files_newer_than = time(0 as *mut time_t) - (60i32 * 60i32) as libc::c_long;
        loop {
            dir_entry = readdir(dir_handle);
            if dir_entry.is_null() {
                break;
            }
            /* name without path or `.` or `..` */
            let mut name: *const libc::c_char = (*dir_entry).d_name.as_mut_ptr();
            let mut name_len: libc::c_int = strlen(name) as libc::c_int;
            if name_len == 1i32 && *name.offset(0isize) as libc::c_int == '.' as i32
                || name_len == 2i32
                    && *name.offset(0isize) as libc::c_int == '.' as i32
                    && *name.offset(1isize) as libc::c_int == '.' as i32
            {
                continue;
            }
            if 0 != is_file_in_use(&mut files_in_use, 0 as *const libc::c_char, name)
                || 0 != is_file_in_use(
                    &mut files_in_use,
                    b".increation\x00" as *const u8 as *const libc::c_char,
                    name,
                )
                || 0 != is_file_in_use(
                    &mut files_in_use,
                    b".waveform\x00" as *const u8 as *const libc::c_char,
                    name,
                )
                || 0 != is_file_in_use(
                    &mut files_in_use,
                    b"-preview.jpg\x00" as *const u8 as *const libc::c_char,
                    name,
                )
            {
                continue;
            }
            unreferenced_count += 1;
            free(path as *mut libc::c_void);
            path = dc_mprintf(
                b"%s/%s\x00" as *const u8 as *const libc::c_char,
                (*context).blobdir,
                name,
            );
            let mut st: stat = stat {
                st_dev: 0,
                st_mode: 0,
                st_nlink: 0,
                st_ino: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                st_atimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_mtimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_ctimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_birthtimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_size: 0,
                st_blocks: 0,
                st_blksize: 0,
                st_flags: 0,
                st_gen: 0,
                st_lspare: 0,
                st_qspare: [0; 2],
            };
            if stat(path, &mut st) == 0i32 {
                if st.st_mtimespec.tv_sec > keep_files_newer_than
                    || st.st_atimespec.tv_sec > keep_files_newer_than
                    || st.st_ctimespec.tv_sec > keep_files_newer_than
                {
                    dc_log_info(
                        context,
                        0i32,
                        b"Housekeeping: Keeping new unreferenced file #%i: %s\x00" as *const u8
                            as *const libc::c_char,
                        unreferenced_count,
                        name,
                    );
                    continue;
                }
            }
            dc_log_info(
                context,
                0i32,
                b"Housekeeping: Deleting unreferenced file #%i: %s\x00" as *const u8
                    as *const libc::c_char,
                unreferenced_count,
                name,
            );
            dc_delete_file(context, path);
        }
    }
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    sqlite3_finalize(stmt);
    dc_hash_clear(&mut files_in_use);
    free(path as *mut libc::c_void);
    dc_log_info(
        context,
        0i32,
        b"Housekeeping done.\x00" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn is_file_in_use(
    mut files_in_use: *mut dc_hash_t,
    mut namespc: *const libc::c_char,
    mut name: *const libc::c_char,
) -> libc::c_int {
    let mut name_to_check: *mut libc::c_char = dc_strdup(name);
    if !namespc.is_null() {
        let mut name_len: libc::c_int = strlen(name) as libc::c_int;
        let mut namespc_len: libc::c_int = strlen(namespc) as libc::c_int;
        if name_len <= namespc_len
            || strcmp(&*name.offset((name_len - namespc_len) as isize), namespc) != 0i32
        {
            return 0i32;
        }
        *name_to_check.offset((name_len - namespc_len) as isize) = 0i32 as libc::c_char
    }
    let mut ret: libc::c_int = (dc_hash_find(
        files_in_use,
        name_to_check as *const libc::c_void,
        strlen(name_to_check) as libc::c_int,
    ) != 0 as *mut libc::c_void) as libc::c_int;
    free(name_to_check as *mut libc::c_void);
    return ret;
}
/* ******************************************************************************
 * Housekeeping
 ******************************************************************************/
unsafe extern "C" fn maybe_add_file(
    mut files_in_use: *mut dc_hash_t,
    mut file: *const libc::c_char,
) {
    if strncmp(
        file,
        b"$BLOBDIR/\x00" as *const u8 as *const libc::c_char,
        9i32 as libc::c_ulong,
    ) != 0i32
    {
        return;
    }
    let mut raw_name: *const libc::c_char = &*file.offset(9isize) as *const libc::c_char;
    dc_hash_insert(
        files_in_use,
        raw_name as *const libc::c_void,
        strlen(raw_name) as libc::c_int,
        1i32 as *mut libc::c_void,
    );
}
unsafe extern "C" fn maybe_add_from_param(
    mut context: *mut dc_context_t,
    mut files_in_use: *mut dc_hash_t,
    mut query: *const libc::c_char,
    mut param_id: libc::c_int,
) {
    let mut param: *mut dc_param_t = dc_param_new();
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare((*context).sql, query);
    while sqlite3_step(stmt) == 100i32 {
        dc_param_set_packed(
            param,
            sqlite3_column_text(stmt, 0i32) as *const libc::c_char,
        );
        let mut file: *mut libc::c_char = dc_param_get(param, param_id, 0 as *const libc::c_char);
        if !file.is_null() {
            maybe_add_file(files_in_use, file);
            free(file as *mut libc::c_void);
        }
    }
    sqlite3_finalize(stmt);
    dc_param_unref(param);
}

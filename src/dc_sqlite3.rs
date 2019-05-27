use std::collections::HashSet;

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
#[repr(C)]
pub struct dc_sqlite3_t {
    pub cobj: *mut sqlite3,
}

pub fn dc_sqlite3_new() -> dc_sqlite3_t {
    dc_sqlite3_t {
        cobj: std::ptr::null_mut(),
    }
}

pub unsafe fn dc_sqlite3_unref(context: &Context, sql: &mut dc_sqlite3_t) {
    if !sql.cobj.is_null() {
        dc_sqlite3_close(context, sql);
    }
}

pub unsafe fn dc_sqlite3_close(context: &Context, sql: &mut dc_sqlite3_t) {
    if !sql.cobj.is_null() {
        sqlite3_close(sql.cobj);
        sql.cobj = 0 as *mut sqlite3
    }

    dc_log_info(
        context,
        0,
        b"Database closed.\x00" as *const u8 as *const libc::c_char,
    );
}

pub unsafe fn dc_sqlite3_open(
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
        if sqlite3_threadsafe() == 0 {
            dc_log_error(
                context,
                0,
                b"Sqlite3 compiled thread-unsafe; this is not supported.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else if !sql.cobj.is_null() {
            dc_log_error(
                context,
                0,
                b"Cannot open, database \"%s\" already opened.\x00" as *const u8
                    as *const libc::c_char,
                dbfile,
            );
        } else if sqlite3_open_v2(
            dbfile,
            &mut sql.cobj,
            SQLITE_OPEN_FULLMUTEX
                | (if 0 != (flags & DC_OPEN_READONLY as i32) {
                    SQLITE_OPEN_READONLY
                } else {
                    SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
                }),
            std::ptr::null(),
        ) != 0
        {
            dc_sqlite3_log_error(
                context,
                sql,
                b"Cannot open database \"%s\".\x00" as *const u8 as *const libc::c_char,
                dbfile,
            );
        } else {
            dc_sqlite3_execute(
                context,
                sql,
                b"PRAGMA secure_delete=on;\x00" as *const u8 as *const libc::c_char,
            );
            sqlite3_busy_timeout(sql.cobj, 10 * 1000);
            if 0 == flags & DC_OPEN_READONLY as i32 {
                let mut exists_before_update = 0;
                let mut dbversion_before_update = 0;
                /* Init tables to dbversion=0 */
                if 0 == dc_sqlite3_table_exists(
                    context,
                    sql,
                    b"config\x00" as *const u8 as *const libc::c_char,
                ) {
                    dc_log_info(
                        context,
                        0,
                        b"First time init: creating tables in \"%s\".\x00" as *const u8
                            as *const libc::c_char,
                        dbfile,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE TABLE config (id INTEGER PRIMARY KEY, keyname TEXT, value TEXT);\x00"
                            as *const u8 as
                            *const libc::c_char
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX config_index1 ON config (keyname);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE TABLE contacts (\
                          id INTEGER PRIMARY KEY AUTOINCREMENT, \
                          name TEXT DEFAULT \'\', \
                          addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                          origin INTEGER DEFAULT 0, \
                          blocked INTEGER DEFAULT 0, \
                          last_seen INTEGER DEFAULT 0, \
                          param TEXT DEFAULT \'\');\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX contacts_index1 ON contacts (name COLLATE NOCASE);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX contacts_index2 ON contacts (addr COLLATE NOCASE);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"INSERT INTO contacts (id,name,origin) VALUES \
                          (1,\'self\',262144), (2,\'device\',262144), (3,\'rsvd\',262144), \
                          (4,\'rsvd\',262144), (5,\'rsvd\',262144), (6,\'rsvd\',262144), \
                          (7,\'rsvd\',262144), (8,\'rsvd\',262144), (9,\'rsvd\',262144);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE TABLE chats (\
                          id INTEGER PRIMARY KEY AUTOINCREMENT,  \
                          type INTEGER DEFAULT 0, \
                          name TEXT DEFAULT \'\', \
                          draft_timestamp INTEGER DEFAULT 0, \
                          draft_txt TEXT DEFAULT \'\', \
                          blocked INTEGER DEFAULT 0, \
                          grpid TEXT DEFAULT \'\', \
                          param TEXT DEFAULT \'\');\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX chats_index1 ON chats (grpid);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"INSERT INTO chats (id,type,name) VALUES \
                          (1,120,\'deaddrop\'), (2,120,\'rsvd\'), (3,120,\'trash\'), \
                          (4,120,\'msgs_in_creation\'), (5,120,\'starred\'), (6,120,\'archivedlink\'), \
                          (7,100,\'rsvd\'), (8,100,\'rsvd\'), (9,100,\'rsvd\');\x00"
                            as *const u8 as *const libc::c_char
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE TABLE msgs (\
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
                          param TEXT DEFAULT \'\');\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX msgs_index1 ON msgs (rfc724_mid);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX msgs_index2 ON msgs (chat_id);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX msgs_index3 ON msgs (timestamp);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX msgs_index4 ON msgs (state);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"INSERT INTO msgs (id,msgrmsg,txt) VALUES \
                          (1,0,\'marker1\'), (2,0,\'rsvd\'), (3,0,\'rsvd\'), \
                          (4,0,\'rsvd\'), (5,0,\'rsvd\'), (6,0,\'rsvd\'), (7,0,\'rsvd\'), \
                          (8,0,\'rsvd\'), (9,0,\'daymarker\');\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE TABLE jobs (\
                          id INTEGER PRIMARY KEY AUTOINCREMENT, \
                          added_timestamp INTEGER, \
                          desired_timestamp INTEGER DEFAULT 0, \
                          action INTEGER, \
                          foreign_id INTEGER, \
                          param TEXT DEFAULT \'\');\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        context,
                        sql,
                        b"CREATE INDEX jobs_index1 ON jobs (desired_timestamp);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    if 0 == dc_sqlite3_table_exists(
                        context,
                        sql,
                        b"config\x00" as *const u8 as *const libc::c_char,
                    ) || 0
                        == dc_sqlite3_table_exists(
                            context,
                            sql,
                            b"contacts\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            context,
                            sql,
                            b"chats\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            context,
                            sql,
                            b"chats_contacts\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            context,
                            sql,
                            b"msgs\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            context,
                            sql,
                            b"jobs\x00" as *const u8 as *const libc::c_char,
                        )
                    {
                        dc_sqlite3_log_error(
                            context,
                            sql,
                            b"Cannot create tables in new database \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            dbfile,
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
                                b"CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT \'\');\x00"
                                    as *const u8 as *const libc::c_char
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX leftgrps_index1 ON leftgrps (grpid);\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT \'\';\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"CREATE TABLE keypairs (\
                                  id INTEGER PRIMARY KEY, \
                                  addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                                  is_default INTEGER DEFAULT 0, \
                                  private_key, \
                                  public_key, \
                                  created INTEGER DEFAULT 0);\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"CREATE TABLE acpeerstates (\
                                  id INTEGER PRIMARY KEY, \
                                  addr TEXT DEFAULT \'\' COLLATE NOCASE, \
                                  last_seen INTEGER DEFAULT 0, \
                                  last_seen_autocrypt INTEGER DEFAULT 0, \
                                  public_key, \
                                  prefer_encrypted INTEGER DEFAULT 0);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                context, sql,
                                b"CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);\x00"
                                    as *const u8 as
                                    *const libc::c_char);
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
                                b"ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX chats_index2 ON chats (archived);\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX msgs_index5 ON msgs (starred);\x00" as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;\x00"
                                    as *const u8 as
                                    *const libc::c_char);
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN gossip_key;\x00" as *const u8
                                    as *const libc::c_char,
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
                                b"DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                b"CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);\x00"
                                    as *const u8 as
                                    *const libc::c_char
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                b"ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;\x00"
                                    as *const u8 as
                                    *const libc::c_char
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT \'\';\x00"
                                    as *const u8 as
                                    *const libc::c_char);
                            dc_sqlite3_execute(
                                context, sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT \'\';\x00"
                                    as *const u8 as
                                    *const libc::c_char);
                            dc_sqlite3_execute(
                                context, sql,
                                b"CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);\x00"
                                    as *const u8 as
                                    *const libc::c_char);
                            dc_sqlite3_execute(
                                context, sql,
                                b"CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);\x00"
                                    as *const u8 as
                                    *const libc::c_char);
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
                                b"CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT \'\', timestamp INTEGER DEFAULT 0);\x00"
                                    as *const u8 as
                                    *const libc::c_char);
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN verified_key;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context, sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT \'\';\x00"
                                    as *const u8 as
                                    *const libc::c_char);
                            dc_sqlite3_execute(
                                context, sql,
                                b"CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);\x00"
                                    as *const u8 as
                                    *const libc::c_char);
                            if dbversion_before_update == 34 {
                                dc_sqlite3_execute(
                                    context, sql,
                                    b"UPDATE acpeerstates SET verified_key=gossip_key, verified_key_fingerprint=gossip_key_fingerprint WHERE gossip_key_verified=2;\x00"
                                        as *const u8 as
                                        *const libc::c_char);
                                dc_sqlite3_execute(
                                    context, sql,
                                    b"UPDATE acpeerstates SET verified_key=public_key, verified_key_fingerprint=public_key_fingerprint WHERE public_key_verified=2;\x00"
                                        as *const u8 as
                                        *const libc::c_char);
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
                                b"ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"UPDATE msgs SET txt=\'\' WHERE type!=10\x00" as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE msgs ADD COLUMN mime_headers TEXT;\x00" as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN mime_references TEXT;\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            if 0 != !(DC_MOVE_STATE_UNDEFINED as libc::c_int == 0) as usize {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    559,
                                    b"DC_MOVE_STATE_UNDEFINED == 0\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_PENDING as libc::c_int == 1) as usize {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    560,
                                    b"DC_MOVE_STATE_PENDING == 1\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_STAY as libc::c_int == 2) as usize {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    561,
                                    b"DC_MOVE_STATE_STAY == 2\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_MOVING as libc::c_int == 3) as usize {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    562,
                                    b"DC_MOVE_STATE_MOVING == 3\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
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
                                b"ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;\x00"
                                    as *const u8 as
                                    *const libc::c_char);
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
                            dc_sqlite3_execute(context, sql,
                                               b"CREATE TABLE locations ( id INTEGER PRIMARY KEY AUTOINCREMENT, latitude REAL DEFAULT 0.0, longitude REAL DEFAULT 0.0, accuracy REAL DEFAULT 0.0, timestamp INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0);\x00"
                                               as *const u8 as
                                               *const libc::c_char);
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX locations_index1 ON locations (from_id);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX locations_index2 ON locations (timestamp);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(context, sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;\x00"
                                               as *const u8 as
                                               *const libc::c_char);
                            dc_sqlite3_execute(context, sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;\x00"
                                               as *const u8 as
                                               *const libc::c_char);
                            dc_sqlite3_execute(context, sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;\x00"
                                               as *const u8 as
                                               *const libc::c_char);
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX chats_index3 ON chats (locations_send_until);\x00"
                                    as *const u8
                                    as *const libc::c_char,
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
                                b"ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                context,
                                sql,
                                b"CREATE INDEX msgs_index6 ON msgs (location_id);\x00" as *const u8
                                    as *const libc::c_char,
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
                                context, sql,
                                b"ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;\x00" as *const u8 as *const libc::c_char
                            );

                            dc_sqlite3_set_config_int(
                                context,
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                55,
                            );
                        }

                        if 0 != recalc_fingerprints {
                            let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
                                context,
                                sql,
                                b"SELECT addr FROM acpeerstates;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            while sqlite3_step(stmt) == 100 {
                                if let Some(ref mut peerstate) = Peerstate::from_addr(
                                    context,
                                    sql,
                                    to_str(sqlite3_column_text(stmt, 0) as *const libc::c_char),
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
                            if 0 != !('f' as i32 == 'f' as i32) as usize {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    656,
                                    b"\'f\'==DC_PARAM_FILE\x00" as *const u8 as *const libc::c_char,
                                );
                            } else {
                            };
                            let mut q3: *mut libc::c_char =
                                sqlite3_mprintf(b"UPDATE msgs SET param=replace(param, \'f=%q/\', \'f=$BLOBDIR/\');\x00"
                                                as *const u8 as
                                                *const libc::c_char,
                                                repl_from);
                            dc_sqlite3_execute(context, sql, q3);
                            sqlite3_free(q3 as *mut libc::c_void);
                            if 0 != !('i' as i32 == 'i' as i32) as usize {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    661,
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
pub unsafe fn dc_sqlite3_set_config(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: *const libc::c_char,
    value: *const libc::c_char,
) -> libc::c_int {
    let mut state;
    let mut stmt;
    if key.is_null() {
        dc_log_error(
            context,
            0,
            b"dc_sqlite3_set_config(): Bad parameter.\x00" as *const u8 as *const libc::c_char,
        );
        return 0;
    }
    if 0 == dc_sqlite3_is_open(sql) {
        dc_log_error(
            context,
            0,
            b"dc_sqlite3_set_config(): Database not ready.\x00" as *const u8 as *const libc::c_char,
        );
        return 0;
    }
    if !value.is_null() {
        stmt = dc_sqlite3_prepare(
            context,
            sql,
            b"SELECT value FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1, key, -1, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        if state == 101 {
            stmt = dc_sqlite3_prepare(
                context,
                sql,
                b"INSERT INTO config (keyname, value) VALUES (?, ?);\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_text(stmt, 1, key, -1, None);
            sqlite3_bind_text(stmt, 2, value, -1, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else if state == 100 {
            stmt = dc_sqlite3_prepare(
                context,
                sql,
                b"UPDATE config SET value=? WHERE keyname=?;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_text(stmt, 1, value, -1, None);
            sqlite3_bind_text(stmt, 2, key, -1, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else {
            dc_log_error(
                context,
                0,
                b"dc_sqlite3_set_config(): Cannot read value.\x00" as *const u8
                    as *const libc::c_char,
            );
            return 0;
        }
    } else {
        stmt = dc_sqlite3_prepare(
            context,
            sql,
            b"DELETE FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1, key, -1, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    }
    if state != 101 {
        dc_log_error(
            context,
            0,
            b"dc_sqlite3_set_config(): Cannot change value.\x00" as *const u8
                as *const libc::c_char,
        );
        return 0;
    }

    1
}

/* tools, these functions are compatible to the corresponding sqlite3_* functions */
/* the result mus be freed using sqlite3_finalize() */
pub unsafe fn dc_sqlite3_prepare(
    context: &Context,
    sql: &dc_sqlite3_t,
    querystr: *const libc::c_char,
) -> *mut sqlite3_stmt {
    let mut stmt = 0 as *mut sqlite3_stmt;
    if querystr.is_null() || sql.cobj.is_null() {
        return 0 as *mut sqlite3_stmt;
    }
    if sqlite3_prepare_v2(
        sql.cobj,
        querystr,
        -1,
        &mut stmt,
        0 as *mut *const libc::c_char,
    ) != 0
    {
        dc_sqlite3_log_error(
            context,
            sql,
            b"Query failed: %s\x00" as *const u8 as *const libc::c_char,
            querystr,
        );
        return 0 as *mut sqlite3_stmt;
    }
    stmt
}

pub unsafe extern "C" fn dc_sqlite3_log_error(
    context: &Context,
    sql: &dc_sqlite3_t,
    msg_format: *const libc::c_char,
    va: ...
) {
    let msg;
    if msg_format.is_null() {
        return;
    }
    // FIXME: evil transmute
    msg = sqlite3_vmprintf(msg_format, std::mem::transmute(va));
    dc_log_error(
        context,
        0,
        b"%s SQLite says: %s\x00" as *const u8 as *const libc::c_char,
        if !msg.is_null() {
            msg
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        if !sql.cobj.is_null() {
            sqlite3_errmsg(sql.cobj)
        } else {
            b"SQLite object not set up.\x00" as *const u8 as *const libc::c_char
        },
    );
    sqlite3_free(msg as *mut libc::c_void);
}

pub unsafe fn dc_sqlite3_is_open(sql: &dc_sqlite3_t) -> libc::c_int {
    if sql.cobj.is_null() {
        0
    } else {
        1
    }
}

/* the returned string must be free()'d, returns NULL on errors */
pub unsafe fn dc_sqlite3_get_config(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: *const libc::c_char,
    def: *const libc::c_char,
) -> *mut libc::c_char {
    let stmt;
    if 0 == dc_sqlite3_is_open(sql) || key.is_null() {
        return dc_strdup_keep_null(def);
    }
    stmt = dc_sqlite3_prepare(
        context,
        sql,
        b"SELECT value FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
    );
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
    dc_strdup_keep_null(def)
}

pub unsafe fn dc_sqlite3_execute(
    context: &Context,
    sql: &dc_sqlite3_t,
    querystr: *const libc::c_char,
) -> libc::c_int {
    let mut success = 0;
    let sqlState;
    let stmt = dc_sqlite3_prepare(context, sql, querystr);
    if !stmt.is_null() {
        sqlState = sqlite3_step(stmt);
        if sqlState != 101 && sqlState != 100 {
            dc_sqlite3_log_error(
                context,
                sql,
                b"Cannot execute \"%s\".\x00" as *const u8 as *const libc::c_char,
                querystr,
            );
        } else {
            success = 1
        }
    }
    sqlite3_finalize(stmt);
    success
}

pub unsafe fn dc_sqlite3_set_config_int(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: *const libc::c_char,
    value: int32_t,
) -> libc::c_int {
    let value_str = dc_mprintf(
        b"%i\x00" as *const u8 as *const libc::c_char,
        value as libc::c_int,
    );
    if value_str.is_null() {
        return 0;
    }
    let ret = dc_sqlite3_set_config(context, sql, key, value_str);
    free(value_str as *mut libc::c_void);

    ret
}

pub unsafe fn dc_sqlite3_get_config_int(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: *const libc::c_char,
    def: int32_t,
) -> int32_t {
    let str = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    if str.is_null() {
        return def;
    }
    let ret = atoi(str) as int32_t;
    free(str as *mut libc::c_void);
    ret
}

pub unsafe fn dc_sqlite3_table_exists(
    context: &Context,
    sql: &dc_sqlite3_t,
    name: *const libc::c_char,
) -> libc::c_int {
    let mut ret = 0;
    let mut stmt = 0 as *mut sqlite3_stmt;
    let sqlState;

    let querystr = sqlite3_mprintf(
        b"PRAGMA table_info(%s)\x00" as *const u8 as *const libc::c_char,
        name,
    );
    if querystr.is_null() {
        /* this statement cannot be used with binded variables */
        dc_log_error(
            context,
            0,
            b"dc_sqlite3_table_exists_(): Out of memory.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        stmt = dc_sqlite3_prepare(context, sql, querystr);
        if !stmt.is_null() {
            sqlState = sqlite3_step(stmt);
            if sqlState == 100 {
                ret = 1
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
    ret
}

pub unsafe fn dc_sqlite3_set_config_int64(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: *const libc::c_char,
    value: int64_t,
) -> libc::c_int {
    let value_str = dc_mprintf(
        b"%lld\x00" as *const u8 as *const libc::c_char,
        value as time_t,
    );
    if value_str.is_null() {
        return 0;
    }
    let ret = dc_sqlite3_set_config(context, sql, key, value_str);
    free(value_str as *mut libc::c_void);
    ret
}

pub unsafe fn dc_sqlite3_get_config_int64(
    context: &Context,
    sql: &dc_sqlite3_t,
    key: *const libc::c_char,
    def: int64_t,
) -> int64_t {
    let str = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    if str.is_null() {
        return def;
    }
    let mut ret = 0 as int64_t;
    sscanf(
        str,
        b"%lld\x00" as *const u8 as *const libc::c_char,
        &mut ret as *mut int64_t,
    );
    free(str as *mut libc::c_void);
    ret
}

pub unsafe fn dc_sqlite3_try_execute(
    context: &Context,
    sql: &dc_sqlite3_t,
    querystr: *const libc::c_char,
) -> libc::c_int {
    // same as dc_sqlite3_execute() but does not pass error to ui
    let mut success = 0;
    let sql_state;
    let stmt = dc_sqlite3_prepare(context, sql, querystr);
    if !stmt.is_null() {
        sql_state = sqlite3_step(stmt);
        if sql_state != 101 && sql_state != 100 {
            dc_log_warning(
                context,
                0,
                b"Try-execute for \"%s\" failed: %s\x00" as *const u8 as *const libc::c_char,
                querystr,
                sqlite3_errmsg(sql.cobj),
            );
        } else {
            success = 1
        }
    }
    sqlite3_finalize(stmt);
    success
}

pub unsafe fn dc_sqlite3_get_rowid(
    context: &Context,
    sql: &dc_sqlite3_t,
    table: *const libc::c_char,
    field: *const libc::c_char,
    value: *const libc::c_char,
) -> uint32_t {
    // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    // the ORDER BY ensures, this function always returns the most recent id,
    // eg. if a Message-ID is splitted into different messages.
    let mut id = 0 as uint32_t;
    let q3 = sqlite3_mprintf(
        b"SELECT id FROM %s WHERE %s=%Q ORDER BY id DESC;\x00" as *const u8 as *const libc::c_char,
        table,
        field,
        value,
    );
    let stmt = dc_sqlite3_prepare(context, sql, q3);
    if 100 == sqlite3_step(stmt) {
        id = sqlite3_column_int(stmt, 0) as uint32_t
    }
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    id
}

pub unsafe fn dc_sqlite3_get_rowid2(
    context: &Context,
    sql: &dc_sqlite3_t,
    table: *const libc::c_char,
    field: *const libc::c_char,
    value: uint64_t,
    field2: *const libc::c_char,
    value2: uint32_t,
) -> uint32_t {
    // same as dc_sqlite3_get_rowid() with a key over two columns
    let mut id = 0 as uint32_t;
    // see https://www.sqlite.org/printf.html for sqlite-printf modifiers
    let q3 = sqlite3_mprintf(
        b"SELECT id FROM %s WHERE %s=%lli AND %s=%i ORDER BY id DESC;\x00" as *const u8
            as *const libc::c_char,
        table,
        field,
        value,
        field2,
        value2,
    );
    let stmt = dc_sqlite3_prepare(context, sql, q3);
    if 100 == sqlite3_step(stmt) {
        id = sqlite3_column_int(stmt, 0) as uint32_t
    }
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    id
}

pub unsafe fn dc_housekeeping(context: &Context) {
    let stmt;
    let dir_handle;
    let mut dir_entry;
    let mut files_in_use = HashSet::new();
    let mut path = 0 as *mut libc::c_char;
    let mut unreferenced_count = 0;

    dc_log_info(
        context,
        0,
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
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT value FROM config;\x00" as *const u8 as *const libc::c_char,
    );
    while sqlite3_step(stmt) == 100 {
        maybe_add_file(
            &mut files_in_use,
            sqlite3_column_text(stmt, 0) as *const libc::c_char,
        );
    }
    dc_log_info(
        context,
        0,
        b"%i files in use.\x00" as *const u8 as *const libc::c_char,
        files_in_use.len() as libc::c_int,
    );
    /* go through directory and delete unused files */
    dir_handle = opendir(context.get_blobdir());
    if dir_handle.is_null() {
        dc_log_warning(
            context,
            0,
            b"Housekeeping: Cannot open %s.\x00" as *const u8 as *const libc::c_char,
            context.get_blobdir(),
        );
    } else {
        /* avoid deletion of files that are just created to build a message object */
        let diff = std::time::Duration::from_secs(60 * 60);
        let keep_files_newer_than = std::time::SystemTime::now().checked_sub(diff).unwrap();

        loop {
            dir_entry = readdir(dir_handle);
            if dir_entry.is_null() {
                break;
            }
            /* name without path or `.` or `..` */
            let name: *const libc::c_char = (*dir_entry).d_name.as_mut_ptr();
            let name_len: libc::c_int = strlen(name) as libc::c_int;
            if name_len == 1 && *name.offset(0isize) as libc::c_int == '.' as i32
                || name_len == 2
                    && *name.offset(0isize) as libc::c_int == '.' as i32
                    && *name.offset(1isize) as libc::c_int == '.' as i32
            {
                continue;
            }
            if is_file_in_use(&mut files_in_use, 0 as *const libc::c_char, name)
                || is_file_in_use(
                    &mut files_in_use,
                    b".increation\x00" as *const u8 as *const libc::c_char,
                    name,
                )
                || is_file_in_use(
                    &mut files_in_use,
                    b".waveform\x00" as *const u8 as *const libc::c_char,
                    name,
                )
                || is_file_in_use(
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
                context.get_blobdir(),
                name,
            );

            match std::fs::metadata(std::ffi::CStr::from_ptr(path).to_str().unwrap()) {
                Ok(stats) => {
                    let created =
                        stats.created().is_ok() && stats.created().unwrap() > keep_files_newer_than;
                    let modified = stats.modified().is_ok()
                        && stats.modified().unwrap() > keep_files_newer_than;
                    let accessed = stats.accessed().is_ok()
                        && stats.accessed().unwrap() > keep_files_newer_than;

                    if created || modified || accessed {
                        dc_log_info(
                            context,
                            0,
                            b"Housekeeping: Keeping new unreferenced file #%i: %s\x00" as *const u8
                                as *const libc::c_char,
                            unreferenced_count,
                            name,
                        );
                        continue;
                    }
                }
                Err(_) => {}
            }
            dc_log_info(
                context,
                0,
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

    free(path as *mut libc::c_void);
    dc_log_info(
        context,
        0,
        b"Housekeeping done.\x00" as *const u8 as *const libc::c_char,
    );
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

    let contains = files_in_use.contains(to_str(name_to_check));
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

unsafe fn maybe_add_from_param(
    context: &Context,
    files_in_use: &mut HashSet<String>,
    query: *const libc::c_char,
    param_id: libc::c_int,
) {
    let param = dc_param_new();
    let stmt = dc_sqlite3_prepare(context, &context.sql.clone().read().unwrap(), query);
    while sqlite3_step(stmt) == 100 {
        dc_param_set_packed(param, sqlite3_column_text(stmt, 0) as *const libc::c_char);
        let file = dc_param_get(param, param_id, 0 as *const libc::c_char);
        if !file.is_null() {
            maybe_add_file(files_in_use, file);
            free(file as *mut libc::c_void);
        }
    }
    sqlite3_finalize(stmt);
    dc_param_unref(param);
}

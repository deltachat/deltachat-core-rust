use libc;

use crate::dc_aheader::*;
use crate::dc_chat::*;
use crate::dc_context::dc_context_t;
use crate::dc_hash::*;
use crate::dc_key::*;
use crate::dc_sqlite3::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/* prefer-encrypt states */
/* *
 * @class dc_apeerstate_t
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_apeerstate_t {
    pub context: *mut dc_context_t,
    pub addr: *mut libc::c_char,
    pub last_seen: time_t,
    pub last_seen_autocrypt: time_t,
    pub prefer_encrypt: libc::c_int,
    pub public_key: *mut dc_key_t,
    pub public_key_fingerprint: *mut libc::c_char,
    pub gossip_key: *mut dc_key_t,
    pub gossip_timestamp: time_t,
    pub gossip_key_fingerprint: *mut libc::c_char,
    pub verified_key: *mut dc_key_t,
    pub verified_key_fingerprint: *mut libc::c_char,
    pub to_save: libc::c_int,
    pub degrade_event: libc::c_int,
}

/* the returned pointer is ref'd and must be unref'd after usage */
pub unsafe fn dc_apeerstate_new(mut context: *mut dc_context_t) -> *mut dc_apeerstate_t {
    let mut peerstate: *mut dc_apeerstate_t = 0 as *mut dc_apeerstate_t;
    peerstate = calloc(1, ::std::mem::size_of::<dc_apeerstate_t>()) as *mut dc_apeerstate_t;
    if peerstate.is_null() {
        exit(43i32);
    }
    (*peerstate).context = context;
    return peerstate;
}
pub unsafe fn dc_apeerstate_unref(mut peerstate: *mut dc_apeerstate_t) {
    dc_apeerstate_empty(peerstate);
    free(peerstate as *mut libc::c_void);
}
/* ******************************************************************************
 * dc_apeerstate_t represents the state of an Autocrypt peer - Load/save
 ******************************************************************************/
unsafe fn dc_apeerstate_empty(mut peerstate: *mut dc_apeerstate_t) {
    if peerstate.is_null() {
        return;
    }
    (*peerstate).last_seen = 0i32 as time_t;
    (*peerstate).last_seen_autocrypt = 0i32 as time_t;
    (*peerstate).prefer_encrypt = 0i32;
    (*peerstate).to_save = 0i32;
    free((*peerstate).addr as *mut libc::c_void);
    (*peerstate).addr = 0 as *mut libc::c_char;
    free((*peerstate).public_key_fingerprint as *mut libc::c_void);
    (*peerstate).public_key_fingerprint = 0 as *mut libc::c_char;
    free((*peerstate).gossip_key_fingerprint as *mut libc::c_void);
    (*peerstate).gossip_key_fingerprint = 0 as *mut libc::c_char;
    free((*peerstate).verified_key_fingerprint as *mut libc::c_void);
    (*peerstate).verified_key_fingerprint = 0 as *mut libc::c_char;
    dc_key_unref((*peerstate).public_key);
    (*peerstate).public_key = 0 as *mut dc_key_t;
    (*peerstate).gossip_timestamp = 0i32 as time_t;
    dc_key_unref((*peerstate).gossip_key);
    (*peerstate).gossip_key = 0 as *mut dc_key_t;
    dc_key_unref((*peerstate).verified_key);
    (*peerstate).verified_key = 0 as *mut dc_key_t;
    (*peerstate).degrade_event = 0i32;
}
pub unsafe fn dc_apeerstate_init_from_header(
    mut peerstate: *mut dc_apeerstate_t,
    mut header: *const dc_aheader_t,
    mut message_time: time_t,
) -> libc::c_int {
    if peerstate.is_null() || header.is_null() {
        return 0i32;
    }
    dc_apeerstate_empty(peerstate);
    (*peerstate).addr = dc_strdup((*header).addr);
    (*peerstate).last_seen = message_time;
    (*peerstate).last_seen_autocrypt = message_time;
    (*peerstate).to_save |= 0x2i32;
    (*peerstate).prefer_encrypt = (*header).prefer_encrypt;
    (*peerstate).public_key = dc_key_new();
    dc_key_set_from_key((*peerstate).public_key, (*header).public_key);
    dc_apeerstate_recalc_fingerprint(peerstate);
    return 1i32;
}
pub unsafe fn dc_apeerstate_recalc_fingerprint(mut peerstate: *mut dc_apeerstate_t) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut old_public_fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut old_gossip_fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    if !peerstate.is_null() {
        if !(*peerstate).public_key.is_null() {
            old_public_fingerprint = (*peerstate).public_key_fingerprint;
            (*peerstate).public_key_fingerprint = dc_key_get_fingerprint((*peerstate).public_key);
            if old_public_fingerprint.is_null()
                || *old_public_fingerprint.offset(0isize) as libc::c_int == 0i32
                || (*peerstate).public_key_fingerprint.is_null()
                || *(*peerstate).public_key_fingerprint.offset(0isize) as libc::c_int == 0i32
                || strcasecmp(old_public_fingerprint, (*peerstate).public_key_fingerprint) != 0i32
            {
                (*peerstate).to_save |= 0x2i32;
                if !old_public_fingerprint.is_null()
                    && 0 != *old_public_fingerprint.offset(0isize) as libc::c_int
                {
                    (*peerstate).degrade_event |= 0x2i32
                }
            }
        }
        if !(*peerstate).gossip_key.is_null() {
            old_gossip_fingerprint = (*peerstate).gossip_key_fingerprint;
            (*peerstate).gossip_key_fingerprint = dc_key_get_fingerprint((*peerstate).gossip_key);
            if old_gossip_fingerprint.is_null()
                || *old_gossip_fingerprint.offset(0isize) as libc::c_int == 0i32
                || (*peerstate).gossip_key_fingerprint.is_null()
                || *(*peerstate).gossip_key_fingerprint.offset(0isize) as libc::c_int == 0i32
                || strcasecmp(old_gossip_fingerprint, (*peerstate).gossip_key_fingerprint) != 0i32
            {
                (*peerstate).to_save |= 0x2i32;
                if !old_gossip_fingerprint.is_null()
                    && 0 != *old_gossip_fingerprint.offset(0isize) as libc::c_int
                {
                    (*peerstate).degrade_event |= 0x2i32
                }
            }
        }
        success = 1i32
    }
    free(old_public_fingerprint as *mut libc::c_void);
    free(old_gossip_fingerprint as *mut libc::c_void);
    return success;
}
pub unsafe extern "C" fn dc_apeerstate_init_from_gossip(
    mut peerstate: *mut dc_apeerstate_t,
    mut gossip_header: *const dc_aheader_t,
    mut message_time: time_t,
) -> libc::c_int {
    if peerstate.is_null() || gossip_header.is_null() {
        return 0i32;
    }
    dc_apeerstate_empty(peerstate);
    (*peerstate).addr = dc_strdup((*gossip_header).addr);
    (*peerstate).gossip_timestamp = message_time;
    (*peerstate).to_save |= 0x2i32;
    (*peerstate).gossip_key = dc_key_new();
    dc_key_set_from_key((*peerstate).gossip_key, (*gossip_header).public_key);
    dc_apeerstate_recalc_fingerprint(peerstate);
    return 1i32;
}
pub unsafe fn dc_apeerstate_degrade_encryption(
    mut peerstate: *mut dc_apeerstate_t,
    mut message_time: time_t,
) -> libc::c_int {
    if peerstate.is_null() {
        return 0i32;
    }
    if (*peerstate).prefer_encrypt == 1i32 {
        (*peerstate).degrade_event |= 0x1i32
    }
    (*peerstate).prefer_encrypt = 20i32;
    (*peerstate).last_seen = message_time;
    (*peerstate).to_save |= 0x2i32;
    return 1i32;
}
pub unsafe fn dc_apeerstate_apply_header(
    mut peerstate: *mut dc_apeerstate_t,
    mut header: *const dc_aheader_t,
    mut message_time: time_t,
) {
    if peerstate.is_null()
        || header.is_null()
        || (*peerstate).addr.is_null()
        || (*header).addr.is_null()
        || (*(*header).public_key).binary.is_null()
        || strcasecmp((*peerstate).addr, (*header).addr) != 0i32
    {
        return;
    }
    if message_time > (*peerstate).last_seen_autocrypt {
        (*peerstate).last_seen = message_time;
        (*peerstate).last_seen_autocrypt = message_time;
        (*peerstate).to_save |= 0x1i32;
        if ((*header).prefer_encrypt == 1i32 || (*header).prefer_encrypt == 0i32)
            && (*header).prefer_encrypt != (*peerstate).prefer_encrypt
        {
            if (*peerstate).prefer_encrypt == 1i32 && (*header).prefer_encrypt != 1i32 {
                (*peerstate).degrade_event |= 0x1i32
            }
            (*peerstate).prefer_encrypt = (*header).prefer_encrypt;
            (*peerstate).to_save |= 0x2i32
        }
        if (*peerstate).public_key.is_null() {
            (*peerstate).public_key = dc_key_new()
        }
        if 0 == dc_key_equals((*peerstate).public_key, (*header).public_key) {
            dc_key_set_from_key((*peerstate).public_key, (*header).public_key);
            dc_apeerstate_recalc_fingerprint(peerstate);
            (*peerstate).to_save |= 0x2i32
        }
    };
}
pub unsafe fn dc_apeerstate_apply_gossip(
    mut peerstate: *mut dc_apeerstate_t,
    mut gossip_header: *const dc_aheader_t,
    mut message_time: time_t,
) {
    if peerstate.is_null()
        || gossip_header.is_null()
        || (*peerstate).addr.is_null()
        || (*gossip_header).addr.is_null()
        || (*(*gossip_header).public_key).binary.is_null()
        || strcasecmp((*peerstate).addr, (*gossip_header).addr) != 0i32
    {
        return;
    }
    if message_time > (*peerstate).gossip_timestamp {
        (*peerstate).gossip_timestamp = message_time;
        (*peerstate).to_save |= 0x1i32;
        if (*peerstate).gossip_key.is_null() {
            (*peerstate).gossip_key = dc_key_new()
        }
        if 0 == dc_key_equals((*peerstate).gossip_key, (*gossip_header).public_key) {
            dc_key_set_from_key((*peerstate).gossip_key, (*gossip_header).public_key);
            dc_apeerstate_recalc_fingerprint(peerstate);
            (*peerstate).to_save |= 0x2i32
        }
    };
}
pub unsafe fn dc_apeerstate_render_gossip_header(
    mut peerstate: *const dc_apeerstate_t,
    mut min_verified: libc::c_int,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut autocryptheader: *mut dc_aheader_t = dc_aheader_new();
    if !(peerstate.is_null() || (*peerstate).addr.is_null()) {
        (*autocryptheader).prefer_encrypt = 0i32;
        (*autocryptheader).addr = dc_strdup((*peerstate).addr);
        (*autocryptheader).public_key = dc_key_ref(dc_apeerstate_peek_key(peerstate, min_verified));
        ret = dc_aheader_render(autocryptheader)
    }
    dc_aheader_unref(autocryptheader);
    return ret;
}
pub unsafe fn dc_apeerstate_peek_key(
    mut peerstate: *const dc_apeerstate_t,
    mut min_verified: libc::c_int,
) -> *mut dc_key_t {
    if peerstate.is_null()
        || !(*peerstate).public_key.is_null()
            && ((*(*peerstate).public_key).binary.is_null()
                || (*(*peerstate).public_key).bytes <= 0i32)
        || !(*peerstate).gossip_key.is_null()
            && ((*(*peerstate).gossip_key).binary.is_null()
                || (*(*peerstate).gossip_key).bytes <= 0i32)
        || !(*peerstate).verified_key.is_null()
            && ((*(*peerstate).verified_key).binary.is_null()
                || (*(*peerstate).verified_key).bytes <= 0i32)
    {
        return 0 as *mut dc_key_t;
    }
    if 0 != min_verified {
        return (*peerstate).verified_key;
    }
    if !(*peerstate).public_key.is_null() {
        return (*peerstate).public_key;
    }
    return (*peerstate).gossip_key;
}
pub unsafe fn dc_apeerstate_set_verified(
    mut peerstate: *mut dc_apeerstate_t,
    mut which_key: libc::c_int,
    mut fingerprint: *const libc::c_char,
    mut verified: libc::c_int,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    if !(peerstate.is_null() || which_key != 0i32 && which_key != 1i32 || verified != 2i32) {
        if which_key == 1i32
            && !(*peerstate).public_key_fingerprint.is_null()
            && *(*peerstate).public_key_fingerprint.offset(0isize) as libc::c_int != 0i32
            && *fingerprint.offset(0isize) as libc::c_int != 0i32
            && strcasecmp((*peerstate).public_key_fingerprint, fingerprint) == 0i32
        {
            (*peerstate).to_save |= 0x2i32;
            (*peerstate).verified_key = dc_key_ref((*peerstate).public_key);
            (*peerstate).verified_key_fingerprint = dc_strdup((*peerstate).public_key_fingerprint);
            success = 1i32
        }
        if which_key == 0i32
            && !(*peerstate).gossip_key_fingerprint.is_null()
            && *(*peerstate).gossip_key_fingerprint.offset(0isize) as libc::c_int != 0i32
            && *fingerprint.offset(0isize) as libc::c_int != 0i32
            && strcasecmp((*peerstate).gossip_key_fingerprint, fingerprint) == 0i32
        {
            (*peerstate).to_save |= 0x2i32;
            (*peerstate).verified_key = dc_key_ref((*peerstate).gossip_key);
            (*peerstate).verified_key_fingerprint = dc_strdup((*peerstate).gossip_key_fingerprint);
            success = 1i32
        }
    }
    return success;
}
pub unsafe fn dc_apeerstate_load_by_addr(
    mut peerstate: *mut dc_apeerstate_t,
    mut sql: *mut dc_sqlite3_t,
    mut addr: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(peerstate.is_null() || sql.is_null() || addr.is_null()) {
        dc_apeerstate_empty(peerstate);
        stmt =
            dc_sqlite3_prepare(sql,
                               b"SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates  WHERE addr=? COLLATE NOCASE;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1i32, addr, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            dc_apeerstate_set_from_stmt(peerstate, stmt);
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
unsafe fn dc_apeerstate_set_from_stmt(
    mut peerstate: *mut dc_apeerstate_t,
    mut stmt: *mut sqlite3_stmt,
) {
    (*peerstate).addr = dc_strdup(sqlite3_column_text(stmt, 0i32) as *mut libc::c_char);
    (*peerstate).last_seen = sqlite3_column_int64(stmt, 1i32) as time_t;
    (*peerstate).last_seen_autocrypt = sqlite3_column_int64(stmt, 2i32) as time_t;
    (*peerstate).prefer_encrypt = sqlite3_column_int(stmt, 3i32);
    (*peerstate).gossip_timestamp = sqlite3_column_int(stmt, 5i32) as time_t;
    (*peerstate).public_key_fingerprint =
        dc_strdup(sqlite3_column_text(stmt, 7i32) as *mut libc::c_char);
    (*peerstate).gossip_key_fingerprint =
        dc_strdup(sqlite3_column_text(stmt, 8i32) as *mut libc::c_char);
    (*peerstate).verified_key_fingerprint =
        dc_strdup(sqlite3_column_text(stmt, 10i32) as *mut libc::c_char);
    if sqlite3_column_type(stmt, 4i32) != 5i32 {
        (*peerstate).public_key = dc_key_new();
        dc_key_set_from_stmt((*peerstate).public_key, stmt, 4i32, 0i32);
    }
    if sqlite3_column_type(stmt, 6i32) != 5i32 {
        (*peerstate).gossip_key = dc_key_new();
        dc_key_set_from_stmt((*peerstate).gossip_key, stmt, 6i32, 0i32);
    }
    if sqlite3_column_type(stmt, 9i32) != 5i32 {
        (*peerstate).verified_key = dc_key_new();
        dc_key_set_from_stmt((*peerstate).verified_key, stmt, 9i32, 0i32);
    };
}
pub unsafe fn dc_apeerstate_load_by_fingerprint(
    mut peerstate: *mut dc_apeerstate_t,
    mut sql: *mut dc_sqlite3_t,
    mut fingerprint: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(peerstate.is_null() || sql.is_null() || fingerprint.is_null()) {
        dc_apeerstate_empty(peerstate);
        stmt =
            dc_sqlite3_prepare(sql,
                               b"SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates  WHERE public_key_fingerprint=? COLLATE NOCASE     OR gossip_key_fingerprint=? COLLATE NOCASE  ORDER BY public_key_fingerprint=? DESC;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1i32, fingerprint, -1i32, None);
        sqlite3_bind_text(stmt, 2i32, fingerprint, -1i32, None);
        sqlite3_bind_text(stmt, 3i32, fingerprint, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            dc_apeerstate_set_from_stmt(peerstate, stmt);
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
pub unsafe fn dc_apeerstate_save_to_db(
    mut peerstate: *const dc_apeerstate_t,
    mut sql: *mut dc_sqlite3_t,
    mut create: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if peerstate.is_null() || sql.is_null() || (*peerstate).addr.is_null() {
        return 0i32;
    }
    if 0 != create {
        stmt = dc_sqlite3_prepare(
            sql,
            b"INSERT INTO acpeerstates (addr) VALUES(?);\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, (*peerstate).addr, -1i32, None);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt
    }
    if 0 != (*peerstate).to_save & 0x2i32 || 0 != create {
        stmt =
            dc_sqlite3_prepare(sql,
                               b"UPDATE acpeerstates    SET last_seen=?, last_seen_autocrypt=?, prefer_encrypted=?,        public_key=?, gossip_timestamp=?, gossip_key=?, public_key_fingerprint=?, gossip_key_fingerprint=?, verified_key=?, verified_key_fingerprint=?  WHERE addr=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(stmt, 1i32, (*peerstate).last_seen as sqlite3_int64);
        sqlite3_bind_int64(
            stmt,
            2i32,
            (*peerstate).last_seen_autocrypt as sqlite3_int64,
        );
        sqlite3_bind_int64(stmt, 3i32, (*peerstate).prefer_encrypt as sqlite3_int64);
        sqlite3_bind_blob(
            stmt,
            4i32,
            if !(*peerstate).public_key.is_null() {
                (*(*peerstate).public_key).binary
            } else {
                0 as *mut libc::c_void
            },
            if !(*peerstate).public_key.is_null() {
                (*(*peerstate).public_key).bytes
            } else {
                0i32
            },
            None,
        );
        sqlite3_bind_int64(stmt, 5i32, (*peerstate).gossip_timestamp as sqlite3_int64);
        sqlite3_bind_blob(
            stmt,
            6i32,
            if !(*peerstate).gossip_key.is_null() {
                (*(*peerstate).gossip_key).binary
            } else {
                0 as *mut libc::c_void
            },
            if !(*peerstate).gossip_key.is_null() {
                (*(*peerstate).gossip_key).bytes
            } else {
                0i32
            },
            None,
        );
        sqlite3_bind_text(stmt, 7i32, (*peerstate).public_key_fingerprint, -1i32, None);
        sqlite3_bind_text(stmt, 8i32, (*peerstate).gossip_key_fingerprint, -1i32, None);
        sqlite3_bind_blob(
            stmt,
            9i32,
            if !(*peerstate).verified_key.is_null() {
                (*(*peerstate).verified_key).binary
            } else {
                0 as *mut libc::c_void
            },
            if !(*peerstate).verified_key.is_null() {
                (*(*peerstate).verified_key).bytes
            } else {
                0i32
            },
            None,
        );
        sqlite3_bind_text(
            stmt,
            10i32,
            (*peerstate).verified_key_fingerprint,
            -1i32,
            None,
        );
        sqlite3_bind_text(stmt, 11i32, (*peerstate).addr, -1i32, None);
        if sqlite3_step(stmt) != 101i32 {
            current_block = 7258450500457619456;
        } else {
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            current_block = 11913429853522160501;
        }
    } else if 0 != (*peerstate).to_save & 0x1i32 {
        stmt =
            dc_sqlite3_prepare(sql,
                               b"UPDATE acpeerstates SET last_seen=?, last_seen_autocrypt=?, gossip_timestamp=? WHERE addr=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(stmt, 1i32, (*peerstate).last_seen as sqlite3_int64);
        sqlite3_bind_int64(
            stmt,
            2i32,
            (*peerstate).last_seen_autocrypt as sqlite3_int64,
        );
        sqlite3_bind_int64(stmt, 3i32, (*peerstate).gossip_timestamp as sqlite3_int64);
        sqlite3_bind_text(stmt, 4i32, (*peerstate).addr, -1i32, None);
        if sqlite3_step(stmt) != 101i32 {
            current_block = 7258450500457619456;
        } else {
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            current_block = 11913429853522160501;
        }
    } else {
        current_block = 11913429853522160501;
    }
    match current_block {
        11913429853522160501 => {
            if 0 != (*peerstate).to_save & 0x2i32 || 0 != create {
                dc_reset_gossiped_timestamp((*peerstate).context, 0i32 as uint32_t);
            }
            success = 1i32
        }
        _ => {}
    }
    sqlite3_finalize(stmt);
    return success;
}
pub unsafe fn dc_apeerstate_has_verified_key(
    mut peerstate: *const dc_apeerstate_t,
    mut fingerprints: *const dc_hash_t,
) -> libc::c_int {
    if peerstate.is_null() || fingerprints.is_null() {
        return 0i32;
    }
    if !(*peerstate).verified_key.is_null()
        && !(*peerstate).verified_key_fingerprint.is_null()
        && !dc_hash_find(
            fingerprints,
            (*peerstate).verified_key_fingerprint as *const libc::c_void,
            strlen((*peerstate).verified_key_fingerprint) as libc::c_int,
        )
        .is_null()
    {
        return 1i32;
    }
    return 0i32;
}

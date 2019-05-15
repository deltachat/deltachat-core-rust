use std::ffi::{CStr, CString};

use num_traits::ToPrimitive;

use crate::constants::*;
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
/**
 * @class dc_apeerstate_t
 * Library-internal.
 */
pub struct dc_apeerstate_t<'a> {
    pub context: &'a dc_context_t,
    pub addr: *mut libc::c_char,
    pub last_seen: time_t,
    pub last_seen_autocrypt: time_t,
    pub prefer_encrypt: libc::c_int,
    pub public_key: Option<Key>,
    pub public_key_fingerprint: *mut libc::c_char,
    pub gossip_key: Option<Key>,
    pub gossip_timestamp: time_t,
    pub gossip_key_fingerprint: *mut libc::c_char,
    // TODO: this should be a reference to either the public_key or verified_key
    pub verified_key: Option<Key>,
    pub verified_key_fingerprint: *mut libc::c_char,
    pub to_save: libc::c_int,
    pub degrade_event: libc::c_int,
}

/* the returned pointer is ref'd and must be unref'd after usage */
pub fn dc_apeerstate_new<'a>(context: &'a dc_context_t) -> dc_apeerstate_t<'a> {
    dc_apeerstate_t {
        context,
        addr: std::ptr::null_mut(),
        last_seen: 0,
        last_seen_autocrypt: 0,
        prefer_encrypt: 0,
        public_key: None,
        public_key_fingerprint: std::ptr::null_mut(),
        gossip_key: None,
        gossip_key_fingerprint: std::ptr::null_mut(),
        gossip_timestamp: 0,
        verified_key: None,
        verified_key_fingerprint: std::ptr::null_mut(),
        to_save: 0,
        degrade_event: 0,
    }
}

pub unsafe fn dc_apeerstate_unref(peerstate: &mut dc_apeerstate_t) {
    dc_apeerstate_empty(peerstate);
}

/*******************************************************************************
 * dc_apeerstate_t represents the state of an Autocrypt peer - Load/save
 ******************************************************************************/
unsafe fn dc_apeerstate_empty(peerstate: &mut dc_apeerstate_t) {
    peerstate.last_seen = 0i32 as time_t;
    peerstate.last_seen_autocrypt = 0i32 as time_t;
    peerstate.prefer_encrypt = 0i32;
    peerstate.to_save = 0i32;
    free(peerstate.addr as *mut libc::c_void);
    peerstate.addr = 0 as *mut libc::c_char;
    free(peerstate.public_key_fingerprint as *mut libc::c_void);
    peerstate.public_key_fingerprint = 0 as *mut libc::c_char;
    free(peerstate.gossip_key_fingerprint as *mut libc::c_void);
    peerstate.gossip_key_fingerprint = 0 as *mut libc::c_char;
    free(peerstate.verified_key_fingerprint as *mut libc::c_void);
    peerstate.verified_key_fingerprint = 0 as *mut libc::c_char;

    peerstate.public_key = None;
    peerstate.gossip_timestamp = 0i32 as time_t;
    peerstate.gossip_key = None;
    peerstate.verified_key = None;
    peerstate.degrade_event = 0i32;
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_init_from_header(
    peerstate: &mut dc_apeerstate_t,
    header: &Aheader,
    message_time: time_t,
) -> libc::c_int {
    dc_apeerstate_empty(peerstate);
    peerstate.addr = dc_strdup(CString::new(header.addr.clone()).unwrap().as_ptr());
    peerstate.last_seen = message_time;
    peerstate.last_seen_autocrypt = message_time;
    peerstate.to_save |= 0x2i32;
    peerstate.prefer_encrypt = header.prefer_encrypt.to_i32().unwrap();
    peerstate.public_key = Some(header.public_key.clone());
    dc_apeerstate_recalc_fingerprint(peerstate);

    1
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_recalc_fingerprint(peerstate: &mut dc_apeerstate_t) -> libc::c_int {
    let mut old_public_fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut old_gossip_fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;

    if let Some(ref public_key) = peerstate.public_key {
        old_public_fingerprint = peerstate.public_key_fingerprint;
        peerstate.public_key_fingerprint = public_key.fingerprint_c();
        if old_public_fingerprint.is_null()
            || *old_public_fingerprint.offset(0isize) as libc::c_int == 0i32
            || peerstate.public_key_fingerprint.is_null()
            || *peerstate.public_key_fingerprint.offset(0isize) as libc::c_int == 0i32
            || strcasecmp(old_public_fingerprint, peerstate.public_key_fingerprint) != 0i32
        {
            peerstate.to_save |= 0x2i32;
            if !old_public_fingerprint.is_null()
                && 0 != *old_public_fingerprint.offset(0isize) as libc::c_int
            {
                peerstate.degrade_event |= 0x2i32;
            }
        }
    }

    if let Some(ref gossip_key) = peerstate.gossip_key {
        old_gossip_fingerprint = peerstate.gossip_key_fingerprint;
        peerstate.gossip_key_fingerprint = gossip_key.fingerprint_c();

        if old_gossip_fingerprint.is_null()
            || *old_gossip_fingerprint.offset(0isize) as libc::c_int == 0i32
            || peerstate.gossip_key_fingerprint.is_null()
            || *peerstate.gossip_key_fingerprint.offset(0isize) as libc::c_int == 0i32
            || strcasecmp(old_gossip_fingerprint, peerstate.gossip_key_fingerprint) != 0i32
        {
            peerstate.to_save |= 0x2i32;
            if !old_gossip_fingerprint.is_null()
                && 0 != *old_gossip_fingerprint.offset(0isize) as libc::c_int
            {
                peerstate.degrade_event |= 0x2i32
            }
        }
    }

    free(old_public_fingerprint as *mut libc::c_void);
    free(old_gossip_fingerprint as *mut libc::c_void);

    1
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_init_from_gossip(
    peerstate: &mut dc_apeerstate_t,
    gossip_header: &Aheader,
    message_time: time_t,
) -> libc::c_int {
    dc_apeerstate_empty(peerstate);
    peerstate.addr = dc_strdup(CString::new(gossip_header.addr.clone()).unwrap().as_ptr());
    peerstate.gossip_timestamp = message_time;
    peerstate.to_save |= 0x2i32;
    peerstate.gossip_key = Some(gossip_header.public_key.clone());
    dc_apeerstate_recalc_fingerprint(peerstate);

    1
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_degrade_encryption(
    peerstate: &mut dc_apeerstate_t,
    message_time: time_t,
) -> libc::c_int {
    if peerstate.prefer_encrypt == 1i32 {
        peerstate.degrade_event |= 0x1i32
    }
    peerstate.prefer_encrypt = 20i32;
    peerstate.last_seen = message_time;
    peerstate.to_save |= 0x2i32;

    1
}

pub unsafe fn dc_apeerstate_apply_header(
    peerstate: &mut dc_apeerstate_t,
    header: &Aheader,
    message_time: time_t,
) {
    if peerstate.addr.is_null()
        || CStr::from_ptr(peerstate.addr)
            .to_str()
            .unwrap()
            .to_lowercase()
            != header.addr.to_lowercase()
    {
        return;
    }

    if message_time > peerstate.last_seen_autocrypt {
        peerstate.last_seen = message_time;
        peerstate.last_seen_autocrypt = message_time;
        peerstate.to_save |= 0x1i32;
        if (header.prefer_encrypt == EncryptPreference::Mutual
            || header.prefer_encrypt == EncryptPreference::NoPreference)
            && header.prefer_encrypt.to_i32().unwrap() != peerstate.prefer_encrypt
        {
            if peerstate.prefer_encrypt == 1i32
                && header.prefer_encrypt != EncryptPreference::Mutual
            {
                peerstate.degrade_event |= 0x1i32
            }
            peerstate.prefer_encrypt = header.prefer_encrypt.to_i32().unwrap();
            peerstate.to_save |= 0x2i32
        }

        if peerstate.public_key.as_ref() != Some(&header.public_key) {
            peerstate.public_key = Some(header.public_key.clone());
            dc_apeerstate_recalc_fingerprint(peerstate);
            peerstate.to_save |= 0x2i32;
        }
    }
}

pub unsafe fn dc_apeerstate_apply_gossip(
    peerstate: &mut dc_apeerstate_t,
    gossip_header: &Aheader,
    message_time: time_t,
) {
    if peerstate.addr.is_null()
        || CStr::from_ptr(peerstate.addr)
            .to_str()
            .unwrap()
            .to_lowercase()
            != gossip_header.addr.to_lowercase()
    {
        return;
    }

    if message_time > peerstate.gossip_timestamp {
        peerstate.gossip_timestamp = message_time;
        peerstate.to_save |= 0x1i32;
        if peerstate.gossip_key.as_ref() == Some(&gossip_header.public_key) {
            peerstate.gossip_key = Some(gossip_header.public_key.clone());
            dc_apeerstate_recalc_fingerprint(peerstate);
            peerstate.to_save |= 0x2i32
        }
    };
}

pub unsafe fn dc_apeerstate_render_gossip_header(
    peerstate: &dc_apeerstate_t,
    min_verified: libc::c_int,
) -> *mut libc::c_char {
    if peerstate.addr.is_null() {
        return std::ptr::null_mut();
    }

    let addr = CStr::from_ptr(peerstate.addr).to_str().unwrap().into();
    if let Some(key) = dc_apeerstate_peek_key(peerstate, min_verified) {
        // TODO: avoid cloning
        let header = Aheader::new(addr, key.clone(), EncryptPreference::NoPreference);
        let rendered = header.to_string();
        let rendered_c = CString::new(rendered).unwrap();
        return libc::strdup(rendered_c.as_ptr());
    }

    std::ptr::null_mut()
}

pub unsafe fn dc_apeerstate_peek_key<'a>(
    peerstate: &'a dc_apeerstate_t<'a>,
    min_verified: libc::c_int,
) -> Option<&'a Key> {
    if peerstate.public_key.is_none()
        && peerstate.gossip_key.is_none()
        && peerstate.verified_key.is_none()
    {
        return None;
    }

    if 0 != min_verified {
        return peerstate.verified_key.as_ref();
    }
    if peerstate.public_key.is_some() {
        return peerstate.public_key.as_ref();
    }

    peerstate.gossip_key.as_ref()
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_set_verified(
    peerstate: &mut dc_apeerstate_t,
    which_key: libc::c_int,
    fingerprint: *const libc::c_char,
    verified: libc::c_int,
) -> libc::c_int {
    let mut success: libc::c_int = 0;
    if !(which_key != 0 && which_key != 1 || verified != 2) {
        if which_key == 1
            && !peerstate.public_key_fingerprint.is_null()
            && *peerstate.public_key_fingerprint.offset(0isize) as libc::c_int != 0
            && *fingerprint.offset(0isize) as libc::c_int != 0
            && strcasecmp(peerstate.public_key_fingerprint, fingerprint) == 0
        {
            peerstate.to_save |= 0x2;
            peerstate.verified_key = peerstate.public_key.clone();
            peerstate.verified_key_fingerprint = dc_strdup(peerstate.public_key_fingerprint);
            success = 1
        }
        if which_key == 0
            && !peerstate.gossip_key_fingerprint.is_null()
            && *peerstate.gossip_key_fingerprint.offset(0isize) as libc::c_int != 0
            && *fingerprint.offset(0isize) as libc::c_int != 0
            && strcasecmp(peerstate.gossip_key_fingerprint, fingerprint) == 0
        {
            peerstate.to_save |= 0x2;
            peerstate.verified_key = peerstate.gossip_key.clone();
            peerstate.verified_key_fingerprint = dc_strdup(peerstate.gossip_key_fingerprint);
            success = 1
        }
    }

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_load_by_addr(
    peerstate: &mut dc_apeerstate_t,
    sql: &dc_sqlite3_t,
    addr: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !addr.is_null() {
        dc_apeerstate_empty(peerstate);
        stmt =
            dc_sqlite3_prepare(
                peerstate.context,
                sql,
                               b"SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates  WHERE addr=? COLLATE NOCASE;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1, addr, -1, None);
        if !(sqlite3_step(stmt) != 100) {
            dc_apeerstate_set_from_stmt(peerstate, stmt);
            success = 1
        }
    }
    sqlite3_finalize(stmt);
    success
}

unsafe fn dc_apeerstate_set_from_stmt(
    mut peerstate: &mut dc_apeerstate_t,
    stmt: *mut sqlite3_stmt,
) {
    peerstate.addr = dc_strdup(sqlite3_column_text(stmt, 0) as *mut libc::c_char);
    peerstate.last_seen = sqlite3_column_int64(stmt, 1) as time_t;
    peerstate.last_seen_autocrypt = sqlite3_column_int64(stmt, 2) as time_t;
    peerstate.prefer_encrypt = sqlite3_column_int(stmt, 3);
    peerstate.gossip_timestamp = sqlite3_column_int(stmt, 5) as time_t;
    peerstate.public_key_fingerprint = dc_strdup(sqlite3_column_text(stmt, 7) as *mut libc::c_char);
    peerstate.gossip_key_fingerprint = dc_strdup(sqlite3_column_text(stmt, 8) as *mut libc::c_char);
    peerstate.verified_key_fingerprint =
        dc_strdup(sqlite3_column_text(stmt, 10) as *mut libc::c_char);

    if sqlite3_column_type(stmt, 4) != 5 {
        peerstate.public_key = Key::from_stmt(stmt, 4, KeyType::Public);
    }
    if sqlite3_column_type(stmt, 6) != 5 {
        peerstate.gossip_key = Key::from_stmt(stmt, 6, KeyType::Public);
    }
    if sqlite3_column_type(stmt, 9) != 5 {
        peerstate.verified_key = Key::from_stmt(stmt, 9, KeyType::Public);
    }
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_load_by_fingerprint(
    peerstate: &mut dc_apeerstate_t,
    sql: &dc_sqlite3_t,
    fingerprint: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !fingerprint.is_null() {
        dc_apeerstate_empty(peerstate);
        stmt =
            dc_sqlite3_prepare(
                peerstate.context,
                sql,
                b"SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates  WHERE public_key_fingerprint=? COLLATE NOCASE     OR gossip_key_fingerprint=? COLLATE NOCASE  ORDER BY public_key_fingerprint=? DESC;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1, fingerprint, -1, None);
        sqlite3_bind_text(stmt, 2, fingerprint, -1, None);
        sqlite3_bind_text(stmt, 3, fingerprint, -1, None);
        if sqlite3_step(stmt) == 100 {
            dc_apeerstate_set_from_stmt(peerstate, stmt);
            success = 1
        }
    }
    sqlite3_finalize(stmt);
    success
}

// TODO should return bool /rtn
pub unsafe fn dc_apeerstate_save_to_db(
    peerstate: &dc_apeerstate_t,
    sql: &dc_sqlite3_t,
    create: libc::c_int,
) -> libc::c_int {
    let current_block: u64;
    let mut success: libc::c_int = 0;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if peerstate.addr.is_null() {
        return 0;
    }
    if 0 != create {
        stmt = dc_sqlite3_prepare(
            peerstate.context,
            sql,
            b"INSERT INTO acpeerstates (addr) VALUES(?);\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1, peerstate.addr, -1, None);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt
    }
    if 0 != peerstate.to_save & 0x2 || 0 != create {
        stmt =
            dc_sqlite3_prepare(
                peerstate.context,sql,
                               b"UPDATE acpeerstates    SET last_seen=?, last_seen_autocrypt=?, prefer_encrypted=?,        public_key=?, gossip_timestamp=?, gossip_key=?, public_key_fingerprint=?, gossip_key_fingerprint=?, verified_key=?, verified_key_fingerprint=?  WHERE addr=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(stmt, 1, peerstate.last_seen as sqlite3_int64);
        sqlite3_bind_int64(stmt, 2, peerstate.last_seen_autocrypt as sqlite3_int64);
        sqlite3_bind_int64(stmt, 3, peerstate.prefer_encrypt as sqlite3_int64);

        if let Some(ref key) = peerstate.public_key {
            let b = key.to_bytes();
            sqlite3_bind_blob(
                stmt,
                4,
                b.as_ptr() as *const _,
                b.len() as libc::c_int,
                None,
            );
        } else {
            sqlite3_bind_blob(stmt, 4, std::ptr::null(), 0, None);
        }

        sqlite3_bind_int64(stmt, 5, peerstate.gossip_timestamp as sqlite3_int64);
        if let Some(ref key) = peerstate.gossip_key {
            let b = key.to_bytes();
            sqlite3_bind_blob(
                stmt,
                6,
                b.as_ptr() as *const _,
                b.len() as libc::c_int,
                None,
            );
        } else {
            sqlite3_bind_blob(stmt, 6, std::ptr::null(), 0, None);
        }

        sqlite3_bind_text(stmt, 7, peerstate.public_key_fingerprint, -1, None);
        sqlite3_bind_text(stmt, 8, peerstate.gossip_key_fingerprint, -1, None);
        if let Some(ref key) = peerstate.verified_key {
            let b = key.to_bytes();
            sqlite3_bind_blob(
                stmt,
                9,
                b.as_ptr() as *const _,
                b.len() as libc::c_int,
                None,
            );
        } else {
            sqlite3_bind_blob(stmt, 9, std::ptr::null(), 0, None);
        }

        sqlite3_bind_text(stmt, 10, peerstate.verified_key_fingerprint, -1, None);
        sqlite3_bind_text(stmt, 11, peerstate.addr, -1, None);
        if sqlite3_step(stmt) != 101 {
            current_block = 7258450500457619456;
        } else {
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            current_block = 11913429853522160501;
        }
    } else if 0 != peerstate.to_save & 0x1 {
        stmt =
            dc_sqlite3_prepare(
                peerstate.context,sql,
                               b"UPDATE acpeerstates SET last_seen=?, last_seen_autocrypt=?, gossip_timestamp=? WHERE addr=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(stmt, 1, peerstate.last_seen as sqlite3_int64);
        sqlite3_bind_int64(stmt, 2, peerstate.last_seen_autocrypt as sqlite3_int64);
        sqlite3_bind_int64(stmt, 3, peerstate.gossip_timestamp as sqlite3_int64);
        sqlite3_bind_text(stmt, 4, peerstate.addr, -1, None);
        if sqlite3_step(stmt) != 101 {
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
            if 0 != peerstate.to_save & 0x2 || 0 != create {
                dc_reset_gossiped_timestamp(peerstate.context, 0 as uint32_t);
            }
            success = 1
        }
        _ => {}
    }
    sqlite3_finalize(stmt);

    success
}

pub unsafe fn dc_apeerstate_has_verified_key(
    peerstate: &dc_apeerstate_t,
    fingerprints: *const dc_hash_t,
) -> bool {
    if fingerprints.is_null() {
        return false;
    }

    if peerstate.verified_key.is_some()
        && !peerstate.verified_key_fingerprint.is_null()
        && !dc_hash_find(
            fingerprints,
            peerstate.verified_key_fingerprint as *const libc::c_void,
            strlen(peerstate.verified_key_fingerprint) as libc::c_int,
        )
        .is_null()
    {
        return true;
    }

    false
}

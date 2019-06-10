use crate::aheader::EncryptPreference;
use crate::constants::Event;
use crate::context::Context;
use crate::context::*;
use crate::dc_array::*;
use crate::dc_e2ee::*;
use crate::dc_loginparam::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_tools::*;
use crate::key::*;
use crate::peerstate::*;
use crate::types::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_contact_t<'a> {
    pub magic: uint32_t,
    pub context: &'a Context,
    pub id: uint32_t,
    pub name: *mut libc::c_char,
    pub authname: *mut libc::c_char,
    pub addr: *mut libc::c_char,
    pub blocked: libc::c_int,
    pub origin: libc::c_int,
}

pub fn dc_marknoticed_contact(context: &Context, contact_id: u32) {
    if dc_sqlite3_execute(
        context,
        &context.sql.clone().read().unwrap(),
        "UPDATE msgs SET state=13 WHERE from_id=? AND state=10;",
        params![contact_id as i32],
    ) {
        unsafe { ((*context).cb)(context, Event::MSGS_CHANGED, 0, 0) };
    }
}

/// Returns false if addr is an invalid address, otherwise true.
pub unsafe fn dc_may_be_valid_addr(addr: *const libc::c_char) -> bool {
    if addr.is_null() {
        return false;
    }
    let at: *const libc::c_char = strchr(addr, '@' as i32);
    if at.is_null() || at.wrapping_offset_from(addr) < 1 {
        return false;
    }
    let dot: *const libc::c_char = strchr(at, '.' as i32);
    if dot.is_null()
        || dot.wrapping_offset_from(at) < 2
        || *dot.offset(1isize) as libc::c_int == 0i32
        || *dot.offset(2isize) as libc::c_int == 0i32
    {
        return false;
    }

    true
}

pub unsafe fn dc_lookup_contact_id_by_addr(
    context: &Context,
    addr: *const libc::c_char,
) -> uint32_t {
    if addr.is_null() || *addr.offset(0) as libc::c_int == 0 {
        return 0;
    }

    let addr_normalized_c = dc_addr_normalize(addr);
    let addr_normalized = as_str(addr_normalized_c);
    let addr_self = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "configured_addr",
        None,
    )
    .unwrap_or_default();

    let contact_id = if addr_normalized == addr_self {
        1
    } else {
        dc_sqlite3_query_row(
            context,
            &context.sql.clone().read().unwrap(),
            "SELECT id FROM contacts WHERE addr=?1 COLLATE NOCASE AND id>?2 AND origin>=?3 AND blocked=0;",
            params![addr_normalized, 9, 0x100],
            0
        ).unwrap_or_default()
    };
    free(addr_normalized_c as *mut libc::c_void);

    contact_id
}

pub unsafe fn dc_addr_normalize(addr: *const libc::c_char) -> *mut libc::c_char {
    let mut addr_normalized: *mut libc::c_char = dc_strdup(addr);
    dc_trim(addr_normalized);
    if strncmp(
        addr_normalized,
        b"mailto:\x00" as *const u8 as *const libc::c_char,
        7,
    ) == 0i32
    {
        let old: *mut libc::c_char = addr_normalized;
        addr_normalized = dc_strdup(&mut *old.offset(7isize));
        free(old as *mut libc::c_void);
        dc_trim(addr_normalized);
    }

    addr_normalized
}

pub fn dc_addr_normalize_safe(addr: &str) -> &str {
    let norm = addr.trim();

    if norm.starts_with("mailto:") {
        return &norm[7..];
    }

    norm
}

pub unsafe fn dc_create_contact(
    context: &Context,
    name: *const libc::c_char,
    addr: *const libc::c_char,
) -> uint32_t {
    let mut contact_id: uint32_t = 0i32 as uint32_t;
    let mut sth_modified: libc::c_int = 0i32;
    let blocked: bool;
    if !(addr.is_null() || *addr.offset(0isize) as libc::c_int == 0i32) {
        contact_id = dc_add_or_lookup_contact(context, name, addr, 0x4000000i32, &mut sth_modified);
        blocked = dc_is_contact_blocked(context, contact_id);
        ((*context).cb)(
            context,
            Event::CONTACTS_CHANGED,
            (if sth_modified == 2i32 {
                contact_id
            } else {
                0i32 as libc::c_uint
            }) as uintptr_t,
            0i32 as uintptr_t,
        );
        if blocked {
            dc_block_contact(context, contact_id, 0i32);
        }
    }

    contact_id
}

pub unsafe fn dc_block_contact(context: &Context, contact_id: uint32_t, new_blocking: libc::c_int) {
    if contact_id <= 9 {
        return;
    }

    let contact = dc_contact_new(context);

    if dc_contact_load_from_db(contact, &context.sql.clone().read().unwrap(), contact_id)
        && (*contact).blocked != new_blocking
    {
        if dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            "UPDATE contacts SET blocked=? WHERE id=?;",
            params![new_blocking, contact_id as i32],
        ) {
            // also (un)block all chats with _only_ this contact - we do not delete them to allow a
            // non-destructive blocking->unblocking.
            // (Maybe, beside normal chats (type=100) we should also block group chats with only this user.
            // However, I'm not sure about this point; it may be confusing if the user wants to add other people;
            // this would result in recreating the same group...)
            if dc_sqlite3_execute(
                context,
                &context.sql.clone().read().unwrap(),
                "UPDATE chats SET blocked=? WHERE type=? AND id IN (SELECT chat_id FROM chats_contacts WHERE contact_id=?);",
                params![new_blocking, 100, contact_id as i32],
            ) {
                dc_marknoticed_contact(context, contact_id);
                ((*context).cb)(
                    context,
                    Event::CONTACTS_CHANGED,
                    0,
                    0,
                );
            }
        }
    }

    dc_contact_unref(contact);
}

/**
 * @class dc_contact_t
 *
 * An object representing a single contact in memory.
 * The contact object is not updated.
 * If you want an update, you have to recreate the object.
 *
 * The library makes sure
 * only to use names _authorized_ by the contact in `To:` or `Cc:`.
 * _Given-names _as "Daddy" or "Honey" are not used there.
 * For this purpose, internally, two names are tracked -
 * authorized-name and given-name.
 * By default, these names are equal,
 * but functions working with contact names
 * (eg. dc_contact_get_name(), dc_contact_get_display_name(),
 * dc_contact_get_name_n_addr(), dc_contact_get_first_name(),
 * dc_create_contact() or dc_add_address_book())
 * only affect the given-name.
 */
pub unsafe fn dc_contact_new<'a>(context: &'a Context) -> *mut dc_contact_t<'a> {
    let mut contact: *mut dc_contact_t;
    contact = calloc(1, ::std::mem::size_of::<dc_contact_t>()) as *mut dc_contact_t;
    assert!(!contact.is_null());

    (*contact).magic = 0xc047ac7i32 as uint32_t;
    (*contact).context = context;

    contact
}

pub unsafe fn dc_contact_unref(contact: *mut dc_contact_t) {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return;
    }
    dc_contact_empty(contact);
    (*contact).magic = 0i32 as uint32_t;
    free(contact as *mut libc::c_void);
}

pub unsafe fn dc_contact_empty(mut contact: *mut dc_contact_t) {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return;
    }
    (*contact).id = 0i32 as uint32_t;
    free((*contact).name as *mut libc::c_void);
    (*contact).name = 0 as *mut libc::c_char;
    free((*contact).authname as *mut libc::c_void);
    (*contact).authname = 0 as *mut libc::c_char;
    free((*contact).addr as *mut libc::c_void);
    (*contact).addr = 0 as *mut libc::c_char;
    (*contact).origin = 0i32;
    (*contact).blocked = 0i32;
}

/* From: of incoming messages of unknown sender */
/* Cc: of incoming messages of unknown sender */
/* To: of incoming messages of unknown sender */
/* address scanned but not verified */
/* Reply-To: of incoming message of known sender */
/* Cc: of incoming message of known sender */
/* additional To:'s of incoming message of known sender */
/* a chat was manually created for this user, but no message yet sent */
/* message sent by us */
/* message sent by us */
/* message sent by us */
/* internal use */
/* address is in our address book */
/* set on Alice's side for contacts like Bob that have scanned the QR code offered by her. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling dc_contact_is_verified() ! */
/* set on Bob's side for contacts scanned and verified from a QR code. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling dc_contact_is_verified() ! */
/* contact added mannually by dc_create_contact(), this should be the largets origin as otherwise the user cannot modify the names */
/* contacts with at least this origin value are shown in the contact list */
/* contacts with at least this origin value are verified and known not to be spam */
/* contacts with at least this origin value start a new "normal" chat, defaults to off */
pub unsafe fn dc_contact_load_from_db(
    contact: *mut dc_contact_t,
    sql: &dc_sqlite3_t,
    contact_id: u32,
) -> bool {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return false;
    }

    dc_contact_empty(contact);

    if contact_id == 1 as libc::c_uint {
        (*contact).id = contact_id;
        (*contact).name = dc_stock_str((*contact).context, 2);
        (*contact).addr = dc_strdup(
            to_cstring(
                dc_sqlite3_get_config((*contact).context, sql, "configured_addr", Some(""))
                    .unwrap_or_default(),
            )
            .as_ptr(),
        );
        true
    } else {
        dc_sqlite3_prepare(
            (*contact).context,sql,
            "SELECT c.name, c.addr, c.origin, c.blocked, c.authname  FROM contacts c  WHERE c.id=?;",
        ).and_then(|mut stmt| {
            stmt.query_row(
                params![contact_id as i32],
                |row| {
                    (*contact).id = contact_id;
                    (*contact).name = dc_strdup(to_cstring(row.get::<_, String>(0)?).as_ptr());
                    (*contact).addr = dc_strdup(to_cstring(row.get::<_, String>(1)?).as_ptr());
                    (*contact).origin = row.get(2)?;
                    (*contact).blocked = row.get(3)?;
                    (*contact).authname = dc_strdup(to_cstring(row.get::<_, String>(4)?).as_ptr());
                    Ok(())
                }
            ).ok()
        }).is_some()
    }
}

pub unsafe fn dc_is_contact_blocked(context: &Context, contact_id: uint32_t) -> bool {
    let mut is_blocked = false;
    let contact: *mut dc_contact_t = dc_contact_new(context);
    if dc_contact_load_from_db(contact, &context.sql.clone().read().unwrap(), contact_id) {
        if 0 != (*contact).blocked {
            is_blocked = true
        }
    }
    dc_contact_unref(contact);

    is_blocked
}

/*can be NULL*/
pub fn dc_add_or_lookup_contact(
    context: &Context,
    name: *const libc::c_char,
    addr__: *const libc::c_char,
    origin: libc::c_int,
    mut sth_modified: *mut libc::c_int,
) -> uint32_t {
    let mut dummy = 0;

    if sth_modified.is_null() {
        sth_modified = &mut dummy;
    }
    *sth_modified = 0;

    if addr__.is_null() || origin <= 0 {
        return 0;
    }

    let addr_c = dc_addr_normalize(addr__);
    let addr = as_str(addr_c);
    let addr_self = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "configured_addr",
        Some(""),
    )
    .unwrap_or_default();

    if addr == addr_self {
        return 1;
    }

    if !dc_may_be_valid_addr(addr_c) {
        warn!(
            context,
            0,
            "Bad address \"{}\" for contact \"{}\".",
            addr,
            if !name.is_null() {
                as_str(name)
            } else {
                "<unset>"
            },
        );
        return 0;
    }

    let mut update_addr = false;
    let mut update_name = false;
    let mut update_authname = false;
    let mut row_id = 0;

    if let Some((id, row_name, row_addr, row_origin, row_authname)) = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT id, name, addr, origin, authname FROM contacts WHERE addr=? COLLATE NOCASE;",
    )
    .and_then(|mut stmt| {
        stmt.query_row(params![addr], |row| {
            let row_id = row.get(0)?;
            let row_name: String = row.get(1)?;
            let row_addr: String = row.get(2)?;
            let row_origin = row.get(3)?;
            let row_authname: String = row.get(4)?;

            if !name.is_null() && 0 != *name.offset(0) as libc::c_int {
                if !row_name.is_empty() {
                    if origin >= row_origin && as_str(name) != row_name {
                        update_name = true;
                    }
                }
            } else {
                update_name = true;
            }
            if origin == 0x10 && as_str(name) != row_authname {
                update_authname = true;
            }
            Ok((row_id, row_name, row_addr, row_origin, row_authname))
        })
        .ok()
    }) {
        row_id = id;
        if origin >= row_origin && addr != row_addr {
            update_addr = true;
        }
        if update_name || update_authname || update_addr || origin > row_origin {
            dc_sqlite3_execute(
                context,
                &context.sql.clone().read().unwrap(),
                "UPDATE contacts SET name=?, addr=?, origin=?, authname=? WHERE id=?;",
                params![
                    if update_name { as_str(name) } else { &row_name },
                    if update_addr { addr } else { &row_addr },
                    if origin > row_origin {
                        origin
                    } else {
                        row_origin
                    },
                    if update_authname {
                        as_str(name)
                    } else {
                        &row_authname
                    },
                    row_id
                ],
            );

            if update_name {
                dc_sqlite3_execute(
                    context,
                    &context.sql.clone().read().unwrap(),
                    "UPDATE chats SET name=? WHERE type=? AND id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?);",
                    params![as_str(name), 100, row_id]
                );
            }
            *sth_modified = 1;
        }
    } else {
        if dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            "INSERT INTO contacts (name, addr, origin) VALUES(?, ?, ?);",
            params![
                if !name.is_null() { as_str(name) } else { "" },
                addr,
                origin,
            ],
        ) {
            row_id = dc_sqlite3_get_rowid(
                context,
                &context.sql.clone().read().unwrap(),
                "contacts",
                "addr",
                addr,
            );
            *sth_modified = 2;
        } else {
            error!(context, 0, "Cannot add contact.");
        }
    }

    free(addr_c as *mut libc::c_void);

    row_id
}

pub unsafe fn dc_add_address_book(context: &Context, adr_book: *const libc::c_char) -> libc::c_int {
    let mut lines: *mut carray = 0 as *mut carray;
    let mut i: size_t;
    let iCnt: size_t;
    let mut sth_modified: libc::c_int = 0i32;
    let mut modify_cnt: libc::c_int = 0i32;
    if !(adr_book.is_null()) {
        lines = dc_split_into_lines(adr_book);
        if !lines.is_null() {
            iCnt = carray_count(lines) as size_t;
            i = 0i32 as size_t;
            while i.wrapping_add(1) < iCnt {
                let name: *mut libc::c_char =
                    carray_get(lines, i as libc::c_uint) as *mut libc::c_char;
                let addr: *mut libc::c_char =
                    carray_get(lines, i.wrapping_add(1) as libc::c_uint) as *mut libc::c_char;
                dc_normalize_name(name);
                dc_add_or_lookup_contact(context, name, addr, 0x80000i32, &mut sth_modified);
                if 0 != sth_modified {
                    modify_cnt += 1
                }
                i = (i as libc::c_ulong).wrapping_add(2i32 as libc::c_ulong) as size_t as size_t
            }
            if 0 != modify_cnt {
                ((*context).cb)(
                    context,
                    Event::CONTACTS_CHANGED,
                    0i32 as uintptr_t,
                    0i32 as uintptr_t,
                );
            }
        }
    }
    dc_free_splitted_lines(lines);

    modify_cnt
}

// Working with names
pub unsafe fn dc_normalize_name(full_name: *mut libc::c_char) {
    if full_name.is_null() {
        return;
    }
    dc_trim(full_name);
    let len: libc::c_int = strlen(full_name) as libc::c_int;
    if len > 0i32 {
        let firstchar: libc::c_char = *full_name.offset(0isize);
        let lastchar: libc::c_char = *full_name.offset((len - 1i32) as isize);
        if firstchar as libc::c_int == '\'' as i32 && lastchar as libc::c_int == '\'' as i32
            || firstchar as libc::c_int == '\"' as i32 && lastchar as libc::c_int == '\"' as i32
            || firstchar as libc::c_int == '<' as i32 && lastchar as libc::c_int == '>' as i32
        {
            *full_name.offset(0isize) = ' ' as i32 as libc::c_char;
            *full_name.offset((len - 1i32) as isize) = ' ' as i32 as libc::c_char
        }
    }
    let p1: *mut libc::c_char = strchr(full_name, ',' as i32);
    if !p1.is_null() {
        *p1 = 0i32 as libc::c_char;
        let last_name: *mut libc::c_char = dc_strdup(full_name);
        let first_name: *mut libc::c_char = dc_strdup(p1.offset(1isize));
        dc_trim(last_name);
        dc_trim(first_name);
        strcpy(full_name, first_name);
        strcat(full_name, b" \x00" as *const u8 as *const libc::c_char);
        strcat(full_name, last_name);
        free(last_name as *mut libc::c_void);
        free(first_name as *mut libc::c_void);
    } else {
        dc_trim(full_name);
    };
}

pub fn dc_get_contacts(
    context: &Context,
    listflags: u32,
    query: *const libc::c_char,
) -> *mut dc_array_t {
    let self_addr = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "configured_addr",
        Some(""),
    )
    .unwrap_or_default();

    let mut add_self = false;
    let ret = unsafe { dc_array_new(100) };

    let process_row = |row: &rusqlite::Row| {
        unsafe { dc_array_add_id(ret, row.get(0)?) };
        Ok(())
    };

    if 0 == listflags & 0x1 || query.is_null() {
        add_self = true;

        dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            "SELECT id FROM contacts WHERE addr!=?1 AND id>?2 AND origin>=?3 AND blocked=0 ORDER BY LOWER(name||addr),id;",
        ).and_then(|mut stmt| {
            stmt.query_map(params![self_addr, 9, 0x100], process_row).ok()
        });
    } else {
        let mut self_name: *mut libc::c_char = 0 as *mut libc::c_char;
        let mut self_name2: *mut libc::c_char = 0 as *mut libc::c_char;

        let s3strLikeCmd = format!("%{}%", if !query.is_null() { as_str(query) } else { "" });

        dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            "SELECT c.id FROM contacts c \
             LEFT JOIN acpeerstates ps ON c.addr=ps.addr  \
             WHERE c.addr!=?1 \
             AND c.id>?2 \
             AND c.origin>=?3 \
             AND c.blocked=0 \
             AND (c.name LIKE ?4 OR c.addr LIKE ?5) \
             AND (1=?6 OR LENGTH(ps.verified_key_fingerprint)!=0)  \
             ORDER BY LOWER(c.name||c.addr),c.id;",
        )
        .and_then(|mut stmt| {
            stmt.query_map(
                params![
                    self_addr,
                    9,
                    0x100,
                    &s3strLikeCmd,
                    &s3strLikeCmd,
                    if 0 != listflags & 0x1 { 0 } else { 1 },
                ],
                process_row,
            )
            .ok()
        });

        let self_name = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            "displayname",
            Some(""),
        )
        .unwrap_or_default();

        let self_name2 = unsafe { dc_stock_str(context, 2) };

        if query.is_null()
            || self_addr.contains(as_str(query))
            || self_name.contains(as_str(query))
            || 0 != unsafe { dc_str_contains(self_name2, query) }
        {
            add_self = true;
        }
        unsafe { free(self_name2 as *mut _) };
    }

    if 0 != listflags & 0x2 && add_self {
        unsafe { dc_array_add_id(ret, 1) };
    }

    ret
}

pub fn dc_get_blocked_cnt(context: &Context) -> libc::c_int {
    dc_sqlite3_query_row(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT COUNT(*) FROM contacts WHERE id>? AND blocked!=0",
        params![9],
        0,
    )
    .unwrap_or_default()
}

pub fn dc_get_blocked_contacts(context: &Context) -> *mut dc_array_t {
    let ret = unsafe { dc_array_new(100) };

    if dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT id FROM contacts WHERE id>? AND blocked!=0 ORDER BY LOWER(name||addr),id;",
    )
    .and_then(|mut stmt| {
        stmt.query_map(params![9], |row| {
            unsafe { dc_array_add_id(ret, row.get(0)?) };
            Ok(())
        })
        .ok()
    })
    .is_none()
    {
        unsafe { dc_array_unref(ret) };
        return std::ptr::null_mut();
    }

    ret
}

pub unsafe fn dc_get_contact_encrinfo(
    context: &Context,
    contact_id: uint32_t,
) -> *mut libc::c_char {
    let mut ret = String::new();
    let contact: *mut dc_contact_t = dc_contact_new(context);

    let mut fingerprint_self: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint_other_verified: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint_other_unverified: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p: *mut libc::c_char;

    if !(!dc_contact_load_from_db(contact, &context.sql.clone().read().unwrap(), contact_id)) {
        let peerstate = Peerstate::from_addr(
            context,
            &context.sql.clone().read().unwrap(),
            as_str((*contact).addr),
        );
        let loginparam =
            dc_loginparam_read(context, &context.sql.clone().read().unwrap(), "configured_");

        let mut self_key = Key::from_self_public(
            context,
            &loginparam.addr,
            &context.sql.clone().read().unwrap(),
        );

        if peerstate.is_some() && peerstate.as_ref().and_then(|p| p.peek_key(0)).is_some() {
            let peerstate = peerstate.as_ref().unwrap();
            p = dc_stock_str(
                context,
                if peerstate.prefer_encrypt == EncryptPreference::Mutual {
                    34i32
                } else {
                    25i32
                },
            );
            ret += as_str(p);
            free(p as *mut libc::c_void);
            if self_key.is_none() {
                dc_ensure_secret_key_exists(context);
                self_key = Key::from_self_public(
                    context,
                    &loginparam.addr,
                    &context.sql.clone().read().unwrap(),
                );
            }
            p = dc_stock_str(context, 30i32);
            ret += &format!(" {}:", as_str(p));
            free(p as *mut libc::c_void);

            fingerprint_self = self_key
                .map(|k| k.formatted_fingerprint_c())
                .unwrap_or(std::ptr::null_mut());
            fingerprint_other_verified = peerstate
                .peek_key(2)
                .map(|k| k.formatted_fingerprint_c())
                .unwrap_or(std::ptr::null_mut());
            fingerprint_other_unverified = peerstate
                .peek_key(0)
                .map(|k| k.formatted_fingerprint_c())
                .unwrap_or(std::ptr::null_mut());
            if peerstate.addr.is_some() && &loginparam.addr < peerstate.addr.as_ref().unwrap() {
                cat_fingerprint(
                    &mut ret,
                    &loginparam.addr,
                    fingerprint_self,
                    0 as *const libc::c_char,
                );
                cat_fingerprint(
                    &mut ret,
                    peerstate.addr.as_ref().unwrap(),
                    fingerprint_other_verified,
                    fingerprint_other_unverified,
                );
            } else {
                cat_fingerprint(
                    &mut ret,
                    peerstate.addr.as_ref().unwrap(),
                    fingerprint_other_verified,
                    fingerprint_other_unverified,
                );
                cat_fingerprint(
                    &mut ret,
                    &loginparam.addr,
                    fingerprint_self,
                    0 as *const libc::c_char,
                );
            }
        } else if 0 == loginparam.server_flags & 0x400 && 0 == loginparam.server_flags & 0x40000 {
            p = dc_stock_str(context, 27);
            ret += as_str(p);
            free(p as *mut libc::c_void);
        } else {
            p = dc_stock_str(context, 28);
            ret += as_str(p);
            free(p as *mut libc::c_void);
        }
    }

    dc_contact_unref(contact);

    free(fingerprint_self as *mut libc::c_void);
    free(fingerprint_other_verified as *mut libc::c_void);
    free(fingerprint_other_unverified as *mut libc::c_void);

    strdup(to_cstring(ret).as_ptr())
}

unsafe fn cat_fingerprint(
    ret: &mut String,
    addr: impl AsRef<str>,
    fingerprint_verified: *const libc::c_char,
    fingerprint_unverified: *const libc::c_char,
) {
    *ret += &format!(
        "\n\n{}:\n{}",
        addr.as_ref(),
        if !fingerprint_verified.is_null()
            && 0 != *fingerprint_verified.offset(0isize) as libc::c_int
        {
            as_str(fingerprint_verified)
        } else {
            as_str(fingerprint_unverified)
        },
    );
    if !fingerprint_verified.is_null()
        && 0 != *fingerprint_verified.offset(0isize) as libc::c_int
        && !fingerprint_unverified.is_null()
        && 0 != *fingerprint_unverified.offset(0isize) as libc::c_int
        && strcmp(fingerprint_verified, fingerprint_unverified) != 0
    {
        *ret += &format!(
            "\n\n{} (alternative):\n{}",
            addr.as_ref(),
            as_str(fingerprint_unverified)
        );
    }
}

pub fn dc_delete_contact(context: &Context, contact_id: u32) -> bool {
    if contact_id <= 9 {
        return false;
    }

    let count_contacts: i32 = dc_sqlite3_query_row(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT COUNT(*) FROM chats_contacts WHERE contact_id=?;",
        params![contact_id as i32],
        0,
    )
    .unwrap_or_default();

    let count_msgs: i32 = if count_contacts > 0 {
        dc_sqlite3_query_row(
            context,
            &context.sql.clone().read().unwrap(),
            "SELECT COUNT(*) FROM msgs WHERE from_id=? OR to_id=?;",
            params![contact_id as i32, contact_id as i32],
            0,
        )
        .unwrap_or_default()
    } else {
        0
    };

    if count_msgs > 0 {
        if dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            "DELETE FROM contacts WHERE id=?;",
            params![contact_id as i32],
        ) {
            unsafe { ((*context).cb)(context, Event::CONTACTS_CHANGED, 0, 0) };
            true
        } else {
            false
        }
    } else {
        false
    }
}

pub unsafe fn dc_get_contact(context: &Context, contact_id: uint32_t) -> *mut dc_contact_t {
    let mut ret: *mut dc_contact_t = dc_contact_new(context);
    if !dc_contact_load_from_db(ret, &context.sql.clone().read().unwrap(), contact_id) {
        dc_contact_unref(ret);
        ret = 0 as *mut dc_contact_t
    }
    ret
}

pub unsafe fn dc_contact_get_id(contact: *const dc_contact_t) -> uint32_t {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    (*contact).id
}

pub unsafe fn dc_contact_get_addr(contact: *const dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    dc_strdup((*contact).addr)
}

pub unsafe fn dc_contact_get_name(contact: *const dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    dc_strdup((*contact).name)
}

pub unsafe fn dc_contact_get_display_name(contact: *const dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
        return dc_strdup((*contact).name);
    }
    dc_strdup((*contact).addr)
}

pub unsafe fn dc_contact_get_name_n_addr(contact: *const dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
        return dc_mprintf(
            b"%s (%s)\x00" as *const u8 as *const libc::c_char,
            (*contact).name,
            (*contact).addr,
        );
    }
    dc_strdup((*contact).addr)
}

pub unsafe fn dc_contact_get_first_name(contact: *const dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
        return dc_get_first_name((*contact).name);
    }
    dc_strdup((*contact).addr)
}

pub unsafe fn dc_get_first_name(full_name: *const libc::c_char) -> *mut libc::c_char {
    let mut first_name: *mut libc::c_char = dc_strdup(full_name);
    let p1: *mut libc::c_char = strchr(first_name, ' ' as i32);
    if !p1.is_null() {
        *p1 = 0i32 as libc::c_char;
        dc_rtrim(first_name);
        if *first_name.offset(0isize) as libc::c_int == 0i32 {
            free(first_name as *mut libc::c_void);
            first_name = dc_strdup(full_name)
        }
    }
    first_name
}

pub fn dc_contact_get_profile_image(contact: *const dc_contact_t) -> *mut libc::c_char {
    let mut image_abs = 0 as *mut libc::c_char;

    if contact.is_null() || unsafe { (*contact).magic != 0xc047ac7 } {
        return image_abs;
    }

    if unsafe { (*contact).id } == 1 {
        let avatar = dc_get_config(unsafe { (*contact).context }, "selfavatar");
        if !avatar.is_empty() {
            image_abs = unsafe { dc_strdup(to_cstring(avatar).as_ptr()) };
        }
    }
    // TODO: else get image_abs from contact param
    image_abs
}

pub unsafe fn dc_contact_get_color(contact: *const dc_contact_t) -> uint32_t {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    dc_str_to_color((*contact).addr) as uint32_t
}

pub unsafe fn dc_contact_is_blocked(contact: *const dc_contact_t) -> libc::c_int {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return 0i32;
    }
    (*contact).blocked
}

/// Check if a contact was verified. E.g. by a secure-join QR code scan
/// and if the key has not changed since this verification.
///
/// The UI may draw a checkbox or something like that beside verified contacts.
///
/// Returns
///   - 0: contact is not verified.
///   -  2: SELF and contact have verified their fingerprints in both directions; in the UI typically checkmarks are shown.
pub unsafe fn dc_contact_is_verified(contact: *mut dc_contact_t) -> libc::c_int {
    dc_contact_is_verified_ex(contact, None)
}

/// Same as dc_contact_is_verified() but allows speeding up things
/// by adding the peerstate belonging to the contact.
/// If you do not have the peerstate available, it is loaded automatically.
pub unsafe fn dc_contact_is_verified_ex<'a>(
    contact: *mut dc_contact_t<'a>,
    peerstate: Option<&Peerstate<'a>>,
) -> libc::c_int {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return 0;
    }

    // we're always sort of secured-verified as we could verify the key on this device any time with the key
    // on this device
    if (*contact).id == 1 as libc::c_uint {
        return 2;
    }

    if let Some(peerstate) = peerstate {
        if peerstate.verified_key().is_some() {
            2
        } else {
            0
        }
    } else {
        let peerstate = Peerstate::from_addr(
            (*contact).context,
            &(*contact).context.sql.clone().read().unwrap(),
            as_str((*contact).addr),
        );

        let res = if let Some(ps) = peerstate {
            if ps.verified_key().is_some() {
                2
            } else {
                0
            }
        } else {
            0
        };

        res
    }
}

// Working with e-mail-addresses
pub fn dc_addr_cmp(addr1: impl AsRef<str>, addr2: impl AsRef<str>) -> bool {
    let norm1 = dc_addr_normalize_safe(addr1.as_ref());
    let norm2 = dc_addr_normalize_safe(addr2.as_ref());

    norm1 == norm2
}

pub fn dc_addr_equals_self(context: &Context, addr: *const libc::c_char) -> libc::c_int {
    let mut ret = 0;

    if !addr.is_null() {
        let normalized_addr = unsafe { dc_addr_normalize(addr) };
        if let Some(self_addr) = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            "configured_addr",
            None,
        ) {
            ret = (as_str(normalized_addr) == self_addr) as libc::c_int;
        }
        unsafe { free(normalized_addr as *mut libc::c_void) };
    }

    ret
}

pub unsafe fn dc_addr_equals_contact(
    context: &Context,
    addr: impl AsRef<str>,
    contact_id: u32,
) -> bool {
    if addr.as_ref().is_empty() {
        return false;
    }

    let contact = dc_contact_new(context);
    let mut addr_are_equal = false;

    if dc_contact_load_from_db(contact, &context.sql.clone().read().unwrap(), contact_id) {
        if !(*contact).addr.is_null() {
            let normalized_addr = dc_addr_normalize_safe(addr.as_ref());
            if as_str((*contact).addr) == normalized_addr {
                addr_are_equal = true;
            }
        }
        dc_contact_unref(contact);
    }

    addr_are_equal
}

// Context functions to work with contacts
pub fn dc_get_real_contact_cnt(context: &Context) -> size_t {
    if context.sql.clone().read().unwrap().conn().is_none() {
        return 0;
    }

    dc_sqlite3_query_row(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT COUNT(*) FROM contacts WHERE id>?;",
        params![9],
        0,
    )
    .unwrap_or_default()
}

pub unsafe fn dc_get_contact_origin(
    context: &Context,
    contact_id: uint32_t,
    mut ret_blocked: *mut libc::c_int,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut dummy: libc::c_int = 0i32;
    if ret_blocked.is_null() {
        ret_blocked = &mut dummy
    }
    let contact: *mut dc_contact_t = dc_contact_new(context);
    *ret_blocked = 0i32;
    if dc_contact_load_from_db(contact, &context.sql.clone().read().unwrap(), contact_id) {
        /* we could optimize this by loading only the needed fields */
        if 0 != (*contact).blocked {
            *ret_blocked = 1i32
        } else {
            ret = (*contact).origin
        }
    }
    dc_contact_unref(contact);
    ret
}

pub fn dc_real_contact_exists(context: &Context, contact_id: u32) -> bool {
    if context.sql.clone().read().unwrap().conn().is_none() || contact_id <= 9 {
        return false;
    }

    dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT id FROM contacts WHERE id=?;",
    )
    .map(|mut stmt| stmt.exists(params![contact_id as i32]).unwrap_or_default())
    .unwrap_or_default()
}

pub fn dc_scaleup_contact_origin(context: &Context, contact_id: u32, origin: libc::c_int) -> bool {
    dc_sqlite3_execute(
        context,
        &context.sql.clone().read().unwrap(),
        "UPDATE contacts SET origin=? WHERE id=? AND origin<?;",
        params![origin, contact_id as i32, origin],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_may_be_valid_addr() {
        unsafe {
            assert_eq!(dc_may_be_valid_addr(0 as *const libc::c_char), false);
            assert_eq!(
                dc_may_be_valid_addr(b"\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"user@domain.tld\x00" as *const u8 as *const libc::c_char),
                true
            );
            assert_eq!(
                dc_may_be_valid_addr(b"uuu\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"dd.tt\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"tt.dd@uu\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"u@d\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"u@d.\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"u@d.t\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"u@d.tt\x00" as *const u8 as *const libc::c_char),
                true
            );
            assert_eq!(
                dc_may_be_valid_addr(b"u@.tt\x00" as *const u8 as *const libc::c_char),
                false
            );
            assert_eq!(
                dc_may_be_valid_addr(b"@d.tt\x00" as *const u8 as *const libc::c_char),
                false
            );
        }
    }
}

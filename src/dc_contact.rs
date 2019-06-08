use crate::aheader::EncryptPreference;
use crate::constants::Event;
use crate::context::Context;
use crate::context::*;
use crate::dc_array::*;
use crate::dc_e2ee::*;
use crate::dc_log::*;
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

pub unsafe fn dc_marknoticed_contact(context: &Context, contact_id: uint32_t) {
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"UPDATE msgs SET state=13 WHERE from_id=? AND state=10;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    ((*context).cb)(
        context,
        Event::MSGS_CHANGED,
        0i32 as uintptr_t,
        0i32 as uintptr_t,
    );
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
    let mut contact_id: libc::c_int = 0i32;
    let mut addr_normalized: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut addr_self: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(addr.is_null() || *addr.offset(0isize) as libc::c_int == 0i32) {
        addr_normalized = dc_addr_normalize(addr);
        addr_self = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if strcasecmp(addr_normalized, addr_self) == 0i32 {
            contact_id = 1i32
        } else {
            stmt =
                dc_sqlite3_prepare(
                    context,&context.sql.clone().read().unwrap(),
                                   b"SELECT id FROM contacts WHERE addr=?1 COLLATE NOCASE AND id>?2 AND origin>=?3 AND blocked=0;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_text(
                stmt,
                1i32,
                addr_normalized as *const libc::c_char,
                -1i32,
                None,
            );
            sqlite3_bind_int(stmt, 2i32, 9i32);
            sqlite3_bind_int(stmt, 3i32, 0x100i32);
            if sqlite3_step(stmt) == 100i32 {
                contact_id = sqlite3_column_int(stmt, 0i32)
            }
        }
    }
    sqlite3_finalize(stmt);
    free(addr_normalized as *mut libc::c_void);
    free(addr_self as *mut libc::c_void);

    contact_id as uint32_t
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
    let current_block: u64;
    let mut send_event: libc::c_int = 0i32;
    let contact: *mut dc_contact_t = dc_contact_new(context);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(contact_id <= 9i32 as libc::c_uint) {
        if dc_contact_load_from_db(contact, &context.sql.clone().read().unwrap(), contact_id)
            && (*contact).blocked != new_blocking
        {
            stmt = dc_sqlite3_prepare(
                context,
                &context.sql.clone().read().unwrap(),
                b"UPDATE contacts SET blocked=? WHERE id=?;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_int(stmt, 1i32, new_blocking);
            sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
            if sqlite3_step(stmt) != 101i32 {
                current_block = 5249903830285462583;
            } else {
                sqlite3_finalize(stmt);
                stmt =
                    dc_sqlite3_prepare(
                        context,&context.sql.clone().read().unwrap(),
                                       b"UPDATE chats SET blocked=? WHERE type=? AND id IN (SELECT chat_id FROM chats_contacts WHERE contact_id=?);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, new_blocking);
                sqlite3_bind_int(stmt, 2i32, 100i32);
                sqlite3_bind_int(stmt, 3i32, contact_id as libc::c_int);
                if sqlite3_step(stmt) != 101i32 {
                    current_block = 5249903830285462583;
                } else {
                    dc_marknoticed_contact(context, contact_id);
                    send_event = 1i32;
                    current_block = 15652330335145281839;
                }
            }
        } else {
            current_block = 15652330335145281839;
        }
        match current_block {
            5249903830285462583 => {}
            _ => {
                if 0 != send_event {
                    ((*context).cb)(
                        context,
                        Event::CONTACTS_CHANGED,
                        0i32 as uintptr_t,
                        0i32 as uintptr_t,
                    );
                }
            }
        }
    }
    sqlite3_finalize(stmt);
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
    contact_id: uint32_t,
) -> bool {
    let current_block: u64;
    let mut success = false;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint) {
        dc_contact_empty(contact);
        if contact_id == 1i32 as libc::c_uint {
            (*contact).id = contact_id;
            (*contact).name = dc_stock_str((*contact).context, 2i32);
            (*contact).addr = dc_sqlite3_get_config(
                (*contact).context,
                sql,
                b"configured_addr\x00" as *const u8 as *const libc::c_char,
                b"\x00" as *const u8 as *const libc::c_char,
            );
            current_block = 5143058163439228106;
        } else {
            stmt =
                dc_sqlite3_prepare(
                    (*contact).context,sql,
                                   b"SELECT c.name, c.addr, c.origin, c.blocked, c.authname  FROM contacts c  WHERE c.id=?;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
            if sqlite3_step(stmt) != 100i32 {
                current_block = 12908855840294526070;
            } else {
                (*contact).id = contact_id;
                (*contact).name = dc_strdup(sqlite3_column_text(stmt, 0i32) as *mut libc::c_char);
                (*contact).addr = dc_strdup(sqlite3_column_text(stmt, 1i32) as *mut libc::c_char);
                (*contact).origin = sqlite3_column_int(stmt, 2i32);
                (*contact).blocked = sqlite3_column_int(stmt, 3i32);
                (*contact).authname =
                    dc_strdup(sqlite3_column_text(stmt, 4i32) as *mut libc::c_char);
                current_block = 5143058163439228106;
            }
        }
        match current_block {
            12908855840294526070 => {}
            _ => success = true,
        }
    }
    sqlite3_finalize(stmt);

    success
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
pub unsafe fn dc_add_or_lookup_contact(
    context: &Context,
    name: *const libc::c_char,
    addr__: *const libc::c_char,
    origin: libc::c_int,
    mut sth_modified: *mut libc::c_int,
) -> uint32_t {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut row_id: uint32_t = 0i32 as uint32_t;
    let mut dummy: libc::c_int = 0i32;
    let mut addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut addr_self: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut row_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut row_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut row_authname: *mut libc::c_char = 0 as *mut libc::c_char;
    if sth_modified.is_null() {
        sth_modified = &mut dummy
    }
    *sth_modified = 0i32;
    if !(addr__.is_null() || origin <= 0i32) {
        addr = dc_addr_normalize(addr__);
        addr_self = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if strcasecmp(addr, addr_self) == 0i32 {
            row_id = 1i32 as uint32_t
        } else if !dc_may_be_valid_addr(addr) {
            dc_log_warning(
                context,
                0i32,
                b"Bad address \"%s\" for contact \"%s\".\x00" as *const u8 as *const libc::c_char,
                addr,
                if !name.is_null() {
                    name
                } else {
                    b"<unset>\x00" as *const u8 as *const libc::c_char
                },
            );
        } else {
            stmt =
                dc_sqlite3_prepare(
                    context,&context.sql.clone().read().unwrap(),
                                   b"SELECT id, name, addr, origin, authname FROM contacts WHERE addr=? COLLATE NOCASE;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_text(stmt, 1i32, addr as *const libc::c_char, -1i32, None);
            if sqlite3_step(stmt) == 100i32 {
                let row_origin: libc::c_int;
                let mut update_addr: libc::c_int = 0i32;
                let mut update_name: libc::c_int = 0i32;
                let mut update_authname: libc::c_int = 0i32;
                row_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
                row_name = dc_strdup(sqlite3_column_text(stmt, 1i32) as *mut libc::c_char);
                row_addr = dc_strdup(sqlite3_column_text(stmt, 2i32) as *mut libc::c_char);
                row_origin = sqlite3_column_int(stmt, 3i32);
                row_authname = dc_strdup(sqlite3_column_text(stmt, 4i32) as *mut libc::c_char);
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                if !name.is_null() && 0 != *name.offset(0isize) as libc::c_int {
                    if 0 != *row_name.offset(0isize) {
                        if origin >= row_origin && strcmp(name, row_name) != 0i32 {
                            update_name = 1i32
                        }
                    } else {
                        update_name = 1i32
                    }
                    if origin == 0x10i32 && strcmp(name, row_authname) != 0i32 {
                        update_authname = 1i32
                    }
                }
                if origin >= row_origin && strcmp(addr, row_addr) != 0i32 {
                    update_addr = 1i32
                }
                if 0 != update_name
                    || 0 != update_authname
                    || 0 != update_addr
                    || origin > row_origin
                {
                    stmt = dc_sqlite3_prepare(
                        context,
                        &context.sql.clone().read().unwrap(),
                        b"UPDATE contacts SET name=?, addr=?, origin=?, authname=? WHERE id=?;\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    sqlite3_bind_text(
                        stmt,
                        1i32,
                        if 0 != update_name { name } else { row_name },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_text(
                        stmt,
                        2i32,
                        if 0 != update_addr { addr } else { row_addr },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_int(
                        stmt,
                        3i32,
                        if origin > row_origin {
                            origin
                        } else {
                            row_origin
                        },
                    );
                    sqlite3_bind_text(
                        stmt,
                        4i32,
                        if 0 != update_authname {
                            name
                        } else {
                            row_authname
                        },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_int(stmt, 5i32, row_id as libc::c_int);
                    sqlite3_step(stmt);
                    sqlite3_finalize(stmt);
                    stmt = 0 as *mut sqlite3_stmt;
                    if 0 != update_name {
                        stmt =
                            dc_sqlite3_prepare(
                                context,&context.sql.clone().read().unwrap(),
                                               b"UPDATE chats SET name=? WHERE type=? AND id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                        sqlite3_bind_text(stmt, 1i32, name, -1i32, None);
                        sqlite3_bind_int(stmt, 2i32, 100i32);
                        sqlite3_bind_int(stmt, 3i32, row_id as libc::c_int);
                        sqlite3_step(stmt);
                    }
                    *sth_modified = 1i32
                }
            } else {
                sqlite3_finalize(stmt);
                stmt = dc_sqlite3_prepare(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"INSERT INTO contacts (name, addr, origin) VALUES(?, ?, ?);\x00" as *const u8
                        as *const libc::c_char,
                );
                sqlite3_bind_text(
                    stmt,
                    1i32,
                    if !name.is_null() {
                        name
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    -1i32,
                    None,
                );
                sqlite3_bind_text(stmt, 2i32, addr, -1i32, None);
                sqlite3_bind_int(stmt, 3i32, origin);
                if sqlite3_step(stmt) == 101i32 {
                    row_id = dc_sqlite3_get_rowid(
                        context,
                        &context.sql.clone().read().unwrap(),
                        b"contacts\x00" as *const u8 as *const libc::c_char,
                        b"addr\x00" as *const u8 as *const libc::c_char,
                        addr,
                    );
                    *sth_modified = 2i32
                } else {
                    dc_log_error(
                        context,
                        0i32,
                        b"Cannot add contact.\x00" as *const u8 as *const libc::c_char,
                    );
                }
            }
        }
    }
    free(addr as *mut libc::c_void);
    free(addr_self as *mut libc::c_void);
    free(row_addr as *mut libc::c_void);
    free(row_name as *mut libc::c_void);
    free(row_authname as *mut libc::c_void);
    sqlite3_finalize(stmt);

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

pub unsafe fn dc_get_contacts(
    context: &Context,
    listflags: uint32_t,
    query: *const libc::c_char,
) -> *mut dc_array_t {
    let current_block: u64;
    let self_addr: *mut libc::c_char;
    let mut self_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_name2: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut add_self: libc::c_int = 0i32;
    let ret: *mut dc_array_t = dc_array_new(100i32 as size_t);
    let mut s3strLikeCmd: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;

    self_addr = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        b"\x00" as *const u8 as *const libc::c_char,
    );
    if 0 != listflags & 0x1i32 as libc::c_uint || !query.is_null() {
        s3strLikeCmd = sqlite3_mprintf(
            b"%%%s%%\x00" as *const u8 as *const libc::c_char,
            if !query.is_null() {
                query
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
        );
        if s3strLikeCmd.is_null() {
            current_block = 7597307149762829253;
        } else {
            stmt =
                dc_sqlite3_prepare(
                    context,&context.sql.clone().read().unwrap(),
                                       b"SELECT c.id FROM contacts c LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.addr!=?1 AND c.id>?2 AND c.origin>=?3 AND c.blocked=0 AND (c.name LIKE ?4 OR c.addr LIKE ?5) AND (1=?6 OR LENGTH(ps.verified_key_fingerprint)!=0)  ORDER BY LOWER(c.name||c.addr),c.id;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
            sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
            sqlite3_bind_int(stmt, 2i32, 9i32);
            sqlite3_bind_int(stmt, 3i32, 0x100i32);
            sqlite3_bind_text(stmt, 4i32, s3strLikeCmd, -1i32, None);
            sqlite3_bind_text(stmt, 5i32, s3strLikeCmd, -1i32, None);
            sqlite3_bind_int(
                stmt,
                6i32,
                if 0 != listflags & 0x1i32 as libc::c_uint {
                    0i32
                } else {
                    1i32
                },
            );
            self_name = dc_sqlite3_get_config(
                context,
                &context.sql.clone().read().unwrap(),
                b"displayname\x00" as *const u8 as *const libc::c_char,
                b"\x00" as *const u8 as *const libc::c_char,
            );
            self_name2 = dc_stock_str(context, 2i32);
            if query.is_null()
                || 0 != dc_str_contains(self_addr, query)
                || 0 != dc_str_contains(self_name, query)
                || 0 != dc_str_contains(self_name2, query)
            {
                add_self = 1i32
            }
            current_block = 15768484401365413375;
        }
    } else {
        stmt =
            dc_sqlite3_prepare(
                context,&context.sql.clone().read().unwrap(),
                                   b"SELECT id FROM contacts WHERE addr!=?1 AND id>?2 AND origin>=?3 AND blocked=0 ORDER BY LOWER(name||addr),id;\x00"
                                       as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
        sqlite3_bind_int(stmt, 2i32, 9i32);
        sqlite3_bind_int(stmt, 3i32, 0x100i32);
        add_self = 1i32;
        current_block = 15768484401365413375;
    }
    match current_block {
        7597307149762829253 => {}
        _ => {
            while sqlite3_step(stmt) == 100i32 {
                dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
            }
            if 0 != listflags & 0x2i32 as libc::c_uint && 0 != add_self {
                dc_array_add_id(ret, 1i32 as uint32_t);
            }
        }
    }

    sqlite3_finalize(stmt);
    sqlite3_free(s3strLikeCmd as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    free(self_name as *mut libc::c_void);
    free(self_name2 as *mut libc::c_void);

    ret
}

pub unsafe fn dc_get_blocked_cnt(context: &Context) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let stmt: *mut sqlite3_stmt;

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT COUNT(*) FROM contacts WHERE id>? AND blocked!=0\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, 9i32);
    if !(sqlite3_step(stmt) != 100i32) {
        ret = sqlite3_column_int(stmt, 0i32)
    }

    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_get_blocked_contacts(context: &Context) -> *mut dc_array_t {
    let ret: *mut dc_array_t = dc_array_new(100i32 as size_t);
    let stmt: *mut sqlite3_stmt;

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT id FROM contacts WHERE id>? AND blocked!=0 ORDER BY LOWER(name||addr),id;\x00"
            as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, 9i32);
    while sqlite3_step(stmt) == 100i32 {
        dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
    }

    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_get_contact_encrinfo(
    context: &Context,
    contact_id: uint32_t,
) -> *mut libc::c_char {
    let mut ret = String::new();
    let loginparam: *mut dc_loginparam_t = dc_loginparam_new();
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
        dc_loginparam_read(
            context,
            loginparam,
            &context.sql.clone().read().unwrap(),
            b"configured_\x00" as *const u8 as *const libc::c_char,
        );
        let mut self_key = Key::from_self_public(
            context,
            (*loginparam).addr,
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
                    (*loginparam).addr,
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
            if peerstate.addr.is_some()
                && as_str((*loginparam).addr) < peerstate.addr.as_ref().unwrap().as_str()
            {
                cat_fingerprint(
                    &mut ret,
                    to_string((*loginparam).addr),
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
                    to_string((*loginparam).addr),
                    fingerprint_self,
                    0 as *const libc::c_char,
                );
            }
        } else if 0 == (*loginparam).server_flags & 0x400i32
            && 0 == (*loginparam).server_flags & 0x40000i32
        {
            p = dc_stock_str(context, 27i32);
            ret += as_str(p);
            free(p as *mut libc::c_void);
        } else {
            p = dc_stock_str(context, 28i32);
            ret += as_str(p);
            free(p as *mut libc::c_void);
        }
    }

    dc_contact_unref(contact);
    dc_loginparam_unref(loginparam);

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
        && strcmp(fingerprint_verified, fingerprint_unverified) != 0i32
    {
        *ret += &format!(
            "\n\n{} (alternative):\n{}",
            addr.as_ref(),
            as_str(fingerprint_unverified)
        );
    }
}

pub unsafe fn dc_delete_contact(context: &Context, contact_id: uint32_t) -> bool {
    let mut success = false;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(contact_id <= 9i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT COUNT(*) FROM chats_contacts WHERE contact_id=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
        if !(sqlite3_step(stmt) != 100i32 || sqlite3_column_int(stmt, 0i32) >= 1i32) {
            sqlite3_finalize(stmt);
            stmt = dc_sqlite3_prepare(
                context,
                &context.sql.clone().read().unwrap(),
                b"SELECT COUNT(*) FROM msgs WHERE from_id=? OR to_id=?;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
            sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
            if !(sqlite3_step(stmt) != 100i32 || sqlite3_column_int(stmt, 0i32) >= 1i32) {
                sqlite3_finalize(stmt);
                stmt = dc_sqlite3_prepare(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"DELETE FROM contacts WHERE id=?;\x00" as *const u8 as *const libc::c_char,
                );
                sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
                if !(sqlite3_step(stmt) != 101i32) {
                    ((*context).cb)(
                        context,
                        Event::CONTACTS_CHANGED,
                        0i32 as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    success = true
                }
            }
        }
    }
    sqlite3_finalize(stmt);

    success
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

pub unsafe fn dc_contact_get_profile_image(contact: *const dc_contact_t) -> *mut libc::c_char {
    let mut selfavatar: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut image_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint) {
        if (*contact).id == 1i32 as libc::c_uint {
            selfavatar = dc_get_config(
                (*contact).context,
                b"selfavatar\x00" as *const u8 as *const libc::c_char,
            );
            if !selfavatar.is_null() && 0 != *selfavatar.offset(0isize) as libc::c_int {
                image_abs = dc_strdup(selfavatar)
            }
        }
    }
    // TODO: else get image_abs from contact param
    free(selfavatar as *mut libc::c_void);
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
pub unsafe fn dc_addr_cmp(addr1: *const libc::c_char, addr2: *const libc::c_char) -> libc::c_int {
    let norm1: *mut libc::c_char = dc_addr_normalize(addr1);
    let norm2: *mut libc::c_char = dc_addr_normalize(addr2);
    let ret: libc::c_int = strcasecmp(addr1, addr2);
    free(norm1 as *mut libc::c_void);
    free(norm2 as *mut libc::c_void);
    ret
}

pub unsafe fn dc_addr_equals_self(context: &Context, addr: *const libc::c_char) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut normalized_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    if !addr.is_null() {
        normalized_addr = dc_addr_normalize(addr);
        self_addr = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if !self_addr.is_null() {
            ret = if strcasecmp(normalized_addr, self_addr) == 0i32 {
                1i32
            } else {
                0i32
            }
        }
    }
    free(self_addr as *mut libc::c_void);
    free(normalized_addr as *mut libc::c_void);
    ret
}

pub unsafe fn dc_addr_equals_contact(
    context: &Context,
    addr: *const libc::c_char,
    contact_id: uint32_t,
) -> bool {
    let mut addr_are_equal = false;
    if !addr.is_null() {
        let contact: *mut dc_contact_t = dc_contact_new(context);
        if dc_contact_load_from_db(contact, &context.sql.clone().read().unwrap(), contact_id) {
            if !(*contact).addr.is_null() {
                let normalized_addr: *mut libc::c_char = dc_addr_normalize(addr);
                if strcasecmp((*contact).addr, normalized_addr) == 0i32 {
                    addr_are_equal = true;
                }
                free(normalized_addr as *mut libc::c_void);
            }
        }
        dc_contact_unref(contact);
    }
    addr_are_equal
}

// Context functions to work with contacts
pub unsafe fn dc_get_real_contact_cnt(context: &Context) -> size_t {
    let mut ret: size_t = 0i32 as size_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !context.sql.clone().read().unwrap().cobj.is_null() {
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT COUNT(*) FROM contacts WHERE id>?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, 9i32);
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32) as size_t
        }
    }
    sqlite3_finalize(stmt);
    ret
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

pub unsafe fn dc_real_contact_exists(context: &Context, contact_id: uint32_t) -> bool {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut ret = false;
    if !(context.sql.clone().read().unwrap().cobj.is_null() || contact_id <= 9i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT id FROM contacts WHERE id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
        if sqlite3_step(stmt) == 100i32 {
            ret = true
        }
    }
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_scaleup_contact_origin(
    context: &Context,
    contact_id: uint32_t,
    origin: libc::c_int,
) {
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"UPDATE contacts SET origin=? WHERE id=? AND origin<?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, origin);
    sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
    sqlite3_bind_int(stmt, 3i32, origin);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
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

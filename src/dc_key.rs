use libc;

use crate::dc_context::dc_context_t;
use crate::dc_log::*;
use crate::dc_pgp::*;
use crate::dc_sqlite3::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_key_t {
    pub binary: *mut libc::c_void,
    pub bytes: libc::c_int,
    pub type_0: libc::c_int,
    pub _m_heap_refcnt: libc::c_int,
}

#[inline]
pub unsafe fn toupper(mut _c: libc::c_int) -> libc::c_int {
    return __toupper(_c);
}

pub unsafe fn dc_key_new() -> *mut dc_key_t {
    let mut key: *mut dc_key_t;
    key = calloc(1, ::std::mem::size_of::<dc_key_t>()) as *mut dc_key_t;
    if key.is_null() {
        exit(44i32);
    }
    (*key)._m_heap_refcnt = 1i32;

    key
}

pub unsafe fn dc_key_ref(mut key: *mut dc_key_t) -> *mut dc_key_t {
    if key.is_null() {
        return 0 as *mut dc_key_t;
    }
    (*key)._m_heap_refcnt += 1;

    key
}

pub unsafe fn dc_key_unref(mut key: *mut dc_key_t) {
    if key.is_null() {
        return;
    }
    (*key)._m_heap_refcnt -= 1;
    if (*key)._m_heap_refcnt != 0i32 {
        return;
    }
    dc_key_empty(key);
    free(key as *mut libc::c_void);
}

unsafe fn dc_key_empty(mut key: *mut dc_key_t) {
    if key.is_null() {
        return;
    }
    if (*key).type_0 == 1i32 {
        dc_wipe_secret_mem((*key).binary, (*key).bytes as size_t);
    }
    free((*key).binary);
    (*key).binary = 0 as *mut libc::c_void;
    (*key).bytes = 0i32;
    (*key).type_0 = 0i32;
}

pub unsafe fn dc_wipe_secret_mem(mut buf: *mut libc::c_void, mut buf_bytes: size_t) {
    if buf.is_null() || buf_bytes <= 0 {
        return;
    }
    memset(buf, 0i32, buf_bytes);
}

// TODO should return bool /rtn
pub unsafe fn dc_key_set_from_binary(
    mut key: *mut dc_key_t,
    mut data: *const libc::c_void,
    mut bytes: libc::c_int,
    mut type_0: libc::c_int,
) -> libc::c_int {
    dc_key_empty(key);
    if key.is_null() || data == 0 as *mut libc::c_void || bytes <= 0i32 {
        return 0i32;
    }
    (*key).binary = malloc(bytes as size_t);
    if (*key).binary.is_null() {
        exit(40i32);
    }
    memcpy((*key).binary, data, bytes as size_t);
    (*key).bytes = bytes;
    (*key).type_0 = type_0;

    1
}

pub unsafe fn dc_key_set_from_key(mut key: *mut dc_key_t, mut o: *const dc_key_t) -> libc::c_int {
    dc_key_empty(key);
    if key.is_null() || o.is_null() {
        return 0i32;
    }

    dc_key_set_from_binary(key, (*o).binary, (*o).bytes, (*o).type_0)
}

// TODO should return bool /rtn
pub unsafe extern "C" fn dc_key_set_from_stmt(
    mut key: *mut dc_key_t,
    mut stmt: *mut sqlite3_stmt,
    mut index: libc::c_int,
    mut type_0: libc::c_int,
) -> libc::c_int {
    dc_key_empty(key);
    if key.is_null() || stmt.is_null() {
        return 0i32;
    }

    dc_key_set_from_binary(
        key,
        sqlite3_column_blob(stmt, index) as *mut libc::c_uchar as *const libc::c_void,
        sqlite3_column_bytes(stmt, index),
        type_0,
    )
}

// TODO should return bool /rtn
pub unsafe fn dc_key_set_from_base64(
    mut key: *mut dc_key_t,
    mut base64: *const libc::c_char,
    mut type_0: libc::c_int,
) -> libc::c_int {
    let mut indx: size_t = 0i32 as size_t;
    let mut result_len: size_t = 0i32 as size_t;
    let mut result: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_key_empty(key);
    if key.is_null() || base64.is_null() {
        return 0i32;
    }
    if mailmime_base64_body_parse(
        base64,
        strlen(base64),
        &mut indx,
        &mut result,
        &mut result_len,
    ) != MAILIMF_NO_ERROR as libc::c_int
        || result.is_null()
        || result_len == 0
    {
        return 0;
    }
    dc_key_set_from_binary(
        key,
        result as *const libc::c_void,
        result_len as libc::c_int,
        type_0,
    );
    mmap_string_unref(result);

    1
}

// TODO should return bool /rtn
pub unsafe fn dc_key_equals(mut key: *const dc_key_t, mut o: *const dc_key_t) -> libc::c_int {
    if key.is_null()
        || o.is_null()
        || (*key).binary.is_null()
        || (*key).bytes <= 0i32
        || (*o).binary.is_null()
        || (*o).bytes <= 0i32
    {
        return 0;
    }
    if (*key).bytes != (*o).bytes {
        return 0;
    }
    if (*key).type_0 != (*o).type_0 {
        return 0;
    }

    if memcmp((*key).binary, (*o).binary, (*o).bytes as size_t) == 0 {
        1
    } else {
        0
    }
}

// TODO should return bool /rtn
pub unsafe fn dc_key_save_self_keypair(
    context: &dc_context_t,
    public_key: *const dc_key_t,
    private_key: *const dc_key_t,
    addr: *const libc::c_char,
    is_default: libc::c_int,
    sql: &dc_sqlite3_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(public_key.is_null()
        || private_key.is_null()
        || addr.is_null()
        || (*public_key).binary.is_null()
        || (*private_key).binary.is_null())
    {
        stmt =
            dc_sqlite3_prepare(
                context,
                sql,
                b"INSERT INTO keypairs (addr, is_default, public_key, private_key, created) VALUES (?,?,?,?,?);\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1i32, addr, -1i32, None);
        sqlite3_bind_int(stmt, 2i32, is_default);
        sqlite3_bind_blob(stmt, 3i32, (*public_key).binary, (*public_key).bytes, None);
        sqlite3_bind_blob(
            stmt,
            4i32,
            (*private_key).binary,
            (*private_key).bytes,
            None,
        );
        sqlite3_bind_int64(stmt, 5i32, time(0 as *mut time_t) as sqlite3_int64);
        if !(sqlite3_step(stmt) != 101i32) {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_key_load_self_public(
    context: &dc_context_t,
    key: *mut dc_key_t,
    self_addr: *const libc::c_char,
    sql: &dc_sqlite3_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(key.is_null() || self_addr.is_null()) {
        dc_key_empty(key);
        stmt = dc_sqlite3_prepare(
            context,
            sql,
            b"SELECT public_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            dc_key_set_from_stmt(key, stmt, 0i32, 0i32);
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_key_load_self_private(
    context: &dc_context_t,
    key: *mut dc_key_t,
    self_addr: *const libc::c_char,
    sql: &dc_sqlite3_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(key.is_null() || self_addr.is_null()) {
        dc_key_empty(key);
        stmt = dc_sqlite3_prepare(
            context,
            sql,
            b"SELECT private_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            dc_key_set_from_stmt(key, stmt, 0i32, 1i32);
            success = 1i32;
        }
    }
    sqlite3_finalize(stmt);

    success
}

/* the result must be freed */
pub unsafe fn dc_render_base64(
    mut buf: *const libc::c_void,
    mut buf_bytes: size_t,
    mut break_every: libc::c_int,
    mut break_chars: *const libc::c_char,
    mut add_checksum: libc::c_int,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(buf == 0 as *mut libc::c_void || buf_bytes <= 0) {
        ret = encode_base64(buf as *const libc::c_char, buf_bytes as libc::c_int);
        if !ret.is_null() {
            if break_every > 0i32 {
                let mut temp: *mut libc::c_char = ret;
                ret = dc_insert_breaks(temp, break_every, break_chars);
                free(temp as *mut libc::c_void);
            }
            if add_checksum == 2i32 {
                let mut checksum: libc::c_long = crc_octets(buf as *const libc::c_uchar, buf_bytes);
                let mut c: [uint8_t; 3] = [0; 3];
                c[0usize] = (checksum >> 16i32 & 0xffi32 as libc::c_long) as uint8_t;
                c[1usize] = (checksum >> 8i32 & 0xffi32 as libc::c_long) as uint8_t;
                c[2usize] = (checksum & 0xffi32 as libc::c_long) as uint8_t;
                let mut c64: *mut libc::c_char =
                    encode_base64(c.as_mut_ptr() as *const libc::c_char, 3i32);
                let mut temp_0: *mut libc::c_char = ret;
                ret = dc_mprintf(
                    b"%s%s=%s\x00" as *const u8 as *const libc::c_char,
                    temp_0,
                    break_chars,
                    c64,
                );
                free(temp_0 as *mut libc::c_void);
                free(c64 as *mut libc::c_void);
            }
        }
    }

    ret
}

/*******************************************************************************
 * Render keys
 ******************************************************************************/
unsafe fn crc_octets(mut octets: *const libc::c_uchar, mut len: size_t) -> libc::c_long {
    let mut crc: libc::c_long = 0xb704ce;
    loop {
        let fresh0 = len;
        len = len.wrapping_sub(1);
        if !(0 != fresh0) {
            break;
        }
        let fresh1 = octets;
        octets = octets.offset(1);
        crc ^= ((*fresh1 as libc::c_int) << 16i32) as libc::c_long;
        let mut i: libc::c_int = 0i32;
        while i < 8i32 {
            crc <<= 1i32;
            if 0 != crc & 0x1000000 as libc::c_long {
                crc ^= 0x1864cfb
            }
            i += 1
        }
    }

    crc & 0xffffff
}

/* the result must be freed */
pub unsafe fn dc_key_render_base64(
    mut key: *const dc_key_t,
    mut break_every: libc::c_int,
    mut break_chars: *const libc::c_char,
    mut add_checksum: libc::c_int,
) -> *mut libc::c_char {
    if key.is_null() {
        return 0 as *mut libc::c_char;
    }

    dc_render_base64(
        (*key).binary,
        (*key).bytes as size_t,
        break_every,
        break_chars,
        add_checksum,
    )
}

/* each header line must be terminated by \r\n, the result must be freed */
pub unsafe fn dc_key_render_asc(
    mut key: *const dc_key_t,
    mut add_header_lines: *const libc::c_char,
) -> *mut libc::c_char {
    /* see RFC 4880, 6.2.  Forming ASCII Armor, https://tools.ietf.org/html/rfc4880#section-6.2 */
    let mut base64: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !key.is_null() {
        base64 = dc_key_render_base64(
            key,
            76i32,
            b"\r\n\x00" as *const u8 as *const libc::c_char,
            2i32,
        );
        if !base64.is_null() {
            /*checksum in new line*/
            /* RFC: The encoded output stream must be represented in lines of no more than 76 characters each. */
            ret =
                dc_mprintf(b"-----BEGIN PGP %s KEY BLOCK-----\r\n%s\r\n%s\r\n-----END PGP %s KEY BLOCK-----\r\n\x00"
                               as *const u8 as *const libc::c_char,
                           if (*key).type_0 == 0i32 {
                               b"PUBLIC\x00" as *const u8 as
                                   *const libc::c_char
                           } else {
                               b"PRIVATE\x00" as *const u8 as
                                   *const libc::c_char
                           },
                           if !add_header_lines.is_null() {
                               add_header_lines
                           } else {
                               b"\x00" as *const u8 as *const libc::c_char
                           }, base64,
                           if (*key).type_0 == 0i32 {
                               b"PUBLIC\x00" as *const u8 as
                                   *const libc::c_char
                           } else {
                               b"PRIVATE\x00" as *const u8 as
                                   *const libc::c_char
                           })
        }
    }
    free(base64 as *mut libc::c_void);

    ret
}

// TODO should return bool /rtn
pub unsafe fn dc_key_render_asc_to_file(
    mut key: *const dc_key_t,
    mut file: *const libc::c_char,
    mut context: &dc_context_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut file_content: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(key.is_null() || file.is_null()) {
        file_content = dc_key_render_asc(key, 0 as *const libc::c_char);
        if !file_content.is_null() {
            if 0 == dc_write_file(
                context,
                file,
                file_content as *const libc::c_void,
                strlen(file_content),
            ) {
                dc_log_error(
                    context,
                    0i32,
                    b"Cannot write key to %s\x00" as *const u8 as *const libc::c_char,
                    file,
                );
            } else {
                success = 1i32
            }
        }
    }
    free(file_content as *mut libc::c_void);

    success
}

pub unsafe fn dc_format_fingerprint(mut fingerprint: *const libc::c_char) -> *mut libc::c_char {
    let mut i: libc::c_int = 0i32;
    let mut fingerprint_len: libc::c_int = strlen(fingerprint) as libc::c_int;
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    while 0 != *fingerprint.offset(i as isize) {
        dc_strbuilder_catf(
            &mut ret as *mut dc_strbuilder_t,
            b"%c\x00" as *const u8 as *const libc::c_char,
            *fingerprint.offset(i as isize) as libc::c_int,
        );
        i += 1;
        if i != fingerprint_len {
            if i % 20i32 == 0i32 {
                dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
            } else if i % 4i32 == 0i32 {
                dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
            }
        }
    }

    ret.buf
}

pub unsafe fn dc_normalize_fingerprint(mut in_0: *const libc::c_char) -> *mut libc::c_char {
    if in_0.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut out: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut out, 0i32);
    let mut p1: *const libc::c_char = in_0;
    while 0 != *p1 {
        if *p1 as libc::c_int >= '0' as i32 && *p1 as libc::c_int <= '9' as i32
            || *p1 as libc::c_int >= 'A' as i32 && *p1 as libc::c_int <= 'F' as i32
            || *p1 as libc::c_int >= 'a' as i32 && *p1 as libc::c_int <= 'f' as i32
        {
            dc_strbuilder_catf(
                &mut out as *mut dc_strbuilder_t,
                b"%c\x00" as *const u8 as *const libc::c_char,
                toupper(*p1 as libc::c_int),
            );
        }
        p1 = p1.offset(1isize)
    }

    out.buf
}

pub unsafe fn dc_key_get_fingerprint(
    context: &dc_context_t,
    key: *const dc_key_t,
) -> *mut libc::c_char {
    let mut fingerprint_buf: *mut uint8_t = 0 as *mut uint8_t;
    let mut fingerprint_bytes: size_t = 0i32 as size_t;
    let mut fingerprint_hex: *mut libc::c_char = 0 as *mut libc::c_char;
    if !key.is_null() {
        if !(0
            == dc_pgp_calc_fingerprint(context, key, &mut fingerprint_buf, &mut fingerprint_bytes))
        {
            fingerprint_hex = dc_binary_to_uc_hex(fingerprint_buf, fingerprint_bytes)
        }
    }
    free(fingerprint_buf as *mut libc::c_void);
    return if !fingerprint_hex.is_null() {
        fingerprint_hex
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}

pub unsafe fn dc_key_get_formatted_fingerprint(
    context: &dc_context_t,
    key: *const dc_key_t,
) -> *mut libc::c_char {
    let mut rawhex: *mut libc::c_char = dc_key_get_fingerprint(context, key);
    let mut formatted: *mut libc::c_char = dc_format_fingerprint(rawhex);
    free(rawhex as *mut libc::c_void);

    formatted
}

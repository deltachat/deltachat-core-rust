use libc;

use crate::dc_sqlite3::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_loginparam_t {
    pub addr: *mut libc::c_char,
    pub mail_server: *mut libc::c_char,
    pub mail_user: *mut libc::c_char,
    pub mail_pw: *mut libc::c_char,
    pub mail_port: i32,
    pub send_server: *mut libc::c_char,
    pub send_user: *mut libc::c_char,
    pub send_pw: *mut libc::c_char,
    pub send_port: i32,
    pub server_flags: i32,
}

pub unsafe fn dc_loginparam_new() -> *mut dc_loginparam_t {
    let mut loginparam: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    loginparam = calloc(1, ::std::mem::size_of::<dc_loginparam_t>()) as *mut dc_loginparam_t;
    if loginparam.is_null() {
        exit(22i32);
    }
    return loginparam;
}
pub unsafe fn dc_loginparam_unref(mut loginparam: *mut dc_loginparam_t) {
    if loginparam.is_null() {
        return;
    }
    dc_loginparam_empty(loginparam);
    free(loginparam as *mut libc::c_void);
}
/* clears all data and frees its memory. All pointers are NULL after this function is called. */
pub unsafe fn dc_loginparam_empty(mut loginparam: *mut dc_loginparam_t) {
    if loginparam.is_null() {
        return;
    }
    free((*loginparam).addr as *mut libc::c_void);
    (*loginparam).addr = 0 as *mut libc::c_char;
    free((*loginparam).mail_server as *mut libc::c_void);
    (*loginparam).mail_server = 0 as *mut libc::c_char;
    (*loginparam).mail_port = 0i32;
    free((*loginparam).mail_user as *mut libc::c_void);
    (*loginparam).mail_user = 0 as *mut libc::c_char;
    free((*loginparam).mail_pw as *mut libc::c_void);
    (*loginparam).mail_pw = 0 as *mut libc::c_char;
    free((*loginparam).send_server as *mut libc::c_void);
    (*loginparam).send_server = 0 as *mut libc::c_char;
    (*loginparam).send_port = 0i32;
    free((*loginparam).send_user as *mut libc::c_void);
    (*loginparam).send_user = 0 as *mut libc::c_char;
    free((*loginparam).send_pw as *mut libc::c_void);
    (*loginparam).send_pw = 0 as *mut libc::c_char;
    (*loginparam).server_flags = 0i32;
}
pub unsafe fn dc_loginparam_read(
    mut loginparam: *mut dc_loginparam_t,
    mut sql: &mut dc_sqlite3_t,
    mut prefix: *const libc::c_char,
) {
    let mut key: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_loginparam_empty(loginparam);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"addr\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).addr = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_server\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_server = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_port\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_port = dc_sqlite3_get_config_int(sql, key, 0i32);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_user\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_user = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_pw\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_pw = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_server\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_server = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_port\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_port = dc_sqlite3_get_config_int(sql, key, 0i32);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_user\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_user = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_pw\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_pw = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"server_flags\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).server_flags = dc_sqlite3_get_config_int(sql, key, 0i32);
    sqlite3_free(key as *mut libc::c_void);
}
pub unsafe fn dc_loginparam_write(
    mut loginparam: *const dc_loginparam_t,
    mut sql: &mut dc_sqlite3_t,
    mut prefix: *const libc::c_char,
) {
    let mut key: *mut libc::c_char = 0 as *mut libc::c_char;
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"addr\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(sql, key, (*loginparam).addr);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_server\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(sql, key, (*loginparam).mail_server);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_port\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config_int(sql, key, (*loginparam).mail_port);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_user\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(sql, key, (*loginparam).mail_user);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_pw\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(sql, key, (*loginparam).mail_pw);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_server\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(sql, key, (*loginparam).send_server);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_port\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config_int(sql, key, (*loginparam).send_port);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_user\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(sql, key, (*loginparam).send_user);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_pw\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(sql, key, (*loginparam).send_pw);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"server_flags\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config_int(sql, key, (*loginparam).server_flags);
    sqlite3_free(key as *mut libc::c_void);
}
pub unsafe fn dc_loginparam_get_readable(
    mut loginparam: *const dc_loginparam_t,
) -> *mut libc::c_char {
    let mut unset: *const libc::c_char = b"0\x00" as *const u8 as *const libc::c_char;
    let mut pw: *const libc::c_char = b"***\x00" as *const u8 as *const libc::c_char;
    if loginparam.is_null() {
        return dc_strdup(0 as *const libc::c_char);
    }
    let mut flags_readable: *mut libc::c_char = get_readable_flags((*loginparam).server_flags);
    let mut ret: *mut libc::c_char = dc_mprintf(
        b"%s %s:%s:%s:%i %s:%s:%s:%i %s\x00" as *const u8 as *const libc::c_char,
        if !(*loginparam).addr.is_null() {
            (*loginparam).addr
        } else {
            unset
        },
        if !(*loginparam).mail_user.is_null() {
            (*loginparam).mail_user
        } else {
            unset
        },
        if !(*loginparam).mail_pw.is_null() {
            pw
        } else {
            unset
        },
        if !(*loginparam).mail_server.is_null() {
            (*loginparam).mail_server
        } else {
            unset
        },
        (*loginparam).mail_port,
        if !(*loginparam).send_user.is_null() {
            (*loginparam).send_user
        } else {
            unset
        },
        if !(*loginparam).send_pw.is_null() {
            pw
        } else {
            unset
        },
        if !(*loginparam).send_server.is_null() {
            (*loginparam).send_server
        } else {
            unset
        },
        (*loginparam).send_port,
        flags_readable,
    );
    free(flags_readable as *mut libc::c_void);
    return ret;
}
unsafe fn get_readable_flags(mut flags: libc::c_int) -> *mut libc::c_char {
    let mut strbuilder: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut strbuilder, 0i32);
    let mut bit: libc::c_int = 0i32;
    while bit <= 30i32 {
        if 0 != flags & 1i32 << bit {
            let mut flag_added: libc::c_int = 0i32;
            if 1i32 << bit == 0x2i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"OAUTH2 \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 1i32 << bit == 0x4i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"AUTH_NORMAL \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 1i32 << bit == 0x100i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"IMAP_STARTTLS \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 1i32 << bit == 0x200i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"IMAP_SSL \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 1i32 << bit == 0x400i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"IMAP_PLAIN \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 1i32 << bit == 0x10000i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"SMTP_STARTTLS \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 1i32 << bit == 0x20000i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"SMTP_SSL \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 1i32 << bit == 0x40000i32 {
                dc_strbuilder_cat(
                    &mut strbuilder,
                    b"SMTP_PLAIN \x00" as *const u8 as *const libc::c_char,
                );
                flag_added = 1i32
            }
            if 0 == flag_added {
                let mut temp: *mut libc::c_char = dc_mprintf(
                    b"0x%x \x00" as *const u8 as *const libc::c_char,
                    1i32 << bit,
                );
                dc_strbuilder_cat(&mut strbuilder, temp);
                free(temp as *mut libc::c_void);
            }
        }
        bit += 1
    }
    if *strbuilder.buf.offset(0isize) as libc::c_int == 0i32 {
        dc_strbuilder_cat(
            &mut strbuilder,
            b"0\x00" as *const u8 as *const libc::c_char,
        );
    }
    dc_trim(strbuilder.buf);
    return strbuilder.buf;
}

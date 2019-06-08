use crate::context::Context;
use crate::dc_sqlite3::*;
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
    let loginparam: *mut dc_loginparam_t;
    loginparam = calloc(1, ::std::mem::size_of::<dc_loginparam_t>()) as *mut dc_loginparam_t;
    assert!(!loginparam.is_null());

    loginparam
}

pub unsafe fn dc_loginparam_unref(loginparam: *mut dc_loginparam_t) {
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
    context: &Context,
    loginparam: *mut dc_loginparam_t,
    sql: &dc_sqlite3_t,
    prefix: *const libc::c_char,
) {
    let mut key: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_loginparam_empty(loginparam);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"addr\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).addr = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_server\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_server = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_port\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_port = dc_sqlite3_get_config_int(context, sql, key, 0i32);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_user\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_user = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_pw\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).mail_pw = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_server\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_server = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_port\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_port = dc_sqlite3_get_config_int(context, sql, key, 0i32);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_user\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_user = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_pw\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).send_pw = dc_sqlite3_get_config(context, sql, key, 0 as *const libc::c_char);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"server_flags\x00" as *const u8 as *const libc::c_char,
    );
    (*loginparam).server_flags = dc_sqlite3_get_config_int(context, sql, key, 0i32);
    sqlite3_free(key as *mut libc::c_void);
}

pub unsafe fn dc_loginparam_write(
    context: &Context,
    loginparam: *const dc_loginparam_t,
    sql: &dc_sqlite3_t,
    prefix: *const libc::c_char,
) {
    let mut key: *mut libc::c_char = 0 as *mut libc::c_char;
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"addr\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(context, sql, key, (*loginparam).addr);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_server\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(context, sql, key, (*loginparam).mail_server);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_port\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config_int(context, sql, key, (*loginparam).mail_port);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_user\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(context, sql, key, (*loginparam).mail_user);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"mail_pw\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(context, sql, key, (*loginparam).mail_pw);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_server\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(context, sql, key, (*loginparam).send_server);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_port\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config_int(context, sql, key, (*loginparam).send_port);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_user\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(context, sql, key, (*loginparam).send_user);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"send_pw\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config(context, sql, key, (*loginparam).send_pw);
    sqlite3_free(key as *mut libc::c_void);
    key = sqlite3_mprintf(
        b"%s%s\x00" as *const u8 as *const libc::c_char,
        prefix,
        b"server_flags\x00" as *const u8 as *const libc::c_char,
    );
    dc_sqlite3_set_config_int(context, sql, key, (*loginparam).server_flags);
    sqlite3_free(key as *mut libc::c_void);
}

pub unsafe fn dc_loginparam_get_readable(loginparam: *const dc_loginparam_t) -> *mut libc::c_char {
    let unset: *const libc::c_char = b"0\x00" as *const u8 as *const libc::c_char;
    let pw: *const libc::c_char = b"***\x00" as *const u8 as *const libc::c_char;
    if loginparam.is_null() {
        return dc_strdup(0 as *const libc::c_char);
    }
    let flags_readable: *mut libc::c_char = get_readable_flags((*loginparam).server_flags);
    let ret: *mut libc::c_char = dc_mprintf(
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

    ret
}

fn get_readable_flags(flags: libc::c_int) -> *mut libc::c_char {
    let mut res = String::new();
    for bit in 0..31 {
        if 0 != flags & 1 << bit {
            let mut flag_added: libc::c_int = 0;
            if 1 << bit == 0x2 {
                res += "OAUTH2 ";
                flag_added = 1;
            }
            if 1 << bit == 0x4 {
                res += "AUTH_NORMAL ";
                flag_added = 1;
            }
            if 1 << bit == 0x100 {
                res += "IMAP_STARTTLS ";
                flag_added = 1;
            }
            if 1 << bit == 0x200 {
                res += "IMAP_SSL ";
                flag_added = 1;
            }
            if 1 << bit == 0x400 {
                res += "IMAP_PLAIN ";
                flag_added = 1;
            }
            if 1 << bit == 0x10000 {
                res += "SMTP_STARTTLS ";
                flag_added = 1
            }
            if 1 << bit == 0x20000 {
                res += "SMTP_SSL ";
                flag_added = 1
            }
            if 1 << bit == 0x40000 {
                res += "SMTP_PLAIN ";
                flag_added = 1
            }
            if 0 == flag_added {
                res += &format!("{:#0x}", 1 << bit);
            }
        }
    }
    if res.is_empty() {
        res += "0";
    }

    unsafe { strdup(to_cstring(res).as_ptr()) }
}

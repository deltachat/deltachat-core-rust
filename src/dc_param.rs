use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/// for msgs and jobs
pub const DC_PARAM_FILE: char = 'f';
/// for msgs
pub const DC_PARAM_WIDTH: char = 'w';
/// for msgs
pub const DC_PARAM_HEIGHT: char = 'h';
/// for msgs
pub const DC_PARAM_DURATION: char = 'd';
/// for msgs
pub const DC_PARAM_MIMETYPE: char = 'm';
/// for msgs: incoming: message is encryoted, outgoing: guarantee E2EE or the message is not send
pub const DC_PARAM_GUARANTEE_E2EE: char = 'c';
/// for msgs: decrypted with validation errors or without mutual set, if neither 'c' nor 'e' are preset, the messages is only transport encrypted
pub const DC_PARAM_ERRONEOUS_E2EE: char = 'e';
/// for msgs: force unencrypted message, either DC_FP_ADD_AUTOCRYPT_HEADER (1), DC_FP_NO_AUTOCRYPT_HEADER (2) or 0
pub const DC_PARAM_FORCE_PLAINTEXT: char = 'u';
/// for msgs: an incoming message which requestes a MDN (aka read receipt)
pub const DC_PARAM_WANTS_MDN: char = 'r';
/// for msgs
pub const DC_PARAM_FORWARDED: char = 'a';
/// for msgs
pub const DC_PARAM_CMD: char = 'S';
/// for msgs
pub const DC_PARAM_CMD_ARG: char = 'E';
/// for msgs
pub const DC_PARAM_CMD_ARG2: char = 'F';
/// for msgs
pub const DC_PARAM_CMD_ARG3: char = 'G';
/// for msgs
pub const DC_PARAM_CMD_ARG4: char = 'H';
/// for msgs
pub const DC_PARAM_ERROR: char = 'L';
/// for msgs in PREPARING: space-separated list of message IDs of forwarded copies
pub const DC_PARAM_PREP_FORWARDS: char = 'P';
/// for msgs
pub const DC_PARAM_SET_LATITUDE: char = 'l';
/// for msgs
pub const DC_PARAM_SET_LONGITUDE: char = 'n';

/// for jobs
pub const DC_PARAM_SERVER_FOLDER: char = 'Z';
/// for jobs
pub const DC_PARAM_SERVER_UID: char = 'z';
/// for jobs
pub const DC_PARAM_ALSO_MOVE: char = 'M';
/// for jobs: space-separated list of message recipients
pub const DC_PARAM_RECIPIENTS: char = 'R';
/// for groups
pub const DC_PARAM_UNPROMOTED: char = 'U';
/// for groups and contacts
pub const DC_PARAM_PROFILE_IMAGE: char = 'i';
/// for chats
pub const DC_PARAM_SELFTALK: char = 'K';

// values for DC_PARAM_FORCE_PLAINTEXT
pub const DC_FP_ADD_AUTOCRYPT_HEADER: u8 = 1;
pub const DC_FP_NO_AUTOCRYPT_HEADER: u8 = 2;

/// An object for handling key=value parameter lists; for the key, curently only
/// a single character is allowed.
///
/// The object is used eg. by dc_chat_t or dc_msg_t, for readable paramter names,
/// these classes define some DC_PARAM_* constantats.
///
/// Only for library-internal use.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_param_t {
    pub packed: *mut libc::c_char,
}

// values for DC_PARAM_FORCE_PLAINTEXT
/* user functions */
pub unsafe fn dc_param_exists(param: *mut dc_param_t, key: libc::c_int) -> libc::c_int {
    let mut p2: *mut libc::c_char = 0 as *mut libc::c_char;
    if param.is_null() || key == 0i32 {
        return 0i32;
    }
    return if !find_param((*param).packed, key, &mut p2).is_null() {
        1i32
    } else {
        0i32
    };
}

unsafe extern "C" fn find_param(
    haystack: *mut libc::c_char,
    key: libc::c_int,
    ret_p2: *mut *mut libc::c_char,
) -> *mut libc::c_char {
    let mut p1: *mut libc::c_char;
    let mut p2: *mut libc::c_char;
    p1 = haystack;
    loop {
        if p1.is_null() || *p1 as libc::c_int == 0i32 {
            return 0 as *mut libc::c_char;
        } else {
            if *p1 as libc::c_int == key && *p1.offset(1isize) as libc::c_int == '=' as i32 {
                break;
            }
            p1 = strchr(p1, '\n' as i32);
            if !p1.is_null() {
                p1 = p1.offset(1isize)
            }
        }
    }
    p2 = strchr(p1, '\n' as i32);
    if p2.is_null() {
        p2 = &mut *p1.offset(strlen(p1) as isize) as *mut libc::c_char
    }
    *ret_p2 = p2;

    p1
}

/* the value may be an empty string, "def" is returned only if the value unset.  The result must be free()'d in any case. */
pub unsafe fn dc_param_get(
    param: *const dc_param_t,
    key: libc::c_int,
    def: *const libc::c_char,
) -> *mut libc::c_char {
    let mut p1: *mut libc::c_char;
    let mut p2: *mut libc::c_char = 0 as *mut libc::c_char;
    let bak: libc::c_char;
    let ret: *mut libc::c_char;
    if param.is_null() || key == 0i32 {
        return if !def.is_null() {
            dc_strdup(def)
        } else {
            0 as *mut libc::c_char
        };
    }
    p1 = find_param((*param).packed, key, &mut p2);
    if p1.is_null() {
        return if !def.is_null() {
            dc_strdup(def)
        } else {
            0 as *mut libc::c_char
        };
    }
    p1 = p1.offset(2isize);
    bak = *p2;
    *p2 = 0i32 as libc::c_char;
    ret = dc_strdup(p1);
    dc_rtrim(ret);
    *p2 = bak;

    ret
}

pub unsafe fn dc_param_get_int(
    param: *const dc_param_t,
    key: libc::c_int,
    def: int32_t,
) -> int32_t {
    if param.is_null() || key == 0i32 {
        return def;
    }
    let str: *mut libc::c_char = dc_param_get(param, key, 0 as *const libc::c_char);
    if str.is_null() {
        return def;
    }
    let ret: int32_t = atol(str) as int32_t;
    free(str as *mut libc::c_void);

    ret
}

/**
 * Get value of a parameter.
 *
 * @memberof dc_param_t
 * @param param Parameter object to query.
 * @param key Key of the parameter to get, one of the DC_PARAM_* constants.
 * @param def Value to return if the parameter is not set.
 * @return The stored value or the default value.
 */
pub unsafe fn dc_param_get_float(
    param: *const dc_param_t,
    key: libc::c_int,
    def: libc::c_double,
) -> libc::c_double {
    if param.is_null() || key == 0 {
        return def;
    }

    let str = dc_param_get(param, key, std::ptr::null());
    if str.is_null() {
        return def;
    }

    let ret = dc_atof(str) as libc::c_double;
    free(str as *mut libc::c_void);

    ret
}

pub unsafe fn dc_param_set(
    mut param: *mut dc_param_t,
    key: libc::c_int,
    value: *const libc::c_char,
) {
    let mut old1: *mut libc::c_char;
    let mut old2: *mut libc::c_char;
    let new1: *mut libc::c_char;
    if param.is_null() || key == 0i32 {
        return;
    }
    old1 = (*param).packed;
    old2 = 0 as *mut libc::c_char;
    if !old1.is_null() {
        let p1: *mut libc::c_char;
        let mut p2: *mut libc::c_char = 0 as *mut libc::c_char;
        p1 = find_param(old1, key, &mut p2);
        if !p1.is_null() {
            *p1 = 0i32 as libc::c_char;
            old2 = p2
        } else if value.is_null() {
            return;
        }
    }
    dc_rtrim(old1);
    dc_ltrim(old2);
    if !old1.is_null() && *old1.offset(0isize) as libc::c_int == 0i32 {
        old1 = 0 as *mut libc::c_char
    }
    if !old2.is_null() && *old2.offset(0isize) as libc::c_int == 0i32 {
        old2 = 0 as *mut libc::c_char
    }
    if !value.is_null() {
        new1 = dc_mprintf(
            b"%s%s%c=%s%s%s\x00" as *const u8 as *const libc::c_char,
            if !old1.is_null() {
                old1
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            if !old1.is_null() {
                b"\n\x00" as *const u8 as *const libc::c_char
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            key,
            value,
            if !old2.is_null() {
                b"\n\x00" as *const u8 as *const libc::c_char
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            if !old2.is_null() {
                old2
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
        )
    } else {
        new1 = dc_mprintf(
            b"%s%s%s\x00" as *const u8 as *const libc::c_char,
            if !old1.is_null() {
                old1
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            if !old1.is_null() && !old2.is_null() {
                b"\n\x00" as *const u8 as *const libc::c_char
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            if !old2.is_null() {
                old2
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
        )
    }
    free((*param).packed as *mut libc::c_void);
    (*param).packed = new1;
}

pub unsafe fn dc_param_set_int(param: *mut dc_param_t, key: libc::c_int, value: int32_t) {
    if param.is_null() || key == 0i32 {
        return;
    }
    let value_str: *mut libc::c_char = dc_mprintf(
        b"%i\x00" as *const u8 as *const libc::c_char,
        value as libc::c_int,
    );
    if value_str.is_null() {
        return;
    }
    dc_param_set(param, key, value_str);
    free(value_str as *mut libc::c_void);
}

/* library-private */
pub unsafe fn dc_param_new() -> *mut dc_param_t {
    let mut param: *mut dc_param_t;
    param = calloc(1, ::std::mem::size_of::<dc_param_t>()) as *mut dc_param_t;
    if param.is_null() {
        exit(28i32);
    }
    (*param).packed = calloc(1, 1) as *mut libc::c_char;

    param
}

pub unsafe fn dc_param_empty(param: *mut dc_param_t) {
    if param.is_null() {
        return;
    }
    *(*param).packed.offset(0isize) = 0i32 as libc::c_char;
}

pub unsafe fn dc_param_unref(param: *mut dc_param_t) {
    if param.is_null() {
        return;
    }
    dc_param_empty(param);
    free((*param).packed as *mut libc::c_void);
    free(param as *mut libc::c_void);
}

pub unsafe fn dc_param_set_packed(mut param: *mut dc_param_t, packed: *const libc::c_char) {
    if param.is_null() {
        return;
    }
    dc_param_empty(param);
    if !packed.is_null() {
        free((*param).packed as *mut libc::c_void);
        (*param).packed = dc_strdup(packed)
    };
}

pub unsafe fn dc_param_set_urlencoded(mut param: *mut dc_param_t, urlencoded: *const libc::c_char) {
    if param.is_null() {
        return;
    }
    dc_param_empty(param);
    if !urlencoded.is_null() {
        free((*param).packed as *mut libc::c_void);
        (*param).packed = dc_strdup(urlencoded);
        dc_str_replace(
            &mut (*param).packed,
            b"&\x00" as *const u8 as *const libc::c_char,
            b"\n\x00" as *const u8 as *const libc::c_char,
        );
    };
}

/**
 * Set parameter to a float.
 *
 * @memberof dc_param_t
 * @param param Parameter object to modify.
 * @param key Key of the parameter to modify, one of the DC_PARAM_* constants.
 * @param value Value to store for key.
 * @return None.
 */
pub unsafe fn dc_param_set_float(param: *mut dc_param_t, key: libc::c_int, value: libc::c_double) {
    if param.is_null() || key == 0 {
        return;
    }

    let value_str = dc_ftoa(value);
    if value_str.is_null() {
        return;
    }
    dc_param_set(param, key, value_str);
    free(value_str as *mut libc::c_void);
}

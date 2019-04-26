use libc;

use crate::dc_contact::*;
use crate::dc_key::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/* *
 * @class dc_aheader_t
 * Library-internal. Parse and create [Autocrypt-headers](https://autocrypt.org/en/latest/level1.html#the-autocrypt-header).
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_aheader_t {
    pub addr: *mut libc::c_char,
    pub public_key: *mut dc_key_t,
    pub prefer_encrypt: libc::c_int,
}

/* the returned pointer is ref'd and must be unref'd after usage */
#[no_mangle]
pub unsafe extern "C" fn dc_aheader_new() -> *mut dc_aheader_t {
    let mut aheader: *mut dc_aheader_t = 0 as *mut dc_aheader_t;
    aheader = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_aheader_t>() as libc::c_ulong,
    ) as *mut dc_aheader_t;
    if aheader.is_null() {
        exit(37i32);
    }
    (*aheader).public_key = dc_key_new();
    return aheader;
}
#[no_mangle]
pub unsafe extern "C" fn dc_aheader_new_from_imffields(
    mut wanted_from: *const libc::c_char,
    mut header: *const mailimf_fields,
) -> *mut dc_aheader_t {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut fine_header: *mut dc_aheader_t = 0 as *mut dc_aheader_t;
    if wanted_from.is_null() || header.is_null() {
        return 0 as *mut dc_aheader_t;
    }
    cur = (*(*header).fld_list).first;
    while !cur.is_null() {
        let mut field: *mut mailimf_field = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            let mut optional_field: *mut mailimf_optional_field =
                (*field).fld_data.fld_optional_field;
            if !optional_field.is_null()
                && !(*optional_field).fld_name.is_null()
                && strcasecmp(
                    (*optional_field).fld_name,
                    b"Autocrypt\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                let mut test: *mut dc_aheader_t = dc_aheader_new();
                if 0 == dc_aheader_set_from_string(test, (*optional_field).fld_value)
                    || dc_addr_cmp((*test).addr, wanted_from) != 0i32
                {
                    dc_aheader_unref(test);
                    test = 0 as *mut dc_aheader_t
                }
                if fine_header.is_null() {
                    fine_header = test
                } else if !test.is_null() {
                    dc_aheader_unref(fine_header);
                    dc_aheader_unref(test);
                    return 0 as *mut dc_aheader_t;
                }
            }
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return fine_header;
}
#[no_mangle]
pub unsafe extern "C" fn dc_aheader_unref(mut aheader: *mut dc_aheader_t) {
    if aheader.is_null() {
        return;
    }
    free((*aheader).addr as *mut libc::c_void);
    dc_key_unref((*aheader).public_key);
    free(aheader as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_aheader_set_from_string(
    mut aheader: *mut dc_aheader_t,
    mut header_str__: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    /* according to RFC 5322 (Internet Message Format), the given string may contain `\r\n` before any whitespace.
    we can ignore this issue as
    (a) no key or value is expected to contain spaces,
    (b) for the key, non-base64-characters are ignored and
    (c) for parsing, we ignore `\r\n` as well as tabs for spaces */
    let mut header_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut beg_attr_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut after_attr_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut beg_attr_value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut success: libc::c_int = 0i32;
    dc_aheader_empty(aheader);
    if !(aheader.is_null() || header_str__.is_null()) {
        (*aheader).prefer_encrypt = 0i32;
        header_str = dc_strdup(header_str__);
        p = header_str;
        loop {
            if !(0 != *p) {
                current_block = 5689316957504528238;
                break;
            }
            p = p.offset(strspn(p, b"\t\r\n =;\x00" as *const u8 as *const libc::c_char) as isize);
            beg_attr_name = p;
            beg_attr_value = 0 as *mut libc::c_char;
            p = p.offset(strcspn(p, b"\t\r\n =;\x00" as *const u8 as *const libc::c_char) as isize);
            if !(p != beg_attr_name) {
                continue;
            }
            after_attr_name = p;
            p = p.offset(strspn(p, b"\t\r\n \x00" as *const u8 as *const libc::c_char) as isize);
            if *p as libc::c_int == '=' as i32 {
                p = p.offset(
                    strspn(p, b"\t\r\n =\x00" as *const u8 as *const libc::c_char) as isize,
                );
                beg_attr_value = p;
                p = p.offset(strcspn(p, b";\x00" as *const u8 as *const libc::c_char) as isize);
                if *p as libc::c_int != '\u{0}' as i32 {
                    *p = '\u{0}' as i32 as libc::c_char;
                    p = p.offset(1isize)
                }
                dc_trim(beg_attr_value);
            } else {
                p = p
                    .offset(strspn(p, b"\t\r\n ;\x00" as *const u8 as *const libc::c_char) as isize)
            }
            *after_attr_name = '\u{0}' as i32 as libc::c_char;
            if !(0 == add_attribute(aheader, beg_attr_name, beg_attr_value)) {
                continue;
            }
            /* a bad attribute makes the whole header invalid */
            current_block = 9271062167157603455;
            break;
        }
        match current_block {
            9271062167157603455 => {}
            _ => {
                if !(*aheader).addr.is_null() && !(*(*aheader).public_key).binary.is_null() {
                    success = 1i32
                }
            }
        }
    }
    free(header_str as *mut libc::c_void);
    if 0 == success {
        dc_aheader_empty(aheader);
    }
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_aheader_empty(mut aheader: *mut dc_aheader_t) {
    if aheader.is_null() {
        return;
    }
    (*aheader).prefer_encrypt = 0i32;
    free((*aheader).addr as *mut libc::c_void);
    (*aheader).addr = 0 as *mut libc::c_char;
    if !(*(*aheader).public_key).binary.is_null() {
        dc_key_unref((*aheader).public_key);
        (*aheader).public_key = dc_key_new()
    };
}
/* ******************************************************************************
 * Parse Autocrypt Header
 ******************************************************************************/
unsafe extern "C" fn add_attribute(
    mut aheader: *mut dc_aheader_t,
    mut name: *const libc::c_char,
    mut value: *const libc::c_char,
) -> libc::c_int {
    if strcasecmp(name, b"addr\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if value.is_null() || 0 == dc_may_be_valid_addr(value) || !(*aheader).addr.is_null() {
            return 0i32;
        }
        (*aheader).addr = dc_addr_normalize(value);
        return 1i32;
    } else {
        if strcasecmp(
            name,
            b"prefer-encrypt\x00" as *const u8 as *const libc::c_char,
        ) == 0i32
        {
            if !value.is_null()
                && strcasecmp(value, b"mutual\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                (*aheader).prefer_encrypt = 1i32;
                return 1i32;
            }
            return 1i32;
        } else {
            if strcasecmp(name, b"keydata\x00" as *const u8 as *const libc::c_char) == 0i32 {
                if value.is_null()
                    || !(*(*aheader).public_key).binary.is_null()
                    || 0 != (*(*aheader).public_key).bytes
                {
                    return 0i32;
                }
                return dc_key_set_from_base64((*aheader).public_key, value, 0i32);
            } else {
                if *name.offset(0isize) as libc::c_int == '_' as i32 {
                    return 1i32;
                }
            }
        }
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_aheader_render(mut aheader: *const dc_aheader_t) -> *mut libc::c_char {
    let mut success: libc::c_int = 0i32;
    let mut keybase64_wrapped: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    if !(aheader.is_null()
        || (*aheader).addr.is_null()
        || (*(*aheader).public_key).binary.is_null()
        || (*(*aheader).public_key).type_0 != 0i32)
    {
        dc_strbuilder_cat(&mut ret, b"addr=\x00" as *const u8 as *const libc::c_char);
        dc_strbuilder_cat(&mut ret, (*aheader).addr);
        dc_strbuilder_cat(&mut ret, b"; \x00" as *const u8 as *const libc::c_char);
        if (*aheader).prefer_encrypt == 1i32 {
            dc_strbuilder_cat(
                &mut ret,
                b"prefer-encrypt=mutual; \x00" as *const u8 as *const libc::c_char,
            );
        }
        dc_strbuilder_cat(
            &mut ret,
            b"keydata= \x00" as *const u8 as *const libc::c_char,
        );
        /* adds a whitespace every 78 characters, this allows libEtPan to wrap the lines according to RFC 5322
        (which may insert a linebreak before every whitespace) */
        keybase64_wrapped = dc_key_render_base64(
            (*aheader).public_key,
            78i32,
            b" \x00" as *const u8 as *const libc::c_char,
            0i32,
        );
        if !keybase64_wrapped.is_null() {
            /*no checksum*/
            dc_strbuilder_cat(&mut ret, keybase64_wrapped);
            success = 1i32
        }
    }
    if 0 == success {
        free(ret.buf as *mut libc::c_void);
        ret.buf = 0 as *mut libc::c_char
    }
    free(keybase64_wrapped as *mut libc::c_void);
    return ret.buf;
}

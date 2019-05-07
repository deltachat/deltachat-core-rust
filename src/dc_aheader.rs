use mmime::mailimf_types::*;

use crate::dc_contact::*;
use crate::dc_key::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/// Parse and create [Autocrypt-headers](https://autocrypt.org/en/latest/level1.html#the-autocrypt-header).
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_aheader_t {
    pub addr: *mut libc::c_char,
    pub public_key: *mut dc_key_t,
    pub prefer_encrypt: libc::c_int,
}

/// the returned pointer is ref'd and must be unref'd after usage
pub unsafe fn dc_aheader_new() -> *mut dc_aheader_t {
    let mut aheader = calloc(1, ::std::mem::size_of::<dc_aheader_t>()) as *mut dc_aheader_t;

    if aheader.is_null() {
        // TODO replace with enum (hardcoded in deltachat-core) /rtn
        exit(37);
    }

    (*aheader).public_key = dc_key_new();

    aheader
}

pub unsafe fn dc_aheader_new_from_imffields(
    wanted_from: *const libc::c_char,
    header: *const mailimf_fields,
) -> *mut dc_aheader_t {
    let mut cur;
    let mut fine_header = 0 as *mut dc_aheader_t;

    if wanted_from.is_null() || header.is_null() {
        return 0 as *mut dc_aheader_t;
    }

    cur = (*(*header).fld_list).first;
    while !cur.is_null() {
        let field = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;

        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            let optional_field = (*field).fld_data.fld_optional_field;
            if !optional_field.is_null()
                && !(*optional_field).fld_name.is_null()
                && strcasecmp(
                    (*optional_field).fld_name,
                    b"Autocrypt\x00" as *const u8 as *const libc::c_char,
                ) == 0
            {
                let mut test = dc_aheader_new();
                if 0 == dc_aheader_set_from_string(test, (*optional_field).fld_value)
                    || dc_addr_cmp((*test).addr, wanted_from) != 0
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
            0 as *mut clistcell
        }
    }

    fine_header
}

pub unsafe fn dc_aheader_unref(aheader: *mut dc_aheader_t) {
    if aheader.is_null() {
        return;
    }
    free((*aheader).addr as *mut libc::c_void);
    dc_key_unref((*aheader).public_key);
    free(aheader as *mut libc::c_void);
}

pub unsafe fn dc_aheader_set_from_string(
    mut aheader: *mut dc_aheader_t,
    header_str__: *const libc::c_char,
) -> libc::c_int {
    let current_block: u64;
    /* according to RFC 5322 (Internet Message Format), the given string may contain `\r\n` before any whitespace.
    we can ignore this issue as
    (a) no key or value is expected to contain spaces,
    (b) for the key, non-base64-characters are ignored and
    (c) for parsing, we ignore `\r\n` as well as tabs for spaces */
    let mut header_str = 0 as *mut libc::c_char;
    let mut p;
    let mut beg_attr_name;
    let mut after_attr_name;
    let mut beg_attr_value;
    let mut success: libc::c_int = 0;

    dc_aheader_empty(aheader);
    if !(aheader.is_null() || header_str__.is_null()) {
        (*aheader).prefer_encrypt = 0;
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
                    success = 1
                }
            }
        }
    }
    free(header_str as *mut libc::c_void);
    if 0 == success {
        dc_aheader_empty(aheader);
    }

    success
}

pub unsafe fn dc_aheader_empty(mut aheader: *mut dc_aheader_t) {
    if aheader.is_null() {
        return;
    }
    (*aheader).prefer_encrypt = 0;
    free((*aheader).addr as *mut libc::c_void);
    (*aheader).addr = 0 as *mut libc::c_char;

    if !(*(*aheader).public_key).binary.is_null() {
        dc_key_unref((*aheader).public_key);
        (*aheader).public_key = dc_key_new()
    }
}

/* ******************************************************************************
 * Parse Autocrypt Header
 ******************************************************************************/

unsafe fn add_attribute(
    mut aheader: *mut dc_aheader_t,
    name: *const libc::c_char,
    value: *const libc::c_char,
) -> libc::c_int {
    if strcasecmp(name, b"addr\x00" as *const u8 as *const libc::c_char) == 0 {
        if value.is_null() || 0 == dc_may_be_valid_addr(value) || !(*aheader).addr.is_null() {
            return 0;
        }
        (*aheader).addr = dc_addr_normalize(value);
        return 1;
    } else {
        if strcasecmp(
            name,
            b"prefer-encrypt\x00" as *const u8 as *const libc::c_char,
        ) == 0
        {
            if !value.is_null()
                && strcasecmp(value, b"mutual\x00" as *const u8 as *const libc::c_char) == 0
            {
                (*aheader).prefer_encrypt = 1;
                return 1;
            }
            return 1;
        } else {
            if strcasecmp(name, b"keydata\x00" as *const u8 as *const libc::c_char) == 0 {
                if value.is_null()
                    || !(*(*aheader).public_key).binary.is_null()
                    || 0 != (*(*aheader).public_key).bytes
                {
                    return 0;
                }
                return dc_key_set_from_base64((*aheader).public_key, value, 0);
            } else {
                if *name.offset(0isize) as libc::c_int == '_' as i32 {
                    return 1;
                }
            }
        }
    }

    0
}

pub unsafe fn dc_aheader_render(aheader: *const dc_aheader_t) -> *mut libc::c_char {
    let mut success: bool = false;
    let mut keybase64_wrapped: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0);
    if !(aheader.is_null()
        || (*aheader).addr.is_null()
        || (*(*aheader).public_key).binary.is_null()
        || (*(*aheader).public_key).type_0 != 0)
    {
        dc_strbuilder_cat(&mut ret, b"addr=\x00" as *const u8 as *const libc::c_char);
        dc_strbuilder_cat(&mut ret, (*aheader).addr);
        dc_strbuilder_cat(&mut ret, b"; \x00" as *const u8 as *const libc::c_char);
        if (*aheader).prefer_encrypt == 1 {
            dc_strbuilder_cat(
                &mut ret,
                b"prefer-encrypt=mutual; \x00" as *const u8 as *const libc::c_char,
            );
        }
        dc_strbuilder_cat(
            &mut ret,
            b"keydata= \x00" as *const u8 as *const libc::c_char,
        );
        // TODO replace 78 with enum /rtn
        /* adds a whitespace every 78 characters, this allows libEtPan to wrap the lines according to RFC 5322
        (which may insert a linebreak before every whitespace) */
        keybase64_wrapped = dc_key_render_base64((*aheader).public_key, 78);

        if !keybase64_wrapped.is_null() {
            /*no checksum*/
            dc_strbuilder_cat(&mut ret, keybase64_wrapped);
            success = true;
        }
    }

    if !success {
        free(ret.buf as *mut libc::c_void);
        ret.buf = 0 as *mut libc::c_char
    }

    free(keybase64_wrapped as *mut libc::c_void);

    ret.buf
}

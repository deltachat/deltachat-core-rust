use libc;
extern "C" {
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn strcspn(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strspn(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    // handle contacts
    #[no_mangle]
    fn dc_may_be_valid_addr(addr: *const libc::c_char) -> libc::c_int;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_key_new() -> *mut dc_key_t;
    #[no_mangle]
    fn dc_key_unref(_: *mut dc_key_t);
    #[no_mangle]
    fn dc_key_set_from_base64(
        _: *mut dc_key_t,
        base64: *const libc::c_char,
        type_0: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_key_render_base64(
        _: *const dc_key_t,
        break_every: libc::c_int,
        break_chars: *const libc::c_char,
        add_checksum: libc::c_int,
    ) -> *mut libc::c_char;
    // Working with e-mail-addresses
    #[no_mangle]
    fn dc_addr_cmp(addr1: *const libc::c_char, addr2: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_addr_normalize(addr: *const libc::c_char) -> *mut libc::c_char;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct clistcell_s {
    pub data: *mut libc::c_void,
    pub previous: *mut clistcell_s,
    pub next: *mut clistcell_s,
}
pub type clistcell = clistcell_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct clist_s {
    pub first: *mut clistcell,
    pub last: *mut clistcell,
    pub count: libc::c_int,
}
pub type clist = clist_s;
pub type clistiter = clistcell;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_date_time {
    pub dt_day: libc::c_int,
    pub dt_month: libc::c_int,
    pub dt_year: libc::c_int,
    pub dt_hour: libc::c_int,
    pub dt_min: libc::c_int,
    pub dt_sec: libc::c_int,
    pub dt_zone: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_mailbox_list {
    pub mb_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_mailbox {
    pub mb_display_name: *mut libc::c_char,
    pub mb_addr_spec: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_address_list {
    pub ad_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_fields {
    pub fld_list: *mut clist,
}
pub type unnamed = libc::c_uint;
pub const MAILIMF_FIELD_OPTIONAL_FIELD: unnamed = 22;
pub const MAILIMF_FIELD_KEYWORDS: unnamed = 21;
pub const MAILIMF_FIELD_COMMENTS: unnamed = 20;
pub const MAILIMF_FIELD_SUBJECT: unnamed = 19;
pub const MAILIMF_FIELD_REFERENCES: unnamed = 18;
pub const MAILIMF_FIELD_IN_REPLY_TO: unnamed = 17;
pub const MAILIMF_FIELD_MESSAGE_ID: unnamed = 16;
pub const MAILIMF_FIELD_BCC: unnamed = 15;
pub const MAILIMF_FIELD_CC: unnamed = 14;
pub const MAILIMF_FIELD_TO: unnamed = 13;
pub const MAILIMF_FIELD_REPLY_TO: unnamed = 12;
pub const MAILIMF_FIELD_SENDER: unnamed = 11;
pub const MAILIMF_FIELD_FROM: unnamed = 10;
pub const MAILIMF_FIELD_ORIG_DATE: unnamed = 9;
pub const MAILIMF_FIELD_RESENT_MSG_ID: unnamed = 8;
pub const MAILIMF_FIELD_RESENT_BCC: unnamed = 7;
pub const MAILIMF_FIELD_RESENT_CC: unnamed = 6;
pub const MAILIMF_FIELD_RESENT_TO: unnamed = 5;
pub const MAILIMF_FIELD_RESENT_SENDER: unnamed = 4;
pub const MAILIMF_FIELD_RESENT_FROM: unnamed = 3;
pub const MAILIMF_FIELD_RESENT_DATE: unnamed = 2;
pub const MAILIMF_FIELD_RETURN_PATH: unnamed = 1;
pub const MAILIMF_FIELD_NONE: unnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_0 {
    pub fld_return_path: *mut mailimf_return,
    pub fld_resent_date: *mut mailimf_orig_date,
    pub fld_resent_from: *mut mailimf_from,
    pub fld_resent_sender: *mut mailimf_sender,
    pub fld_resent_to: *mut mailimf_to,
    pub fld_resent_cc: *mut mailimf_cc,
    pub fld_resent_bcc: *mut mailimf_bcc,
    pub fld_resent_msg_id: *mut mailimf_message_id,
    pub fld_orig_date: *mut mailimf_orig_date,
    pub fld_from: *mut mailimf_from,
    pub fld_sender: *mut mailimf_sender,
    pub fld_reply_to: *mut mailimf_reply_to,
    pub fld_to: *mut mailimf_to,
    pub fld_cc: *mut mailimf_cc,
    pub fld_bcc: *mut mailimf_bcc,
    pub fld_message_id: *mut mailimf_message_id,
    pub fld_in_reply_to: *mut mailimf_in_reply_to,
    pub fld_references: *mut mailimf_references,
    pub fld_subject: *mut mailimf_subject,
    pub fld_comments: *mut mailimf_comments,
    pub fld_keywords: *mut mailimf_keywords,
    pub fld_optional_field: *mut mailimf_optional_field,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_optional_field {
    pub fld_name: *mut libc::c_char,
    pub fld_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_keywords {
    pub kw_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_comments {
    pub cm_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_subject {
    pub sbj_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_references {
    pub mid_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_in_reply_to {
    pub mid_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_message_id {
    pub mid_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_bcc {
    pub bcc_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_cc {
    pub cc_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_to {
    pub to_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_reply_to {
    pub rt_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_sender {
    pub snd_mb: *mut mailimf_mailbox,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_from {
    pub frm_mb_list: *mut mailimf_mailbox_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_orig_date {
    pub dt_date_time: *mut mailimf_date_time,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_return {
    pub ret_path: *mut mailimf_path,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_path {
    pub pt_addr_spec: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type dc_strbuilder_t = _dc_strbuilder;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_key {
    pub binary: *mut libc::c_void,
    pub bytes: libc::c_int,
    pub type_0: libc::c_int,
    pub _m_heap_refcnt: libc::c_int,
}
pub type dc_key_t = _dc_key;
/* *
 * @class dc_aheader_t
 * Library-internal. Parse and create [Autocrypt-headers](https://autocrypt.org/en/latest/level1.html#the-autocrypt-header).
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_aheader {
    pub addr: *mut libc::c_char,
    pub public_key: *mut dc_key_t,
    pub prefer_encrypt: libc::c_int,
}
pub type dc_aheader_t = _dc_aheader;
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
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
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

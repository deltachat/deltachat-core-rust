use percent_encoding::percent_decode_str;

use crate::context::Context;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_lot::*;
use crate::dc_param::*;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::key::*;
use crate::peerstate::*;
use crate::types::*;
use crate::x::*;

// out-of-band verification
// id=contact
// text1=groupname
// id=contact
// id=contact
// test1=formatted fingerprint
// id=contact
// text1=text
// text1=URL
// text1=error string
pub unsafe fn dc_check_qr(context: &Context, qr: *const libc::c_char) -> *mut dc_lot_t {
    let mut current_block: u64;
    let mut payload: *mut libc::c_char = 0 as *mut libc::c_char;
    // must be normalized, if set
    let mut addr: *mut libc::c_char = 0 as *mut libc::c_char;
    // must be normalized, if set
    let mut fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut invitenumber: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut auth: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut qr_parsed: *mut dc_lot_t = dc_lot_new();
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut device_msg: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpname: *mut libc::c_char = 0 as *mut libc::c_char;
    (*qr_parsed).state = 0i32;
    if !qr.is_null() {
        info!(context, 0, "Scanned QR code: {}", as_str(qr),);
        /* split parameters from the qr code
        ------------------------------------ */
        if strncasecmp(
            qr,
            b"OPENPGP4FPR:\x00" as *const u8 as *const libc::c_char,
            strlen(b"OPENPGP4FPR:\x00" as *const u8 as *const libc::c_char),
        ) == 0i32
        {
            payload =
                dc_strdup(&*qr.offset(strlen(
                    b"OPENPGP4FPR:\x00" as *const u8 as *const libc::c_char,
                ) as isize));
            let mut fragment: *mut libc::c_char = strchr(payload, '#' as i32);
            if !fragment.is_null() {
                *fragment = 0i32 as libc::c_char;
                fragment = fragment.offset(1isize);
                let mut param = dc_param_new();
                dc_param_set_urlencoded(&mut param, as_str(fragment)).expect("invalid params");
                addr = dc_param_get(&param, Param::Forwarded)
                    .map(|s| to_cstring(s))
                    .unwrap_or_else(|| std::ptr::null_mut());
                if !addr.is_null() {
                    if let Some(ref name_enc) = dc_param_get(&param, Param::SetLongitude) {
                        let name_r = percent_decode_str(name_enc)
                            .decode_utf8()
                            .expect("invalid name");
                        name = to_cstring(name_r);
                        dc_normalize_name(name);
                    }
                    invitenumber = dc_param_get(&param, Param::ProfileImage)
                        .map(|s| to_cstring(s))
                        .unwrap_or_else(|| std::ptr::null_mut());
                    auth = dc_param_get(&param, Param::Auth)
                        .map(|s| to_cstring(s))
                        .unwrap_or_else(|| std::ptr::null_mut());
                    grpid = dc_param_get(&param, Param::GroupId)
                        .map(|s| to_cstring(s))
                        .unwrap_or_else(|| std::ptr::null_mut());
                    if !grpid.is_null() {
                        if let Some(grpname_enc) = dc_param_get(&param, Param::GroupName) {
                            let grpname_r = percent_decode_str(grpname_enc)
                                .decode_utf8()
                                .expect("invalid groupname");
                            grpname = to_cstring(grpname_r);
                        }
                    }
                }
            }
            fingerprint = dc_normalize_fingerprint_c(payload);
            current_block = 5023038348526654800;
        } else if strncasecmp(
            qr,
            b"mailto:\x00" as *const u8 as *const libc::c_char,
            strlen(b"mailto:\x00" as *const u8 as *const libc::c_char),
        ) == 0i32
        {
            payload = dc_strdup(
                &*qr.offset(strlen(b"mailto:\x00" as *const u8 as *const libc::c_char) as isize),
            );
            let query: *mut libc::c_char = strchr(payload, '?' as i32);
            if !query.is_null() {
                *query = 0i32 as libc::c_char
            }
            addr = dc_strdup(payload);
            current_block = 5023038348526654800;
        } else if strncasecmp(
            qr,
            b"SMTP:\x00" as *const u8 as *const libc::c_char,
            strlen(b"SMTP:\x00" as *const u8 as *const libc::c_char),
        ) == 0i32
        {
            payload = dc_strdup(
                &*qr.offset(strlen(b"SMTP:\x00" as *const u8 as *const libc::c_char) as isize),
            );
            let colon: *mut libc::c_char = strchr(payload, ':' as i32);
            if !colon.is_null() {
                *colon = 0i32 as libc::c_char
            }
            addr = dc_strdup(payload);
            current_block = 5023038348526654800;
        } else if strncasecmp(
            qr,
            b"MATMSG:\x00" as *const u8 as *const libc::c_char,
            strlen(b"MATMSG:\x00" as *const u8 as *const libc::c_char),
        ) == 0i32
        {
            /* scheme: `MATMSG:TO:addr...;SUB:subject...;BODY:body...;` - there may or may not be linebreaks after the fields */
            /* does not work when the text `TO:` is used in subject/body _and_ TO: is not the first field. we ignore this case. */
            let to: *mut libc::c_char = strstr(qr, b"TO:\x00" as *const u8 as *const libc::c_char);
            if !to.is_null() {
                addr = dc_strdup(&mut *to.offset(3isize));
                let semicolon: *mut libc::c_char = strchr(addr, ';' as i32);
                if !semicolon.is_null() {
                    *semicolon = 0i32 as libc::c_char
                }
                current_block = 5023038348526654800;
            } else {
                (*qr_parsed).state = 400i32;
                (*qr_parsed).text1 =
                    dc_strdup(b"Bad e-mail address.\x00" as *const u8 as *const libc::c_char);
                current_block = 16562876845594826114;
            }
        } else {
            if strncasecmp(
                qr,
                b"BEGIN:VCARD\x00" as *const u8 as *const libc::c_char,
                strlen(b"BEGIN:VCARD\x00" as *const u8 as *const libc::c_char),
            ) == 0i32
            {
                let lines: *mut carray = dc_split_into_lines(qr);
                let mut i: libc::c_int = 0i32;
                while (i as libc::c_uint) < carray_count(lines) {
                    let key: *mut libc::c_char =
                        carray_get(lines, i as libc::c_uint) as *mut libc::c_char;
                    dc_trim(key);
                    let mut value: *mut libc::c_char = strchr(key, ':' as i32);
                    if !value.is_null() {
                        *value = 0i32 as libc::c_char;
                        value = value.offset(1isize);
                        let mut semicolon_0: *mut libc::c_char = strchr(key, ';' as i32);
                        if !semicolon_0.is_null() {
                            *semicolon_0 = 0i32 as libc::c_char
                        }
                        if strcasecmp(key, b"EMAIL\x00" as *const u8 as *const libc::c_char) == 0i32
                        {
                            semicolon_0 = strchr(value, ';' as i32);
                            if !semicolon_0.is_null() {
                                *semicolon_0 = 0i32 as libc::c_char
                            }
                            addr = dc_strdup(value)
                        } else if strcasecmp(key, b"N\x00" as *const u8 as *const libc::c_char)
                            == 0i32
                        {
                            semicolon_0 = strchr(value, ';' as i32);
                            if !semicolon_0.is_null() {
                                semicolon_0 = strchr(semicolon_0.offset(1isize), ';' as i32);
                                if !semicolon_0.is_null() {
                                    *semicolon_0 = 0i32 as libc::c_char
                                }
                            }
                            name = dc_strdup(value);
                            dc_str_replace(
                                &mut name,
                                b";\x00" as *const u8 as *const libc::c_char,
                                b",\x00" as *const u8 as *const libc::c_char,
                            );
                            dc_normalize_name(name);
                        }
                    }
                    i += 1
                }
                dc_free_splitted_lines(lines);
            }
            current_block = 5023038348526654800;
        }
        match current_block {
            16562876845594826114 => {}
            _ => {
                /* check the parameters
                ---------------------- */
                if !addr.is_null() {
                    /* urldecoding is needed at least for OPENPGP4FPR but should not hurt in the other cases */
                    let mut temp: *mut libc::c_char = dc_urldecode(addr);
                    free(addr as *mut libc::c_void);
                    addr = temp;
                    temp = dc_addr_normalize(addr);
                    free(addr as *mut libc::c_void);
                    addr = temp;
                    if !dc_may_be_valid_addr(addr) {
                        (*qr_parsed).state = 400i32;
                        (*qr_parsed).text1 = dc_strdup(
                            b"Bad e-mail address.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 16562876845594826114;
                    } else {
                        current_block = 14116432890150942211;
                    }
                } else {
                    current_block = 14116432890150942211;
                }
                match current_block {
                    16562876845594826114 => {}
                    _ => {
                        if !fingerprint.is_null() {
                            if strlen(fingerprint) != 40 {
                                (*qr_parsed).state = 400i32;
                                (*qr_parsed).text1 = dc_strdup(
                                    b"Bad fingerprint length in QR code.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                current_block = 16562876845594826114;
                            } else {
                                current_block = 5409161009579131794;
                            }
                        } else {
                            current_block = 5409161009579131794;
                        }
                        match current_block {
                            16562876845594826114 => {}
                            _ => {
                                if !fingerprint.is_null() {
                                    let peerstate = Peerstate::from_fingerprint(
                                        context,
                                        &context.sql,
                                        as_str(fingerprint),
                                    );
                                    if addr.is_null() || invitenumber.is_null() || auth.is_null() {
                                        if let Some(peerstate) = peerstate {
                                            (*qr_parsed).state = 210i32;
                                            let addr_ptr = if let Some(ref addr) = peerstate.addr {
                                                to_cstring(addr)
                                            } else {
                                                std::ptr::null()
                                            };
                                            (*qr_parsed).id = dc_add_or_lookup_contact(
                                                context,
                                                0 as *const libc::c_char,
                                                addr_ptr,
                                                0x80i32,
                                                0 as *mut libc::c_int,
                                            );
                                            free(addr_ptr as *mut _);
                                            dc_create_or_lookup_nchat_by_contact_id(
                                                context,
                                                (*qr_parsed).id,
                                                2i32,
                                                &mut chat_id,
                                                0 as *mut libc::c_int,
                                            );
                                            device_msg = dc_mprintf(
                                                b"%s verified.\x00" as *const u8
                                                    as *const libc::c_char,
                                                peerstate.addr,
                                            )
                                        } else {
                                            (*qr_parsed).text1 =
                                                dc_format_fingerprint_c(fingerprint);
                                            (*qr_parsed).state = 230i32
                                        }
                                    } else {
                                        if !grpid.is_null() && !grpname.is_null() {
                                            (*qr_parsed).state = 202i32;
                                            (*qr_parsed).text1 = dc_strdup(grpname);
                                            (*qr_parsed).text2 = dc_strdup(grpid)
                                        } else {
                                            (*qr_parsed).state = 200i32
                                        }
                                        (*qr_parsed).id = dc_add_or_lookup_contact(
                                            context,
                                            name,
                                            addr,
                                            0x80i32,
                                            0 as *mut libc::c_int,
                                        );
                                        (*qr_parsed).fingerprint = dc_strdup(fingerprint);
                                        (*qr_parsed).invitenumber = dc_strdup(invitenumber);
                                        (*qr_parsed).auth = dc_strdup(auth)
                                    }
                                } else if !addr.is_null() {
                                    (*qr_parsed).state = 320i32;
                                    (*qr_parsed).id = dc_add_or_lookup_contact(
                                        context,
                                        name,
                                        addr,
                                        0x80i32,
                                        0 as *mut libc::c_int,
                                    )
                                } else if strstr(
                                    qr,
                                    b"http://\x00" as *const u8 as *const libc::c_char,
                                ) == qr as *mut libc::c_char
                                    || strstr(
                                        qr,
                                        b"https://\x00" as *const u8 as *const libc::c_char,
                                    ) == qr as *mut libc::c_char
                                {
                                    (*qr_parsed).state = 332i32;
                                    (*qr_parsed).text1 = dc_strdup(qr)
                                } else {
                                    (*qr_parsed).state = 330i32;
                                    (*qr_parsed).text1 = dc_strdup(qr)
                                }
                                if !device_msg.is_null() {
                                    dc_add_device_msg(context, chat_id, device_msg);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    free(addr as *mut libc::c_void);
    free(fingerprint as *mut libc::c_void);
    free(payload as *mut libc::c_void);
    free(name as *mut libc::c_void);
    free(invitenumber as *mut libc::c_void);
    free(auth as *mut libc::c_void);
    free(device_msg as *mut libc::c_void);
    free(grpname as *mut libc::c_void);
    free(grpid as *mut libc::c_void);

    qr_parsed
}

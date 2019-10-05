use std::collections::HashSet;
use std::ffi::CString;
use std::ptr;

use crate::contact::addr_normalize;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::error::Error;
use mmime::clist::*;
// use mmime::display::*;
use mmime::mailimf::mailimf_msg_id_parse;
use mmime::mailimf::types::*;
use mmime::mailimf::types_helper::*;
use mmime::mailmime::content::*;
use mmime::mailmime::disposition::*;
use mmime::mailmime::types::*;
use mmime::mailmime::types_helper::*;
use mmime::mailmime::*;
use mmime::mmapstring::*;
use mmime::other::*;

#[macro_export]
macro_rules! clist_append {
    ($clist:expr,  $item:expr) => {
        if clist_insert_after(
            $clist as *mut clist,
            (*$clist).last,
            $item as *mut libc::c_void,
        ) != 0
        {
            bail!("could not allocate or append list item");
        }
    };
}

/**************************************
* mime parsing API
**************************************/

pub fn get_ct_subtype(mime: *mut Mailmime) -> Option<String> {
    unsafe {
        let ct: *mut mailmime_content = (*mime).mm_content_type;

        if !ct.is_null() && !(*ct).ct_subtype.is_null() {
            Some(to_string_lossy((*ct).ct_subtype))
        } else {
            None
        }
    }
}

pub fn parse_message_id(message_id: &str) -> Result<String, Error> {
    let mut dummy = 0;
    let c_message_id = CString::new(message_id).unwrap_or_default();
    let c_ptr = c_message_id.as_ptr();
    let mut rfc724_mid_c = std::ptr::null_mut();
    if unsafe { mailimf_msg_id_parse(c_ptr, libc::strlen(c_ptr), &mut dummy, &mut rfc724_mid_c) }
        == MAIL_NO_ERROR as libc::c_int
        && !rfc724_mid_c.is_null()
    {
        let res = to_string_lossy(rfc724_mid_c);
        unsafe { libc::free(rfc724_mid_c.cast()) };
        Ok(res)
    } else {
        bail!("could not parse message_id: {}", message_id);
    }
}

pub fn get_autocrypt_mime(
    mime_undetermined: *mut Mailmime,
) -> Result<(*mut Mailmime, *mut Mailmime), Error> {
    /* return Result with two mime pointers:

    First mime pointer is to the multipart-mime message
    (which is replaced with a decrypted version later)

    Second one is to the encrypted payload.
    For non-autocrypt message an Error is returned.
    */
    unsafe {
        ensure!(
            (*mime_undetermined).mm_type == MAILMIME_MESSAGE as libc::c_int,
            "Not a root mime message"
        );
        let mime = (*mime_undetermined).mm_data.mm_message.mm_msg_mime;

        ensure!(
            (*mime).mm_type == MAILMIME_MULTIPLE as libc::c_int
                && "encrypted" == get_ct_subtype(mime).unwrap_or_default(),
            "Not a multipart/encrypted message"
        );
        let parts: Vec<_> = (*(*mime).mm_data.mm_multipart.mm_mp_list)
            .into_iter()
            .map(|c| c as *mut Mailmime)
            .collect();
        ensure!(parts.len() == 2, "Invalid Autocrypt Level 1 Mime Parts");
        // XXX ensure protocol-parameter "application/pgp-encrypted")
        // XXX ensure wrapmime::get_content_type(parts[1])) == "application/octetstream"
        // a proper OpenPGP multipart/encrypted Autocrypt Level 1 message
        // https://tools.ietf.org/html/rfc3156.html#section-4
        Ok((mime, parts[1]))
    }
}

pub fn has_decryptable_data(mime_data: *mut mailmime_data) -> bool {
    /* MAILMIME_DATA_FILE indicates, the data is in a file; AFAIK this is not used on parsing */
    unsafe {
        (*mime_data).dt_type == MAILMIME_DATA_TEXT as libc::c_int
            && !(*mime_data).dt_data.dt_text.dt_data.is_null()
            && (*mime_data).dt_data.dt_text.dt_length > 0
    }
}

pub fn get_field_from(imffields: *mut mailimf_fields) -> Result<String, Error> {
    let field = mailimf_find_field(imffields, MAILIMF_FIELD_FROM as libc::c_int);
    if !field.is_null() && unsafe { !(*field).fld_data.fld_from.is_null() } {
        let mb_list = unsafe { (*(*field).fld_data.fld_from).frm_mb_list };
        if let Some(addr) = mailimf_find_first_addr(mb_list) {
            return Ok(addr);
        }
    }
    bail!("not From field found");
}

pub fn get_field_date(imffields: *mut mailimf_fields) -> Result<i64, Error> {
    let field = mailimf_find_field(imffields, MAILIMF_FIELD_ORIG_DATE as libc::c_int);
    let mut message_time = 0;

    if !field.is_null() && unsafe { !(*field).fld_data.fld_orig_date.is_null() } {
        let orig_date = unsafe { (*field).fld_data.fld_orig_date };

        if !orig_date.is_null() {
            let dt = unsafe { (*orig_date).dt_date_time };
            message_time = dc_timestamp_from_date(dt);
            if message_time != 0 && message_time > time() {
                message_time = time()
            }
        }
    }

    Ok(message_time)
}

fn mailimf_get_recipients_add_addr(recipients: &mut HashSet<String>, mb: *mut mailimf_mailbox) {
    if !mb.is_null() {
        let addr_norm = addr_normalize(as_str(unsafe { (*mb).mb_addr_spec }));
        recipients.insert(addr_norm.into());
    }
}

/*the result is a pointer to mime, must not be freed*/
pub fn mailimf_find_field(
    header: *mut mailimf_fields,
    wanted_fld_type: libc::c_int,
) -> *mut mailimf_field {
    if header.is_null() {
        return ptr::null_mut();
    }

    let header = unsafe { (*header) };
    if header.fld_list.is_null() {
        return ptr::null_mut();
    }

    for cur in unsafe { &(*header.fld_list) } {
        let field = cur as *mut mailimf_field;
        if !field.is_null() {
            if unsafe { (*field).fld_type } == wanted_fld_type {
                return field;
            }
        }
    }

    ptr::null_mut()
}

/*the result is a pointer to mime, must not be freed*/
pub unsafe fn mailmime_find_mailimf_fields(mime: *mut Mailmime) -> *mut mailimf_fields {
    if mime.is_null() {
        return ptr::null_mut();
    }

    match (*mime).mm_type as _ {
        MAILMIME_MULTIPLE => {
            for cur_data in (*(*mime).mm_data.mm_multipart.mm_mp_list).into_iter() {
                let header = mailmime_find_mailimf_fields(cur_data as *mut _);
                if !header.is_null() {
                    return header;
                }
            }
        }
        MAILMIME_MESSAGE => return (*mime).mm_data.mm_message.mm_fields,
        _ => {}
    }

    ptr::null_mut()
}

pub unsafe fn mailimf_find_optional_field(
    header: *mut mailimf_fields,
    wanted_fld_name: *const libc::c_char,
) -> *mut mailimf_optional_field {
    if header.is_null() || (*header).fld_list.is_null() {
        return ptr::null_mut();
    }
    for cur_data in (*(*header).fld_list).into_iter() {
        let field: *mut mailimf_field = cur_data as *mut _;

        if (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            let optional_field: *mut mailimf_optional_field = (*field).fld_data.fld_optional_field;
            if !optional_field.is_null()
                && !(*optional_field).fld_name.is_null()
                && !(*optional_field).fld_value.is_null()
                && strcasecmp((*optional_field).fld_name, wanted_fld_name) == 0i32
            {
                return optional_field;
            }
        }
    }

    ptr::null_mut()
}

pub fn mailimf_get_recipients(imffields: *mut mailimf_fields) -> HashSet<String> {
    /* returned addresses are normalized. */
    let mut recipients: HashSet<String> = Default::default();

    for cur in unsafe { (*(*imffields).fld_list).into_iter() } {
        let fld = cur as *mut mailimf_field;

        let fld_to: *mut mailimf_to;
        let fld_cc: *mut mailimf_cc;

        let mut addr_list: *mut mailimf_address_list = ptr::null_mut();
        if fld.is_null() {
            continue;
        }

        let fld = unsafe { *fld };

        // TODO match on enums /rtn
        match fld.fld_type {
            13 => {
                fld_to = unsafe { fld.fld_data.fld_to };
                if !fld_to.is_null() {
                    addr_list = unsafe { (*fld_to).to_addr_list };
                }
            }
            14 => {
                fld_cc = unsafe { fld.fld_data.fld_cc };
                if !fld_cc.is_null() {
                    addr_list = unsafe { (*fld_cc).cc_addr_list };
                }
            }
            _ => {}
        }

        if !addr_list.is_null() {
            for cur2 in unsafe { &(*(*addr_list).ad_list) } {
                let adr = cur2 as *mut mailimf_address;

                if adr.is_null() {
                    continue;
                }
                let adr = unsafe { *adr };

                if adr.ad_type == MAILIMF_ADDRESS_MAILBOX as libc::c_int {
                    mailimf_get_recipients_add_addr(&mut recipients, unsafe {
                        adr.ad_data.ad_mailbox
                    });
                } else if adr.ad_type == MAILIMF_ADDRESS_GROUP as libc::c_int {
                    let group = unsafe { adr.ad_data.ad_group };
                    if !group.is_null() && unsafe { !(*group).grp_mb_list.is_null() } {
                        for cur3 in unsafe { &(*(*(*group).grp_mb_list).mb_list) } {
                            mailimf_get_recipients_add_addr(
                                &mut recipients,
                                cur3 as *mut mailimf_mailbox,
                            );
                        }
                    }
                }
            }
        }
    }

    recipients
}

pub fn mailmime_transfer_decode(mime: *mut Mailmime) -> Result<Vec<u8>, Error> {
    ensure!(!mime.is_null(), "invalid inputs");

    let mime_transfer_encoding =
        get_mime_transfer_encoding(mime).unwrap_or(MAILMIME_MECHANISM_BINARY as i32);

    let mime_data = unsafe { (*mime).mm_data.mm_single };

    decode_dt_data(mime_data, mime_transfer_encoding)
}

pub fn get_mime_transfer_encoding(mime: *mut Mailmime) -> Option<libc::c_int> {
    unsafe {
        let mm_mime_fields = (*mime).mm_mime_fields;
        if !mm_mime_fields.is_null() {
            for cur_data in (*(*mm_mime_fields).fld_list).into_iter() {
                let field: *mut mailmime_field = cur_data as *mut _;
                if (*field).fld_type == MAILMIME_FIELD_TRANSFER_ENCODING as libc::c_int
                    && !(*field).fld_data.fld_encoding.is_null()
                {
                    return Some((*(*field).fld_data.fld_encoding).enc_type);
                }
            }
        }
    }
    None
}

pub fn decode_dt_data(
    mime_data: *mut mailmime_data,
    mime_transfer_encoding: libc::c_int,
) -> Result<Vec<u8>, Error> {
    // Decode data according to mime_transfer_encoding
    // returns Ok with a (decoded_data,decoded_data_bytes) pointer
    // where the caller must make sure to free it.
    // It may return Ok(ptr::null_mut(), 0)
    if mime_transfer_encoding == MAILMIME_MECHANISM_7BIT as libc::c_int
        || mime_transfer_encoding == MAILMIME_MECHANISM_8BIT as libc::c_int
        || mime_transfer_encoding == MAILMIME_MECHANISM_BINARY as libc::c_int
    {
        let decoded_data = unsafe { (*mime_data).dt_data.dt_text.dt_data };
        let decoded_data_bytes = unsafe { (*mime_data).dt_data.dt_text.dt_length };

        if decoded_data.is_null() || decoded_data_bytes == 0 {
            bail!("No data to decode found");
        } else {
            let result = unsafe {
                std::slice::from_raw_parts(decoded_data as *const u8, decoded_data_bytes)
            };
            return Ok(result.to_vec());
        }
    }
    // unsafe { display_mime_data(mime_data) };

    let mut current_index = 0;
    let mut transfer_decoding_buffer = ptr::null_mut();
    let mut decoded_data_bytes = 0;

    let r = unsafe {
        mailmime_part_parse(
            (*mime_data).dt_data.dt_text.dt_data,
            (*mime_data).dt_data.dt_text.dt_length,
            &mut current_index,
            mime_transfer_encoding,
            &mut transfer_decoding_buffer,
            &mut decoded_data_bytes,
        )
    };

    if r == MAILIMF_NO_ERROR as libc::c_int
        && !transfer_decoding_buffer.is_null()
        && decoded_data_bytes > 0
    {
        let result = unsafe {
            std::slice::from_raw_parts(transfer_decoding_buffer as *const u8, decoded_data_bytes)
        }
        .to_vec();
        // we return a fresh vec and transfer_decoding_buffer is not used or passed anywhere
        // so it's safe to free it right away, as mailman_part_parse has
        // allocated it fresh.
        unsafe { mmap_string_unref(transfer_decoding_buffer) };

        return Ok(result);
    }

    Err(format_err!("Failed to to decode"))
}

pub fn mailimf_find_first_addr(mb_list: *const mailimf_mailbox_list) -> Option<String> {
    if mb_list.is_null() {
        return None;
    }

    for cur in unsafe { (*(*mb_list).mb_list).into_iter() } {
        let mb = cur as *mut mailimf_mailbox;
        if !mb.is_null() && !unsafe { (*mb).mb_addr_spec.is_null() } {
            let addr = unsafe { as_str((*mb).mb_addr_spec) };
            return Some(addr_normalize(addr).to_string());
        }
    }

    None
}

/**************************************
* mime creation API
**************************************/

pub fn add_filename_part(
    message: *mut Mailmime,
    basename: &str,
    mime_type: &str,
    file_content: &str,
) -> Result<(), Error> {
    let mime_type_c = CString::new(mime_type.to_string()).expect("failed to create CString");
    unsafe {
        let content_type = mailmime_content_new_with_str(mime_type_c.as_ptr());
        let mime_fields = mailmime_fields_new_filename(
            MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
            basename.strdup(),
            MAILMIME_MECHANISM_8BIT as libc::c_int,
        );
        let file_mime_part = mailmime_new_empty(content_type, mime_fields);
        set_body_text(file_mime_part, file_content)?;
        mailmime_smart_add_part(message, file_mime_part);
    }
    Ok(())
}

pub fn new_custom_field(fields: *mut mailimf_fields, name: &str, value: &str) {
    unsafe {
        let field = mailimf_field_new_custom(name.strdup(), value.strdup());
        let res = mailimf_fields_add(fields, field);
        assert!(
            res as u32 == MAILIMF_NO_ERROR,
            "could not create mailimf field"
        );
    }
}

pub fn build_body_text(text: &str) -> Result<*mut Mailmime, Error> {
    let mime_fields: *mut mailmime_fields;
    let message_part: *mut Mailmime;

    let content = new_content_type("text/plain")?;
    append_ct_param(content, "charset", "utf-8")?;

    unsafe {
        mime_fields = mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
        message_part = mailmime_new_empty(content, mime_fields);
    }
    set_body_text(message_part, text)?;

    Ok(message_part)
}

pub fn append_ct_param(
    content: *mut mailmime_content,
    name: &str,
    value: &str,
) -> Result<(), Error> {
    unsafe {
        let name_c = CString::new(name).unwrap_or_default();
        let value_c = CString::new(value).unwrap_or_default();

        clist_append!(
            (*content).ct_parameters,
            mailmime_param_new_with_data(
                name_c.as_ptr() as *const u8 as *const libc::c_char as *mut libc::c_char,
                value_c.as_ptr() as *const u8 as *const libc::c_char as *mut libc::c_char
            )
        );
    }
    Ok(())
}

pub fn new_content_type(content_type: &str) -> Result<*mut mailmime_content, Error> {
    let ct = CString::new(content_type).unwrap_or_default();
    let content: *mut mailmime_content;
    // mailmime_content_new_with_str only parses but does not retain/own ct
    unsafe {
        content = mailmime_content_new_with_str(ct.as_ptr());
    }
    ensure!(!content.is_null(), "mailimf failed to allocate");
    Ok(content)
}

pub fn set_body_text(part: *mut Mailmime, text: &str) -> Result<(), Error> {
    use libc::strlen;
    unsafe {
        let text_c = text.strdup();
        if 0 != mailmime_set_body_text(part, text_c, strlen(text_c)) {
            bail!("could not set body text on mime-structure");
        }
    }
    Ok(())
}

pub fn content_type_needs_encoding(content: *const mailmime_content) -> bool {
    unsafe {
        if (*(*content).ct_type).tp_type == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int {
            let composite = (*(*content).ct_type).tp_data.tp_composite_type;
            match (*composite).ct_type as u32 {
                MAILMIME_COMPOSITE_TYPE_MESSAGE => as_str((*content).ct_subtype) != "rfc822",
                MAILMIME_COMPOSITE_TYPE_MULTIPART => false,
                _ => false,
            }
        } else {
            true
        }
    }
}

pub fn new_mailbox_list(displayname: &str, addr: &str) -> *mut mailimf_mailbox_list {
    let mbox: *mut mailimf_mailbox_list = unsafe { mailimf_mailbox_list_new_empty() };
    unsafe {
        mailimf_mailbox_list_add(
            mbox,
            mailimf_mailbox_new(
                if !displayname.is_empty() {
                    dc_encode_header_words(&displayname).strdup()
                } else {
                    ptr::null_mut()
                },
                addr.strdup(),
            ),
        );
    }
    mbox
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_encoding() {
        assert!(content_type_needs_encoding(
            new_content_type("text/plain").unwrap()
        ));
        assert!(content_type_needs_encoding(
            new_content_type("application/octect-stream").unwrap()
        ));
        assert!(!content_type_needs_encoding(
            new_content_type("multipart/encrypted").unwrap()
        ));
        assert!(content_type_needs_encoding(
            new_content_type("application/pgp-encrypted").unwrap()
        ));
    }

    #[test]
    fn test_parse_message_id() {
        assert_eq!(
            parse_message_id("Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org").unwrap(),
            "Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org"
        );
        assert_eq!(
            parse_message_id("<Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org>").unwrap(),
            "Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org"
        );
    }
}

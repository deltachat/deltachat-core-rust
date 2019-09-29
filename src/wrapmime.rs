use std::ffi::CString;
use std::ptr;

use crate::dc_tools::*;
use crate::error::Error;
use mmime::clist::*;
use mmime::display::*;
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
            Some(to_string((*ct).ct_subtype))
        } else {
            None
        }
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

        if decoded_data.is_null() || decoded_data_bytes <= 0 {
            bail!("No data to decode found");
        } else {
            let result = unsafe {
                std::slice::from_raw_parts(decoded_data as *const u8, decoded_data_bytes)
            };
            return Ok(result.to_vec());
        }
    }
    unsafe { display_mime_data(mime_data) };

    let mut current_index = 0;
    let mut transfer_decoding_buffer = ptr::null_mut();
    let mut decoded_data_bytes = 0;

    let r = unsafe { mailmime_part_parse(
        (*mime_data).dt_data.dt_text.dt_data,
        (*mime_data).dt_data.dt_text.dt_length,
        &mut current_index,
        mime_transfer_encoding,
        &mut transfer_decoding_buffer,
        &mut decoded_data_bytes,
    ) };

    if r == MAILIMF_NO_ERROR as libc::c_int
        && !transfer_decoding_buffer.is_null()
        && decoded_data_bytes > 0
    {
        let result = unsafe { std::slice::from_raw_parts(
            transfer_decoding_buffer as *const u8,
            decoded_data_bytes,
        ) }
        .to_vec();
        // we return a fresh vec and transfer_decoding_buffer is not used or passed anywhere
        // so it's safe to free it right away, as mailman_part_parse has
        // allocated it fresh.
        unsafe { mmap_string_unref(transfer_decoding_buffer) };

        return Ok(result);
    }

    Err(format_err!("Failed to to decode"))
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
        let name_c = CString::new(name).unwrap();
        let value_c = CString::new(value).unwrap();

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
    let ct = CString::new(content_type).unwrap();
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
}

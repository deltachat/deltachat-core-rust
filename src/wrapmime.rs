use std::ffi::CString;

use crate::dc_tools::*;
use crate::error::Error;
use mmime::clist::*;
use mmime::mailimf::types::*;
use mmime::mailimf::types_helper::*;
use mmime::mailmime::disposition::*;
use mmime::mailmime::types::*;
use mmime::mailmime::types_helper::*;
use mmime::mailmime::*;
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

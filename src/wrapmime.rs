use std::ffi::CString;

use crate::dc_tools::*;
use crate::error::Error;
use mmime::clist::*;
use mmime::mailimf_types::*;
use mmime::mailimf_types_helper::*;
use mmime::mailmime_disposition::*;
use mmime::mailmime_types::*;
use mmime::mailmime_types_helper::*;
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
    message: *mut mailmime,
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


pub fn build_body_text(text: &str) -> Result<*mut mailmime, Error> {
    let mime_fields: *mut mailmime_fields;
    let message_part: *mut mailmime;

    let content = new_mailmime_content_type("text/plain");
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
        let name_c = CString::new(name).unwrap().as_ptr();
        let value_c = CString::new(value).unwrap().as_ptr();

        clist_append!(
            (*content).ct_parameters,
            mailmime_param_new_with_data(
                name_c as *const u8 as *const libc::c_char as *mut libc::c_char,
                value_c as *const u8 as *const libc::c_char as *mut libc::c_char
            )
        );
    }
    Ok(())
}

pub fn new_mailmime_content_type(content_type: &str) -> *mut mailmime_content {
    let ct = CString::new(content_type).unwrap();
    let content: *mut mailmime_content;
    // mailmime_content_new_with_str only parses but does not retain/own ct
    //
    unsafe {
        content = mailmime_content_new_with_str(ct.as_ptr());
    }
    if content.is_null() {
        panic!("mailimf failed to allocate");
    }
    content
}

pub fn set_body_text(part: *mut mailmime, text: &str) -> Result<(), Error> {
    use libc::strlen;
    unsafe {
        let text_c = text.strdup();
        if 0 != mailmime_set_body_text(part, text_c, strlen(text_c)) {
            bail!("could not set body text on mime-structure");
        }
    }
    Ok(())
}

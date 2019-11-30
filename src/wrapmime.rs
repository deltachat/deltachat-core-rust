use std::collections::{HashMap, HashSet};

use mailparse::ParsedMail;

use crate::contact::addr_normalize;
use crate::error::Error;

/**************************************
* mime parsing API
**************************************/

pub fn parse_message_id(message_id: &[u8]) -> Result<String, Error> {
    let value = std::str::from_utf8(message_id)?;
    let addrs = mailparse::addrparse(value)
        .map_err(|err| format_err!("failed to parse message id {:?}", err))?;

    if let Some(info) = addrs.extract_single_info() {
        return Ok(info.addr);
    }

    bail!(
        "could not parse message_id: {}",
        String::from_utf8_lossy(message_id)
    );
}

/// Returns a reference to the encrypted payload and validates the autocrypt structure.
pub fn get_autocrypt_mime<'a, 'b>(mail: &'a ParsedMail<'b>) -> Result<&'a ParsedMail<'b>, Error> {
    ensure!(
        mail.ctype.mimetype == "multipart/encrypted",
        "Not a multipart/encrypted message"
    );
    ensure!(
        mail.subparts.len() == 2,
        "Invalid Autocrypt Level 1 Mime Parts"
    );

    ensure!(
        mail.subparts[0].ctype.mimetype == "application/pgp-encrypted",
        "Invalid Autocrypt Level 1 version part"
    );

    ensure!(
        mail.subparts[1].ctype.mimetype == "application/octetstream",
        "Invalid Autocrypt Level 1 encrypted part"
    );

    Ok(&mail.subparts[1])
}

// returned addresses are normalized.
pub fn mailimf_get_recipients(headers: &HashMap<String, String>) -> HashSet<String> {
    let mut recipients: HashSet<String> = Default::default();

    for (hkey, hvalue) in headers.iter() {
        if hkey == "to" || hkey == "cc" {
            if let Ok(addrs) = mailparse::addrparse(hvalue) {
                for addr in addrs.iter() {
                    match addr {
                        mailparse::MailAddr::Single(ref info) => {
                            recipients.insert(addr_normalize(&info.addr).into());
                        }
                        mailparse::MailAddr::Group(ref infos) => {
                            for info in &infos.addrs {
                                recipients.insert(addr_normalize(&info.addr).into());
                            }
                        }
                    }
                }
            }
        }
    }

    recipients
}

/**************************************
* mime creation API
**************************************/

// pub fn add_filename_part(
//     message: *mut Mailmime,
//     basename: &str,
//     mime_type: &str,
//     file_content: &str,
// ) -> Result<(), Error> {
//     let mime_type_c = CString::new(mime_type.to_string()).expect("failed to create CString");
//      {
//         let content_type = mailmime_content_new_with_str(mime_type_c.as_ptr());
//         let mime_fields = mailmime_fields_new_filename(
//             MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
//             basename.strdup(),
//             MAILMIME_MECHANISM_8BIT as libc::c_int,
//         );
//         let file_mime_part = mailmime_new_empty(content_type, mime_fields);
//         set_body_text(file_mime_part, file_content)?;
//         mailmime_smart_add_part(message, file_mime_part);
//     }
//     Ok(())
// }

// pub fn new_custom_field(fields: *mut mailimf_fields, name: &str, value: &str) {
//      {
//         let field = mailimf_field_new_custom(name.strdup(), value.strdup());
//         let res = mailimf_fields_add(fields, field);
//         assert!(
//             res as u32 == MAILIMF_NO_ERROR,
//             "could not create mailimf field"
//         );
//     }
// }

// pub fn build_body_text(text: &str) -> Result<*mut Mailmime, Error> {
//     let mime_fields: *mut mailmime_fields;
//     let message_part: *mut Mailmime;

//     let content = new_content_type("text/plain")?;
//     append_ct_param(content, "charset", "utf-8")?;

//      {
//         mime_fields = mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
//         message_part = mailmime_new_empty(content, mime_fields);
//     }
//     set_body_text(message_part, text)?;

//     Ok(message_part)
// }

// pub fn append_ct_param(
//     content: *mut mailmime_content,
//     name: &str,
//     value: &str,
// ) -> Result<(), Error> {
//      {
//         let name_c = CString::new(name).unwrap_or_default();
//         let value_c = CString::new(value).unwrap_or_default();

//         clist_append!(
//             (*content).ct_parameters,
//             mailmime_param_new_with_data(
//                 name_c.as_ptr() as *const u8 as *const libc::c_char as *mut libc::c_char,
//                 value_c.as_ptr() as *const u8 as *const libc::c_char as *mut libc::c_char
//             )
//         );
//     }
//     Ok(())
// }

// pub fn new_content_type(content_type: &str) -> Result<*mut mailmime_content, Error> {
//     let ct = CString::new(content_type).unwrap_or_default();
//     let content: *mut mailmime_content;
//     // mailmime_content_new_with_str only parses but does not retain/own ct
//      {
//         content = mailmime_content_new_with_str(ct.as_ptr());
//     }
//     ensure!(!content.is_null(), "mailimf failed to allocate");
//     Ok(content)
// }

// pub fn set_body_text(part: *mut Mailmime, text: &str) -> Result<(), Error> {
//     use libc::strlen;
//      {
//         let text_c = text.strdup();
//         if 0 != mailmime_set_body_text(part, text_c, strlen(text_c)) {
//             bail!("could not set body text on mime-structure");
//         }
//     }
//     Ok(())
// }

// pub fn content_type_needs_encoding(content: *const mailmime_content) -> bool {
//      {
//         if (*(*content).ct_type).tp_type == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int {
//             let composite = (*(*content).ct_type).tp_data.tp_composite_type;
//             match (*composite).ct_type as u32 {
//                 MAILMIME_COMPOSITE_TYPE_MESSAGE => {
//                     to_string_lossy((*content).ct_subtype) != "rfc822"
//                 }
//                 MAILMIME_COMPOSITE_TYPE_MULTIPART => false,
//                 _ => false,
//             }
//         } else {
//             true
//         }
//     }
// }

// pub fn new_mailbox_list(displayname: &str, addr: &str) -> *mut mailimf_mailbox_list {
//     let mbox: *mut mailimf_mailbox_list =  { mailimf_mailbox_list_new_empty() };
//      {
//         mailimf_mailbox_list_add(
//             mbox,
//             mailimf_mailbox_new(
//                 if !displayname.is_empty() {
//                     dc_encode_header_words(&displayname).strdup()
//                 } else {
//                     ptr::null_mut()
//                 },
//                 addr.strdup(),
//             ),
//         );
//     }
//     mbox
// }

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_needs_encoding() {
    //     assert!(content_type_needs_encoding(
    //         new_content_type("text/plain").unwrap()
    //     ));
    //     assert!(content_type_needs_encoding(
    //         new_content_type("application/octect-stream").unwrap()
    //     ));
    //     assert!(!content_type_needs_encoding(
    //         new_content_type("multipart/encrypted").unwrap()
    //     ));
    //     assert!(content_type_needs_encoding(
    //         new_content_type("application/pgp-encrypted").unwrap()
    //     ));
    // }

    #[test]
    fn test_parse_message_id() {
        assert_eq!(
            parse_message_id(b"Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org").unwrap(),
            "Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org"
        );
        assert_eq!(
            parse_message_id(b"<Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org>").unwrap(),
            "Mr.PRUe8HJBoaO.3whNvLCMFU0@testrun.org"
        );
    }
}

use libc;
use libc::toupper;

use crate::clist::*;
use crate::mailimf_types::*;
use crate::mailmime_decode::*;
use crate::mailmime_types::*;
use crate::mmapstring::*;
use crate::other::*;

pub const UNSTRUCTURED_START: libc::c_uint = 0;
pub const UNSTRUCTURED_LF: libc::c_uint = 2;
pub const UNSTRUCTURED_CR: libc::c_uint = 1;
pub const UNSTRUCTURED_WSP: libc::c_uint = 3;
pub const UNSTRUCTURED_OUT: libc::c_uint = 4;

pub const STATE_ZONE_ERR: libc::c_uint = 4;
pub const STATE_ZONE_OK: libc::c_uint = 3;
pub const STATE_ZONE_3: libc::c_uint = 2;
pub const STATE_ZONE_2: libc::c_uint = 1;
pub const STATE_ZONE_CONT: libc::c_uint = 5;
pub const STATE_ZONE_1: libc::c_uint = 0;

pub const MONTH_A: libc::c_uint = 5;
pub const MONTH_MA: libc::c_uint = 4;
pub const MONTH_M: libc::c_uint = 3;
pub const MONTH_JU: libc::c_uint = 2;
pub const MONTH_J: libc::c_uint = 1;
pub const MONTH_START: libc::c_uint = 0;
pub const DAY_NAME_S: libc::c_uint = 2;
pub const DAY_NAME_T: libc::c_uint = 1;
pub const DAY_NAME_START: libc::c_uint = 0;
pub const HEADER_RES: libc::c_uint = 5;
pub const HEADER_S: libc::c_uint = 4;
pub const HEADER_RE: libc::c_uint = 3;
pub const HEADER_R: libc::c_uint = 2;
pub const HEADER_C: libc::c_uint = 1;
pub const HEADER_START: libc::c_uint = 0;

/*
day-name        =       "Mon" / "Tue" / "Wed" / "Thu" /
                        "Fri" / "Sat" / "Sun"
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_token_value {
    pub value: libc::c_int,
    pub str_0: *mut libc::c_char,
}

/*
  mailimf_message_parse will parse the given message

  @param message this is a string containing the message content
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the message in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_message_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_message,
) -> libc::c_int {
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut body: *mut mailimf_body = 0 as *mut mailimf_body;
    let mut msg: *mut mailimf_message = 0 as *mut mailimf_message;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_fields_parse(message, length, &mut cur_token, &mut fields);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_crlf_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
            res = r
        } else {
            r = mailimf_body_parse(message, length, &mut cur_token, &mut body);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                msg = mailimf_message_new(fields, body);
                if msg.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                    mailimf_body_free(body);
                } else {
                    *indx = cur_token;
                    *result = msg;
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
            mailimf_fields_free(fields);
        }
    }
    return res;
}
/*
  mailimf_body_parse will parse the given text part of a message

  @param message this is a string containing the message text part
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the message text part in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_body_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_body,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut body: *mut mailimf_body = 0 as *mut mailimf_body;
    cur_token = *indx;
    body = mailimf_body_new(
        message.offset(cur_token as isize),
        length.wrapping_sub(cur_token),
    );
    if body.is_null() {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    cur_token = length;
    *result = body;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_crlf_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_char_parse(message, length, &mut cur_token, '\r' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_char_parse(message, length, &mut cur_token, '\n' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_char_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut token: libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if *message.offset(cur_token as isize) as libc::c_int == token as libc::c_int {
        cur_token = cur_token.wrapping_add(1);
        *indx = cur_token;
        return MAILIMF_NO_ERROR as libc::c_int;
    } else {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    };
}
/*
  mailimf_fields_parse will parse the given header fields

  @param message this is a string containing the header fields
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the header fields in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_fields_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_fields,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    list = 0 as *mut clist;
    r = mailimf_struct_multiple_parse(
        message,
        length,
        &mut cur_token,
        &mut list,
        ::std::mem::transmute::<
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut *mut mailimf_field,
                ) -> libc::c_int,
            >,
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut libc::c_void,
                ) -> libc::c_int,
            >,
        >(Some(mailimf_field_parse)),
        ::std::mem::transmute::<
            Option<unsafe fn(_: *mut mailimf_field) -> ()>,
            Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
        >(Some(mailimf_field_free)),
    );
    /*
    if ((r != MAILIMF_NO_ERROR) && (r != MAILIMF_ERROR_PARSE)) {
      res = r;
      goto err;
    }
    */
    match r {
        0 => {
            /* do nothing */
            current_block = 11050875288958768710;
        }
        1 => {
            list = clist_new();
            if list.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 6724962012950341805;
            } else {
                current_block = 11050875288958768710;
            }
        }
        _ => {
            res = r;
            current_block = 6724962012950341805;
        }
    }
    match current_block {
        11050875288958768710 => {
            fields = mailimf_fields_new(list);
            if fields.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                if !list.is_null() {
                    clist_foreach(
                        list,
                        ::std::mem::transmute::<
                            Option<unsafe fn(_: *mut mailimf_field) -> ()>,
                            clist_func,
                        >(Some(mailimf_field_free)),
                        0 as *mut libc::c_void,
                    );
                    clist_free(list);
                }
            } else {
                *result = fields;
                *indx = cur_token;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
        _ => {}
    }
    return res;
}
unsafe fn mailimf_field_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_field,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut type_0: libc::c_int = 0;
    let mut return_path: *mut mailimf_return = 0 as *mut mailimf_return;
    let mut resent_date: *mut mailimf_orig_date = 0 as *mut mailimf_orig_date;
    let mut resent_from: *mut mailimf_from = 0 as *mut mailimf_from;
    let mut resent_sender: *mut mailimf_sender = 0 as *mut mailimf_sender;
    let mut resent_to: *mut mailimf_to = 0 as *mut mailimf_to;
    let mut resent_cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
    let mut resent_bcc: *mut mailimf_bcc = 0 as *mut mailimf_bcc;
    let mut resent_msg_id: *mut mailimf_message_id = 0 as *mut mailimf_message_id;
    let mut orig_date: *mut mailimf_orig_date = 0 as *mut mailimf_orig_date;
    let mut from: *mut mailimf_from = 0 as *mut mailimf_from;
    let mut sender: *mut mailimf_sender = 0 as *mut mailimf_sender;
    let mut reply_to: *mut mailimf_reply_to = 0 as *mut mailimf_reply_to;
    let mut to: *mut mailimf_to = 0 as *mut mailimf_to;
    let mut cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
    let mut bcc: *mut mailimf_bcc = 0 as *mut mailimf_bcc;
    let mut message_id: *mut mailimf_message_id = 0 as *mut mailimf_message_id;
    let mut in_reply_to: *mut mailimf_in_reply_to = 0 as *mut mailimf_in_reply_to;
    let mut references: *mut mailimf_references = 0 as *mut mailimf_references;
    let mut subject: *mut mailimf_subject = 0 as *mut mailimf_subject;
    let mut comments: *mut mailimf_comments = 0 as *mut mailimf_comments;
    let mut keywords: *mut mailimf_keywords = 0 as *mut mailimf_keywords;
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    let mut guessed_type: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    return_path = 0 as *mut mailimf_return;
    resent_date = 0 as *mut mailimf_orig_date;
    resent_from = 0 as *mut mailimf_from;
    resent_sender = 0 as *mut mailimf_sender;
    resent_to = 0 as *mut mailimf_to;
    resent_cc = 0 as *mut mailimf_cc;
    resent_bcc = 0 as *mut mailimf_bcc;
    resent_msg_id = 0 as *mut mailimf_message_id;
    orig_date = 0 as *mut mailimf_orig_date;
    from = 0 as *mut mailimf_from;
    sender = 0 as *mut mailimf_sender;
    reply_to = 0 as *mut mailimf_reply_to;
    to = 0 as *mut mailimf_to;
    cc = 0 as *mut mailimf_cc;
    bcc = 0 as *mut mailimf_bcc;
    message_id = 0 as *mut mailimf_message_id;
    in_reply_to = 0 as *mut mailimf_in_reply_to;
    references = 0 as *mut mailimf_references;
    subject = 0 as *mut mailimf_subject;
    comments = 0 as *mut mailimf_comments;
    keywords = 0 as *mut mailimf_keywords;
    optional_field = 0 as *mut mailimf_optional_field;
    guessed_type = guess_header_type(message, length, cur_token);
    type_0 = MAILIMF_FIELD_NONE as libc::c_int;
    match guessed_type {
        9 => {
            r = mailimf_orig_date_parse(message, length, &mut cur_token, &mut orig_date);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = MAILIMF_FIELD_ORIG_DATE as libc::c_int;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        10 => {
            r = mailimf_from_parse(message, length, &mut cur_token, &mut from);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        11 => {
            r = mailimf_sender_parse(message, length, &mut cur_token, &mut sender);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        12 => {
            r = mailimf_reply_to_parse(message, length, &mut cur_token, &mut reply_to);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        13 => {
            r = mailimf_to_parse(message, length, &mut cur_token, &mut to);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        14 => {
            r = mailimf_cc_parse(message, length, &mut cur_token, &mut cc);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        15 => {
            r = mailimf_bcc_parse(message, length, &mut cur_token, &mut bcc);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        16 => {
            r = mailimf_message_id_parse(message, length, &mut cur_token, &mut message_id);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        17 => {
            r = mailimf_in_reply_to_parse(message, length, &mut cur_token, &mut in_reply_to);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        18 => {
            r = mailimf_references_parse(message, length, &mut cur_token, &mut references);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        19 => {
            r = mailimf_subject_parse(message, length, &mut cur_token, &mut subject);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        20 => {
            r = mailimf_comments_parse(message, length, &mut cur_token, &mut comments);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        21 => {
            r = mailimf_keywords_parse(message, length, &mut cur_token, &mut keywords);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        1 => {
            r = mailimf_return_parse(message, length, &mut cur_token, &mut return_path);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        2 => {
            r = mailimf_resent_date_parse(message, length, &mut cur_token, &mut resent_date);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        3 => {
            r = mailimf_resent_from_parse(message, length, &mut cur_token, &mut resent_from);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        4 => {
            r = mailimf_resent_sender_parse(message, length, &mut cur_token, &mut resent_sender);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        5 => {
            r = mailimf_resent_to_parse(message, length, &mut cur_token, &mut resent_to);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        6 => {
            r = mailimf_resent_cc_parse(message, length, &mut cur_token, &mut resent_cc);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        7 => {
            r = mailimf_resent_bcc_parse(message, length, &mut cur_token, &mut resent_bcc);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        8 => {
            r = mailimf_resent_msg_id_parse(message, length, &mut cur_token, &mut resent_msg_id);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 9846950269610550213;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 9846950269610550213;
            } else {
                /* do nothing */
                res = r;
                current_block = 10956553358380301606;
            }
        }
        _ => {
            current_block = 9846950269610550213;
        }
    }
    match current_block {
        9846950269610550213 => {
            if type_0 == MAILIMF_FIELD_NONE as libc::c_int {
                r = mailimf_optional_field_parse(
                    message,
                    length,
                    &mut cur_token,
                    &mut optional_field,
                );
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r;
                    current_block = 10956553358380301606;
                } else {
                    type_0 = MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int;
                    current_block = 2920409193602730479;
                }
            } else {
                current_block = 2920409193602730479;
            }
            match current_block {
                10956553358380301606 => {}
                _ => {
                    field = mailimf_field_new(
                        type_0,
                        return_path,
                        resent_date,
                        resent_from,
                        resent_sender,
                        resent_to,
                        resent_cc,
                        resent_bcc,
                        resent_msg_id,
                        orig_date,
                        from,
                        sender,
                        reply_to,
                        to,
                        cc,
                        bcc,
                        message_id,
                        in_reply_to,
                        references,
                        subject,
                        comments,
                        keywords,
                        optional_field,
                    );
                    if field.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        if !return_path.is_null() {
                            mailimf_return_free(return_path);
                        }
                        if !resent_date.is_null() {
                            mailimf_orig_date_free(resent_date);
                        }
                        if !resent_from.is_null() {
                            mailimf_from_free(resent_from);
                        }
                        if !resent_sender.is_null() {
                            mailimf_sender_free(resent_sender);
                        }
                        if !resent_to.is_null() {
                            mailimf_to_free(resent_to);
                        }
                        if !resent_cc.is_null() {
                            mailimf_cc_free(resent_cc);
                        }
                        if !resent_bcc.is_null() {
                            mailimf_bcc_free(resent_bcc);
                        }
                        if !resent_msg_id.is_null() {
                            mailimf_message_id_free(resent_msg_id);
                        }
                        if !orig_date.is_null() {
                            mailimf_orig_date_free(orig_date);
                        }
                        if !from.is_null() {
                            mailimf_from_free(from);
                        }
                        if !sender.is_null() {
                            mailimf_sender_free(sender);
                        }
                        if !reply_to.is_null() {
                            mailimf_reply_to_free(reply_to);
                        }
                        if !to.is_null() {
                            mailimf_to_free(to);
                        }
                        if !cc.is_null() {
                            mailimf_cc_free(cc);
                        }
                        if !bcc.is_null() {
                            mailimf_bcc_free(bcc);
                        }
                        if !message_id.is_null() {
                            mailimf_message_id_free(message_id);
                        }
                        if !in_reply_to.is_null() {
                            mailimf_in_reply_to_free(in_reply_to);
                        }
                        if !references.is_null() {
                            mailimf_references_free(references);
                        }
                        if !subject.is_null() {
                            mailimf_subject_free(subject);
                        }
                        if !comments.is_null() {
                            mailimf_comments_free(comments);
                        }
                        if !keywords.is_null() {
                            mailimf_keywords_free(keywords);
                        }
                        if !optional_field.is_null() {
                            mailimf_optional_field_free(optional_field);
                        }
                    } else {
                        *result = field;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
            }
        }
        _ => {}
    }
    return res;
}
unsafe fn mailimf_optional_field_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_optional_field,
) -> libc::c_int {
    let mut name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_field_name_parse(message, length, &mut cur_token, &mut name);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_unstructured_parse(message, length, &mut cur_token, &mut value);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    optional_field = mailimf_optional_field_new(name, value);
                    if optional_field.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = optional_field;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_unstructured_free(value);
            }
        }
        mailimf_field_name_free(name);
    }
    return res;
}
unsafe fn mailimf_unstrict_crlf_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    mailimf_cfws_parse(message, length, &mut cur_token);
    r = mailimf_char_parse(message, length, &mut cur_token, '\r' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_char_parse(message, length, &mut cur_token, '\n' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_cfws_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut has_comment: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    has_comment = 0i32;
    loop {
        r = mailimf_cfws_fws_comment_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                break;
            }
            return r;
        } else {
            has_comment = 1i32
        }
    }
    if 0 == has_comment {
        r = mailimf_fws_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/* internal use, exported for MIME */
pub unsafe fn mailimf_fws_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut final_token: size_t = 0;
    let mut fws_1: libc::c_int = 0;
    let mut fws_2: libc::c_int = 0;
    let mut fws_3: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    fws_1 = 0i32;
    loop {
        r = mailimf_wsp_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                break;
            }
            return r;
        } else {
            fws_1 = 1i32
        }
    }
    final_token = cur_token;
    r = mailimf_crlf_parse(message, length, &mut cur_token);
    match r {
        0 => fws_2 = 1i32,
        1 => fws_2 = 0i32,
        _ => return r,
    }
    fws_3 = 0i32;
    if 0 != fws_2 {
        loop {
            r = mailimf_wsp_parse(message, length, &mut cur_token);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    break;
                }
                return r;
            } else {
                fws_3 = 1i32
            }
        }
    }
    if 0 == fws_1 && 0 == fws_3 {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if 0 == fws_3 {
        cur_token = final_token
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
#[inline]
unsafe fn mailimf_wsp_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if *message.offset(cur_token as isize) as libc::c_int != ' ' as i32
        && *message.offset(cur_token as isize) as libc::c_int != '\t' as i32
    {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    cur_token = cur_token.wrapping_add(1);
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
[FWS] comment
*/
#[inline]
unsafe fn mailimf_cfws_fws_comment_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_comment_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
#[inline]
unsafe fn mailimf_comment_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_oparenth_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    loop {
        r = mailimf_comment_fws_ccontent_parse(message, length, &mut cur_token);
        if !(r != MAILIMF_NO_ERROR as libc::c_int) {
            continue;
        }
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            break;
        }
        return r;
    }
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_cparenth_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_cparenth_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_char_parse(message, length, indx, ')' as i32 as libc::c_char);
}
#[inline]
unsafe fn mailimf_comment_fws_ccontent_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_ccontent_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
#[inline]
unsafe fn mailimf_ccontent_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut ch: libc::c_char = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if 0 != is_ctext(*message.offset(cur_token as isize)) {
        cur_token = cur_token.wrapping_add(1)
    } else {
        r = mailimf_quoted_pair_parse(message, length, &mut cur_token, &mut ch);
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            r = mailimf_comment_parse(message, length, &mut cur_token)
        }
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            return r;
        }
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
#[inline]
unsafe fn mailimf_quoted_pair_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    if cur_token.wrapping_add(1i32 as libc::size_t) >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if *message.offset(cur_token as isize) as libc::c_int != '\\' as i32 {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    cur_token = cur_token.wrapping_add(1);
    *result = *message.offset(cur_token as isize);
    cur_token = cur_token.wrapping_add(1);
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
ctext           =       NO-WS-CTL /     ; Non white space controls

                        %d33-39 /       ; The rest of the US-ASCII
                        %d42-91 /       ;  characters not including "(",
                        %d93-126        ;  ")", or "\"
*/
#[inline]
unsafe fn is_ctext(mut ch: libc::c_char) -> libc::c_int {
    let mut uch: libc::c_uchar = ch as libc::c_uchar;
    if 0 != is_no_ws_ctl(ch) {
        return 1i32;
    }
    if (uch as libc::c_int) < 33i32 {
        return 0i32;
    }
    if uch as libc::c_int == 40i32 || uch as libc::c_int == 41i32 {
        return 0i32;
    }
    if uch as libc::c_int == 92i32 {
        return 0i32;
    }
    if uch as libc::c_int == 127i32 {
        return 0i32;
    }
    return 1i32;
}
/* ************************************************************************ */
/* RFC 2822 grammar */
/*
NO-WS-CTL       =       %d1-8 /         ; US-ASCII control characters
                        %d11 /          ;  that do not include the
                        %d12 /          ;  carriage return, line feed,
                        %d14-31 /       ;  and white space characters
                        %d127
*/
#[inline]
unsafe fn is_no_ws_ctl(mut ch: libc::c_char) -> libc::c_int {
    if ch as libc::c_int == 9i32 || ch as libc::c_int == 10i32 || ch as libc::c_int == 13i32 {
        return 0i32;
    }
    if ch as libc::c_int == 127i32 {
        return 1i32;
    }
    return (ch as libc::c_int >= 1i32 && ch as libc::c_int <= 31i32) as libc::c_int;
}
unsafe fn mailimf_oparenth_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_char_parse(message, length, indx, '(' as i32 as libc::c_char);
}
unsafe fn mailimf_unstructured_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut state: libc::c_int = 0;
    let mut begin: size_t = 0;
    let mut terminal: size_t = 0;
    let mut str: *mut libc::c_char = 0 as *mut libc::c_char;
    cur_token = *indx;
    loop {
        let mut r: libc::c_int = 0;
        r = mailimf_wsp_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            continue;
        }
        /* do nothing */
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            break;
        }
        return r;
    }
    state = UNSTRUCTURED_START as libc::c_int;
    begin = cur_token;
    terminal = cur_token;
    while state != UNSTRUCTURED_OUT as libc::c_int {
        match state {
            0 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                terminal = cur_token;
                match *message.offset(cur_token as isize) as libc::c_int {
                    13 => state = UNSTRUCTURED_CR as libc::c_int,
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            1 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            2 => {
                if cur_token >= length {
                    state = UNSTRUCTURED_OUT as libc::c_int
                } else {
                    match *message.offset(cur_token as isize) as libc::c_int {
                        9 | 32 => state = UNSTRUCTURED_WSP as libc::c_int,
                        _ => state = UNSTRUCTURED_OUT as libc::c_int,
                    }
                }
            }
            3 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    13 => state = UNSTRUCTURED_CR as libc::c_int,
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            _ => {}
        }
        cur_token = cur_token.wrapping_add(1)
    }
    str = malloc(
        terminal
            .wrapping_sub(begin)
            .wrapping_add(1i32 as libc::size_t),
    ) as *mut libc::c_char;
    if str.is_null() {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    strncpy(
        str,
        message.offset(begin as isize),
        terminal.wrapping_sub(begin),
    );
    *str.offset(terminal.wrapping_sub(begin) as isize) = '\u{0}' as i32 as libc::c_char;
    *indx = terminal;
    *result = str;
    return MAILIMF_NO_ERROR as libc::c_int;
}

unsafe fn mailimf_colon_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, ':' as i32 as libc::c_char);
}

pub unsafe fn mailimf_unstrict_char_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut token: libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_char_parse(message, length, &mut cur_token, token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_field_name_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut field_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut end: size_t = 0;
    cur_token = *indx;
    end = cur_token;
    if end >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    while 0 != is_ftext(*message.offset(end as isize)) {
        end = end.wrapping_add(1);
        if end >= length {
            break;
        }
    }
    if end == cur_token {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    field_name = malloc(
        end.wrapping_sub(cur_token)
            .wrapping_add(1i32 as libc::size_t),
    ) as *mut libc::c_char;
    if field_name.is_null() {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    strncpy(
        field_name,
        message.offset(cur_token as isize),
        end.wrapping_sub(cur_token),
    );
    *field_name.offset(end.wrapping_sub(cur_token) as isize) = '\u{0}' as i32 as libc::c_char;
    cur_token = end;
    *indx = cur_token;
    *result = field_name;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
field-name      =       1*ftext
*/
#[inline]
unsafe fn is_ftext(mut ch: libc::c_char) -> libc::c_int {
    let mut uch: libc::c_uchar = ch as libc::c_uchar;
    if (uch as libc::c_int) < 33i32 {
        return 0i32;
    }
    if uch as libc::c_int == 58i32 {
        return 0i32;
    }
    return 1i32;
}
unsafe fn mailimf_resent_msg_id_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_message_id,
) -> libc::c_int {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut message_id: *mut mailimf_message_id = 0 as *mut mailimf_message_id;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Resent-Message-ID\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Resent-Message-ID\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_msg_id_parse(message, length, &mut cur_token, &mut value);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    message_id = mailimf_message_id_new(value);
                    if message_id.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = message_id;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_msg_id_free(value);
            }
        }
    }
    return res;
}

pub unsafe fn mailimf_msg_id_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut msg_id: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_lower_parse(message, length, &mut cur_token);
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_addr_spec_msg_id_parse(message, length, &mut cur_token, &mut msg_id);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            *result = msg_id;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    } else if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_lower_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            current_block = 2668756484064249700;
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
            current_block = 2668756484064249700;
        } else {
            // ok
            res = r;
            current_block = 9394595304415473402;
        }
        match current_block {
            9394595304415473402 => {}
            _ => {
                r = mailimf_addr_spec_msg_id_parse(message, length, &mut cur_token, &mut msg_id);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    r = mailimf_greater_parse(message, length, &mut cur_token);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        free(msg_id as *mut libc::c_void);
                        res = r
                    } else {
                        r = mailimf_greater_parse(message, length, &mut cur_token);
                        if r == MAILIMF_NO_ERROR as libc::c_int {
                            current_block = 6450636197030046351;
                        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                            current_block = 6450636197030046351;
                        } else {
                            // ok
                            free(msg_id as *mut libc::c_void);
                            res = r;
                            current_block = 9394595304415473402;
                        }
                        match current_block {
                            9394595304415473402 => {}
                            _ => {
                                *result = msg_id;
                                *indx = cur_token;
                                return MAILIMF_NO_ERROR as libc::c_int;
                            }
                        }
                    }
                }
            }
        }
    }
    return res;
}
unsafe fn mailimf_greater_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, '>' as i32 as libc::c_char);
}
/*
for msg id
addr-spec       =       local-part "@" domain
*/
unsafe fn mailimf_addr_spec_msg_id_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut addr_spec: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut begin: size_t = 0;
    let mut end: size_t = 0;
    let mut final_0: libc::c_int = 0;
    let mut count: size_t = 0;
    let mut src: *const libc::c_char = 0 as *const libc::c_char;
    let mut dest: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut i: size_t = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        end = cur_token;
        if end >= length {
            res = MAILIMF_ERROR_PARSE as libc::c_int
        } else {
            begin = cur_token;
            final_0 = 0i32;
            loop {
                match *message.offset(end as isize) as libc::c_int {
                    62 | 13 | 10 => final_0 = 1i32,
                    _ => {}
                }
                if 0 != final_0 {
                    break;
                }
                end = end.wrapping_add(1);
                if end >= length {
                    break;
                }
            }
            if end == begin {
                res = MAILIMF_ERROR_PARSE as libc::c_int
            } else {
                addr_spec = malloc(
                    end.wrapping_sub(cur_token)
                        .wrapping_add(1i32 as libc::size_t),
                ) as *mut libc::c_char;
                if addr_spec.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int
                } else {
                    count = end.wrapping_sub(cur_token);
                    src = message.offset(cur_token as isize);
                    dest = addr_spec;
                    i = 0i32 as size_t;
                    while i < count {
                        if *src as libc::c_int != ' ' as i32 && *src as libc::c_int != '\t' as i32 {
                            *dest = *src;
                            dest = dest.offset(1isize)
                        }
                        src = src.offset(1isize);
                        i = i.wrapping_add(1)
                    }
                    *dest = '\u{0}' as i32 as libc::c_char;
                    cur_token = end;
                    *result = addr_spec;
                    *indx = cur_token;
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
        }
    }
    return res;
}
unsafe fn mailimf_lower_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, '<' as i32 as libc::c_char);
}

pub unsafe fn mailimf_token_case_insensitive_len_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut token: *mut libc::c_char,
    mut token_length: size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    if cur_token
        .wrapping_add(token_length)
        .wrapping_sub(1i32 as libc::size_t)
        >= length
    {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if strncasecmp(message.offset(cur_token as isize), token, token_length) == 0i32 {
        cur_token = (cur_token as libc::size_t).wrapping_add(token_length) as size_t as size_t;
        *indx = cur_token;
        return MAILIMF_NO_ERROR as libc::c_int;
    } else {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    };
}
unsafe fn mailimf_resent_bcc_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_bcc,
) -> libc::c_int {
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut bcc: *mut mailimf_bcc = 0 as *mut mailimf_bcc;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    bcc = 0 as *mut mailimf_bcc;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Resent-Bcc\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Resent-Bcc\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            addr_list = 0 as *mut mailimf_address_list;
            r = mailimf_address_list_parse(message, length, &mut cur_token, &mut addr_list);
            if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    bcc = mailimf_bcc_new(addr_list);
                    if bcc.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = bcc;
                        *indx = cur_token;
                        return 1i32;
                    }
                }
                mailimf_address_list_free(addr_list);
            }
        }
    }
    return res;
}
/*
  mailimf_address_list_parse will parse the given address list

  @param message this is a string containing the address list
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the address list in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_address_list_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_address_list,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut address_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_struct_list_parse(
        message,
        length,
        &mut cur_token,
        &mut list,
        ',' as i32 as libc::c_char,
        ::std::mem::transmute::<
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut *mut mailimf_address,
                ) -> libc::c_int,
            >,
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut libc::c_void,
                ) -> libc::c_int,
            >,
        >(Some(mailimf_address_parse)),
        ::std::mem::transmute::<
            Option<unsafe fn(_: *mut mailimf_address) -> ()>,
            Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
        >(Some(mailimf_address_free)),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        address_list = mailimf_address_list_new(list);
        if address_list.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            clist_foreach(
                list,
                ::std::mem::transmute::<Option<unsafe fn(_: *mut mailimf_address) -> ()>, clist_func>(
                    Some(mailimf_address_free),
                ),
                0 as *mut libc::c_void,
            );
            clist_free(list);
        } else {
            *result = address_list;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
/*
  mailimf_address_parse will parse the given address

  @param message this is a string containing the address
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the address in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_address_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_address,
) -> libc::c_int {
    let mut type_0: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut mailbox: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
    let mut group: *mut mailimf_group = 0 as *mut mailimf_group;
    let mut address: *mut mailimf_address = 0 as *mut mailimf_address;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    mailbox = 0 as *mut mailimf_mailbox;
    group = 0 as *mut mailimf_group;
    type_0 = MAILIMF_ADDRESS_ERROR as libc::c_int;
    r = mailimf_group_parse(message, length, &mut cur_token, &mut group);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        type_0 = MAILIMF_ADDRESS_GROUP as libc::c_int
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_mailbox_parse(message, length, &mut cur_token, &mut mailbox);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILIMF_ADDRESS_MAILBOX as libc::c_int
        }
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        address = mailimf_address_new(type_0, mailbox, group);
        if address.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            if !mailbox.is_null() {
                mailimf_mailbox_free(mailbox);
            }
            if !group.is_null() {
                mailimf_group_free(group);
            }
        } else {
            *result = address;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
/*
  mailimf_mailbox_parse will parse the given address

  @param message this is a string containing the mailbox
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the mailbox in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_mailbox_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_mailbox,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut display_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut mailbox: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
    let mut addr_spec: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    display_name = 0 as *mut libc::c_char;
    addr_spec = 0 as *mut libc::c_char;
    r = mailimf_name_addr_parse(
        message,
        length,
        &mut cur_token,
        &mut display_name,
        &mut addr_spec,
    );
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_addr_spec_parse(message, length, &mut cur_token, &mut addr_spec)
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        mailbox = mailimf_mailbox_new(display_name, addr_spec);
        if mailbox.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            if !display_name.is_null() {
                mailimf_display_name_free(display_name);
            }
            if !addr_spec.is_null() {
                mailimf_addr_spec_free(addr_spec);
            }
        } else {
            *result = mailbox;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
unsafe fn mailimf_addr_spec_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut addr_spec: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut begin: size_t = 0;
    let mut end: size_t = 0;
    let mut final_0: libc::c_int = 0;
    let mut count: size_t = 0;
    let mut src: *const libc::c_char = 0 as *const libc::c_char;
    let mut dest: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut i: size_t = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        end = cur_token;
        if end >= length {
            res = MAILIMF_ERROR_PARSE as libc::c_int
        } else {
            begin = cur_token;
            final_0 = 0i32;
            loop {
                match *message.offset(end as isize) as libc::c_int {
                    62 | 44 | 13 | 10 | 40 | 41 | 58 | 59 => final_0 = 1i32,
                    _ => {}
                }
                if 0 != final_0 {
                    break;
                }
                end = end.wrapping_add(1);
                if end >= length {
                    break;
                }
            }
            if end == begin {
                res = MAILIMF_ERROR_PARSE as libc::c_int
            } else {
                addr_spec = malloc(
                    end.wrapping_sub(cur_token)
                        .wrapping_add(1i32 as libc::size_t),
                ) as *mut libc::c_char;
                if addr_spec.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int
                } else {
                    count = end.wrapping_sub(cur_token);
                    src = message.offset(cur_token as isize);
                    dest = addr_spec;
                    i = 0i32 as size_t;
                    while i < count {
                        if *src as libc::c_int != ' ' as i32 && *src as libc::c_int != '\t' as i32 {
                            *dest = *src;
                            dest = dest.offset(1isize)
                        }
                        src = src.offset(1isize);
                        i = i.wrapping_add(1)
                    }
                    *dest = '\u{0}' as i32 as libc::c_char;
                    cur_token = end;
                    *result = addr_spec;
                    *indx = cur_token;
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
        }
    }
    return res;
}
unsafe fn mailimf_name_addr_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut pdisplay_name: *mut *mut libc::c_char,
    mut pangle_addr: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut display_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut angle_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    display_name = 0 as *mut libc::c_char;
    angle_addr = 0 as *mut libc::c_char;
    r = mailimf_display_name_parse(message, length, &mut cur_token, &mut display_name);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        r = mailimf_angle_addr_parse(message, length, &mut cur_token, &mut angle_addr);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r;
            if !display_name.is_null() {
                mailimf_display_name_free(display_name);
            }
        } else {
            *pdisplay_name = display_name;
            *pangle_addr = angle_addr;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
unsafe fn mailimf_angle_addr_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut addr_spec: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_lower_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_addr_spec_parse(message, length, &mut cur_token, &mut addr_spec);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_greater_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        free(addr_spec as *mut libc::c_void);
        return r;
    }
    *result = addr_spec;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_display_name_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailimf_phrase_parse(message, length, indx, result);
}
unsafe fn mailimf_phrase_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut gphrase: *mut MMAPString = 0 as *mut MMAPString;
    let mut word: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut first: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut has_missing_closing_quote: libc::c_int = 0;
    cur_token = *indx;
    has_missing_closing_quote = 0i32;
    gphrase = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
    if gphrase.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        first = 1i32;
        loop {
            let mut missing_quote: libc::c_int = 0i32;
            r = mailimf_fws_word_parse(
                message,
                length,
                &mut cur_token,
                &mut word,
                &mut missing_quote,
            );
            if 0 != missing_quote {
                has_missing_closing_quote = 1i32
            }
            if r == MAILIMF_NO_ERROR as libc::c_int {
                if 0 == first {
                    if mmap_string_append_c(gphrase, ' ' as i32 as libc::c_char).is_null() {
                        mailimf_word_free(word);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 17261756273978092585;
                        break;
                    }
                }
                if mmap_string_append(gphrase, word).is_null() {
                    mailimf_word_free(word);
                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                    current_block = 17261756273978092585;
                    break;
                } else {
                    mailimf_word_free(word);
                    first = 0i32
                }
            } else {
                if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 11636175345244025579;
                    break;
                }
                res = r;
                current_block = 17261756273978092585;
                break;
            }
        }
        match current_block {
            11636175345244025579 => {
                if 0 != first {
                    res = MAILIMF_ERROR_PARSE as libc::c_int
                } else {
                    if 0 != has_missing_closing_quote {
                        r = mailimf_char_parse(
                            message,
                            length,
                            &mut cur_token,
                            '\"' as i32 as libc::c_char,
                        )
                    }
                    str = strdup((*gphrase).str_0);
                    if str.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        mmap_string_free(gphrase);
                        *result = str;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
            }
            _ => {}
        }
        mmap_string_free(gphrase);
    }
    return res;
}

pub unsafe fn mailimf_fws_word_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut p_missing_closing_quote: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut word: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut missing_closing_quote: libc::c_int = 0;
    cur_token = *indx;
    missing_closing_quote = 0i32;
    r = mailimf_fws_atom_for_word_parse(
        message,
        length,
        &mut cur_token,
        &mut word,
        &mut missing_closing_quote,
    );
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_fws_quoted_string_parse(message, length, &mut cur_token, &mut word)
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *result = word;
    *indx = cur_token;
    *p_missing_closing_quote = missing_closing_quote;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_fws_quoted_string_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut gstr: *mut MMAPString = 0 as *mut MMAPString;
    let mut ch: libc::c_char = 0;
    let mut str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        r = mailimf_dquote_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            gstr = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
            if gstr.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int
            } else {
                loop {
                    r = mailimf_fws_parse(message, length, &mut cur_token);
                    if r == MAILIMF_NO_ERROR as libc::c_int {
                        if mmap_string_append_c(gstr, ' ' as i32 as libc::c_char).is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                            current_block = 15096897878952122875;
                            break;
                        }
                    } else if r != MAILIMF_ERROR_PARSE as libc::c_int {
                        res = r;
                        current_block = 15096897878952122875;
                        break;
                    }
                    r = mailimf_qcontent_parse(message, length, &mut cur_token, &mut ch);
                    if r == MAILIMF_NO_ERROR as libc::c_int {
                        if !mmap_string_append_c(gstr, ch).is_null() {
                            continue;
                        }
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 15096897878952122875;
                        break;
                    } else {
                        if r == MAILIMF_ERROR_PARSE as libc::c_int {
                            current_block = 5494826135382683477;
                            break;
                        }
                        res = r;
                        current_block = 15096897878952122875;
                        break;
                    }
                }
                match current_block {
                    5494826135382683477 => {
                        r = mailimf_dquote_parse(message, length, &mut cur_token);
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r
                        } else {
                            str = strdup((*gstr).str_0);
                            if str.is_null() {
                                res = MAILIMF_ERROR_MEMORY as libc::c_int
                            } else {
                                mmap_string_free(gstr);
                                *indx = cur_token;
                                *result = str;
                                return MAILIMF_NO_ERROR as libc::c_int;
                            }
                        }
                    }
                    _ => {}
                }
                mmap_string_free(gstr);
            }
        }
    }
    return res;
}
unsafe fn mailimf_dquote_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_char_parse(message, length, indx, '\"' as i32 as libc::c_char);
}
unsafe fn mailimf_qcontent_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut ch: libc::c_char = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if 0 != is_qtext(*message.offset(cur_token as isize)) {
        ch = *message.offset(cur_token as isize);
        cur_token = cur_token.wrapping_add(1)
    } else {
        r = mailimf_quoted_pair_parse(message, length, &mut cur_token, &mut ch);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
    }
    *result = ch;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
dot-atom        =       [CFWS] dot-atom-text [CFWS]
*/
/*
dot-atom-text   =       1*atext *("." 1*atext)
*/
/*
qtext           =       NO-WS-CTL /     ; Non white space controls

                        %d33 /          ; The rest of the US-ASCII
                        %d35-91 /       ;  characters not including "\"
                        %d93-126        ;  or the quote character
*/
#[inline]
unsafe fn is_qtext(mut ch: libc::c_char) -> libc::c_int {
    let mut uch: libc::c_uchar = ch as libc::c_uchar;
    if 0 != is_no_ws_ctl(ch) {
        return 1i32;
    }
    if (uch as libc::c_int) < 33i32 {
        return 0i32;
    }
    if uch as libc::c_int == 34i32 {
        return 0i32;
    }
    if uch as libc::c_int == 92i32 {
        return 0i32;
    }
    if uch as libc::c_int == 127i32 {
        return 0i32;
    }
    return 1i32;
}
unsafe fn mailimf_fws_atom_for_word_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut p_missing_closing_quote: *mut libc::c_int,
) -> libc::c_int {
    let mut end: size_t = 0;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut word: *mut mailmime_encoded_word = 0 as *mut mailmime_encoded_word;
    let mut has_fwd: libc::c_int = 0;
    let mut missing_closing_quote: libc::c_int = 0;
    let mut atom: *mut libc::c_char = 0 as *mut libc::c_char;
    cur_token = *indx;
    missing_closing_quote = 0i32;
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        end = cur_token;
        r = mailmime_encoded_word_parse(
            message,
            length,
            &mut cur_token,
            &mut word,
            &mut has_fwd,
            &mut missing_closing_quote,
        );
        if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
            res = r
        } else {
            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                return mailimf_fws_atom_parse(message, length, indx, result);
            }
            mailmime_encoded_word_free(word);
            atom = malloc(
                cur_token
                    .wrapping_sub(end)
                    .wrapping_add(1i32 as libc::size_t),
            ) as *mut libc::c_char;
            if atom.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int
            } else {
                strncpy(
                    atom,
                    message.offset(end as isize),
                    cur_token.wrapping_sub(end),
                );
                *atom.offset(cur_token.wrapping_sub(end) as isize) = '\u{0}' as i32 as libc::c_char;
                *result = atom;
                *indx = cur_token;
                *p_missing_closing_quote = missing_closing_quote;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
    }
    return res;
}

pub unsafe fn mailimf_fws_atom_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut atom: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut end: size_t = 0;
    cur_token = *indx;
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        end = cur_token;
        if end >= length {
            res = MAILIMF_ERROR_PARSE as libc::c_int
        } else {
            while 0 != is_atext(*message.offset(end as isize)) {
                end = end.wrapping_add(1);
                if end >= length {
                    break;
                }
            }
            if end == cur_token {
                res = MAILIMF_ERROR_PARSE as libc::c_int
            } else {
                atom = malloc(
                    end.wrapping_sub(cur_token)
                        .wrapping_add(1i32 as libc::size_t),
                ) as *mut libc::c_char;
                if atom.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int
                } else {
                    strncpy(
                        atom,
                        message.offset(cur_token as isize),
                        end.wrapping_sub(cur_token),
                    );
                    *atom.offset(end.wrapping_sub(cur_token) as isize) =
                        '\u{0}' as i32 as libc::c_char;
                    cur_token = end;
                    *indx = cur_token;
                    *result = atom;
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
        }
    }
    return res;
}
/*
atext           =       ALPHA / DIGIT / ; Any character except controls,
                        "!" / "#" /     ;  SP, and specials.
                        "$" / "%" /     ;  Used for atoms
                        "&" / "'" /
                        "*" / "+" /
                        "-" / "/" /
                        "=" / "?" /
                        "^" / "_" /
                        "`" / "{" /
                        "|" / "}" /
                        "~"
*/
#[inline]
unsafe fn is_atext(mut ch: libc::c_char) -> libc::c_int {
    match ch as libc::c_int {
        32 | 9 | 10 | 13 | 60 | 62 | 44 | 34 | 58 | 59 => return 0i32,
        _ => return 1i32,
    };
}
unsafe fn mailimf_group_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_group,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut display_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut mailbox_list: *mut mailimf_mailbox_list = 0 as *mut mailimf_mailbox_list;
    let mut group: *mut mailimf_group = 0 as *mut mailimf_group;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut list: *mut clist = 0 as *mut clist;
    cur_token = *indx;
    mailbox_list = 0 as *mut mailimf_mailbox_list;
    r = mailimf_display_name_parse(message, length, &mut cur_token, &mut display_name);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_mailbox_list_parse(message, length, &mut cur_token, &mut mailbox_list);
            match r {
                0 => {
                    current_block = 1608152415753874203;
                }
                1 => {
                    r = mailimf_cfws_parse(message, length, &mut cur_token);
                    if r != MAILIMF_NO_ERROR as libc::c_int
                        && r != MAILIMF_ERROR_PARSE as libc::c_int
                    {
                        res = r;
                        current_block = 14904789583098922708;
                    } else {
                        list = clist_new();
                        if list.is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                            current_block = 14904789583098922708;
                        } else {
                            mailbox_list = mailimf_mailbox_list_new(list);
                            if mailbox_list.is_null() {
                                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                clist_free(list);
                                current_block = 14904789583098922708;
                            } else {
                                current_block = 1608152415753874203;
                            }
                        }
                    }
                }
                _ => {
                    res = r;
                    current_block = 14904789583098922708;
                }
            }
            match current_block {
                14904789583098922708 => {}
                _ => {
                    r = mailimf_semi_colon_parse(message, length, &mut cur_token);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        res = r
                    } else {
                        group = mailimf_group_new(display_name, mailbox_list);
                        if group.is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int
                        } else {
                            *indx = cur_token;
                            *result = group;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                    if !mailbox_list.is_null() {
                        mailimf_mailbox_list_free(mailbox_list);
                    }
                }
            }
        }
        mailimf_display_name_free(display_name);
    }
    return res;
}

unsafe fn mailimf_semi_colon_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, ';' as i32 as libc::c_char);
}

/*
  mailimf_mailbox_list_parse will parse the given mailbox list

  @param message this is a string containing the mailbox list
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the mailbox list in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_mailbox_list_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_mailbox_list,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut mailbox_list: *mut mailimf_mailbox_list = 0 as *mut mailimf_mailbox_list;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_struct_list_parse(
        message,
        length,
        &mut cur_token,
        &mut list,
        ',' as i32 as libc::c_char,
        ::std::mem::transmute::<
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut *mut mailimf_mailbox,
                ) -> libc::c_int,
            >,
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut libc::c_void,
                ) -> libc::c_int,
            >,
        >(Some(mailimf_mailbox_parse)),
        ::std::mem::transmute::<
            Option<unsafe fn(_: *mut mailimf_mailbox) -> ()>,
            Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
        >(Some(mailimf_mailbox_free)),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        mailbox_list = mailimf_mailbox_list_new(list);
        if mailbox_list.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            clist_foreach(
                list,
                ::std::mem::transmute::<Option<unsafe fn(_: *mut mailimf_mailbox) -> ()>, clist_func>(
                    Some(mailimf_mailbox_free),
                ),
                0 as *mut libc::c_void,
            );
            clist_free(list);
        } else {
            *result = mailbox_list;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
unsafe fn mailimf_struct_list_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut clist,
    mut symbol: libc::c_char,
    mut parser: Option<
        unsafe fn(
            _: *const libc::c_char,
            _: size_t,
            _: *mut size_t,
            _: *mut libc::c_void,
        ) -> libc::c_int,
    >,
    mut destructor: Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
) -> libc::c_int {
    let mut current_block: u64;
    let mut struct_list: *mut clist = 0 as *mut clist;
    let mut cur_token: size_t = 0;
    let mut value: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut final_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = parser.expect("non-null function pointer")(
        message,
        length,
        &mut cur_token,
        &mut value as *mut *mut libc::c_void as *mut libc::c_void,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        struct_list = clist_new();
        if struct_list.is_null() {
            destructor.expect("non-null function pointer")(value);
            res = MAILIMF_ERROR_MEMORY as libc::c_int
        } else {
            r = clist_insert_after(struct_list, (*struct_list).last, value);
            if r < 0i32 {
                destructor.expect("non-null function pointer")(value);
                res = MAILIMF_ERROR_MEMORY as libc::c_int
            } else {
                final_token = cur_token;
                loop {
                    r = mailimf_unstrict_char_parse(message, length, &mut cur_token, symbol);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        if r == MAILIMF_ERROR_PARSE as libc::c_int {
                            current_block = 9853141518545631134;
                            break;
                        }
                        res = r;
                        current_block = 17524159567010234572;
                        break;
                    } else {
                        r = parser.expect("non-null function pointer")(
                            message,
                            length,
                            &mut cur_token,
                            &mut value as *mut *mut libc::c_void as *mut libc::c_void,
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                                current_block = 9853141518545631134;
                                break;
                            }
                            res = r;
                            current_block = 17524159567010234572;
                            break;
                        } else {
                            r = clist_insert_after(struct_list, (*struct_list).last, value);
                            if r < 0i32 {
                                destructor.expect("non-null function pointer")(value);
                                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                current_block = 17524159567010234572;
                                break;
                            } else {
                                final_token = cur_token
                            }
                        }
                    }
                }
                match current_block {
                    17524159567010234572 => {}
                    _ => {
                        *result = struct_list;
                        *indx = final_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
            }
            clist_foreach(
                struct_list,
                ::std::mem::transmute::<
                    Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
                    clist_func,
                >(destructor),
                0 as *mut libc::c_void,
            );
            clist_free(struct_list);
        }
    }
    return res;
}
unsafe fn mailimf_resent_cc_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_cc,
) -> libc::c_int {
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Resent-Cc\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Resent-Cc\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_address_list_parse(message, length, &mut cur_token, &mut addr_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    cc = mailimf_cc_new(addr_list);
                    if cc.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = cc;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_address_list_free(addr_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_resent_to_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_to,
) -> libc::c_int {
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut to: *mut mailimf_to = 0 as *mut mailimf_to;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Resent-To\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Resent-To\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_address_list_parse(message, length, &mut cur_token, &mut addr_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    to = mailimf_to_new(addr_list);
                    if to.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = to;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_address_list_free(addr_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_resent_sender_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_sender,
) -> libc::c_int {
    let mut mb: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
    let mut sender: *mut mailimf_sender = 0 as *mut mailimf_sender;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = length;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Resent-Sender\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Resent-Sender\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_mailbox_parse(message, length, &mut cur_token, &mut mb);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    sender = mailimf_sender_new(mb);
                    if sender.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = sender;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_mailbox_free(mb);
            }
        }
    }
    return res;
}
unsafe fn mailimf_resent_from_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_from,
) -> libc::c_int {
    let mut mb_list: *mut mailimf_mailbox_list = 0 as *mut mailimf_mailbox_list;
    let mut from: *mut mailimf_from = 0 as *mut mailimf_from;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Resent-From\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Resent-From\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_mailbox_list_parse(message, length, &mut cur_token, &mut mb_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    from = mailimf_from_new(mb_list);
                    if from.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = from;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_mailbox_list_free(mb_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_resent_date_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_orig_date,
) -> libc::c_int {
    let mut orig_date: *mut mailimf_orig_date = 0 as *mut mailimf_orig_date;
    let mut date_time: *mut mailimf_date_time = 0 as *mut mailimf_date_time;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Resent-Date\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Resent-Date\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_date_time_parse(message, length, &mut cur_token, &mut date_time);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    orig_date = mailimf_orig_date_new(date_time);
                    if orig_date.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = orig_date;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_date_time_free(date_time);
            }
        }
    }
    return res;
}
/*
  mailimf_date_time_parse will parse the given RFC 2822 date

  @param message this is a string containing the date
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the date in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_date_time_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_date_time,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut day_of_week: libc::c_int = 0;
    let mut date_time: *mut mailimf_date_time = 0 as *mut mailimf_date_time;
    let mut day: libc::c_int = 0;
    let mut month: libc::c_int = 0;
    let mut year: libc::c_int = 0;
    let mut hour: libc::c_int = 0;
    let mut min: libc::c_int = 0;
    let mut sec: libc::c_int = 0;
    let mut zone: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    day_of_week = -1i32;
    r = mailimf_day_of_week_parse(message, length, &mut cur_token, &mut day_of_week);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        r = mailimf_comma_parse(message, length, &mut cur_token);
        if !(r == MAILIMF_ERROR_PARSE as libc::c_int) {
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
    } else if r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    day = 0i32;
    month = 0i32;
    year = 0i32;
    r = mailimf_date_parse(
        message,
        length,
        &mut cur_token,
        &mut day,
        &mut month,
        &mut year,
    );
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_broken_date_parse(
            message,
            length,
            &mut cur_token,
            &mut day,
            &mut month,
            &mut year,
        )
    } else if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    hour = 0i32;
    min = 0i32;
    sec = 0i32;
    zone = 0i32;
    r = mailimf_time_parse(
        message,
        length,
        &mut cur_token,
        &mut hour,
        &mut min,
        &mut sec,
        &mut zone,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    date_time = mailimf_date_time_new(day, month, year, hour, min, sec, zone);
    if date_time.is_null() {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    *indx = cur_token;
    *result = date_time;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_time_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut phour: *mut libc::c_int,
    mut pmin: *mut libc::c_int,
    mut psec: *mut libc::c_int,
    mut pzone: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut hour: libc::c_int = 0;
    let mut min: libc::c_int = 0;
    let mut sec: libc::c_int = 0;
    let mut zone: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_time_of_day_parse(
        message,
        length,
        &mut cur_token,
        &mut hour,
        &mut min,
        &mut sec,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_zone_parse(message, length, &mut cur_token, &mut zone);
    if !(r == MAILIMF_NO_ERROR as libc::c_int) {
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            zone = 0i32
        } else {
            return r;
        }
    }
    *phour = hour;
    *pmin = min;
    *psec = sec;
    *pzone = zone;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_zone_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut zone: libc::c_int = 0;
    let mut sign: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut value: uint32_t = 0;
    cur_token = *indx;
    if cur_token.wrapping_add(1i32 as libc::size_t) < length {
        if *message.offset(cur_token as isize) as libc::c_int == 'U' as i32
            && *message.offset(cur_token.wrapping_add(1i32 as libc::size_t) as isize) as libc::c_int
                == 'T' as i32
        {
            *result = 1i32;
            *indx = cur_token.wrapping_add(2i32 as libc::size_t);
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    zone = 0i32;
    if cur_token.wrapping_add(2i32 as libc::size_t) < length {
        let mut state: libc::c_int = 0;
        state = STATE_ZONE_1 as libc::c_int;
        while state <= 2i32 {
            match state {
                0 => match *message.offset(cur_token as isize) as libc::c_int {
                    71 => {
                        if *message.offset(cur_token.wrapping_add(1i32 as libc::size_t) as isize)
                            as libc::c_int
                            == 'M' as i32
                            && *message
                                .offset(cur_token.wrapping_add(2i32 as libc::size_t) as isize)
                                as libc::c_int
                                == 'T' as i32
                        {
                            if cur_token.wrapping_add(3i32 as libc::size_t) < length
                                && (*message
                                    .offset(cur_token.wrapping_add(3i32 as libc::size_t) as isize)
                                    as libc::c_int
                                    == '+' as i32
                                    || *message.offset(
                                        cur_token.wrapping_add(3i32 as libc::size_t) as isize
                                    ) as libc::c_int
                                        == '-' as i32)
                            {
                                cur_token = (cur_token as libc::size_t)
                                    .wrapping_add(3i32 as libc::size_t)
                                    as size_t as size_t;
                                state = STATE_ZONE_CONT as libc::c_int
                            } else {
                                zone = 0i32;
                                state = STATE_ZONE_OK as libc::c_int
                            }
                        } else {
                            state = STATE_ZONE_ERR as libc::c_int
                        }
                    }
                    69 => {
                        zone = -5i32;
                        state = STATE_ZONE_2 as libc::c_int
                    }
                    67 => {
                        zone = -6i32;
                        state = STATE_ZONE_2 as libc::c_int
                    }
                    77 => {
                        zone = -7i32;
                        state = STATE_ZONE_2 as libc::c_int
                    }
                    80 => {
                        zone = -8i32;
                        state = STATE_ZONE_2 as libc::c_int
                    }
                    _ => state = STATE_ZONE_CONT as libc::c_int,
                },
                1 => {
                    match *message.offset(cur_token.wrapping_add(1i32 as libc::size_t) as isize)
                        as libc::c_int
                    {
                        83 => state = STATE_ZONE_3 as libc::c_int,
                        68 => {
                            zone += 1;
                            state = STATE_ZONE_3 as libc::c_int
                        }
                        _ => state = STATE_ZONE_ERR as libc::c_int,
                    }
                }
                2 => {
                    if *message.offset(cur_token.wrapping_add(2i32 as libc::size_t) as isize)
                        as libc::c_int
                        == 'T' as i32
                    {
                        zone *= 100i32;
                        state = STATE_ZONE_OK as libc::c_int
                    } else {
                        state = STATE_ZONE_ERR as libc::c_int
                    }
                }
                _ => {}
            }
        }
        match state {
            3 => {
                *result = zone;
                *indx = cur_token.wrapping_add(3i32 as libc::size_t);
                return MAILIMF_NO_ERROR as libc::c_int;
            }
            4 => return MAILIMF_ERROR_PARSE as libc::c_int,
            _ => {}
        }
    }
    sign = 1i32;
    r = mailimf_plus_parse(message, length, &mut cur_token);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        sign = 1i32
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_minus_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            sign = -1i32
        }
    }
    if !(r == MAILIMF_NO_ERROR as libc::c_int) {
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            sign = 1i32
        } else {
            return r;
        }
    }
    r = mailimf_number_parse(message, length, &mut cur_token, &mut value);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    zone = value.wrapping_mul(sign as libc::c_uint) as libc::c_int;
    *indx = cur_token;
    *result = zone;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_number_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut uint32_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut digit: libc::c_int = 0;
    let mut number: uint32_t = 0;
    let mut parsed: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    parsed = 0i32;
    number = 0i32 as uint32_t;
    loop {
        r = mailimf_digit_parse(message, length, &mut cur_token, &mut digit);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                break;
            }
            return r;
        } else {
            number = (number as libc::c_uint).wrapping_mul(10i32 as libc::c_uint) as uint32_t
                as uint32_t;
            number = (number as libc::c_uint).wrapping_add(digit as libc::c_uint) as uint32_t
                as uint32_t;
            parsed = 1i32
        }
    }
    if 0 == parsed {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    *result = number;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_digit_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if 0 != is_digit(*message.offset(cur_token as isize)) {
        *result = *message.offset(cur_token as isize) as libc::c_int - '0' as i32;
        cur_token = cur_token.wrapping_add(1);
        *indx = cur_token;
        return MAILIMF_NO_ERROR as libc::c_int;
    } else {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    };
}
/* *************************************************************** */
#[inline]
unsafe fn is_digit(mut ch: libc::c_char) -> libc::c_int {
    return (ch as libc::c_int >= '0' as i32 && ch as libc::c_int <= '9' as i32) as libc::c_int;
}
unsafe fn mailimf_minus_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, '-' as i32 as libc::c_char);
}
unsafe fn mailimf_plus_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, '+' as i32 as libc::c_char);
}
unsafe fn mailimf_time_of_day_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut phour: *mut libc::c_int,
    mut pmin: *mut libc::c_int,
    mut psec: *mut libc::c_int,
) -> libc::c_int {
    let mut hour: libc::c_int = 0;
    let mut min: libc::c_int = 0;
    let mut sec: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_hour_parse(message, length, &mut cur_token, &mut hour);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_colon_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_minute_parse(message, length, &mut cur_token, &mut min);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_colon_parse(message, length, &mut cur_token);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        r = mailimf_second_parse(message, length, &mut cur_token, &mut sec);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
    } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        sec = 0i32
    } else {
        return r;
    }
    *phour = hour;
    *pmin = min;
    *psec = sec;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_second_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut second: uint32_t = 0;
    let mut r: libc::c_int = 0;
    r = mailimf_number_parse(message, length, indx, &mut second);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *result = second as libc::c_int;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_minute_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut minute: uint32_t = 0;
    let mut r: libc::c_int = 0;
    r = mailimf_number_parse(message, length, indx, &mut minute);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *result = minute as libc::c_int;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_hour_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut hour: uint32_t = 0;
    let mut r: libc::c_int = 0;
    r = mailimf_number_parse(message, length, indx, &mut hour);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *result = hour as libc::c_int;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_broken_date_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut pday: *mut libc::c_int,
    mut pmonth: *mut libc::c_int,
    mut pyear: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut day: libc::c_int = 0;
    let mut month: libc::c_int = 0;
    let mut year: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    month = 1i32;
    r = mailimf_month_parse(message, length, &mut cur_token, &mut month);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    day = 1i32;
    r = mailimf_day_parse(message, length, &mut cur_token, &mut day);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    year = 2001i32;
    r = mailimf_year_parse(message, length, &mut cur_token, &mut year);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *pday = day;
    *pmonth = month;
    *pyear = year;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_year_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut number: uint32_t = 0;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_number_parse(message, length, &mut cur_token, &mut number);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    *result = number as libc::c_int;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_day_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut day: uint32_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_number_parse(message, length, &mut cur_token, &mut day);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *result = day as libc::c_int;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_month_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut month: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_month_name_parse(message, length, &mut cur_token, &mut month);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *result = month;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_month_name_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut month: libc::c_int = 0;
    let mut guessed_month: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    guessed_month = guess_month(message, length, cur_token);
    if guessed_month == -1i32 {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        month_names[(guessed_month - 1i32) as usize].str_0,
        strlen(month_names[(guessed_month - 1i32) as usize].str_0),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    month = guessed_month;
    *result = month;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
month-name      =       "Jan" / "Feb" / "Mar" / "Apr" /
                        "May" / "Jun" / "Jul" / "Aug" /
                        "Sep" / "Oct" / "Nov" / "Dec"
*/
static mut month_names: [mailimf_token_value; 12] = [
    mailimf_token_value {
        value: 1i32,
        str_0: b"Jan\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 2i32,
        str_0: b"Feb\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 3i32,
        str_0: b"Mar\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 4i32,
        str_0: b"Apr\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 5i32,
        str_0: b"May\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 6i32,
        str_0: b"Jun\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 7i32,
        str_0: b"Jul\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 8i32,
        str_0: b"Aug\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 9i32,
        str_0: b"Sep\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 10i32,
        str_0: b"Oct\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 11i32,
        str_0: b"Nov\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 12i32,
        str_0: b"Dec\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
];
unsafe fn guess_month(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: size_t,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    state = MONTH_START as libc::c_int;
    loop {
        if indx >= length {
            return -1i32;
        }
        match state {
            0 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    74 => state = MONTH_J as libc::c_int,
                    70 => return 2i32,
                    77 => state = MONTH_M as libc::c_int,
                    65 => state = MONTH_A as libc::c_int,
                    83 => return 9i32,
                    79 => return 10i32,
                    78 => return 11i32,
                    68 => return 12i32,
                    _ => return -1i32,
                }
            }
            1 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    65 => return 1i32,
                    85 => state = MONTH_JU as libc::c_int,
                    _ => return -1i32,
                }
            }
            2 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    78 => return 6i32,
                    76 => return 7i32,
                    _ => return -1i32,
                }
            }
            3 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    65 => state = MONTH_MA as libc::c_int,
                    _ => return -1i32,
                }
            }
            4 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    89 => return 5i32,
                    82 => return 3i32,
                    _ => return -1i32,
                }
            }
            5 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    80 => return 4i32,
                    85 => return 8i32,
                    _ => return -1i32,
                }
            }
            _ => {}
        }
        indx = indx.wrapping_add(1)
    }
}

unsafe fn mailimf_date_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut pday: *mut libc::c_int,
    mut pmonth: *mut libc::c_int,
    mut pyear: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut day: libc::c_int = 0;
    let mut month: libc::c_int = 0;
    let mut year: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    day = 1i32;
    r = mailimf_day_parse(message, length, &mut cur_token, &mut day);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    month = 1i32;
    r = mailimf_month_parse(message, length, &mut cur_token, &mut month);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    year = 2001i32;
    r = mailimf_year_parse(message, length, &mut cur_token, &mut year);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *pday = day;
    *pmonth = month;
    *pyear = year;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_comma_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, ',' as i32 as libc::c_char);
}
unsafe fn mailimf_day_of_week_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut day_of_week: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_day_name_parse(message, length, &mut cur_token, &mut day_of_week);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    *result = day_of_week;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_day_name_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut day_of_week: libc::c_int = 0;
    let mut guessed_day: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    guessed_day = guess_day_name(message, length, cur_token);
    if guessed_day == -1i32 {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        day_names[(guessed_day - 1i32) as usize].str_0,
        strlen(day_names[(guessed_day - 1i32) as usize].str_0),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    day_of_week = guessed_day;
    *result = day_of_week;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
static mut day_names: [mailimf_token_value; 7] = [
    mailimf_token_value {
        value: 1i32,
        str_0: b"Mon\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 2i32,
        str_0: b"Tue\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 3i32,
        str_0: b"Wed\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 4i32,
        str_0: b"Thu\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 5i32,
        str_0: b"Fri\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 6i32,
        str_0: b"Sat\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
    mailimf_token_value {
        value: 7i32,
        str_0: b"Sun\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    },
];
unsafe fn guess_day_name(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: size_t,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    state = DAY_NAME_START as libc::c_int;
    loop {
        if indx >= length {
            return -1i32;
        }
        match state {
            0 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    77 => return 1i32,
                    84 => state = DAY_NAME_T as libc::c_int,
                    87 => return 3i32,
                    70 => return 5i32,
                    83 => state = DAY_NAME_S as libc::c_int,
                    _ => return -1i32,
                }
            }
            1 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    85 => return 2i32,
                    72 => return 4i32,
                    _ => return -1i32,
                }
            }
            2 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    65 => return 6i32,
                    85 => return 7i32,
                    _ => return -1i32,
                }
            }
            _ => {}
        }
        indx = indx.wrapping_add(1)
    }
}
unsafe fn mailimf_return_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_return,
) -> libc::c_int {
    let mut path: *mut mailimf_path = 0 as *mut mailimf_path;
    let mut return_path: *mut mailimf_return = 0 as *mut mailimf_return;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Return-Path\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Return-Path\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            path = 0 as *mut mailimf_path;
            r = mailimf_path_parse(message, length, &mut cur_token, &mut path);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    return_path = mailimf_return_new(path);
                    if return_path.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = return_path;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_path_free(path);
            }
        }
    }
    return res;
}
unsafe fn mailimf_path_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_path,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut addr_spec: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut path: *mut mailimf_path = 0 as *mut mailimf_path;
    let mut res: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    addr_spec = 0 as *mut libc::c_char;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        r = mailimf_lower_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_addr_spec_parse(message, length, &mut cur_token, &mut addr_spec);
            match r {
                0 => {
                    current_block = 2370887241019905314;
                }
                1 => {
                    r = mailimf_cfws_parse(message, length, &mut cur_token);
                    if r != MAILIMF_NO_ERROR as libc::c_int
                        && r != MAILIMF_ERROR_PARSE as libc::c_int
                    {
                        res = r;
                        current_block = 14973541214802992465;
                    } else {
                        current_block = 2370887241019905314;
                    }
                }
                _ => return r,
            }
            match current_block {
                14973541214802992465 => {}
                _ => {
                    r = mailimf_greater_parse(message, length, &mut cur_token);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        res = r
                    } else {
                        path = mailimf_path_new(addr_spec);
                        if path.is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                            if addr_spec.is_null() {
                                mailimf_addr_spec_free(addr_spec);
                            }
                        } else {
                            *indx = cur_token;
                            *result = path;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                }
            }
        }
    }
    return res;
}
unsafe fn mailimf_keywords_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_keywords,
) -> libc::c_int {
    let mut keywords: *mut mailimf_keywords = 0 as *mut mailimf_keywords;
    let mut list: *mut clist = 0 as *mut clist;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Keywords\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Keywords\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_struct_list_parse(
                message,
                length,
                &mut cur_token,
                &mut list,
                ',' as i32 as libc::c_char,
                ::std::mem::transmute::<
                    Option<
                        unsafe fn(
                            _: *const libc::c_char,
                            _: size_t,
                            _: *mut size_t,
                            _: *mut *mut libc::c_char,
                        ) -> libc::c_int,
                    >,
                    Option<
                        unsafe fn(
                            _: *const libc::c_char,
                            _: size_t,
                            _: *mut size_t,
                            _: *mut libc::c_void,
                        ) -> libc::c_int,
                    >,
                >(Some(mailimf_phrase_parse)),
                ::std::mem::transmute::<
                    Option<unsafe fn(_: *mut libc::c_char) -> ()>,
                    Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
                >(Some(mailimf_phrase_free)),
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    keywords = mailimf_keywords_new(list);
                    if keywords.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = keywords;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                clist_foreach(
                    list,
                    ::std::mem::transmute::<
                        Option<unsafe fn(_: *mut libc::c_char) -> ()>,
                        clist_func,
                    >(Some(mailimf_phrase_free)),
                    0 as *mut libc::c_void,
                );
                clist_free(list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_comments_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_comments,
) -> libc::c_int {
    let mut comments: *mut mailimf_comments = 0 as *mut mailimf_comments;
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Comments\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Comments\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_unstructured_parse(message, length, &mut cur_token, &mut value);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    comments = mailimf_comments_new(value);
                    if comments.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = comments;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_unstructured_free(value);
            }
        }
    }
    return res;
}
unsafe fn mailimf_subject_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_subject,
) -> libc::c_int {
    let mut subject: *mut mailimf_subject = 0 as *mut mailimf_subject;
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Subject\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Subject\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_unstructured_parse(message, length, &mut cur_token, &mut value);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    subject = mailimf_subject_new(value);
                    if subject.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = subject;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_unstructured_free(value);
            }
        }
    }
    return res;
}

/* exported for IMAP */
pub unsafe fn mailimf_references_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_references,
) -> libc::c_int {
    let mut references: *mut mailimf_references = 0 as *mut mailimf_references;
    let mut cur_token: size_t = 0;
    let mut msg_id_list: *mut clist = 0 as *mut clist;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"References\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"References\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_msg_id_list_parse(message, length, &mut cur_token, &mut msg_id_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    references = mailimf_references_new(msg_id_list);
                    if references.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = references;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                clist_foreach(
                    msg_id_list,
                    ::std::mem::transmute::<
                        Option<unsafe fn(_: *mut libc::c_char) -> ()>,
                        clist_func,
                    >(Some(mailimf_msg_id_free)),
                    0 as *mut libc::c_void,
                );
                clist_free(msg_id_list);
            }
        }
    }
    return res;
}

pub unsafe fn mailimf_msg_id_list_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut clist,
) -> libc::c_int {
    return mailimf_struct_multiple_parse(
        message,
        length,
        indx,
        result,
        ::std::mem::transmute::<
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut *mut libc::c_char,
                ) -> libc::c_int,
            >,
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut libc::c_void,
                ) -> libc::c_int,
            >,
        >(Some(mailimf_unstrict_msg_id_parse)),
        ::std::mem::transmute::<
            Option<unsafe fn(_: *mut libc::c_char) -> ()>,
            Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
        >(Some(mailimf_msg_id_free)),
    );
}
unsafe fn mailimf_unstrict_msg_id_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut msgid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_parse_unwanted_msg_id(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_msg_id_parse(message, length, &mut cur_token, &mut msgid);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_parse_unwanted_msg_id(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        free(msgid as *mut libc::c_void);
        return r;
    }
    *result = msgid;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_parse_unwanted_msg_id(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut word: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut token_parsed: libc::c_int = 0;
    cur_token = *indx;
    token_parsed = 1i32;
    while 0 != token_parsed {
        token_parsed = 0i32;
        r = mailimf_word_parse(message, length, &mut cur_token, &mut word);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            mailimf_word_free(word);
            token_parsed = 1i32
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
        r = mailimf_semi_colon_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            token_parsed = 1i32
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
        r = mailimf_comma_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            token_parsed = 1i32
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
        r = mailimf_plus_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            token_parsed = 1i32
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            token_parsed = 1i32
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
        r = mailimf_point_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            token_parsed = 1i32
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
        r = mailimf_at_sign_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            token_parsed = 1i32
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_at_sign_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, '@' as i32 as libc::c_char);
}
unsafe fn mailimf_point_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    return mailimf_unstrict_char_parse(message, length, indx, '.' as i32 as libc::c_char);
}

pub unsafe fn mailimf_word_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut word: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_atom_parse(message, length, &mut cur_token, &mut word);
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_quoted_string_parse(message, length, &mut cur_token, &mut word)
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *result = word;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_quoted_string_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut gstr: *mut MMAPString = 0 as *mut MMAPString;
    let mut ch: libc::c_char = 0;
    let mut str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        r = mailimf_dquote_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            gstr = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
            if gstr.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int
            } else {
                loop {
                    r = mailimf_fws_parse(message, length, &mut cur_token);
                    if r == MAILIMF_NO_ERROR as libc::c_int {
                        if mmap_string_append_c(gstr, ' ' as i32 as libc::c_char).is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                            current_block = 14861901777624095282;
                            break;
                        }
                    } else if r != MAILIMF_ERROR_PARSE as libc::c_int {
                        res = r;
                        current_block = 14861901777624095282;
                        break;
                    }
                    r = mailimf_qcontent_parse(message, length, &mut cur_token, &mut ch);
                    if r == MAILIMF_NO_ERROR as libc::c_int {
                        if !mmap_string_append_c(gstr, ch).is_null() {
                            continue;
                        }
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 14861901777624095282;
                        break;
                    } else {
                        if r == MAILIMF_ERROR_PARSE as libc::c_int {
                            current_block = 5494826135382683477;
                            break;
                        }
                        res = r;
                        current_block = 14861901777624095282;
                        break;
                    }
                }
                match current_block {
                    5494826135382683477 => {
                        r = mailimf_dquote_parse(message, length, &mut cur_token);
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r
                        } else {
                            str = strdup((*gstr).str_0);
                            if str.is_null() {
                                res = MAILIMF_ERROR_MEMORY as libc::c_int
                            } else {
                                mmap_string_free(gstr);
                                *indx = cur_token;
                                *result = str;
                                return MAILIMF_NO_ERROR as libc::c_int;
                            }
                        }
                    }
                    _ => {}
                }
                mmap_string_free(gstr);
            }
        }
    }
    return res;
}

pub unsafe fn mailimf_atom_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut atom: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut end: size_t = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        end = cur_token;
        if end >= length {
            res = MAILIMF_ERROR_PARSE as libc::c_int
        } else {
            while 0 != is_atext(*message.offset(end as isize)) {
                end = end.wrapping_add(1);
                if end >= length {
                    break;
                }
            }
            if end == cur_token {
                res = MAILIMF_ERROR_PARSE as libc::c_int
            } else {
                atom = malloc(
                    end.wrapping_sub(cur_token)
                        .wrapping_add(1i32 as libc::size_t),
                ) as *mut libc::c_char;
                if atom.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int
                } else {
                    strncpy(
                        atom,
                        message.offset(cur_token as isize),
                        end.wrapping_sub(cur_token),
                    );
                    *atom.offset(end.wrapping_sub(cur_token) as isize) =
                        '\u{0}' as i32 as libc::c_char;
                    cur_token = end;
                    *indx = cur_token;
                    *result = atom;
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
        }
    }
    return res;
}
unsafe fn mailimf_struct_multiple_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut clist,
    mut parser: Option<
        unsafe fn(
            _: *const libc::c_char,
            _: size_t,
            _: *mut size_t,
            _: *mut libc::c_void,
        ) -> libc::c_int,
    >,
    mut destructor: Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
) -> libc::c_int {
    let mut current_block: u64;
    let mut struct_list: *mut clist = 0 as *mut clist;
    let mut cur_token: size_t = 0;
    let mut value: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = parser.expect("non-null function pointer")(
        message,
        length,
        &mut cur_token,
        &mut value as *mut *mut libc::c_void as *mut libc::c_void,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        struct_list = clist_new();
        if struct_list.is_null() {
            destructor.expect("non-null function pointer")(value);
            res = MAILIMF_ERROR_MEMORY as libc::c_int
        } else {
            r = clist_insert_after(struct_list, (*struct_list).last, value);
            if r < 0i32 {
                destructor.expect("non-null function pointer")(value);
                res = MAILIMF_ERROR_MEMORY as libc::c_int
            } else {
                loop {
                    r = parser.expect("non-null function pointer")(
                        message,
                        length,
                        &mut cur_token,
                        &mut value as *mut *mut libc::c_void as *mut libc::c_void,
                    );
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        if r == MAILIMF_ERROR_PARSE as libc::c_int {
                            current_block = 11057878835866523405;
                            break;
                        }
                        res = r;
                        current_block = 8222683242185098763;
                        break;
                    } else {
                        r = clist_insert_after(struct_list, (*struct_list).last, value);
                        if !(r < 0i32) {
                            continue;
                        }
                        destructor.expect("non-null function pointer")(value);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 8222683242185098763;
                        break;
                    }
                }
                match current_block {
                    8222683242185098763 => {}
                    _ => {
                        *result = struct_list;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
            }
            clist_foreach(
                struct_list,
                ::std::mem::transmute::<
                    Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
                    clist_func,
                >(destructor),
                0 as *mut libc::c_void,
            );
            clist_free(struct_list);
        }
    }
    return res;
}
unsafe fn mailimf_in_reply_to_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_in_reply_to,
) -> libc::c_int {
    let mut in_reply_to: *mut mailimf_in_reply_to = 0 as *mut mailimf_in_reply_to;
    let mut cur_token: size_t = 0;
    let mut msg_id_list: *mut clist = 0 as *mut clist;
    let mut res: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"In-Reply-To\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"In-Reply-To\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_msg_id_list_parse(message, length, &mut cur_token, &mut msg_id_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    in_reply_to = mailimf_in_reply_to_new(msg_id_list);
                    if in_reply_to.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = in_reply_to;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                clist_foreach(
                    msg_id_list,
                    ::std::mem::transmute::<
                        Option<unsafe fn(_: *mut libc::c_char) -> ()>,
                        clist_func,
                    >(Some(mailimf_msg_id_free)),
                    0 as *mut libc::c_void,
                );
                clist_free(msg_id_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_message_id_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_message_id,
) -> libc::c_int {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut message_id: *mut mailimf_message_id = 0 as *mut mailimf_message_id;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Message-ID\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Message-ID\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_msg_id_parse(message, length, &mut cur_token, &mut value);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    message_id = mailimf_message_id_new(value);
                    if message_id.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = message_id;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_msg_id_free(value);
            }
        }
    }
    return res;
}
unsafe fn mailimf_bcc_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_bcc,
) -> libc::c_int {
    let mut current_block: u64;
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut bcc: *mut mailimf_bcc = 0 as *mut mailimf_bcc;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    addr_list = 0 as *mut mailimf_address_list;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Bcc\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Bcc\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_address_list_parse(message, length, &mut cur_token, &mut addr_list);
            match r {
                0 => {
                    /* do nothing */
                    current_block = 2838571290723028321;
                }
                1 => {
                    r = mailimf_cfws_parse(message, length, &mut cur_token);
                    if r != MAILIMF_NO_ERROR as libc::c_int
                        && r != MAILIMF_ERROR_PARSE as libc::c_int
                    {
                        res = r;
                        current_block = 15260376711225273221;
                    } else {
                        current_block = 2838571290723028321;
                    }
                }
                _ => {
                    res = r;
                    current_block = 15260376711225273221;
                }
            }
            match current_block {
                15260376711225273221 => {}
                _ => {
                    r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        res = r
                    } else {
                        bcc = mailimf_bcc_new(addr_list);
                        if bcc.is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int
                        } else {
                            *result = bcc;
                            *indx = cur_token;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                }
            }
        }
    }
    if !addr_list.is_null() {
        mailimf_address_list_free(addr_list);
    }
    return res;
}
unsafe fn mailimf_cc_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_cc,
) -> libc::c_int {
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Cc\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Cc\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_address_list_parse(message, length, &mut cur_token, &mut addr_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    cc = mailimf_cc_new(addr_list);
                    if cc.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = cc;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_address_list_free(addr_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_to_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_to,
) -> libc::c_int {
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut to: *mut mailimf_to = 0 as *mut mailimf_to;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"To\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"To\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_address_list_parse(message, length, &mut cur_token, &mut addr_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    to = mailimf_to_new(addr_list);
                    if to.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = to;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_address_list_free(addr_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_reply_to_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_reply_to,
) -> libc::c_int {
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    let mut reply_to: *mut mailimf_reply_to = 0 as *mut mailimf_reply_to;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Reply-To\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Reply-To\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_address_list_parse(message, length, &mut cur_token, &mut addr_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    reply_to = mailimf_reply_to_new(addr_list);
                    if reply_to.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = reply_to;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_address_list_free(addr_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_sender_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_sender,
) -> libc::c_int {
    let mut mb: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
    let mut sender: *mut mailimf_sender = 0 as *mut mailimf_sender;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Sender\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Sender\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_mailbox_parse(message, length, &mut cur_token, &mut mb);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    sender = mailimf_sender_new(mb);
                    if sender.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = sender;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_mailbox_free(mb);
            }
        }
    }
    return res;
}
unsafe fn mailimf_from_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_from,
) -> libc::c_int {
    let mut mb_list: *mut mailimf_mailbox_list = 0 as *mut mailimf_mailbox_list;
    let mut from: *mut mailimf_from = 0 as *mut mailimf_from;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"From\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"From\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_colon_parse(message, length, &mut cur_token);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_mailbox_list_parse(message, length, &mut cur_token, &mut mb_list);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    from = mailimf_from_new(mb_list);
                    if from.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = from;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                mailimf_mailbox_list_free(mb_list);
            }
        }
    }
    return res;
}
unsafe fn mailimf_orig_date_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_orig_date,
) -> libc::c_int {
    let mut date_time: *mut mailimf_date_time = 0 as *mut mailimf_date_time;
    let mut orig_date: *mut mailimf_orig_date = 0 as *mut mailimf_orig_date;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"Date:\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"Date:\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_date_time_parse(message, length, &mut cur_token, &mut date_time);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_ignore_unstructured_parse(message, length, &mut cur_token);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                res = r
            } else {
                r = mailimf_unstrict_crlf_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    orig_date = mailimf_orig_date_new(date_time);
                    if orig_date.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = orig_date;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
            }
            mailimf_date_time_free(date_time);
        }
    }
    return res;
}
unsafe fn mailimf_ignore_unstructured_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut state: libc::c_int = 0;
    let mut terminal: size_t = 0;
    cur_token = *indx;
    state = UNSTRUCTURED_START as libc::c_int;
    terminal = cur_token;
    while state != UNSTRUCTURED_OUT as libc::c_int {
        match state {
            0 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                terminal = cur_token;
                match *message.offset(cur_token as isize) as libc::c_int {
                    13 => state = UNSTRUCTURED_CR as libc::c_int,
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            1 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            2 => {
                if cur_token >= length {
                    state = UNSTRUCTURED_OUT as libc::c_int
                } else {
                    match *message.offset(cur_token as isize) as libc::c_int {
                        9 | 32 => state = UNSTRUCTURED_WSP as libc::c_int,
                        _ => state = UNSTRUCTURED_OUT as libc::c_int,
                    }
                }
            }
            3 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    13 => state = UNSTRUCTURED_CR as libc::c_int,
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            _ => {}
        }
        cur_token = cur_token.wrapping_add(1)
    }
    *indx = terminal;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn guess_header_type(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: size_t,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    state = HEADER_START as libc::c_int;
    loop {
        if indx >= length {
            return MAILIMF_FIELD_NONE as libc::c_int;
        }
        match state {
            0 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    66 => return MAILIMF_FIELD_BCC as libc::c_int,
                    67 => state = HEADER_C as libc::c_int,
                    68 => return MAILIMF_FIELD_ORIG_DATE as libc::c_int,
                    70 => return MAILIMF_FIELD_FROM as libc::c_int,
                    73 => return MAILIMF_FIELD_IN_REPLY_TO as libc::c_int,
                    75 => return MAILIMF_FIELD_KEYWORDS as libc::c_int,
                    77 => return MAILIMF_FIELD_MESSAGE_ID as libc::c_int,
                    82 => state = HEADER_R as libc::c_int,
                    84 => return MAILIMF_FIELD_TO as libc::c_int,
                    83 => state = HEADER_S as libc::c_int,
                    _ => return MAILIMF_FIELD_NONE as libc::c_int,
                }
            }
            1 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    79 => return MAILIMF_FIELD_COMMENTS as libc::c_int,
                    67 => return MAILIMF_FIELD_CC as libc::c_int,
                    _ => return MAILIMF_FIELD_NONE as libc::c_int,
                }
            }
            2 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    69 => state = HEADER_RE as libc::c_int,
                    _ => return MAILIMF_FIELD_NONE as libc::c_int,
                }
            }
            3 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    70 => return MAILIMF_FIELD_REFERENCES as libc::c_int,
                    80 => return MAILIMF_FIELD_REPLY_TO as libc::c_int,
                    83 => state = HEADER_RES as libc::c_int,
                    84 => return MAILIMF_FIELD_RETURN_PATH as libc::c_int,
                    _ => return MAILIMF_FIELD_NONE as libc::c_int,
                }
            }
            4 => {
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    69 => return MAILIMF_FIELD_SENDER as libc::c_int,
                    85 => return MAILIMF_FIELD_SUBJECT as libc::c_int,
                    _ => return MAILIMF_FIELD_NONE as libc::c_int,
                }
            }
            5 => {
                r = mailimf_token_case_insensitive_len_parse(
                    message,
                    length,
                    &mut indx,
                    b"ent-\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                    strlen(b"ent-\x00" as *const u8 as *const libc::c_char),
                );
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    return MAILIMF_FIELD_NONE as libc::c_int;
                }
                if indx >= length {
                    return MAILIMF_FIELD_NONE as libc::c_int;
                }
                match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int)
                    as libc::c_char as libc::c_int
                {
                    68 => return MAILIMF_FIELD_RESENT_DATE as libc::c_int,
                    70 => return MAILIMF_FIELD_RESENT_FROM as libc::c_int,
                    83 => return MAILIMF_FIELD_RESENT_SENDER as libc::c_int,
                    84 => return MAILIMF_FIELD_RESENT_TO as libc::c_int,
                    67 => return MAILIMF_FIELD_RESENT_CC as libc::c_int,
                    66 => return MAILIMF_FIELD_RESENT_BCC as libc::c_int,
                    77 => return MAILIMF_FIELD_RESENT_MSG_ID as libc::c_int,
                    _ => return MAILIMF_FIELD_NONE as libc::c_int,
                }
            }
            _ => {}
        }
        indx = indx.wrapping_add(1)
    }
}
/*
  mailimf_envelope_fields_parse will parse the given fields (Date,
  From, Sender, Reply-To, To, Cc, Bcc, Message-ID, In-Reply-To,
  References and Subject)

  @param message this is a string containing the header fields
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the header fields in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/

pub unsafe fn mailimf_envelope_fields_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_fields,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    list = clist_new();
    if list.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        loop {
            let mut elt: *mut mailimf_field = 0 as *mut mailimf_field;
            r = mailimf_envelope_field_parse(message, length, &mut cur_token, &mut elt);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                r = clist_insert_after(list, (*list).last, elt as *mut libc::c_void);
                if !(r < 0i32) {
                    continue;
                }
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 894413572976700158;
                break;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                r = mailimf_ignore_field_parse(message, length, &mut cur_token);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    continue;
                }
                /* do nothing */
                if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 2719512138335094285;
                    break;
                }
                res = r;
                current_block = 894413572976700158;
                break;
            } else {
                res = r;
                current_block = 894413572976700158;
                break;
            }
        }
        match current_block {
            2719512138335094285 => {
                fields = mailimf_fields_new(list);
                if fields.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int
                } else {
                    *result = fields;
                    *indx = cur_token;
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
            _ => {}
        }
        if !list.is_null() {
            clist_foreach(
                list,
                ::std::mem::transmute::<Option<unsafe fn(_: *mut mailimf_field) -> ()>, clist_func>(
                    Some(mailimf_field_free),
                ),
                0 as *mut libc::c_void,
            );
            clist_free(list);
        }
    }
    return res;
}
/*
  mailimf_ignore_field_parse will skip the given field

  @param message this is a string containing the header field
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the header field in
    the given string, (* indx) is modified to point at the end
    of the parsed data

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_ignore_field_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut has_field: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut state: libc::c_int = 0;
    let mut terminal: size_t = 0;
    has_field = 0i32;
    cur_token = *indx;
    terminal = cur_token;
    state = UNSTRUCTURED_START as libc::c_int;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    match *message.offset(cur_token as isize) as libc::c_int {
        13 => return MAILIMF_ERROR_PARSE as libc::c_int,
        10 => return MAILIMF_ERROR_PARSE as libc::c_int,
        _ => {}
    }
    while state != UNSTRUCTURED_OUT as libc::c_int {
        match state {
            0 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    13 => state = UNSTRUCTURED_CR as libc::c_int,
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    58 => {
                        has_field = 1i32;
                        state = UNSTRUCTURED_START as libc::c_int
                    }
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            1 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    58 => {
                        has_field = 1i32;
                        state = UNSTRUCTURED_START as libc::c_int
                    }
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            2 => {
                if cur_token >= length {
                    terminal = cur_token;
                    state = UNSTRUCTURED_OUT as libc::c_int
                } else {
                    match *message.offset(cur_token as isize) as libc::c_int {
                        9 | 32 => state = UNSTRUCTURED_WSP as libc::c_int,
                        _ => {
                            terminal = cur_token;
                            state = UNSTRUCTURED_OUT as libc::c_int
                        }
                    }
                }
            }
            3 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    13 => state = UNSTRUCTURED_CR as libc::c_int,
                    10 => state = UNSTRUCTURED_LF as libc::c_int,
                    58 => {
                        has_field = 1i32;
                        state = UNSTRUCTURED_START as libc::c_int
                    }
                    _ => state = UNSTRUCTURED_START as libc::c_int,
                }
            }
            _ => {}
        }
        cur_token = cur_token.wrapping_add(1)
    }
    if 0 == has_field {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    *indx = terminal;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
static int mailimf_ftext_parse(const char * message, size_t length,
                    size_t * indx, gchar * result)
{
  return mailimf_typed_text_parse(message, length, indx, result, is_ftext);
}
*/
unsafe fn mailimf_envelope_field_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_field,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut type_0: libc::c_int = 0;
    let mut orig_date: *mut mailimf_orig_date = 0 as *mut mailimf_orig_date;
    let mut from: *mut mailimf_from = 0 as *mut mailimf_from;
    let mut sender: *mut mailimf_sender = 0 as *mut mailimf_sender;
    let mut reply_to: *mut mailimf_reply_to = 0 as *mut mailimf_reply_to;
    let mut to: *mut mailimf_to = 0 as *mut mailimf_to;
    let mut cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
    let mut bcc: *mut mailimf_bcc = 0 as *mut mailimf_bcc;
    let mut message_id: *mut mailimf_message_id = 0 as *mut mailimf_message_id;
    let mut in_reply_to: *mut mailimf_in_reply_to = 0 as *mut mailimf_in_reply_to;
    let mut references: *mut mailimf_references = 0 as *mut mailimf_references;
    let mut subject: *mut mailimf_subject = 0 as *mut mailimf_subject;
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    let mut guessed_type: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    orig_date = 0 as *mut mailimf_orig_date;
    from = 0 as *mut mailimf_from;
    sender = 0 as *mut mailimf_sender;
    reply_to = 0 as *mut mailimf_reply_to;
    to = 0 as *mut mailimf_to;
    cc = 0 as *mut mailimf_cc;
    bcc = 0 as *mut mailimf_bcc;
    message_id = 0 as *mut mailimf_message_id;
    in_reply_to = 0 as *mut mailimf_in_reply_to;
    references = 0 as *mut mailimf_references;
    subject = 0 as *mut mailimf_subject;
    optional_field = 0 as *mut mailimf_optional_field;
    guessed_type = guess_header_type(message, length, cur_token);
    type_0 = MAILIMF_FIELD_NONE as libc::c_int;
    match guessed_type {
        9 => {
            r = mailimf_orig_date_parse(message, length, &mut cur_token, &mut orig_date);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        10 => {
            r = mailimf_from_parse(message, length, &mut cur_token, &mut from);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        11 => {
            r = mailimf_sender_parse(message, length, &mut cur_token, &mut sender);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        12 => {
            r = mailimf_reply_to_parse(message, length, &mut cur_token, &mut reply_to);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        13 => {
            r = mailimf_to_parse(message, length, &mut cur_token, &mut to);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        14 => {
            r = mailimf_cc_parse(message, length, &mut cur_token, &mut cc);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        15 => {
            r = mailimf_bcc_parse(message, length, &mut cur_token, &mut bcc);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        16 => {
            r = mailimf_message_id_parse(message, length, &mut cur_token, &mut message_id);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        17 => {
            r = mailimf_in_reply_to_parse(message, length, &mut cur_token, &mut in_reply_to);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        18 => {
            r = mailimf_references_parse(message, length, &mut cur_token, &mut references);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        19 => {
            r = mailimf_subject_parse(message, length, &mut cur_token, &mut subject);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = guessed_type;
                current_block = 4488496028633655612;
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                current_block = 4488496028633655612;
            } else {
                /* do nothing */
                res = r;
                current_block = 15605017197726313499;
            }
        }
        _ => {
            current_block = 4488496028633655612;
        }
    }
    match current_block {
        4488496028633655612 => {
            if type_0 == MAILIMF_FIELD_NONE as libc::c_int {
                res = MAILIMF_ERROR_PARSE as libc::c_int
            } else {
                field = mailimf_field_new(
                    type_0,
                    0 as *mut mailimf_return,
                    0 as *mut mailimf_orig_date,
                    0 as *mut mailimf_from,
                    0 as *mut mailimf_sender,
                    0 as *mut mailimf_to,
                    0 as *mut mailimf_cc,
                    0 as *mut mailimf_bcc,
                    0 as *mut mailimf_message_id,
                    orig_date,
                    from,
                    sender,
                    reply_to,
                    to,
                    cc,
                    bcc,
                    message_id,
                    in_reply_to,
                    references,
                    subject,
                    0 as *mut mailimf_comments,
                    0 as *mut mailimf_keywords,
                    optional_field,
                );
                if field.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                    if !orig_date.is_null() {
                        mailimf_orig_date_free(orig_date);
                    }
                    if !from.is_null() {
                        mailimf_from_free(from);
                    }
                    if !sender.is_null() {
                        mailimf_sender_free(sender);
                    }
                    if !reply_to.is_null() {
                        mailimf_reply_to_free(reply_to);
                    }
                    if !to.is_null() {
                        mailimf_to_free(to);
                    }
                    if !cc.is_null() {
                        mailimf_cc_free(cc);
                    }
                    if !bcc.is_null() {
                        mailimf_bcc_free(bcc);
                    }
                    if !message_id.is_null() {
                        mailimf_message_id_free(message_id);
                    }
                    if !in_reply_to.is_null() {
                        mailimf_in_reply_to_free(in_reply_to);
                    }
                    if !references.is_null() {
                        mailimf_references_free(references);
                    }
                    if !subject.is_null() {
                        mailimf_subject_free(subject);
                    }
                    if !optional_field.is_null() {
                        mailimf_optional_field_free(optional_field);
                    }
                } else {
                    *result = field;
                    *indx = cur_token;
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
        }
        _ => {}
    }
    return res;
}
/*
  mailimf_envelope_fields will parse the given fields (Date,
  From, Sender, Reply-To, To, Cc, Bcc, Message-ID, In-Reply-To,
  References and Subject), other fields will be added as optional
  fields.

  @param message this is a string containing the header fields
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the header fields in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_envelope_and_optional_fields_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_fields,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    list = 0 as *mut clist;
    r = mailimf_struct_multiple_parse(
        message,
        length,
        &mut cur_token,
        &mut list,
        ::std::mem::transmute::<
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut *mut mailimf_field,
                ) -> libc::c_int,
            >,
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut libc::c_void,
                ) -> libc::c_int,
            >,
        >(Some(mailimf_envelope_or_optional_field_parse)),
        ::std::mem::transmute::<
            Option<unsafe fn(_: *mut mailimf_field) -> ()>,
            Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
        >(Some(mailimf_field_free)),
    );
    match r {
        0 => {
            /* do nothing */
            current_block = 11050875288958768710;
        }
        1 => {
            list = clist_new();
            if list.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 7755940856643933760;
            } else {
                current_block = 11050875288958768710;
            }
        }
        _ => {
            res = r;
            current_block = 7755940856643933760;
        }
    }
    match current_block {
        11050875288958768710 => {
            fields = mailimf_fields_new(list);
            if fields.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                if !list.is_null() {
                    clist_foreach(
                        list,
                        ::std::mem::transmute::<
                            Option<unsafe fn(_: *mut mailimf_field) -> ()>,
                            clist_func,
                        >(Some(mailimf_field_free)),
                        0 as *mut libc::c_void,
                    );
                    clist_free(list);
                }
            } else {
                *result = fields;
                *indx = cur_token;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
        _ => {}
    }
    return res;
}
unsafe fn mailimf_envelope_or_optional_field_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_field,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    r = mailimf_envelope_field_parse(message, length, indx, result);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        return MAILIMF_NO_ERROR as libc::c_int;
    }
    cur_token = *indx;
    r = mailimf_optional_field_parse(message, length, &mut cur_token, &mut optional_field);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    field = mailimf_field_new(
        MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int,
        0 as *mut mailimf_return,
        0 as *mut mailimf_orig_date,
        0 as *mut mailimf_from,
        0 as *mut mailimf_sender,
        0 as *mut mailimf_to,
        0 as *mut mailimf_cc,
        0 as *mut mailimf_bcc,
        0 as *mut mailimf_message_id,
        0 as *mut mailimf_orig_date,
        0 as *mut mailimf_from,
        0 as *mut mailimf_sender,
        0 as *mut mailimf_reply_to,
        0 as *mut mailimf_to,
        0 as *mut mailimf_cc,
        0 as *mut mailimf_bcc,
        0 as *mut mailimf_message_id,
        0 as *mut mailimf_in_reply_to,
        0 as *mut mailimf_references,
        0 as *mut mailimf_subject,
        0 as *mut mailimf_comments,
        0 as *mut mailimf_keywords,
        optional_field,
    );
    if field.is_null() {
        mailimf_optional_field_free(optional_field);
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    *result = field;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

/*
  mailimf_envelope_fields will parse the given fields as optional
  fields.

  @param message this is a string containing the header fields
  @param length this is the size of the given string
  @param indx this is a pointer to the start of the header fields in
    the given string, (* indx) is modified to point at the end
    of the parsed data
  @param result the result of the parse operation is stored in
    (* result)

  @return MAILIMF_NO_ERROR on success, MAILIMF_ERROR_XXX on error
*/
pub unsafe fn mailimf_optional_fields_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_fields,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    list = 0 as *mut clist;
    r = mailimf_struct_multiple_parse(
        message,
        length,
        &mut cur_token,
        &mut list,
        ::std::mem::transmute::<
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut *mut mailimf_field,
                ) -> libc::c_int,
            >,
            Option<
                unsafe fn(
                    _: *const libc::c_char,
                    _: size_t,
                    _: *mut size_t,
                    _: *mut libc::c_void,
                ) -> libc::c_int,
            >,
        >(Some(mailimf_only_optional_field_parse)),
        ::std::mem::transmute::<
            Option<unsafe fn(_: *mut mailimf_field) -> ()>,
            Option<unsafe fn(_: *mut libc::c_void) -> libc::c_int>,
        >(Some(mailimf_field_free)),
    );
    match r {
        0 => {
            /* do nothing */
            current_block = 11050875288958768710;
        }
        1 => {
            list = clist_new();
            if list.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 4409055091581443388;
            } else {
                current_block = 11050875288958768710;
            }
        }
        _ => {
            res = r;
            current_block = 4409055091581443388;
        }
    }
    match current_block {
        11050875288958768710 => {
            fields = mailimf_fields_new(list);
            if fields.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                if !list.is_null() {
                    clist_foreach(
                        list,
                        ::std::mem::transmute::<
                            Option<unsafe fn(_: *mut mailimf_field) -> ()>,
                            clist_func,
                        >(Some(mailimf_field_free)),
                        0 as *mut libc::c_void,
                    );
                    clist_free(list);
                }
            } else {
                *result = fields;
                *indx = cur_token;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
        _ => {}
    }
    return res;
}
unsafe fn mailimf_only_optional_field_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailimf_field,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    cur_token = *indx;
    r = mailimf_optional_field_parse(message, length, &mut cur_token, &mut optional_field);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    field = mailimf_field_new(
        MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int,
        0 as *mut mailimf_return,
        0 as *mut mailimf_orig_date,
        0 as *mut mailimf_from,
        0 as *mut mailimf_sender,
        0 as *mut mailimf_to,
        0 as *mut mailimf_cc,
        0 as *mut mailimf_bcc,
        0 as *mut mailimf_message_id,
        0 as *mut mailimf_orig_date,
        0 as *mut mailimf_from,
        0 as *mut mailimf_sender,
        0 as *mut mailimf_reply_to,
        0 as *mut mailimf_to,
        0 as *mut mailimf_cc,
        0 as *mut mailimf_bcc,
        0 as *mut mailimf_message_id,
        0 as *mut mailimf_in_reply_to,
        0 as *mut mailimf_references,
        0 as *mut mailimf_subject,
        0 as *mut mailimf_comments,
        0 as *mut mailimf_keywords,
        optional_field,
    );
    if field.is_null() {
        mailimf_optional_field_free(optional_field);
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    *result = field;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_custom_string_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut is_custom_char: Option<unsafe fn(_: libc::c_char) -> libc::c_int>,
) -> libc::c_int {
    let mut begin: size_t = 0;
    let mut end: size_t = 0;
    let mut gstr: *mut libc::c_char = 0 as *mut libc::c_char;
    begin = *indx;
    end = begin;
    if end >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    while 0 != is_custom_char.expect("non-null function pointer")(*message.offset(end as isize)) {
        end = end.wrapping_add(1);
        if end >= length {
            break;
        }
    }
    if end != begin {
        gstr =
            malloc(end.wrapping_sub(begin).wrapping_add(1i32 as libc::size_t)) as *mut libc::c_char;
        if gstr.is_null() {
            return MAILIMF_ERROR_MEMORY as libc::c_int;
        }
        strncpy(
            gstr,
            message.offset(begin as isize),
            end.wrapping_sub(begin),
        );
        *gstr.offset(end.wrapping_sub(begin) as isize) = '\u{0}' as i32 as libc::c_char;
        *indx = end;
        *result = gstr;
        return MAILIMF_NO_ERROR as libc::c_int;
    } else {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    };
}

use chrono::{Datelike, Local, TimeZone, Timelike};

use crate::clist::*;
use crate::mailimf_types::*;
use crate::mailimf_types_helper::*;
use crate::mailmime_types::*;
use crate::mailmime_types_helper::*;

pub(crate) use libc::{
    calloc, close, free, isalpha, isdigit, malloc, memcmp, memcpy, memmove, memset, realloc,
    strcpy, strlen, strncmp, strncpy,
};

pub(crate) unsafe fn strcasecmp(s1: *const libc::c_char, s2: *const libc::c_char) -> libc::c_int {
    let s1 = std::ffi::CStr::from_ptr(s1)
        .to_string_lossy()
        .to_lowercase();
    let s2 = std::ffi::CStr::from_ptr(s2)
        .to_string_lossy()
        .to_lowercase();
    if s1 == s2 {
        0
    } else {
        1
    }
}

pub(crate) unsafe fn strncasecmp(
    s1: *const libc::c_char,
    s2: *const libc::c_char,
    n: libc::size_t,
) -> libc::c_int {
    let s1 = std::ffi::CStr::from_ptr(s1)
        .to_string_lossy()
        .to_lowercase();
    let s2 = std::ffi::CStr::from_ptr(s2)
        .to_string_lossy()
        .to_lowercase();
    let m1 = std::cmp::min(n, s1.len());
    let m2 = std::cmp::min(n, s2.len());

    if s1[..m1] == s2[..m2] {
        0
    } else {
        1
    }
}

pub(crate) unsafe fn strdup(s: *const libc::c_char) -> *mut libc::c_char {
    let slen = libc::strlen(s);
    let result = libc::malloc(slen + 1);
    if result.is_null() {
        return std::ptr::null_mut();
    }

    libc::memcpy(result, s as *const _, slen + 1);
    result as *mut _
}

pub(crate) type size_t = libc::size_t;
pub(crate) type uint32_t = libc::c_uint;

pub const MAIL_ERROR_SSL: libc::c_uint = 58;
pub const MAIL_ERROR_FOLDER: libc::c_uint = 57;
pub const MAIL_ERROR_UNABLE: libc::c_uint = 56;
pub const MAIL_ERROR_SYSTEM: libc::c_uint = 55;
pub const MAIL_ERROR_COMMAND: libc::c_uint = 54;
pub const MAIL_ERROR_SEND: libc::c_uint = 53;
pub const MAIL_ERROR_CHAR_ENCODING_FAILED: libc::c_uint = 52;
pub const MAIL_ERROR_SUBJECT_NOT_FOUND: libc::c_uint = 51;
/* 50 */
pub const MAIL_ERROR_PROGRAM_ERROR: libc::c_uint = 50;
pub const MAIL_ERROR_NO_PERMISSION: libc::c_uint = 49;
pub const MAIL_ERROR_COMMAND_NOT_SUPPORTED: libc::c_uint = 48;
pub const MAIL_ERROR_NO_APOP: libc::c_uint = 47;
pub const MAIL_ERROR_READONLY: libc::c_uint = 46;
pub const MAIL_ERROR_FATAL: libc::c_uint = 45;
pub const MAIL_ERROR_CLOSE: libc::c_uint = 44;
pub const MAIL_ERROR_CAPABILITY: libc::c_uint = 43;
pub const MAIL_ERROR_PROTOCOL: libc::c_uint = 42;
/* misc errors */
pub const MAIL_ERROR_MISC: libc::c_uint = 41;
/* 40 */
pub const MAIL_ERROR_EXPUNGE: libc::c_uint = 40;
pub const MAIL_ERROR_NO_TLS: libc::c_uint = 39;
pub const MAIL_ERROR_CACHE_MISS: libc::c_uint = 38;
pub const MAIL_ERROR_STARTTLS: libc::c_uint = 37;
pub const MAIL_ERROR_MOVE: libc::c_uint = 36;
pub const MAIL_ERROR_FOLDER_NOT_FOUND: libc::c_uint = 35;
pub const MAIL_ERROR_REMOVE: libc::c_uint = 34;
pub const MAIL_ERROR_PART_NOT_FOUND: libc::c_uint = 33;
pub const MAIL_ERROR_INVAL: libc::c_uint = 32;
pub const MAIL_ERROR_PARSE: libc::c_uint = 31;
/* 30 */
pub const MAIL_ERROR_MSG_NOT_FOUND: libc::c_uint = 30;
pub const MAIL_ERROR_DISKSPACE: libc::c_uint = 29;
pub const MAIL_ERROR_SEARCH: libc::c_uint = 28;
pub const MAIL_ERROR_STORE: libc::c_uint = 27;
pub const MAIL_ERROR_FETCH: libc::c_uint = 26;
pub const MAIL_ERROR_COPY: libc::c_uint = 25;
pub const MAIL_ERROR_APPEND: libc::c_uint = 24;
pub const MAIL_ERROR_LSUB: libc::c_uint = 23;
pub const MAIL_ERROR_LIST: libc::c_uint = 22;
pub const MAIL_ERROR_UNSUBSCRIBE: libc::c_uint = 21;
/* 20 */
pub const MAIL_ERROR_SUBSCRIBE: libc::c_uint = 20;
pub const MAIL_ERROR_STATUS: libc::c_uint = 19;
pub const MAIL_ERROR_MEMORY: libc::c_uint = 18;
pub const MAIL_ERROR_SELECT: libc::c_uint = 17;
pub const MAIL_ERROR_EXAMINE: libc::c_uint = 16;
pub const MAIL_ERROR_CHECK: libc::c_uint = 15;
pub const MAIL_ERROR_RENAME: libc::c_uint = 14;
pub const MAIL_ERROR_NOOP: libc::c_uint = 13;
pub const MAIL_ERROR_LOGOUT: libc::c_uint = 12;
pub const MAIL_ERROR_DELETE: libc::c_uint = 11;
/* 10 */
pub const MAIL_ERROR_CREATE: libc::c_uint = 10;
pub const MAIL_ERROR_LOGIN: libc::c_uint = 9;
pub const MAIL_ERROR_STREAM: libc::c_uint = 8;
pub const MAIL_ERROR_FILE: libc::c_uint = 7;
pub const MAIL_ERROR_BAD_STATE: libc::c_uint = 6;
pub const MAIL_ERROR_CONNECT: libc::c_uint = 5;
pub const MAIL_ERROR_UNKNOWN: libc::c_uint = 4;
pub const MAIL_ERROR_NOT_IMPLEMENTED: libc::c_uint = 3;
pub const MAIL_NO_ERROR_NON_AUTHENTICATED: libc::c_uint = 2;
pub const MAIL_NO_ERROR_AUTHENTICATED: libc::c_uint = 1;
pub const MAIL_NO_ERROR: libc::c_uint = 0;

pub const MAILIMF_ERROR_FILE: libc::c_uint = 4;
pub const MAILIMF_ERROR_INVAL: libc::c_uint = 3;
pub const MAILIMF_ERROR_MEMORY: libc::c_uint = 2;
pub const MAILIMF_ERROR_PARSE: libc::c_uint = 1;
pub const MAILIMF_NO_ERROR: libc::c_uint = 0;

pub unsafe fn mailprivacy_prepare_mime(mut mime: *mut mailmime) {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    match (*mime).mm_type {
        1 => {
            if !(*mime).mm_data.mm_single.is_null() {
                prepare_mime_single(mime);
            }
        }
        2 => {
            cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
            while !cur.is_null() {
                let mut child: *mut mailmime = 0 as *mut mailmime;
                child = (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime;
                mailprivacy_prepare_mime(child);
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell
                }
            }
        }
        3 => {
            if !(*mime).mm_data.mm_message.mm_msg_mime.is_null() {
                mailprivacy_prepare_mime((*mime).mm_data.mm_message.mm_msg_mime);
            }
        }
        _ => {}
    };
}

unsafe fn prepare_mime_single(mut mime: *mut mailmime) {
    let mut single_fields: mailmime_single_fields = mailmime_single_fields {
        fld_content: 0 as *mut mailmime_content,
        fld_content_charset: 0 as *mut libc::c_char,
        fld_content_boundary: 0 as *mut libc::c_char,
        fld_content_name: 0 as *mut libc::c_char,
        fld_encoding: 0 as *mut mailmime_mechanism,
        fld_id: 0 as *mut libc::c_char,
        fld_description: 0 as *mut libc::c_char,
        fld_version: 0,
        fld_disposition: 0 as *mut mailmime_disposition,
        fld_disposition_filename: 0 as *mut libc::c_char,
        fld_disposition_creation_date: 0 as *mut libc::c_char,
        fld_disposition_modification_date: 0 as *mut libc::c_char,
        fld_disposition_read_date: 0 as *mut libc::c_char,
        fld_disposition_size: 0,
        fld_language: 0 as *mut mailmime_language,
        fld_location: 0 as *mut libc::c_char,
    };
    let mut encoding: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    if !(*mime).mm_mime_fields.is_null() {
        mailmime_single_fields_init(
            &mut single_fields,
            (*mime).mm_mime_fields,
            (*mime).mm_content_type,
        );
        if !single_fields.fld_encoding.is_null() {
            encoding = (*single_fields.fld_encoding).enc_type;
            match encoding {
                2 | 1 | 3 => {
                    (*single_fields.fld_encoding).enc_type =
                        MAILMIME_MECHANISM_QUOTED_PRINTABLE as libc::c_int
                }
                _ => {}
            }
        } else {
            let mut mechanism: *mut mailmime_mechanism = 0 as *mut mailmime_mechanism;
            let mut field: *mut mailmime_field = 0 as *mut mailmime_field;
            mechanism = mailmime_mechanism_new(
                MAILMIME_MECHANISM_QUOTED_PRINTABLE as libc::c_int,
                0 as *mut libc::c_char,
            );
            if mechanism.is_null() {
                return;
            }
            field = mailmime_field_new(
                MAILMIME_FIELD_TRANSFER_ENCODING as libc::c_int,
                0 as *mut mailmime_content,
                mechanism,
                0 as *mut libc::c_char,
                0 as *mut libc::c_char,
                0i32 as uint32_t,
                0 as *mut mailmime_disposition,
                0 as *mut mailmime_language,
                0 as *mut libc::c_char,
            );
            if field.is_null() {
                mailmime_mechanism_free(mechanism);
                return;
            }
            r = clist_insert_after(
                (*(*mime).mm_mime_fields).fld_list,
                (*(*(*mime).mm_mime_fields).fld_list).last,
                field as *mut libc::c_void,
            );
            if r < 0i32 {
                mailmime_field_free(field);
                return;
            }
        }
    }
    if (*mime).mm_type == MAILMIME_SINGLE as libc::c_int {
        match (*(*mime).mm_data.mm_single).dt_encoding {
            2 | 1 | 3 => {
                (*(*mime).mm_data.mm_single).dt_encoding =
                    MAILMIME_MECHANISM_QUOTED_PRINTABLE as libc::c_int;
                (*(*mime).mm_data.mm_single).dt_encoded = 0i32
            }
            _ => {}
        }
    };
}

pub unsafe fn mailmime_substitute(
    mut old_mime: *mut mailmime,
    mut new_mime: *mut mailmime,
) -> libc::c_int {
    let mut parent: *mut mailmime = 0 as *mut mailmime;
    parent = (*old_mime).mm_parent;
    if parent.is_null() {
        return MAIL_ERROR_INVAL as libc::c_int;
    }
    if (*old_mime).mm_parent_type == MAILMIME_MESSAGE as libc::c_int {
        (*parent).mm_data.mm_message.mm_msg_mime = new_mime
    } else {
        (*(*old_mime).mm_multipart_pos).data = new_mime as *mut libc::c_void
    }
    (*new_mime).mm_parent = parent;
    (*new_mime).mm_parent_type = (*old_mime).mm_parent_type;
    (*old_mime).mm_parent = 0 as *mut mailmime;
    (*old_mime).mm_parent_type = MAILMIME_NONE as libc::c_int;
    return MAIL_NO_ERROR as libc::c_int;
}

/*
  mailimf_address_list_new_empty creates an empty list of addresses
*/
pub unsafe fn mailimf_address_list_new_empty() -> *mut mailimf_address_list {
    let mut list: *mut clist = 0 as *mut clist;
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    list = clist_new();
    if list.is_null() {
        return 0 as *mut mailimf_address_list;
    }
    addr_list = mailimf_address_list_new(list);
    if addr_list.is_null() {
        return 0 as *mut mailimf_address_list;
    }
    return addr_list;
}

/*
  mailimf_mailbox_list_new_empty creates an empty list of mailboxes
*/
pub unsafe fn mailimf_mailbox_list_new_empty() -> *mut mailimf_mailbox_list {
    let mut list: *mut clist = 0 as *mut clist;
    let mut mb_list: *mut mailimf_mailbox_list = 0 as *mut mailimf_mailbox_list;
    list = clist_new();
    if list.is_null() {
        return 0 as *mut mailimf_mailbox_list;
    }
    mb_list = mailimf_mailbox_list_new(list);
    if mb_list.is_null() {
        return 0 as *mut mailimf_mailbox_list;
    }
    return mb_list;
}

/*
  mailimf_mailbox_list_add adds a mailbox to the list of mailboxes

  @return MAILIMF_NO_ERROR will be returned on success,
  other code will be returned otherwise
*/
pub unsafe fn mailimf_mailbox_list_add(
    mut mailbox_list: *mut mailimf_mailbox_list,
    mut mb: *mut mailimf_mailbox,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = clist_insert_after(
        (*mailbox_list).mb_list,
        (*(*mailbox_list).mb_list).last,
        mb as *mut libc::c_void,
    );
    if r < 0i32 {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

/*
  mailimf_address_list_add adds a mailbox to the list of addresses

  @return MAILIMF_NO_ERROR will be returned on success,
  other code will be returned otherwise
*/
pub unsafe fn mailimf_address_list_add(
    mut address_list: *mut mailimf_address_list,
    mut addr: *mut mailimf_address,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = clist_insert_after(
        (*address_list).ad_list,
        (*(*address_list).ad_list).last,
        addr as *mut libc::c_void,
    );
    if r < 0i32 {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

/*
  mailimf_fields_new_with_data_all creates a new mailimf_fields
  structure with a set of fields

  if you don't want a given field in the set to be added in the list
  of fields, you can give NULL as argument

  @param message_id sould be allocated with malloc()
  @param subject should be allocated with malloc()
  @param in_reply_to each elements of this list should be allocated
    with malloc()
  @param references each elements of this list should be allocated
    with malloc()

  @return MAILIMF_NO_ERROR will be returned on success,
  other code will be returned otherwise
*/
pub unsafe fn mailimf_fields_new_with_data_all(
    mut date: *mut mailimf_date_time,
    mut from: *mut mailimf_mailbox_list,
    mut sender: *mut mailimf_mailbox,
    mut reply_to: *mut mailimf_address_list,
    mut to: *mut mailimf_address_list,
    mut cc: *mut mailimf_address_list,
    mut bcc: *mut mailimf_address_list,
    mut message_id: *mut libc::c_char,
    mut in_reply_to: *mut clist,
    mut references: *mut clist,
    mut subject: *mut libc::c_char,
) -> *mut mailimf_fields {
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut r: libc::c_int = 0;
    fields = mailimf_fields_new_empty();
    if !fields.is_null() {
        r = mailimf_fields_add_data(
            fields,
            date,
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
        );
        if r != MAILIMF_NO_ERROR as libc::c_int {
            mailimf_fields_free(fields);
        } else {
            return fields;
        }
    }
    return 0 as *mut mailimf_fields;
}

/*
  mailimf_fields_add_data adds a set of fields in the
  given mailimf_fields structure.

  if you don't want a given field in the set to be added in the list
  of fields, you can give NULL as argument

  @param msg_id sould be allocated with malloc()
  @param subject should be allocated with malloc()
  @param in_reply_to each elements of this list should be allocated
    with malloc()
  @param references each elements of this list should be allocated
    with malloc()

  @return MAILIMF_NO_ERROR will be returned on success,
  other code will be returned otherwise
*/
pub unsafe fn mailimf_fields_add_data(
    mut fields: *mut mailimf_fields,
    mut date: *mut mailimf_date_time,
    mut from: *mut mailimf_mailbox_list,
    mut sender: *mut mailimf_mailbox,
    mut reply_to: *mut mailimf_address_list,
    mut to: *mut mailimf_address_list,
    mut cc: *mut mailimf_address_list,
    mut bcc: *mut mailimf_address_list,
    mut msg_id: *mut libc::c_char,
    mut in_reply_to: *mut clist,
    mut references: *mut clist,
    mut subject: *mut libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut imf_date: *mut mailimf_orig_date = 0 as *mut mailimf_orig_date;
    let mut imf_from: *mut mailimf_from = 0 as *mut mailimf_from;
    let mut imf_sender: *mut mailimf_sender = 0 as *mut mailimf_sender;
    let mut imf_reply_to: *mut mailimf_reply_to = 0 as *mut mailimf_reply_to;
    let mut imf_to: *mut mailimf_to = 0 as *mut mailimf_to;
    let mut imf_cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
    let mut imf_bcc: *mut mailimf_bcc = 0 as *mut mailimf_bcc;
    let mut imf_msg_id: *mut mailimf_message_id = 0 as *mut mailimf_message_id;
    let mut imf_references: *mut mailimf_references = 0 as *mut mailimf_references;
    let mut imf_in_reply_to: *mut mailimf_in_reply_to = 0 as *mut mailimf_in_reply_to;
    let mut imf_subject: *mut mailimf_subject = 0 as *mut mailimf_subject;
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    let mut r: libc::c_int = 0;
    imf_date = 0 as *mut mailimf_orig_date;
    imf_from = 0 as *mut mailimf_from;
    imf_sender = 0 as *mut mailimf_sender;
    imf_reply_to = 0 as *mut mailimf_reply_to;
    imf_to = 0 as *mut mailimf_to;
    imf_cc = 0 as *mut mailimf_cc;
    imf_bcc = 0 as *mut mailimf_bcc;
    imf_msg_id = 0 as *mut mailimf_message_id;
    imf_references = 0 as *mut mailimf_references;
    imf_in_reply_to = 0 as *mut mailimf_in_reply_to;
    imf_subject = 0 as *mut mailimf_subject;
    field = 0 as *mut mailimf_field;
    if !date.is_null() {
        imf_date = mailimf_orig_date_new(date);
        if imf_date.is_null() {
            current_block = 16539016819803454162;
        } else {
            field = mailimf_field_new(
                MAILIMF_FIELD_ORIG_DATE as libc::c_int,
                0 as *mut mailimf_return,
                0 as *mut mailimf_orig_date,
                0 as *mut mailimf_from,
                0 as *mut mailimf_sender,
                0 as *mut mailimf_to,
                0 as *mut mailimf_cc,
                0 as *mut mailimf_bcc,
                0 as *mut mailimf_message_id,
                imf_date,
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
                0 as *mut mailimf_optional_field,
            );
            /* return-path */
            /* resent date */
            /* resent from */
            /* resent sender */
            /* resent to */
            /* resent cc */
            /* resent bcc */
            /* resent msg id */
            /* date */
            /* from */
            /* sender */
            /* reply-to */
            /* to */
            /* cc */
            /* bcc */
            /* message id */
            /* in reply to */
            /* references */
            /* subject */
            /* comments */
            /* keywords */
            /* optional field */
            if field.is_null() {
                current_block = 16539016819803454162;
            } else {
                r = mailimf_fields_add(fields, field);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    current_block = 13813460800808168376;
                } else {
                    current_block = 2719512138335094285;
                }
            }
        }
    } else {
        current_block = 2719512138335094285;
    }
    match current_block {
        2719512138335094285 => {
            if !from.is_null() {
                imf_from = mailimf_from_new(from);
                if imf_from.is_null() {
                    current_block = 13813460800808168376;
                } else {
                    field = mailimf_field_new(
                        MAILIMF_FIELD_FROM as libc::c_int,
                        0 as *mut mailimf_return,
                        0 as *mut mailimf_orig_date,
                        0 as *mut mailimf_from,
                        0 as *mut mailimf_sender,
                        0 as *mut mailimf_to,
                        0 as *mut mailimf_cc,
                        0 as *mut mailimf_bcc,
                        0 as *mut mailimf_message_id,
                        0 as *mut mailimf_orig_date,
                        imf_from,
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
                        0 as *mut mailimf_optional_field,
                    );
                    /* return-path */
                    /* resent date */
                    /* resent from */
                    /* resent sender */
                    /* resent to */
                    /* resent cc */
                    /* resent bcc */
                    /* resent msg id */
                    /* date */
                    /* from */
                    /* sender */
                    /* reply-to */
                    /* to */
                    /* cc */
                    /* bcc */
                    /* message id */
                    /* in reply to */
                    /* references */
                    /* subject */
                    /* comments */
                    /* keywords */
                    /* optional field */
                    if field.is_null() {
                        current_block = 16539016819803454162;
                    } else {
                        r = mailimf_fields_add(fields, field);
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            current_block = 13813460800808168376;
                        } else {
                            current_block = 3275366147856559585;
                        }
                    }
                }
            } else {
                current_block = 3275366147856559585;
            }
            match current_block {
                13813460800808168376 => {}
                16539016819803454162 => {}
                _ => {
                    if !sender.is_null() {
                        imf_sender = mailimf_sender_new(sender);
                        if imf_sender.is_null() {
                            current_block = 16539016819803454162;
                        } else {
                            field = mailimf_field_new(
                                MAILIMF_FIELD_SENDER as libc::c_int,
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
                                imf_sender,
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
                                0 as *mut mailimf_optional_field,
                            );
                            /* return-path */
                            /* resent date */
                            /* resent from */
                            /* resent sender */
                            /* resent to */
                            /* resent cc */
                            /* resent bcc */
                            /* resent msg id */
                            /* date */
                            /* from */
                            /* sender */
                            /* reply-to */
                            /* to */
                            /* cc */
                            /* bcc */
                            /* message id */
                            /* in reply to */
                            /* references */
                            /* subject */
                            /* comments */
                            /* keywords */
                            /* optional field */
                            if field.is_null() {
                                current_block = 16539016819803454162;
                            } else {
                                r = mailimf_fields_add(fields, field);
                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                    current_block = 13813460800808168376;
                                } else {
                                    current_block = 15090052786889560393;
                                }
                            }
                        }
                    } else {
                        current_block = 15090052786889560393;
                    }
                    match current_block {
                        16539016819803454162 => {}
                        13813460800808168376 => {}
                        _ => {
                            if !reply_to.is_null() {
                                imf_reply_to = mailimf_reply_to_new(reply_to);
                                if imf_reply_to.is_null() {
                                    current_block = 16539016819803454162;
                                } else {
                                    field = mailimf_field_new(
                                        MAILIMF_FIELD_REPLY_TO as libc::c_int,
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
                                        imf_reply_to,
                                        0 as *mut mailimf_to,
                                        0 as *mut mailimf_cc,
                                        0 as *mut mailimf_bcc,
                                        0 as *mut mailimf_message_id,
                                        0 as *mut mailimf_in_reply_to,
                                        0 as *mut mailimf_references,
                                        0 as *mut mailimf_subject,
                                        0 as *mut mailimf_comments,
                                        0 as *mut mailimf_keywords,
                                        0 as *mut mailimf_optional_field,
                                    );
                                    /* return-path */
                                    /* resent date */
                                    /* resent from */
                                    /* resent sender */
                                    /* resent to */
                                    /* resent cc */
                                    /* resent bcc */
                                    /* resent msg id */
                                    /* date */
                                    /* from */
                                    /* sender */
                                    /* reply-to */
                                    /* to */
                                    /* cc */
                                    /* bcc */
                                    /* message id */
                                    /* in reply to */
                                    /* references */
                                    /* subject */
                                    /* comments */
                                    /* keywords */
                                    /* optional field */
                                    if field.is_null() {
                                        current_block = 16539016819803454162;
                                    } else {
                                        r = mailimf_fields_add(fields, field);
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            current_block = 13813460800808168376;
                                        } else {
                                            current_block = 10150597327160359210;
                                        }
                                    }
                                }
                            } else {
                                current_block = 10150597327160359210;
                            }
                            match current_block {
                                16539016819803454162 => {}
                                13813460800808168376 => {}
                                _ => {
                                    if !to.is_null() {
                                        imf_to = mailimf_to_new(to);
                                        if imf_to.is_null() {
                                            current_block = 16539016819803454162;
                                        } else {
                                            field = mailimf_field_new(
                                                MAILIMF_FIELD_TO as libc::c_int,
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
                                                imf_to,
                                                0 as *mut mailimf_cc,
                                                0 as *mut mailimf_bcc,
                                                0 as *mut mailimf_message_id,
                                                0 as *mut mailimf_in_reply_to,
                                                0 as *mut mailimf_references,
                                                0 as *mut mailimf_subject,
                                                0 as *mut mailimf_comments,
                                                0 as *mut mailimf_keywords,
                                                0 as *mut mailimf_optional_field,
                                            );
                                            /* return-path */
                                            /* resent date */
                                            /* resent from */
                                            /* resent sender */
                                            /* resent to */
                                            /* resent cc */
                                            /* resent bcc */
                                            /* resent msg id */
                                            /* date */
                                            /* from */
                                            /* sender */
                                            /* reply-to */
                                            /* to */
                                            /* cc */
                                            /* bcc */
                                            /* message id */
                                            /* in reply to */
                                            /* references */
                                            /* subject */
                                            /* comments */
                                            /* keywords */
                                            /* optional field */
                                            if field.is_null() {
                                                current_block = 16539016819803454162;
                                            } else {
                                                r = mailimf_fields_add(fields, field);
                                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                                    current_block = 13813460800808168376;
                                                } else {
                                                    current_block = 17233182392562552756;
                                                }
                                            }
                                        }
                                    } else {
                                        current_block = 17233182392562552756;
                                    }
                                    match current_block {
                                        16539016819803454162 => {}
                                        13813460800808168376 => {}
                                        _ => {
                                            if !cc.is_null() {
                                                imf_cc = mailimf_cc_new(cc);
                                                if imf_cc.is_null() {
                                                    current_block = 16539016819803454162;
                                                } else {
                                                    field = mailimf_field_new(
                                                        MAILIMF_FIELD_CC as libc::c_int,
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
                                                        imf_cc,
                                                        0 as *mut mailimf_bcc,
                                                        0 as *mut mailimf_message_id,
                                                        0 as *mut mailimf_in_reply_to,
                                                        0 as *mut mailimf_references,
                                                        0 as *mut mailimf_subject,
                                                        0 as *mut mailimf_comments,
                                                        0 as *mut mailimf_keywords,
                                                        0 as *mut mailimf_optional_field,
                                                    );
                                                    /* return-path */
                                                    /* resent date */
                                                    /* resent from */
                                                    /* resent sender */
                                                    /* resent to */
                                                    /* resent cc */
                                                    /* resent bcc */
                                                    /* resent msg id */
                                                    /* date */
                                                    /* from */
                                                    /* sender */
                                                    /* reply-to */
                                                    /* to */
                                                    /* cc */
                                                    /* bcc */
                                                    /* message id */
                                                    /* in reply to */
                                                    /* references */
                                                    /* subject */
                                                    /* comments */
                                                    /* keywords */
                                                    /* optional field */
                                                    if field.is_null() {
                                                        current_block = 16539016819803454162;
                                                    } else {
                                                        r = mailimf_fields_add(fields, field);
                                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                                            current_block = 13813460800808168376;
                                                        } else {
                                                            current_block = 12930649117290160518;
                                                        }
                                                    }
                                                }
                                            } else {
                                                current_block = 12930649117290160518;
                                            }
                                            match current_block {
                                                16539016819803454162 => {}
                                                13813460800808168376 => {}
                                                _ => {
                                                    if !bcc.is_null() {
                                                        imf_bcc = mailimf_bcc_new(bcc);
                                                        if imf_bcc.is_null() {
                                                            current_block = 16539016819803454162;
                                                        } else {
                                                            field = mailimf_field_new(
                                                                MAILIMF_FIELD_BCC as libc::c_int,
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
                                                                imf_bcc,
                                                                0 as *mut mailimf_message_id,
                                                                0 as *mut mailimf_in_reply_to,
                                                                0 as *mut mailimf_references,
                                                                0 as *mut mailimf_subject,
                                                                0 as *mut mailimf_comments,
                                                                0 as *mut mailimf_keywords,
                                                                0 as *mut mailimf_optional_field,
                                                            );
                                                            /* return-path */
                                                            /* resent date */
                                                            /* resent from */
                                                            /* resent sender */
                                                            /* resent to */
                                                            /* resent cc */
                                                            /* resent bcc */
                                                            /* resent msg id */
                                                            /* date */
                                                            /* from */
                                                            /* sender */
                                                            /* reply-to */
                                                            /* to */
                                                            /* cc */
                                                            /* bcc */
                                                            /* message id */
                                                            /* in reply to */
                                                            /* references */
                                                            /* subject */
                                                            /* comments */
                                                            /* keywords */
                                                            /* optional field */
                                                            if field.is_null() {
                                                                current_block =
                                                                    16539016819803454162;
                                                            } else {
                                                                r = mailimf_fields_add(
                                                                    fields, field,
                                                                );
                                                                if r != MAILIMF_NO_ERROR
                                                                    as libc::c_int
                                                                {
                                                                    current_block =
                                                                        13813460800808168376;
                                                                } else {
                                                                    current_block =
                                                                        7858101417678297991;
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        current_block = 7858101417678297991;
                                                    }
                                                    match current_block {
                                                        16539016819803454162 => {}
                                                        13813460800808168376 => {}
                                                        _ => {
                                                            if !msg_id.is_null() {
                                                                imf_msg_id =
                                                                    mailimf_message_id_new(msg_id);
                                                                if imf_msg_id.is_null() {
                                                                    current_block =
                                                                        16539016819803454162;
                                                                } else {
                                                                    field =
                                                                        mailimf_field_new(MAILIMF_FIELD_MESSAGE_ID
                                                                                              as
                                                                                              libc::c_int,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_return,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_orig_date,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_from,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_sender,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_to,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_cc,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_bcc,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_message_id,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_orig_date,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_from,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_sender,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_reply_to,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_to,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_cc,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_bcc,
                                                                                          imf_msg_id,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_in_reply_to,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_references,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_subject,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_comments,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_keywords,
                                                                                          0
                                                                                              as
                                                                                              *mut mailimf_optional_field);
                                                                    /* return-path */
                                                                    /* resent date */
                                                                    /* resent from */
                                                                    /* resent sender */
                                                                    /* resent to */
                                                                    /* resent cc */
                                                                    /* resent bcc */
                                                                    /* resent msg id */
                                                                    /* date */
                                                                    /* from */
                                                                    /* sender */
                                                                    /* reply-to */
                                                                    /* to */
                                                                    /* cc */
                                                                    /* bcc */
                                                                    /* message id */
                                                                    /* in reply to */
                                                                    /* references */
                                                                    /* subject */
                                                                    /* comments */
                                                                    /* keywords */
                                                                    /* optional field */
                                                                    if field.is_null() {
                                                                        current_block =
                                                                            16539016819803454162;
                                                                    } else {
                                                                        r = mailimf_fields_add(
                                                                            fields, field,
                                                                        );
                                                                        if r != MAILIMF_NO_ERROR
                                                                            as libc::c_int
                                                                        {
                                                                            current_block
                                                                                =
                                                                                13813460800808168376;
                                                                        } else {
                                                                            current_block
                                                                                =
                                                                                15514718523126015390;
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                current_block =
                                                                    15514718523126015390;
                                                            }
                                                            match current_block {
                                                                13813460800808168376 => {}
                                                                16539016819803454162 => {}
                                                                _ => {
                                                                    if !in_reply_to.is_null() {
                                                                        imf_in_reply_to =
                                                                            mailimf_in_reply_to_new(
                                                                                in_reply_to,
                                                                            );
                                                                        if imf_in_reply_to.is_null()
                                                                        {
                                                                            current_block
                                                                                =
                                                                                16539016819803454162;
                                                                        } else {
                                                                            field
                                                                                =
                                                                                mailimf_field_new(MAILIMF_FIELD_IN_REPLY_TO
                                                                                                      as
                                                                                                      libc::c_int,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_return,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_orig_date,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_from,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_sender,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_to,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_cc,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_bcc,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_message_id,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_orig_date,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_from,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_sender,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_reply_to,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_to,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_cc,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_bcc,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_message_id,
                                                                                                  imf_in_reply_to,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_references,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_subject,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_comments,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_keywords,
                                                                                                  0
                                                                                                      as
                                                                                                      *mut mailimf_optional_field);
                                                                            /* return-path */
                                                                            /* resent date */
                                                                            /* resent from */
                                                                            /* resent sender */
                                                                            /* resent to */
                                                                            /* resent cc */
                                                                            /* resent bcc */
                                                                            /* resent msg id */
                                                                            /* date */
                                                                            /* from */
                                                                            /* sender */
                                                                            /* reply-to */
                                                                            /* to */
                                                                            /* cc */
                                                                            /* bcc */
                                                                            /* message id */
                                                                            /* in reply to */
                                                                            /* references */
                                                                            /* subject */
                                                                            /* comments */
                                                                            /* keywords */
                                                                            /* optional field */
                                                                            if field.is_null() {
                                                                                current_block
                                                                                    =
                                                                                    16539016819803454162;
                                                                            } else {
                                                                                r
                                                                                    =
                                                                                    mailimf_fields_add(fields,
                                                                                                       field);
                                                                                if r
                                                                                       !=
                                                                                       MAILIMF_NO_ERROR
                                                                                           as
                                                                                           libc::c_int
                                                                                   {
                                                                                    current_block
                                                                                        =
                                                                                        13813460800808168376;
                                                                                } else {
                                                                                    current_block
                                                                                        =
                                                                                        15587532755333643506;
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        current_block =
                                                                            15587532755333643506;
                                                                    }
                                                                    match current_block {
                                                                        13813460800808168376 => {}
                                                                        16539016819803454162 => {}
                                                                        _ => {
                                                                            if !references.is_null()
                                                                            {
                                                                                imf_references
                                                                                    =
                                                                                    mailimf_references_new(references);
                                                                                if imf_references
                                                                                    .is_null()
                                                                                {
                                                                                    current_block
                                                                                        =
                                                                                        16539016819803454162;
                                                                                } else {
                                                                                    field
                                                                                        =
                                                                                        mailimf_field_new(MAILIMF_FIELD_REFERENCES
                                                                                                              as
                                                                                                              libc::c_int,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_return,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_orig_date,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_from,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_sender,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_to,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_cc,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_bcc,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_message_id,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_orig_date,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_from,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_sender,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_reply_to,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_to,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_cc,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_bcc,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_message_id,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_in_reply_to,
                                                                                                          imf_references,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_subject,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_comments,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_keywords,
                                                                                                          0
                                                                                                              as
                                                                                                              *mut mailimf_optional_field);
                                                                                    /* return-path */
                                                                                    /* resent date */
                                                                                    /* resent from */
                                                                                    /* resent sender */
                                                                                    /* resent to */
                                                                                    /* resent cc */
                                                                                    /* resent bcc */
                                                                                    /* resent msg id */
                                                                                    /* date */
                                                                                    /* from */
                                                                                    /* sender */
                                                                                    /* reply-to */
                                                                                    /* to */
                                                                                    /* cc */
                                                                                    /* bcc */
                                                                                    /* message id */
                                                                                    /* in reply to */
                                                                                    /* references */
                                                                                    /* subject */
                                                                                    /* comments */
                                                                                    /* keywords */
                                                                                    /* optional field */
                                                                                    if field
                                                                                        .is_null()
                                                                                    {
                                                                                        current_block
                                                                                            =
                                                                                            16539016819803454162;
                                                                                    } else {
                                                                                        r
                                                                                            =
                                                                                            mailimf_fields_add(fields,
                                                                                                               field);
                                                                                        if r
                                                                                               !=
                                                                                               MAILIMF_NO_ERROR
                                                                                                   as
                                                                                                   libc::c_int
                                                                                           {
                                                                                            current_block
                                                                                                =
                                                                                                13813460800808168376;
                                                                                        } else {
                                                                                            current_block
                                                                                                =
                                                                                                7301440000599063274;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                current_block
                                                                                    =
                                                                                    7301440000599063274;
                                                                            }
                                                                            match current_block
                                                                                {
                                                                                13813460800808168376
                                                                                =>
                                                                                {
                                                                                }
                                                                                16539016819803454162
                                                                                =>
                                                                                {
                                                                                }
                                                                                _
                                                                                =>
                                                                                {
                                                                                    if !subject.is_null()
                                                                                       {
                                                                                        imf_subject
                                                                                            =
                                                                                            mailimf_subject_new(subject);
                                                                                        if imf_subject.is_null()
                                                                                           {
                                                                                            current_block
                                                                                                =
                                                                                                16539016819803454162;
                                                                                        } else {
                                                                                            field
                                                                                                =
                                                                                                mailimf_field_new(MAILIMF_FIELD_SUBJECT
                                                                                                                      as
                                                                                                                      libc::c_int,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_return,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_orig_date,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_from,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_sender,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_to,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_cc,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_bcc,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_message_id,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_orig_date,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_from,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_sender,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_reply_to,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_to,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_cc,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_bcc,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_message_id,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_in_reply_to,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_references,
                                                                                                                  imf_subject,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_comments,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_keywords,
                                                                                                                  0
                                                                                                                      as
                                                                                                                      *mut mailimf_optional_field);
                                                                                            /* return-path */
                                                                                            /* resent date */
                                                                                            /* resent from */
                                                                                            /* resent sender */
                                                                                            /* resent to */
                                                                                            /* resent cc */
                                                                                            /* resent bcc */
                                                                                            /* resent msg id */
                                                                                            /* date */
                                                                                            /* from */
                                                                                            /* sender */
                                                                                            /* reply-to */
                                                                                            /* to */
                                                                                            /* cc */
                                                                                            /* bcc */
                                                                                            /* message id */
                                                                                            /* in reply to */
                                                                                            /* references */
                                                                                            /* subject */
                                                                                            /* comments */
                                                                                            /* keywords */
                                                                                            /* optional field */
                                                                                            if field.is_null()
                                                                                               {
                                                                                                current_block
                                                                                                    =
                                                                                                    16539016819803454162;
                                                                                            } else {
                                                                                                r
                                                                                                    =
                                                                                                    mailimf_fields_add(fields,
                                                                                                                       field);
                                                                                                if r
                                                                                                       !=
                                                                                                       MAILIMF_NO_ERROR
                                                                                                           as
                                                                                                           libc::c_int
                                                                                                   {
                                                                                                    current_block
                                                                                                        =
                                                                                                        13813460800808168376;
                                                                                                } else {
                                                                                                    current_block
                                                                                                        =
                                                                                                        10153752038087260855;
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        current_block
                                                                                            =
                                                                                            10153752038087260855;
                                                                                    }
                                                                                    match current_block
                                                                                        {
                                                                                        13813460800808168376
                                                                                        =>
                                                                                        {
                                                                                        }
                                                                                        16539016819803454162
                                                                                        =>
                                                                                        {
                                                                                        }
                                                                                        _
                                                                                        =>
                                                                                        {
                                                                                            return MAILIMF_NO_ERROR
                                                                                                       as
                                                                                                       libc::c_int
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    match current_block {
        13813460800808168376 => {
            if !field.is_null() {
                detach_field(field);
                mailimf_field_free(field);
            }
        }
        _ => {}
    }
    detach_free_fields(
        imf_date,
        imf_from,
        imf_sender,
        imf_reply_to,
        imf_to,
        imf_cc,
        imf_bcc,
        imf_msg_id,
        imf_in_reply_to,
        imf_references,
        imf_subject,
    );
    return MAILIMF_ERROR_MEMORY as libc::c_int;
}

unsafe fn detach_free_fields(
    mut date: *mut mailimf_orig_date,
    mut from: *mut mailimf_from,
    mut sender: *mut mailimf_sender,
    mut reply_to: *mut mailimf_reply_to,
    mut to: *mut mailimf_to,
    mut cc: *mut mailimf_cc,
    mut bcc: *mut mailimf_bcc,
    mut msg_id: *mut mailimf_message_id,
    mut in_reply_to: *mut mailimf_in_reply_to,
    mut references: *mut mailimf_references,
    mut subject: *mut mailimf_subject,
) {
    detach_free_common_fields(date, from, sender, to, cc, bcc, msg_id);
    if !reply_to.is_null() {
        (*reply_to).rt_addr_list = 0 as *mut mailimf_address_list;
        mailimf_reply_to_free(reply_to);
    }
    if !in_reply_to.is_null() {
        (*in_reply_to).mid_list = 0 as *mut clist;
        mailimf_in_reply_to_free(in_reply_to);
    }
    if !references.is_null() {
        (*references).mid_list = 0 as *mut clist;
        mailimf_references_free(references);
    }
    if !subject.is_null() {
        (*subject).sbj_value = 0 as *mut libc::c_char;
        mailimf_subject_free(subject);
    };
}

unsafe fn detach_field(mut field: *mut mailimf_field) {
    (*field).fld_type = MAILIMF_FIELD_NONE as libc::c_int;
    mailimf_field_free(field);
}

unsafe fn detach_free_common_fields(
    mut imf_date: *mut mailimf_orig_date,
    mut imf_from: *mut mailimf_from,
    mut imf_sender: *mut mailimf_sender,
    mut imf_to: *mut mailimf_to,
    mut imf_cc: *mut mailimf_cc,
    mut imf_bcc: *mut mailimf_bcc,
    mut imf_msg_id: *mut mailimf_message_id,
) {
    if !imf_date.is_null() {
        (*imf_date).dt_date_time = 0 as *mut mailimf_date_time;
        mailimf_orig_date_free(imf_date);
    }
    if !imf_from.is_null() {
        (*imf_from).frm_mb_list = 0 as *mut mailimf_mailbox_list;
        mailimf_from_free(imf_from);
    }
    if !imf_sender.is_null() {
        (*imf_sender).snd_mb = 0 as *mut mailimf_mailbox;
        mailimf_sender_free(imf_sender);
    }
    if !imf_to.is_null() {
        (*imf_to).to_addr_list = 0 as *mut mailimf_address_list;
        mailimf_to_free(imf_to);
    }
    if !imf_cc.is_null() {
        (*imf_cc).cc_addr_list = 0 as *mut mailimf_address_list;
        mailimf_to_free(imf_to);
    }
    if !imf_bcc.is_null() {
        (*imf_bcc).bcc_addr_list = 0 as *mut mailimf_address_list;
        mailimf_bcc_free(imf_bcc);
    }
    if !imf_msg_id.is_null() {
        (*imf_msg_id).mid_value = 0 as *mut libc::c_char;
        mailimf_message_id_free(imf_msg_id);
    };
}

pub fn mailimf_get_date(t: i64) -> *mut mailimf_date_time {
    let lt = Local.timestamp(t, 0);

    let off = (lt.offset().local_minus_utc() / (60 * 60)) * 100;

    unsafe {
        mailimf_date_time_new(
            lt.day() as libc::c_int,
            lt.month() as libc::c_int,
            lt.year() as libc::c_int,
            lt.hour() as libc::c_int,
            lt.minute() as libc::c_int,
            lt.second() as libc::c_int,
            off,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::ffi::CString;

    #[test]
    fn test_strcasecmp() {
        assert_eq!(0, unsafe {
            strcasecmp(
                CString::new("hello").unwrap().as_ptr(),
                CString::new("Hello").unwrap().as_ptr(),
            )
        });
    }

    #[test]
    fn test_strncasecmp() {
        assert_eq!(0, unsafe {
            strncasecmp(
                CString::new("helloworld").unwrap().as_ptr(),
                CString::new("Helloward").unwrap().as_ptr(),
                4,
            )
        });
    }

    #[test]
    fn test_get_date() {
        let now_utc = Utc::now();

        let now_local = Local.from_utc_datetime(&now_utc.naive_local());
        let t_local = now_local.timestamp();

        let converted = unsafe { *mailimf_get_date(t_local as i64) };

        assert_eq!(converted.dt_day as u32, now_local.day());
        assert_eq!(converted.dt_month as u32, now_local.month());
        assert_eq!(converted.dt_year, now_local.year());
        assert_eq!(converted.dt_hour as u32, now_local.hour());
        assert_eq!(converted.dt_min as u32, now_local.minute());
        assert_eq!(converted.dt_sec as u32, now_local.second());
        assert_eq!(
            converted.dt_zone,
            (now_local.offset().local_minus_utc() / (60 * 60)) * 100
        );
    }
}

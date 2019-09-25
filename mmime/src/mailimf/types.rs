use libc;

use crate::clist::*;
use crate::other::*;

/*
  IMPORTANT NOTE:

  All allocation functions will take as argument allocated data
  and will store these data in the structure they will allocate.
  Data should be persistant during all the use of the structure
  and will be freed by the free function of the structure

  allocation functions will return NULL on failure
*/
/*
  mailimf_date_time is a date

  - day is the day of month (1 to 31)

  - month (1 to 12)

  - year (4 digits)

  - hour (0 to 23)

  - min (0 to 59)

  - sec (0 to 59)

  - zone (this is the decimal value that we can read, for example:
    for "-0200", the value is -200)
*/
#[derive(Copy, Clone, Debug)]
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
/* this is the type of address */
pub type unnamed = libc::c_uint;
/* if this is a group
(group_name: address1@domain1,
    address2@domain2; ) */
pub const MAILIMF_ADDRESS_GROUP: unnamed = 2;
/* if this is a mailbox (mailbox@domain) */
pub const MAILIMF_ADDRESS_MAILBOX: unnamed = 1;
/* on parse error */
pub const MAILIMF_ADDRESS_ERROR: unnamed = 0;
/*
  mailimf_address is an address

  - type can be MAILIMF_ADDRESS_MAILBOX or MAILIMF_ADDRESS_GROUP

  - mailbox is a mailbox if type is MAILIMF_ADDRESS_MAILBOX

  - group is a group if type is MAILIMF_ADDRESS_GROUP
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_address {
    pub ad_type: libc::c_int,
    pub ad_data: unnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_0 {
    pub ad_mailbox: *mut mailimf_mailbox,
    pub ad_group: *mut mailimf_group,
}
/*
  mailimf_group is a group

  - display_name is the name that will be displayed for this group,
    for example 'group_name' in
    'group_name: address1@domain1, address2@domain2;', should be allocated
    with malloc()

  - mb_list is a list of mailboxes
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_group {
    pub grp_display_name: *mut libc::c_char,
    pub grp_mb_list: *mut mailimf_mailbox_list,
}
/*
  mailimf_mailbox_list is a list of mailboxes

  - list is a list of mailboxes
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_mailbox_list {
    pub mb_list: *mut clist,
}
/*
  mailimf_mailbox is a mailbox

  - display_name is the name that will be displayed for this mailbox,
    for example 'name' in '"name" <mailbox@domain>,
    should be allocated with malloc()

  - addr_spec is the mailbox, for example 'mailbox@domain'
    in '"name" <mailbox@domain>, should be allocated with malloc()
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_mailbox {
    pub mb_display_name: *mut libc::c_char,
    pub mb_addr_spec: *mut libc::c_char,
}
/*
  mailimf_address_list is a list of addresses

  - list is a list of addresses
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_address_list {
    pub ad_list: *mut clist,
}
/*
  mailimf_body is the text part of a message

  - text is the beginning of the text part, it is a substring
    of an other string

  - size is the size of the text part
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_body {
    pub bd_text: *const libc::c_char,
    pub bd_size: size_t,
}
/*
  mailimf_message is the content of the message

  - msg_fields is the header fields of the message

  - msg_body is the text part of the message
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_message {
    pub msg_fields: *mut mailimf_fields,
    pub msg_body: *mut mailimf_body,
}
/*
  mailimf_fields is a list of header fields

  - fld_list is a list of header fields
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_fields {
    pub fld_list: *mut clist,
}
/*
  mailimf_field is a field

  - fld_type is the type of the field

  - fld_data.fld_return_path is the parsed content of the Return-Path
    field if type is MAILIMF_FIELD_RETURN_PATH

  - fld_data.fld_resent_date is the parsed content of the Resent-Date field
    if type is MAILIMF_FIELD_RESENT_DATE

  - fld_data.fld_resent_from is the parsed content of the Resent-From field

  - fld_data.fld_resent_sender is the parsed content of the Resent-Sender field

  - fld_data.fld_resent_to is the parsed content of the Resent-To field

  - fld_data.fld_resent_cc is the parsed content of the Resent-Cc field

  - fld_data.fld_resent_bcc is the parsed content of the Resent-Bcc field

  - fld_data.fld_resent_msg_id is the parsed content of the Resent-Message-ID
    field

  - fld_data.fld_orig_date is the parsed content of the Date field

  - fld_data.fld_from is the parsed content of the From field

  - fld_data.fld_sender is the parsed content of the Sender field

  - fld_data.fld_reply_to is the parsed content of the Reply-To field

  - fld_data.fld_to is the parsed content of the To field

  - fld_data.fld_cc is the parsed content of the Cc field

  - fld_data.fld_bcc is the parsed content of the Bcc field

  - fld_data.fld_message_id is the parsed content of the Message-ID field

  - fld_data.fld_in_reply_to is the parsed content of the In-Reply-To field

  - fld_data.fld_references is the parsed content of the References field

  - fld_data.fld_subject is the content of the Subject field

  - fld_data.fld_comments is the content of the Comments field

  - fld_data.fld_keywords is the parsed content of the Keywords field

  - fld_data.fld_optional_field is an other field and is not parsed
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_1 {
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
/*
  mailimf_optional_field is a non-parsed field

  - fld_name is the name of the field

  - fld_value is the value of the field
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_optional_field {
    pub fld_name: *mut libc::c_char,
    pub fld_value: *mut libc::c_char,
}
/*
  mailimf_keywords is the parsed Keywords field

  - kw_list is the list of keywords
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_keywords {
    pub kw_list: *mut clist,
}
/*
  mailimf_comments is the parsed Comments field

  - cm_value is the value of the field
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_comments {
    pub cm_value: *mut libc::c_char,
}
/*
  mailimf_subject is the parsed Subject field

  - sbj_value is the value of the field
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_subject {
    pub sbj_value: *mut libc::c_char,
}
/*
 mailimf_references is the parsed References field

 - msg_id_list is the list of message identifiers
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_references {
    pub mid_list: *mut clist,
}
/*
  mailimf_in_reply_to is the parsed In-Reply-To field

  - mid_list is the list of message identifers
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_in_reply_to {
    pub mid_list: *mut clist,
}
/*
  mailimf_message_id is the parsed Message-ID field

  - mid_value is the message identifier
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_message_id {
    pub mid_value: *mut libc::c_char,
}
/*
  mailimf_bcc is the parsed Bcc field

  - bcc_addr_list is the parsed addres list
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_bcc {
    pub bcc_addr_list: *mut mailimf_address_list,
}
/*
  mailimf_cc is the parsed Cc field

  - cc_addr_list is the parsed addres list
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_cc {
    pub cc_addr_list: *mut mailimf_address_list,
}
/*
  mailimf_to is the parsed To field

  - to_addr_list is the parsed address list
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_to {
    pub to_addr_list: *mut mailimf_address_list,
}
/*
 mailimf_reply_to is the parsed Reply-To field

 - rt_addr_list is the parsed address list
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_reply_to {
    pub rt_addr_list: *mut mailimf_address_list,
}
/*
  mailimf_sender is the parsed Sender field

  - snd_mb is the parsed mailbox
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_sender {
    pub snd_mb: *mut mailimf_mailbox,
}
/*
  mailimf_from is the parsed From field

  - mb_list is the parsed mailbox list
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_from {
    pub frm_mb_list: *mut mailimf_mailbox_list,
}
/*
  mailimf_orig_date is the parsed Date field

  - date_time is the parsed date
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_orig_date {
    pub dt_date_time: *mut mailimf_date_time,
}
/*
  mailimf_return is the parsed Return-Path field

  - ret_path is the parsed value of Return-Path
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_return {
    pub ret_path: *mut mailimf_path,
}
/*
  mailimf_path is the parsed value of Return-Path

  - pt_addr_spec is a mailbox
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_path {
    pub pt_addr_spec: *mut libc::c_char,
}
/* other field */
pub const MAILIMF_FIELD_OPTIONAL_FIELD: unnamed_2 = 22;
/* Keywords */
pub const MAILIMF_FIELD_KEYWORDS: unnamed_2 = 21;
/* Comments */
pub const MAILIMF_FIELD_COMMENTS: unnamed_2 = 20;
/* Subject */
pub const MAILIMF_FIELD_SUBJECT: unnamed_2 = 19;
/* References */
pub const MAILIMF_FIELD_REFERENCES: unnamed_2 = 18;
/* In-Reply-To */
pub const MAILIMF_FIELD_IN_REPLY_TO: unnamed_2 = 17;
/* Message-ID */
pub const MAILIMF_FIELD_MESSAGE_ID: unnamed_2 = 16;
/* Bcc */
pub const MAILIMF_FIELD_BCC: unnamed_2 = 15;
/* Cc */
pub const MAILIMF_FIELD_CC: unnamed_2 = 14;
/* To */
pub const MAILIMF_FIELD_TO: unnamed_2 = 13;
/* Reply-To */
pub const MAILIMF_FIELD_REPLY_TO: unnamed_2 = 12;
/* Sender */
pub const MAILIMF_FIELD_SENDER: unnamed_2 = 11;
/* From */
pub const MAILIMF_FIELD_FROM: unnamed_2 = 10;
/* Date */
pub const MAILIMF_FIELD_ORIG_DATE: unnamed_2 = 9;
/* Resent-Message-ID */
pub const MAILIMF_FIELD_RESENT_MSG_ID: unnamed_2 = 8;
/* Resent-Bcc */
pub const MAILIMF_FIELD_RESENT_BCC: unnamed_2 = 7;
/* Resent-Cc */
pub const MAILIMF_FIELD_RESENT_CC: unnamed_2 = 6;
/* Resent-To */
pub const MAILIMF_FIELD_RESENT_TO: unnamed_2 = 5;
/* Resent-Sender */
pub const MAILIMF_FIELD_RESENT_SENDER: unnamed_2 = 4;
/* Resent-From */
pub const MAILIMF_FIELD_RESENT_FROM: unnamed_2 = 3;
/* Resent-Date */
pub const MAILIMF_FIELD_RESENT_DATE: unnamed_2 = 2;
/* Return-Path */
pub const MAILIMF_FIELD_RETURN_PATH: unnamed_2 = 1;
/* this is a type of field */
pub type unnamed_2 = libc::c_uint;
/* on parse error */
pub const MAILIMF_FIELD_NONE: unnamed_2 = 0;
#[no_mangle]
pub unsafe fn mailimf_date_time_new(
    mut dt_day: libc::c_int,
    mut dt_month: libc::c_int,
    mut dt_year: libc::c_int,
    mut dt_hour: libc::c_int,
    mut dt_min: libc::c_int,
    mut dt_sec: libc::c_int,
    mut dt_zone: libc::c_int,
) -> *mut mailimf_date_time {
    let mut date_time: *mut mailimf_date_time = 0 as *mut mailimf_date_time;
    date_time = malloc(::std::mem::size_of::<mailimf_date_time>() as libc::size_t)
        as *mut mailimf_date_time;
    if date_time.is_null() {
        return 0 as *mut mailimf_date_time;
    }
    (*date_time).dt_day = dt_day;
    (*date_time).dt_month = dt_month;
    (*date_time).dt_year = dt_year;
    (*date_time).dt_hour = dt_hour;
    (*date_time).dt_min = dt_min;
    (*date_time).dt_sec = dt_sec;
    (*date_time).dt_zone = dt_zone;
    return date_time;
}
#[no_mangle]
pub unsafe fn mailimf_date_time_free(mut date_time: *mut mailimf_date_time) {
    free(date_time as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_address_new(
    mut ad_type: libc::c_int,
    mut ad_mailbox: *mut mailimf_mailbox,
    mut ad_group: *mut mailimf_group,
) -> *mut mailimf_address {
    let mut address: *mut mailimf_address = 0 as *mut mailimf_address;
    address =
        malloc(::std::mem::size_of::<mailimf_address>() as libc::size_t) as *mut mailimf_address;
    if address.is_null() {
        return 0 as *mut mailimf_address;
    }
    (*address).ad_type = ad_type;
    match ad_type {
        1 => (*address).ad_data.ad_mailbox = ad_mailbox,
        2 => (*address).ad_data.ad_group = ad_group,
        _ => {}
    }
    return address;
}
#[no_mangle]
pub unsafe fn mailimf_address_free(mut address: *mut mailimf_address) {
    match (*address).ad_type {
        1 => {
            mailimf_mailbox_free((*address).ad_data.ad_mailbox);
        }
        2 => {
            mailimf_group_free((*address).ad_data.ad_group);
        }
        _ => {}
    }
    free(address as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_group_free(mut group: *mut mailimf_group) {
    if !(*group).grp_mb_list.is_null() {
        mailimf_mailbox_list_free((*group).grp_mb_list);
    }
    mailimf_display_name_free((*group).grp_display_name);
    free(group as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_display_name_free(mut display_name: *mut libc::c_char) {
    mailimf_phrase_free(display_name);
}
#[no_mangle]
pub unsafe fn mailimf_phrase_free(mut phrase: *mut libc::c_char) {
    free(phrase as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_mailbox_list_free(mut mb_list: *mut mailimf_mailbox_list) {
    clist_foreach(
        (*mb_list).mb_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut mailimf_mailbox) -> ()>, clist_func>(
            Some(mailimf_mailbox_free),
        ),
        0 as *mut libc::c_void,
    );
    clist_free((*mb_list).mb_list);
    free(mb_list as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_mailbox_free(mut mailbox: *mut mailimf_mailbox) {
    if !(*mailbox).mb_display_name.is_null() {
        mailimf_display_name_free((*mailbox).mb_display_name);
    }
    mailimf_addr_spec_free((*mailbox).mb_addr_spec);
    free(mailbox as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_addr_spec_free(mut addr_spec: *mut libc::c_char) {
    free(addr_spec as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_mailbox_new(
    mut mb_display_name: *mut libc::c_char,
    mut mb_addr_spec: *mut libc::c_char,
) -> *mut mailimf_mailbox {
    let mut mb: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
    mb = malloc(::std::mem::size_of::<mailimf_mailbox>() as libc::size_t) as *mut mailimf_mailbox;
    if mb.is_null() {
        return 0 as *mut mailimf_mailbox;
    }
    (*mb).mb_display_name = mb_display_name;
    (*mb).mb_addr_spec = mb_addr_spec;
    return mb;
}
#[no_mangle]
pub unsafe fn mailimf_group_new(
    mut grp_display_name: *mut libc::c_char,
    mut grp_mb_list: *mut mailimf_mailbox_list,
) -> *mut mailimf_group {
    let mut group: *mut mailimf_group = 0 as *mut mailimf_group;
    group = malloc(::std::mem::size_of::<mailimf_group>() as libc::size_t) as *mut mailimf_group;
    if group.is_null() {
        return 0 as *mut mailimf_group;
    }
    (*group).grp_display_name = grp_display_name;
    (*group).grp_mb_list = grp_mb_list;
    return group;
}
#[no_mangle]
pub unsafe fn mailimf_mailbox_list_new(mut mb_list: *mut clist) -> *mut mailimf_mailbox_list {
    let mut mbl: *mut mailimf_mailbox_list = 0 as *mut mailimf_mailbox_list;
    mbl = malloc(::std::mem::size_of::<mailimf_mailbox_list>() as libc::size_t)
        as *mut mailimf_mailbox_list;
    if mbl.is_null() {
        return 0 as *mut mailimf_mailbox_list;
    }
    (*mbl).mb_list = mb_list;
    return mbl;
}

pub unsafe fn mailimf_address_list_new(mut ad_list: *mut clist) -> *mut mailimf_address_list {
    let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
    addr_list = malloc(::std::mem::size_of::<mailimf_address_list>() as libc::size_t)
        as *mut mailimf_address_list;
    if addr_list.is_null() {
        return 0 as *mut mailimf_address_list;
    }
    (*addr_list).ad_list = ad_list;
    return addr_list;
}
#[no_mangle]
pub unsafe fn mailimf_address_list_free(mut addr_list: *mut mailimf_address_list) {
    clist_foreach(
        (*addr_list).ad_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut mailimf_address) -> ()>, clist_func>(
            Some(mailimf_address_free),
        ),
        0 as *mut libc::c_void,
    );
    clist_free((*addr_list).ad_list);
    free(addr_list as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_body_new(
    mut bd_text: *const libc::c_char,
    mut bd_size: size_t,
) -> *mut mailimf_body {
    let mut body: *mut mailimf_body = 0 as *mut mailimf_body;
    body = malloc(::std::mem::size_of::<mailimf_body>() as libc::size_t) as *mut mailimf_body;
    if body.is_null() {
        return 0 as *mut mailimf_body;
    }
    (*body).bd_text = bd_text;
    (*body).bd_size = bd_size;
    return body;
}
#[no_mangle]
pub unsafe fn mailimf_body_free(mut body: *mut mailimf_body) {
    free(body as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_message_new(
    mut msg_fields: *mut mailimf_fields,
    mut msg_body: *mut mailimf_body,
) -> *mut mailimf_message {
    let mut message: *mut mailimf_message = 0 as *mut mailimf_message;
    message =
        malloc(::std::mem::size_of::<mailimf_message>() as libc::size_t) as *mut mailimf_message;
    if message.is_null() {
        return 0 as *mut mailimf_message;
    }
    (*message).msg_fields = msg_fields;
    (*message).msg_body = msg_body;
    return message;
}
#[no_mangle]
pub unsafe fn mailimf_message_free(mut message: *mut mailimf_message) {
    mailimf_body_free((*message).msg_body);
    mailimf_fields_free((*message).msg_fields);
    free(message as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_fields_free(mut fields: *mut mailimf_fields) {
    if !(*fields).fld_list.is_null() {
        clist_foreach(
            (*fields).fld_list,
            ::std::mem::transmute::<Option<unsafe fn(_: *mut mailimf_field) -> ()>, clist_func>(
                Some(mailimf_field_free),
            ),
            0 as *mut libc::c_void,
        );
        clist_free((*fields).fld_list);
    }
    free(fields as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_field_free(mut field: *mut mailimf_field) {
    match (*field).fld_type {
        1 => {
            mailimf_return_free((*field).fld_data.fld_return_path);
        }
        2 => {
            mailimf_orig_date_free((*field).fld_data.fld_resent_date);
        }
        3 => {
            mailimf_from_free((*field).fld_data.fld_resent_from);
        }
        4 => {
            mailimf_sender_free((*field).fld_data.fld_resent_sender);
        }
        5 => {
            mailimf_to_free((*field).fld_data.fld_resent_to);
        }
        6 => {
            mailimf_cc_free((*field).fld_data.fld_resent_cc);
        }
        7 => {
            mailimf_bcc_free((*field).fld_data.fld_resent_bcc);
        }
        8 => {
            mailimf_message_id_free((*field).fld_data.fld_resent_msg_id);
        }
        9 => {
            mailimf_orig_date_free((*field).fld_data.fld_orig_date);
        }
        10 => {
            mailimf_from_free((*field).fld_data.fld_from);
        }
        11 => {
            mailimf_sender_free((*field).fld_data.fld_sender);
        }
        12 => {
            mailimf_reply_to_free((*field).fld_data.fld_reply_to);
        }
        13 => {
            mailimf_to_free((*field).fld_data.fld_to);
        }
        14 => {
            mailimf_cc_free((*field).fld_data.fld_cc);
        }
        15 => {
            mailimf_bcc_free((*field).fld_data.fld_bcc);
        }
        16 => {
            mailimf_message_id_free((*field).fld_data.fld_message_id);
        }
        17 => {
            mailimf_in_reply_to_free((*field).fld_data.fld_in_reply_to);
        }
        18 => {
            mailimf_references_free((*field).fld_data.fld_references);
        }
        19 => {
            mailimf_subject_free((*field).fld_data.fld_subject);
        }
        20 => {
            mailimf_comments_free((*field).fld_data.fld_comments);
        }
        21 => {
            mailimf_keywords_free((*field).fld_data.fld_keywords);
        }
        22 => {
            mailimf_optional_field_free((*field).fld_data.fld_optional_field);
        }
        _ => {}
    }
    free(field as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_optional_field_free(mut opt_field: *mut mailimf_optional_field) {
    mailimf_field_name_free((*opt_field).fld_name);
    mailimf_unstructured_free((*opt_field).fld_value);
    free(opt_field as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_unstructured_free(mut unstructured: *mut libc::c_char) {
    free(unstructured as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_field_name_free(mut field_name: *mut libc::c_char) {
    free(field_name as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_keywords_free(mut keywords: *mut mailimf_keywords) {
    clist_foreach(
        (*keywords).kw_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut libc::c_char) -> ()>, clist_func>(Some(
            mailimf_phrase_free,
        )),
        0 as *mut libc::c_void,
    );
    clist_free((*keywords).kw_list);
    free(keywords as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_comments_free(mut comments: *mut mailimf_comments) {
    mailimf_unstructured_free((*comments).cm_value);
    free(comments as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_subject_free(mut subject: *mut mailimf_subject) {
    mailimf_unstructured_free((*subject).sbj_value);
    free(subject as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_references_free(mut references: *mut mailimf_references) {
    clist_foreach(
        (*references).mid_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut libc::c_char) -> ()>, clist_func>(Some(
            mailimf_msg_id_free,
        )),
        0 as *mut libc::c_void,
    );
    clist_free((*references).mid_list);
    free(references as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_msg_id_free(mut msg_id: *mut libc::c_char) {
    free(msg_id as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_in_reply_to_free(mut in_reply_to: *mut mailimf_in_reply_to) {
    clist_foreach(
        (*in_reply_to).mid_list,
        ::std::mem::transmute::<Option<unsafe fn(_: *mut libc::c_char) -> ()>, clist_func>(Some(
            mailimf_msg_id_free,
        )),
        0 as *mut libc::c_void,
    );
    clist_free((*in_reply_to).mid_list);
    free(in_reply_to as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_message_id_free(mut message_id: *mut mailimf_message_id) {
    if !(*message_id).mid_value.is_null() {
        mailimf_msg_id_free((*message_id).mid_value);
    }
    free(message_id as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_bcc_free(mut bcc: *mut mailimf_bcc) {
    if !(*bcc).bcc_addr_list.is_null() {
        mailimf_address_list_free((*bcc).bcc_addr_list);
    }
    free(bcc as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_cc_free(mut cc: *mut mailimf_cc) {
    if !(*cc).cc_addr_list.is_null() {
        mailimf_address_list_free((*cc).cc_addr_list);
    }
    free(cc as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_to_free(mut to: *mut mailimf_to) {
    if !(*to).to_addr_list.is_null() {
        mailimf_address_list_free((*to).to_addr_list);
    }
    free(to as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_reply_to_free(mut reply_to: *mut mailimf_reply_to) {
    if !(*reply_to).rt_addr_list.is_null() {
        mailimf_address_list_free((*reply_to).rt_addr_list);
    }
    free(reply_to as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_sender_free(mut sender: *mut mailimf_sender) {
    if !(*sender).snd_mb.is_null() {
        mailimf_mailbox_free((*sender).snd_mb);
    }
    free(sender as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_from_free(mut from: *mut mailimf_from) {
    if !(*from).frm_mb_list.is_null() {
        mailimf_mailbox_list_free((*from).frm_mb_list);
    }
    free(from as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_orig_date_free(mut orig_date: *mut mailimf_orig_date) {
    if !(*orig_date).dt_date_time.is_null() {
        mailimf_date_time_free((*orig_date).dt_date_time);
    }
    free(orig_date as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_return_free(mut return_path: *mut mailimf_return) {
    mailimf_path_free((*return_path).ret_path);
    free(return_path as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_path_free(mut path: *mut mailimf_path) {
    if !(*path).pt_addr_spec.is_null() {
        mailimf_addr_spec_free((*path).pt_addr_spec);
    }
    free(path as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_fields_new(mut fld_list: *mut clist) -> *mut mailimf_fields {
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    fields = malloc(::std::mem::size_of::<mailimf_fields>() as libc::size_t) as *mut mailimf_fields;
    if fields.is_null() {
        return 0 as *mut mailimf_fields;
    }
    (*fields).fld_list = fld_list;
    return fields;
}
#[no_mangle]
pub unsafe fn mailimf_field_new(
    mut fld_type: libc::c_int,
    mut fld_return_path: *mut mailimf_return,
    mut fld_resent_date: *mut mailimf_orig_date,
    mut fld_resent_from: *mut mailimf_from,
    mut fld_resent_sender: *mut mailimf_sender,
    mut fld_resent_to: *mut mailimf_to,
    mut fld_resent_cc: *mut mailimf_cc,
    mut fld_resent_bcc: *mut mailimf_bcc,
    mut fld_resent_msg_id: *mut mailimf_message_id,
    mut fld_orig_date: *mut mailimf_orig_date,
    mut fld_from: *mut mailimf_from,
    mut fld_sender: *mut mailimf_sender,
    mut fld_reply_to: *mut mailimf_reply_to,
    mut fld_to: *mut mailimf_to,
    mut fld_cc: *mut mailimf_cc,
    mut fld_bcc: *mut mailimf_bcc,
    mut fld_message_id: *mut mailimf_message_id,
    mut fld_in_reply_to: *mut mailimf_in_reply_to,
    mut fld_references: *mut mailimf_references,
    mut fld_subject: *mut mailimf_subject,
    mut fld_comments: *mut mailimf_comments,
    mut fld_keywords: *mut mailimf_keywords,
    mut fld_optional_field: *mut mailimf_optional_field,
) -> *mut mailimf_field {
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    field = malloc(::std::mem::size_of::<mailimf_field>() as libc::size_t) as *mut mailimf_field;
    if field.is_null() {
        return 0 as *mut mailimf_field;
    }
    (*field).fld_type = fld_type;
    match fld_type {
        1 => (*field).fld_data.fld_return_path = fld_return_path,
        2 => (*field).fld_data.fld_resent_date = fld_resent_date,
        3 => (*field).fld_data.fld_resent_from = fld_resent_from,
        4 => (*field).fld_data.fld_resent_sender = fld_resent_sender,
        5 => (*field).fld_data.fld_resent_to = fld_resent_to,
        6 => (*field).fld_data.fld_resent_cc = fld_resent_cc,
        7 => (*field).fld_data.fld_resent_bcc = fld_resent_bcc,
        8 => (*field).fld_data.fld_resent_msg_id = fld_resent_msg_id,
        9 => (*field).fld_data.fld_orig_date = fld_orig_date,
        10 => (*field).fld_data.fld_from = fld_from,
        11 => (*field).fld_data.fld_sender = fld_sender,
        12 => (*field).fld_data.fld_reply_to = fld_reply_to,
        13 => (*field).fld_data.fld_to = fld_to,
        14 => (*field).fld_data.fld_cc = fld_cc,
        15 => (*field).fld_data.fld_bcc = fld_bcc,
        16 => (*field).fld_data.fld_message_id = fld_message_id,
        17 => (*field).fld_data.fld_in_reply_to = fld_in_reply_to,
        18 => (*field).fld_data.fld_references = fld_references,
        19 => (*field).fld_data.fld_subject = fld_subject,
        20 => (*field).fld_data.fld_comments = fld_comments,
        21 => (*field).fld_data.fld_keywords = fld_keywords,
        22 => (*field).fld_data.fld_optional_field = fld_optional_field,
        _ => {}
    }
    return field;
}
#[no_mangle]
pub unsafe fn mailimf_orig_date_new(
    mut dt_date_time: *mut mailimf_date_time,
) -> *mut mailimf_orig_date {
    let mut orig_date: *mut mailimf_orig_date = 0 as *mut mailimf_orig_date;
    orig_date = malloc(::std::mem::size_of::<mailimf_orig_date>() as libc::size_t)
        as *mut mailimf_orig_date;
    if orig_date.is_null() {
        return 0 as *mut mailimf_orig_date;
    }
    (*orig_date).dt_date_time = dt_date_time;
    return orig_date;
}
#[no_mangle]
pub unsafe fn mailimf_from_new(mut frm_mb_list: *mut mailimf_mailbox_list) -> *mut mailimf_from {
    let mut from: *mut mailimf_from = 0 as *mut mailimf_from;
    from = malloc(::std::mem::size_of::<mailimf_from>() as libc::size_t) as *mut mailimf_from;
    if from.is_null() {
        return 0 as *mut mailimf_from;
    }
    (*from).frm_mb_list = frm_mb_list;
    return from;
}
#[no_mangle]
pub unsafe fn mailimf_sender_new(mut snd_mb: *mut mailimf_mailbox) -> *mut mailimf_sender {
    let mut sender: *mut mailimf_sender = 0 as *mut mailimf_sender;
    sender = malloc(::std::mem::size_of::<mailimf_sender>() as libc::size_t) as *mut mailimf_sender;
    if sender.is_null() {
        return 0 as *mut mailimf_sender;
    }
    (*sender).snd_mb = snd_mb;
    return sender;
}
#[no_mangle]
pub unsafe fn mailimf_reply_to_new(
    mut rt_addr_list: *mut mailimf_address_list,
) -> *mut mailimf_reply_to {
    let mut reply_to: *mut mailimf_reply_to = 0 as *mut mailimf_reply_to;
    reply_to =
        malloc(::std::mem::size_of::<mailimf_reply_to>() as libc::size_t) as *mut mailimf_reply_to;
    if reply_to.is_null() {
        return 0 as *mut mailimf_reply_to;
    }
    (*reply_to).rt_addr_list = rt_addr_list;
    return reply_to;
}
#[no_mangle]
pub unsafe fn mailimf_to_new(mut to_addr_list: *mut mailimf_address_list) -> *mut mailimf_to {
    let mut to: *mut mailimf_to = 0 as *mut mailimf_to;
    to = malloc(::std::mem::size_of::<mailimf_to>() as libc::size_t) as *mut mailimf_to;
    if to.is_null() {
        return 0 as *mut mailimf_to;
    }
    (*to).to_addr_list = to_addr_list;
    return to;
}
#[no_mangle]
pub unsafe fn mailimf_cc_new(mut cc_addr_list: *mut mailimf_address_list) -> *mut mailimf_cc {
    let mut cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
    cc = malloc(::std::mem::size_of::<mailimf_cc>() as libc::size_t) as *mut mailimf_cc;
    if cc.is_null() {
        return 0 as *mut mailimf_cc;
    }
    (*cc).cc_addr_list = cc_addr_list;
    return cc;
}
#[no_mangle]
pub unsafe fn mailimf_bcc_new(mut bcc_addr_list: *mut mailimf_address_list) -> *mut mailimf_bcc {
    let mut bcc: *mut mailimf_bcc = 0 as *mut mailimf_bcc;
    bcc = malloc(::std::mem::size_of::<mailimf_bcc>() as libc::size_t) as *mut mailimf_bcc;
    if bcc.is_null() {
        return 0 as *mut mailimf_bcc;
    }
    (*bcc).bcc_addr_list = bcc_addr_list;
    return bcc;
}
#[no_mangle]
pub unsafe fn mailimf_message_id_new(mut mid_value: *mut libc::c_char) -> *mut mailimf_message_id {
    let mut message_id: *mut mailimf_message_id = 0 as *mut mailimf_message_id;
    message_id = malloc(::std::mem::size_of::<mailimf_message_id>() as libc::size_t)
        as *mut mailimf_message_id;
    if message_id.is_null() {
        return 0 as *mut mailimf_message_id;
    }
    (*message_id).mid_value = mid_value;
    return message_id;
}
#[no_mangle]
pub unsafe fn mailimf_in_reply_to_new(mut mid_list: *mut clist) -> *mut mailimf_in_reply_to {
    let mut in_reply_to: *mut mailimf_in_reply_to = 0 as *mut mailimf_in_reply_to;
    in_reply_to = malloc(::std::mem::size_of::<mailimf_in_reply_to>() as libc::size_t)
        as *mut mailimf_in_reply_to;
    if in_reply_to.is_null() {
        return 0 as *mut mailimf_in_reply_to;
    }
    (*in_reply_to).mid_list = mid_list;
    return in_reply_to;
}
/* != NULL */
#[no_mangle]
pub unsafe fn mailimf_references_new(mut mid_list: *mut clist) -> *mut mailimf_references {
    let mut ref_0: *mut mailimf_references = 0 as *mut mailimf_references;
    ref_0 = malloc(::std::mem::size_of::<mailimf_references>() as libc::size_t)
        as *mut mailimf_references;
    if ref_0.is_null() {
        return 0 as *mut mailimf_references;
    }
    (*ref_0).mid_list = mid_list;
    return ref_0;
}
#[no_mangle]
pub unsafe fn mailimf_subject_new(mut sbj_value: *mut libc::c_char) -> *mut mailimf_subject {
    let mut subject: *mut mailimf_subject = 0 as *mut mailimf_subject;
    subject =
        malloc(::std::mem::size_of::<mailimf_subject>() as libc::size_t) as *mut mailimf_subject;
    if subject.is_null() {
        return 0 as *mut mailimf_subject;
    }
    (*subject).sbj_value = sbj_value;
    return subject;
}
#[no_mangle]
pub unsafe fn mailimf_comments_new(mut cm_value: *mut libc::c_char) -> *mut mailimf_comments {
    let mut comments: *mut mailimf_comments = 0 as *mut mailimf_comments;
    comments =
        malloc(::std::mem::size_of::<mailimf_comments>() as libc::size_t) as *mut mailimf_comments;
    if comments.is_null() {
        return 0 as *mut mailimf_comments;
    }
    (*comments).cm_value = cm_value;
    return comments;
}
#[no_mangle]
pub unsafe fn mailimf_keywords_new(mut kw_list: *mut clist) -> *mut mailimf_keywords {
    let mut keywords: *mut mailimf_keywords = 0 as *mut mailimf_keywords;
    keywords =
        malloc(::std::mem::size_of::<mailimf_keywords>() as libc::size_t) as *mut mailimf_keywords;
    if keywords.is_null() {
        return 0 as *mut mailimf_keywords;
    }
    (*keywords).kw_list = kw_list;
    return keywords;
}
#[no_mangle]
pub unsafe fn mailimf_return_new(mut ret_path: *mut mailimf_path) -> *mut mailimf_return {
    let mut return_path: *mut mailimf_return = 0 as *mut mailimf_return;
    return_path =
        malloc(::std::mem::size_of::<mailimf_return>() as libc::size_t) as *mut mailimf_return;
    if return_path.is_null() {
        return 0 as *mut mailimf_return;
    }
    (*return_path).ret_path = ret_path;
    return return_path;
}
#[no_mangle]
pub unsafe fn mailimf_path_new(mut pt_addr_spec: *mut libc::c_char) -> *mut mailimf_path {
    let mut path: *mut mailimf_path = 0 as *mut mailimf_path;
    path = malloc(::std::mem::size_of::<mailimf_path>() as libc::size_t) as *mut mailimf_path;
    if path.is_null() {
        return 0 as *mut mailimf_path;
    }
    (*path).pt_addr_spec = pt_addr_spec;
    return path;
}
#[no_mangle]
pub unsafe fn mailimf_optional_field_new(
    mut fld_name: *mut libc::c_char,
    mut fld_value: *mut libc::c_char,
) -> *mut mailimf_optional_field {
    let mut opt_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    opt_field = malloc(::std::mem::size_of::<mailimf_optional_field>() as libc::size_t)
        as *mut mailimf_optional_field;
    if opt_field.is_null() {
        return 0 as *mut mailimf_optional_field;
    }
    (*opt_field).fld_name = fld_name;
    (*opt_field).fld_value = fld_value;
    return opt_field;
}
/* internal use */
#[no_mangle]
pub unsafe fn mailimf_atom_free(mut atom: *mut libc::c_char) {
    free(atom as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_dot_atom_free(mut dot_atom: *mut libc::c_char) {
    free(dot_atom as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_dot_atom_text_free(mut dot_atom: *mut libc::c_char) {
    free(dot_atom as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_quoted_string_free(mut quoted_string: *mut libc::c_char) {
    free(quoted_string as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_word_free(mut word: *mut libc::c_char) {
    free(word as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_angle_addr_free(mut angle_addr: *mut libc::c_char) {
    free(angle_addr as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_local_part_free(mut local_part: *mut libc::c_char) {
    free(local_part as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_domain_free(mut domain: *mut libc::c_char) {
    free(domain as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_domain_literal_free(mut domain_literal: *mut libc::c_char) {
    free(domain_literal as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_id_left_free(mut id_left: *mut libc::c_char) {
    free(id_left as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_id_right_free(mut id_right: *mut libc::c_char) {
    free(id_right as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_no_fold_quote_free(mut nfq: *mut libc::c_char) {
    free(nfq as *mut libc::c_void);
}
#[no_mangle]
pub unsafe fn mailimf_no_fold_literal_free(mut nfl: *mut libc::c_char) {
    free(nfl as *mut libc::c_void);
}

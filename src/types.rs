use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::constants::Event;
use crate::dc_context::dc_context_t;
use crate::x::*;

pub use libc::{dirent, tm, DIR, FILE};
pub use libsqlite3_sys::*;

extern "C" {
    pub type __sFILEX;

    pub type _telldir;
    pub type mailstream_cancel;
}

pub type sqlite_int64 = libc::int64_t;
pub type sqlite3_int64 = sqlite_int64;

pub type useconds_t = libc::useconds_t;
pub type int32_t = libc::int32_t;
pub type int64_t = libc::int64_t;
pub type uintptr_t = libc::uintptr_t;
pub type __uint8_t = libc::uint8_t;
pub type __uint16_t = libc::uint16_t;
pub type __int32_t = libc::int32_t;
pub type __uint64_t = libc::uint64_t;

pub type time_t = libc::time_t;
pub type pid_t = libc::pid_t;
pub type size_t = libc::size_t;
pub type ssize_t = libc::ssize_t;
pub type uint32_t = libc::c_uint;
pub type uint8_t = libc::c_uchar;
pub type uint16_t = libc::c_ushort;

pub type __uint32_t = libc::c_uint;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct carray_s {
    pub array: *mut *mut libc::c_void,
    pub len: libc::c_uint,
    pub max: libc::c_uint,
}
pub type carray = carray_s;

pub type mailimap_msg_att_handler =
    unsafe extern "C" fn(_: *mut mailimap_msg_att, _: *mut libc::c_void) -> ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap {
    pub imap_response: *mut libc::c_char,
    pub imap_stream: *mut mailstream,
    pub imap_progr_rate: size_t,
    pub imap_progr_fun: Option<unsafe extern "C" fn(_: size_t, _: size_t) -> ()>,
    pub imap_stream_buffer: *mut MMAPString,
    pub imap_response_buffer: *mut MMAPString,
    pub imap_state: libc::c_int,
    pub imap_tag: libc::c_int,
    pub imap_connection_info: *mut mailimap_connection_info,
    pub imap_selection_info: *mut mailimap_selection_info,
    pub imap_response_info: *mut mailimap_response_info,
    pub imap_sasl: unnamed_2,
    pub imap_idle_timestamp: time_t,
    pub imap_idle_maxdelay: time_t,
    pub imap_body_progress_fun:
        Option<unsafe extern "C" fn(_: size_t, _: size_t, _: *mut libc::c_void) -> ()>,
    pub imap_items_progress_fun:
        Option<unsafe extern "C" fn(_: size_t, _: size_t, _: *mut libc::c_void) -> ()>,
    pub imap_progress_context: *mut libc::c_void,
    pub imap_msg_att_handler:
        Option<unsafe extern "C" fn(_: *mut mailimap_msg_att, _: *mut libc::c_void) -> ()>,
    pub imap_msg_att_handler_context: *mut libc::c_void,
    pub imap_msg_body_handler: Option<
        unsafe extern "C" fn(
            _: libc::c_int,
            _: *mut mailimap_msg_att_body_section,
            _: *const libc::c_char,
            _: size_t,
            _: *mut libc::c_void,
        ) -> bool,
    >,
    pub imap_msg_body_handler_context: *mut libc::c_void,
    pub imap_timeout: time_t,
    pub imap_logger: Option<
        unsafe extern "C" fn(
            _: *mut mailimap,
            _: libc::c_int,
            _: *const libc::c_char,
            _: size_t,
            _: *mut libc::c_void,
        ) -> (),
    >,
    pub imap_logger_context: *mut libc::c_void,
    pub is_163_workaround_enabled: libc::c_int,
    pub is_rambler_workaround_enabled: libc::c_int,
    pub is_qip_workaround_enabled: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct _mailstream {
    pub buffer_max_size: size_t,
    pub write_buffer: *mut libc::c_char,
    pub write_buffer_len: size_t,
    pub read_buffer: *mut libc::c_char,
    pub read_buffer_len: size_t,
    pub low: *mut mailstream_low,
    pub idle: *mut mailstream_cancel,
    pub idling: libc::c_int,
    pub logger: Option<
        unsafe extern "C" fn(
            _: *mut mailstream,
            _: libc::c_int,
            _: *const libc::c_char,
            _: size_t,
            _: *mut libc::c_void,
        ) -> (),
    >,
    pub logger_context: *mut libc::c_void,
}
pub type mailstream = _mailstream;
pub type mailstream_low = _mailstream_low;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _mailstream_low {
    pub data: *mut libc::c_void,
    pub driver: *mut mailstream_low_driver,
    pub privacy: libc::c_int,
    pub identifier: *mut libc::c_char,
    pub timeout: libc::c_ulong,
    pub logger: Option<
        unsafe extern "C" fn(
            _: *mut mailstream_low,
            _: libc::c_int,
            _: *const libc::c_char,
            _: size_t,
            _: *mut libc::c_void,
        ) -> (),
    >,
    pub logger_context: *mut libc::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailstream_low_driver {
    pub mailstream_read: Option<
        unsafe extern "C" fn(_: *mut mailstream_low, _: *mut libc::c_void, _: size_t) -> ssize_t,
    >,
    pub mailstream_write: Option<
        unsafe extern "C" fn(_: *mut mailstream_low, _: *const libc::c_void, _: size_t) -> ssize_t,
    >,
    pub mailstream_close: Option<unsafe extern "C" fn(_: *mut mailstream_low) -> libc::c_int>,
    pub mailstream_get_fd: Option<unsafe extern "C" fn(_: *mut mailstream_low) -> libc::c_int>,
    pub mailstream_free: Option<unsafe extern "C" fn(_: *mut mailstream_low) -> ()>,
    pub mailstream_cancel: Option<unsafe extern "C" fn(_: *mut mailstream_low) -> ()>,
    pub mailstream_get_cancel:
        Option<unsafe extern "C" fn(_: *mut mailstream_low) -> *mut mailstream_cancel>,
    pub mailstream_get_certificate_chain:
        Option<unsafe extern "C" fn(_: *mut mailstream_low) -> *mut carray>,
    pub mailstream_setup_idle: Option<unsafe extern "C" fn(_: *mut mailstream_low) -> libc::c_int>,
    pub mailstream_unsetup_idle:
        Option<unsafe extern "C" fn(_: *mut mailstream_low) -> libc::c_int>,
    pub mailstream_interrupt_idle:
        Option<unsafe extern "C" fn(_: *mut mailstream_low) -> libc::c_int>,
}
pub type progress_function = unsafe extern "C" fn(_: size_t, _: size_t) -> ();
pub type mailprogress_function =
    unsafe extern "C" fn(_: size_t, _: size_t, _: *mut libc::c_void) -> ();
pub type unnamed = libc::c_uint;
pub const MAILSTREAM_IDLE_CANCELLED: unnamed = 4;
pub const MAILSTREAM_IDLE_TIMEOUT: unnamed = 3;
pub const MAILSTREAM_IDLE_HASDATA: unnamed = 2;
pub const MAILSTREAM_IDLE_INTERRUPTED: unnamed = 1;
pub const MAILSTREAM_IDLE_ERROR: unnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _MMAPString {
    pub str_0: *mut libc::c_char,
    pub len: size_t,
    pub allocated_len: size_t,
    pub fd: libc::c_int,
    pub mmapped_size: size_t,
}
pub type MMAPString = _MMAPString;
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
pub struct mailsmtp {
    pub stream: *mut mailstream,
    pub progr_rate: size_t,
    pub progr_fun: Option<unsafe extern "C" fn(_: size_t, _: size_t) -> ()>,
    pub response: *mut libc::c_char,
    pub line_buffer: *mut MMAPString,
    pub response_buffer: *mut MMAPString,
    pub esmtp: libc::c_int,
    pub auth: libc::c_int,
    pub smtp_sasl: unnamed_0,
    pub smtp_max_msg_size: size_t,
    pub smtp_progress_fun:
        Option<unsafe extern "C" fn(_: size_t, _: size_t, _: *mut libc::c_void) -> ()>,
    pub smtp_progress_context: *mut libc::c_void,
    pub response_code: libc::c_int,
    pub smtp_timeout: time_t,
    pub smtp_logger: Option<
        unsafe extern "C" fn(
            _: *mut mailsmtp,
            _: libc::c_int,
            _: *const libc::c_char,
            _: size_t,
            _: *mut libc::c_void,
        ) -> (),
    >,
    pub smtp_logger_context: *mut libc::c_void,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_0 {
    pub sasl_conn: *mut libc::c_void,
    pub sasl_server_fqdn: *const libc::c_char,
    pub sasl_login: *const libc::c_char,
    pub sasl_auth_name: *const libc::c_char,
    pub sasl_password: *const libc::c_char,
    pub sasl_realm: *const libc::c_char,
    pub sasl_secret: *mut libc::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body {
    pub bd_type: libc::c_int,
    pub bd_data: unnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_1 {
    pub bd_body_1part: *mut mailimap_body_type_1part,
    pub bd_body_mpart: *mut mailimap_body_type_mpart,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_type_mpart {
    pub bd_list: *mut clist,
    pub bd_media_subtype: *mut libc::c_char,
    pub bd_ext_mpart: *mut mailimap_body_ext_mpart,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_ext_mpart {
    pub bd_parameter: *mut mailimap_body_fld_param,
    pub bd_disposition: *mut mailimap_body_fld_dsp,
    pub bd_language: *mut mailimap_body_fld_lang,
    pub bd_loc: *mut libc::c_char,
    pub bd_extension_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_fld_lang {
    pub lg_type: libc::c_int,
    pub lg_data: unnamed_2,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_2 {
    pub lg_single: *mut libc::c_char,
    pub lg_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_fld_dsp {
    pub dsp_type: *mut libc::c_char,
    pub dsp_attributes: *mut mailimap_body_fld_param,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_fld_param {
    pub pa_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_type_1part {
    pub bd_type: libc::c_int,
    pub bd_data: unnamed_3,
    pub bd_ext_1part: *mut mailimap_body_ext_1part,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_ext_1part {
    pub bd_md5: *mut libc::c_char,
    pub bd_disposition: *mut mailimap_body_fld_dsp,
    pub bd_language: *mut mailimap_body_fld_lang,
    pub bd_loc: *mut libc::c_char,
    pub bd_extension_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_3 {
    pub bd_type_basic: *mut mailimap_body_type_basic,
    pub bd_type_msg: *mut mailimap_body_type_msg,
    pub bd_type_text: *mut mailimap_body_type_text,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_type_text {
    pub bd_media_text: *mut libc::c_char,
    pub bd_fields: *mut mailimap_body_fields,
    pub bd_lines: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_fields {
    pub bd_parameter: *mut mailimap_body_fld_param,
    pub bd_id: *mut libc::c_char,
    pub bd_description: *mut libc::c_char,
    pub bd_encoding: *mut mailimap_body_fld_enc,
    pub bd_size: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_fld_enc {
    pub enc_type: libc::c_int,
    pub enc_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_type_msg {
    pub bd_fields: *mut mailimap_body_fields,
    pub bd_envelope: *mut mailimap_envelope,
    pub bd_body: *mut mailimap_body,
    pub bd_lines: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_envelope {
    pub env_date: *mut libc::c_char,
    pub env_subject: *mut libc::c_char,
    pub env_from: *mut mailimap_env_from,
    pub env_sender: *mut mailimap_env_sender,
    pub env_reply_to: *mut mailimap_env_reply_to,
    pub env_to: *mut mailimap_env_to,
    pub env_cc: *mut mailimap_env_cc,
    pub env_bcc: *mut mailimap_env_bcc,
    pub env_in_reply_to: *mut libc::c_char,
    pub env_message_id: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_env_bcc {
    pub bcc_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_env_cc {
    pub cc_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_env_to {
    pub to_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_env_reply_to {
    pub rt_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_env_sender {
    pub snd_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_env_from {
    pub frm_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_body_type_basic {
    pub bd_media_basic: *mut mailimap_media_basic,
    pub bd_fields: *mut mailimap_body_fields,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_media_basic {
    pub med_type: libc::c_int,
    pub med_basic_type: *mut libc::c_char,
    pub med_subtype: *mut libc::c_char,
}
pub type unnamed_4 = libc::c_uint;
pub const MAILIMAP_CAPABILITY_NAME: unnamed_4 = 1;
pub const MAILIMAP_CAPABILITY_AUTH_TYPE: unnamed_4 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_capability {
    pub cap_type: libc::c_int,
    pub cap_data: unnamed_5,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_5 {
    pub cap_auth_type: *mut libc::c_char,
    pub cap_name: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_capability_data {
    pub cap_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_extension_data {
    pub ext_extension: *mut mailimap_extension_api,
    pub ext_type: libc::c_int,
    pub ext_data: *mut libc::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_extension_api {
    pub ext_name: *mut libc::c_char,
    pub ext_id: libc::c_int,
    pub ext_parser: Option<
        unsafe extern "C" fn(
            _: libc::c_int,
            _: *mut mailstream,
            _: *mut MMAPString,
            _: *mut mailimap_parser_context,
            _: *mut size_t,
            _: *mut *mut mailimap_extension_data,
            _: size_t,
            _: Option<unsafe extern "C" fn(_: size_t, _: size_t) -> ()>,
        ) -> libc::c_int,
    >,
    pub ext_free: Option<unsafe extern "C" fn(_: *mut mailimap_extension_data) -> ()>,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_parser_context {
    pub is_rambler_workaround_enabled: libc::c_int,
    pub is_qip_workaround_enabled: libc::c_int,
    pub msg_body_handler: Option<
        unsafe extern "C" fn(
            _: libc::c_int,
            _: *mut mailimap_msg_att_body_section,
            _: *const libc::c_char,
            _: size_t,
            _: *mut libc::c_void,
        ) -> bool,
    >,
    pub msg_body_handler_context: *mut libc::c_void,
    pub msg_body_section: *mut mailimap_msg_att_body_section,
    pub msg_body_att_type: libc::c_int,
    pub msg_body_parse_in_progress: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_msg_att_body_section {
    pub sec_section: *mut mailimap_section,
    pub sec_origin_octet: uint32_t,
    pub sec_body_part: *mut libc::c_char,
    pub sec_length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_section {
    pub sec_spec: *mut mailimap_section_spec,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_section_spec {
    pub sec_type: libc::c_int,
    pub sec_data: unnamed_6,
    pub sec_text: *mut mailimap_section_text,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_section_text {
    pub sec_type: libc::c_int,
    pub sec_msgtext: *mut mailimap_section_msgtext,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_section_msgtext {
    pub sec_type: libc::c_int,
    pub sec_header_list: *mut mailimap_header_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_header_list {
    pub hdr_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_6 {
    pub sec_msgtext: *mut mailimap_section_msgtext,
    pub sec_part: *mut mailimap_section_part,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_section_part {
    pub sec_id: *mut clist,
}
pub type mailimap_msg_body_handler = unsafe fn(
    _: libc::c_int,
    _: *mut mailimap_msg_att_body_section,
    _: *const libc::c_char,
    _: size_t,
    _: *mut libc::c_void,
) -> bool;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_date_time {
    pub dt_day: libc::c_int,
    pub dt_month: libc::c_int,
    pub dt_year: libc::c_int,
    pub dt_hour: libc::c_int,
    pub dt_min: libc::c_int,
    pub dt_sec: libc::c_int,
    pub dt_zone: libc::c_int,
}
pub type unnamed_7 = libc::c_uint;
pub const MAILIMAP_FLAG_EXTENSION: unnamed_7 = 6;
pub const MAILIMAP_FLAG_KEYWORD: unnamed_7 = 5;
pub const MAILIMAP_FLAG_DRAFT: unnamed_7 = 4;
pub const MAILIMAP_FLAG_SEEN: unnamed_7 = 3;
pub const MAILIMAP_FLAG_DELETED: unnamed_7 = 2;
pub const MAILIMAP_FLAG_FLAGGED: unnamed_7 = 1;
pub const MAILIMAP_FLAG_ANSWERED: unnamed_7 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_flag {
    pub fl_type: libc::c_int,
    pub fl_data: unnamed_8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_8 {
    pub fl_keyword: *mut libc::c_char,
    pub fl_extension: *mut libc::c_char,
}
pub type unnamed_9 = libc::c_uint;
pub const MAILIMAP_FLAG_FETCH_OTHER: unnamed_9 = 2;
pub const MAILIMAP_FLAG_FETCH_RECENT: unnamed_9 = 1;
pub const MAILIMAP_FLAG_FETCH_ERROR: unnamed_9 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_flag_fetch {
    pub fl_type: libc::c_int,
    pub fl_flag: *mut mailimap_flag,
}
pub type unnamed_10 = libc::c_uint;
pub const MAILIMAP_FLAG_PERM_ALL: unnamed_10 = 2;
pub const MAILIMAP_FLAG_PERM_FLAG: unnamed_10 = 1;
pub const MAILIMAP_FLAG_PERM_ERROR: unnamed_10 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_flag_perm {
    pub fl_type: libc::c_int,
    pub fl_flag: *mut mailimap_flag,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_flag_list {
    pub fl_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_mailbox_data_status {
    pub st_mailbox: *mut libc::c_char,
    pub st_info_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_msg_att {
    pub att_list: *mut clist,
    pub att_number: uint32_t,
}
pub type unnamed_11 = libc::c_uint;
pub const MAILIMAP_MSG_ATT_ITEM_EXTENSION: unnamed_11 = 3;
pub const MAILIMAP_MSG_ATT_ITEM_STATIC: unnamed_11 = 2;
pub const MAILIMAP_MSG_ATT_ITEM_DYNAMIC: unnamed_11 = 1;
pub const MAILIMAP_MSG_ATT_ITEM_ERROR: unnamed_11 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_msg_att_item {
    pub att_type: libc::c_int,
    pub att_data: unnamed_12,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_12 {
    pub att_dyn: *mut mailimap_msg_att_dynamic,
    pub att_static: *mut mailimap_msg_att_static,
    pub att_extension_data: *mut mailimap_extension_data,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_msg_att_static {
    pub att_type: libc::c_int,
    pub att_data: unnamed_13,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_13 {
    pub att_env: *mut mailimap_envelope,
    pub att_internal_date: *mut mailimap_date_time,
    pub att_rfc822: unnamed_16,
    pub att_rfc822_header: unnamed_15,
    pub att_rfc822_text: unnamed_14,
    pub att_rfc822_size: uint32_t,
    pub att_bodystructure: *mut mailimap_body,
    pub att_body: *mut mailimap_body,
    pub att_body_section: *mut mailimap_msg_att_body_section,
    pub att_uid: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_14 {
    pub att_content: *mut libc::c_char,
    pub att_length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_15 {
    pub att_content: *mut libc::c_char,
    pub att_length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_16 {
    pub att_content: *mut libc::c_char,
    pub att_length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_msg_att_dynamic {
    pub att_list: *mut clist,
}
pub type unnamed_17 = libc::c_uint;
pub const MAILIMAP_MSG_ATT_UID: unnamed_17 = 10;
pub const MAILIMAP_MSG_ATT_BODY_SECTION: unnamed_17 = 9;
pub const MAILIMAP_MSG_ATT_BODYSTRUCTURE: unnamed_17 = 8;
pub const MAILIMAP_MSG_ATT_BODY: unnamed_17 = 7;
pub const MAILIMAP_MSG_ATT_RFC822_SIZE: unnamed_17 = 6;
pub const MAILIMAP_MSG_ATT_RFC822_TEXT: unnamed_17 = 5;
pub const MAILIMAP_MSG_ATT_RFC822_HEADER: unnamed_17 = 4;
pub const MAILIMAP_MSG_ATT_RFC822: unnamed_17 = 3;
pub const MAILIMAP_MSG_ATT_INTERNALDATE: unnamed_17 = 2;
pub const MAILIMAP_MSG_ATT_ENVELOPE: unnamed_17 = 1;
pub const MAILIMAP_MSG_ATT_ERROR: unnamed_17 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_set_item {
    pub set_first: uint32_t,
    pub set_last: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_set {
    pub set_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_fetch_att {
    pub att_type: libc::c_int,
    pub att_section: *mut mailimap_section,
    pub att_offset: uint32_t,
    pub att_size: uint32_t,
    pub att_extension: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_fetch_type {
    pub ft_type: libc::c_int,
    pub ft_data: unnamed_18,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_18 {
    pub ft_fetch_att: *mut mailimap_fetch_att,
    pub ft_fetch_att_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_store_att_flags {
    pub fl_sign: libc::c_int,
    pub fl_silent: libc::c_int,
    pub fl_flag_list: *mut mailimap_flag_list,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_19 {
    pub sasl_conn: *mut libc::c_void,
    pub sasl_server_fqdn: *const libc::c_char,
    pub sasl_login: *const libc::c_char,
    pub sasl_auth_name: *const libc::c_char,
    pub sasl_password: *const libc::c_char,
    pub sasl_realm: *const libc::c_char,
    pub sasl_secret: *mut libc::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_response_info {
    pub rsp_alert: *mut libc::c_char,
    pub rsp_parse: *mut libc::c_char,
    pub rsp_badcharset: *mut clist,
    pub rsp_trycreate: libc::c_int,
    pub rsp_mailbox_list: *mut clist,
    pub rsp_mailbox_lsub: *mut clist,
    pub rsp_search_result: *mut clist,
    pub rsp_status: *mut mailimap_mailbox_data_status,
    pub rsp_expunged: *mut clist,
    pub rsp_fetch_list: *mut clist,
    pub rsp_extension_list: *mut clist,
    pub rsp_atom: *mut libc::c_char,
    pub rsp_value: *mut libc::c_char,
}
#[derive(BitfieldStruct, Clone, Copy)]
#[repr(C)]
pub struct mailimap_selection_info {
    pub sel_perm_flags: *mut clist,
    pub sel_perm: libc::c_int,
    pub sel_uidnext: uint32_t,
    pub sel_uidvalidity: uint32_t,
    pub sel_first_unseen: uint32_t,
    pub sel_flags: *mut mailimap_flag_list,
    pub sel_exists: uint32_t,
    pub sel_recent: uint32_t,
    pub sel_unseen: uint32_t,
    #[bitfield(name = "sel_has_exists", ty = "uint8_t", bits = "0..=0")]
    #[bitfield(name = "sel_has_recent", ty = "uint8_t", bits = "1..=1")]
    pub sel_has_exists_sel_has_recent: [u8; 1],
    pub _pad: [u8; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_connection_info {
    pub imap_capability: *mut mailimap_capability_data,
}
pub type unnamed_20 = libc::c_uint;
pub const MAILIMAP_ERROR_CLIENTID: unnamed_20 = 46;
pub const MAILIMAP_ERROR_CUSTOM_COMMAND: unnamed_20 = 45;
pub const MAILIMAP_ERROR_NEEDS_MORE_DATA: unnamed_20 = 44;
pub const MAILIMAP_ERROR_SSL: unnamed_20 = 43;
pub const MAILIMAP_ERROR_SASL: unnamed_20 = 42;
pub const MAILIMAP_ERROR_EXTENSION: unnamed_20 = 41;
pub const MAILIMAP_ERROR_INVAL: unnamed_20 = 40;
pub const MAILIMAP_ERROR_STARTTLS: unnamed_20 = 39;
pub const MAILIMAP_ERROR_UNSUBSCRIBE: unnamed_20 = 38;
pub const MAILIMAP_ERROR_SUBSCRIBE: unnamed_20 = 37;
pub const MAILIMAP_ERROR_UID_STORE: unnamed_20 = 36;
pub const MAILIMAP_ERROR_STORE: unnamed_20 = 35;
pub const MAILIMAP_ERROR_STATUS: unnamed_20 = 34;
pub const MAILIMAP_ERROR_SELECT: unnamed_20 = 33;
pub const MAILIMAP_ERROR_UID_SEARCH: unnamed_20 = 32;
pub const MAILIMAP_ERROR_SEARCH: unnamed_20 = 31;
pub const MAILIMAP_ERROR_RENAME: unnamed_20 = 30;
pub const MAILIMAP_ERROR_LSUB: unnamed_20 = 29;
pub const MAILIMAP_ERROR_LOGIN: unnamed_20 = 28;
pub const MAILIMAP_ERROR_LIST: unnamed_20 = 27;
pub const MAILIMAP_ERROR_UID_FETCH: unnamed_20 = 26;
pub const MAILIMAP_ERROR_FETCH: unnamed_20 = 25;
pub const MAILIMAP_ERROR_EXAMINE: unnamed_20 = 24;
pub const MAILIMAP_ERROR_DELETE: unnamed_20 = 23;
pub const MAILIMAP_ERROR_CREATE: unnamed_20 = 22;
pub const MAILIMAP_ERROR_UID_MOVE: unnamed_20 = 21;
pub const MAILIMAP_ERROR_MOVE: unnamed_20 = 20;
pub const MAILIMAP_ERROR_UID_COPY: unnamed_20 = 19;
pub const MAILIMAP_ERROR_COPY: unnamed_20 = 18;
pub const MAILIMAP_ERROR_EXPUNGE: unnamed_20 = 17;
pub const MAILIMAP_ERROR_CLOSE: unnamed_20 = 16;
pub const MAILIMAP_ERROR_CHECK: unnamed_20 = 15;
pub const MAILIMAP_ERROR_CAPABILITY: unnamed_20 = 14;
pub const MAILIMAP_ERROR_LOGOUT: unnamed_20 = 13;
pub const MAILIMAP_ERROR_NOOP: unnamed_20 = 12;
pub const MAILIMAP_ERROR_APPEND: unnamed_20 = 11;
pub const MAILIMAP_ERROR_DONT_ACCEPT_CONNECTION: unnamed_20 = 10;
pub const MAILIMAP_ERROR_PROTOCOL: unnamed_20 = 9;
pub const MAILIMAP_ERROR_FATAL: unnamed_20 = 8;
pub const MAILIMAP_ERROR_MEMORY: unnamed_20 = 7;
pub const MAILIMAP_ERROR_CONNECTION_REFUSED: unnamed_20 = 6;
pub const MAILIMAP_ERROR_PARSE: unnamed_20 = 5;
pub const MAILIMAP_ERROR_STREAM: unnamed_20 = 4;
pub const MAILIMAP_ERROR_BAD_STATE: unnamed_20 = 3;
pub const MAILIMAP_NO_ERROR_NON_AUTHENTICATED: unnamed_20 = 2;
pub const MAILIMAP_NO_ERROR_AUTHENTICATED: unnamed_20 = 1;
pub const MAILIMAP_NO_ERROR: unnamed_20 = 0;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_fields {
    pub fld_list: *mut clist,
}

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
    pub fld_data: field_data,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union field_data {
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

pub const MAILMIME_MESSAGE: unnamed_11 = 3;
pub const MAILMIME_MULTIPLE: unnamed_11 = 2;
pub const MAILMIME_SINGLE: unnamed_11 = 1;
pub const MAILMIME_NONE: unnamed_11 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime {
    pub mm_parent_type: libc::c_int,
    pub mm_parent: *mut mailmime,
    pub mm_multipart_pos: *mut clistiter,
    pub mm_type: libc::c_int,
    pub mm_mime_start: *const libc::c_char,
    pub mm_length: size_t,
    pub mm_mime_fields: *mut mailmime_fields,
    pub mm_content_type: *mut mailmime_content,
    pub mm_body: *mut mailmime_data,
    pub mm_data: unnamed_12n,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_12n {
    pub mm_single: *mut mailmime_data,
    pub mm_multipart: unnamed_14n,
    pub mm_message: unnamed_13n,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_13n {
    pub mm_fields: *mut mailimf_fields,
    pub mm_msg_mime: *mut mailmime,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_14n {
    pub mm_preamble: *mut mailmime_data,
    pub mm_epilogue: *mut mailmime_data,
    pub mm_mp_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_fields {
    pub fld_list: *mut clist,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_content {
    pub ct_type: *mut mailmime_type,
    pub ct_subtype: *mut libc::c_char,
    pub ct_parameters: *mut clist,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_data {
    pub dt_type: libc::c_int,
    pub dt_encoding: libc::c_int,
    pub dt_encoded: libc::c_int,
    pub dt_data: unnamed_9n,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_9n {
    pub dt_text: unnamed_10n,
    pub dt_filename: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_10n {
    pub dt_data: *const libc::c_char,
    pub dt_length: size_t,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_type {
    pub tp_type: libc::c_int,
    pub tp_data: unnamed_3n,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_3n {
    pub tp_discrete_type: *mut mailmime_discrete_type,
    pub tp_composite_type: *mut mailmime_composite_type,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_discrete_type {
    pub dt_type: libc::c_int,
    pub dt_extension: *mut libc::c_char,
}

pub const MAILMIME_COMPOSITE_TYPE_EXTENSION: libc::c_int = 3;
pub const MAILMIME_COMPOSITE_TYPE_MULTIPART: libc::c_int = 2;
pub const MAILMIME_COMPOSITE_TYPE_MESSAGE: libc::c_int = 1;
pub const MAILMIME_COMPOSITE_TYPE_ERROR: libc::c_int = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_composite_type {
    pub ct_type: libc::c_int,
    pub ct_token: *mut libc::c_char,
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
pub struct mailimf_mailbox_list {
    pub mb_list: *mut clist,
}

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

pub type __builtin_va_list = [__va_list_tag; 1];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list_tag {
    pub gp_offset: libc::c_uint,
    pub fp_offset: libc::c_uint,
    pub overflow_arg_area: *mut libc::c_void,
    pub reg_save_area: *mut libc::c_void,
}
pub type va_list = __builtin_va_list;
pub type __int64_t = libc::c_longlong;
pub type __darwin_ct_rune_t = libc::c_int;
pub type __darwin_wchar_t = libc::c_int;
pub type __darwin_rune_t = __darwin_wchar_t;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: libc::time_t,
    pub tv_nsec: libc::c_long,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneEntry {
    pub __min: __darwin_rune_t,
    pub __max: __darwin_rune_t,
    pub __map: __darwin_rune_t,
    pub __types: *mut __uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneRange {
    pub __nranges: libc::c_int,
    pub __ranges: *mut _RuneEntry,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneCharClass {
    pub __name: [libc::c_char; 14],
    pub __mask: __uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneLocale {
    pub __magic: [libc::c_char; 8],
    pub __encoding: [libc::c_char; 32],
    pub __sgetrune: Option<
        unsafe extern "C" fn(
            _: *const libc::c_char,
            _: size_t,
            _: *mut *const libc::c_char,
        ) -> __darwin_rune_t,
    >,
    pub __sputrune: Option<
        unsafe extern "C" fn(
            _: __darwin_rune_t,
            _: *mut libc::c_char,
            _: size_t,
            _: *mut *mut libc::c_char,
        ) -> libc::c_int,
    >,
    pub __invalid_rune: __darwin_rune_t,
    pub __runetype: [__uint32_t; 256],
    pub __maplower: [__darwin_rune_t; 256],
    pub __mapupper: [__darwin_rune_t; 256],
    pub __runetype_ext: _RuneRange,
    pub __maplower_ext: _RuneRange,
    pub __mapupper_ext: _RuneRange,
    pub __variable: *mut libc::c_void,
    pub __variable_len: libc::c_int,
    pub __ncharclasses: libc::c_int,
    pub __charclasses: *mut _RuneCharClass,
}
pub type mode_t = libc::mode_t;
pub type off_t = libc::off_t;

pub type uint64_t = libc::c_ulonglong;
pub type uid_t = libc::uid_t;
pub type gid_t = libc::gid_t;
pub type dev_t = libc::dev_t;
pub type blkcnt_t = libc::blkcnt_t;
pub type blksize_t = libc::blksize_t;
pub type nlink_t = __uint16_t;

pub type dc_receive_imf_t = Option<
    unsafe fn(
        _: &dc_context_t,
        _: *const libc::c_char,
        _: size_t,
        _: *const libc::c_char,
        _: uint32_t,
        _: uint32_t,
    ) -> (),
>;

/* Purpose: Reading from IMAP servers with no dependencies to the database.
dc_context_t is only used for logging and to get information about
the online state. */

pub type dc_precheck_imf_t = Option<
    unsafe fn(
        _: &dc_context_t,
        _: *const libc::c_char,
        _: *const libc::c_char,
        _: uint32_t,
    ) -> libc::c_int,
>;
pub type dc_set_config_t =
    Option<unsafe fn(_: &dc_context_t, _: *const libc::c_char, _: *const libc::c_char) -> ()>;
pub type dc_get_config_t = Option<
    unsafe fn(
        _: &dc_context_t,
        _: *const libc::c_char,
        _: *const libc::c_char,
    ) -> *mut libc::c_char,
>;

#[inline]
pub unsafe fn isascii(mut _c: libc::c_int) -> libc::c_int {
    return (_c & !0x7fi32 == 0i32) as libc::c_int;
}

#[inline]
pub unsafe fn isspace(mut _c: libc::c_int) -> libc::c_int {
    if _c < std::u8::MAX as libc::c_int {
        ((_c as u8 as char) == ' ') as libc::c_int
    } else {
        0
    }
}

#[inline]
pub unsafe fn tolower(mut _c: libc::c_int) -> libc::c_int {
    return __tolower(_c);
}

#[inline]
pub unsafe fn carray_count(mut array: *mut carray) -> libc::c_uint {
    return (*array).len;
}

#[inline]
pub unsafe fn carray_get(mut array: *mut carray, mut indx: libc::c_uint) -> *mut libc::c_void {
    return *(*array).array.offset(indx as isize);
}

/* *
 * Callback function that should be given to dc_context_new().
 *
 * @memberof dc_context_t
 * @param context The context object as returned by dc_context_new().
 * @param event one of the @ref DC_EVENT constants
 * @param data1 depends on the event parameter
 * @param data2 depends on the event parameter
 * @return return 0 unless stated otherwise in the event parameter documentation
 */
pub type dc_callback_t =
    unsafe extern "C" fn(_: &dc_context_t, _: Event, _: uintptr_t, _: uintptr_t) -> uintptr_t;

pub const DC_MOVE_STATE_MOVING: libc::c_uint = 3;
pub const DC_MOVE_STATE_STAY: libc::c_uint = 2;
pub const DC_MOVE_STATE_PENDING: libc::c_uint = 1;
pub const DC_MOVE_STATE_UNDEFINED: libc::c_uint = 0;

pub const MAILIMF_ERROR_FILE: libc::c_uint = 4;
pub const MAILIMF_ERROR_INVAL: libc::c_uint = 3;
pub const MAILIMF_ERROR_MEMORY: libc::c_uint = 2;
pub const MAILIMF_ERROR_PARSE: libc::c_uint = 1;
pub const MAILIMF_NO_ERROR: libc::c_uint = 0;

pub const MAIL_CHARCONV_ERROR_CONV: libc::c_uint = 3;
pub const MAIL_CHARCONV_ERROR_MEMORY: libc::c_uint = 2;
pub const MAIL_CHARCONV_ERROR_UNKNOWN_CHARSET: libc::c_uint = 1;
pub const MAIL_CHARCONV_NO_ERROR: libc::c_uint = 0;

pub const MAILSMTP_ERROR_CLIENTID_NOT_SUPPORTED: libc::c_uint = 28;
pub const MAILSMTP_ERROR_SSL: libc::c_uint = 27;
pub const MAILSMTP_ERROR_AUTH_AUTHENTICATION_FAILED: libc::c_uint = 26;
pub const MAILSMTP_ERROR_CONNECTION_REFUSED: libc::c_uint = 25;
pub const MAILSMTP_ERROR_STARTTLS_NOT_SUPPORTED: libc::c_uint = 24;
pub const MAILSMTP_ERROR_STARTTLS_TEMPORARY_FAILURE: libc::c_uint = 23;
pub const MAILSMTP_ERROR_AUTH_ENCRYPTION_REQUIRED: libc::c_uint = 22;
pub const MAILSMTP_ERROR_AUTH_TEMPORARY_FAILTURE: libc::c_uint = 21;
pub const MAILSMTP_ERROR_AUTH_TRANSITION_NEEDED: libc::c_uint = 20;
pub const MAILSMTP_ERROR_AUTH_TOO_WEAK: libc::c_uint = 19;
pub const MAILSMTP_ERROR_AUTH_REQUIRED: libc::c_uint = 18;
pub const MAILSMTP_ERROR_AUTH_LOGIN: libc::c_uint = 17;
pub const MAILSMTP_ERROR_AUTH_NOT_SUPPORTED: libc::c_uint = 16;
pub const MAILSMTP_ERROR_MEMORY: libc::c_uint = 15;
pub const MAILSMTP_ERROR_TRANSACTION_FAILED: libc::c_uint = 14;
pub const MAILSMTP_ERROR_USER_NOT_LOCAL: libc::c_uint = 13;
pub const MAILSMTP_ERROR_BAD_SEQUENCE_OF_COMMAND: libc::c_uint = 12;
pub const MAILSMTP_ERROR_MAILBOX_NAME_NOT_ALLOWED: libc::c_uint = 11;
pub const MAILSMTP_ERROR_MAILBOX_UNAVAILABLE: libc::c_uint = 10;
pub const MAILSMTP_ERROR_INSUFFICIENT_SYSTEM_STORAGE: libc::c_uint = 9;
pub const MAILSMTP_ERROR_IN_PROCESSING: libc::c_uint = 8;
pub const MAILSMTP_ERROR_EXCEED_STORAGE_ALLOCATION: libc::c_uint = 7;
pub const MAILSMTP_ERROR_ACTION_NOT_TAKEN: libc::c_uint = 6;
pub const MAILSMTP_ERROR_NOT_IMPLEMENTED: libc::c_uint = 5;
pub const MAILSMTP_ERROR_HOSTNAME: libc::c_uint = 4;
pub const MAILSMTP_ERROR_STREAM: libc::c_uint = 3;
pub const MAILSMTP_ERROR_SERVICE_NOT_AVAILABLE: libc::c_uint = 2;
pub const MAILSMTP_ERROR_UNEXPECTED_CODE: libc::c_uint = 1;
pub const MAILSMTP_NO_ERROR: libc::c_uint = 0;
pub const MAILSMTP_AUTH_KERBEROS_V4: libc::c_uint = 256;
pub const MAILSMTP_AUTH_NTLM: libc::c_uint = 128;
pub const MAILSMTP_AUTH_SRP: libc::c_uint = 64;
pub const MAILSMTP_AUTH_GSSAPI: libc::c_uint = 32;
pub const MAILSMTP_AUTH_DIGEST_MD5: libc::c_uint = 16;
pub const MAILSMTP_AUTH_LOGIN: libc::c_uint = 8;
pub const MAILSMTP_AUTH_PLAIN: libc::c_uint = 4;
pub const MAILSMTP_AUTH_CRAM_MD5: libc::c_uint = 2;
pub const MAILSMTP_AUTH_CHECKED: libc::c_uint = 1;
pub const MAILSMTP_AUTH_NOT_CHECKED: libc::c_uint = 0;

pub type dc_imap_res = libc::c_uint;
pub const DC_SUCCESS: dc_imap_res = 3;
pub const DC_ALREADY_DONE: dc_imap_res = 2;
pub const DC_RETRY_LATER: dc_imap_res = 1;
pub const DC_FAILED: dc_imap_res = 0;

pub type dc_move_state_t = libc::c_uint;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition_parm {
    pub pa_type: libc::c_int,
    pub pa_data: unnamed_20n,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_20n {
    pub pa_filename: *mut libc::c_char,
    pub pa_creation_date: *mut libc::c_char,
    pub pa_modification_date: *mut libc::c_char,
    pub pa_read_date: *mut libc::c_char,
    pub pa_size: size_t,
    pub pa_parameter: *mut mailmime_parameter,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_5n,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_5n {
    pub fld_content: *mut mailmime_content,
    pub fld_encoding: *mut mailmime_mechanism,
    pub fld_id: *mut libc::c_char,
    pub fld_description: *mut libc::c_char,
    pub fld_version: uint32_t,
    pub fld_disposition: *mut mailmime_disposition,
    pub fld_language: *mut mailmime_language,
    pub fld_location: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_language {
    pub lg_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition {
    pub dsp_type: *mut mailmime_disposition_type,
    pub dsp_parms: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition_type {
    pub dsp_type: libc::c_int,
    pub dsp_extension: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_mechanism {
    pub enc_type: libc::c_int,
    pub enc_token: *mut libc::c_char,
}

pub const MAILIMF_ADDRESS_GROUP: libc::c_uint = 2;
pub const MAILIMF_ADDRESS_MAILBOX: libc::c_uint = 1;
pub const MAILIMF_ADDRESS_ERROR: libc::c_uint = 0;
pub const MAILMIME_MECHANISM_TOKEN: libc::c_uint = 6;
pub const MAILMIME_MECHANISM_BASE64: libc::c_uint = 5;
pub const MAILMIME_MECHANISM_QUOTED_PRINTABLE: libc::c_uint = 4;
pub const MAILMIME_MECHANISM_BINARY: libc::c_uint = 3;
pub const MAILMIME_MECHANISM_8BIT: libc::c_uint = 2;
pub const MAILMIME_MECHANISM_7BIT: libc::c_uint = 1;
pub const MAILMIME_MECHANISM_ERROR: libc::c_uint = 0;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_group {
    pub grp_display_name: *mut libc::c_char,
    pub grp_mb_list: *mut mailimf_mailbox_list,
}

pub const MAILMIME_DISPOSITION_TYPE_EXTENSION: libc::c_uint = 3;
pub const MAILMIME_DISPOSITION_TYPE_ATTACHMENT: libc::c_uint = 2;
pub const MAILMIME_DISPOSITION_TYPE_INLINE: libc::c_uint = 1;
pub const MAILMIME_DISPOSITION_TYPE_ERROR: libc::c_uint = 0;
pub const MAILMIME_DISPOSITION_PARM_PARAMETER: libc::c_uint = 5;
pub const MAILMIME_DISPOSITION_PARM_SIZE: libc::c_uint = 4;
pub const MAILMIME_DISPOSITION_PARM_READ_DATE: libc::c_uint = 3;
pub const MAILMIME_DISPOSITION_PARM_MODIFICATION_DATE: libc::c_uint = 2;
pub const MAILMIME_DISPOSITION_PARM_CREATION_DATE: libc::c_uint = 1;
pub const MAILMIME_DISPOSITION_PARM_FILENAME: libc::c_uint = 0;

pub const MAILMIME_FIELD_LOCATION: libc::c_uint = 8;
pub const MAILMIME_FIELD_LANGUAGE: libc::c_uint = 7;
pub const MAILMIME_FIELD_DISPOSITION: libc::c_uint = 6;
pub const MAILMIME_FIELD_VERSION: libc::c_uint = 5;
pub const MAILMIME_FIELD_DESCRIPTION: libc::c_uint = 4;
pub const MAILMIME_FIELD_ID: libc::c_uint = 3;
pub const MAILMIME_FIELD_TRANSFER_ENCODING: libc::c_uint = 2;
pub const MAILMIME_FIELD_TYPE: libc::c_uint = 1;
pub const MAILMIME_FIELD_NONE: libc::c_uint = 0;

pub const MAILMIME_TYPE_COMPOSITE_TYPE: libc::c_uint = 2;
pub const MAILMIME_TYPE_DISCRETE_TYPE: libc::c_uint = 1;
pub const MAILMIME_TYPE_ERROR: libc::c_uint = 0;
pub const MAILMIME_DATA_FILE: libc::c_uint = 1;
pub const MAILMIME_DATA_TEXT: libc::c_uint = 0;

pub const MAIL_ERROR_SSL: libc::c_uint = 58;
pub const MAIL_ERROR_FOLDER: libc::c_uint = 57;
pub const MAIL_ERROR_UNABLE: libc::c_uint = 56;
pub const MAIL_ERROR_SYSTEM: libc::c_uint = 55;
pub const MAIL_ERROR_COMMAND: libc::c_uint = 54;
pub const MAIL_ERROR_SEND: libc::c_uint = 53;
pub const MAIL_ERROR_CHAR_ENCODING_FAILED: libc::c_uint = 52;
pub const MAIL_ERROR_SUBJECT_NOT_FOUND: libc::c_uint = 51;
pub const MAIL_ERROR_PROGRAM_ERROR: libc::c_uint = 50;
pub const MAIL_ERROR_NO_PERMISSION: libc::c_uint = 49;
pub const MAIL_ERROR_COMMAND_NOT_SUPPORTED: libc::c_uint = 48;
pub const MAIL_ERROR_NO_APOP: libc::c_uint = 47;
pub const MAIL_ERROR_READONLY: libc::c_uint = 46;
pub const MAIL_ERROR_FATAL: libc::c_uint = 45;
pub const MAIL_ERROR_CLOSE: libc::c_uint = 44;
pub const MAIL_ERROR_CAPABILITY: libc::c_uint = 43;
pub const MAIL_ERROR_PROTOCOL: libc::c_uint = 42;
pub const MAIL_ERROR_MISC: libc::c_uint = 41;
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

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_address {
    pub ad_type: libc::c_int,
    pub ad_data: unnamed_0n,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_0n {
    pub ad_mailbox: *mut mailimf_mailbox,
    pub ad_group: *mut mailimf_group,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_parameter {
    pub pa_name: *mut libc::c_char,
    pub pa_value: *mut libc::c_char,
}

pub const MAILMIME_DISCRETE_TYPE_EXTENSION: libc::c_uint = 6;
pub const MAILMIME_DISCRETE_TYPE_APPLICATION: libc::c_uint = 5;
pub const MAILMIME_DISCRETE_TYPE_VIDEO: libc::c_uint = 4;
pub const MAILMIME_DISCRETE_TYPE_AUDIO: libc::c_uint = 3;
pub const MAILMIME_DISCRETE_TYPE_IMAGE: libc::c_uint = 2;
pub const MAILMIME_DISCRETE_TYPE_TEXT: libc::c_uint = 1;
pub const MAILMIME_DISCRETE_TYPE_ERROR: libc::c_uint = 0;
pub const MAILIMAP_MBX_LIST_OFLAG_FLAG_EXT: libc::c_uint = 2;
pub const MAILIMAP_MBX_LIST_OFLAG_NOINFERIORS: libc::c_uint = 1;
pub const MAILIMAP_MBX_LIST_OFLAG_ERROR: libc::c_uint = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_mbx_list_oflag {
    pub of_type: libc::c_int,
    pub of_flag_ext: *mut libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_mailbox_list {
    pub mb_flag: *mut mailimap_mbx_list_flags,
    pub mb_delimiter: libc::c_char,
    pub mb_name: *mut libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_mbx_list_flags {
    pub mbf_type: libc::c_int,
    pub mbf_oflags: *mut clist,
    pub mbf_sflag: libc::c_int,
}

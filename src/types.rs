use libc;

use crate::constants::Event;
use crate::dc_context::dc_context_t;

pub use libc::{dirent, tm, DIR, FILE};
pub use libsqlite3_sys::*;

extern "C" {
    pub type __sFILEX;

    pub type _telldir;
    pub type mailstream_cancel;
}

/**
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

pub type dc_move_state_t = u32;

pub type dc_receive_imf_t = unsafe fn(
    _: &dc_context_t,
    _: *const libc::c_char,
    _: size_t,
    _: *const libc::c_char,
    _: uint32_t,
    _: uint32_t,
) -> ();

/* Purpose: Reading from IMAP servers with no dependencies to the database.
dc_context_t is only used for logging and to get information about
the online state. */

pub type dc_precheck_imf_t = unsafe fn(
    _: &dc_context_t,
    _: *const libc::c_char,
    _: *const libc::c_char,
    _: u32,
) -> libc::c_int;
pub type dc_set_config_t =
    unsafe fn(_: &dc_context_t, _: *const libc::c_char, _: *const libc::c_char) -> ();
pub type dc_get_config_t = unsafe fn(
    _: &dc_context_t,
    _: *const libc::c_char,
    _: *const libc::c_char,
) -> *mut libc::c_char;

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
pub struct mailimap_date_time {
    pub dt_day: libc::c_int,
    pub dt_month: libc::c_int,
    pub dt_year: libc::c_int,
    pub dt_hour: libc::c_int,
    pub dt_min: libc::c_int,
    pub dt_sec: libc::c_int,
    pub dt_zone: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_fields {
    pub fld_list: *mut clist,
}

pub const MAILMIME_DISCRETE_TYPE_EXTENSION: libc::c_uint = 6;
pub const MAILMIME_DISCRETE_TYPE_APPLICATION: libc::c_uint = 5;
pub const MAILMIME_DISCRETE_TYPE_VIDEO: libc::c_uint = 4;
pub const MAILMIME_DISCRETE_TYPE_AUDIO: libc::c_uint = 3;
pub const MAILMIME_DISCRETE_TYPE_IMAGE: libc::c_uint = 2;
pub const MAILMIME_DISCRETE_TYPE_TEXT: libc::c_uint = 1;
pub const MAILMIME_DISCRETE_TYPE_ERROR: libc::c_uint = 0;

pub const MAILIMF_FIELD_OPTIONAL_FIELD: libc::c_uint = 22;
pub const MAILIMF_FIELD_KEYWORDS: libc::c_uint = 21;
pub const MAILIMF_FIELD_COMMENTS: libc::c_uint = 20;
pub const MAILIMF_FIELD_SUBJECT: libc::c_uint = 19;
pub const MAILIMF_FIELD_REFERENCES: libc::c_uint = 18;
pub const MAILIMF_FIELD_IN_REPLY_TO: libc::c_uint = 17;
pub const MAILIMF_FIELD_MESSAGE_ID: libc::c_uint = 16;
pub const MAILIMF_FIELD_BCC: libc::c_uint = 15;
pub const MAILIMF_FIELD_CC: libc::c_uint = 14;
pub const MAILIMF_FIELD_TO: libc::c_uint = 13;
pub const MAILIMF_FIELD_REPLY_TO: libc::c_uint = 12;
pub const MAILIMF_FIELD_SENDER: libc::c_uint = 11;
pub const MAILIMF_FIELD_FROM: libc::c_uint = 10;
pub const MAILIMF_FIELD_ORIG_DATE: libc::c_uint = 9;
pub const MAILIMF_FIELD_RESENT_MSG_ID: libc::c_uint = 8;
pub const MAILIMF_FIELD_RESENT_BCC: libc::c_uint = 7;
pub const MAILIMF_FIELD_RESENT_CC: libc::c_uint = 6;
pub const MAILIMF_FIELD_RESENT_TO: libc::c_uint = 5;
pub const MAILIMF_FIELD_RESENT_SENDER: libc::c_uint = 4;
pub const MAILIMF_FIELD_RESENT_FROM: libc::c_uint = 3;
pub const MAILIMF_FIELD_RESENT_DATE: libc::c_uint = 2;
pub const MAILIMF_FIELD_RETURN_PATH: libc::c_uint = 1;
pub const MAILIMF_FIELD_NONE: libc::c_uint = 0;

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

pub const MAILMIME_MESSAGE: libc::c_uint = 3;
pub const MAILMIME_MULTIPLE: libc::c_uint = 2;
pub const MAILMIME_SINGLE: libc::c_uint = 1;
pub const MAILMIME_NONE: libc::c_uint = 0;

pub const MAILMIME_COMPOSITE_TYPE_EXTENSION: libc::c_int = 3;
pub const MAILMIME_COMPOSITE_TYPE_MULTIPART: libc::c_int = 2;
pub const MAILMIME_COMPOSITE_TYPE_MESSAGE: libc::c_int = 1;
pub const MAILMIME_COMPOSITE_TYPE_ERROR: libc::c_int = 0;

pub const MAILIMF_ERROR_FILE: libc::c_uint = 4;
pub const MAILIMF_ERROR_INVAL: libc::c_uint = 3;
pub const MAILIMF_ERROR_MEMORY: libc::c_uint = 2;
pub const MAILIMF_ERROR_PARSE: libc::c_uint = 1;
pub const MAILIMF_NO_ERROR: libc::c_uint = 0;

pub const MAIL_CHARCONV_ERROR_CONV: libc::c_uint = 3;
pub const MAIL_CHARCONV_ERROR_MEMORY: libc::c_uint = 2;
pub const MAIL_CHARCONV_ERROR_UNKNOWN_CHARSET: libc::c_uint = 1;
pub const MAIL_CHARCONV_NO_ERROR: libc::c_uint = 0;

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

#[inline]
pub unsafe fn carray_count(mut array: *mut carray) -> libc::c_uint {
    return (*array).len;
}

#[inline]
pub unsafe fn carray_get(mut array: *mut carray, mut indx: libc::c_uint) -> *mut libc::c_void {
    return *(*array).array.offset(indx as isize);
}

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

#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_group {
    pub grp_display_name: *mut libc::c_char,
    pub grp_mb_list: *mut mailimf_mailbox_list,
}

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

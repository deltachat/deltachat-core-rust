use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_context::dc_context_t;
use crate::dc_imap::dc_imap_t;
use crate::dc_jobthread::dc_jobthread_t;
use crate::dc_smtp::dc_smtp_t;
use crate::types::*;

extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn atoi(_: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn memcpy(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strstr(_: *const libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn clist_new() -> *mut clist;
    #[no_mangle]
    fn clist_free(_: *mut clist);
    #[no_mangle]
    fn clist_insert_after(_: *mut clist, _: *mut clistiter, _: *mut libc::c_void) -> libc::c_int;
    #[no_mangle]
    fn mailimap_xlist(
        session: *mut mailimap,
        mb: *const libc::c_char,
        list_mb: *const libc::c_char,
        result: *mut *mut clist,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailimap_create(session: *mut mailimap, mb: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn mailimap_list(
        session: *mut mailimap,
        mb: *const libc::c_char,
        list_mb: *const libc::c_char,
        result: *mut *mut clist,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailimap_list_result_free(list: *mut clist);
    #[no_mangle]
    fn mailimap_subscribe(session: *mut mailimap, mb: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_job_add(
        _: *mut dc_context_t,
        action: libc::c_int,
        foreign_id: libc::c_int,
        param: *const libc::c_char,
        delay: libc::c_int,
    );
    #[no_mangle]
    fn dc_job_kill_action(_: *mut dc_context_t, action: libc::c_int);
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_sqlite3_get_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: int32_t,
    ) -> int32_t;
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_sqlite3_is_open(_: *const dc_sqlite3_t) -> libc::c_int;
    /* handle configurations, private */
    #[no_mangle]
    fn dc_sqlite3_set_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        value: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_set_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        value: int32_t,
    ) -> libc::c_int;
    /* Some tools and enhancements to the used libraries, there should be
    no references to dc_context_t and other "larger" classes here. */
    // for carray etc.
    /* ** library-private **********************************************************/
    /* math tools */
    #[no_mangle]
    fn dc_exactly_one_bit_set(v: libc::c_int) -> libc::c_int;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_atoi_null_is_0(_: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_strlower_in_place(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_str_replace(
        haystack: *mut *mut libc::c_char,
        needle: *const libc::c_char,
        replacement: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_urlencode(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_decode_modified_utf7(
        _: *const libc::c_char,
        change_spaces: libc::c_int,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_loginparam_new() -> *mut dc_loginparam_t;
    #[no_mangle]
    fn dc_loginparam_unref(_: *mut dc_loginparam_t);
    #[no_mangle]
    fn dc_loginparam_read(
        _: *mut dc_loginparam_t,
        _: *mut dc_sqlite3_t,
        prefix: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_loginparam_write(
        _: *const dc_loginparam_t,
        _: *mut dc_sqlite3_t,
        prefix: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_loginparam_get_readable(_: *const dc_loginparam_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_imap_connect(_: *mut dc_imap_t, _: *const dc_loginparam_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_disconnect(_: *mut dc_imap_t);
    #[no_mangle]
    fn dc_imap_is_connected(_: *const dc_imap_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_is_error(imap: *mut dc_imap_t, code: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_smtp_connect(_: *mut dc_smtp_t, _: *const dc_loginparam_t) -> libc::c_int;
    #[no_mangle]
    fn dc_smtp_disconnect(_: *mut dc_smtp_t);
    #[no_mangle]
    fn dc_ensure_secret_key_exists(_: *mut dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_saxparser_parse(_: *mut dc_saxparser_t, text: *const libc::c_char);
    #[no_mangle]
    fn dc_saxparser_set_text_handler(_: *mut dc_saxparser_t, _: dc_saxparser_text_cb_t);
    #[no_mangle]
    fn dc_attr_find(attr: *mut *mut libc::c_char, key: *const libc::c_char) -> *const libc::c_char;
    #[no_mangle]
    fn dc_saxparser_set_tag_handler(
        _: *mut dc_saxparser_t,
        _: dc_saxparser_starttag_cb_t,
        _: dc_saxparser_endtag_cb_t,
    );
    #[no_mangle]
    fn dc_saxparser_init(_: *mut dc_saxparser_t, userData: *mut libc::c_void);
    #[no_mangle]
    fn dc_get_oauth2_addr(
        _: *mut dc_context_t,
        addr: *const libc::c_char,
        code: *const libc::c_char,
    ) -> *mut libc::c_char;
}
pub type __darwin_size_t = libc::c_ulong;
pub type __darwin_ssize_t = libc::c_long;
pub type __darwin_time_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_cond_t {
    pub __sig: libc::c_long,
    pub __opaque: [libc::c_char; 40],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_mutex_t {
    pub __sig: libc::c_long,
    pub __opaque: [libc::c_char; 56],
}
pub type __darwin_pthread_cond_t = _opaque_pthread_cond_t;
pub type __darwin_pthread_mutex_t = _opaque_pthread_mutex_t;
pub type size_t = __darwin_size_t;
pub type uint32_t = libc::c_uint;
pub type int32_t = libc::c_int;
pub type uintptr_t = libc::c_ulong;
pub type ssize_t = __darwin_ssize_t;
pub type time_t = __darwin_time_t;
pub type uint8_t = libc::c_uchar;
pub type uint16_t = libc::c_ushort;
pub type pthread_cond_t = __darwin_pthread_cond_t;
pub type pthread_mutex_t = __darwin_pthread_mutex_t;
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
    pub smtp_sasl: unnamed,
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
pub struct unnamed {
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
pub struct mailimap_capability_data {
    pub cap_list: *mut clist,
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
    pub sec_data: unnamed_0,
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
pub union unnamed_0 {
    pub sec_msgtext: *mut mailimap_section_msgtext,
    pub sec_part: *mut mailimap_section_part,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_section_part {
    pub sec_id: *mut clist,
}
pub type mailimap_msg_body_handler = unsafe extern "C" fn(
    _: libc::c_int,
    _: *mut mailimap_msg_att_body_section,
    _: *const libc::c_char,
    _: size_t,
    _: *mut libc::c_void,
) -> bool;
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
pub type unnamed_1 = libc::c_uint;
pub const MAILIMAP_MBX_LIST_OFLAG_FLAG_EXT: unnamed_1 = 2;
pub const MAILIMAP_MBX_LIST_OFLAG_NOINFERIORS: unnamed_1 = 1;
pub const MAILIMAP_MBX_LIST_OFLAG_ERROR: unnamed_1 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_mbx_list_oflag {
    pub of_type: libc::c_int,
    pub of_flag_ext: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimap_msg_att {
    pub att_list: *mut clist,
    pub att_number: uint32_t,
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
    pub ft_data: unnamed_2,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_2 {
    pub ft_fetch_att: *mut mailimap_fetch_att,
    pub ft_fetch_att_list: *mut clist,
}
pub type mailimap_msg_att_handler =
    unsafe extern "C" fn(_: *mut mailimap_msg_att, _: *mut libc::c_void) -> ();
// #[derive(Copy, Clone)]
// #[repr(C)]
// pub struct mailimap {
//     pub imap_response: *mut libc::c_char,
//     pub imap_stream: *mut mailstream,
//     pub imap_progr_rate: size_t,
//     pub imap_progr_fun: Option<unsafe extern "C" fn(_: size_t, _: size_t) -> ()>,
//     pub imap_stream_buffer: *mut MMAPString,
//     pub imap_response_buffer: *mut MMAPString,
//     pub imap_state: libc::c_int,
//     pub imap_tag: libc::c_int,
//     pub imap_connection_info: *mut mailimap_connection_info,
//     pub imap_selection_info: *mut mailimap_selection_info,
//     pub imap_response_info: *mut mailimap_response_info,
//     pub imap_sasl: unnamed_3,
//     pub imap_idle_timestamp: time_t,
//     pub imap_idle_maxdelay: time_t,
//     pub imap_body_progress_fun:
//         Option<unsafe extern "C" fn(_: size_t, _: size_t, _: *mut libc::c_void) -> ()>,
//     pub imap_items_progress_fun:
//         Option<unsafe extern "C" fn(_: size_t, _: size_t, _: *mut libc::c_void) -> ()>,
//     pub imap_progress_context: *mut libc::c_void,
//     pub imap_msg_att_handler:
//         Option<unsafe extern "C" fn(_: *mut mailimap_msg_att, _: *mut libc::c_void) -> ()>,
//     pub imap_msg_att_handler_context: *mut libc::c_void,
//     pub imap_msg_body_handler: Option<
//         unsafe extern "C" fn(
//             _: libc::c_int,
//             _: *mut mailimap_msg_att_body_section,
//             _: *const libc::c_char,
//             _: size_t,
//             _: *mut libc::c_void,
//         ) -> bool,
//     >,
//     pub imap_msg_body_handler_context: *mut libc::c_void,
//     pub imap_timeout: time_t,
//     pub imap_logger: Option<
//         unsafe extern "C" fn(
//             _: *mut mailimap,
//             _: libc::c_int,
//             _: *const libc::c_char,
//             _: size_t,
//             _: *mut libc::c_void,
//         ) -> (),
//     >,
//     pub imap_logger_context: *mut libc::c_void,
//     pub is_163_workaround_enabled: libc::c_int,
//     pub is_rambler_workaround_enabled: libc::c_int,
//     pub is_qip_workaround_enabled: libc::c_int,
// }
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_3 {
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
/* define DC_USE_RPGP to enable use of rPGP instead of netpgp where available;
preferrably, this should be done in the project configuration currently */
//#define DC_USE_RPGP 1
/* Includes that are used frequently.  This file may also be used to create predefined headers. */
/* * Structure behind dc_context_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_context {
    pub magic: uint32_t,
    pub userdata: *mut libc::c_void,
    pub dbfile: *mut libc::c_char,
    pub blobdir: *mut libc::c_char,
    pub sql: *mut dc_sqlite3_t,
    pub inbox: *mut dc_imap_t,
    pub inboxidle_condmutex: pthread_mutex_t,
    pub perform_inbox_jobs_needed: libc::c_int,
    pub probe_imap_network: libc::c_int,
    pub sentbox_thread: dc_jobthread_t,
    pub mvbox_thread: dc_jobthread_t,
    pub smtp: *mut dc_smtp_t,
    pub smtpidle_cond: pthread_cond_t,
    pub smtpidle_condmutex: pthread_mutex_t,
    pub smtpidle_condflag: libc::c_int,
    pub smtp_suspended: libc::c_int,
    pub smtp_doing_jobs: libc::c_int,
    pub perform_smtp_jobs_needed: libc::c_int,
    pub probe_smtp_network: libc::c_int,
    pub oauth2_critical: pthread_mutex_t,
    pub cb: dc_callback_t,
    pub os_name: *mut libc::c_char,
    pub cmdline_sel_chat_id: uint32_t,
    pub bob_expects: libc::c_int,
    pub bobs_status: libc::c_int,
    pub bobs_qr_scan: *mut dc_lot_t,
    pub bobs_qr_critical: pthread_mutex_t,
    pub last_smeared_timestamp: time_t,
    pub smear_critical: pthread_mutex_t,
    pub ongoing_running: libc::c_int,
    pub shall_stop_ongoing: libc::c_int,
}
use crate::dc_lot::dc_lot_t;
/* * Structure behind dc_lot_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_lot {
    pub magic: uint32_t,
    pub text1_meaning: libc::c_int,
    pub text1: *mut libc::c_char,
    pub text2: *mut libc::c_char,
    pub timestamp: time_t,
    pub state: libc::c_int,
    pub id: uint32_t,
    pub fingerprint: *mut libc::c_char,
    pub invitenumber: *mut libc::c_char,
    pub auth: *mut libc::c_char,
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
pub type dc_callback_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_context_t,
        _: libc::c_int,
        _: uintptr_t,
        _: uintptr_t,
    ) -> uintptr_t,
>;
/* *
 * @mainpage Getting started
 *
 * This document describes how to handle the Delta Chat core library.
 * For general information about Delta Chat itself,
 * see <https://delta.chat> and <https://github.com/deltachat>.
 *
 * Let's start.
 *
 * First of all, you have to **define an event-handler-function**
 * that is called by the library on specific events
 * (eg. when the configuration is done or when fresh messages arrive).
 * With this function you can create a Delta Chat context then:
 *
 * ~~~
 * #include <deltachat.h>
 *
 * uintptr_t event_handler_func(dc_context_t* context, int event,
 *                              uintptr_t data1, uintptr_t data2)
 * {
 *     return 0; // for unhandled events, it is always safe to return 0
 * }
 *
 * dc_context_t* context = dc_context_new(event_handler_func, NULL, NULL);
 * ~~~
 *
 * After that, you should make sure,
 * sending and receiving jobs are processed as needed.
 * For this purpose, you have to **create two threads:**
 *
 * ~~~
 * #include <pthread.h>
 *
 * void* imap_thread_func(void* context)
 * {
 *     while (true) {
 *         dc_perform_imap_jobs(context);
 *         dc_perform_imap_fetch(context);
 *         dc_perform_imap_idle(context);
 *     }
 * }
 *
 * void* smtp_thread_func(void* context)
 * {
 *     while (true) {
 *         dc_perform_smtp_jobs(context);
 *         dc_perform_smtp_idle(context);
 *     }
 * }
 *
 * static pthread_t imap_thread, smtp_thread;
 * pthread_create(&imap_thread, NULL, imap_thread_func, context);
 * pthread_create(&smtp_thread, NULL, smtp_thread_func, context);
 * ~~~
 *
 * The example above uses "pthreads",
 * however, you can also use anything else for thread handling.
 * NB: The deltachat-core library itself does not create any threads on its own,
 * however, functions, unless stated otherwise, are thread-safe.
 *
 * After that you can  **define and open a database.**
 * The database is a normal sqlite-file and is created as needed:
 *
 * ~~~
 * dc_open(context, "example.db", NULL);
 * ~~~
 *
 * Now you can **configure the context:**
 *
 * ~~~
 * // use some real test credentials here
 * dc_set_config(context, "addr", "alice@example.org");
 * dc_set_config(context, "mail_pw", "***");
 * dc_configure(context);
 * ~~~
 *
 * dc_configure() returns immediately, the configuration itself may take a while
 * and is done by a job in the imap-thread you've defined above.
 * Once done, the #DC_EVENT_CONFIGURE_PROGRESS reports success
 * to the event_handler_func() that is also defined above.
 *
 * The configuration result is saved in the database,
 * on subsequent starts it is not needed to call dc_configure()
 * (you can check this using dc_is_configured()).
 *
 * Now you can **send the first message:**
 *
 * ~~~
 * // use a real testing address here
 * uint32_t contact_id = dc_create_contact(context, NULL, "bob@example.org");
 * uint32_t chat_id    = dc_create_chat_by_contact_id(context, contact_id);
 *
 * dc_send_text_msg(context, chat_id, "Hi, here is my first message!");
 * ~~~
 *
 * dc_send_text_msg() returns immediately;
 * the sending itself is done by a job in the smtp-thread you've defined above.
 * If you check the testing address (bob)
 * and you should have received a normal email.
 * Answer this email in any email program with "Got it!"
 * and the imap-thread you've create above will **receive the message**.
 *
 * You can then **list all messages** of a chat as follow:
 *
 * ~~~
 * dc_array_t* msglist = dc_get_chat_msgs(context, chat_id, 0, 0);
 * for (int i = 0; i < dc_array_get_cnt(msglist); i++)
 * {
 *     uint32_t  msg_id = dc_array_get_id(msglist, i);
 *     dc_msg_t* msg    = dc_get_msg(context, msg_id);
 *     char*     text   = dc_msg_get_text(msg);
 *
 *     printf("Message %i: %s\n", i+1, text);
 *
 *     free(text);
 *     dc_msg_unref(msg);
 * }
 * dc_array_unref(msglist);
 * ~~~
 *
 * This will output the following two lines:
 *
 * ~~~
 * Message 1: Hi, here is my first message!
 * Message 2: Got it!
 * ~~~
 *
 *
 * ## Class reference
 *
 * For a class reference, see the "Classes" link atop.
 *
 *
 * ## Further hints
 *
 * Here are some additional, unsorted hints that may be useful.
 *
 * - For `get`-functions, you have to unref the return value in some way.
 *
 * - Strings in function arguments or return values are usually UTF-8 encoded.
 *
 * - The issue-tracker for the core library is here:
 *   <https://github.com/deltachat/deltachat-core/issues>
 *
 * The following points are important mainly
 * for the authors of the library itself:
 *
 * - For indentation, use tabs.
 *   Alignments that are not placed at the beginning of a line
 *   should be done with spaces.
 *
 * - For padding between functions,
 *   classes etc. use 2 empty lines
 *
 * - Source files are encoded as UTF-8 with Unix line endings
 *   (a simple `LF`, `0x0A` or `\n`)
 *
 * If you need further assistance,
 * please do not hesitate to contact us
 * through the channels shown at https://delta.chat/en/contribute
 *
 * Please keep in mind, that your derived work
 * must respect the Mozilla Public License 2.0 of libdeltachat
 * and the respective licenses of the libraries libdeltachat links with.
 *
 * See you.
 */
/* *
 * @class dc_context_t
 *
 * An object representing a single account.
 *
 * Each account is linked to an IMAP/SMTP account and uses a separate
 * SQLite database for offline functionality and for account-related
 * settings.
 */

/* ** library-private **********************************************************/
// pub type dc_smtp_t = _dc_smtp;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_smtp {
    pub etpan: *mut mailsmtp,
    pub from: *mut libc::c_char,
    pub esmtp: libc::c_int,
    pub log_connect_errors: libc::c_int,
    pub context: *mut dc_context_t,
    pub error: *mut libc::c_char,
    pub error_etpan: libc::c_int,
}
// pub type dc_jobthread_t = _dc_jobthread;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_jobthread {
    pub context: *mut dc_context_t,
    pub name: *mut libc::c_char,
    pub folder_config_name: *mut libc::c_char,
    pub imap: *mut _dc_imap,
    pub mutex: pthread_mutex_t,
    pub idle_cond: pthread_cond_t,
    pub idle_condflag: libc::c_int,
    pub jobs_needed: libc::c_int,
    pub suspended: libc::c_int,
    pub using_handle: libc::c_int,
}
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_imap {
    pub addr: *mut libc::c_char,
    pub imap_server: *mut libc::c_char,
    pub imap_port: libc::c_int,
    pub imap_user: *mut libc::c_char,
    pub imap_pw: *mut libc::c_char,
    pub server_flags: libc::c_int,
    pub connected: libc::c_int,
    pub etpan: *mut mailimap,
    pub idle_set_up: libc::c_int,
    pub selected_folder: *mut libc::c_char,
    pub selected_folder_needs_expunge: libc::c_int,
    pub should_reconnect: libc::c_int,
    pub can_idle: libc::c_int,
    pub has_xlist: libc::c_int,
    pub imap_delimiter: libc::c_char,
    pub watch_folder: *mut libc::c_char,
    pub watch_cond: pthread_cond_t,
    pub watch_condmutex: pthread_mutex_t,
    pub watch_condflag: libc::c_int,
    pub fetch_type_prefetch: *mut mailimap_fetch_type,
    pub fetch_type_body: *mut mailimap_fetch_type,
    pub fetch_type_flags: *mut mailimap_fetch_type,
    pub get_config: dc_get_config_t,
    pub set_config: dc_set_config_t,
    pub precheck_imf: dc_precheck_imf_t,
    pub receive_imf: dc_receive_imf_t,
    pub userData: *mut libc::c_void,
    pub context: *mut dc_context_t,
    pub log_connect_errors: libc::c_int,
    pub skip_log_capabilities: libc::c_int,
}
pub type dc_receive_imf_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_imap_t,
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
// pub type dc_imap_t = _dc_imap;
pub type dc_precheck_imf_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_imap_t,
        _: *const libc::c_char,
        _: *const libc::c_char,
        _: uint32_t,
    ) -> libc::c_int,
>;
pub type dc_set_config_t = Option<
    unsafe extern "C" fn(_: *mut dc_imap_t, _: *const libc::c_char, _: *const libc::c_char) -> (),
>;
pub type dc_get_config_t = Option<
    unsafe extern "C" fn(
        _: *mut dc_imap_t,
        _: *const libc::c_char,
        _: *const libc::c_char,
    ) -> *mut libc::c_char,
>;
/* ** library-private **********************************************************/
use crate::dc_sqlite3::dc_sqlite3_t;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_sqlite3 {
    pub cobj: *mut sqlite3,
    pub context: *mut dc_context_t,
}
pub type dc_param_t = _dc_param;
/* *
 * @class dc_param_t
 *
 * An object for handling key=value parameter lists; for the key, curently only
 * a single character is allowed.
 *
 * The object is used eg. by dc_chat_t or dc_msg_t, for readable paramter names,
 * these classes define some DC_PARAM_* constantats.
 *
 * Only for library-internal use.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_param {
    pub packed: *mut libc::c_char,
}
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_loginparam {
    pub addr: *mut libc::c_char,
    pub mail_server: *mut libc::c_char,
    pub mail_user: *mut libc::c_char,
    pub mail_pw: *mut libc::c_char,
    pub mail_port: uint16_t,
    pub send_server: *mut libc::c_char,
    pub send_user: *mut libc::c_char,
    pub send_pw: *mut libc::c_char,
    pub send_port: libc::c_int,
    pub server_flags: libc::c_int,
}
pub type dc_loginparam_t = _dc_loginparam;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_job {
    pub job_id: uint32_t,
    pub action: libc::c_int,
    pub foreign_id: uint32_t,
    pub desired_timestamp: time_t,
    pub added_timestamp: time_t,
    pub tries: libc::c_int,
    pub param: *mut dc_param_t,
    pub try_again: libc::c_int,
    pub pending_error: *mut libc::c_char,
}
// thread IDs
// jobs in the INBOX-thread, range from DC_IMAP_THREAD..DC_IMAP_THREAD+999
// low priority ...
// ... high priority
// jobs in the SMTP-thread, range from DC_SMTP_THREAD..DC_SMTP_THREAD+999
// low priority ...
// ... high priority
// timeouts until actions are aborted.
// this may also affects IDLE to return, so a re-connect may take this time.
// mailcore2 uses 30 seconds, k-9 uses 10 seconds
pub type dc_job_t = _dc_job;
/* ******************************************************************************
 * Configure folders
 ******************************************************************************/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_imapfolder_t {
    pub name_to_select: *mut libc::c_char,
    pub name_utf8: *mut libc::c_char,
    pub meaning: libc::c_int,
}
/* ******************************************************************************
 * Thunderbird's Autoconfigure
 ******************************************************************************/
/* documentation: https://developer.mozilla.org/en-US/docs/Mozilla/Thunderbird/Autoconfiguration */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct moz_autoconfigure_t {
    pub in_0: *const dc_loginparam_t,
    pub in_emaildomain: *mut libc::c_char,
    pub in_emaillocalpart: *mut libc::c_char,
    pub out: *mut dc_loginparam_t,
    pub out_imap_set: libc::c_int,
    pub out_smtp_set: libc::c_int,
    pub tag_server: libc::c_int,
    pub tag_config: libc::c_int,
}
pub type dc_saxparser_t = _dc_saxparser;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_saxparser {
    pub starttag_cb: dc_saxparser_starttag_cb_t,
    pub endtag_cb: dc_saxparser_endtag_cb_t,
    pub text_cb: dc_saxparser_text_cb_t,
    pub userdata: *mut libc::c_void,
}
/* len is only informational, text is already null-terminated */
pub type dc_saxparser_text_cb_t = Option<
    unsafe extern "C" fn(_: *mut libc::c_void, _: *const libc::c_char, _: libc::c_int) -> (),
>;
pub type dc_saxparser_endtag_cb_t =
    Option<unsafe extern "C" fn(_: *mut libc::c_void, _: *const libc::c_char) -> ()>;
pub type dc_saxparser_starttag_cb_t = Option<
    unsafe extern "C" fn(
        _: *mut libc::c_void,
        _: *const libc::c_char,
        _: *mut *mut libc::c_char,
    ) -> (),
>;
/* ******************************************************************************
 * Outlook's Autodiscover
 ******************************************************************************/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct outlk_autodiscover_t {
    pub in_0: *const dc_loginparam_t,
    pub out: *mut dc_loginparam_t,
    pub out_imap_set: libc::c_int,
    pub out_smtp_set: libc::c_int,
    pub tag_config: libc::c_int,
    pub config: [*mut libc::c_char; 6],
    pub redirect: *mut libc::c_char,
}
// connect
#[no_mangle]
pub unsafe extern "C" fn dc_configure(mut context: *mut dc_context_t) {
    if 0 != dc_has_ongoing(context) {
        dc_log_warning(
            context,
            0i32,
            b"There is already another ongoing process running.\x00" as *const u8
                as *const libc::c_char,
        );
        return;
    }
    dc_job_kill_action(context, 900i32);
    dc_job_add(context, 900i32, 0i32, 0 as *const libc::c_char, 0i32);
}
#[no_mangle]
pub unsafe extern "C" fn dc_has_ongoing(mut context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0i32;
    }
    return if 0 != (*context).ongoing_running || (*context).shall_stop_ongoing == 0i32 {
        1i32
    } else {
        0i32
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_configured(mut context: *const dc_context_t) -> libc::c_int {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0i32;
    }
    return if 0
        != dc_sqlite3_get_config_int(
            (*context).sql,
            b"configured\x00" as *const u8 as *const libc::c_char,
            0i32,
        ) {
        1i32
    } else {
        0i32
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_stop_ongoing_process(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    if 0 != (*context).ongoing_running && (*context).shall_stop_ongoing == 0i32 {
        dc_log_info(
            context,
            0i32,
            b"Signaling the ongoing process to stop ASAP.\x00" as *const u8 as *const libc::c_char,
        );
        (*context).shall_stop_ongoing = 1i32
    } else {
        dc_log_info(
            context,
            0i32,
            b"No ongoing process to stop.\x00" as *const u8 as *const libc::c_char,
        );
    };
}
// the other dc_job_do_DC_JOB_*() functions are declared static in the c-file
#[no_mangle]
pub unsafe extern "C" fn dc_job_do_DC_JOB_CONFIGURE_IMAP(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut flags: libc::c_int = 0;
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut imap_connected_here: libc::c_int = 0i32;
    let mut smtp_connected_here: libc::c_int = 0i32;
    let mut ongoing_allocated_here: libc::c_int = 0i32;
    let mut mvbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    /* just a pointer inside param, must not be freed! */
    let mut param_domain: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param_addr_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param_autoconfig: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(0 == dc_alloc_ongoing(context)) {
            ongoing_allocated_here = 1i32;
            if 0 == dc_sqlite3_is_open((*context).sql) {
                dc_log_error(
                    context,
                    0i32,
                    b"Cannot configure, database not opened.\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                dc_imap_disconnect((*context).inbox);
                dc_imap_disconnect((*context).sentbox_thread.imap);
                dc_imap_disconnect((*context).mvbox_thread.imap);
                dc_smtp_disconnect((*context).smtp);
                (*(*context).smtp).log_connect_errors = 1i32;
                (*(*context).inbox).log_connect_errors = 1i32;
                (*(*context).sentbox_thread.imap).log_connect_errors = 1i32;
                (*(*context).mvbox_thread.imap).log_connect_errors = 1i32;
                dc_log_info(
                    context,
                    0i32,
                    b"Configure ...\x00" as *const u8 as *const libc::c_char,
                );
                if !(0 != (*context).shall_stop_ongoing) {
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2041i32,
                        (if 0i32 < 1i32 {
                            1i32
                        } else if 0i32 > 999i32 {
                            999i32
                        } else {
                            0i32
                        }) as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    param = dc_loginparam_new();
                    dc_loginparam_read(
                        param,
                        (*context).sql,
                        b"\x00" as *const u8 as *const libc::c_char,
                    );
                    if (*param).addr.is_null() {
                        dc_log_error(
                            context,
                            0i32,
                            b"Please enter the email address.\x00" as *const u8
                                as *const libc::c_char,
                        );
                    } else {
                        dc_trim((*param).addr);
                        if 0 != (*param).server_flags & 0x2i32 {
                            // the used oauth2 addr may differ, check this.
                            // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                            // just use the given one.
                            if 0 != (*context).shall_stop_ongoing {
                                current_block = 2927484062889439186;
                            } else {
                                (*context).cb.expect("non-null function pointer")(
                                    context,
                                    2041i32,
                                    (if 10i32 < 1i32 {
                                        1i32
                                    } else if 10i32 > 999i32 {
                                        999i32
                                    } else {
                                        10i32
                                    }) as uintptr_t,
                                    0i32 as uintptr_t,
                                );
                                let mut oauth2_addr: *mut libc::c_char =
                                    dc_get_oauth2_addr(context, (*param).addr, (*param).mail_pw);
                                if !oauth2_addr.is_null() {
                                    free((*param).addr as *mut libc::c_void);
                                    (*param).addr = oauth2_addr;
                                    dc_sqlite3_set_config(
                                        (*context).sql,
                                        b"addr\x00" as *const u8 as *const libc::c_char,
                                        (*param).addr,
                                    );
                                }
                                if 0 != (*context).shall_stop_ongoing {
                                    current_block = 2927484062889439186;
                                } else {
                                    (*context).cb.expect("non-null function pointer")(
                                        context,
                                        2041i32,
                                        (if 20i32 < 1i32 {
                                            1i32
                                        } else if 20i32 > 999i32 {
                                            999i32
                                        } else {
                                            20i32
                                        }) as uintptr_t,
                                        0i32 as uintptr_t,
                                    );
                                    current_block = 7746103178988627676;
                                }
                            }
                        } else {
                            current_block = 7746103178988627676;
                        }
                        match current_block {
                            2927484062889439186 => {}
                            _ => {
                                param_domain = strchr((*param).addr, '@' as i32);
                                if param_domain.is_null()
                                    || *param_domain.offset(0isize) as libc::c_int == 0i32
                                {
                                    dc_log_error(
                                        context,
                                        0i32,
                                        b"Bad email-address.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                } else {
                                    param_domain = param_domain.offset(1isize);
                                    param_addr_urlencoded = dc_urlencode((*param).addr);
                                    if (*param).mail_pw.is_null() {
                                        (*param).mail_pw = dc_strdup(0 as *const libc::c_char)
                                    }
                                    if !(0 != (*context).shall_stop_ongoing) {
                                        (*context).cb.expect("non-null function pointer")(
                                            context,
                                            2041i32,
                                            (if 200i32 < 1i32 {
                                                1i32
                                            } else if 200i32 > 999i32 {
                                                999i32
                                            } else {
                                                200i32
                                            })
                                                as uintptr_t,
                                            0i32 as uintptr_t,
                                        );
                                        /* 2.  Autoconfig
                                         **************************************************************************/
                                        if (*param).mail_server.is_null()
                                            && (*param).mail_port as libc::c_int == 0i32
                                            && (*param).send_server.is_null()
                                            && (*param).send_port == 0i32
                                            && (*param).send_user.is_null()
                                            && (*param).server_flags & !0x2i32 == 0i32
                                        {
                                            /*&&param->mail_user   ==NULL -- the user can enter a loginname which is used by autoconfig then */
                                            /*&&param->send_pw     ==NULL -- the password cannot be auto-configured and is no criterion for autoconfig or not */
                                            /* flags but OAuth2 avoid autoconfig */
                                            let mut keep_flags: libc::c_int =
                                                (*param).server_flags & 0x2i32;
                                            /* A.  Search configurations from the domain used in the email-address, prefer encrypted */
                                            if param_autoconfig.is_null() {
                                                let mut url:
                                                        *mut libc::c_char =
                                                    dc_mprintf(b"https://autoconfig.%s/mail/config-v1.1.xml?emailaddress=%s\x00"
                                                                   as
                                                                   *const u8
                                                                   as
                                                                   *const libc::c_char,
                                                               param_domain,
                                                               param_addr_urlencoded);
                                                param_autoconfig =
                                                    moz_autoconfigure(context, url, param);
                                                free(url as *mut libc::c_void);
                                                if 0 != (*context).shall_stop_ongoing {
                                                    current_block = 2927484062889439186;
                                                } else {
                                                    (*context)
                                                        .cb
                                                        .expect("non-null function pointer")(
                                                        context,
                                                        2041i32,
                                                        (if 300i32 < 1i32 {
                                                            1i32
                                                        } else if 300i32 > 999i32 {
                                                            999i32
                                                        } else {
                                                            300i32
                                                        })
                                                            as uintptr_t,
                                                        0i32 as uintptr_t,
                                                    );
                                                    current_block = 13325891313334703151;
                                                }
                                            } else {
                                                current_block = 13325891313334703151;
                                            }
                                            match current_block {
                                                2927484062889439186 => {}
                                                _ => {
                                                    if param_autoconfig.is_null() {
                                                        // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see https://releases.mozilla.org/pub/thunderbird/ ,  which makes some sense
                                                        let mut url_0:
                                                                *mut libc::c_char =
                                                            dc_mprintf(b"https://%s/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress=%s\x00"
                                                                           as
                                                                           *const u8
                                                                           as
                                                                           *const libc::c_char,
                                                                       param_domain,
                                                                       param_addr_urlencoded);
                                                        param_autoconfig = moz_autoconfigure(
                                                            context, url_0, param,
                                                        );
                                                        free(url_0 as *mut libc::c_void);
                                                        if 0 != (*context).shall_stop_ongoing {
                                                            current_block = 2927484062889439186;
                                                        } else {
                                                            (*context).cb.expect(
                                                                "non-null function pointer",
                                                            )(
                                                                context,
                                                                2041i32,
                                                                (if 310i32 < 1i32 {
                                                                    1i32
                                                                } else if 310i32 > 999i32 {
                                                                    999i32
                                                                } else {
                                                                    310i32
                                                                })
                                                                    as uintptr_t,
                                                                0i32 as uintptr_t,
                                                            );
                                                            current_block = 5597585068398118923;
                                                        }
                                                    } else {
                                                        current_block = 5597585068398118923;
                                                    }
                                                    match current_block {
                                                        2927484062889439186 => {}
                                                        _ => {
                                                            let mut i: libc::c_int = 0i32;
                                                            loop {
                                                                if !(i <= 1i32) {
                                                                    current_block =
                                                                        12961834331865314435;
                                                                    break;
                                                                }
                                                                if param_autoconfig.is_null() {
                                                                    /* Outlook uses always SSL but different domains */
                                                                    let mut url_1:
                                                                            *mut libc::c_char =
                                                                        dc_mprintf(b"https://%s%s/autodiscover/autodiscover.xml\x00"
                                                                                       as
                                                                                       *const u8
                                                                                       as
                                                                                       *const libc::c_char,
                                                                                   if i
                                                                                          ==
                                                                                          0i32
                                                                                      {
                                                                                       b"\x00"
                                                                                           as
                                                                                           *const u8
                                                                                           as
                                                                                           *const libc::c_char
                                                                                   } else {
                                                                                       b"autodiscover.\x00"
                                                                                           as
                                                                                           *const u8
                                                                                           as
                                                                                           *const libc::c_char
                                                                                   },
                                                                                   param_domain);
                                                                    param_autoconfig =
                                                                        outlk_autodiscover(
                                                                            context, url_1, param,
                                                                        );
                                                                    free(
                                                                        url_1 as *mut libc::c_void,
                                                                    );
                                                                    if 0 != (*context)
                                                                        .shall_stop_ongoing
                                                                    {
                                                                        current_block =
                                                                            2927484062889439186;
                                                                        break;
                                                                    }
                                                                    (*context).cb.expect(
                                                                        "non-null function pointer",
                                                                    )(
                                                                        context,
                                                                        2041i32,
                                                                        (if 320i32 + i * 10i32
                                                                            < 1i32
                                                                        {
                                                                            1i32
                                                                        } else if 320i32 + i * 10i32
                                                                            > 999i32
                                                                        {
                                                                            999i32
                                                                        } else {
                                                                            320i32 + i * 10i32
                                                                        })
                                                                            as uintptr_t,
                                                                        0i32 as uintptr_t,
                                                                    );
                                                                }
                                                                i += 1
                                                            }
                                                            match current_block {
                                                                2927484062889439186 => {}
                                                                _ => {
                                                                    if param_autoconfig.is_null() {
                                                                        let mut url_2:
                                                                                *mut libc::c_char =
                                                                            dc_mprintf(b"http://autoconfig.%s/mail/config-v1.1.xml?emailaddress=%s\x00"
                                                                                           as
                                                                                           *const u8
                                                                                           as
                                                                                           *const libc::c_char,
                                                                                       param_domain,
                                                                                       param_addr_urlencoded);
                                                                        param_autoconfig =
                                                                            moz_autoconfigure(
                                                                                context, url_2,
                                                                                param,
                                                                            );
                                                                        free(url_2
                                                                                 as
                                                                                 *mut libc::c_void);
                                                                        if 0 != (*context)
                                                                            .shall_stop_ongoing
                                                                        {
                                                                            current_block =
                                                                                2927484062889439186;
                                                                        } else {
                                                                            (*context).cb.expect("non-null function pointer")(context,
                                                                                                                              2041i32,
                                                                                                                              (if 340i32
                                                                                                                                      <
                                                                                                                                      1i32
                                                                                                                                  {
                                                                                                                                   1i32
                                                                                                                               } else if 340i32
                                                                                                                                             >
                                                                                                                                             999i32
                                                                                                                                {
                                                                                                                                   999i32
                                                                                                                               } else {
                                                                                                                                   340i32
                                                                                                                               })
                                                                                                                                  as
                                                                                                                                  uintptr_t,
                                                                                                                              0i32
                                                                                                                                  as
                                                                                                                                  uintptr_t);
                                                                            current_block
                                                                                =
                                                                                10778260831612459202;
                                                                        }
                                                                    } else {
                                                                        current_block =
                                                                            10778260831612459202;
                                                                    }
                                                                    match current_block {
                                                                        2927484062889439186 => {}
                                                                        _ => {
                                                                            if param_autoconfig
                                                                                .is_null()
                                                                            {
                                                                                // do not transfer the email-address unencrypted
                                                                                let mut url_3:
                                                                                        *mut libc::c_char =
                                                                                    dc_mprintf(b"http://%s/.well-known/autoconfig/mail/config-v1.1.xml\x00"
                                                                                                   as
                                                                                                   *const u8
                                                                                                   as
                                                                                                   *const libc::c_char,
                                                                                               param_domain);
                                                                                param_autoconfig
                                                                                    =
                                                                                    moz_autoconfigure(context,
                                                                                                      url_3,
                                                                                                      param);
                                                                                free(url_3
                                                                                         as
                                                                                         *mut libc::c_void);
                                                                                if 0
                                                                                       !=
                                                                                       (*context).shall_stop_ongoing
                                                                                   {
                                                                                    current_block
                                                                                        =
                                                                                        2927484062889439186;
                                                                                } else {
                                                                                    (*context).cb.expect("non-null function pointer")(context,
                                                                                                                                      2041i32,
                                                                                                                                      (if 350i32
                                                                                                                                              <
                                                                                                                                              1i32
                                                                                                                                          {
                                                                                                                                           1i32
                                                                                                                                       } else if 350i32
                                                                                                                                                     >
                                                                                                                                                     999i32
                                                                                                                                        {
                                                                                                                                           999i32
                                                                                                                                       } else {
                                                                                                                                           350i32
                                                                                                                                       })
                                                                                                                                          as
                                                                                                                                          uintptr_t,
                                                                                                                                      0i32
                                                                                                                                          as
                                                                                                                                          uintptr_t);
                                                                                    current_block
                                                                                        =
                                                                                        5207889489643863322;
                                                                                }
                                                                            } else {
                                                                                current_block
                                                                                    =
                                                                                    5207889489643863322;
                                                                            }
                                                                            match current_block
                                                                                {
                                                                                2927484062889439186
                                                                                =>
                                                                                {
                                                                                }
                                                                                _
                                                                                =>
                                                                                {
                                                                                    /* B.  If we have no configuration yet, search configuration in Thunderbird's centeral database */
                                                                                    if param_autoconfig.is_null()
                                                                                       {
                                                                                        /* always SSL for Thunderbird's database */
                                                                                        let mut url_4:
                                                                                                *mut libc::c_char =
                                                                                            dc_mprintf(b"https://autoconfig.thunderbird.net/v1.1/%s\x00"
                                                                                                           as
                                                                                                           *const u8
                                                                                                           as
                                                                                                           *const libc::c_char,
                                                                                                       param_domain);
                                                                                        param_autoconfig
                                                                                            =
                                                                                            moz_autoconfigure(context,
                                                                                                              url_4,
                                                                                                              param);
                                                                                        free(url_4
                                                                                                 as
                                                                                                 *mut libc::c_void);
                                                                                        if 0
                                                                                               !=
                                                                                               (*context).shall_stop_ongoing
                                                                                           {
                                                                                            current_block
                                                                                                =
                                                                                                2927484062889439186;
                                                                                        } else {
                                                                                            (*context).cb.expect("non-null function pointer")(context,
                                                                                                                                              2041i32,
                                                                                                                                              (if 500i32
                                                                                                                                                      <
                                                                                                                                                      1i32
                                                                                                                                                  {
                                                                                                                                                   1i32
                                                                                                                                               } else if 500i32
                                                                                                                                                             >
                                                                                                                                                             999i32
                                                                                                                                                {
                                                                                                                                                   999i32
                                                                                                                                               } else {
                                                                                                                                                   500i32
                                                                                                                                               })
                                                                                                                                                  as
                                                                                                                                                  uintptr_t,
                                                                                                                                              0i32
                                                                                                                                                  as
                                                                                                                                                  uintptr_t);
                                                                                            current_block
                                                                                                =
                                                                                                2798392256336243897;
                                                                                        }
                                                                                    } else {
                                                                                        current_block
                                                                                            =
                                                                                            2798392256336243897;
                                                                                    }
                                                                                    match current_block
                                                                                        {
                                                                                        2927484062889439186
                                                                                        =>
                                                                                        {
                                                                                        }
                                                                                        _
                                                                                        =>
                                                                                        {
                                                                                            if !param_autoconfig.is_null()
                                                                                               {
                                                                                                let mut r:
                                                                                                        *mut libc::c_char =
                                                                                                    dc_loginparam_get_readable(param_autoconfig);
                                                                                                dc_log_info(context,
                                                                                                            0i32,
                                                                                                            b"Got autoconfig: %s\x00"
                                                                                                                as
                                                                                                                *const u8
                                                                                                                as
                                                                                                                *const libc::c_char,
                                                                                                            r);
                                                                                                free(r
                                                                                                         as
                                                                                                         *mut libc::c_void);
                                                                                                if !(*param_autoconfig).mail_user.is_null()
                                                                                                   {
                                                                                                    free((*param).mail_user
                                                                                                             as
                                                                                                             *mut libc::c_void);
                                                                                                    (*param).mail_user
                                                                                                        =
                                                                                                        dc_strdup_keep_null((*param_autoconfig).mail_user)
                                                                                                }
                                                                                                (*param).mail_server
                                                                                                    =
                                                                                                    dc_strdup_keep_null((*param_autoconfig).mail_server);
                                                                                                (*param).mail_port
                                                                                                    =
                                                                                                    (*param_autoconfig).mail_port;
                                                                                                (*param).send_server
                                                                                                    =
                                                                                                    dc_strdup_keep_null((*param_autoconfig).send_server);
                                                                                                (*param).send_port
                                                                                                    =
                                                                                                    (*param_autoconfig).send_port;
                                                                                                (*param).send_user
                                                                                                    =
                                                                                                    dc_strdup_keep_null((*param_autoconfig).send_user);
                                                                                                (*param).server_flags
                                                                                                    =
                                                                                                    (*param_autoconfig).server_flags
                                                                                            }
                                                                                            (*param).server_flags
                                                                                                |=
                                                                                                keep_flags;
                                                                                            current_block
                                                                                                =
                                                                                                3024367268842933116;
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
                                        } else {
                                            current_block = 3024367268842933116;
                                        }
                                        match current_block {
                                            2927484062889439186 => {}
                                            _ => {
                                                if (*param).mail_server.is_null() {
                                                    (*param).mail_server = dc_mprintf(
                                                        b"imap.%s\x00" as *const u8
                                                            as *const libc::c_char,
                                                        param_domain,
                                                    )
                                                }
                                                if (*param).mail_port as libc::c_int == 0i32 {
                                                    (*param).mail_port = (if 0
                                                        != (*param).server_flags
                                                            & (0x100i32 | 0x400i32)
                                                    {
                                                        143i32
                                                    } else {
                                                        993i32
                                                    })
                                                        as uint16_t
                                                }
                                                if (*param).mail_user.is_null() {
                                                    (*param).mail_user = dc_strdup((*param).addr)
                                                }
                                                if (*param).send_server.is_null()
                                                    && !(*param).mail_server.is_null()
                                                {
                                                    (*param).send_server =
                                                        dc_strdup((*param).mail_server);
                                                    if strncmp(
                                                        (*param).send_server,
                                                        b"imap.\x00" as *const u8
                                                            as *const libc::c_char,
                                                        5i32 as libc::c_ulong,
                                                    ) == 0i32
                                                    {
                                                        memcpy(
                                                            (*param).send_server
                                                                as *mut libc::c_void,
                                                            b"smtp\x00" as *const u8
                                                                as *const libc::c_char
                                                                as *const libc::c_void,
                                                            4i32 as libc::c_ulong,
                                                        );
                                                    }
                                                }
                                                if (*param).send_port == 0i32 {
                                                    (*param).send_port =
                                                        if 0 != (*param).server_flags & 0x10000i32 {
                                                            587i32
                                                        } else if 0
                                                            != (*param).server_flags & 0x40000i32
                                                        {
                                                            25i32
                                                        } else {
                                                            465i32
                                                        }
                                                }
                                                if (*param).send_user.is_null()
                                                    && !(*param).mail_user.is_null()
                                                {
                                                    (*param).send_user =
                                                        dc_strdup((*param).mail_user)
                                                }
                                                if (*param).send_pw.is_null()
                                                    && !(*param).mail_pw.is_null()
                                                {
                                                    (*param).send_pw = dc_strdup((*param).mail_pw)
                                                }
                                                if 0 == dc_exactly_one_bit_set(
                                                    (*param).server_flags & (0x2i32 | 0x4i32),
                                                ) {
                                                    (*param).server_flags &= !(0x2i32 | 0x4i32);
                                                    (*param).server_flags |= 0x4i32
                                                }
                                                if 0 == dc_exactly_one_bit_set(
                                                    (*param).server_flags
                                                        & (0x100i32 | 0x200i32 | 0x400i32),
                                                ) {
                                                    (*param).server_flags &=
                                                        !(0x100i32 | 0x200i32 | 0x400i32);
                                                    (*param).server_flags |=
                                                        if (*param).send_port == 143i32 {
                                                            0x100i32
                                                        } else {
                                                            0x200i32
                                                        }
                                                }
                                                if 0 == dc_exactly_one_bit_set(
                                                    (*param).server_flags
                                                        & (0x10000i32 | 0x20000i32 | 0x40000i32),
                                                ) {
                                                    (*param).server_flags &=
                                                        !(0x10000i32 | 0x20000i32 | 0x40000i32);
                                                    (*param).server_flags |=
                                                        if (*param).send_port == 587i32 {
                                                            0x10000i32
                                                        } else if (*param).send_port == 25i32 {
                                                            0x40000i32
                                                        } else {
                                                            0x20000i32
                                                        }
                                                }
                                                /* do we have a complete configuration? */
                                                if (*param).addr.is_null()
                                                    || (*param).mail_server.is_null()
                                                    || (*param).mail_port as libc::c_int == 0i32
                                                    || (*param).mail_user.is_null()
                                                    || (*param).mail_pw.is_null()
                                                    || (*param).send_server.is_null()
                                                    || (*param).send_port == 0i32
                                                    || (*param).send_user.is_null()
                                                    || (*param).send_pw.is_null()
                                                    || (*param).server_flags == 0i32
                                                {
                                                    dc_log_error(
                                                        context,
                                                        0i32,
                                                        b"Account settings incomplete.\x00"
                                                            as *const u8
                                                            as *const libc::c_char,
                                                    );
                                                } else if !(0 != (*context).shall_stop_ongoing) {
                                                    (*context)
                                                        .cb
                                                        .expect("non-null function pointer")(
                                                        context,
                                                        2041i32,
                                                        (if 600i32 < 1i32 {
                                                            1i32
                                                        } else if 600i32 > 999i32 {
                                                            999i32
                                                        } else {
                                                            600i32
                                                        })
                                                            as uintptr_t,
                                                        0i32 as uintptr_t,
                                                    );
                                                    /* try to connect to IMAP - if we did not got an autoconfig,
                                                    do some further tries with different settings and username variations */
                                                    let mut username_variation: libc::c_int = 0i32;
                                                    loop {
                                                        if !(username_variation <= 1i32) {
                                                            current_block = 14187386403465544025;
                                                            break;
                                                        }
                                                        let mut r_0: *mut libc::c_char =
                                                            dc_loginparam_get_readable(param);
                                                        dc_log_info(
                                                            context,
                                                            0i32,
                                                            b"Trying: %s\x00" as *const u8
                                                                as *const libc::c_char,
                                                            r_0,
                                                        );
                                                        free(r_0 as *mut libc::c_void);
                                                        if 0 != dc_imap_connect(
                                                            (*context).inbox,
                                                            param,
                                                        ) {
                                                            current_block = 14187386403465544025;
                                                            break;
                                                        }
                                                        if !param_autoconfig.is_null() {
                                                            current_block = 2927484062889439186;
                                                            break;
                                                        }
                                                        // probe STARTTLS/993
                                                        if 0 != (*context).shall_stop_ongoing {
                                                            current_block = 2927484062889439186;
                                                            break;
                                                        }
                                                        (*context)
                                                            .cb
                                                            .expect("non-null function pointer")(
                                                            context,
                                                            2041i32,
                                                            (if 650i32 + username_variation * 30i32
                                                                < 1i32
                                                            {
                                                                1i32
                                                            } else if 650i32
                                                                + username_variation * 30i32
                                                                > 999i32
                                                            {
                                                                999i32
                                                            } else {
                                                                650i32 + username_variation * 30i32
                                                            })
                                                                as uintptr_t,
                                                            0i32 as uintptr_t,
                                                        );
                                                        (*param).server_flags &=
                                                            !(0x100i32 | 0x200i32 | 0x400i32);
                                                        (*param).server_flags |= 0x100i32;
                                                        let mut r_1: *mut libc::c_char =
                                                            dc_loginparam_get_readable(param);
                                                        dc_log_info(
                                                            context,
                                                            0i32,
                                                            b"Trying: %s\x00" as *const u8
                                                                as *const libc::c_char,
                                                            r_1,
                                                        );
                                                        free(r_1 as *mut libc::c_void);
                                                        if 0 != dc_imap_connect(
                                                            (*context).inbox,
                                                            param,
                                                        ) {
                                                            current_block = 14187386403465544025;
                                                            break;
                                                        }
                                                        // probe STARTTLS/143
                                                        if 0 != (*context).shall_stop_ongoing {
                                                            current_block = 2927484062889439186;
                                                            break;
                                                        }
                                                        (*context)
                                                            .cb
                                                            .expect("non-null function pointer")(
                                                            context,
                                                            2041i32,
                                                            (if 660i32 + username_variation * 30i32
                                                                < 1i32
                                                            {
                                                                1i32
                                                            } else if 660i32
                                                                + username_variation * 30i32
                                                                > 999i32
                                                            {
                                                                999i32
                                                            } else {
                                                                660i32 + username_variation * 30i32
                                                            })
                                                                as uintptr_t,
                                                            0i32 as uintptr_t,
                                                        );
                                                        (*param).mail_port = 143i32 as uint16_t;
                                                        let mut r_2: *mut libc::c_char =
                                                            dc_loginparam_get_readable(param);
                                                        dc_log_info(
                                                            context,
                                                            0i32,
                                                            b"Trying: %s\x00" as *const u8
                                                                as *const libc::c_char,
                                                            r_2,
                                                        );
                                                        free(r_2 as *mut libc::c_void);
                                                        if 0 != dc_imap_connect(
                                                            (*context).inbox,
                                                            param,
                                                        ) {
                                                            current_block = 14187386403465544025;
                                                            break;
                                                        }
                                                        if 0 != username_variation {
                                                            current_block = 2927484062889439186;
                                                            break;
                                                        }
                                                        // next probe round with only the localpart of the email-address as the loginname
                                                        if 0 != (*context).shall_stop_ongoing {
                                                            current_block = 2927484062889439186;
                                                            break;
                                                        }
                                                        (*context)
                                                            .cb
                                                            .expect("non-null function pointer")(
                                                            context,
                                                            2041i32,
                                                            (if 670i32 + username_variation * 30i32
                                                                < 1i32
                                                            {
                                                                1i32
                                                            } else if 670i32
                                                                + username_variation * 30i32
                                                                > 999i32
                                                            {
                                                                999i32
                                                            } else {
                                                                670i32 + username_variation * 30i32
                                                            })
                                                                as uintptr_t,
                                                            0i32 as uintptr_t,
                                                        );
                                                        (*param).server_flags &=
                                                            !(0x100i32 | 0x200i32 | 0x400i32);
                                                        (*param).server_flags |= 0x200i32;
                                                        (*param).mail_port = 993i32 as uint16_t;
                                                        let mut at: *mut libc::c_char =
                                                            strchr((*param).mail_user, '@' as i32);
                                                        if !at.is_null() {
                                                            *at = 0i32 as libc::c_char
                                                        }
                                                        at = strchr((*param).send_user, '@' as i32);
                                                        if !at.is_null() {
                                                            *at = 0i32 as libc::c_char
                                                        }
                                                        username_variation += 1
                                                    }
                                                    match current_block {
                                                        2927484062889439186 => {}
                                                        _ => {
                                                            imap_connected_here = 1i32;
                                                            if !(0 != (*context).shall_stop_ongoing)
                                                            {
                                                                (*context).cb.expect(
                                                                    "non-null function pointer",
                                                                )(
                                                                    context,
                                                                    2041i32,
                                                                    (if 800i32 < 1i32 {
                                                                        1i32
                                                                    } else if 800i32 > 999i32 {
                                                                        999i32
                                                                    } else {
                                                                        800i32
                                                                    })
                                                                        as uintptr_t,
                                                                    0i32 as uintptr_t,
                                                                );
                                                                /* try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do a second try with STARTTLS-587 */
                                                                if 0 == dc_smtp_connect(
                                                                    (*context).smtp,
                                                                    param,
                                                                ) {
                                                                    if !param_autoconfig.is_null() {
                                                                        current_block =
                                                                            2927484062889439186;
                                                                    } else if 0
                                                                        != (*context)
                                                                            .shall_stop_ongoing
                                                                    {
                                                                        current_block =
                                                                            2927484062889439186;
                                                                    } else {
                                                                        (*context).cb.expect("non-null function pointer")(context,
                                                                                                                          2041i32,
                                                                                                                          (if 850i32
                                                                                                                                  <
                                                                                                                                  1i32
                                                                                                                              {
                                                                                                                               1i32
                                                                                                                           } else if 850i32
                                                                                                                                         >
                                                                                                                                         999i32
                                                                                                                            {
                                                                                                                               999i32
                                                                                                                           } else {
                                                                                                                               850i32
                                                                                                                           })
                                                                                                                              as
                                                                                                                              uintptr_t,
                                                                                                                          0i32
                                                                                                                              as
                                                                                                                              uintptr_t);
                                                                        (*param).server_flags &=
                                                                            !(0x10000i32
                                                                                | 0x20000i32
                                                                                | 0x40000i32);
                                                                        (*param).server_flags |=
                                                                            0x10000i32;
                                                                        (*param).send_port = 587i32;
                                                                        let mut r_3:
                                                                                *mut libc::c_char =
                                                                            dc_loginparam_get_readable(param);
                                                                        dc_log_info(context,
                                                                                    0i32,
                                                                                    b"Trying: %s\x00"
                                                                                        as
                                                                                        *const u8
                                                                                        as
                                                                                        *const libc::c_char,
                                                                                    r_3);
                                                                        free(r_3
                                                                                 as
                                                                                 *mut libc::c_void);
                                                                        if 0 == dc_smtp_connect(
                                                                            (*context).smtp,
                                                                            param,
                                                                        ) {
                                                                            if 0 != (*context)
                                                                                .shall_stop_ongoing
                                                                            {
                                                                                current_block
                                                                                    =
                                                                                    2927484062889439186;
                                                                            } else {
                                                                                (*context).cb.expect("non-null function pointer")(context,
                                                                                                                                  2041i32,
                                                                                                                                  (if 860i32
                                                                                                                                          <
                                                                                                                                          1i32
                                                                                                                                      {
                                                                                                                                       1i32
                                                                                                                                   } else if 860i32
                                                                                                                                                 >
                                                                                                                                                 999i32
                                                                                                                                    {
                                                                                                                                       999i32
                                                                                                                                   } else {
                                                                                                                                       860i32
                                                                                                                                   })
                                                                                                                                      as
                                                                                                                                      uintptr_t,
                                                                                                                                  0i32
                                                                                                                                      as
                                                                                                                                      uintptr_t);
                                                                                (*param).server_flags
                                                                                    &=
                                                                                    !(0x10000i32
                                                                                          |
                                                                                          0x20000i32
                                                                                          |
                                                                                          0x40000i32);
                                                                                (*param).server_flags
                                                                                    |=
                                                                                    0x10000i32;
                                                                                (*param)
                                                                                    .send_port =
                                                                                    25i32;
                                                                                let mut r_4:
                                                                                        *mut libc::c_char =
                                                                                    dc_loginparam_get_readable(param);
                                                                                dc_log_info(context,
                                                                                            0i32,
                                                                                            b"Trying: %s\x00"
                                                                                                as
                                                                                                *const u8
                                                                                                as
                                                                                                *const libc::c_char,
                                                                                            r_4);
                                                                                free(r_4
                                                                                         as
                                                                                         *mut libc::c_void);
                                                                                if 0
                                                                                       ==
                                                                                       dc_smtp_connect((*context).smtp,
                                                                                                       param)
                                                                                   {
                                                                                    current_block
                                                                                        =
                                                                                        2927484062889439186;
                                                                                } else {
                                                                                    current_block
                                                                                        =
                                                                                        5083741289379115417;
                                                                                }
                                                                            }
                                                                        } else {
                                                                            current_block =
                                                                                5083741289379115417;
                                                                        }
                                                                    }
                                                                } else {
                                                                    current_block =
                                                                        5083741289379115417;
                                                                }
                                                                match current_block {
                                                                    2927484062889439186 => {}
                                                                    _ => {
                                                                        smtp_connected_here = 1i32;
                                                                        if !(0
                                                                            != (*context)
                                                                                .shall_stop_ongoing)
                                                                        {
                                                                            (*context).cb.expect("non-null function pointer")(context,
                                                                                                                              2041i32,
                                                                                                                              (if 900i32
                                                                                                                                      <
                                                                                                                                      1i32
                                                                                                                                  {
                                                                                                                                   1i32
                                                                                                                               } else if 900i32
                                                                                                                                             >
                                                                                                                                             999i32
                                                                                                                                {
                                                                                                                                   999i32
                                                                                                                               } else {
                                                                                                                                   900i32
                                                                                                                               })
                                                                                                                                  as
                                                                                                                                  uintptr_t,
                                                                                                                              0i32
                                                                                                                                  as
                                                                                                                                  uintptr_t);
                                                                            flags
                                                                                =
                                                                                if 0
                                                                                       !=
                                                                                       dc_sqlite3_get_config_int((*context).sql,
                                                                                                                 b"mvbox_watch\x00"
                                                                                                                     as
                                                                                                                     *const u8
                                                                                                                     as
                                                                                                                     *const libc::c_char,
                                                                                                                 1i32)
                                                                                       ||
                                                                                       0
                                                                                           !=
                                                                                           dc_sqlite3_get_config_int((*context).sql,
                                                                                                                     b"mvbox_move\x00"
                                                                                                                         as
                                                                                                                         *const u8
                                                                                                                         as
                                                                                                                         *const libc::c_char,
                                                                                                                     1i32)
                                                                                   {
                                                                                    0x1i32
                                                                                } else {
                                                                                    0i32
                                                                                };
                                                                            dc_configure_folders(
                                                                                context,
                                                                                (*context).inbox,
                                                                                flags,
                                                                            );
                                                                            if !(0 != (*context)
                                                                                .shall_stop_ongoing)
                                                                            {
                                                                                (*context).cb.expect("non-null function pointer")(context,
                                                                                                                                  2041i32,
                                                                                                                                  (if 910i32
                                                                                                                                          <
                                                                                                                                          1i32
                                                                                                                                      {
                                                                                                                                       1i32
                                                                                                                                   } else if 910i32
                                                                                                                                                 >
                                                                                                                                                 999i32
                                                                                                                                    {
                                                                                                                                       999i32
                                                                                                                                   } else {
                                                                                                                                       910i32
                                                                                                                                   })
                                                                                                                                      as
                                                                                                                                      uintptr_t,
                                                                                                                                  0i32
                                                                                                                                      as
                                                                                                                                      uintptr_t);
                                                                                dc_loginparam_write(param,
                                                                                                    (*context).sql,
                                                                                                    b"configured_\x00"
                                                                                                        as
                                                                                                        *const u8
                                                                                                        as
                                                                                                        *const libc::c_char);
                                                                                dc_sqlite3_set_config_int((*context).sql,
                                                                                                          b"configured\x00"
                                                                                                              as
                                                                                                              *const u8
                                                                                                              as
                                                                                                              *const libc::c_char,
                                                                                                          1i32);
                                                                                if !(0
                                                                                         !=
                                                                                         (*context).shall_stop_ongoing)
                                                                                   {
                                                                                    (*context).cb.expect("non-null function pointer")(context,
                                                                                                                                      2041i32,
                                                                                                                                      (if 920i32
                                                                                                                                              <
                                                                                                                                              1i32
                                                                                                                                          {
                                                                                                                                           1i32
                                                                                                                                       } else if 920i32
                                                                                                                                                     >
                                                                                                                                                     999i32
                                                                                                                                        {
                                                                                                                                           999i32
                                                                                                                                       } else {
                                                                                                                                           920i32
                                                                                                                                       })
                                                                                                                                          as
                                                                                                                                          uintptr_t,
                                                                                                                                      0i32
                                                                                                                                          as
                                                                                                                                          uintptr_t);
                                                                                    dc_ensure_secret_key_exists(context);
                                                                                    success
                                                                                        =
                                                                                        1i32;
                                                                                    dc_log_info(context,
                                                                                                0i32,
                                                                                                b"Configure completed.\x00"
                                                                                                    as
                                                                                                    *const u8
                                                                                                    as
                                                                                                    *const libc::c_char);
                                                                                    if !(0
                                                                                             !=
                                                                                             (*context).shall_stop_ongoing)
                                                                                       {
                                                                                        (*context).cb.expect("non-null function pointer")(context,
                                                                                                                                          2041i32,
                                                                                                                                          (if 940i32
                                                                                                                                                  <
                                                                                                                                                  1i32
                                                                                                                                              {
                                                                                                                                               1i32
                                                                                                                                           } else if 940i32
                                                                                                                                                         >
                                                                                                                                                         999i32
                                                                                                                                            {
                                                                                                                                               999i32
                                                                                                                                           } else {
                                                                                                                                               940i32
                                                                                                                                           })
                                                                                                                                              as
                                                                                                                                              uintptr_t,
                                                                                                                                          0i32
                                                                                                                                              as
                                                                                                                                              uintptr_t);
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
    if 0 != imap_connected_here {
        dc_imap_disconnect((*context).inbox);
    }
    if 0 != smtp_connected_here {
        dc_smtp_disconnect((*context).smtp);
    }
    dc_loginparam_unref(param);
    dc_loginparam_unref(param_autoconfig);
    free(param_addr_urlencoded as *mut libc::c_void);
    if 0 != ongoing_allocated_here {
        dc_free_ongoing(context);
    }
    free(mvbox_folder as *mut libc::c_void);
    (*context).cb.expect("non-null function pointer")(
        context,
        2041i32,
        (if 0 != success { 1000i32 } else { 0i32 }) as uintptr_t,
        0i32 as uintptr_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_free_ongoing(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    (*context).ongoing_running = 0i32;
    (*context).shall_stop_ongoing = 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_configure_folders(
    mut context: *mut dc_context_t,
    mut imap: *mut dc_imap_t,
    mut flags: libc::c_int,
) {
    let mut folder_list: *mut clist = 0 as *mut clist;
    let mut iter: *mut clistiter = 0 as *mut clistiter;
    let mut mvbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut sentbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fallback_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(imap.is_null() || (*imap).etpan.is_null()) {
        dc_log_info(
            context,
            0i32,
            b"Configuring IMAP-folders.\x00" as *const u8 as *const libc::c_char,
        );
        folder_list = list_folders(imap);
        fallback_folder = dc_mprintf(
            b"INBOX%c%s\x00" as *const u8 as *const libc::c_char,
            (*imap).imap_delimiter as libc::c_int,
            b"DeltaChat\x00" as *const u8 as *const libc::c_char,
        );
        iter = (*folder_list).first;
        while !iter.is_null() {
            let mut folder: *mut dc_imapfolder_t = (if !iter.is_null() {
                (*iter).data
            } else {
                0 as *mut libc::c_void
            }) as *mut dc_imapfolder_t;
            if strcmp(
                (*folder).name_utf8,
                b"DeltaChat\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp((*folder).name_utf8, fallback_folder) == 0i32
            {
                if mvbox_folder.is_null() {
                    mvbox_folder = dc_strdup((*folder).name_to_select)
                }
            }
            if (*folder).meaning == 1i32 {
                if sentbox_folder.is_null() {
                    sentbox_folder = dc_strdup((*folder).name_to_select)
                }
            }
            iter = if !iter.is_null() {
                (*iter).next
            } else {
                0 as *mut clistcell_s
            }
        }
        if mvbox_folder.is_null() && 0 != flags & 0x1i32 {
            dc_log_info(
                context,
                0i32,
                b"Creating MVBOX-folder \"%s\"...\x00" as *const u8 as *const libc::c_char,
                b"DeltaChat\x00" as *const u8 as *const libc::c_char,
            );
            let mut r: libc::c_int = mailimap_create(
                (*imap).etpan,
                b"DeltaChat\x00" as *const u8 as *const libc::c_char,
            );
            if 0 != dc_imap_is_error(imap, r) {
                dc_log_warning(
                    context,
                    0i32,
                    b"Cannot create MVBOX-folder, using trying INBOX subfolder.\x00" as *const u8
                        as *const libc::c_char,
                );
                r = mailimap_create((*imap).etpan, fallback_folder);
                if 0 != dc_imap_is_error(imap, r) {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Cannot create MVBOX-folder.\x00" as *const u8 as *const libc::c_char,
                    );
                } else {
                    mvbox_folder = dc_strdup(fallback_folder);
                    dc_log_info(
                        context,
                        0i32,
                        b"MVBOX-folder created as INBOX subfolder.\x00" as *const u8
                            as *const libc::c_char,
                    );
                }
            } else {
                mvbox_folder = dc_strdup(b"DeltaChat\x00" as *const u8 as *const libc::c_char);
                dc_log_info(
                    context,
                    0i32,
                    b"MVBOX-folder created.\x00" as *const u8 as *const libc::c_char,
                );
            }
            mailimap_subscribe((*imap).etpan, mvbox_folder);
        }
        dc_sqlite3_set_config_int(
            (*context).sql,
            b"folders_configured\x00" as *const u8 as *const libc::c_char,
            3i32,
        );
        dc_sqlite3_set_config(
            (*context).sql,
            b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
            mvbox_folder,
        );
        dc_sqlite3_set_config(
            (*context).sql,
            b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
            sentbox_folder,
        );
    }
    free_folders(folder_list);
    free(mvbox_folder as *mut libc::c_void);
    free(fallback_folder as *mut libc::c_void);
}
unsafe extern "C" fn free_folders(mut folders: *mut clist) {
    if !folders.is_null() {
        let mut iter1: *mut clistiter = 0 as *mut clistiter;
        iter1 = (*folders).first;
        while !iter1.is_null() {
            let mut ret_folder: *mut dc_imapfolder_t = (if !iter1.is_null() {
                (*iter1).data
            } else {
                0 as *mut libc::c_void
            }) as *mut dc_imapfolder_t;
            free((*ret_folder).name_to_select as *mut libc::c_void);
            free((*ret_folder).name_utf8 as *mut libc::c_void);
            free(ret_folder as *mut libc::c_void);
            iter1 = if !iter1.is_null() {
                (*iter1).next
            } else {
                0 as *mut clistcell_s
            }
        }
        clist_free(folders);
    };
}
unsafe extern "C" fn list_folders(mut imap: *mut dc_imap_t) -> *mut clist {
    let mut imap_list: *mut clist = 0 as *mut clist;
    let mut iter1: *mut clistiter = 0 as *mut clistiter;
    let mut ret_list: *mut clist = clist_new();
    let mut r: libc::c_int = 0i32;
    let mut xlist_works: libc::c_int = 0i32;
    if !(imap.is_null() || (*imap).etpan.is_null()) {
        if 0 != (*imap).has_xlist {
            r = mailimap_xlist(
                (*imap).etpan,
                b"\x00" as *const u8 as *const libc::c_char,
                b"*\x00" as *const u8 as *const libc::c_char,
                &mut imap_list,
            )
        } else {
            r = mailimap_list(
                (*imap).etpan,
                b"\x00" as *const u8 as *const libc::c_char,
                b"*\x00" as *const u8 as *const libc::c_char,
                &mut imap_list,
            )
        }
        if 0 != dc_imap_is_error(imap, r) || imap_list.is_null() {
            imap_list = 0 as *mut clist;
            dc_log_warning(
                (*imap).context,
                0i32,
                b"Cannot get folder list.\x00" as *const u8 as *const libc::c_char,
            );
        } else if (*imap_list).count <= 0i32 {
            dc_log_warning(
                (*imap).context,
                0i32,
                b"Folder list is empty.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            (*imap).imap_delimiter = '.' as i32 as libc::c_char;
            iter1 = (*imap_list).first;
            while !iter1.is_null() {
                let mut imap_folder: *mut mailimap_mailbox_list = (if !iter1.is_null() {
                    (*iter1).data
                } else {
                    0 as *mut libc::c_void
                })
                    as *mut mailimap_mailbox_list;
                if 0 != (*imap_folder).mb_delimiter {
                    (*imap).imap_delimiter = (*imap_folder).mb_delimiter
                }
                let mut ret_folder: *mut dc_imapfolder_t = calloc(
                    1i32 as libc::c_ulong,
                    ::std::mem::size_of::<dc_imapfolder_t>() as libc::c_ulong,
                )
                    as *mut dc_imapfolder_t;
                if strcasecmp(
                    (*imap_folder).mb_name,
                    b"INBOX\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    (*ret_folder).name_to_select =
                        dc_strdup(b"INBOX\x00" as *const u8 as *const libc::c_char)
                } else {
                    (*ret_folder).name_to_select = dc_strdup((*imap_folder).mb_name)
                }
                (*ret_folder).name_utf8 = dc_decode_modified_utf7((*imap_folder).mb_name, 0i32);
                (*ret_folder).meaning = get_folder_meaning((*imap_folder).mb_flag);
                if (*ret_folder).meaning == 2i32 || (*ret_folder).meaning == 1i32 {
                    xlist_works = 1i32
                }
                clist_insert_after(ret_list, (*ret_list).last, ret_folder as *mut libc::c_void);
                iter1 = if !iter1.is_null() {
                    (*iter1).next
                } else {
                    0 as *mut clistcell_s
                }
            }
            if 0 == xlist_works {
                iter1 = (*ret_list).first;
                while !iter1.is_null() {
                    let mut ret_folder_0: *mut dc_imapfolder_t = (if !iter1.is_null() {
                        (*iter1).data
                    } else {
                        0 as *mut libc::c_void
                    })
                        as *mut dc_imapfolder_t;
                    (*ret_folder_0).meaning = get_folder_meaning_by_name((*ret_folder_0).name_utf8);
                    iter1 = if !iter1.is_null() {
                        (*iter1).next
                    } else {
                        0 as *mut clistcell_s
                    }
                }
            }
        }
    }
    if !imap_list.is_null() {
        mailimap_list_result_free(imap_list);
    }
    return ret_list;
}
unsafe extern "C" fn get_folder_meaning_by_name(
    mut folder_name: *const libc::c_char,
) -> libc::c_int {
    // try to get the folder meaning by the name of the folder.
    // only used if the server does not support XLIST.
    let mut ret_meaning: libc::c_int = 0i32;
    // TODO: lots languages missing - maybe there is a list somewhere on other MUAs?
    // however, if we fail to find out the sent-folder,
    // only watching this folder is not working. at least, this is no show stopper.
    // CAVE: if possible, take care not to add a name here that is "sent" in one language
    // but sth. different in others - a hard job.
    static mut sent_names: *const libc::c_char =
        b",sent,sent objects,gesendet,\x00" as *const u8 as *const libc::c_char;
    let mut lower: *mut libc::c_char =
        dc_mprintf(b",%s,\x00" as *const u8 as *const libc::c_char, folder_name);
    dc_strlower_in_place(lower);
    if !strstr(sent_names, lower).is_null() {
        ret_meaning = 1i32
    }
    free(lower as *mut libc::c_void);
    return ret_meaning;
}
unsafe extern "C" fn get_folder_meaning(mut flags: *mut mailimap_mbx_list_flags) -> libc::c_int {
    let mut ret_meaning: libc::c_int = 0i32;
    if !flags.is_null() {
        let mut iter2: *mut clistiter = 0 as *mut clistiter;
        iter2 = (*(*flags).mbf_oflags).first;
        while !iter2.is_null() {
            let mut oflag: *mut mailimap_mbx_list_oflag = (if !iter2.is_null() {
                (*iter2).data
            } else {
                0 as *mut libc::c_void
            })
                as *mut mailimap_mbx_list_oflag;
            match (*oflag).of_type {
                2 => {
                    if strcasecmp(
                        (*oflag).of_flag_ext,
                        b"spam\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                        || strcasecmp(
                            (*oflag).of_flag_ext,
                            b"trash\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        || strcasecmp(
                            (*oflag).of_flag_ext,
                            b"drafts\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        || strcasecmp(
                            (*oflag).of_flag_ext,
                            b"junk\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                    {
                        ret_meaning = 2i32
                    } else if strcasecmp(
                        (*oflag).of_flag_ext,
                        b"sent\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                    {
                        ret_meaning = 1i32
                    }
                }
                _ => {}
            }
            iter2 = if !iter2.is_null() {
                (*iter2).next
            } else {
                0 as *mut clistcell_s
            }
        }
    }
    return ret_meaning;
}
unsafe extern "C" fn moz_autoconfigure(
    mut context: *mut dc_context_t,
    mut url: *const libc::c_char,
    mut param_in: *const dc_loginparam_t,
) -> *mut dc_loginparam_t {
    let mut p: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut saxparser: dc_saxparser_t = _dc_saxparser {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };
    let mut xml_raw: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut moz_ac: moz_autoconfigure_t = moz_autoconfigure_t {
        in_0: 0 as *const dc_loginparam_t,
        in_emaildomain: 0 as *mut libc::c_char,
        in_emaillocalpart: 0 as *mut libc::c_char,
        out: 0 as *mut dc_loginparam_t,
        out_imap_set: 0,
        out_smtp_set: 0,
        tag_server: 0,
        tag_config: 0,
    };
    memset(
        &mut moz_ac as *mut moz_autoconfigure_t as *mut libc::c_void,
        0i32,
        ::std::mem::size_of::<moz_autoconfigure_t>() as libc::c_ulong,
    );
    xml_raw = read_autoconf_file(context, url);
    if !xml_raw.is_null() {
        moz_ac.in_0 = param_in;
        moz_ac.in_emaillocalpart = dc_strdup((*param_in).addr);
        p = strchr(moz_ac.in_emaillocalpart, '@' as i32);
        if !p.is_null() {
            *p = 0i32 as libc::c_char;
            moz_ac.in_emaildomain = dc_strdup(p.offset(1isize));
            moz_ac.out = dc_loginparam_new();
            saxparser = _dc_saxparser {
                starttag_cb: None,
                endtag_cb: None,
                text_cb: None,
                userdata: 0 as *mut libc::c_void,
            };
            dc_saxparser_init(
                &mut saxparser,
                &mut moz_ac as *mut moz_autoconfigure_t as *mut libc::c_void,
            );
            dc_saxparser_set_tag_handler(
                &mut saxparser,
                Some(moz_autoconfigure_starttag_cb),
                Some(moz_autoconfigure_endtag_cb),
            );
            dc_saxparser_set_text_handler(&mut saxparser, Some(moz_autoconfigure_text_cb));
            dc_saxparser_parse(&mut saxparser, xml_raw);
            if (*moz_ac.out).mail_server.is_null()
                || (*moz_ac.out).mail_port as libc::c_int == 0i32
                || (*moz_ac.out).send_server.is_null()
                || (*moz_ac.out).send_port == 0i32
            {
                let mut r: *mut libc::c_char = dc_loginparam_get_readable(moz_ac.out);
                dc_log_warning(
                    context,
                    0i32,
                    b"Bad or incomplete autoconfig: %s\x00" as *const u8 as *const libc::c_char,
                    r,
                );
                free(r as *mut libc::c_void);
                dc_loginparam_unref(moz_ac.out);
                moz_ac.out = 0 as *mut dc_loginparam_t
            }
        }
    }
    free(xml_raw as *mut libc::c_void);
    free(moz_ac.in_emaildomain as *mut libc::c_void);
    free(moz_ac.in_emaillocalpart as *mut libc::c_void);
    return moz_ac.out;
}
unsafe extern "C" fn moz_autoconfigure_text_cb(
    mut userdata: *mut libc::c_void,
    mut text: *const libc::c_char,
    mut len: libc::c_int,
) {
    let mut moz_ac: *mut moz_autoconfigure_t = userdata as *mut moz_autoconfigure_t;
    let mut val: *mut libc::c_char = dc_strdup(text);
    dc_trim(val);
    dc_str_replace(
        &mut val,
        b"%EMAILADDRESS%\x00" as *const u8 as *const libc::c_char,
        (*(*moz_ac).in_0).addr,
    );
    dc_str_replace(
        &mut val,
        b"%EMAILLOCALPART%\x00" as *const u8 as *const libc::c_char,
        (*moz_ac).in_emaillocalpart,
    );
    dc_str_replace(
        &mut val,
        b"%EMAILDOMAIN%\x00" as *const u8 as *const libc::c_char,
        (*moz_ac).in_emaildomain,
    );
    if (*moz_ac).tag_server == 1i32 {
        match (*moz_ac).tag_config {
            10 => {
                free((*(*moz_ac).out).mail_server as *mut libc::c_void);
                (*(*moz_ac).out).mail_server = val;
                val = 0 as *mut libc::c_char
            }
            11 => (*(*moz_ac).out).mail_port = atoi(val) as uint16_t,
            12 => {
                free((*(*moz_ac).out).mail_user as *mut libc::c_void);
                (*(*moz_ac).out).mail_user = val;
                val = 0 as *mut libc::c_char
            }
            13 => {
                if strcasecmp(val, b"ssl\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x200i32
                }
                if strcasecmp(val, b"starttls\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x100i32
                }
                if strcasecmp(val, b"plain\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x400i32
                }
            }
            _ => {}
        }
    } else if (*moz_ac).tag_server == 2i32 {
        match (*moz_ac).tag_config {
            10 => {
                free((*(*moz_ac).out).send_server as *mut libc::c_void);
                (*(*moz_ac).out).send_server = val;
                val = 0 as *mut libc::c_char
            }
            11 => (*(*moz_ac).out).send_port = atoi(val),
            12 => {
                free((*(*moz_ac).out).send_user as *mut libc::c_void);
                (*(*moz_ac).out).send_user = val;
                val = 0 as *mut libc::c_char
            }
            13 => {
                if strcasecmp(val, b"ssl\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x20000i32
                }
                if strcasecmp(val, b"starttls\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x10000i32
                }
                if strcasecmp(val, b"plain\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x40000i32
                }
            }
            _ => {}
        }
    }
    free(val as *mut libc::c_void);
}
unsafe extern "C" fn moz_autoconfigure_endtag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
) {
    let mut moz_ac: *mut moz_autoconfigure_t = userdata as *mut moz_autoconfigure_t;
    if strcmp(
        tag,
        b"incomingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        (*moz_ac).tag_server = 0i32;
        (*moz_ac).tag_config = 0i32;
        (*moz_ac).out_imap_set = 1i32
    } else if strcmp(
        tag,
        b"outgoingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        (*moz_ac).tag_server = 0i32;
        (*moz_ac).tag_config = 0i32;
        (*moz_ac).out_smtp_set = 1i32
    } else {
        (*moz_ac).tag_config = 0i32
    };
}
unsafe extern "C" fn moz_autoconfigure_starttag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
    mut attr: *mut *mut libc::c_char,
) {
    let mut moz_ac: *mut moz_autoconfigure_t = userdata as *mut moz_autoconfigure_t;
    let mut p1: *const libc::c_char = 0 as *const libc::c_char;
    if strcmp(
        tag,
        b"incomingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        (*moz_ac).tag_server = if (*moz_ac).out_imap_set == 0i32
            && {
                p1 = dc_attr_find(attr, b"type\x00" as *const u8 as *const libc::c_char);
                !p1.is_null()
            }
            && strcasecmp(p1, b"imap\x00" as *const u8 as *const libc::c_char) == 0i32
        {
            1i32
        } else {
            0i32
        };
        (*moz_ac).tag_config = 0i32
    } else if strcmp(
        tag,
        b"outgoingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        (*moz_ac).tag_server = if (*moz_ac).out_smtp_set == 0i32 {
            2i32
        } else {
            0i32
        };
        (*moz_ac).tag_config = 0i32
    } else if strcmp(tag, b"hostname\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 10i32
    } else if strcmp(tag, b"port\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 11i32
    } else if strcmp(tag, b"sockettype\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 13i32
    } else if strcmp(tag, b"username\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 12i32
    };
}
unsafe extern "C" fn read_autoconf_file(
    mut context: *mut dc_context_t,
    mut url: *const libc::c_char,
) -> *mut libc::c_char {
    let mut filecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_log_info(
        context,
        0i32,
        b"Testing %s ...\x00" as *const u8 as *const libc::c_char,
        url,
    );
    filecontent = (*context).cb.expect("non-null function pointer")(
        context,
        2100i32,
        url as uintptr_t,
        0i32 as uintptr_t,
    ) as *mut libc::c_char;
    if filecontent.is_null() || *filecontent.offset(0isize) as libc::c_int == 0i32 {
        free(filecontent as *mut libc::c_void);
        dc_log_info(
            context,
            0i32,
            b"Can\'t read file.\x00" as *const u8 as *const libc::c_char,
        );
        return 0 as *mut libc::c_char;
    }
    return filecontent;
}
unsafe extern "C" fn outlk_autodiscover(
    mut context: *mut dc_context_t,
    mut url__: *const libc::c_char,
    mut param_in: *const dc_loginparam_t,
) -> *mut dc_loginparam_t {
    let mut current_block: u64;
    let mut xml_raw: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut url: *mut libc::c_char = dc_strdup(url__);
    let mut outlk_ad: outlk_autodiscover_t = outlk_autodiscover_t {
        in_0: 0 as *const dc_loginparam_t,
        out: 0 as *mut dc_loginparam_t,
        out_imap_set: 0,
        out_smtp_set: 0,
        tag_config: 0,
        config: [0 as *mut libc::c_char; 6],
        redirect: 0 as *mut libc::c_char,
    };
    let mut i: libc::c_int = 0;
    i = 0i32;
    loop {
        if !(i < 10i32) {
            current_block = 11584701595673473500;
            break;
        }
        memset(
            &mut outlk_ad as *mut outlk_autodiscover_t as *mut libc::c_void,
            0i32,
            ::std::mem::size_of::<outlk_autodiscover_t>() as libc::c_ulong,
        );
        xml_raw = read_autoconf_file(context, url);
        if xml_raw.is_null() {
            current_block = 3070887585260837332;
            break;
        }
        outlk_ad.in_0 = param_in;
        outlk_ad.out = dc_loginparam_new();
        let mut saxparser: dc_saxparser_t = _dc_saxparser {
            starttag_cb: None,
            endtag_cb: None,
            text_cb: None,
            userdata: 0 as *mut libc::c_void,
        };
        dc_saxparser_init(
            &mut saxparser,
            &mut outlk_ad as *mut outlk_autodiscover_t as *mut libc::c_void,
        );
        dc_saxparser_set_tag_handler(
            &mut saxparser,
            Some(outlk_autodiscover_starttag_cb),
            Some(outlk_autodiscover_endtag_cb),
        );
        dc_saxparser_set_text_handler(&mut saxparser, Some(outlk_autodiscover_text_cb));
        dc_saxparser_parse(&mut saxparser, xml_raw);
        if !(!outlk_ad.config[5usize].is_null()
            && 0 != *outlk_ad.config[5usize].offset(0isize) as libc::c_int)
        {
            current_block = 11584701595673473500;
            break;
        }
        free(url as *mut libc::c_void);
        url = dc_strdup(outlk_ad.config[5usize]);
        dc_loginparam_unref(outlk_ad.out);
        outlk_clean_config(&mut outlk_ad);
        free(xml_raw as *mut libc::c_void);
        xml_raw = 0 as *mut libc::c_char;
        i += 1
    }
    match current_block {
        11584701595673473500 => {
            if (*outlk_ad.out).mail_server.is_null()
                || (*outlk_ad.out).mail_port as libc::c_int == 0i32
                || (*outlk_ad.out).send_server.is_null()
                || (*outlk_ad.out).send_port == 0i32
            {
                let mut r: *mut libc::c_char = dc_loginparam_get_readable(outlk_ad.out);
                dc_log_warning(
                    context,
                    0i32,
                    b"Bad or incomplete autoconfig: %s\x00" as *const u8 as *const libc::c_char,
                    r,
                );
                free(r as *mut libc::c_void);
                dc_loginparam_unref(outlk_ad.out);
                outlk_ad.out = 0 as *mut dc_loginparam_t
            }
        }
        _ => {}
    }
    free(url as *mut libc::c_void);
    free(xml_raw as *mut libc::c_void);
    outlk_clean_config(&mut outlk_ad);
    return outlk_ad.out;
}
unsafe extern "C" fn outlk_clean_config(mut outlk_ad: *mut outlk_autodiscover_t) {
    let mut i: libc::c_int = 0;
    i = 0i32;
    while i < 6i32 {
        free((*outlk_ad).config[i as usize] as *mut libc::c_void);
        (*outlk_ad).config[i as usize] = 0 as *mut libc::c_char;
        i += 1
    }
}
unsafe extern "C" fn outlk_autodiscover_text_cb(
    mut userdata: *mut libc::c_void,
    mut text: *const libc::c_char,
    mut len: libc::c_int,
) {
    let mut outlk_ad: *mut outlk_autodiscover_t = userdata as *mut outlk_autodiscover_t;
    let mut val: *mut libc::c_char = dc_strdup(text);
    dc_trim(val);
    free((*outlk_ad).config[(*outlk_ad).tag_config as usize] as *mut libc::c_void);
    (*outlk_ad).config[(*outlk_ad).tag_config as usize] = val;
}
unsafe extern "C" fn outlk_autodiscover_endtag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
) {
    let mut outlk_ad: *mut outlk_autodiscover_t = userdata as *mut outlk_autodiscover_t;
    if strcmp(tag, b"protocol\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !(*outlk_ad).config[1usize].is_null() {
            let mut port: libc::c_int = dc_atoi_null_is_0((*outlk_ad).config[3usize]);
            let mut ssl_on: libc::c_int = (!(*outlk_ad).config[4usize].is_null()
                && strcasecmp(
                    (*outlk_ad).config[4usize],
                    b"on\x00" as *const u8 as *const libc::c_char,
                ) == 0i32) as libc::c_int;
            let mut ssl_off: libc::c_int = (!(*outlk_ad).config[4usize].is_null()
                && strcasecmp(
                    (*outlk_ad).config[4usize],
                    b"off\x00" as *const u8 as *const libc::c_char,
                ) == 0i32) as libc::c_int;
            if strcasecmp(
                (*outlk_ad).config[1usize],
                b"imap\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                && (*outlk_ad).out_imap_set == 0i32
            {
                (*(*outlk_ad).out).mail_server = dc_strdup_keep_null((*outlk_ad).config[2usize]);
                (*(*outlk_ad).out).mail_port = port as uint16_t;
                if 0 != ssl_on {
                    (*(*outlk_ad).out).server_flags |= 0x200i32
                } else if 0 != ssl_off {
                    (*(*outlk_ad).out).server_flags |= 0x400i32
                }
                (*outlk_ad).out_imap_set = 1i32
            } else if strcasecmp(
                (*outlk_ad).config[1usize],
                b"smtp\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                && (*outlk_ad).out_smtp_set == 0i32
            {
                (*(*outlk_ad).out).send_server = dc_strdup_keep_null((*outlk_ad).config[2usize]);
                (*(*outlk_ad).out).send_port = port;
                if 0 != ssl_on {
                    (*(*outlk_ad).out).server_flags |= 0x20000i32
                } else if 0 != ssl_off {
                    (*(*outlk_ad).out).server_flags |= 0x40000i32
                }
                (*outlk_ad).out_smtp_set = 1i32
            }
        }
        outlk_clean_config(outlk_ad);
    }
    (*outlk_ad).tag_config = 0i32;
}
unsafe extern "C" fn outlk_autodiscover_starttag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
    mut attr: *mut *mut libc::c_char,
) {
    let mut outlk_ad: *mut outlk_autodiscover_t = userdata as *mut outlk_autodiscover_t;
    if strcmp(tag, b"protocol\x00" as *const u8 as *const libc::c_char) == 0i32 {
        outlk_clean_config(outlk_ad);
    } else if strcmp(tag, b"type\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 1i32
    } else if strcmp(tag, b"server\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 2i32
    } else if strcmp(tag, b"port\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 3i32
    } else if strcmp(tag, b"ssl\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 4i32
    } else if strcmp(tag, b"redirecturl\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 5i32
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_alloc_ongoing(mut context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0i32;
    }
    if 0 != dc_has_ongoing(context) {
        dc_log_warning(
            context,
            0i32,
            b"There is already another ongoing process running.\x00" as *const u8
                as *const libc::c_char,
        );
        return 0i32;
    }
    (*context).ongoing_running = 1i32;
    (*context).shall_stop_ongoing = 0i32;
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_connect_to_configured_imap(
    mut context: *mut dc_context_t,
    mut imap: *mut dc_imap_t,
) -> libc::c_int {
    let mut ret_connected: libc::c_int = 0i32;
    let mut param: *mut dc_loginparam_t = dc_loginparam_new();
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || imap.is_null() {
        dc_log_warning(
            (*imap).context,
            0i32,
            b"Cannot connect to IMAP: Bad parameters.\x00" as *const u8 as *const libc::c_char,
        );
    } else if 0 != dc_imap_is_connected(imap) {
        ret_connected = 1i32
    } else if dc_sqlite3_get_config_int(
        (*(*imap).context).sql,
        b"configured\x00" as *const u8 as *const libc::c_char,
        0i32,
    ) == 0i32
    {
        dc_log_warning(
            (*imap).context,
            0i32,
            b"Not configured, cannot connect.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        dc_loginparam_read(
            param,
            (*(*imap).context).sql,
            b"configured_\x00" as *const u8 as *const libc::c_char,
        );
        /*the trailing underscore is correct*/
        if !(0 == dc_imap_connect(imap, param)) {
            ret_connected = 2i32
        }
    }
    dc_loginparam_unref(param);
    return ret_connected;
}

use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type sqlite3_stmt;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn strcat(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_get_config(_: *mut dc_context_t, key: *const libc::c_char) -> *mut libc::c_char;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_text(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_char,
        _: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    /* Return the string with the given ID by calling DC_EVENT_GET_STRING.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str(_: *mut dc_context_t, id: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_sqlite3_get_rowid(
        _: *mut dc_sqlite3_t,
        table: *const libc::c_char,
        field: *const libc::c_char,
        value: *const libc::c_char,
    ) -> uint32_t;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_free_splitted_lines(lines: *mut carray);
    #[no_mangle]
    fn dc_sqlite3_commit(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_begin_transaction(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_split_into_lines(buf_terminated: *const libc::c_char) -> *mut carray;
    #[no_mangle]
    fn dc_array_new(_: *mut dc_context_t, initsize: size_t) -> *mut dc_array_t;
    #[no_mangle]
    fn sqlite3_free(_: *mut libc::c_void);
    #[no_mangle]
    fn dc_array_add_id(_: *mut dc_array_t, _: uint32_t);
    #[no_mangle]
    fn dc_str_contains(haystack: *const libc::c_char, needle: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_mprintf(_: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_key_new() -> *mut dc_key_t;
    #[no_mangle]
    fn dc_key_unref(_: *mut dc_key_t);
    #[no_mangle]
    fn dc_loginparam_new() -> *mut dc_loginparam_t;
    #[no_mangle]
    fn dc_loginparam_unref(_: *mut dc_loginparam_t);
    #[no_mangle]
    fn dc_apeerstate_new(_: *mut dc_context_t) -> *mut dc_apeerstate_t;
    #[no_mangle]
    fn dc_apeerstate_unref(_: *mut dc_apeerstate_t);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_apeerstate_peek_key(
        _: *const dc_apeerstate_t,
        min_verified: libc::c_int,
    ) -> *mut dc_key_t;
    #[no_mangle]
    fn dc_key_get_formatted_fingerprint(_: *const dc_key_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_key_load_self_public(
        _: *mut dc_key_t,
        self_addr: *const libc::c_char,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_ensure_secret_key_exists(_: *mut dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_pgp_rand_seed(_: *mut dc_context_t, buf: *const libc::c_void, bytes: size_t);
    #[no_mangle]
    fn dc_loginparam_read(
        _: *mut dc_loginparam_t,
        _: *mut dc_sqlite3_t,
        prefix: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_apeerstate_load_by_addr(
        _: *mut dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        addr: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_rtrim(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_str_to_color(_: *const libc::c_char) -> libc::c_int;
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
pub type uintptr_t = libc::c_ulong;
pub type size_t = __darwin_size_t;
pub type uint8_t = libc::c_uchar;
pub type uint16_t = libc::c_ushort;
pub type uint32_t = libc::c_uint;
pub type ssize_t = __darwin_ssize_t;
pub type time_t = __darwin_time_t;
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
    pub ft_data: unnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_1 {
    pub ft_fetch_att: *mut mailimap_fetch_att,
    pub ft_fetch_att_list: *mut clist,
}
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
pub struct unnamed_2 {
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
use crate::dc_context::dc_context_t;
/* ** library-private **********************************************************/
pub type dc_smtp_t = _dc_smtp;
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
pub type dc_jobthread_t = _dc_jobthread;
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
pub type dc_imap_t = _dc_imap;
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
/* * the structure behind dc_array_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_array {
    pub magic: uint32_t,
    pub context: *mut dc_context_t,
    pub allocated: size_t,
    pub count: size_t,
    pub type_0: libc::c_int,
    pub array: *mut uintptr_t,
}
pub type dc_array_t = _dc_array;
/* * the structure behind dc_contact_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_contact {
    pub magic: uint32_t,
    pub context: *mut dc_context_t,
    pub id: uint32_t,
    pub name: *mut libc::c_char,
    pub authname: *mut libc::c_char,
    pub addr: *mut libc::c_char,
    pub blocked: libc::c_int,
    pub origin: libc::c_int,
}
pub type dc_contact_t = _dc_contact;
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
pub type dc_strbuilder_t = _dc_strbuilder;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type dc_key_t = _dc_key;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_key {
    pub binary: *mut libc::c_void,
    pub bytes: libc::c_int,
    pub type_0: libc::c_int,
    pub _m_heap_refcnt: libc::c_int,
}
pub type dc_loginparam_t = _dc_loginparam;
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
pub type dc_apeerstate_t = _dc_apeerstate;
/* prefer-encrypt states */
/* *
 * @class dc_apeerstate_t
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_apeerstate {
    pub context: *mut dc_context_t,
    pub addr: *mut libc::c_char,
    pub last_seen: time_t,
    pub last_seen_autocrypt: time_t,
    pub prefer_encrypt: libc::c_int,
    pub public_key: *mut dc_key_t,
    pub public_key_fingerprint: *mut libc::c_char,
    pub gossip_key: *mut dc_key_t,
    pub gossip_timestamp: time_t,
    pub gossip_key_fingerprint: *mut libc::c_char,
    pub verified_key: *mut dc_key_t,
    pub verified_key_fingerprint: *mut libc::c_char,
    pub to_save: libc::c_int,
    pub degrade_event: libc::c_int,
}
#[inline]
unsafe extern "C" fn carray_count(mut array: *mut carray) -> libc::c_uint {
    return (*array).len;
}
#[inline]
unsafe extern "C" fn carray_get(
    mut array: *mut carray,
    mut indx: libc::c_uint,
) -> *mut libc::c_void {
    return *(*array).array.offset(indx as isize);
}
#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_contact(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE msgs SET state=13 WHERE from_id=? AND state=10;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    (*context).cb.expect("non-null function pointer")(
        context,
        2000i32,
        0i32 as uintptr_t,
        0i32 as uintptr_t,
    );
}
// handle contacts
#[no_mangle]
pub unsafe extern "C" fn dc_may_be_valid_addr(mut addr: *const libc::c_char) -> libc::c_int {
    if addr.is_null() {
        return 0i32;
    }
    let mut at: *const libc::c_char = strchr(addr, '@' as i32);
    if at.is_null() || (at.wrapping_offset_from(addr) as libc::c_long) < 1i32 as libc::c_long {
        return 0i32;
    }
    let mut dot: *const libc::c_char = strchr(at, '.' as i32);
    if dot.is_null()
        || (dot.wrapping_offset_from(at) as libc::c_long) < 2i32 as libc::c_long
        || *dot.offset(1isize) as libc::c_int == 0i32
        || *dot.offset(2isize) as libc::c_int == 0i32
    {
        return 0i32;
    }
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_lookup_contact_id_by_addr(
    mut context: *mut dc_context_t,
    mut addr: *const libc::c_char,
) -> uint32_t {
    let mut contact_id: libc::c_int = 0i32;
    let mut addr_normalized: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut addr_self: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || addr.is_null()
        || *addr.offset(0isize) as libc::c_int == 0i32)
    {
        addr_normalized = dc_addr_normalize(addr);
        addr_self = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if strcasecmp(addr_normalized, addr_self) == 0i32 {
            contact_id = 1i32
        } else {
            stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT id FROM contacts WHERE addr=?1 COLLATE NOCASE AND id>?2 AND origin>=?3 AND blocked=0;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_text(
                stmt,
                1i32,
                addr_normalized as *const libc::c_char,
                -1i32,
                None,
            );
            sqlite3_bind_int(stmt, 2i32, 9i32);
            sqlite3_bind_int(stmt, 3i32, 0x100i32);
            if sqlite3_step(stmt) == 100i32 {
                contact_id = sqlite3_column_int(stmt, 0i32)
            }
        }
    }
    sqlite3_finalize(stmt);
    free(addr_normalized as *mut libc::c_void);
    free(addr_self as *mut libc::c_void);
    return contact_id as uint32_t;
}
#[no_mangle]
pub unsafe extern "C" fn dc_addr_normalize(mut addr: *const libc::c_char) -> *mut libc::c_char {
    let mut addr_normalized: *mut libc::c_char = dc_strdup(addr);
    dc_trim(addr_normalized);
    if strncmp(
        addr_normalized,
        b"mailto:\x00" as *const u8 as *const libc::c_char,
        7i32 as libc::c_ulong,
    ) == 0i32
    {
        let mut old: *mut libc::c_char = addr_normalized;
        addr_normalized = dc_strdup(&mut *old.offset(7isize));
        free(old as *mut libc::c_void);
        dc_trim(addr_normalized);
    }
    return addr_normalized;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_contact(
    mut context: *mut dc_context_t,
    mut name: *const libc::c_char,
    mut addr: *const libc::c_char,
) -> uint32_t {
    let mut contact_id: uint32_t = 0i32 as uint32_t;
    let mut sth_modified: libc::c_int = 0i32;
    let mut blocked: libc::c_int = 0i32;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || addr.is_null()
        || *addr.offset(0isize) as libc::c_int == 0i32)
    {
        contact_id = dc_add_or_lookup_contact(context, name, addr, 0x4000000i32, &mut sth_modified);
        blocked = dc_is_contact_blocked(context, contact_id);
        (*context).cb.expect("non-null function pointer")(
            context,
            2030i32,
            (if sth_modified == 2i32 {
                contact_id
            } else {
                0i32 as libc::c_uint
            }) as uintptr_t,
            0i32 as uintptr_t,
        );
        if 0 != blocked {
            dc_block_contact(context, contact_id, 0i32);
        }
    }
    return contact_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_block_contact(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
    mut new_blocking: libc::c_int,
) {
    let mut current_block: u64;
    let mut send_event: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = dc_contact_new(context);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || contact_id <= 9i32 as libc::c_uint)
    {
        if 0 != dc_contact_load_from_db(contact, (*context).sql, contact_id)
            && (*contact).blocked != new_blocking
        {
            stmt = dc_sqlite3_prepare(
                (*context).sql,
                b"UPDATE contacts SET blocked=? WHERE id=?;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_int(stmt, 1i32, new_blocking);
            sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
            if sqlite3_step(stmt) != 101i32 {
                current_block = 5249903830285462583;
            } else {
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"UPDATE chats SET blocked=? WHERE type=? AND id IN (SELECT chat_id FROM chats_contacts WHERE contact_id=?);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, new_blocking);
                sqlite3_bind_int(stmt, 2i32, 100i32);
                sqlite3_bind_int(stmt, 3i32, contact_id as libc::c_int);
                if sqlite3_step(stmt) != 101i32 {
                    current_block = 5249903830285462583;
                } else {
                    dc_marknoticed_contact(context, contact_id);
                    send_event = 1i32;
                    current_block = 15652330335145281839;
                }
            }
        } else {
            current_block = 15652330335145281839;
        }
        match current_block {
            5249903830285462583 => {}
            _ => {
                if 0 != send_event {
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2030i32,
                        0i32 as uintptr_t,
                        0i32 as uintptr_t,
                    );
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    dc_contact_unref(contact);
}
/* *
 * @class dc_contact_t
 *
 * An object representing a single contact in memory.
 * The contact object is not updated.
 * If you want an update, you have to recreate the object.
 *
 * The library makes sure
 * only to use names _authorized_ by the contact in `To:` or `Cc:`.
 * _Given-names _as "Daddy" or "Honey" are not used there.
 * For this purpose, internally, two names are tracked -
 * authorized-name and given-name.
 * By default, these names are equal,
 * but functions working with contact names
 * (eg. dc_contact_get_name(), dc_contact_get_display_name(),
 * dc_contact_get_name_n_addr(), dc_contact_get_first_name(),
 * dc_create_contact() or dc_add_address_book())
 * only affect the given-name.
 */
#[no_mangle]
pub unsafe extern "C" fn dc_contact_new(mut context: *mut dc_context_t) -> *mut dc_contact_t {
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    contact = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_contact_t>() as libc::c_ulong,
    ) as *mut dc_contact_t;
    if contact.is_null() {
        exit(19i32);
    }
    (*contact).magic = 0xc047ac7i32 as uint32_t;
    (*contact).context = context;
    return contact;
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_unref(mut contact: *mut dc_contact_t) {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return;
    }
    dc_contact_empty(contact);
    (*contact).magic = 0i32 as uint32_t;
    free(contact as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_empty(mut contact: *mut dc_contact_t) {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return;
    }
    (*contact).id = 0i32 as uint32_t;
    free((*contact).name as *mut libc::c_void);
    (*contact).name = 0 as *mut libc::c_char;
    free((*contact).authname as *mut libc::c_void);
    (*contact).authname = 0 as *mut libc::c_char;
    free((*contact).addr as *mut libc::c_void);
    (*contact).addr = 0 as *mut libc::c_char;
    (*contact).origin = 0i32;
    (*contact).blocked = 0i32;
}
/* From: of incoming messages of unknown sender */
/* Cc: of incoming messages of unknown sender */
/* To: of incoming messages of unknown sender */
/* address scanned but not verified */
/* Reply-To: of incoming message of known sender */
/* Cc: of incoming message of known sender */
/* additional To:'s of incoming message of known sender */
/* a chat was manually created for this user, but no message yet sent */
/* message sent by us */
/* message sent by us */
/* message sent by us */
/* internal use */
/* address is in our address book */
/* set on Alice's side for contacts like Bob that have scanned the QR code offered by her. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling dc_contact_is_verified() ! */
/* set on Bob's side for contacts scanned and verified from a QR code. Only means the contact has once been established using the "securejoin" procedure in the past, getting the current key verification status requires calling dc_contact_is_verified() ! */
/* contact added mannually by dc_create_contact(), this should be the largets origin as otherwise the user cannot modify the names */
/* contacts with at least this origin value are shown in the contact list */
/* contacts with at least this origin value are verified and known not to be spam */
/* contacts with at least this origin value start a new "normal" chat, defaults to off */
#[no_mangle]
pub unsafe extern "C" fn dc_contact_load_from_db(
    mut contact: *mut dc_contact_t,
    mut sql: *mut dc_sqlite3_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint || sql.is_null()) {
        dc_contact_empty(contact);
        if contact_id == 1i32 as libc::c_uint {
            (*contact).id = contact_id;
            (*contact).name = dc_stock_str((*contact).context, 2i32);
            (*contact).addr = dc_sqlite3_get_config(
                sql,
                b"configured_addr\x00" as *const u8 as *const libc::c_char,
                b"\x00" as *const u8 as *const libc::c_char,
            );
            current_block = 5143058163439228106;
        } else {
            stmt =
                dc_sqlite3_prepare(sql,
                                   b"SELECT c.name, c.addr, c.origin, c.blocked, c.authname  FROM contacts c  WHERE c.id=?;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
            if sqlite3_step(stmt) != 100i32 {
                current_block = 12908855840294526070;
            } else {
                (*contact).id = contact_id;
                (*contact).name = dc_strdup(sqlite3_column_text(stmt, 0i32) as *mut libc::c_char);
                (*contact).addr = dc_strdup(sqlite3_column_text(stmt, 1i32) as *mut libc::c_char);
                (*contact).origin = sqlite3_column_int(stmt, 2i32);
                (*contact).blocked = sqlite3_column_int(stmt, 3i32);
                (*contact).authname =
                    dc_strdup(sqlite3_column_text(stmt, 4i32) as *mut libc::c_char);
                current_block = 5143058163439228106;
            }
        }
        match current_block {
            12908855840294526070 => {}
            _ => success = 1i32,
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_contact_blocked(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    let mut is_blocked: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = dc_contact_new(context);
    if 0 != dc_contact_load_from_db(contact, (*context).sql, contact_id) {
        if 0 != (*contact).blocked {
            is_blocked = 1i32
        }
    }
    dc_contact_unref(contact);
    return is_blocked;
}
/*can be NULL*/
#[no_mangle]
pub unsafe extern "C" fn dc_add_or_lookup_contact(
    mut context: *mut dc_context_t,
    mut name: *const libc::c_char,
    mut addr__: *const libc::c_char,
    mut origin: libc::c_int,
    mut sth_modified: *mut libc::c_int,
) -> uint32_t {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut row_id: uint32_t = 0i32 as uint32_t;
    let mut dummy: libc::c_int = 0i32;
    let mut addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut addr_self: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut row_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut row_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut row_authname: *mut libc::c_char = 0 as *mut libc::c_char;
    if sth_modified.is_null() {
        sth_modified = &mut dummy
    }
    *sth_modified = 0i32;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || addr__.is_null()
        || origin <= 0i32)
    {
        addr = dc_addr_normalize(addr__);
        addr_self = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if strcasecmp(addr, addr_self) == 0i32 {
            row_id = 1i32 as uint32_t
        } else if 0 == dc_may_be_valid_addr(addr) {
            dc_log_warning(
                context,
                0i32,
                b"Bad address \"%s\" for contact \"%s\".\x00" as *const u8 as *const libc::c_char,
                addr,
                if !name.is_null() {
                    name
                } else {
                    b"<unset>\x00" as *const u8 as *const libc::c_char
                },
            );
        } else {
            stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT id, name, addr, origin, authname FROM contacts WHERE addr=? COLLATE NOCASE;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_text(stmt, 1i32, addr as *const libc::c_char, -1i32, None);
            if sqlite3_step(stmt) == 100i32 {
                let mut row_origin: libc::c_int = 0;
                let mut update_addr: libc::c_int = 0i32;
                let mut update_name: libc::c_int = 0i32;
                let mut update_authname: libc::c_int = 0i32;
                row_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
                row_name = dc_strdup(sqlite3_column_text(stmt, 1i32) as *mut libc::c_char);
                row_addr = dc_strdup(sqlite3_column_text(stmt, 2i32) as *mut libc::c_char);
                row_origin = sqlite3_column_int(stmt, 3i32);
                row_authname = dc_strdup(sqlite3_column_text(stmt, 4i32) as *mut libc::c_char);
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                if !name.is_null() && 0 != *name.offset(0isize) as libc::c_int {
                    if 0 != *row_name.offset(0isize) {
                        if origin >= row_origin && strcmp(name, row_name) != 0i32 {
                            update_name = 1i32
                        }
                    } else {
                        update_name = 1i32
                    }
                    if origin == 0x10i32 && strcmp(name, row_authname) != 0i32 {
                        update_authname = 1i32
                    }
                }
                if origin >= row_origin && strcmp(addr, row_addr) != 0i32 {
                    update_addr = 1i32
                }
                if 0 != update_name
                    || 0 != update_authname
                    || 0 != update_addr
                    || origin > row_origin
                {
                    stmt = dc_sqlite3_prepare(
                        (*context).sql,
                        b"UPDATE contacts SET name=?, addr=?, origin=?, authname=? WHERE id=?;\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    sqlite3_bind_text(
                        stmt,
                        1i32,
                        if 0 != update_name { name } else { row_name },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_text(
                        stmt,
                        2i32,
                        if 0 != update_addr { addr } else { row_addr },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_int(
                        stmt,
                        3i32,
                        if origin > row_origin {
                            origin
                        } else {
                            row_origin
                        },
                    );
                    sqlite3_bind_text(
                        stmt,
                        4i32,
                        if 0 != update_authname {
                            name
                        } else {
                            row_authname
                        },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_int(stmt, 5i32, row_id as libc::c_int);
                    sqlite3_step(stmt);
                    sqlite3_finalize(stmt);
                    stmt = 0 as *mut sqlite3_stmt;
                    if 0 != update_name {
                        stmt =
                            dc_sqlite3_prepare((*context).sql,
                                               b"UPDATE chats SET name=? WHERE type=? AND id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                        sqlite3_bind_text(stmt, 1i32, name, -1i32, None);
                        sqlite3_bind_int(stmt, 2i32, 100i32);
                        sqlite3_bind_int(stmt, 3i32, row_id as libc::c_int);
                        sqlite3_step(stmt);
                    }
                    *sth_modified = 1i32
                }
            } else {
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"INSERT INTO contacts (name, addr, origin) VALUES(?, ?, ?);\x00" as *const u8
                        as *const libc::c_char,
                );
                sqlite3_bind_text(
                    stmt,
                    1i32,
                    if !name.is_null() {
                        name
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    -1i32,
                    None,
                );
                sqlite3_bind_text(stmt, 2i32, addr, -1i32, None);
                sqlite3_bind_int(stmt, 3i32, origin);
                if sqlite3_step(stmt) == 101i32 {
                    row_id = dc_sqlite3_get_rowid(
                        (*context).sql,
                        b"contacts\x00" as *const u8 as *const libc::c_char,
                        b"addr\x00" as *const u8 as *const libc::c_char,
                        addr,
                    );
                    *sth_modified = 2i32
                } else {
                    dc_log_error(
                        context,
                        0i32,
                        b"Cannot add contact.\x00" as *const u8 as *const libc::c_char,
                    );
                }
            }
        }
    }
    free(addr as *mut libc::c_void);
    free(addr_self as *mut libc::c_void);
    free(row_addr as *mut libc::c_void);
    free(row_name as *mut libc::c_void);
    free(row_authname as *mut libc::c_void);
    sqlite3_finalize(stmt);
    return row_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_add_address_book(
    mut context: *mut dc_context_t,
    mut adr_book: *const libc::c_char,
) -> libc::c_int {
    let mut lines: *mut carray = 0 as *mut carray;
    let mut i: size_t = 0i32 as size_t;
    let mut iCnt: size_t = 0i32 as size_t;
    let mut sth_modified: libc::c_int = 0i32;
    let mut modify_cnt: libc::c_int = 0i32;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || adr_book.is_null())
    {
        lines = dc_split_into_lines(adr_book);
        if !lines.is_null() {
            dc_sqlite3_begin_transaction((*context).sql);
            iCnt = carray_count(lines) as size_t;
            i = 0i32 as size_t;
            while i.wrapping_add(1i32 as libc::c_ulong) < iCnt {
                let mut name: *mut libc::c_char =
                    carray_get(lines, i as libc::c_uint) as *mut libc::c_char;
                let mut addr: *mut libc::c_char =
                    carray_get(lines, i.wrapping_add(1i32 as libc::c_ulong) as libc::c_uint)
                        as *mut libc::c_char;
                dc_normalize_name(name);
                dc_add_or_lookup_contact(context, name, addr, 0x80000i32, &mut sth_modified);
                if 0 != sth_modified {
                    modify_cnt += 1
                }
                i = (i as libc::c_ulong).wrapping_add(2i32 as libc::c_ulong) as size_t as size_t
            }
            dc_sqlite3_commit((*context).sql);
            if 0 != modify_cnt {
                (*context).cb.expect("non-null function pointer")(
                    context,
                    2030i32,
                    0i32 as uintptr_t,
                    0i32 as uintptr_t,
                );
            }
        }
    }
    dc_free_splitted_lines(lines);
    return modify_cnt;
}
// Working with names
#[no_mangle]
pub unsafe extern "C" fn dc_normalize_name(mut full_name: *mut libc::c_char) {
    if full_name.is_null() {
        return;
    }
    dc_trim(full_name);
    let mut len: libc::c_int = strlen(full_name) as libc::c_int;
    if len > 0i32 {
        let mut firstchar: libc::c_char = *full_name.offset(0isize);
        let mut lastchar: libc::c_char = *full_name.offset((len - 1i32) as isize);
        if firstchar as libc::c_int == '\'' as i32 && lastchar as libc::c_int == '\'' as i32
            || firstchar as libc::c_int == '\"' as i32 && lastchar as libc::c_int == '\"' as i32
            || firstchar as libc::c_int == '<' as i32 && lastchar as libc::c_int == '>' as i32
        {
            *full_name.offset(0isize) = ' ' as i32 as libc::c_char;
            *full_name.offset((len - 1i32) as isize) = ' ' as i32 as libc::c_char
        }
    }
    let mut p1: *mut libc::c_char = strchr(full_name, ',' as i32);
    if !p1.is_null() {
        *p1 = 0i32 as libc::c_char;
        let mut last_name: *mut libc::c_char = dc_strdup(full_name);
        let mut first_name: *mut libc::c_char = dc_strdup(p1.offset(1isize));
        dc_trim(last_name);
        dc_trim(first_name);
        strcpy(full_name, first_name);
        strcat(full_name, b" \x00" as *const u8 as *const libc::c_char);
        strcat(full_name, last_name);
        free(last_name as *mut libc::c_void);
        free(first_name as *mut libc::c_void);
    } else {
        dc_trim(full_name);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_contacts(
    mut context: *mut dc_context_t,
    mut listflags: uint32_t,
    mut query: *const libc::c_char,
) -> *mut dc_array_t {
    let mut current_block: u64;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_name2: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut add_self: libc::c_int = 0i32;
    let mut ret: *mut dc_array_t = dc_array_new(context, 100i32 as size_t);
    let mut s3strLikeCmd: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != listflags & 0x1i32 as libc::c_uint || !query.is_null() {
            s3strLikeCmd = sqlite3_mprintf(
                b"%%%s%%\x00" as *const u8 as *const libc::c_char,
                if !query.is_null() {
                    query
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
            );
            if s3strLikeCmd.is_null() {
                current_block = 7597307149762829253;
            } else {
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"SELECT c.id FROM contacts c LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.addr!=?1 AND c.id>?2 AND c.origin>=?3 AND c.blocked=0 AND (c.name LIKE ?4 OR c.addr LIKE ?5) AND (1=?6 OR LENGTH(ps.verified_key_fingerprint)!=0)  ORDER BY LOWER(c.name||c.addr),c.id;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
                sqlite3_bind_int(stmt, 2i32, 9i32);
                sqlite3_bind_int(stmt, 3i32, 0x100i32);
                sqlite3_bind_text(stmt, 4i32, s3strLikeCmd, -1i32, None);
                sqlite3_bind_text(stmt, 5i32, s3strLikeCmd, -1i32, None);
                sqlite3_bind_int(
                    stmt,
                    6i32,
                    if 0 != listflags & 0x1i32 as libc::c_uint {
                        0i32
                    } else {
                        1i32
                    },
                );
                self_name = dc_sqlite3_get_config(
                    (*context).sql,
                    b"displayname\x00" as *const u8 as *const libc::c_char,
                    b"\x00" as *const u8 as *const libc::c_char,
                );
                self_name2 = dc_stock_str(context, 2i32);
                if query.is_null()
                    || 0 != dc_str_contains(self_addr, query)
                    || 0 != dc_str_contains(self_name, query)
                    || 0 != dc_str_contains(self_name2, query)
                {
                    add_self = 1i32
                }
                current_block = 15768484401365413375;
            }
        } else {
            stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT id FROM contacts WHERE addr!=?1 AND id>?2 AND origin>=?3 AND blocked=0 ORDER BY LOWER(name||addr),id;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
            sqlite3_bind_int(stmt, 2i32, 9i32);
            sqlite3_bind_int(stmt, 3i32, 0x100i32);
            add_self = 1i32;
            current_block = 15768484401365413375;
        }
        match current_block {
            7597307149762829253 => {}
            _ => {
                while sqlite3_step(stmt) == 100i32 {
                    dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
                }
                if 0 != listflags & 0x2i32 as libc::c_uint && 0 != add_self {
                    dc_array_add_id(ret, 1i32 as uint32_t);
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    sqlite3_free(s3strLikeCmd as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    free(self_name as *mut libc::c_void);
    free(self_name2 as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_blocked_cnt(mut context: *mut dc_context_t) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT COUNT(*) FROM contacts WHERE id>? AND blocked!=0\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, 9i32);
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32)
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_blocked_contacts(
    mut context: *mut dc_context_t,
) -> *mut dc_array_t {
    let mut ret: *mut dc_array_t = dc_array_new(context, 100i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id FROM contacts WHERE id>? AND blocked!=0 ORDER BY LOWER(name||addr),id;\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, 9i32);
        while sqlite3_step(stmt) == 100i32 {
            dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_contact_encrinfo(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) -> *mut libc::c_char {
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    let mut loginparam: *mut dc_loginparam_t = dc_loginparam_new();
    let mut contact: *mut dc_contact_t = dc_contact_new(context);
    let mut peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
    let mut self_key: *mut dc_key_t = dc_key_new();
    let mut fingerprint_self: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint_other_verified: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint_other_unverified: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        ret = _dc_strbuilder {
            buf: 0 as *mut libc::c_char,
            allocated: 0,
            free: 0,
            eos: 0 as *mut libc::c_char,
        };
        dc_strbuilder_init(&mut ret, 0i32);
        if !(0 == dc_contact_load_from_db(contact, (*context).sql, contact_id)) {
            dc_apeerstate_load_by_addr(peerstate, (*context).sql, (*contact).addr);
            dc_loginparam_read(
                loginparam,
                (*context).sql,
                b"configured_\x00" as *const u8 as *const libc::c_char,
            );
            dc_key_load_self_public(self_key, (*loginparam).addr, (*context).sql);
            if !dc_apeerstate_peek_key(peerstate, 0i32).is_null() {
                p = dc_stock_str(
                    context,
                    if (*peerstate).prefer_encrypt == 1i32 {
                        34i32
                    } else {
                        25i32
                    },
                );
                dc_strbuilder_cat(&mut ret, p);
                free(p as *mut libc::c_void);
                if (*self_key).binary.is_null() {
                    dc_pgp_rand_seed(
                        context,
                        (*peerstate).addr as *const libc::c_void,
                        strlen((*peerstate).addr),
                    );
                    dc_ensure_secret_key_exists(context);
                    dc_key_load_self_public(self_key, (*loginparam).addr, (*context).sql);
                }
                dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
                p = dc_stock_str(context, 30i32);
                dc_strbuilder_cat(&mut ret, p);
                free(p as *mut libc::c_void);
                dc_strbuilder_cat(&mut ret, b":\x00" as *const u8 as *const libc::c_char);
                fingerprint_self = dc_key_get_formatted_fingerprint(self_key);
                fingerprint_other_verified =
                    dc_key_get_formatted_fingerprint(dc_apeerstate_peek_key(peerstate, 2i32));
                fingerprint_other_unverified =
                    dc_key_get_formatted_fingerprint(dc_apeerstate_peek_key(peerstate, 0i32));
                if strcmp((*loginparam).addr, (*peerstate).addr) < 0i32 {
                    cat_fingerprint(
                        &mut ret,
                        (*loginparam).addr,
                        fingerprint_self,
                        0 as *const libc::c_char,
                    );
                    cat_fingerprint(
                        &mut ret,
                        (*peerstate).addr,
                        fingerprint_other_verified,
                        fingerprint_other_unverified,
                    );
                } else {
                    cat_fingerprint(
                        &mut ret,
                        (*peerstate).addr,
                        fingerprint_other_verified,
                        fingerprint_other_unverified,
                    );
                    cat_fingerprint(
                        &mut ret,
                        (*loginparam).addr,
                        fingerprint_self,
                        0 as *const libc::c_char,
                    );
                }
            } else if 0 == (*loginparam).server_flags & 0x400i32
                && 0 == (*loginparam).server_flags & 0x40000i32
            {
                p = dc_stock_str(context, 27i32);
                dc_strbuilder_cat(&mut ret, p);
                free(p as *mut libc::c_void);
            } else {
                p = dc_stock_str(context, 28i32);
                dc_strbuilder_cat(&mut ret, p);
                free(p as *mut libc::c_void);
            }
        }
    }
    dc_apeerstate_unref(peerstate);
    dc_contact_unref(contact);
    dc_loginparam_unref(loginparam);
    dc_key_unref(self_key);
    free(fingerprint_self as *mut libc::c_void);
    free(fingerprint_other_verified as *mut libc::c_void);
    free(fingerprint_other_unverified as *mut libc::c_void);
    return ret.buf;
}
unsafe extern "C" fn cat_fingerprint(
    mut ret: *mut dc_strbuilder_t,
    mut addr: *const libc::c_char,
    mut fingerprint_verified: *const libc::c_char,
    mut fingerprint_unverified: *const libc::c_char,
) {
    dc_strbuilder_cat(ret, b"\n\n\x00" as *const u8 as *const libc::c_char);
    dc_strbuilder_cat(ret, addr);
    dc_strbuilder_cat(ret, b":\n\x00" as *const u8 as *const libc::c_char);
    dc_strbuilder_cat(
        ret,
        if !fingerprint_verified.is_null()
            && 0 != *fingerprint_verified.offset(0isize) as libc::c_int
        {
            fingerprint_verified
        } else {
            fingerprint_unverified
        },
    );
    if !fingerprint_verified.is_null()
        && 0 != *fingerprint_verified.offset(0isize) as libc::c_int
        && !fingerprint_unverified.is_null()
        && 0 != *fingerprint_unverified.offset(0isize) as libc::c_int
        && strcmp(fingerprint_verified, fingerprint_unverified) != 0i32
    {
        dc_strbuilder_cat(ret, b"\n\n\x00" as *const u8 as *const libc::c_char);
        dc_strbuilder_cat(ret, addr);
        dc_strbuilder_cat(
            ret,
            b" (alternative):\n\x00" as *const u8 as *const libc::c_char,
        );
        dc_strbuilder_cat(ret, fingerprint_unverified);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_delete_contact(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || contact_id <= 9i32 as libc::c_uint)
    {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT COUNT(*) FROM chats_contacts WHERE contact_id=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
        if !(sqlite3_step(stmt) != 100i32 || sqlite3_column_int(stmt, 0i32) >= 1i32) {
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            stmt = dc_sqlite3_prepare(
                (*context).sql,
                b"SELECT COUNT(*) FROM msgs WHERE from_id=? OR to_id=?;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
            sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
            if !(sqlite3_step(stmt) != 100i32 || sqlite3_column_int(stmt, 0i32) >= 1i32) {
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"DELETE FROM contacts WHERE id=?;\x00" as *const u8 as *const libc::c_char,
                );
                sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
                if !(sqlite3_step(stmt) != 101i32) {
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2030i32,
                        0i32 as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    success = 1i32
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_contact(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) -> *mut dc_contact_t {
    let mut ret: *mut dc_contact_t = dc_contact_new(context);
    if 0 == dc_contact_load_from_db(ret, (*context).sql, contact_id) {
        dc_contact_unref(ret);
        ret = 0 as *mut dc_contact_t
    }
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_id(mut contact: *const dc_contact_t) -> uint32_t {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    return (*contact).id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_addr(
    mut contact: *const dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    return dc_strdup((*contact).addr);
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_name(
    mut contact: *const dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    return dc_strdup((*contact).name);
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_display_name(
    mut contact: *const dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
        return dc_strdup((*contact).name);
    }
    return dc_strdup((*contact).addr);
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_name_n_addr(
    mut contact: *const dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
        return dc_mprintf(
            b"%s (%s)\x00" as *const u8 as *const libc::c_char,
            (*contact).name,
            (*contact).addr,
        );
    }
    return dc_strdup((*contact).addr);
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_first_name(
    mut contact: *const dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
        return dc_get_first_name((*contact).name);
    }
    return dc_strdup((*contact).addr);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_first_name(
    mut full_name: *const libc::c_char,
) -> *mut libc::c_char {
    let mut first_name: *mut libc::c_char = dc_strdup(full_name);
    let mut p1: *mut libc::c_char = strchr(first_name, ' ' as i32);
    if !p1.is_null() {
        *p1 = 0i32 as libc::c_char;
        dc_rtrim(first_name);
        if *first_name.offset(0isize) as libc::c_int == 0i32 {
            free(first_name as *mut libc::c_void);
            first_name = dc_strdup(full_name)
        }
    }
    return first_name;
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_profile_image(
    mut contact: *const dc_contact_t,
) -> *mut libc::c_char {
    let mut selfavatar: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut image_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint) {
        if (*contact).id == 1i32 as libc::c_uint {
            selfavatar = dc_get_config(
                (*contact).context,
                b"selfavatar\x00" as *const u8 as *const libc::c_char,
            );
            if !selfavatar.is_null() && 0 != *selfavatar.offset(0isize) as libc::c_int {
                image_abs = dc_strdup(selfavatar)
            }
        }
    }
    // TODO: else get image_abs from contact param
    free(selfavatar as *mut libc::c_void);
    return image_abs;
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_color(mut contact: *const dc_contact_t) -> uint32_t {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    return dc_str_to_color((*contact).addr) as uint32_t;
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_is_blocked(mut contact: *const dc_contact_t) -> libc::c_int {
    if contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint {
        return 0i32;
    }
    return (*contact).blocked;
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_is_verified(mut contact: *mut dc_contact_t) -> libc::c_int {
    return dc_contact_is_verified_ex(contact, 0 as *const dc_apeerstate_t);
}
#[no_mangle]
pub unsafe extern "C" fn dc_contact_is_verified_ex(
    mut contact: *mut dc_contact_t,
    mut peerstate: *const dc_apeerstate_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut contact_verified: libc::c_int = 0i32;
    let mut peerstate_to_delete: *mut dc_apeerstate_t = 0 as *mut dc_apeerstate_t;
    if !(contact.is_null() || (*contact).magic != 0xc047ac7i32 as libc::c_uint) {
        if (*contact).id == 1i32 as libc::c_uint {
            contact_verified = 2i32
        } else {
            // we're always sort of secured-verified as we could verify the key on this device any time with the key on this device
            if peerstate.is_null() {
                peerstate_to_delete = dc_apeerstate_new((*contact).context);
                if 0 == dc_apeerstate_load_by_addr(
                    peerstate_to_delete,
                    (*(*contact).context).sql,
                    (*contact).addr,
                ) {
                    current_block = 8667923638376902112;
                } else {
                    peerstate = peerstate_to_delete;
                    current_block = 13109137661213826276;
                }
            } else {
                current_block = 13109137661213826276;
            }
            match current_block {
                8667923638376902112 => {}
                _ => {
                    contact_verified = if !(*peerstate).verified_key.is_null() {
                        2i32
                    } else {
                        0i32
                    }
                }
            }
        }
    }
    dc_apeerstate_unref(peerstate_to_delete);
    return contact_verified;
}
// Working with e-mail-addresses
#[no_mangle]
pub unsafe extern "C" fn dc_addr_cmp(
    mut addr1: *const libc::c_char,
    mut addr2: *const libc::c_char,
) -> libc::c_int {
    let mut norm1: *mut libc::c_char = dc_addr_normalize(addr1);
    let mut norm2: *mut libc::c_char = dc_addr_normalize(addr2);
    let mut ret: libc::c_int = strcasecmp(addr1, addr2);
    free(norm1 as *mut libc::c_void);
    free(norm2 as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_addr_equals_self(
    mut context: *mut dc_context_t,
    mut addr: *const libc::c_char,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut normalized_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null() || addr.is_null()) {
        normalized_addr = dc_addr_normalize(addr);
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if !self_addr.is_null() {
            ret = if strcasecmp(normalized_addr, self_addr) == 0i32 {
                1i32
            } else {
                0i32
            }
        }
    }
    free(self_addr as *mut libc::c_void);
    free(normalized_addr as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_addr_equals_contact(
    mut context: *mut dc_context_t,
    mut addr: *const libc::c_char,
    mut contact_id: uint32_t,
) -> libc::c_int {
    let mut addr_are_equal: libc::c_int = 0i32;
    if !addr.is_null() {
        let mut contact: *mut dc_contact_t = dc_contact_new(context);
        if 0 != dc_contact_load_from_db(contact, (*context).sql, contact_id) {
            if !(*contact).addr.is_null() {
                let mut normalized_addr: *mut libc::c_char = dc_addr_normalize(addr);
                if strcasecmp((*contact).addr, normalized_addr) == 0i32 {
                    addr_are_equal = 1i32
                }
                free(normalized_addr as *mut libc::c_void);
            }
        }
        dc_contact_unref(contact);
    }
    return addr_are_equal;
}
// Context functions to work with contacts
#[no_mangle]
pub unsafe extern "C" fn dc_get_real_contact_cnt(mut context: *mut dc_context_t) -> size_t {
    let mut ret: size_t = 0i32 as size_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null())
    {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT COUNT(*) FROM contacts WHERE id>?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, 9i32);
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32) as size_t
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_contact_origin(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
    mut ret_blocked: *mut libc::c_int,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut dummy: libc::c_int = 0i32;
    if ret_blocked.is_null() {
        ret_blocked = &mut dummy
    }
    let mut contact: *mut dc_contact_t = dc_contact_new(context);
    *ret_blocked = 0i32;
    if !(0 == dc_contact_load_from_db(contact, (*context).sql, contact_id)) {
        /* we could optimize this by loading only the needed fields */
        if 0 != (*contact).blocked {
            *ret_blocked = 1i32
        } else {
            ret = (*contact).origin
        }
    }
    dc_contact_unref(contact);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_real_contact_exists(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut ret: libc::c_int = 0i32;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null()
        || contact_id <= 9i32 as libc::c_uint)
    {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id FROM contacts WHERE id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
        if sqlite3_step(stmt) == 100i32 {
            ret = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_scaleup_contact_origin(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
    mut origin: libc::c_int,
) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE contacts SET origin=? WHERE id=? AND origin<?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, origin);
    sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
    sqlite3_bind_int(stmt, 3i32, origin);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

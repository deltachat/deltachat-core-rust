use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type sqlite3_stmt;
    #[no_mangle]
    fn __toupper(_: __darwin_ct_rune_t) -> __darwin_ct_rune_t;
    #[no_mangle]
    fn memcmp(_: *const libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn memcpy(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn mmap_string_unref(str: *mut libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn mailmime_base64_body_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut libc::c_char,
        result_len: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_blob(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_void,
        n: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int64(_: *mut sqlite3_stmt, _: libc::c_int, _: sqlite3_int64) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_text(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_char,
        _: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_blob(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_void;
    #[no_mangle]
    fn sqlite3_column_bytes(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_binary_to_uc_hex(buf: *const uint8_t, bytes: size_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_insert_breaks(
        _: *const libc::c_char,
        break_every: libc::c_int,
        break_chars: *const libc::c_char,
    ) -> *mut libc::c_char;
    // from libetpan/src/data-types/base64.h (which cannot be included without adding libetpan/src/... to the include-search-paths, which would result in double-file-name-errors, so, for now, we use this hack)
    #[no_mangle]
    fn encode_base64(in_0: *const libc::c_char, len: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_write_file(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
        buf: *const libc::c_void,
        buf_bytes: size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_read_file(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
        buf: *mut *mut libc::c_void,
        buf_bytes: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strbuilder_catf(_: *mut dc_strbuilder_t, format: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_split_armored_data(
        buf: *mut libc::c_char,
        ret_headerline: *mut *const libc::c_char,
        ret_setupcodebegin: *mut *const libc::c_char,
        ret_preferencrypt: *mut *const libc::c_char,
        ret_base64: *mut *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_pgp_calc_fingerprint(
        _: *const dc_key_t,
        fingerprint: *mut *mut uint8_t,
        fingerprint_bytes: *mut size_t,
    ) -> libc::c_int;
}
pub type __darwin_ct_rune_t = libc::c_int;
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
pub type uintptr_t = libc::c_ulong;
pub type ssize_t = __darwin_ssize_t;
pub type uint8_t = libc::c_uchar;
pub type uint32_t = libc::c_uint;
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
pub type unnamed = libc::c_uint;
pub const MAILIMF_ERROR_FILE: unnamed = 4;
pub const MAILIMF_ERROR_INVAL: unnamed = 3;
pub const MAILIMF_ERROR_MEMORY: unnamed = 2;
pub const MAILIMF_ERROR_PARSE: unnamed = 1;
pub const MAILIMF_NO_ERROR: unnamed = 0;
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
    pub sec_data: unnamed_1,
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
pub union unnamed_1 {
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
    pub imap_sasl: unnamed_3,
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
pub type dc_lot_t = _dc_lot;
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
pub type dc_context_t = _dc_context;
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
pub type dc_sqlite3_t = _dc_sqlite3;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_sqlite3 {
    pub cobj: *mut sqlite3,
    pub context: *mut dc_context_t,
}
pub type sqlite_int64 = libc::c_longlong;
pub type sqlite3_int64 = sqlite_int64;
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type dc_strbuilder_t = _dc_strbuilder;
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
pub type dc_key_t = _dc_key;
#[no_mangle]
#[inline]
pub unsafe extern "C" fn toupper(mut _c: libc::c_int) -> libc::c_int {
    return __toupper(_c);
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_new() -> *mut dc_key_t {
    let mut key: *mut dc_key_t = 0 as *mut dc_key_t;
    key = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_key_t>() as libc::c_ulong,
    ) as *mut dc_key_t;
    if key.is_null() {
        exit(44i32);
    }
    (*key)._m_heap_refcnt = 1i32;
    return key;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_ref(mut key: *mut dc_key_t) -> *mut dc_key_t {
    if key.is_null() {
        return 0 as *mut dc_key_t;
    }
    (*key)._m_heap_refcnt += 1;
    return key;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_unref(mut key: *mut dc_key_t) {
    if key.is_null() {
        return;
    }
    (*key)._m_heap_refcnt -= 1;
    if (*key)._m_heap_refcnt != 0i32 {
        return;
    }
    dc_key_empty(key);
    free(key as *mut libc::c_void);
}
unsafe extern "C" fn dc_key_empty(mut key: *mut dc_key_t) {
    if key.is_null() {
        return;
    }
    if (*key).type_0 == 1i32 {
        dc_wipe_secret_mem((*key).binary, (*key).bytes as size_t);
    }
    free((*key).binary);
    (*key).binary = 0 as *mut libc::c_void;
    (*key).bytes = 0i32;
    (*key).type_0 = 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_wipe_secret_mem(mut buf: *mut libc::c_void, mut buf_bytes: size_t) {
    if buf.is_null() || buf_bytes <= 0i32 as libc::c_ulong {
        return;
    }
    memset(buf, 0i32, buf_bytes);
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_set_from_binary(
    mut key: *mut dc_key_t,
    mut data: *const libc::c_void,
    mut bytes: libc::c_int,
    mut type_0: libc::c_int,
) -> libc::c_int {
    dc_key_empty(key);
    if key.is_null() || data == 0 as *mut libc::c_void || bytes <= 0i32 {
        return 0i32;
    }
    (*key).binary = malloc(bytes as libc::c_ulong);
    if (*key).binary.is_null() {
        exit(40i32);
    }
    memcpy((*key).binary, data, bytes as libc::c_ulong);
    (*key).bytes = bytes;
    (*key).type_0 = type_0;
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_set_from_key(
    mut key: *mut dc_key_t,
    mut o: *const dc_key_t,
) -> libc::c_int {
    dc_key_empty(key);
    if key.is_null() || o.is_null() {
        return 0i32;
    }
    return dc_key_set_from_binary(key, (*o).binary, (*o).bytes, (*o).type_0);
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_set_from_stmt(
    mut key: *mut dc_key_t,
    mut stmt: *mut sqlite3_stmt,
    mut index: libc::c_int,
    mut type_0: libc::c_int,
) -> libc::c_int {
    dc_key_empty(key);
    if key.is_null() || stmt.is_null() {
        return 0i32;
    }
    return dc_key_set_from_binary(
        key,
        sqlite3_column_blob(stmt, index) as *mut libc::c_uchar as *const libc::c_void,
        sqlite3_column_bytes(stmt, index),
        type_0,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_set_from_base64(
    mut key: *mut dc_key_t,
    mut base64: *const libc::c_char,
    mut type_0: libc::c_int,
) -> libc::c_int {
    let mut indx: size_t = 0i32 as size_t;
    let mut result_len: size_t = 0i32 as size_t;
    let mut result: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_key_empty(key);
    if key.is_null() || base64.is_null() {
        return 0i32;
    }
    if mailmime_base64_body_parse(
        base64,
        strlen(base64),
        &mut indx,
        &mut result,
        &mut result_len,
    ) != MAILIMF_NO_ERROR as libc::c_int
        || result.is_null()
        || result_len == 0i32 as libc::c_ulong
    {
        return 0i32;
    }
    dc_key_set_from_binary(
        key,
        result as *const libc::c_void,
        result_len as libc::c_int,
        type_0,
    );
    mmap_string_unref(result);
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_set_from_file(
    mut key: *mut dc_key_t,
    mut pathNfilename: *const libc::c_char,
    mut context: *mut dc_context_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    // just pointer inside buf, must not be freed
    let mut headerline: *const libc::c_char = 0 as *const libc::c_char;
    //   - " -
    let mut base64: *const libc::c_char = 0 as *const libc::c_char;
    let mut buf_bytes: size_t = 0i32 as size_t;
    let mut type_0: libc::c_int = -1i32;
    let mut success: libc::c_int = 0i32;
    dc_key_empty(key);
    if !(key.is_null() || pathNfilename.is_null()) {
        if !(0
            == dc_read_file(
                context,
                pathNfilename,
                &mut buf as *mut *mut libc::c_char as *mut *mut libc::c_void,
                &mut buf_bytes,
            )
            || buf_bytes < 50i32 as libc::c_ulong)
        {
            /* error is already loged */
            if !(0
                == dc_split_armored_data(
                    buf,
                    &mut headerline,
                    0 as *mut *const libc::c_char,
                    0 as *mut *const libc::c_char,
                    &mut base64,
                )
                || headerline.is_null()
                || base64.is_null())
            {
                if strcmp(
                    headerline,
                    b"-----BEGIN PGP PUBLIC KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    type_0 = 0i32;
                    current_block = 7149356873433890176;
                } else if strcmp(
                    headerline,
                    b"-----BEGIN PGP PRIVATE KEY BLOCK-----\x00" as *const u8
                        as *const libc::c_char,
                ) == 0i32
                {
                    type_0 = 1i32;
                    current_block = 7149356873433890176;
                } else {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Header missing for key \"%s\".\x00" as *const u8 as *const libc::c_char,
                        pathNfilename,
                    );
                    current_block = 7704194852291245876;
                }
                match current_block {
                    7704194852291245876 => {}
                    _ => {
                        if 0 == dc_key_set_from_base64(key, base64, type_0) {
                            dc_log_warning(
                                context,
                                0i32,
                                b"Bad data in key \"%s\".\x00" as *const u8 as *const libc::c_char,
                                pathNfilename,
                            );
                        } else {
                            success = 1i32
                        }
                    }
                }
            }
        }
    }
    free(buf as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_equals(
    mut key: *const dc_key_t,
    mut o: *const dc_key_t,
) -> libc::c_int {
    if key.is_null()
        || o.is_null()
        || (*key).binary.is_null()
        || (*key).bytes <= 0i32
        || (*o).binary.is_null()
        || (*o).bytes <= 0i32
    {
        return 0i32;
    }
    if (*key).bytes != (*o).bytes {
        return 0i32;
    }
    if (*key).type_0 != (*o).type_0 {
        return 0i32;
    }
    return if memcmp((*key).binary, (*o).binary, (*o).bytes as libc::c_ulong) == 0i32 {
        1i32
    } else {
        0i32
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_save_self_keypair(
    mut public_key: *const dc_key_t,
    mut private_key: *const dc_key_t,
    mut addr: *const libc::c_char,
    mut is_default: libc::c_int,
    mut sql: *mut dc_sqlite3_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(public_key.is_null()
        || private_key.is_null()
        || addr.is_null()
        || sql.is_null()
        || (*public_key).binary.is_null()
        || (*private_key).binary.is_null())
    {
        stmt =
            dc_sqlite3_prepare(sql,
                               b"INSERT INTO keypairs (addr, is_default, public_key, private_key, created) VALUES (?,?,?,?,?);\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1i32, addr, -1i32, None);
        sqlite3_bind_int(stmt, 2i32, is_default);
        sqlite3_bind_blob(stmt, 3i32, (*public_key).binary, (*public_key).bytes, None);
        sqlite3_bind_blob(
            stmt,
            4i32,
            (*private_key).binary,
            (*private_key).bytes,
            None,
        );
        sqlite3_bind_int64(stmt, 5i32, time(0 as *mut time_t) as sqlite3_int64);
        if !(sqlite3_step(stmt) != 101i32) {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_load_self_public(
    mut key: *mut dc_key_t,
    mut self_addr: *const libc::c_char,
    mut sql: *mut dc_sqlite3_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(key.is_null() || self_addr.is_null() || sql.is_null()) {
        dc_key_empty(key);
        stmt = dc_sqlite3_prepare(
            sql,
            b"SELECT public_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            dc_key_set_from_stmt(key, stmt, 0i32, 0i32);
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_load_self_private(
    mut key: *mut dc_key_t,
    mut self_addr: *const libc::c_char,
    mut sql: *mut dc_sqlite3_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(key.is_null() || self_addr.is_null() || sql.is_null()) {
        dc_key_empty(key);
        stmt = dc_sqlite3_prepare(
            sql,
            b"SELECT private_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, self_addr, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            dc_key_set_from_stmt(key, stmt, 0i32, 1i32);
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
/* the result must be freed */
#[no_mangle]
pub unsafe extern "C" fn dc_render_base64(
    mut buf: *const libc::c_void,
    mut buf_bytes: size_t,
    mut break_every: libc::c_int,
    mut break_chars: *const libc::c_char,
    mut add_checksum: libc::c_int,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(buf == 0 as *mut libc::c_void || buf_bytes <= 0i32 as libc::c_ulong) {
        ret = encode_base64(buf as *const libc::c_char, buf_bytes as libc::c_int);
        if !ret.is_null() {
            if break_every > 0i32 {
                let mut temp: *mut libc::c_char = ret;
                ret = dc_insert_breaks(temp, break_every, break_chars);
                free(temp as *mut libc::c_void);
            }
            if add_checksum == 2i32 {
                let mut checksum: libc::c_long = crc_octets(buf as *const libc::c_uchar, buf_bytes);
                let mut c: [uint8_t; 3] = [0; 3];
                c[0usize] = (checksum >> 16i32 & 0xffi32 as libc::c_long) as uint8_t;
                c[1usize] = (checksum >> 8i32 & 0xffi32 as libc::c_long) as uint8_t;
                c[2usize] = (checksum & 0xffi32 as libc::c_long) as uint8_t;
                let mut c64: *mut libc::c_char =
                    encode_base64(c.as_mut_ptr() as *const libc::c_char, 3i32);
                let mut temp_0: *mut libc::c_char = ret;
                ret = dc_mprintf(
                    b"%s%s=%s\x00" as *const u8 as *const libc::c_char,
                    temp_0,
                    break_chars,
                    c64,
                );
                free(temp_0 as *mut libc::c_void);
                free(c64 as *mut libc::c_void);
            }
        }
    }
    return ret;
}
/* ******************************************************************************
 * Render keys
 ******************************************************************************/
unsafe extern "C" fn crc_octets(mut octets: *const libc::c_uchar, mut len: size_t) -> libc::c_long {
    let mut crc: libc::c_long = 0xb704cei64;
    loop {
        let fresh0 = len;
        len = len.wrapping_sub(1);
        if !(0 != fresh0) {
            break;
        }
        let fresh1 = octets;
        octets = octets.offset(1);
        crc ^= ((*fresh1 as libc::c_int) << 16i32) as libc::c_long;
        let mut i: libc::c_int = 0i32;
        while i < 8i32 {
            crc <<= 1i32;
            if 0 != crc & 0x1000000i32 as libc::c_long {
                crc ^= 0x1864cfbi64
            }
            i += 1
        }
    }
    return crc & 0xffffffi64;
}
/* the result must be freed */
#[no_mangle]
pub unsafe extern "C" fn dc_key_render_base64(
    mut key: *const dc_key_t,
    mut break_every: libc::c_int,
    mut break_chars: *const libc::c_char,
    mut add_checksum: libc::c_int,
) -> *mut libc::c_char {
    if key.is_null() {
        return 0 as *mut libc::c_char;
    }
    return dc_render_base64(
        (*key).binary,
        (*key).bytes as size_t,
        break_every,
        break_chars,
        add_checksum,
    );
}
/* each header line must be terminated by \r\n, the result must be freed */
#[no_mangle]
pub unsafe extern "C" fn dc_key_render_asc(
    mut key: *const dc_key_t,
    mut add_header_lines: *const libc::c_char,
) -> *mut libc::c_char {
    /* see RFC 4880, 6.2.  Forming ASCII Armor, https://tools.ietf.org/html/rfc4880#section-6.2 */
    let mut base64: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !key.is_null() {
        base64 = dc_key_render_base64(
            key,
            76i32,
            b"\r\n\x00" as *const u8 as *const libc::c_char,
            2i32,
        );
        if !base64.is_null() {
            /*checksum in new line*/
            /* RFC: The encoded output stream must be represented in lines of no more than 76 characters each. */
            ret =
                dc_mprintf(b"-----BEGIN PGP %s KEY BLOCK-----\r\n%s\r\n%s\r\n-----END PGP %s KEY BLOCK-----\r\n\x00"
                               as *const u8 as *const libc::c_char,
                           if (*key).type_0 == 0i32 {
                               b"PUBLIC\x00" as *const u8 as
                                   *const libc::c_char
                           } else {
                               b"PRIVATE\x00" as *const u8 as
                                   *const libc::c_char
                           },
                           if !add_header_lines.is_null() {
                               add_header_lines
                           } else {
                               b"\x00" as *const u8 as *const libc::c_char
                           }, base64,
                           if (*key).type_0 == 0i32 {
                               b"PUBLIC\x00" as *const u8 as
                                   *const libc::c_char
                           } else {
                               b"PRIVATE\x00" as *const u8 as
                                   *const libc::c_char
                           })
        }
    }
    free(base64 as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_render_asc_to_file(
    mut key: *const dc_key_t,
    mut file: *const libc::c_char,
    mut context: *mut dc_context_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut file_content: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(key.is_null() || file.is_null() || context.is_null()) {
        file_content = dc_key_render_asc(key, 0 as *const libc::c_char);
        if !file_content.is_null() {
            if 0 == dc_write_file(
                context,
                file,
                file_content as *const libc::c_void,
                strlen(file_content),
            ) {
                dc_log_error(
                    context,
                    0i32,
                    b"Cannot write key to %s\x00" as *const u8 as *const libc::c_char,
                    file,
                );
            } else {
                success = 1i32
            }
        }
    }
    free(file_content as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_format_fingerprint(
    mut fingerprint: *const libc::c_char,
) -> *mut libc::c_char {
    let mut i: libc::c_int = 0i32;
    let mut fingerprint_len: libc::c_int = strlen(fingerprint) as libc::c_int;
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    while 0 != *fingerprint.offset(i as isize) {
        dc_strbuilder_catf(
            &mut ret as *mut dc_strbuilder_t,
            b"%c\x00" as *const u8 as *const libc::c_char,
            *fingerprint.offset(i as isize) as libc::c_int,
        );
        i += 1;
        if i != fingerprint_len {
            if i % 20i32 == 0i32 {
                dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
            } else if i % 4i32 == 0i32 {
                dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
            }
        }
    }
    return ret.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_normalize_fingerprint(
    mut in_0: *const libc::c_char,
) -> *mut libc::c_char {
    if in_0.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut out: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut out, 0i32);
    let mut p1: *const libc::c_char = in_0;
    while 0 != *p1 {
        if *p1 as libc::c_int >= '0' as i32 && *p1 as libc::c_int <= '9' as i32
            || *p1 as libc::c_int >= 'A' as i32 && *p1 as libc::c_int <= 'F' as i32
            || *p1 as libc::c_int >= 'a' as i32 && *p1 as libc::c_int <= 'f' as i32
        {
            dc_strbuilder_catf(
                &mut out as *mut dc_strbuilder_t,
                b"%c\x00" as *const u8 as *const libc::c_char,
                toupper(*p1 as libc::c_int),
            );
        }
        p1 = p1.offset(1isize)
    }
    return out.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_get_fingerprint(mut key: *const dc_key_t) -> *mut libc::c_char {
    let mut fingerprint_buf: *mut uint8_t = 0 as *mut uint8_t;
    let mut fingerprint_bytes: size_t = 0i32 as size_t;
    let mut fingerprint_hex: *mut libc::c_char = 0 as *mut libc::c_char;
    if !key.is_null() {
        if !(0 == dc_pgp_calc_fingerprint(key, &mut fingerprint_buf, &mut fingerprint_bytes)) {
            fingerprint_hex = dc_binary_to_uc_hex(fingerprint_buf, fingerprint_bytes)
        }
    }
    free(fingerprint_buf as *mut libc::c_void);
    return if !fingerprint_hex.is_null() {
        fingerprint_hex
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_key_get_formatted_fingerprint(
    mut key: *const dc_key_t,
) -> *mut libc::c_char {
    let mut rawhex: *mut libc::c_char = dc_key_get_fingerprint(key);
    let mut formatted: *mut libc::c_char = dc_format_fingerprint(rawhex);
    free(rawhex as *mut libc::c_void);
    return formatted;
}

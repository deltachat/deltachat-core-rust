use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type sqlite3_stmt;

    #[no_mangle]
    pub fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;

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
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn gmtime(_: *const time_t) -> *mut tm;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn dc_send_msg(_: *mut dc_context_t, chat_id: uint32_t, _: *mut dc_msg_t) -> uint32_t;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_unref(_: *mut dc_msg_t);
    #[no_mangle]
    fn dc_job_add(
        _: *mut dc_context_t,
        action: libc::c_int,
        foreign_id: libc::c_int,
        param: *const libc::c_char,
        delay: libc::c_int,
    );
    #[no_mangle]
    fn dc_job_action_exists(_: *mut dc_context_t, action: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_add_device_msg(_: *mut dc_context_t, chat_id: uint32_t, text: *const libc::c_char);
    /* Misc. */
    #[no_mangle]
    fn dc_stock_system_msg(
        context: *mut dc_context_t,
        str_id: libc::c_int,
        param1: *const libc::c_char,
        param2: *const libc::c_char,
        from_id: uint32_t,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
    /* *
     * @class dc_msg_t
     *
     * An object representing a single message in memory.
     * The message object is not updated.
     * If you want an update, you have to recreate the object.
     */
    // to check if a mail was sent, use dc_msg_is_sent()
    // approx. max. lenght returned by dc_msg_get_text()
    // approx. max. lenght returned by dc_get_msg_info()
    #[no_mangle]
    fn dc_msg_new(_: *mut dc_context_t, viewtype: libc::c_int) -> *mut dc_msg_t;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int64(_: *mut sqlite3_stmt, _: libc::c_int, _: sqlite3_int64) -> libc::c_int;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_double(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_double) -> libc::c_int;
    #[no_mangle]
    fn dc_array_new_typed(
        _: *mut dc_context_t,
        type_0: libc::c_int,
        initsize: size_t,
    ) -> *mut dc_array_t;
    #[no_mangle]
    fn dc_array_add_ptr(_: *mut dc_array_t, _: *mut libc::c_void);
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    #[no_mangle]
    fn dc_utf8_strlen(_: *const libc::c_char) -> size_t;
    #[no_mangle]
    fn sqlite3_column_int64(_: *mut sqlite3_stmt, iCol: libc::c_int) -> sqlite3_int64;
    #[no_mangle]
    fn sqlite3_column_double(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_double;
    /* *
     * @class dc_array_t
     *
     * An object containing a simple array.
     * This object is used in several places where functions need to return an array.
     * The items of the array are typically IDs.
     * To free an array object, use dc_array_unref().
     */
    #[no_mangle]
    fn dc_array_unref(_: *mut dc_array_t);
    #[no_mangle]
    fn dc_array_get_cnt(_: *const dc_array_t) -> size_t;
    #[no_mangle]
    fn dc_array_get_ptr(_: *const dc_array_t, index: size_t) -> *mut libc::c_void;
    #[no_mangle]
    fn sqlite3_reset(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_sqlite3_get_rowid2(
        _: *mut dc_sqlite3_t,
        table: *const libc::c_char,
        field: *const libc::c_char,
        value: uint64_t,
        field2: *const libc::c_char,
        value2: uint32_t,
    ) -> uint32_t;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_atof(_: *const libc::c_char) -> libc::c_double;
    #[no_mangle]
    fn dc_ftoa(_: libc::c_double) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_str_replace(
        haystack: *mut *mut libc::c_char,
        needle: *const libc::c_char,
        replacement: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_null_terminate(_: *const libc::c_char, bytes: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn mkgmtime(_: *mut tm) -> time_t;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strbuilder_catf(_: *mut dc_strbuilder_t, format: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
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
pub type int32_t = libc::c_int;
pub type uintptr_t = libc::c_ulong;
pub type size_t = __darwin_size_t;
pub type uint8_t = libc::c_uchar;
pub type uint32_t = libc::c_uint;
pub type uint64_t = libc::c_ulonglong;
pub type ssize_t = __darwin_ssize_t;
pub type time_t = __darwin_time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: libc::c_int,
    pub tm_min: libc::c_int,
    pub tm_hour: libc::c_int,
    pub tm_mday: libc::c_int,
    pub tm_mon: libc::c_int,
    pub tm_year: libc::c_int,
    pub tm_wday: libc::c_int,
    pub tm_yday: libc::c_int,
    pub tm_isdst: libc::c_int,
    pub tm_gmtoff: libc::c_long,
    pub tm_zone: *mut libc::c_char,
}
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
/* * the structure behind dc_msg_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_msg {
    pub magic: uint32_t,
    pub id: uint32_t,
    pub from_id: uint32_t,
    pub to_id: uint32_t,
    pub chat_id: uint32_t,
    pub move_state: dc_move_state_t,
    pub type_0: libc::c_int,
    pub state: libc::c_int,
    pub hidden: libc::c_int,
    pub timestamp_sort: time_t,
    pub timestamp_sent: time_t,
    pub timestamp_rcvd: time_t,
    pub text: *mut libc::c_char,
    pub context: *mut dc_context_t,
    pub rfc724_mid: *mut libc::c_char,
    pub in_reply_to: *mut libc::c_char,
    pub server_folder: *mut libc::c_char,
    pub server_uid: uint32_t,
    pub is_dc_message: libc::c_int,
    pub starred: libc::c_int,
    pub chat_blocked: libc::c_int,
    pub location_id: uint32_t,
    pub param: *mut dc_param_t,
}
pub type dc_move_state_t = libc::c_uint;
pub const DC_MOVE_STATE_MOVING: dc_move_state_t = 3;
pub const DC_MOVE_STATE_STAY: dc_move_state_t = 2;
pub const DC_MOVE_STATE_PENDING: dc_move_state_t = 1;
pub const DC_MOVE_STATE_UNDEFINED: dc_move_state_t = 0;
pub type dc_msg_t = _dc_msg;
pub type sqlite3_int64 = sqlite_int64;
pub type sqlite_int64 = libc::c_longlong;
// location handling
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_location {
    pub location_id: uint32_t,
    pub latitude: libc::c_double,
    pub longitude: libc::c_double,
    pub accuracy: libc::c_double,
    pub timestamp: time_t,
    pub contact_id: uint32_t,
    pub msg_id: uint32_t,
    pub chat_id: uint32_t,
    pub marker: *mut libc::c_char,
}
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_kml {
    pub addr: *mut libc::c_char,
    pub locations: *mut dc_array_t,
    pub tag: libc::c_int,
    pub curr: dc_location_t,
}
pub type dc_location_t = _dc_location;
pub type dc_kml_t = _dc_kml;
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
// location streaming
#[no_mangle]
pub unsafe extern "C" fn dc_send_locations_to_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut seconds: libc::c_int,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut now: time_t = time(0 as *mut time_t);
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut stock_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut is_sending_locations_before: libc::c_int = 0i32;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || seconds < 0i32
        || chat_id <= 9i32 as libc::c_uint)
    {
        is_sending_locations_before = dc_is_sending_locations_to_chat(context, chat_id);
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"UPDATE chats    SET locations_send_begin=?,        locations_send_until=?  WHERE id=?\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(
            stmt,
            1i32,
            (if 0 != seconds {
                now
            } else {
                0i32 as libc::c_long
            }) as sqlite3_int64,
        );
        sqlite3_bind_int64(
            stmt,
            2i32,
            (if 0 != seconds {
                now + seconds as libc::c_long
            } else {
                0i32 as libc::c_long
            }) as sqlite3_int64,
        );
        sqlite3_bind_int(stmt, 3i32, chat_id as libc::c_int);
        sqlite3_step(stmt);
        if 0 != seconds && 0 == is_sending_locations_before {
            msg = dc_msg_new(context, 10i32);
            (*msg).text = dc_stock_system_msg(
                context,
                64i32,
                0 as *const libc::c_char,
                0 as *const libc::c_char,
                0i32 as uint32_t,
            );
            dc_param_set_int((*msg).param, 'S' as i32, 8i32);
            dc_send_msg(context, chat_id, msg);
        } else if 0 == seconds && 0 != is_sending_locations_before {
            stock_str = dc_stock_system_msg(
                context,
                65i32,
                0 as *const libc::c_char,
                0 as *const libc::c_char,
                0i32 as uint32_t,
            );
            dc_add_device_msg(context, chat_id, stock_str);
        }
        (*context).cb.expect("non-null function pointer")(
            context,
            2020i32,
            chat_id as uintptr_t,
            0i32 as uintptr_t,
        );
        if 0 != seconds {
            schedule_MAYBE_SEND_LOCATIONS(context, 0i32);
            dc_job_add(
                context,
                5007i32,
                chat_id as libc::c_int,
                0 as *const libc::c_char,
                seconds + 1i32,
            );
        }
    }
    free(stock_str as *mut libc::c_void);
    dc_msg_unref(msg);
    sqlite3_finalize(stmt);
}
/* ******************************************************************************
 * job to send locations out to all chats that want them
 ******************************************************************************/
unsafe extern "C" fn schedule_MAYBE_SEND_LOCATIONS(
    mut context: *mut dc_context_t,
    mut flags: libc::c_int,
) {
    if 0 != flags & 0x1i32 || 0 == dc_job_action_exists(context, 5005i32) {
        dc_job_add(context, 5005i32, 0i32, 0 as *const libc::c_char, 60i32);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_sending_locations_to_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> libc::c_int {
    let mut is_sending_locations: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id  FROM chats  WHERE (? OR id=?)   AND locations_send_until>?;\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(
            stmt,
            1i32,
            if chat_id == 0i32 as libc::c_uint {
                1i32
            } else {
                0i32
            },
        );
        sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
        sqlite3_bind_int64(stmt, 3i32, time(0 as *mut time_t) as sqlite3_int64);
        if !(sqlite3_step(stmt) != 100i32) {
            is_sending_locations = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return is_sending_locations;
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_location(
    mut context: *mut dc_context_t,
    mut latitude: libc::c_double,
    mut longitude: libc::c_double,
    mut accuracy: libc::c_double,
) -> libc::c_int {
    let mut stmt_chats: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stmt_insert: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut continue_streaming: libc::c_int = 0i32;
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || latitude == 0.0f64 && longitude == 0.0f64
    {
        continue_streaming = 1i32
    } else {
        stmt_chats = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id FROM chats WHERE locations_send_until>?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int64(stmt_chats, 1i32, time(0 as *mut time_t) as sqlite3_int64);
        while sqlite3_step(stmt_chats) == 100i32 {
            let mut chat_id: uint32_t = sqlite3_column_int(stmt_chats, 0i32) as uint32_t;
            stmt_insert =
                dc_sqlite3_prepare((*context).sql,
                                   b"INSERT INTO locations  (latitude, longitude, accuracy, timestamp, chat_id, from_id) VALUES (?,?,?,?,?,?);\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_double(stmt_insert, 1i32, latitude);
            sqlite3_bind_double(stmt_insert, 2i32, longitude);
            sqlite3_bind_double(stmt_insert, 3i32, accuracy);
            sqlite3_bind_int64(stmt_insert, 4i32, time(0 as *mut time_t) as sqlite3_int64);
            sqlite3_bind_int(stmt_insert, 5i32, chat_id as libc::c_int);
            sqlite3_bind_int(stmt_insert, 6i32, 1i32);
            sqlite3_step(stmt_insert);
            continue_streaming = 1i32
        }
        if 0 != continue_streaming {
            (*context).cb.expect("non-null function pointer")(
                context,
                2035i32,
                1i32 as uintptr_t,
                0i32 as uintptr_t,
            );
            schedule_MAYBE_SEND_LOCATIONS(context, 0i32);
        }
    }
    sqlite3_finalize(stmt_chats);
    sqlite3_finalize(stmt_insert);
    return continue_streaming;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_locations(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut contact_id: uint32_t,
    mut timestamp_from: time_t,
    mut timestamp_to: time_t,
) -> *mut dc_array_t {
    let mut ret: *mut dc_array_t = dc_array_new_typed(context, 1i32, 500i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if timestamp_to == 0i32 as libc::c_long {
            timestamp_to = time(0 as *mut time_t) + 10i32 as libc::c_long
        }
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT l.id, l.latitude, l.longitude, l.accuracy, l.timestamp,        m.id, l.from_id, l.chat_id, m.txt  FROM locations l  LEFT JOIN msgs m ON l.id=m.location_id  WHERE (? OR l.chat_id=?)    AND (? OR l.from_id=?)    AND l.timestamp>=? AND l.timestamp<=?  ORDER BY l.timestamp DESC, l.id DESC, m.id DESC;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(
            stmt,
            1i32,
            if chat_id == 0i32 as libc::c_uint {
                1i32
            } else {
                0i32
            },
        );
        sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
        sqlite3_bind_int(
            stmt,
            3i32,
            if contact_id == 0i32 as libc::c_uint {
                1i32
            } else {
                0i32
            },
        );
        sqlite3_bind_int(stmt, 4i32, contact_id as libc::c_int);
        sqlite3_bind_int(stmt, 5i32, timestamp_from as libc::c_int);
        sqlite3_bind_int(stmt, 6i32, timestamp_to as libc::c_int);
        while sqlite3_step(stmt) == 100i32 {
            let mut loc: *mut _dc_location = calloc(
                1i32 as libc::c_ulong,
                ::std::mem::size_of::<_dc_location>() as libc::c_ulong,
            ) as *mut _dc_location;
            if loc.is_null() {
                break;
            }
            (*loc).location_id = sqlite3_column_double(stmt, 0i32) as uint32_t;
            (*loc).latitude = sqlite3_column_double(stmt, 1i32);
            (*loc).longitude = sqlite3_column_double(stmt, 2i32);
            (*loc).accuracy = sqlite3_column_double(stmt, 3i32);
            (*loc).timestamp = sqlite3_column_int64(stmt, 4i32) as time_t;
            (*loc).msg_id = sqlite3_column_int(stmt, 5i32) as uint32_t;
            (*loc).contact_id = sqlite3_column_int(stmt, 6i32) as uint32_t;
            (*loc).chat_id = sqlite3_column_int(stmt, 7i32) as uint32_t;
            if 0 != (*loc).msg_id {
                let mut txt: *const libc::c_char =
                    sqlite3_column_text(stmt, 8i32) as *const libc::c_char;
                if 0 != is_marker(txt) {
                    (*loc).marker = strdup(txt)
                }
            }
            dc_array_add_ptr(ret, loc as *mut libc::c_void);
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
unsafe extern "C" fn is_marker(mut txt: *const libc::c_char) -> libc::c_int {
    if !txt.is_null() {
        let mut len: libc::c_int = dc_utf8_strlen(txt) as libc::c_int;
        if len == 1i32 && *txt.offset(0isize) as libc::c_int != ' ' as i32 {
            return 1i32;
        }
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_delete_all_locations(mut context: *mut dc_context_t) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"DELETE FROM locations;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_step(stmt);
        (*context).cb.expect("non-null function pointer")(
            context,
            2035i32,
            0i32 as uintptr_t,
            0i32 as uintptr_t,
        );
    }
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_location_kml(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut last_added_location_id: *mut uint32_t,
) -> *mut libc::c_char {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut now: time_t = time(0 as *mut time_t);
    let mut locations_send_begin: time_t = 0i32 as time_t;
    let mut locations_send_until: time_t = 0i32 as time_t;
    let mut locations_last_sent: time_t = 0i32 as time_t;
    let mut location_count: libc::c_int = 0i32;
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 1000i32);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT locations_send_begin, locations_send_until, locations_last_sent  FROM chats  WHERE id=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        if !(sqlite3_step(stmt) != 100i32) {
            locations_send_begin = sqlite3_column_int64(stmt, 0i32) as time_t;
            locations_send_until = sqlite3_column_int64(stmt, 1i32) as time_t;
            locations_last_sent = sqlite3_column_int64(stmt, 2i32) as time_t;
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            if !(locations_send_begin == 0i32 as libc::c_long || now > locations_send_until) {
                dc_strbuilder_catf(&mut ret as *mut dc_strbuilder_t,
                                   b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document addr=\"%s\">\n\x00"
                                       as *const u8 as *const libc::c_char,
                                   self_addr);
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"SELECT id, latitude, longitude, accuracy, timestamp  FROM locations  WHERE from_id=?    AND timestamp>=?    AND (timestamp>=? OR timestamp=(SELECT MAX(timestamp) FROM locations WHERE from_id=?))    GROUP BY timestamp    ORDER BY timestamp;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, 1i32);
                sqlite3_bind_int64(stmt, 2i32, locations_send_begin as sqlite3_int64);
                sqlite3_bind_int64(stmt, 3i32, locations_last_sent as sqlite3_int64);
                sqlite3_bind_int(stmt, 4i32, 1i32);
                while sqlite3_step(stmt) == 100i32 {
                    let mut location_id: uint32_t = sqlite3_column_int(stmt, 0i32) as uint32_t;
                    let mut latitude: *mut libc::c_char =
                        dc_ftoa(sqlite3_column_double(stmt, 1i32));
                    let mut longitude: *mut libc::c_char =
                        dc_ftoa(sqlite3_column_double(stmt, 2i32));
                    let mut accuracy: *mut libc::c_char =
                        dc_ftoa(sqlite3_column_double(stmt, 3i32));
                    let mut timestamp: *mut libc::c_char =
                        get_kml_timestamp(sqlite3_column_int64(stmt, 4i32) as time_t);
                    dc_strbuilder_catf(&mut ret as *mut dc_strbuilder_t,
                                       b"<Placemark><Timestamp><when>%s</when></Timestamp><Point><coordinates accuracy=\"%s\">%s,%s</coordinates></Point></Placemark>\n\x00"
                                           as *const u8 as
                                           *const libc::c_char, timestamp,
                                       accuracy, longitude, latitude);
                    location_count += 1;
                    if !last_added_location_id.is_null() {
                        *last_added_location_id = location_id
                    }
                    free(latitude as *mut libc::c_void);
                    free(longitude as *mut libc::c_void);
                    free(accuracy as *mut libc::c_void);
                    free(timestamp as *mut libc::c_void);
                }
                if !(location_count == 0i32) {
                    dc_strbuilder_cat(
                        &mut ret,
                        b"</Document>\n</kml>\x00" as *const u8 as *const libc::c_char,
                    );
                    success = 1i32
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    free(self_addr as *mut libc::c_void);
    if 0 == success {
        free(ret.buf as *mut libc::c_void);
    }
    return if 0 != success {
        ret.buf
    } else {
        0 as *mut libc::c_char
    };
}
/* ******************************************************************************
 * create kml-files
 ******************************************************************************/
unsafe extern "C" fn get_kml_timestamp(mut utc: time_t) -> *mut libc::c_char {
    // Returns a string formatted as YYYY-MM-DDTHH:MM:SSZ. The trailing `Z` indicates UTC.
    let mut wanted_struct: tm = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: 0 as *mut libc::c_char,
    };
    memcpy(
        &mut wanted_struct as *mut tm as *mut libc::c_void,
        gmtime(&mut utc) as *const libc::c_void,
        ::std::mem::size_of::<tm>() as libc::c_ulong,
    );
    return dc_mprintf(
        b"%04i-%02i-%02iT%02i:%02i:%02iZ\x00" as *const u8 as *const libc::c_char,
        wanted_struct.tm_year as libc::c_int + 1900i32,
        wanted_struct.tm_mon as libc::c_int + 1i32,
        wanted_struct.tm_mday as libc::c_int,
        wanted_struct.tm_hour as libc::c_int,
        wanted_struct.tm_min as libc::c_int,
        wanted_struct.tm_sec as libc::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_kml_sent_timestamp(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut timestamp: time_t,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE chats SET locations_last_sent=? WHERE id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_msg_location_id(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
    mut location_id: uint32_t,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE msgs SET location_id=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, location_id as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, msg_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_save_locations(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut contact_id: uint32_t,
    mut locations: *const dc_array_t,
) -> uint32_t {
    let mut stmt_test: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stmt_insert: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut newest_timestamp: time_t = 0i32 as time_t;
    let mut newest_location_id: uint32_t = 0i32 as uint32_t;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint
        || locations.is_null())
    {
        stmt_test = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id FROM locations WHERE timestamp=? AND from_id=?\x00" as *const u8
                as *const libc::c_char,
        );
        stmt_insert =
            dc_sqlite3_prepare((*context).sql,
                               b"INSERT INTO locations  (timestamp, from_id, chat_id, latitude, longitude, accuracy) VALUES (?,?,?,?,?,?);\x00"
                                   as *const u8 as *const libc::c_char);
        let mut i: libc::c_int = 0i32;
        while (i as libc::c_ulong) < dc_array_get_cnt(locations) {
            let mut location: *mut dc_location_t =
                dc_array_get_ptr(locations, i as size_t) as *mut dc_location_t;
            sqlite3_reset(stmt_test);
            sqlite3_bind_int64(stmt_test, 1i32, (*location).timestamp as sqlite3_int64);
            sqlite3_bind_int(stmt_test, 2i32, contact_id as libc::c_int);
            if sqlite3_step(stmt_test) != 100i32 {
                sqlite3_reset(stmt_insert);
                sqlite3_bind_int64(stmt_insert, 1i32, (*location).timestamp as sqlite3_int64);
                sqlite3_bind_int(stmt_insert, 2i32, contact_id as libc::c_int);
                sqlite3_bind_int(stmt_insert, 3i32, chat_id as libc::c_int);
                sqlite3_bind_double(stmt_insert, 4i32, (*location).latitude);
                sqlite3_bind_double(stmt_insert, 5i32, (*location).longitude);
                sqlite3_bind_double(stmt_insert, 6i32, (*location).accuracy);
                sqlite3_step(stmt_insert);
            }
            if (*location).timestamp > newest_timestamp {
                newest_timestamp = (*location).timestamp;
                newest_location_id = dc_sqlite3_get_rowid2(
                    (*context).sql,
                    b"locations\x00" as *const u8 as *const libc::c_char,
                    b"timestamp\x00" as *const u8 as *const libc::c_char,
                    (*location).timestamp as uint64_t,
                    b"from_id\x00" as *const u8 as *const libc::c_char,
                    contact_id,
                )
            }
            i += 1
        }
    }
    sqlite3_finalize(stmt_test);
    sqlite3_finalize(stmt_insert);
    return newest_location_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_kml_parse(
    mut context: *mut dc_context_t,
    mut content: *const libc::c_char,
    mut content_bytes: size_t,
) -> *mut dc_kml_t {
    let mut kml: *mut dc_kml_t = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_kml_t>() as libc::c_ulong,
    ) as *mut dc_kml_t;
    let mut content_nullterminated: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut saxparser: dc_saxparser_t = _dc_saxparser {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if content_bytes > (1i32 * 1024i32 * 1024i32) as libc::c_ulong {
            dc_log_warning(
                context,
                0i32,
                b"A kml-files with %i bytes is larger than reasonably expected.\x00" as *const u8
                    as *const libc::c_char,
                content_bytes,
            );
        } else {
            content_nullterminated = dc_null_terminate(content, content_bytes as libc::c_int);
            if !content_nullterminated.is_null() {
                (*kml).locations = dc_array_new_typed(context, 1i32, 100i32 as size_t);
                dc_saxparser_init(&mut saxparser, kml as *mut libc::c_void);
                dc_saxparser_set_tag_handler(
                    &mut saxparser,
                    Some(kml_starttag_cb),
                    Some(kml_endtag_cb),
                );
                dc_saxparser_set_text_handler(&mut saxparser, Some(kml_text_cb));
                dc_saxparser_parse(&mut saxparser, content_nullterminated);
            }
        }
    }
    free(content_nullterminated as *mut libc::c_void);
    return kml;
}
unsafe extern "C" fn kml_text_cb(
    mut userdata: *mut libc::c_void,
    mut text: *const libc::c_char,
    mut len: libc::c_int,
) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if 0 != (*kml).tag & (0x4i32 | 0x10i32) {
        let mut val: *mut libc::c_char = dc_strdup(text);
        dc_str_replace(
            &mut val,
            b"\n\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b"\r\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b"\t\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b" \x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != (*kml).tag & 0x4i32 && strlen(val) >= 19i32 as libc::c_ulong {
            let mut tmval: tm = tm {
                tm_sec: 0,
                tm_min: 0,
                tm_hour: 0,
                tm_mday: 0,
                tm_mon: 0,
                tm_year: 0,
                tm_wday: 0,
                tm_yday: 0,
                tm_isdst: 0,
                tm_gmtoff: 0,
                tm_zone: 0 as *mut libc::c_char,
            };
            memset(
                &mut tmval as *mut tm as *mut libc::c_void,
                0i32,
                ::std::mem::size_of::<tm>() as libc::c_ulong,
            );
            *val.offset(4isize) = 0i32 as libc::c_char;
            tmval.tm_year = atoi(val) - 1900i32;
            *val.offset(7isize) = 0i32 as libc::c_char;
            tmval.tm_mon = atoi(val.offset(5isize)) - 1i32;
            *val.offset(10isize) = 0i32 as libc::c_char;
            tmval.tm_mday = atoi(val.offset(8isize));
            *val.offset(13isize) = 0i32 as libc::c_char;
            tmval.tm_hour = atoi(val.offset(11isize));
            *val.offset(16isize) = 0i32 as libc::c_char;
            tmval.tm_min = atoi(val.offset(14isize));
            *val.offset(19isize) = 0i32 as libc::c_char;
            tmval.tm_sec = atoi(val.offset(17isize));
            (*kml).curr.timestamp = mkgmtime(&mut tmval);
            if (*kml).curr.timestamp > time(0 as *mut time_t) {
                (*kml).curr.timestamp = time(0 as *mut time_t)
            }
        } else if 0 != (*kml).tag & 0x10i32 {
            let mut comma: *mut libc::c_char = strchr(val, ',' as i32);
            if !comma.is_null() {
                let mut longitude: *mut libc::c_char = val;
                let mut latitude: *mut libc::c_char = comma.offset(1isize);
                *comma = 0i32 as libc::c_char;
                comma = strchr(latitude, ',' as i32);
                if !comma.is_null() {
                    *comma = 0i32 as libc::c_char
                }
                (*kml).curr.latitude = dc_atof(latitude);
                (*kml).curr.longitude = dc_atof(longitude)
            }
        }
        free(val as *mut libc::c_void);
    };
}
unsafe extern "C" fn kml_endtag_cb(mut userdata: *mut libc::c_void, mut tag: *const libc::c_char) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if 0 != (*kml).tag & 0x1i32
            && 0 != (*kml).curr.timestamp
            && 0. != (*kml).curr.latitude
            && 0. != (*kml).curr.longitude
        {
            let mut location: *mut dc_location_t = calloc(
                1i32 as libc::c_ulong,
                ::std::mem::size_of::<dc_location_t>() as libc::c_ulong,
            ) as *mut dc_location_t;
            *location = (*kml).curr;
            dc_array_add_ptr((*kml).locations, location as *mut libc::c_void);
        }
        (*kml).tag = 0i32
    };
}
/* ******************************************************************************
 * parse kml-files
 ******************************************************************************/
unsafe extern "C" fn kml_starttag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
    mut attr: *mut *mut libc::c_char,
) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if strcmp(tag, b"document\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let mut addr: *const libc::c_char =
            dc_attr_find(attr, b"addr\x00" as *const u8 as *const libc::c_char);
        if !addr.is_null() {
            (*kml).addr = dc_strdup(addr)
        }
    } else if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*kml).tag = 0x1i32;
        (*kml).curr.timestamp = 0i32 as time_t;
        (*kml).curr.latitude = 0i32 as libc::c_double;
        (*kml).curr.longitude = 0.0f64;
        (*kml).curr.accuracy = 0.0f64
    } else if strcmp(tag, b"timestamp\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x1i32
    {
        (*kml).tag = 0x1i32 | 0x2i32
    } else if strcmp(tag, b"when\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x2i32
    {
        (*kml).tag = 0x1i32 | 0x2i32 | 0x4i32
    } else if strcmp(tag, b"point\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x1i32
    {
        (*kml).tag = 0x1i32 | 0x8i32
    } else if strcmp(tag, b"coordinates\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x8i32
    {
        (*kml).tag = 0x1i32 | 0x8i32 | 0x10i32;
        let mut accuracy: *const libc::c_char =
            dc_attr_find(attr, b"accuracy\x00" as *const u8 as *const libc::c_char);
        if !accuracy.is_null() {
            (*kml).curr.accuracy = dc_atof(accuracy)
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_kml_unref(mut kml: *mut dc_kml_t) {
    if kml.is_null() {
        return;
    }
    dc_array_unref((*kml).locations);
    free((*kml).addr as *mut libc::c_void);
    free(kml as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_job_do_DC_JOB_MAYBE_SEND_LOCATIONS(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut stmt_chats: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stmt_locations: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut now: time_t = time(0 as *mut time_t);
    let mut continue_streaming: libc::c_int = 1i32;
    dc_log_info(
        context,
        0i32,
        b" ----------------- MAYBE_SEND_LOCATIONS -------------- \x00" as *const u8
            as *const libc::c_char,
    );
    stmt_chats =
        dc_sqlite3_prepare((*context).sql,
                           b"SELECT id, locations_send_begin, locations_last_sent   FROM chats   WHERE locations_send_until>?;\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int64(stmt_chats, 1i32, now as sqlite3_int64);
    while sqlite3_step(stmt_chats) == 100i32 {
        let mut chat_id: uint32_t = sqlite3_column_int(stmt_chats, 0i32) as uint32_t;
        let mut locations_send_begin: time_t = sqlite3_column_int64(stmt_chats, 1i32) as time_t;
        let mut locations_last_sent: time_t = sqlite3_column_int64(stmt_chats, 2i32) as time_t;
        continue_streaming = 1i32;
        // be a bit tolerant as the timer may not align exactly with time(NULL)
        if now - locations_last_sent < (60i32 - 3i32) as libc::c_long {
            continue;
        }
        if stmt_locations.is_null() {
            stmt_locations =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT id  FROM locations  WHERE from_id=?    AND timestamp>=?    AND timestamp>?    ORDER BY timestamp;\x00"
                                       as *const u8 as *const libc::c_char)
        } else {
            sqlite3_reset(stmt_locations);
        }
        sqlite3_bind_int(stmt_locations, 1i32, 1i32);
        sqlite3_bind_int64(stmt_locations, 2i32, locations_send_begin as sqlite3_int64);
        sqlite3_bind_int64(stmt_locations, 3i32, locations_last_sent as sqlite3_int64);
        // if there is no new location, there's nothing to send.
        // however, maybe we want to bypass this test eg. 15 minutes
        if sqlite3_step(stmt_locations) != 100i32 {
            continue;
        }
        // pending locations are attached automatically to every message,
        // so also to this empty text message.
        // DC_CMD_LOCATION is only needed to create a nicer subject.
        //
        // for optimisation and to avoid flooding the sending queue,
        // we could sending these messages only if we're really online.
        // the easiest way to determine this, is to check for an empty message queue.
        // (might not be 100%, however, as positions are sent combined later
        // and dc_set_location() is typically called periodically, this is ok)
        let mut msg: *mut dc_msg_t = dc_msg_new(context, 10i32);
        (*msg).hidden = 1i32;
        dc_param_set_int((*msg).param, 'S' as i32, 9i32);
        dc_send_msg(context, chat_id, msg);
        dc_msg_unref(msg);
    }
    if 0 != continue_streaming {
        schedule_MAYBE_SEND_LOCATIONS(context, 0x1i32);
    }
    sqlite3_finalize(stmt_chats);
    sqlite3_finalize(stmt_locations);
}
#[no_mangle]
pub unsafe extern "C" fn dc_job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    // this function is called when location-streaming _might_ have ended for a chat.
    // the function checks, if location-streaming is really ended;
    // if so, a device-message is added if not yet done.
    let mut chat_id: uint32_t = (*job).foreign_id;
    let mut locations_send_begin: time_t = 0i32 as time_t;
    let mut locations_send_until: time_t = 0i32 as time_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stock_str: *mut libc::c_char = 0 as *mut libc::c_char;
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT locations_send_begin, locations_send_until  FROM chats  WHERE id=?\x00"
            as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(stmt) != 100i32) {
        locations_send_begin = sqlite3_column_int64(stmt, 0i32) as time_t;
        locations_send_until = sqlite3_column_int64(stmt, 1i32) as time_t;
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if !(locations_send_begin != 0i32 as libc::c_long
            && time(0 as *mut time_t) <= locations_send_until)
        {
            // still streaming -
            // may happen as several calls to dc_send_locations_to_chat()
            // do not un-schedule pending DC_MAYBE_SEND_LOC_ENDED jobs
            if !(locations_send_begin == 0i32 as libc::c_long
                && locations_send_until == 0i32 as libc::c_long)
            {
                // not streaming, device-message already sent
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"UPDATE chats    SET locations_send_begin=0, locations_send_until=0  WHERE id=?\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                sqlite3_step(stmt);
                stock_str = dc_stock_system_msg(
                    context,
                    65i32,
                    0 as *const libc::c_char,
                    0 as *const libc::c_char,
                    0i32 as uint32_t,
                );
                dc_add_device_msg(context, chat_id, stock_str);
                (*context).cb.expect("non-null function pointer")(
                    context,
                    2020i32,
                    chat_id as uintptr_t,
                    0i32 as uintptr_t,
                );
            }
        }
    }
    sqlite3_finalize(stmt);
    free(stock_str as *mut libc::c_void);
}

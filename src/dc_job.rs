use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_context::dc_context_t;
use crate::dc_imap::dc_imap_t;
use crate::dc_jobthread::dc_jobthread_t;
use crate::dc_smtp::dc_smtp_t;
use crate::types::*;

extern "C" {
    #[no_mangle]
    fn usleep(_: libc::useconds_t) -> libc::c_int;
    #[no_mangle]
    fn pow(_: libc::c_double, _: libc::c_double) -> libc::c_double;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn rand() -> libc::c_int;
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn clock() -> libc::clock_t;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn pthread_cond_signal(_: *mut pthread_cond_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_cond_timedwait(
        _: *mut pthread_cond_t,
        _: *mut pthread_mutex_t,
        _: *const timespec,
    ) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_lock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_unlock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn clist_free(_: *mut clist);
    #[no_mangle]
    fn clist_insert_after(_: *mut clist, _: *mut clistiter, _: *mut libc::c_void) -> libc::c_int;
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_param_unref(_: *mut dc_param_t);
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_set_msg_failed(_: *mut dc_context_t, msg_id: uint32_t, error: *const libc::c_char);
    #[no_mangle]
    fn sqlite3_bind_text(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_char,
        _: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int64(_: *mut sqlite3_stmt, _: libc::c_int, _: sqlite3_int64) -> libc::c_int;
    #[no_mangle]
    fn dc_jobthread_suspend(_: *mut dc_jobthread_t, suspend: libc::c_int);
    /* housekeeping */
    #[no_mangle]
    fn dc_housekeeping(_: *mut dc_context_t);
    #[no_mangle]
    fn dc_job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(_: *mut dc_context_t, _: *mut dc_job_t);
    #[no_mangle]
    fn dc_job_do_DC_JOB_MAYBE_SEND_LOCATIONS(_: *mut dc_context_t, _: *mut dc_job_t);
    #[no_mangle]
    fn dc_job_do_DC_JOB_IMEX_IMAP(_: *mut dc_context_t, _: *mut dc_job_t);
    // the other dc_job_do_DC_JOB_*() functions are declared static in the c-file
    #[no_mangle]
    fn dc_job_do_DC_JOB_CONFIGURE_IMAP(_: *mut dc_context_t, _: *mut dc_job_t);
    /* clist tools */
    #[no_mangle]
    fn clist_free_content(_: *const clist);
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_update_msg_state(_: *mut dc_context_t, msg_id: uint32_t, state: libc::c_int);
    #[no_mangle]
    fn dc_delete_file(_: *mut dc_context_t, pathNFilename: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_smtp_disconnect(_: *mut dc_smtp_t);
    #[no_mangle]
    fn dc_smtp_send_msg(
        _: *mut dc_smtp_t,
        recipients: *const clist,
        data: *const libc::c_char,
        data_bytes: size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    /* as we do not cut inside words, this results in about 32-42 characters.
    Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
    It should also be very clear, the subject is _not_ the whole message.
    The value is also used for CC:-summaries */
    // Context functions to work with messages
    #[no_mangle]
    fn dc_msg_exists(_: *mut dc_context_t, msg_id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_str_to_clist(_: *const libc::c_char, delimiter: *const libc::c_char) -> *mut clist;
    #[no_mangle]
    fn dc_param_get(
        _: *const dc_param_t,
        key: libc::c_int,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_read_file(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
        buf: *mut *mut libc::c_void,
        buf_bytes: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_loginparam_new() -> *mut dc_loginparam_t;
    #[no_mangle]
    fn dc_smtp_connect(_: *mut dc_smtp_t, _: *const dc_loginparam_t) -> libc::c_int;
    #[no_mangle]
    fn dc_loginparam_unref(_: *mut dc_loginparam_t);
    #[no_mangle]
    fn dc_loginparam_read(
        _: *mut dc_loginparam_t,
        _: *mut dc_sqlite3_t,
        prefix: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_smtp_is_connected(_: *const dc_smtp_t) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_new_untyped(_: *mut dc_context_t) -> *mut dc_msg_t;
    #[no_mangle]
    fn dc_msg_unref(_: *mut dc_msg_t);
    #[no_mangle]
    fn dc_update_server_uid(
        _: *mut dc_context_t,
        rfc724_mid: *const libc::c_char,
        server_folder: *const libc::c_char,
        server_uid: uint32_t,
    );
    #[no_mangle]
    fn dc_imap_move(
        _: *mut dc_imap_t,
        folder: *const libc::c_char,
        uid: uint32_t,
        dest_folder: *const libc::c_char,
        dest_uid: *mut uint32_t,
    ) -> dc_imap_res;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_configure_folders(_: *mut dc_context_t, _: *mut dc_imap_t, flags: libc::c_int);
    #[no_mangle]
    fn dc_sqlite3_get_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: int32_t,
    ) -> int32_t;
    #[no_mangle]
    fn dc_msg_load_from_db(_: *mut dc_msg_t, _: *mut dc_context_t, id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_is_connected(_: *const dc_imap_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_set_watch_folder(_: *mut dc_imap_t, watch_folder: *const libc::c_char);
    #[no_mangle]
    fn dc_connect_to_configured_imap(_: *mut dc_context_t, _: *mut dc_imap_t) -> libc::c_int;
    #[no_mangle]
    fn dc_param_get_int(_: *const dc_param_t, key: libc::c_int, def: int32_t) -> int32_t;
    #[no_mangle]
    fn dc_imap_set_seen(
        _: *mut dc_imap_t,
        folder: *const libc::c_char,
        uid: uint32_t,
    ) -> dc_imap_res;
    #[no_mangle]
    fn dc_mimefactory_empty(_: *mut dc_mimefactory_t);
    /* library-private */
    #[no_mangle]
    fn dc_param_new() -> *mut dc_param_t;
    #[no_mangle]
    fn dc_imap_interrupt_idle(_: *mut dc_imap_t);
    #[no_mangle]
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
    #[no_mangle]
    fn dc_str_from_clist(_: *const clist, delimiter: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_write_file(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
        buf: *const libc::c_void,
        buf_bytes: size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_get_fine_pathNfilename(
        _: *mut dc_context_t,
        pathNfolder: *const libc::c_char,
        desired_name: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_mimefactory_render(_: *mut dc_mimefactory_t) -> libc::c_int;
    #[no_mangle]
    fn dc_mimefactory_load_mdn(_: *mut dc_mimefactory_t, msg_id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_mimefactory_init(_: *mut dc_mimefactory_t, _: *mut dc_context_t);
    #[no_mangle]
    fn dc_imap_set_mdnsent(
        _: *mut dc_imap_t,
        folder: *const libc::c_char,
        uid: uint32_t,
    ) -> dc_imap_res;
    #[no_mangle]
    fn dc_delete_msg_from_db(_: *mut dc_context_t, _: uint32_t);
    #[no_mangle]
    fn dc_imap_delete_msg(
        _: *mut dc_imap_t,
        rfc724_mid: *const libc::c_char,
        folder: *const libc::c_char,
        server_uid: uint32_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_rfc724_mid_cnt(_: *mut dc_context_t, rfc724_mid: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_int64(_: *mut sqlite3_stmt, iCol: libc::c_int) -> sqlite3_int64;
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    #[no_mangle]
    fn dc_param_set_packed(_: *mut dc_param_t, _: *const libc::c_char);
    #[no_mangle]
    fn dc_imap_fetch(_: *mut dc_imap_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_idle(_: *mut dc_imap_t);
    #[no_mangle]
    fn dc_jobthread_fetch(_: *mut dc_jobthread_t, use_network: libc::c_int);
    #[no_mangle]
    fn dc_jobthread_idle(_: *mut dc_jobthread_t, use_network: libc::c_int);
    #[no_mangle]
    fn dc_jobthread_interrupt_idle(_: *mut dc_jobthread_t);
    #[no_mangle]
    fn dc_sqlite3_begin_transaction(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_commit(_: *mut dc_sqlite3_t);
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn clist_search_string_nocase(_: *const clist, str: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_get_filemeta(
        buf: *const libc::c_void,
        buf_bytes: size_t,
        ret_width: *mut uint32_t,
        ret_height: *mut uint32_t,
    ) -> libc::c_int;
    /* for msgs and jobs */
    /* for msgs */
    /* for msgs */
    /* for msgs */
    /* for msgs */
    /* for msgs: incoming: message is encryoted, outgoing: guarantee E2EE or the message is not send */
    /* for msgs: decrypted with validation errors or without mutual set, if neither 'c' nor 'e' are preset, the messages is only transport encrypted */
    /* for msgs: force unencrypted message, either DC_FP_ADD_AUTOCRYPT_HEADER (1), DC_FP_NO_AUTOCRYPT_HEADER (2) or 0 */
    /* for msgs: an incoming message which requestes a MDN (aka read receipt) */
    /* for msgs */
    /* for msgs */
    /* for msgs */
    /* for msgs */
    /* for msgs */
    /* for msgs */
    /* for msgs */
    /* for msgs in PREPARING: space-separated list of message IDs of forwarded copies */
    /* for jobs */
    /* for jobs */
    /* for jobs */
    /* for jobs: space-separated list of message recipients */
    /* for groups */
    /* for groups and contacts */
    /* for chats */
    // values for DC_PARAM_FORCE_PLAINTEXT
    /* user functions */
    #[no_mangle]
    fn dc_param_exists(_: *mut dc_param_t, key: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
    #[no_mangle]
    fn dc_set_gossiped_timestamp(_: *mut dc_context_t, chat_id: uint32_t, _: time_t);
    #[no_mangle]
    fn dc_msg_save_param_to_disk(_: *mut dc_msg_t);
    /* yes: uppercase */
    /* library private: key-history */
    #[no_mangle]
    fn dc_add_to_keyhistory(
        _: *mut dc_context_t,
        rfc724_mid: *const libc::c_char,
        _: time_t,
        addr: *const libc::c_char,
        fingerprint: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_set_msg_location_id(_: *mut dc_context_t, msg_id: uint32_t, location_id: uint32_t);
    #[no_mangle]
    fn dc_set_kml_sent_timestamp(_: *mut dc_context_t, chat_id: uint32_t, _: time_t);
    #[no_mangle]
    fn dc_mimefactory_load_msg(_: *mut dc_mimefactory_t, msg_id: uint32_t) -> libc::c_int;
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
pub type unnamed = libc::c_uint;
pub const MAILSMTP_ERROR_CLIENTID_NOT_SUPPORTED: unnamed = 28;
pub const MAILSMTP_ERROR_SSL: unnamed = 27;
pub const MAILSMTP_ERROR_AUTH_AUTHENTICATION_FAILED: unnamed = 26;
pub const MAILSMTP_ERROR_CONNECTION_REFUSED: unnamed = 25;
pub const MAILSMTP_ERROR_STARTTLS_NOT_SUPPORTED: unnamed = 24;
pub const MAILSMTP_ERROR_STARTTLS_TEMPORARY_FAILURE: unnamed = 23;
pub const MAILSMTP_ERROR_AUTH_ENCRYPTION_REQUIRED: unnamed = 22;
pub const MAILSMTP_ERROR_AUTH_TEMPORARY_FAILTURE: unnamed = 21;
pub const MAILSMTP_ERROR_AUTH_TRANSITION_NEEDED: unnamed = 20;
pub const MAILSMTP_ERROR_AUTH_TOO_WEAK: unnamed = 19;
pub const MAILSMTP_ERROR_AUTH_REQUIRED: unnamed = 18;
pub const MAILSMTP_ERROR_AUTH_LOGIN: unnamed = 17;
pub const MAILSMTP_ERROR_AUTH_NOT_SUPPORTED: unnamed = 16;
pub const MAILSMTP_ERROR_MEMORY: unnamed = 15;
pub const MAILSMTP_ERROR_TRANSACTION_FAILED: unnamed = 14;
pub const MAILSMTP_ERROR_USER_NOT_LOCAL: unnamed = 13;
pub const MAILSMTP_ERROR_BAD_SEQUENCE_OF_COMMAND: unnamed = 12;
pub const MAILSMTP_ERROR_MAILBOX_NAME_NOT_ALLOWED: unnamed = 11;
pub const MAILSMTP_ERROR_MAILBOX_UNAVAILABLE: unnamed = 10;
pub const MAILSMTP_ERROR_INSUFFICIENT_SYSTEM_STORAGE: unnamed = 9;
pub const MAILSMTP_ERROR_IN_PROCESSING: unnamed = 8;
pub const MAILSMTP_ERROR_EXCEED_STORAGE_ALLOCATION: unnamed = 7;
pub const MAILSMTP_ERROR_ACTION_NOT_TAKEN: unnamed = 6;
pub const MAILSMTP_ERROR_NOT_IMPLEMENTED: unnamed = 5;
pub const MAILSMTP_ERROR_HOSTNAME: unnamed = 4;
pub const MAILSMTP_ERROR_STREAM: unnamed = 3;
pub const MAILSMTP_ERROR_SERVICE_NOT_AVAILABLE: unnamed = 2;
pub const MAILSMTP_ERROR_UNEXPECTED_CODE: unnamed = 1;
pub const MAILSMTP_NO_ERROR: unnamed = 0;
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
//pub type dc_smtp_t = _dc_smtp;
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
//pub type dc_jobthread_t = _dc_jobthread;
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
/* values for the chats.blocked database field */
/* * the structure behind dc_chat_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_chat {
    pub magic: uint32_t,
    pub id: uint32_t,
    pub type_0: libc::c_int,
    pub name: *mut libc::c_char,
    pub archived: libc::c_int,
    pub context: *mut dc_context_t,
    pub grpid: *mut libc::c_char,
    pub blocked: libc::c_int,
    pub param: *mut dc_param_t,
    pub gossiped_timestamp: time_t,
    pub is_sending_locations: libc::c_int,
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
pub type dc_chat_t = _dc_chat;
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
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
pub type sqlite3_int64 = sqlite_int64;
pub type sqlite_int64 = libc::c_longlong;
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
pub const DC_SUCCESS: dc_imap_res = 3;
pub const DC_ALREADY_DONE: dc_imap_res = 2;
pub const DC_RETRY_LATER: dc_imap_res = 1;
pub const DC_FAILED: dc_imap_res = 0;
pub type dc_imap_res = libc::c_uint;
pub type dc_mimefactory_t = _dc_mimefactory;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_mimefactory {
    pub from_addr: *mut libc::c_char,
    pub from_displayname: *mut libc::c_char,
    pub selfstatus: *mut libc::c_char,
    pub recipients_names: *mut clist,
    pub recipients_addr: *mut clist,
    pub timestamp: time_t,
    pub rfc724_mid: *mut libc::c_char,
    pub loaded: dc_mimefactory_loaded_t,
    pub msg: *mut dc_msg_t,
    pub chat: *mut dc_chat_t,
    pub increation: libc::c_int,
    pub in_reply_to: *mut libc::c_char,
    pub references: *mut libc::c_char,
    pub req_mdn: libc::c_int,
    pub out: *mut MMAPString,
    pub out_encrypted: libc::c_int,
    pub out_gossiped: libc::c_int,
    pub out_last_added_location_id: uint32_t,
    pub error: *mut libc::c_char,
    pub context: *mut dc_context_t,
}
pub type dc_mimefactory_loaded_t = libc::c_uint;
pub const DC_MF_MDN_LOADED: dc_mimefactory_loaded_t = 2;
pub const DC_MF_MSG_LOADED: dc_mimefactory_loaded_t = 1;
pub const DC_MF_NOTHING_LOADED: dc_mimefactory_loaded_t = 0;
#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_jobs(mut context: *mut dc_context_t) {
    dc_log_info(
        context,
        0i32,
        b"INBOX-jobs started...\x00" as *const u8 as *const libc::c_char,
    );
    pthread_mutex_lock(&mut (*context).inboxidle_condmutex);
    let mut probe_imap_network: libc::c_int = (*context).probe_imap_network;
    (*context).probe_imap_network = 0i32;
    (*context).perform_inbox_jobs_needed = 0i32;
    pthread_mutex_unlock(&mut (*context).inboxidle_condmutex);
    dc_job_perform(context, 100i32, probe_imap_network);
    dc_log_info(
        context,
        0i32,
        b"INBOX-jobs ended.\x00" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn dc_job_perform(
    mut context: *mut dc_context_t,
    mut thread: libc::c_int,
    mut probe_network: libc::c_int,
) {
    let mut select_stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut job: dc_job_t = _dc_job {
        job_id: 0,
        action: 0,
        foreign_id: 0,
        desired_timestamp: 0,
        added_timestamp: 0,
        tries: 0,
        param: 0 as *mut dc_param_t,
        try_again: 0,
        pending_error: 0 as *mut libc::c_char,
    };
    memset(
        &mut job as *mut dc_job_t as *mut libc::c_void,
        0i32,
        ::std::mem::size_of::<dc_job_t>() as libc::c_ulong,
    );
    job.param = dc_param_new();
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if probe_network == 0i32 {
            select_stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries FROM jobs WHERE thread=? AND desired_timestamp<=? ORDER BY action DESC, added_timestamp;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int64(select_stmt, 1i32, thread as sqlite3_int64);
            sqlite3_bind_int64(select_stmt, 2i32, time(0 as *mut time_t) as sqlite3_int64);
        } else {
            select_stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries FROM jobs WHERE thread=? AND tries>0 ORDER BY desired_timestamp, action DESC;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int64(select_stmt, 1i32, thread as sqlite3_int64);
        }
        while sqlite3_step(select_stmt) == 100i32 {
            job.job_id = sqlite3_column_int(select_stmt, 0i32) as uint32_t;
            job.action = sqlite3_column_int(select_stmt, 1i32);
            job.foreign_id = sqlite3_column_int(select_stmt, 2i32) as uint32_t;
            dc_param_set_packed(
                job.param,
                sqlite3_column_text(select_stmt, 3i32) as *mut libc::c_char,
            );
            job.added_timestamp = sqlite3_column_int64(select_stmt, 4i32) as time_t;
            job.desired_timestamp = sqlite3_column_int64(select_stmt, 5i32) as time_t;
            job.tries = sqlite3_column_int(select_stmt, 6i32);
            dc_log_info(
                context,
                0i32,
                b"%s-job #%i, action %i started...\x00" as *const u8 as *const libc::c_char,
                if thread == 100i32 {
                    b"INBOX\x00" as *const u8 as *const libc::c_char
                } else {
                    b"SMTP\x00" as *const u8 as *const libc::c_char
                },
                job.job_id as libc::c_int,
                job.action as libc::c_int,
            );
            if 900i32 == job.action || 910i32 == job.action {
                dc_job_kill_action(context, job.action);
                sqlite3_finalize(select_stmt);
                select_stmt = 0 as *mut sqlite3_stmt;
                dc_jobthread_suspend(&mut (*context).sentbox_thread, 1i32);
                dc_jobthread_suspend(&mut (*context).mvbox_thread, 1i32);
                dc_suspend_smtp_thread(context, 1i32);
            }
            let mut tries: libc::c_int = 0i32;
            while tries <= 1i32 {
                job.try_again = 0i32;
                match job.action {
                    5901 => {
                        dc_job_do_DC_JOB_SEND(context, &mut job);
                    }
                    110 => {
                        dc_job_do_DC_JOB_DELETE_MSG_ON_IMAP(context, &mut job);
                    }
                    130 => {
                        dc_job_do_DC_JOB_MARKSEEN_MSG_ON_IMAP(context, &mut job);
                    }
                    120 => {
                        dc_job_do_DC_JOB_MARKSEEN_MDN_ON_IMAP(context, &mut job);
                    }
                    200 => {
                        dc_job_do_DC_JOB_MOVE_MSG(context, &mut job);
                    }
                    5011 => {
                        dc_job_do_DC_JOB_SEND(context, &mut job);
                    }
                    900 => {
                        dc_job_do_DC_JOB_CONFIGURE_IMAP(context, &mut job);
                    }
                    910 => {
                        dc_job_do_DC_JOB_IMEX_IMAP(context, &mut job);
                    }
                    5005 => {
                        dc_job_do_DC_JOB_MAYBE_SEND_LOCATIONS(context, &mut job);
                    }
                    5007 => {
                        dc_job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(context, &mut job);
                    }
                    105 => {
                        dc_housekeeping(context);
                    }
                    _ => {}
                }
                if job.try_again != -1i32 {
                    break;
                }
                tries += 1
            }
            if 900i32 == job.action || 910i32 == job.action {
                dc_jobthread_suspend(&mut (*context).sentbox_thread, 0i32);
                dc_jobthread_suspend(&mut (*context).mvbox_thread, 0i32);
                dc_suspend_smtp_thread(context, 0i32);
                break;
            } else if job.try_again == 2i32 {
                dc_log_info(
                    context,
                    0i32,
                    b"%s-job #%i not yet ready and will be delayed.\x00" as *const u8
                        as *const libc::c_char,
                    if thread == 100i32 {
                        b"INBOX\x00" as *const u8 as *const libc::c_char
                    } else {
                        b"SMTP\x00" as *const u8 as *const libc::c_char
                    },
                    job.job_id as libc::c_int,
                );
            } else if job.try_again == -1i32 || job.try_again == 3i32 {
                let mut tries_0: libc::c_int = job.tries + 1i32;
                if tries_0 < 17i32 {
                    job.tries = tries_0;
                    let mut time_offset: time_t = get_backoff_time_offset(tries_0);
                    job.desired_timestamp = job.added_timestamp + time_offset;
                    dc_job_update(context, &mut job);
                    dc_log_info(context, 0i32,
                                b"%s-job #%i not succeeded on try #%i, retry in ADD_TIME+%i (in %i seconds).\x00"
                                    as *const u8 as *const libc::c_char,
                                if thread == 100i32 {
                                    b"INBOX\x00" as *const u8 as
                                        *const libc::c_char
                                } else {
                                    b"SMTP\x00" as *const u8 as
                                        *const libc::c_char
                                }, job.job_id as libc::c_int, tries_0,
                                time_offset,
                                job.added_timestamp + time_offset -
                                    time(0 as *mut time_t));
                    if thread == 5000i32 && tries_0 < 17i32 - 1i32 {
                        pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
                        (*context).perform_smtp_jobs_needed = 2i32;
                        pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
                    }
                } else {
                    if job.action == 5901i32 {
                        dc_set_msg_failed(context, job.foreign_id, job.pending_error);
                    }
                    dc_job_delete(context, &mut job);
                }
                if !(0 != probe_network) {
                    continue;
                }
                // on dc_maybe_network() we stop trying here;
                // these jobs are already tried once.
                // otherwise, we just continue with the next job
                // to give other jobs a chance being tried at least once.
                break;
            } else {
                dc_job_delete(context, &mut job);
            }
        }
    }
    dc_param_unref(job.param);
    free(job.pending_error as *mut libc::c_void);
    sqlite3_finalize(select_stmt);
}
unsafe extern "C" fn dc_job_delete(mut context: *mut dc_context_t, mut job: *const dc_job_t) {
    let mut delete_stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"DELETE FROM jobs WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(delete_stmt, 1i32, (*job).job_id as libc::c_int);
    sqlite3_step(delete_stmt);
    sqlite3_finalize(delete_stmt);
}
/* ******************************************************************************
 * Tools
 ******************************************************************************/
unsafe extern "C" fn get_backoff_time_offset(mut c_tries: libc::c_int) -> time_t {
    // results in ~3 weeks for the last backoff timespan
    let mut N: time_t = pow(2i32 as libc::c_double, (c_tries - 1i32) as libc::c_double) as time_t;
    N = N * 60i32 as libc::c_long;
    let mut seconds: time_t = rand() as libc::c_long % (N + 1i32 as libc::c_long);
    if seconds < 1i32 as libc::c_long {
        seconds = 1i32 as time_t
    }
    return seconds;
}
unsafe extern "C" fn dc_job_update(mut context: *mut dc_context_t, mut job: *const dc_job_t) {
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE jobs SET desired_timestamp=?, tries=?, param=? WHERE id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, (*job).desired_timestamp as sqlite3_int64);
    sqlite3_bind_int64(stmt, 2i32, (*job).tries as sqlite3_int64);
    sqlite3_bind_text(stmt, 3i32, (*(*job).param).packed, -1i32, None);
    sqlite3_bind_int(stmt, 4i32, (*job).job_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
unsafe extern "C" fn dc_suspend_smtp_thread(
    mut context: *mut dc_context_t,
    mut suspend: libc::c_int,
) {
    pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
    (*context).smtp_suspended = suspend;
    pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
    if 0 != suspend {
        loop {
            pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
            if (*context).smtp_doing_jobs == 0i32 {
                pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
                return;
            }
            pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
            usleep((300i32 * 1000i32) as libc::useconds_t);
        }
    };
}
unsafe extern "C" fn dc_job_do_DC_JOB_SEND(mut context: *mut dc_context_t, mut job: *mut dc_job_t) {
    let mut current_block: u64;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut buf_bytes: size_t = 0i32 as size_t;
    let mut recipients: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut recipients_list: *mut clist = 0 as *mut clist;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    /* connect to SMTP server, if not yet done */
    if 0 == dc_smtp_is_connected((*context).smtp) {
        let mut loginparam: *mut dc_loginparam_t = dc_loginparam_new();
        dc_loginparam_read(
            loginparam,
            (*context).sql,
            b"configured_\x00" as *const u8 as *const libc::c_char,
        );
        let mut connected: libc::c_int = dc_smtp_connect((*context).smtp, loginparam);
        dc_loginparam_unref(loginparam);
        if 0 == connected {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 14216916617354591294;
        } else {
            current_block = 13109137661213826276;
        }
    } else {
        current_block = 13109137661213826276;
    }
    match current_block {
        13109137661213826276 => {
            filename = dc_param_get((*job).param, 'f' as i32, 0 as *const libc::c_char);
            if filename.is_null() {
                dc_log_warning(
                    context,
                    0i32,
                    b"Missing file name for job %d\x00" as *const u8 as *const libc::c_char,
                    (*job).job_id,
                );
            } else if !(0 == dc_read_file(context, filename, &mut buf, &mut buf_bytes)) {
                recipients = dc_param_get((*job).param, 'R' as i32, 0 as *const libc::c_char);
                if recipients.is_null() {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Missing recipients for job %d\x00" as *const u8 as *const libc::c_char,
                        (*job).job_id,
                    );
                } else {
                    recipients_list = dc_str_to_clist(
                        recipients,
                        b"\x1e\x00" as *const u8 as *const libc::c_char,
                    );
                    /* if there is a msg-id and it does not exist in the db, cancel sending.
                    this happends if dc_delete_msgs() was called
                    before the generated mime was sent out */
                    if 0 != (*job).foreign_id {
                        if 0 == dc_msg_exists(context, (*job).foreign_id) {
                            dc_log_warning(
                                context,
                                0i32,
                                b"Message %i for job %i does not exist\x00" as *const u8
                                    as *const libc::c_char,
                                (*job).foreign_id,
                                (*job).job_id,
                            );
                            current_block = 14216916617354591294;
                        } else {
                            current_block = 11194104282611034094;
                        }
                    } else {
                        current_block = 11194104282611034094;
                    }
                    match current_block {
                        14216916617354591294 => {}
                        _ => {
                            /* send message */
                            if 0 == dc_smtp_send_msg(
                                (*context).smtp,
                                recipients_list,
                                buf as *const libc::c_char,
                                buf_bytes,
                            ) {
                                if 0 != (*job).foreign_id
                                    && (MAILSMTP_ERROR_EXCEED_STORAGE_ALLOCATION as libc::c_int
                                        == (*(*context).smtp).error_etpan
                                        || MAILSMTP_ERROR_INSUFFICIENT_SYSTEM_STORAGE
                                            as libc::c_int
                                            == (*(*context).smtp).error_etpan)
                                {
                                    dc_set_msg_failed(
                                        context,
                                        (*job).foreign_id,
                                        (*(*context).smtp).error,
                                    );
                                } else {
                                    dc_smtp_disconnect((*context).smtp);
                                    dc_job_try_again_later(job, -1i32, (*(*context).smtp).error);
                                }
                            } else {
                                dc_delete_file(context, filename);
                                if 0 != (*job).foreign_id {
                                    dc_update_msg_state(context, (*job).foreign_id, 26i32);
                                    stmt = dc_sqlite3_prepare(
                                        (*context).sql,
                                        b"SELECT chat_id FROM msgs WHERE id=?\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    sqlite3_bind_int(stmt, 1i32, (*job).foreign_id as libc::c_int);
                                    let mut chat_id: libc::c_int = if sqlite3_step(stmt) == 100i32 {
                                        sqlite3_column_int(stmt, 0i32)
                                    } else {
                                        0i32
                                    };
                                    (*context).cb.expect("non-null function pointer")(
                                        context,
                                        2010i32,
                                        chat_id as uintptr_t,
                                        (*job).foreign_id as uintptr_t,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    sqlite3_finalize(stmt);
    if !recipients_list.is_null() {
        clist_free_content(recipients_list);
        clist_free(recipients_list);
    }
    free(recipients as *mut libc::c_void);
    free(buf);
    free(filename as *mut libc::c_void);
}
// this value does not increase the number of tries
#[no_mangle]
pub unsafe extern "C" fn dc_job_try_again_later(
    mut job: *mut dc_job_t,
    mut try_again: libc::c_int,
    mut pending_error: *const libc::c_char,
) {
    if job.is_null() {
        return;
    }
    (*job).try_again = try_again;
    free((*job).pending_error as *mut libc::c_void);
    (*job).pending_error = dc_strdup_keep_null(pending_error);
}
unsafe extern "C" fn dc_job_do_DC_JOB_MOVE_MSG(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut current_block: u64;
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut dest_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_uid: uint32_t = 0i32 as uint32_t;
    if 0 == dc_imap_is_connected((*context).inbox) {
        connect_to_inbox(context);
        if 0 == dc_imap_is_connected((*context).inbox) {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 2238328302157162973;
        } else {
            current_block = 2473556513754201174;
        }
    } else {
        current_block = 2473556513754201174;
    }
    match current_block {
        2473556513754201174 => {
            if !(0 == dc_msg_load_from_db(msg, context, (*job).foreign_id)) {
                if dc_sqlite3_get_config_int(
                    (*context).sql,
                    b"folders_configured\x00" as *const u8 as *const libc::c_char,
                    0i32,
                ) < 3i32
                {
                    dc_configure_folders(context, (*context).inbox, 0x1i32);
                }
                dest_folder = dc_sqlite3_get_config(
                    (*context).sql,
                    b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
                    0 as *const libc::c_char,
                );
                match dc_imap_move(
                    (*context).inbox,
                    (*msg).server_folder,
                    (*msg).server_uid,
                    dest_folder,
                    &mut dest_uid,
                ) as libc::c_uint
                {
                    1 => {
                        current_block = 6379107252614456477;
                        match current_block {
                            12072121998757195963 => {
                                dc_update_server_uid(
                                    context,
                                    (*msg).rfc724_mid,
                                    dest_folder,
                                    dest_uid,
                                );
                            }
                            _ => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                        }
                    }
                    3 => {
                        current_block = 12072121998757195963;
                        match current_block {
                            12072121998757195963 => {
                                dc_update_server_uid(
                                    context,
                                    (*msg).rfc724_mid,
                                    dest_folder,
                                    dest_uid,
                                );
                            }
                            _ => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                        }
                    }
                    0 | 2 | _ => {}
                }
            }
        }
        _ => {}
    }
    free(dest_folder as *mut libc::c_void);
    dc_msg_unref(msg);
}
/* ******************************************************************************
 * IMAP-jobs
 ******************************************************************************/
unsafe extern "C" fn connect_to_inbox(mut context: *mut dc_context_t) -> libc::c_int {
    let mut ret_connected: libc::c_int = 0i32;
    ret_connected = dc_connect_to_configured_imap(context, (*context).inbox);
    if !(0 == ret_connected) {
        dc_imap_set_watch_folder(
            (*context).inbox,
            b"INBOX\x00" as *const u8 as *const libc::c_char,
        );
    }
    return ret_connected;
}
unsafe extern "C" fn dc_job_do_DC_JOB_MARKSEEN_MDN_ON_IMAP(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut current_block: u64;
    let mut folder: *mut libc::c_char =
        dc_param_get((*job).param, 'Z' as i32, 0 as *const libc::c_char);
    let mut uid: uint32_t = dc_param_get_int((*job).param, 'z' as i32, 0i32) as uint32_t;
    let mut dest_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_uid: uint32_t = 0i32 as uint32_t;
    if 0 == dc_imap_is_connected((*context).inbox) {
        connect_to_inbox(context);
        if 0 == dc_imap_is_connected((*context).inbox) {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 2670689566614003383;
        } else {
            current_block = 11006700562992250127;
        }
    } else {
        current_block = 11006700562992250127;
    }
    match current_block {
        11006700562992250127 => {
            if dc_imap_set_seen((*context).inbox, folder, uid) as libc::c_uint
                == 0i32 as libc::c_uint
            {
                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            }
            if 0 != dc_param_get_int((*job).param, 'M' as i32, 0i32) {
                if dc_sqlite3_get_config_int(
                    (*context).sql,
                    b"folders_configured\x00" as *const u8 as *const libc::c_char,
                    0i32,
                ) < 3i32
                {
                    dc_configure_folders(context, (*context).inbox, 0x1i32);
                }
                dest_folder = dc_sqlite3_get_config(
                    (*context).sql,
                    b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
                    0 as *const libc::c_char,
                );
                match dc_imap_move((*context).inbox, folder, uid, dest_folder, &mut dest_uid)
                    as libc::c_uint
                {
                    1 => {
                        dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                    }
                    0 | _ => {}
                }
            }
        }
        _ => {}
    }
    free(folder as *mut libc::c_void);
    free(dest_folder as *mut libc::c_void);
}
unsafe extern "C" fn dc_job_do_DC_JOB_MARKSEEN_MSG_ON_IMAP(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut current_block: u64;
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    if 0 == dc_imap_is_connected((*context).inbox) {
        connect_to_inbox(context);
        if 0 == dc_imap_is_connected((*context).inbox) {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 17792648348530113339;
        } else {
            current_block = 15240798224410183470;
        }
    } else {
        current_block = 15240798224410183470;
    }
    match current_block {
        15240798224410183470 => {
            if !(0 == dc_msg_load_from_db(msg, context, (*job).foreign_id)) {
                match dc_imap_set_seen((*context).inbox, (*msg).server_folder, (*msg).server_uid)
                    as libc::c_uint
                {
                    0 => {}
                    1 => {
                        current_block = 12392248546350854223;
                        match current_block {
                            12392248546350854223 => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                            _ => {
                                if 0 != dc_param_get_int((*msg).param, 'r' as i32, 0i32)
                                    && 0 != dc_sqlite3_get_config_int(
                                        (*context).sql,
                                        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                                        1i32,
                                    )
                                {
                                    match dc_imap_set_mdnsent(
                                        (*context).inbox,
                                        (*msg).server_folder,
                                        (*msg).server_uid,
                                    ) as libc::c_uint
                                    {
                                        1 => {
                                            current_block = 4016212065805849280;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        3 => {
                                            current_block = 6186957421461061791;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        0 | 2 | _ => {}
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        current_block = 7746791466490516765;
                        match current_block {
                            12392248546350854223 => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                            _ => {
                                if 0 != dc_param_get_int((*msg).param, 'r' as i32, 0i32)
                                    && 0 != dc_sqlite3_get_config_int(
                                        (*context).sql,
                                        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                                        1i32,
                                    )
                                {
                                    match dc_imap_set_mdnsent(
                                        (*context).inbox,
                                        (*msg).server_folder,
                                        (*msg).server_uid,
                                    ) as libc::c_uint
                                    {
                                        1 => {
                                            current_block = 4016212065805849280;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        3 => {
                                            current_block = 6186957421461061791;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        0 | 2 | _ => {}
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
    dc_msg_unref(msg);
}
unsafe extern "C" fn dc_send_mdn(mut context: *mut dc_context_t, mut msg_id: uint32_t) {
    let mut mimefactory: dc_mimefactory_t = _dc_mimefactory {
        from_addr: 0 as *mut libc::c_char,
        from_displayname: 0 as *mut libc::c_char,
        selfstatus: 0 as *mut libc::c_char,
        recipients_names: 0 as *mut clist,
        recipients_addr: 0 as *mut clist,
        timestamp: 0,
        rfc724_mid: 0 as *mut libc::c_char,
        loaded: DC_MF_NOTHING_LOADED,
        msg: 0 as *mut dc_msg_t,
        chat: 0 as *mut dc_chat_t,
        increation: 0,
        in_reply_to: 0 as *mut libc::c_char,
        references: 0 as *mut libc::c_char,
        req_mdn: 0,
        out: 0 as *mut MMAPString,
        out_encrypted: 0,
        out_gossiped: 0,
        out_last_added_location_id: 0,
        error: 0 as *mut libc::c_char,
        context: 0 as *mut dc_context_t,
    };
    dc_mimefactory_init(&mut mimefactory, context);
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    if !(0 == dc_mimefactory_load_mdn(&mut mimefactory, msg_id)
        || 0 == dc_mimefactory_render(&mut mimefactory))
    {
        dc_add_smtp_job(context, 5011i32, &mut mimefactory);
    }
    dc_mimefactory_empty(&mut mimefactory);
}
/* ******************************************************************************
 * SMTP-jobs
 ******************************************************************************/
/* *
 * Store the MIME message in a file and send it later with a new SMTP job.
 *
 * @param context The context object as created by dc_context_new()
 * @param action One of the DC_JOB_SEND_ constants
 * @param mimefactory An instance of dc_mimefactory_t with a loaded and rendered message or MDN
 * @return 1=success, 0=error
 */
unsafe extern "C" fn dc_add_smtp_job(
    mut context: *mut dc_context_t,
    mut action: libc::c_int,
    mut mimefactory: *mut dc_mimefactory_t,
) -> libc::c_int {
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut success: libc::c_int = 0i32;
    let mut recipients: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param: *mut dc_param_t = dc_param_new();
    pathNfilename = dc_get_fine_pathNfilename(
        context,
        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
        (*mimefactory).rfc724_mid,
    );
    if pathNfilename.is_null() {
        dc_log_error(
            context,
            0i32,
            b"Could not find free file name for message with ID <%s>.\x00" as *const u8
                as *const libc::c_char,
            (*mimefactory).rfc724_mid,
        );
    } else if 0
        == dc_write_file(
            context,
            pathNfilename,
            (*(*mimefactory).out).str_0 as *const libc::c_void,
            (*(*mimefactory).out).len,
        )
    {
        dc_log_error(
            context,
            0i32,
            b"Could not write message <%s> to \"%s\".\x00" as *const u8 as *const libc::c_char,
            (*mimefactory).rfc724_mid,
            pathNfilename,
        );
    } else {
        recipients = dc_str_from_clist(
            (*mimefactory).recipients_addr,
            b"\x1e\x00" as *const u8 as *const libc::c_char,
        );
        dc_param_set(param, 'f' as i32, pathNfilename);
        dc_param_set(param, 'R' as i32, recipients);
        dc_job_add(
            context,
            action,
            (if (*mimefactory).loaded as libc::c_uint
                == DC_MF_MSG_LOADED as libc::c_int as libc::c_uint
            {
                (*(*mimefactory).msg).id
            } else {
                0i32 as libc::c_uint
            }) as libc::c_int,
            (*param).packed,
            0i32,
        );
        success = 1i32
    }
    dc_param_unref(param);
    free(recipients as *mut libc::c_void);
    free(pathNfilename as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_job_add(
    mut context: *mut dc_context_t,
    mut action: libc::c_int,
    mut foreign_id: libc::c_int,
    mut param: *const libc::c_char,
    mut delay_seconds: libc::c_int,
) {
    let mut timestamp: time_t = time(0 as *mut time_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut thread: libc::c_int = 0i32;
    if action >= 100i32 && action < 100i32 + 1000i32 {
        thread = 100i32
    } else if action >= 5000i32 && action < 5000i32 + 1000i32 {
        thread = 5000i32
    } else {
        return;
    }
    stmt =
        dc_sqlite3_prepare((*context).sql,
                           b"INSERT INTO jobs (added_timestamp, thread, action, foreign_id, param, desired_timestamp) VALUES (?,?,?,?,?,?);\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, thread);
    sqlite3_bind_int(stmt, 3i32, action);
    sqlite3_bind_int(stmt, 4i32, foreign_id);
    sqlite3_bind_text(
        stmt,
        5i32,
        if !param.is_null() {
            param
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        -1i32,
        None,
    );
    sqlite3_bind_int64(
        stmt,
        6i32,
        (timestamp + delay_seconds as libc::c_long) as sqlite3_int64,
    );
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    if thread == 100i32 {
        dc_interrupt_imap_idle(context);
    } else {
        dc_interrupt_smtp_idle(context);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_smtp_idle(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        dc_log_warning(
            context,
            0i32,
            b"Interrupt SMTP-idle: Bad parameters.\x00" as *const u8 as *const libc::c_char,
        );
        return;
    }
    dc_log_info(
        context,
        0i32,
        b"Interrupting SMTP-idle...\x00" as *const u8 as *const libc::c_char,
    );
    pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
    (*context).perform_smtp_jobs_needed = 1i32;
    (*context).smtpidle_condflag = 1i32;
    pthread_cond_signal(&mut (*context).smtpidle_cond);
    pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
}
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_imap_idle(mut context: *mut dc_context_t) {
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*context).inbox.is_null()
    {
        dc_log_warning(
            context,
            0i32,
            b"Interrupt IMAP-IDLE: Bad parameters.\x00" as *const u8 as *const libc::c_char,
        );
        return;
    }
    dc_log_info(
        context,
        0i32,
        b"Interrupting IMAP-IDLE...\x00" as *const u8 as *const libc::c_char,
    );
    pthread_mutex_lock(&mut (*context).inboxidle_condmutex);
    (*context).perform_inbox_jobs_needed = 1i32;
    pthread_mutex_unlock(&mut (*context).inboxidle_condmutex);
    dc_imap_interrupt_idle((*context).inbox);
}
unsafe extern "C" fn dc_job_do_DC_JOB_DELETE_MSG_ON_IMAP(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut current_block: u64;
    let mut delete_from_server: libc::c_int = 1i32;
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    if !(0 == dc_msg_load_from_db(msg, context, (*job).foreign_id)
        || (*msg).rfc724_mid.is_null()
        || *(*msg).rfc724_mid.offset(0isize) as libc::c_int == 0i32)
    {
        /* eg. device messages have no Message-ID */
        if dc_rfc724_mid_cnt(context, (*msg).rfc724_mid) != 1i32 {
            dc_log_info(
                context,
                0i32,
                b"The message is deleted from the server when all parts are deleted.\x00"
                    as *const u8 as *const libc::c_char,
            );
            delete_from_server = 0i32
        }
        /* if this is the last existing part of the message, we delete the message from the server */
        if 0 != delete_from_server {
            if 0 == dc_imap_is_connected((*context).inbox) {
                connect_to_inbox(context);
                if 0 == dc_imap_is_connected((*context).inbox) {
                    dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                    current_block = 8913536887710889399;
                } else {
                    current_block = 5399440093318478209;
                }
            } else {
                current_block = 5399440093318478209;
            }
            match current_block {
                8913536887710889399 => {}
                _ => {
                    if 0 == dc_imap_delete_msg(
                        (*context).inbox,
                        (*msg).rfc724_mid,
                        (*msg).server_folder,
                        (*msg).server_uid,
                    ) {
                        dc_job_try_again_later(job, -1i32, 0 as *const libc::c_char);
                        current_block = 8913536887710889399;
                    } else {
                        current_block = 17407779659766490442;
                    }
                }
            }
        } else {
            current_block = 17407779659766490442;
        }
        match current_block {
            8913536887710889399 => {}
            _ => {
                dc_delete_msg_from_db(context, (*msg).id);
            }
        }
    }
    dc_msg_unref(msg);
}
/* delete all pending jobs with the given action */
#[no_mangle]
pub unsafe extern "C" fn dc_job_kill_action(
    mut context: *mut dc_context_t,
    mut action: libc::c_int,
) {
    if context.is_null() {
        return;
    }
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"DELETE FROM jobs WHERE action=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, action);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_fetch(mut context: *mut dc_context_t) {
    let mut start: libc::clock_t = clock();
    if 0 == connect_to_inbox(context) {
        return;
    }
    if dc_sqlite3_get_config_int(
        (*context).sql,
        b"inbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    ) == 0i32
    {
        dc_log_info(
            context,
            0i32,
            b"INBOX-watch disabled.\x00" as *const u8 as *const libc::c_char,
        );
        return;
    }
    dc_log_info(
        context,
        0i32,
        b"INBOX-fetch started...\x00" as *const u8 as *const libc::c_char,
    );
    dc_imap_fetch((*context).inbox);
    if 0 != (*(*context).inbox).should_reconnect {
        dc_log_info(
            context,
            0i32,
            b"INBOX-fetch aborted, starting over...\x00" as *const u8 as *const libc::c_char,
        );
        dc_imap_fetch((*context).inbox);
    }
    dc_log_info(
        context,
        0i32,
        b"INBOX-fetch done in %.0f ms.\x00" as *const u8 as *const libc::c_char,
        clock().wrapping_sub(start) as libc::c_double * 1000.0f64 / 1000000i32 as libc::c_double,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_idle(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    connect_to_inbox(context);
    pthread_mutex_lock(&mut (*context).inboxidle_condmutex);
    if 0 != (*context).perform_inbox_jobs_needed {
        dc_log_info(
            context,
            0i32,
            b"INBOX-IDLE will not be started because of waiting jobs.\x00" as *const u8
                as *const libc::c_char,
        );
        pthread_mutex_unlock(&mut (*context).inboxidle_condmutex);
        return;
    }
    pthread_mutex_unlock(&mut (*context).inboxidle_condmutex);
    dc_log_info(
        context,
        0i32,
        b"INBOX-IDLE started...\x00" as *const u8 as *const libc::c_char,
    );
    dc_imap_idle((*context).inbox);
    dc_log_info(
        context,
        0i32,
        b"INBOX-IDLE ended.\x00" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_mvbox_fetch(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    let mut use_network: libc::c_int = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_fetch(&mut (*context).mvbox_thread, use_network);
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_mvbox_idle(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    let mut use_network: libc::c_int = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_idle(&mut (*context).mvbox_thread, use_network);
}
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_mvbox_idle(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        dc_log_warning(
            context,
            0i32,
            b"Interrupt MVBOX-IDLE: Bad parameters.\x00" as *const u8 as *const libc::c_char,
        );
        return;
    }
    dc_jobthread_interrupt_idle(&mut (*context).mvbox_thread);
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_sentbox_fetch(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    let mut use_network: libc::c_int = dc_sqlite3_get_config_int(
        (*context).sql,
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_fetch(&mut (*context).sentbox_thread, use_network);
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_sentbox_idle(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    let mut use_network: libc::c_int = dc_sqlite3_get_config_int(
        (*context).sql,
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_idle(&mut (*context).sentbox_thread, use_network);
}
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_sentbox_idle(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        dc_log_warning(
            context,
            0i32,
            b"Interrupt SENT-IDLE: Bad parameters.\x00" as *const u8 as *const libc::c_char,
        );
        return;
    }
    dc_jobthread_interrupt_idle(&mut (*context).sentbox_thread);
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_smtp_jobs(mut context: *mut dc_context_t) {
    pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
    let mut probe_smtp_network: libc::c_int = (*context).probe_smtp_network;
    (*context).probe_smtp_network = 0i32;
    (*context).perform_smtp_jobs_needed = 0i32;
    if 0 != (*context).smtp_suspended {
        dc_log_info(
            context,
            0i32,
            b"SMTP-jobs suspended.\x00" as *const u8 as *const libc::c_char,
        );
        pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
        return;
    }
    (*context).smtp_doing_jobs = 1i32;
    pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
    dc_log_info(
        context,
        0i32,
        b"SMTP-jobs started...\x00" as *const u8 as *const libc::c_char,
    );
    dc_job_perform(context, 5000i32, probe_smtp_network);
    dc_log_info(
        context,
        0i32,
        b"SMTP-jobs ended.\x00" as *const u8 as *const libc::c_char,
    );
    pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
    (*context).smtp_doing_jobs = 0i32;
    pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
}
#[no_mangle]
pub unsafe extern "C" fn dc_perform_smtp_idle(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        dc_log_warning(
            context,
            0i32,
            b"Cannot perform SMTP-idle: Bad parameters.\x00" as *const u8 as *const libc::c_char,
        );
        return;
    }
    dc_log_info(
        context,
        0i32,
        b"SMTP-idle started...\x00" as *const u8 as *const libc::c_char,
    );
    pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
    if (*context).perform_smtp_jobs_needed == 1i32 {
        dc_log_info(
            context,
            0i32,
            b"SMTP-idle will not be started because of waiting jobs.\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
        let mut r: libc::c_int = 0i32;
        let mut wakeup_at: timespec = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        memset(
            &mut wakeup_at as *mut timespec as *mut libc::c_void,
            0i32,
            ::std::mem::size_of::<timespec>() as libc::c_ulong,
        );
        wakeup_at.tv_sec = get_next_wakeup_time(context, 5000i32) + 1i32 as libc::c_long;
        while (*context).smtpidle_condflag == 0i32 && r == 0i32 {
            r = pthread_cond_timedwait(
                &mut (*context).smtpidle_cond,
                &mut (*context).smtpidle_condmutex,
                &mut wakeup_at,
            )
        }
        (*context).smtpidle_condflag = 0i32
    }
    pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
    dc_log_info(
        context,
        0i32,
        b"SMTP-idle ended.\x00" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn get_next_wakeup_time(
    mut context: *mut dc_context_t,
    mut thread: libc::c_int,
) -> time_t {
    let mut wakeup_time: time_t = 0i32 as time_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT MIN(desired_timestamp) FROM jobs WHERE thread=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, thread);
    if sqlite3_step(stmt) == 100i32 {
        wakeup_time = sqlite3_column_int(stmt, 0i32) as time_t
    }
    if wakeup_time == 0i32 as libc::c_long {
        wakeup_time = time(0 as *mut time_t) + (10i32 * 60i32) as libc::c_long
    }
    sqlite3_finalize(stmt);
    return wakeup_time;
}
#[no_mangle]
pub unsafe extern "C" fn dc_maybe_network(mut context: *mut dc_context_t) {
    pthread_mutex_lock(&mut (*context).smtpidle_condmutex);
    (*context).probe_smtp_network = 1i32;
    pthread_mutex_unlock(&mut (*context).smtpidle_condmutex);
    pthread_mutex_lock(&mut (*context).inboxidle_condmutex);
    (*context).probe_imap_network = 1i32;
    pthread_mutex_unlock(&mut (*context).inboxidle_condmutex);
    dc_interrupt_smtp_idle(context);
    dc_interrupt_imap_idle(context);
    dc_interrupt_mvbox_idle(context);
    dc_interrupt_sentbox_idle(context);
}
#[no_mangle]
pub unsafe extern "C" fn dc_job_action_exists(
    mut context: *mut dc_context_t,
    mut action: libc::c_int,
) -> libc::c_int {
    let mut job_exists: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT id FROM jobs WHERE action=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, action);
    job_exists = (sqlite3_step(stmt) == 100i32) as libc::c_int;
    sqlite3_finalize(stmt);
    return job_exists;
}
/* special case for DC_JOB_SEND_MSG_TO_SMTP */
#[no_mangle]
pub unsafe extern "C" fn dc_job_send_msg(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut mimefactory: dc_mimefactory_t = _dc_mimefactory {
        from_addr: 0 as *mut libc::c_char,
        from_displayname: 0 as *mut libc::c_char,
        selfstatus: 0 as *mut libc::c_char,
        recipients_names: 0 as *mut clist,
        recipients_addr: 0 as *mut clist,
        timestamp: 0,
        rfc724_mid: 0 as *mut libc::c_char,
        loaded: DC_MF_NOTHING_LOADED,
        msg: 0 as *mut dc_msg_t,
        chat: 0 as *mut dc_chat_t,
        increation: 0,
        in_reply_to: 0 as *mut libc::c_char,
        references: 0 as *mut libc::c_char,
        req_mdn: 0,
        out: 0 as *mut MMAPString,
        out_encrypted: 0,
        out_gossiped: 0,
        out_last_added_location_id: 0,
        error: 0 as *mut libc::c_char,
        context: 0 as *mut dc_context_t,
    };
    dc_mimefactory_init(&mut mimefactory, context);
    /* load message data */
    if 0 == dc_mimefactory_load_msg(&mut mimefactory, msg_id) || mimefactory.from_addr.is_null() {
        dc_log_warning(
            context,
            0i32,
            b"Cannot load data to send, maybe the message is deleted in between.\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
        // no redo, no IMAP. moreover, as the data does not exist, there is no need in calling dc_set_msg_failed()
        if (*mimefactory.msg).type_0 == 20i32
            || (*mimefactory.msg).type_0 == 21i32
            || (*mimefactory.msg).type_0 == 40i32
            || (*mimefactory.msg).type_0 == 41i32
            || (*mimefactory.msg).type_0 == 50i32
            || (*mimefactory.msg).type_0 == 60i32
        {
            let mut pathNfilename: *mut libc::c_char = dc_param_get(
                (*mimefactory.msg).param,
                'f' as i32,
                0 as *const libc::c_char,
            );
            if !pathNfilename.is_null() {
                if ((*mimefactory.msg).type_0 == 20i32 || (*mimefactory.msg).type_0 == 21i32)
                    && 0 == dc_param_exists((*mimefactory.msg).param, 'w' as i32)
                {
                    let mut buf: *mut libc::c_uchar = 0 as *mut libc::c_uchar;
                    let mut buf_bytes: size_t = 0;
                    let mut w: uint32_t = 0;
                    let mut h: uint32_t = 0;
                    dc_param_set_int((*mimefactory.msg).param, 'w' as i32, 0i32);
                    dc_param_set_int((*mimefactory.msg).param, 'h' as i32, 0i32);
                    if 0 != dc_read_file(
                        context,
                        pathNfilename,
                        &mut buf as *mut *mut libc::c_uchar as *mut *mut libc::c_void,
                        &mut buf_bytes,
                    ) {
                        if 0 != dc_get_filemeta(
                            buf as *const libc::c_void,
                            buf_bytes,
                            &mut w,
                            &mut h,
                        ) {
                            dc_param_set_int((*mimefactory.msg).param, 'w' as i32, w as int32_t);
                            dc_param_set_int((*mimefactory.msg).param, 'h' as i32, h as int32_t);
                        }
                    }
                    free(buf as *mut libc::c_void);
                    dc_msg_save_param_to_disk(mimefactory.msg);
                }
            }
            free(pathNfilename as *mut libc::c_void);
        }
        /* create message */
        if 0 == dc_mimefactory_render(&mut mimefactory) {
            dc_set_msg_failed(context, msg_id, mimefactory.error);
        } else if 0 != dc_param_get_int((*mimefactory.msg).param, 'c' as i32, 0i32)
            && 0 == mimefactory.out_encrypted
        {
            dc_set_msg_failed(
                context,
                msg_id,
                b"End-to-end-encryption unavailable unexpectedly.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
            /* unrecoverable */
            if clist_search_string_nocase(mimefactory.recipients_addr, mimefactory.from_addr)
                == 0i32
            {
                clist_insert_after(
                    mimefactory.recipients_names,
                    (*mimefactory.recipients_names).last,
                    0 as *mut libc::c_void,
                );
                clist_insert_after(
                    mimefactory.recipients_addr,
                    (*mimefactory.recipients_addr).last,
                    dc_strdup(mimefactory.from_addr) as *mut libc::c_void,
                );
            }
            dc_sqlite3_begin_transaction((*context).sql);
            if 0 != mimefactory.out_gossiped {
                dc_set_gossiped_timestamp(
                    context,
                    (*mimefactory.msg).chat_id,
                    time(0 as *mut time_t),
                );
            }
            if 0 != mimefactory.out_last_added_location_id {
                dc_set_kml_sent_timestamp(
                    context,
                    (*mimefactory.msg).chat_id,
                    time(0 as *mut time_t),
                );
                if 0 == (*mimefactory.msg).hidden {
                    dc_set_msg_location_id(
                        context,
                        (*mimefactory.msg).id,
                        mimefactory.out_last_added_location_id,
                    );
                }
            }
            if 0 != mimefactory.out_encrypted
                && dc_param_get_int((*mimefactory.msg).param, 'c' as i32, 0i32) == 0i32
            {
                dc_param_set_int((*mimefactory.msg).param, 'c' as i32, 1i32);
                dc_msg_save_param_to_disk(mimefactory.msg);
            }
            dc_add_to_keyhistory(
                context,
                0 as *const libc::c_char,
                0i32 as time_t,
                0 as *const libc::c_char,
                0 as *const libc::c_char,
            );
            dc_sqlite3_commit((*context).sql);
            success = dc_add_smtp_job(context, 5901i32, &mut mimefactory)
        }
    }
    dc_mimefactory_empty(&mut mimefactory);
    return success;
}

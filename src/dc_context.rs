use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_imap::dc_imap_t;
use crate::dc_jobthread::dc_jobthread_t;
use crate::dc_lot::dc_lot_t;
use crate::dc_smtp::dc_smtp_t;
use crate::types::*;

extern "C" {
    #[no_mangle]
    fn getpid() -> pid_t;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn pthread_cond_destroy(_: *mut pthread_cond_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_cond_init(_: *mut pthread_cond_t, _: *const pthread_condattr_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_destroy(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_init(_: *mut pthread_mutex_t, _: *const pthread_mutexattr_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_self() -> pthread_t;
    #[no_mangle]
    fn libetpan_get_version_major() -> libc::c_int;
    #[no_mangle]
    fn libetpan_get_version_minor() -> libc::c_int;
    #[no_mangle]
    fn dc_pgp_rand_seed(_: *mut dc_context_t, buf: *const libc::c_void, bytes: size_t);
    #[no_mangle]
    fn dc_smtp_new(_: *mut dc_context_t) -> *mut dc_smtp_t;
    #[no_mangle]
    fn dc_receive_imf(
        _: *mut dc_context_t,
        imf_raw_not_terminated: *const libc::c_char,
        imf_raw_bytes: size_t,
        server_folder: *const libc::c_char,
        server_uid: uint32_t,
        flags: uint32_t,
    );
    #[no_mangle]
    fn dc_job_add(
        _: *mut dc_context_t,
        action: libc::c_int,
        foreign_id: libc::c_int,
        param: *const libc::c_char,
        delay: libc::c_int,
    );
    #[no_mangle]
    fn dc_do_heuristics_moves(_: *mut dc_context_t, folder: *const libc::c_char, msg_id: uint32_t);
    #[no_mangle]
    fn dc_update_server_uid(
        _: *mut dc_context_t,
        rfc724_mid: *const libc::c_char,
        server_folder: *const libc::c_char,
        server_uid: uint32_t,
    );
    #[no_mangle]
    fn dc_update_msg_move_state(
        _: *mut dc_context_t,
        rfc724_mid: *const libc::c_char,
        _: dc_move_state_t,
    );
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_rfc724_mid_exists(
        _: *mut dc_context_t,
        rfc724_mid: *const libc::c_char,
        ret_server_folder: *mut *mut libc::c_char,
        ret_server_uid: *mut uint32_t,
    ) -> uint32_t;
    /* handle configurations, private */
    #[no_mangle]
    fn dc_sqlite3_set_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        value: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_imap_new(
        _: dc_get_config_t,
        _: dc_set_config_t,
        _: dc_precheck_imf_t,
        _: dc_receive_imf_t,
        userData: *mut libc::c_void,
        _: *mut dc_context_t,
    ) -> *mut dc_imap_t;
    #[no_mangle]
    fn dc_sqlite3_new(_: *mut dc_context_t) -> *mut dc_sqlite3_t;
    /* ** library-private **********************************************************/
    /* validation errors */
    /* misc. */
    #[no_mangle]
    fn dc_pgp_init();
    /* ** library-private **********************************************************/
    #[no_mangle]
    fn dc_openssl_init();
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_jobthread_init(
        _: *mut dc_jobthread_t,
        context: *mut dc_context_t,
        name: *const libc::c_char,
        folder_config_name: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_jobthread_exit(_: *mut dc_jobthread_t);
    #[no_mangle]
    fn dc_openssl_exit();
    #[no_mangle]
    fn dc_sqlite3_unref(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_smtp_unref(_: *mut dc_smtp_t);
    #[no_mangle]
    fn dc_imap_unref(_: *mut dc_imap_t);
    #[no_mangle]
    fn dc_sqlite3_close(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_is_open(_: *const dc_sqlite3_t) -> libc::c_int;
    #[no_mangle]
    fn dc_smtp_disconnect(_: *mut dc_smtp_t);
    #[no_mangle]
    fn dc_imap_disconnect(_: *mut dc_imap_t);
    #[no_mangle]
    fn dc_pgp_exit();
    #[no_mangle]
    fn dc_sqlite3_open(
        _: *mut dc_sqlite3_t,
        dbfile: *const libc::c_char,
        flags: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_create_folder(_: *mut dc_context_t, pathNfilename: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    /* file tools */
    #[no_mangle]
    fn dc_ensure_no_slash(pathNfilename: *mut libc::c_char);
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    /* Return the string with the given ID by calling DC_EVENT_GET_STRING.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str(_: *mut dc_context_t, id: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_interrupt_mvbox_idle(_: *mut dc_context_t);
    #[no_mangle]
    fn dc_interrupt_sentbox_idle(_: *mut dc_context_t);
    #[no_mangle]
    fn dc_interrupt_imap_idle(_: *mut dc_context_t);
    #[no_mangle]
    fn dc_make_rel_and_copy(
        _: *mut dc_context_t,
        pathNfilename: *mut *mut libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_get_abs_path(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_key_new() -> *mut dc_key_t;
    #[no_mangle]
    fn dc_key_unref(_: *mut dc_key_t);
    #[no_mangle]
    fn dc_loginparam_unref(_: *mut dc_loginparam_t);
    #[no_mangle]
    fn sqlite3_threadsafe() -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_get_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: int32_t,
    ) -> int32_t;
    #[no_mangle]
    fn dc_loginparam_get_readable(_: *const dc_loginparam_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_key_get_fingerprint(_: *const dc_key_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_key_load_self_public(
        _: *mut dc_key_t,
        self_addr: *const libc::c_char,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    // Context functions to work with contacts
    #[no_mangle]
    fn dc_get_real_contact_cnt(_: *mut dc_context_t) -> size_t;
    #[no_mangle]
    fn dc_get_deaddrop_msg_cnt(_: *mut dc_context_t) -> size_t;
    #[no_mangle]
    fn dc_get_real_msg_cnt(_: *mut dc_context_t) -> size_t;
    #[no_mangle]
    fn dc_get_chat_cnt(_: *mut dc_context_t) -> size_t;
    #[no_mangle]
    fn dc_loginparam_read(
        _: *mut dc_loginparam_t,
        _: *mut dc_sqlite3_t,
        prefix: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_loginparam_new() -> *mut dc_loginparam_t;
    #[no_mangle]
    fn dc_array_new(_: *mut dc_context_t, initsize: size_t) -> *mut dc_array_t;
    #[no_mangle]
    fn dc_array_add_id(_: *mut dc_array_t, _: uint32_t);
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
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
    fn sqlite3_bind_text(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_char,
        _: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
}
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
pub struct dc_context_t {
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

unsafe impl Send for dc_context_t {}
unsafe impl Sync for dc_context_t {}
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

/* *
 * Library-internal.
 */
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
pub type dc_move_state_t = libc::c_uint;
pub const DC_MOVE_STATE_MOVING: dc_move_state_t = 3;
pub const DC_MOVE_STATE_STAY: dc_move_state_t = 2;
pub const DC_MOVE_STATE_PENDING: dc_move_state_t = 1;
pub const DC_MOVE_STATE_UNDEFINED: dc_move_state_t = 0;
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
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
// create/open/config/information
#[no_mangle]
pub unsafe extern "C" fn dc_context_new(
    mut cb: dc_callback_t,
    mut userdata: *mut libc::c_void,
    mut os_name: *const libc::c_char,
) -> *mut dc_context_t {
    let mut context: *mut dc_context_t = 0 as *mut dc_context_t;
    context = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_context_t>() as libc::c_ulong,
    ) as *mut dc_context_t;
    if context.is_null() {
        exit(23i32);
    }
    pthread_mutex_init(
        &mut (*context).smear_critical,
        0 as *const pthread_mutexattr_t,
    );
    pthread_mutex_init(
        &mut (*context).bobs_qr_critical,
        0 as *const pthread_mutexattr_t,
    );
    pthread_mutex_init(
        &mut (*context).inboxidle_condmutex,
        0 as *const pthread_mutexattr_t,
    );
    dc_jobthread_init(
        &mut (*context).sentbox_thread,
        context,
        b"SENTBOX\x00" as *const u8 as *const libc::c_char,
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
    );
    dc_jobthread_init(
        &mut (*context).mvbox_thread,
        context,
        b"MVBOX\x00" as *const u8 as *const libc::c_char,
        b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
    );
    pthread_mutex_init(
        &mut (*context).smtpidle_condmutex,
        0 as *const pthread_mutexattr_t,
    );
    pthread_cond_init(
        &mut (*context).smtpidle_cond,
        0 as *const pthread_condattr_t,
    );
    pthread_mutex_init(
        &mut (*context).oauth2_critical,
        0 as *const pthread_mutexattr_t,
    );
    (*context).magic = 0x11a11807i32 as uint32_t;
    (*context).userdata = userdata;
    (*context).cb = if cb.is_some() { cb } else { Some(cb_dummy) };
    (*context).os_name = dc_strdup_keep_null(os_name);
    (*context).shall_stop_ongoing = 1i32;
    dc_openssl_init();
    dc_pgp_init();
    (*context).sql = dc_sqlite3_new(context);
    (*context).inbox = dc_imap_new(
        Some(cb_get_config),
        Some(cb_set_config),
        Some(cb_precheck_imf),
        Some(cb_receive_imf),
        context as *mut libc::c_void,
        context,
    );
    (*context).sentbox_thread.imap = dc_imap_new(
        Some(cb_get_config),
        Some(cb_set_config),
        Some(cb_precheck_imf),
        Some(cb_receive_imf),
        context as *mut libc::c_void,
        context,
    );
    (*context).mvbox_thread.imap = dc_imap_new(
        Some(cb_get_config),
        Some(cb_set_config),
        Some(cb_precheck_imf),
        Some(cb_receive_imf),
        context as *mut libc::c_void,
        context,
    );
    (*context).smtp = dc_smtp_new(context);
    /* Random-seed.  An additional seed with more random data is done just before key generation
    (the timespan between this call and the key generation time is typically random.
    Moreover, later, we add a hash of the first message data to the random-seed
    (it would be okay to seed with even more sensible data, the seed values cannot be recovered from the PRNG output, see OpenSSL's RAND_seed()) */
    let mut seed: [uintptr_t; 5] = [0; 5];
    seed[0usize] = time(0 as *mut time_t) as uintptr_t;
    seed[1usize] = seed.as_mut_ptr() as uintptr_t;
    seed[2usize] = context as uintptr_t;
    seed[3usize] = pthread_self() as uintptr_t;
    seed[4usize] = getpid() as uintptr_t;
    dc_pgp_rand_seed(
        context,
        seed.as_mut_ptr() as *const libc::c_void,
        ::std::mem::size_of::<[uintptr_t; 5]>() as libc::c_ulong,
    );
    return context;
}
unsafe extern "C" fn cb_receive_imf(
    mut imap: *mut dc_imap_t,
    mut imf_raw_not_terminated: *const libc::c_char,
    mut imf_raw_bytes: size_t,
    mut server_folder: *const libc::c_char,
    mut server_uid: uint32_t,
    mut flags: uint32_t,
) {
    let mut context: *mut dc_context_t = (*imap).userData as *mut dc_context_t;
    dc_receive_imf(
        context,
        imf_raw_not_terminated,
        imf_raw_bytes,
        server_folder,
        server_uid,
        flags,
    );
}
unsafe extern "C" fn cb_precheck_imf(
    mut imap: *mut dc_imap_t,
    mut rfc724_mid: *const libc::c_char,
    mut server_folder: *const libc::c_char,
    mut server_uid: uint32_t,
) -> libc::c_int {
    let mut rfc724_mid_exists: libc::c_int = 0i32;
    let mut msg_id: uint32_t = 0i32 as uint32_t;
    let mut old_server_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut old_server_uid: uint32_t = 0i32 as uint32_t;
    let mut mark_seen: libc::c_int = 0i32;
    msg_id = dc_rfc724_mid_exists(
        (*imap).context,
        rfc724_mid,
        &mut old_server_folder,
        &mut old_server_uid,
    );
    if msg_id != 0i32 as libc::c_uint {
        rfc724_mid_exists = 1i32;
        if *old_server_folder.offset(0isize) as libc::c_int == 0i32
            && old_server_uid == 0i32 as libc::c_uint
        {
            dc_log_info(
                (*imap).context,
                0i32,
                b"[move] detected bbc-self %s\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            mark_seen = 1i32
        } else if strcmp(old_server_folder, server_folder) != 0i32 {
            dc_log_info(
                (*imap).context,
                0i32,
                b"[move] detected moved message %s\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            dc_update_msg_move_state((*imap).context, rfc724_mid, DC_MOVE_STATE_STAY);
        }
        if strcmp(old_server_folder, server_folder) != 0i32 || old_server_uid != server_uid {
            dc_update_server_uid((*imap).context, rfc724_mid, server_folder, server_uid);
        }
        dc_do_heuristics_moves((*imap).context, server_folder, msg_id);
        if 0 != mark_seen {
            dc_job_add(
                (*imap).context,
                130i32,
                msg_id as libc::c_int,
                0 as *const libc::c_char,
                0i32,
            );
        }
    }
    free(old_server_folder as *mut libc::c_void);
    return rfc724_mid_exists;
}
unsafe extern "C" fn cb_set_config(
    mut imap: *mut dc_imap_t,
    mut key: *const libc::c_char,
    mut value: *const libc::c_char,
) {
    let mut context: *mut dc_context_t = (*imap).userData as *mut dc_context_t;
    dc_sqlite3_set_config((*context).sql, key, value);
}
/* *
 * The following three callback are given to dc_imap_new() to read/write configuration
 * and to handle received messages. As the imap-functions are typically used in
 * a separate user-thread, also these functions may be called from a different thread.
 *
 * @private @memberof dc_context_t
 */
unsafe extern "C" fn cb_get_config(
    mut imap: *mut dc_imap_t,
    mut key: *const libc::c_char,
    mut def: *const libc::c_char,
) -> *mut libc::c_char {
    let mut context: *mut dc_context_t = (*imap).userData as *mut dc_context_t;
    return dc_sqlite3_get_config((*context).sql, key, def);
}
/* *
 * A callback function that is used if no user-defined callback is given to dc_context_new().
 * The callback function simply returns 0 which is safe for every event.
 *
 * @private @memberof dc_context_t
 */
unsafe extern "C" fn cb_dummy(
    mut context: *mut dc_context_t,
    mut event: libc::c_int,
    mut data1: uintptr_t,
    mut data2: uintptr_t,
) -> uintptr_t {
    return 0i32 as uintptr_t;
}
#[no_mangle]
pub unsafe extern "C" fn dc_context_unref(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    dc_pgp_exit();
    if 0 != dc_is_open(context) {
        dc_close(context);
    }
    dc_imap_unref((*context).inbox);
    dc_imap_unref((*context).sentbox_thread.imap);
    dc_imap_unref((*context).mvbox_thread.imap);
    dc_smtp_unref((*context).smtp);
    dc_sqlite3_unref((*context).sql);
    dc_openssl_exit();
    pthread_mutex_destroy(&mut (*context).smear_critical);
    pthread_mutex_destroy(&mut (*context).bobs_qr_critical);
    pthread_mutex_destroy(&mut (*context).inboxidle_condmutex);
    dc_jobthread_exit(&mut (*context).sentbox_thread);
    dc_jobthread_exit(&mut (*context).mvbox_thread);
    pthread_cond_destroy(&mut (*context).smtpidle_cond);
    pthread_mutex_destroy(&mut (*context).smtpidle_condmutex);
    pthread_mutex_destroy(&mut (*context).oauth2_critical);
    free((*context).os_name as *mut libc::c_void);
    (*context).magic = 0i32 as uint32_t;
    free(context as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_close(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    dc_imap_disconnect((*context).inbox);
    dc_imap_disconnect((*context).sentbox_thread.imap);
    dc_imap_disconnect((*context).mvbox_thread.imap);
    dc_smtp_disconnect((*context).smtp);
    if 0 != dc_sqlite3_is_open((*context).sql) {
        dc_sqlite3_close((*context).sql);
    }
    free((*context).dbfile as *mut libc::c_void);
    (*context).dbfile = 0 as *mut libc::c_char;
    free((*context).blobdir as *mut libc::c_void);
    (*context).blobdir = 0 as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_open(mut context: *const dc_context_t) -> libc::c_int {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0i32;
    }
    return dc_sqlite3_is_open((*context).sql);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_userdata(mut context: *mut dc_context_t) -> *mut libc::c_void {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0 as *mut libc::c_void;
    }
    return (*context).userdata;
}
#[no_mangle]
pub unsafe extern "C" fn dc_open(
    mut context: *mut dc_context_t,
    mut dbfile: *const libc::c_char,
    mut blobdir: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    if 0 != dc_is_open(context) {
        return 0i32;
    }
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || dbfile.is_null())
    {
        (*context).dbfile = dc_strdup(dbfile);
        if !blobdir.is_null() && 0 != *blobdir.offset(0isize) as libc::c_int {
            (*context).blobdir = dc_strdup(blobdir);
            dc_ensure_no_slash((*context).blobdir);
        } else {
            (*context).blobdir =
                dc_mprintf(b"%s-blobs\x00" as *const u8 as *const libc::c_char, dbfile);
            dc_create_folder(context, (*context).blobdir);
        }
        /* Create/open sqlite database, this may already use the blobdir */
        if !(0 == dc_sqlite3_open((*context).sql, dbfile, 0i32)) {
            success = 1i32
        }
    }
    if 0 == success {
        dc_close(context);
    }
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_blobdir(mut context: *const dc_context_t) -> *mut libc::c_char {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    return dc_strdup((*context).blobdir);
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_config(
    mut context: *mut dc_context_t,
    mut key: *const libc::c_char,
    mut value: *const libc::c_char,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut rel_path: *mut libc::c_char = 0 as *mut libc::c_char;
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || key.is_null()
        || 0 == is_settable_config_key(key)
    {
        return 0i32;
    }
    if strcmp(key, b"selfavatar\x00" as *const u8 as *const libc::c_char) == 0i32
        && !value.is_null()
    {
        rel_path = dc_strdup(value);
        if !(0 == dc_make_rel_and_copy(context, &mut rel_path)) {
            ret = dc_sqlite3_set_config((*context).sql, key, rel_path)
        }
    } else if strcmp(key, b"inbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_sqlite3_set_config((*context).sql, key, value);
        dc_interrupt_imap_idle(context);
    } else if strcmp(
        key,
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        ret = dc_sqlite3_set_config((*context).sql, key, value);
        dc_interrupt_sentbox_idle(context);
    } else if strcmp(key, b"mvbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_sqlite3_set_config((*context).sql, key, value);
        dc_interrupt_mvbox_idle(context);
    } else if strcmp(key, b"selfstatus\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let mut def: *mut libc::c_char = dc_stock_str(context, 13i32);
        ret = dc_sqlite3_set_config(
            (*context).sql,
            key,
            if value.is_null() || strcmp(value, def) == 0i32 {
                0 as *const libc::c_char
            } else {
                value
            },
        );
        free(def as *mut libc::c_void);
    } else {
        ret = dc_sqlite3_set_config((*context).sql, key, value)
    }
    free(rel_path as *mut libc::c_void);
    return ret;
}
/* ******************************************************************************
 * INI-handling, Information
 ******************************************************************************/
unsafe extern "C" fn is_settable_config_key(mut key: *const libc::c_char) -> libc::c_int {
    let mut i: libc::c_int = 0i32;
    while (i as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 33]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strcmp(key, config_keys[i as usize]) == 0i32 {
            return 1i32;
        }
        i += 1
    }
    return 0i32;
}
static mut config_keys: [*const libc::c_char; 33] = [
    b"addr\x00" as *const u8 as *const libc::c_char,
    b"mail_server\x00" as *const u8 as *const libc::c_char,
    b"mail_user\x00" as *const u8 as *const libc::c_char,
    b"mail_pw\x00" as *const u8 as *const libc::c_char,
    b"mail_port\x00" as *const u8 as *const libc::c_char,
    b"send_server\x00" as *const u8 as *const libc::c_char,
    b"send_user\x00" as *const u8 as *const libc::c_char,
    b"send_pw\x00" as *const u8 as *const libc::c_char,
    b"send_port\x00" as *const u8 as *const libc::c_char,
    b"server_flags\x00" as *const u8 as *const libc::c_char,
    b"imap_folder\x00" as *const u8 as *const libc::c_char,
    b"displayname\x00" as *const u8 as *const libc::c_char,
    b"selfstatus\x00" as *const u8 as *const libc::c_char,
    b"selfavatar\x00" as *const u8 as *const libc::c_char,
    b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
    b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
    b"inbox_watch\x00" as *const u8 as *const libc::c_char,
    b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
    b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
    b"mvbox_move\x00" as *const u8 as *const libc::c_char,
    b"show_emails\x00" as *const u8 as *const libc::c_char,
    b"save_mime_headers\x00" as *const u8 as *const libc::c_char,
    b"configured_addr\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_server\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_user\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_pw\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_port\x00" as *const u8 as *const libc::c_char,
    b"configured_send_server\x00" as *const u8 as *const libc::c_char,
    b"configured_send_user\x00" as *const u8 as *const libc::c_char,
    b"configured_send_pw\x00" as *const u8 as *const libc::c_char,
    b"configured_send_port\x00" as *const u8 as *const libc::c_char,
    b"configured_server_flags\x00" as *const u8 as *const libc::c_char,
    b"configured\x00" as *const u8 as *const libc::c_char,
];
#[no_mangle]
pub unsafe extern "C" fn dc_get_config(
    mut context: *mut dc_context_t,
    mut key: *const libc::c_char,
) -> *mut libc::c_char {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    if !key.is_null()
        && *key.offset(0isize) as libc::c_int == 's' as i32
        && *key.offset(1isize) as libc::c_int == 'y' as i32
        && *key.offset(2isize) as libc::c_int == 's' as i32
        && *key.offset(3isize) as libc::c_int == '.' as i32
    {
        return get_sys_config_str(key);
    }
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || key.is_null()
        || 0 == is_gettable_config_key(key)
    {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    if strcmp(key, b"selfavatar\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let mut rel_path: *mut libc::c_char =
            dc_sqlite3_get_config((*context).sql, key, 0 as *const libc::c_char);
        if !rel_path.is_null() {
            value = dc_get_abs_path(context, rel_path);
            free(rel_path as *mut libc::c_void);
        }
    } else {
        value = dc_sqlite3_get_config((*context).sql, key, 0 as *const libc::c_char)
    }
    if value.is_null() {
        if strcmp(key, b"e2ee_enabled\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"mdns_enabled\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"imap_folder\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_strdup(b"INBOX\x00" as *const u8 as *const libc::c_char)
        } else if strcmp(key, b"inbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(
            key,
            b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        ) == 0i32
        {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"mvbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"mvbox_move\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"show_emails\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 0i32)
        } else if strcmp(key, b"selfstatus\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_stock_str(context, 13i32)
        } else {
            value = dc_mprintf(b"\x00" as *const u8 as *const libc::c_char)
        }
    }
    return value;
}
unsafe extern "C" fn is_gettable_config_key(mut key: *const libc::c_char) -> libc::c_int {
    let mut i: libc::c_int = 0i32;
    while (i as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 3]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strcmp(key, sys_config_keys[i as usize]) == 0i32 {
            return 1i32;
        }
        i += 1
    }
    return is_settable_config_key(key);
}
// deprecated
static mut sys_config_keys: [*const libc::c_char; 3] = [
    b"sys.version\x00" as *const u8 as *const libc::c_char,
    b"sys.msgsize_max_recommended\x00" as *const u8 as *const libc::c_char,
    b"sys.config_keys\x00" as *const u8 as *const libc::c_char,
];
unsafe extern "C" fn get_sys_config_str(mut key: *const libc::c_char) -> *mut libc::c_char {
    if strcmp(key, b"sys.version\x00" as *const u8 as *const libc::c_char) == 0i32 {
        return dc_strdup(b"0.42.0\x00" as *const u8 as *const libc::c_char);
    } else if strcmp(
        key,
        b"sys.msgsize_max_recommended\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        return dc_mprintf(
            b"%i\x00" as *const u8 as *const libc::c_char,
            24i32 * 1024i32 * 1024i32 / 4i32 * 3i32,
        );
    } else if strcmp(
        key,
        b"sys.config_keys\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        return get_config_keys_str();
    } else {
        return dc_strdup(0 as *const libc::c_char);
    };
}
unsafe extern "C" fn get_config_keys_str() -> *mut libc::c_char {
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    let mut i: libc::c_int = 0i32;
    while (i as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 33]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strlen(ret.buf) > 0i32 as libc::c_ulong {
            dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
        }
        dc_strbuilder_cat(&mut ret, config_keys[i as usize]);
        i += 1
    }
    let mut i_0: libc::c_int = 0i32;
    while (i_0 as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 3]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strlen(ret.buf) > 0i32 as libc::c_ulong {
            dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
        }
        dc_strbuilder_cat(&mut ret, sys_config_keys[i_0 as usize]);
        i_0 += 1
    }
    return ret.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_info(mut context: *mut dc_context_t) -> *mut libc::c_char {
    let mut unset: *const libc::c_char = b"0\x00" as *const u8 as *const libc::c_char;
    let mut displayname: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut temp: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut l_readable_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut l2_readable_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut l: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    let mut l2: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    let mut inbox_watch: libc::c_int = 0i32;
    let mut sentbox_watch: libc::c_int = 0i32;
    let mut mvbox_watch: libc::c_int = 0i32;
    let mut mvbox_move: libc::c_int = 0i32;
    let mut folders_configured: libc::c_int = 0i32;
    let mut configured_sentbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut configured_mvbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contacts: libc::c_int = 0i32;
    let mut chats: libc::c_int = 0i32;
    let mut real_msgs: libc::c_int = 0i32;
    let mut deaddrop_msgs: libc::c_int = 0i32;
    let mut is_configured: libc::c_int = 0i32;
    let mut dbversion: libc::c_int = 0i32;
    let mut mdns_enabled: libc::c_int = 0i32;
    let mut e2ee_enabled: libc::c_int = 0i32;
    let mut prv_key_cnt: libc::c_int = 0i32;
    let mut pub_key_cnt: libc::c_int = 0i32;
    let mut self_public: *mut dc_key_t = dc_key_new();
    let mut rpgp_enabled: libc::c_int = 0i32;
    rpgp_enabled = 1i32;
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return dc_strdup(b"ErrBadPtr\x00" as *const u8 as *const libc::c_char);
    }
    l = dc_loginparam_new();
    l2 = dc_loginparam_new();
    dc_loginparam_read(
        l,
        (*context).sql,
        b"\x00" as *const u8 as *const libc::c_char,
    );
    dc_loginparam_read(
        l2,
        (*context).sql,
        b"configured_\x00" as *const u8 as *const libc::c_char,
    );
    displayname = dc_sqlite3_get_config(
        (*context).sql,
        b"displayname\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    chats = dc_get_chat_cnt(context) as libc::c_int;
    real_msgs = dc_get_real_msg_cnt(context) as libc::c_int;
    deaddrop_msgs = dc_get_deaddrop_msg_cnt(context) as libc::c_int;
    contacts = dc_get_real_contact_cnt(context) as libc::c_int;
    is_configured = dc_sqlite3_get_config_int(
        (*context).sql,
        b"configured\x00" as *const u8 as *const libc::c_char,
        0i32,
    );
    dbversion = dc_sqlite3_get_config_int(
        (*context).sql,
        b"dbversion\x00" as *const u8 as *const libc::c_char,
        0i32,
    );
    e2ee_enabled = dc_sqlite3_get_config_int(
        (*context).sql,
        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    mdns_enabled = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT COUNT(*) FROM keypairs;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    prv_key_cnt = sqlite3_column_int(stmt, 0i32);
    sqlite3_finalize(stmt);
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT COUNT(*) FROM acpeerstates;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    pub_key_cnt = sqlite3_column_int(stmt, 0i32);
    sqlite3_finalize(stmt);
    if 0 != dc_key_load_self_public(self_public, (*l2).addr, (*context).sql) {
        fingerprint_str = dc_key_get_fingerprint(self_public)
    } else {
        fingerprint_str = dc_strdup(b"<Not yet calculated>\x00" as *const u8 as *const libc::c_char)
    }
    l_readable_str = dc_loginparam_get_readable(l);
    l2_readable_str = dc_loginparam_get_readable(l2);
    inbox_watch = dc_sqlite3_get_config_int(
        (*context).sql,
        b"inbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    sentbox_watch = dc_sqlite3_get_config_int(
        (*context).sql,
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    mvbox_watch = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    mvbox_move = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mvbox_move\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    folders_configured = dc_sqlite3_get_config_int(
        (*context).sql,
        b"folders_configured\x00" as *const u8 as *const libc::c_char,
        0i32,
    );
    configured_sentbox_folder = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
        b"<unset>\x00" as *const u8 as *const libc::c_char,
    );
    configured_mvbox_folder = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
        b"<unset>\x00" as *const u8 as *const libc::c_char,
    );
    temp =
        dc_mprintf(b"deltachat_core_version=v%s\nsqlite_version=%s\nsqlite_thread_safe=%i\nlibetpan_version=%i.%i\nopenssl_version=%i.%i.%i%c\nrpgp_enabled=%i\ncompile_date=Apr 26 2019, 00:51:50\narch=%i\nnumber_of_chats=%i\nnumber_of_chat_messages=%i\nmessages_in_contact_requests=%i\nnumber_of_contacts=%i\ndatabase_dir=%s\ndatabase_version=%i\nblobdir=%s\ndisplay_name=%s\nis_configured=%i\nentered_account_settings=%s\nused_account_settings=%s\ninbox_watch=%i\nsentbox_watch=%i\nmvbox_watch=%i\nmvbox_move=%i\nfolders_configured=%i\nconfigured_sentbox_folder=%s\nconfigured_mvbox_folder=%s\nmdns_enabled=%i\ne2ee_enabled=%i\nprivate_key_count=%i\npublic_key_count=%i\nfingerprint=%s\n\x00"
                       as *const u8 as *const libc::c_char,
                   b"0.42.0\x00" as *const u8 as *const libc::c_char,
                   b"3.24.0\x00" as *const u8 as *const libc::c_char,
                   sqlite3_threadsafe(), libetpan_get_version_major(),
                   libetpan_get_version_minor(),
                   (0x1000212fi64 >> 28i32) as libc::c_int,
                   (0x1000212fi64 >> 20i32) as libc::c_int & 0xffi32,
                   (0x1000212fi64 >> 12i32) as libc::c_int & 0xffi32,
                   (('a' as i32 - 1i32) as libc::c_long +
                        (0x1000212fi64 >> 4i32 & 0xffi32 as libc::c_long)) as
                       libc::c_char as libc::c_int, rpgp_enabled,
                   (::std::mem::size_of::<*mut libc::c_void>() as
                        libc::c_ulong).wrapping_mul(8i32 as libc::c_ulong),
                   chats, real_msgs, deaddrop_msgs, contacts,
                   if !(*context).dbfile.is_null() {
                       (*context).dbfile
                   } else { unset }, dbversion,
                   if !(*context).blobdir.is_null() {
                       (*context).blobdir
                   } else { unset },
                   if !displayname.is_null() { displayname } else { unset },
                   is_configured, l_readable_str, l2_readable_str,
                   inbox_watch, sentbox_watch, mvbox_watch, mvbox_move,
                   folders_configured, configured_sentbox_folder,
                   configured_mvbox_folder, mdns_enabled, e2ee_enabled,
                   prv_key_cnt, pub_key_cnt, fingerprint_str);
    dc_strbuilder_cat(&mut ret, temp);
    free(temp as *mut libc::c_void);
    dc_loginparam_unref(l);
    dc_loginparam_unref(l2);
    free(displayname as *mut libc::c_void);
    free(l_readable_str as *mut libc::c_void);
    free(l2_readable_str as *mut libc::c_void);
    free(configured_sentbox_folder as *mut libc::c_void);
    free(configured_mvbox_folder as *mut libc::c_void);
    free(fingerprint_str as *mut libc::c_void);
    dc_key_unref(self_public);
    return ret.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_version_str() -> *mut libc::c_char {
    return dc_strdup(b"0.42.0\x00" as *const u8 as *const libc::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_fresh_msgs(mut context: *mut dc_context_t) -> *mut dc_array_t {
    let mut show_deaddrop: libc::c_int = 0i32;
    let mut ret: *mut dc_array_t = dc_array_new(context, 128i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || ret.is_null()) {
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT m.id FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id LEFT JOIN chats c ON m.chat_id=c.id WHERE m.state=?   AND m.hidden=0   AND m.chat_id>?   AND ct.blocked=0   AND (c.blocked=0 OR c.blocked=?) ORDER BY m.timestamp DESC,m.id DESC;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, 10i32);
        sqlite3_bind_int(stmt, 2i32, 9i32);
        sqlite3_bind_int(stmt, 3i32, if 0 != show_deaddrop { 2i32 } else { 0i32 });
        while sqlite3_step(stmt) == 100i32 {
            dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_search_msgs(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut query: *const libc::c_char,
) -> *mut dc_array_t {
    //clock_t       start = clock();
    let mut success: libc::c_int = 0i32;
    let mut ret: *mut dc_array_t = dc_array_new(context, 100i32 as size_t);
    let mut strLikeInText: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut strLikeBeg: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut real_query: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || ret.is_null()
        || query.is_null())
    {
        real_query = dc_strdup(query);
        dc_trim(real_query);
        if *real_query.offset(0isize) as libc::c_int == 0i32 {
            success = 1i32
        } else {
            strLikeInText = dc_mprintf(
                b"%%%s%%\x00" as *const u8 as *const libc::c_char,
                real_query,
            );
            strLikeBeg = dc_mprintf(b"%s%%\x00" as *const u8 as *const libc::c_char, real_query);
            if 0 != chat_id {
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.chat_id=?  AND m.hidden=0  AND ct.blocked=0 AND (txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp,m.id;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                sqlite3_bind_text(stmt, 2i32, strLikeInText, -1i32, None);
                sqlite3_bind_text(stmt, 3i32, strLikeBeg, -1i32, None);
            } else {
                let mut show_deaddrop: libc::c_int = 0i32;
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id LEFT JOIN chats c ON m.chat_id=c.id WHERE m.chat_id>9 AND m.hidden=0  AND (c.blocked=0 OR c.blocked=?) AND ct.blocked=0 AND (m.txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp DESC,m.id DESC;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, if 0 != show_deaddrop { 2i32 } else { 0i32 });
                sqlite3_bind_text(stmt, 2i32, strLikeInText, -1i32, None);
                sqlite3_bind_text(stmt, 3i32, strLikeBeg, -1i32, None);
            }
            while sqlite3_step(stmt) == 100i32 {
                dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
            }
            success = 1i32
        }
    }
    free(strLikeInText as *mut libc::c_void);
    free(strLikeBeg as *mut libc::c_void);
    free(real_query as *mut libc::c_void);
    sqlite3_finalize(stmt);
    if 0 != success {
        return ret;
    } else {
        if !ret.is_null() {
            dc_array_unref(ret);
        }
        return 0 as *mut dc_array_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_inbox(
    mut context: *mut dc_context_t,
    mut folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut is_inbox: libc::c_int = 0i32;
    if !folder_name.is_null() {
        is_inbox = if strcasecmp(
            b"INBOX\x00" as *const u8 as *const libc::c_char,
            folder_name,
        ) == 0i32
        {
            1i32
        } else {
            0i32
        }
    }
    return is_inbox;
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_sentbox(
    mut context: *mut dc_context_t,
    mut folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut sentbox_name: *mut libc::c_char = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    let mut is_sentbox: libc::c_int = 0i32;
    if !sentbox_name.is_null() && !folder_name.is_null() {
        is_sentbox = if strcasecmp(sentbox_name, folder_name) == 0i32 {
            1i32
        } else {
            0i32
        }
    }
    free(sentbox_name as *mut libc::c_void);
    return is_sentbox;
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_mvbox(
    mut context: *mut dc_context_t,
    mut folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut mvbox_name: *mut libc::c_char = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    let mut is_mvbox: libc::c_int = 0i32;
    if !mvbox_name.is_null() && !folder_name.is_null() {
        is_mvbox = if strcasecmp(mvbox_name, folder_name) == 0i32 {
            1i32
        } else {
            0i32
        }
    }
    free(mvbox_name as *mut libc::c_void);
    return is_mvbox;
}

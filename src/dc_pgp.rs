use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type rpgp_Message;
    pub type rpgp_PublicOrSecret;
    pub type rpgp_SignedPublicKey;
    pub type rpgp_SignedSecretKey;
    #[no_mangle]
    fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn memcpy(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strspn(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strstr(_: *const libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn clock() -> clock_t;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_remove_cr_chars(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_key_set_from_binary(
        _: *mut dc_key_t,
        data: *const libc::c_void,
        bytes: libc::c_int,
        type_0: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_hash_insert(
        _: *mut dc_hash_t,
        pKey: *const libc::c_void,
        nKey: libc::c_int,
        pData: *mut libc::c_void,
    ) -> *mut libc::c_void;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn rpgp_create_rsa_skey(
        bits: uint32_t,
        user_id: *const libc::c_char,
    ) -> *mut rpgp_signed_secret_key;
    #[no_mangle]
    fn rpgp_cvec_data(cvec_ptr: *mut rpgp_cvec) -> *const uint8_t;
    #[no_mangle]
    fn rpgp_cvec_drop(cvec_ptr: *mut rpgp_cvec);
    #[no_mangle]
    fn rpgp_cvec_len(cvec_ptr: *mut rpgp_cvec) -> size_t;
    #[no_mangle]
    fn rpgp_encrypt_bytes_to_keys(
        bytes_ptr: *const uint8_t,
        bytes_len: size_t,
        pkeys_ptr: *const *const rpgp_signed_public_key,
        pkeys_len: size_t,
    ) -> *mut rpgp_message;
    #[no_mangle]
    fn rpgp_encrypt_bytes_with_password(
        bytes_ptr: *const uint8_t,
        bytes_len: size_t,
        password_ptr: *const libc::c_char,
    ) -> *mut rpgp_message;
    #[no_mangle]
    fn rpgp_key_drop(key_ptr: *mut rpgp_public_or_secret_key);
    #[no_mangle]
    fn rpgp_key_fingerprint(key_ptr: *mut rpgp_public_or_secret_key) -> *mut rpgp_cvec;
    #[no_mangle]
    fn rpgp_key_from_bytes(raw: *const uint8_t, len: size_t) -> *mut rpgp_public_or_secret_key;
    #[no_mangle]
    fn rpgp_key_is_public(key_ptr: *mut rpgp_public_or_secret_key) -> bool;
    #[no_mangle]
    fn rpgp_key_is_secret(key_ptr: *mut rpgp_public_or_secret_key) -> bool;
    #[no_mangle]
    fn rpgp_last_error_length() -> libc::c_int;
    #[no_mangle]
    fn rpgp_last_error_message() -> *mut libc::c_char;
    #[no_mangle]
    fn rpgp_message_decrypt_result_drop(res_ptr: *mut rpgp_message_decrypt_result);
    #[no_mangle]
    fn rpgp_msg_decrypt_no_pw(
        msg_ptr: *const rpgp_message,
        skeys_ptr: *const *const rpgp_signed_secret_key,
        skeys_len: size_t,
        pkeys_ptr: *const *const rpgp_signed_public_key,
        pkeys_len: size_t,
    ) -> *mut rpgp_message_decrypt_result;
    #[no_mangle]
    fn rpgp_msg_decrypt_with_password(
        msg_ptr: *const rpgp_message,
        password_ptr: *const libc::c_char,
    ) -> *mut rpgp_message;
    #[no_mangle]
    fn rpgp_msg_drop(msg_ptr: *mut rpgp_message);
    #[no_mangle]
    fn rpgp_msg_from_armor(msg_ptr: *const uint8_t, msg_len: size_t) -> *mut rpgp_message;
    #[no_mangle]
    fn rpgp_msg_from_bytes(msg_ptr: *const uint8_t, msg_len: size_t) -> *mut rpgp_message;
    #[no_mangle]
    fn rpgp_msg_to_armored(msg_ptr: *const rpgp_message) -> *mut rpgp_cvec;
    #[no_mangle]
    fn rpgp_msg_to_armored_str(msg_ptr: *const rpgp_message) -> *mut libc::c_char;
    #[no_mangle]
    fn rpgp_msg_to_bytes(msg_ptr: *const rpgp_message) -> *mut rpgp_cvec;
    #[no_mangle]
    fn rpgp_pkey_drop(pkey_ptr: *mut rpgp_signed_public_key);
    #[no_mangle]
    fn rpgp_pkey_from_bytes(raw: *const uint8_t, len: size_t) -> *mut rpgp_signed_public_key;
    #[no_mangle]
    fn rpgp_pkey_to_bytes(pkey_ptr: *mut rpgp_signed_public_key) -> *mut rpgp_cvec;
    #[no_mangle]
    fn rpgp_sign_encrypt_bytes_to_keys(
        bytes_ptr: *const uint8_t,
        bytes_len: size_t,
        pkeys_ptr: *const *const rpgp_signed_public_key,
        pkeys_len: size_t,
        skey_ptr: *const rpgp_signed_secret_key,
    ) -> *mut rpgp_message;
    #[no_mangle]
    fn rpgp_skey_drop(skey_ptr: *mut rpgp_signed_secret_key);
    #[no_mangle]
    fn rpgp_skey_from_bytes(raw: *const uint8_t, len: size_t) -> *mut rpgp_signed_secret_key;
    #[no_mangle]
    fn rpgp_skey_public_key(skey_ptr: *mut rpgp_signed_secret_key) -> *mut rpgp_signed_public_key;
    #[no_mangle]
    fn rpgp_skey_to_bytes(skey_ptr: *mut rpgp_signed_secret_key) -> *mut rpgp_cvec;
    #[no_mangle]
    fn rpgp_string_drop(p: *mut libc::c_char);
}
pub type __darwin_size_t = libc::c_ulong;
pub type __darwin_clock_t = libc::c_ulong;
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
pub type uint32_t = libc::c_uint;
pub type ssize_t = __darwin_ssize_t;
pub type clock_t = __darwin_clock_t;
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
/* A complete hash table is an instance of the following structure.
 * The internals of this structure are intended to be opaque -- client
 * code should not attempt to access or modify the fields of this structure
 * directly.  Change this structure only by using the routines below.
 * However, many of the "procedures" and "functions" for modifying and
 * accessing this structure are really macros, so we can't really make
 * this structure opaque.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_hash {
    pub keyClass: libc::c_char,
    pub copyKey: libc::c_char,
    pub count: libc::c_int,
    pub first: *mut dc_hashelem_t,
    pub htsize: libc::c_int,
    pub ht: *mut _ht,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _ht {
    pub count: libc::c_int,
    pub chain: *mut dc_hashelem_t,
}
pub type dc_hashelem_t = _dc_hashelem;
/* Each element in the hash table is an instance of the following
 * structure.  All elements are stored on a single doubly-linked list.
 *
 * Again, this structure is intended to be opaque, but it can't really
 * be opaque because it is used by macros.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_hashelem {
    pub next: *mut dc_hashelem_t,
    pub prev: *mut dc_hashelem_t,
    pub data: *mut libc::c_void,
    pub pKey: *mut libc::c_void,
    pub nKey: libc::c_int,
}
/* Forward declarations of structures.
 */
pub type dc_hash_t = _dc_hash;
pub type rpgp_signed_secret_key = rpgp_SignedSecretKey;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rpgp_cvec {
    pub data: *mut uint8_t,
    pub len: size_t,
}
pub type rpgp_message = rpgp_Message;
pub type rpgp_signed_public_key = rpgp_SignedPublicKey;
pub type rpgp_public_or_secret_key = rpgp_PublicOrSecret;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rpgp_message_decrypt_result {
    pub message_ptr: *mut rpgp_message,
    pub valid_ids_ptr: *mut *mut libc::c_char,
    pub valid_ids_len: size_t,
}
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_keyring {
    pub keys: *mut *mut dc_key_t,
    pub count: libc::c_int,
    pub allocated: libc::c_int,
}
pub type dc_keyring_t = _dc_keyring;
/* ** library-private **********************************************************/
/* validation errors */
/* misc. */
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_init() {}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_exit() {}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_rand_seed(
    mut context: *mut dc_context_t,
    mut buf: *const libc::c_void,
    mut bytes: size_t,
) {
}
#[no_mangle]
pub unsafe extern "C" fn dc_split_armored_data(
    mut buf: *mut libc::c_char,
    mut ret_headerline: *mut *const libc::c_char,
    mut ret_setupcodebegin: *mut *const libc::c_char,
    mut ret_preferencrypt: *mut *const libc::c_char,
    mut ret_base64: *mut *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut line_chars: size_t = 0i32 as size_t;
    let mut line: *mut libc::c_char = buf;
    let mut p1: *mut libc::c_char = buf;
    let mut p2: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut headerline: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut base64: *mut libc::c_char = 0 as *mut libc::c_char;
    if !ret_headerline.is_null() {
        *ret_headerline = 0 as *const libc::c_char
    }
    if !ret_setupcodebegin.is_null() {
        *ret_setupcodebegin = 0 as *const libc::c_char
    }
    if !ret_preferencrypt.is_null() {
        *ret_preferencrypt = 0 as *const libc::c_char
    }
    if !ret_base64.is_null() {
        *ret_base64 = 0 as *const libc::c_char
    }
    if !(buf.is_null() || ret_headerline.is_null()) {
        dc_remove_cr_chars(buf);
        while 0 != *p1 {
            if *p1 as libc::c_int == '\n' as i32 {
                *line.offset(line_chars as isize) = 0i32 as libc::c_char;
                if headerline.is_null() {
                    dc_trim(line);
                    if strncmp(
                        line,
                        b"-----BEGIN \x00" as *const u8 as *const libc::c_char,
                        11i32 as libc::c_ulong,
                    ) == 0i32
                        && strncmp(
                            &mut *line
                                .offset(strlen(line).wrapping_sub(5i32 as libc::c_ulong) as isize),
                            b"-----\x00" as *const u8 as *const libc::c_char,
                            5i32 as libc::c_ulong,
                        ) == 0i32
                    {
                        headerline = line;
                        if !ret_headerline.is_null() {
                            *ret_headerline = headerline
                        }
                    }
                } else if strspn(line, b"\t\r\n \x00" as *const u8 as *const libc::c_char)
                    == strlen(line)
                {
                    base64 = p1.offset(1isize);
                    break;
                } else {
                    p2 = strchr(line, ':' as i32);
                    if p2.is_null() {
                        *line.offset(line_chars as isize) = '\n' as i32 as libc::c_char;
                        base64 = line;
                        break;
                    } else {
                        *p2 = 0i32 as libc::c_char;
                        dc_trim(line);
                        if strcasecmp(
                            line,
                            b"Passphrase-Begin\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        {
                            p2 = p2.offset(1isize);
                            dc_trim(p2);
                            if !ret_setupcodebegin.is_null() {
                                *ret_setupcodebegin = p2
                            }
                        } else if strcasecmp(
                            line,
                            b"Autocrypt-Prefer-Encrypt\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        {
                            p2 = p2.offset(1isize);
                            dc_trim(p2);
                            if !ret_preferencrypt.is_null() {
                                *ret_preferencrypt = p2
                            }
                        }
                    }
                }
                p1 = p1.offset(1isize);
                line = p1;
                line_chars = 0i32 as size_t
            } else {
                p1 = p1.offset(1isize);
                line_chars = line_chars.wrapping_add(1)
            }
        }
        if !(headerline.is_null() || base64.is_null()) {
            /* now, line points to beginning of base64 data, search end */
            /*the trailing space makes sure, this is not a normal base64 sequence*/
            p1 = strstr(base64, b"-----END \x00" as *const u8 as *const libc::c_char);
            if !(p1.is_null()
                || strncmp(
                    p1.offset(9isize),
                    headerline.offset(11isize),
                    strlen(headerline.offset(11isize)),
                ) != 0i32)
            {
                *p1 = 0i32 as libc::c_char;
                dc_trim(base64);
                if !ret_base64.is_null() {
                    *ret_base64 = base64
                }
                success = 1i32
            }
        }
    }
    return success;
}
/* public key encryption */
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_create_keypair(
    mut context: *mut dc_context_t,
    mut addr: *const libc::c_char,
    mut ret_public_key: *mut dc_key_t,
    mut ret_private_key: *mut dc_key_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut skey: *mut rpgp_signed_secret_key = 0 as *mut rpgp_signed_secret_key;
    let mut pkey: *mut rpgp_signed_public_key = 0 as *mut rpgp_signed_public_key;
    let mut skey_bytes: *mut rpgp_cvec = 0 as *mut rpgp_cvec;
    let mut pkey_bytes: *mut rpgp_cvec = 0 as *mut rpgp_cvec;
    let mut user_id: *mut libc::c_char = 0 as *mut libc::c_char;
    user_id = dc_mprintf(b"<%s>\x00" as *const u8 as *const libc::c_char, addr);
    skey = rpgp_create_rsa_skey(2048i32 as uint32_t, user_id);
    if !(0 != dc_pgp_handle_rpgp_error(context)) {
        skey_bytes = rpgp_skey_to_bytes(skey);
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            pkey = rpgp_skey_public_key(skey);
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                pkey_bytes = rpgp_pkey_to_bytes(pkey);
                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                    dc_key_set_from_binary(
                        ret_private_key,
                        rpgp_cvec_data(skey_bytes) as *const libc::c_void,
                        rpgp_cvec_len(skey_bytes) as libc::c_int,
                        1i32,
                    );
                    if !(0 != dc_pgp_handle_rpgp_error(context)) {
                        dc_key_set_from_binary(
                            ret_public_key,
                            rpgp_cvec_data(pkey_bytes) as *const libc::c_void,
                            rpgp_cvec_len(pkey_bytes) as libc::c_int,
                            0i32,
                        );
                        if !(0 != dc_pgp_handle_rpgp_error(context)) {
                            success = 1i32
                        }
                    }
                }
            }
        }
    }
    /* cleanup */
    if !skey.is_null() {
        rpgp_skey_drop(skey);
    }
    if !skey_bytes.is_null() {
        rpgp_cvec_drop(skey_bytes);
    }
    if !pkey.is_null() {
        rpgp_pkey_drop(pkey);
    }
    if !pkey_bytes.is_null() {
        rpgp_cvec_drop(pkey_bytes);
    }
    if !user_id.is_null() {
        free(user_id as *mut libc::c_void);
    }
    return success;
}
/* returns 0 if there is no error, otherwise logs the error if a context is provided and returns 1*/
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_handle_rpgp_error(mut context: *mut dc_context_t) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut len: libc::c_int = 0i32;
    let mut msg: *mut libc::c_char = 0 as *mut libc::c_char;
    len = rpgp_last_error_length();
    if !(len == 0i32) {
        msg = rpgp_last_error_message();
        if !context.is_null() {
            dc_log_info(
                context,
                0i32,
                b"[rpgp][error] %s\x00" as *const u8 as *const libc::c_char,
                msg,
            );
        }
        success = 1i32
    }
    if !msg.is_null() {
        rpgp_string_drop(msg);
    }
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_is_valid_key(
    mut context: *mut dc_context_t,
    mut raw_key: *const dc_key_t,
) -> libc::c_int {
    let mut key_is_valid: libc::c_int = 0i32;
    let mut key: *mut rpgp_public_or_secret_key = 0 as *mut rpgp_public_or_secret_key;
    if !(context.is_null()
        || raw_key.is_null()
        || (*raw_key).binary.is_null()
        || (*raw_key).bytes <= 0i32)
    {
        key = rpgp_key_from_bytes(
            (*raw_key).binary as *const uint8_t,
            (*raw_key).bytes as size_t,
        );
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            if (*raw_key).type_0 == 0i32 && 0 != rpgp_key_is_public(key) as libc::c_int {
                key_is_valid = 1i32
            } else if (*raw_key).type_0 == 1i32 && 0 != rpgp_key_is_secret(key) as libc::c_int {
                key_is_valid = 1i32
            }
        }
    }
    if !key.is_null() {
        rpgp_key_drop(key);
    }
    return key_is_valid;
}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_calc_fingerprint(
    mut raw_key: *const dc_key_t,
    mut ret_fingerprint: *mut *mut uint8_t,
    mut ret_fingerprint_bytes: *mut size_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut key: *mut rpgp_public_or_secret_key = 0 as *mut rpgp_public_or_secret_key;
    let mut fingerprint: *mut rpgp_cvec = 0 as *mut rpgp_cvec;
    if !(raw_key.is_null()
        || ret_fingerprint.is_null()
        || !(*ret_fingerprint).is_null()
        || ret_fingerprint_bytes.is_null()
        || *ret_fingerprint_bytes != 0i32 as libc::c_ulong
        || (*raw_key).binary.is_null()
        || (*raw_key).bytes <= 0i32)
    {
        key = rpgp_key_from_bytes(
            (*raw_key).binary as *const uint8_t,
            (*raw_key).bytes as size_t,
        );
        if !(0 != dc_pgp_handle_rpgp_error(0 as *mut dc_context_t)) {
            fingerprint = rpgp_key_fingerprint(key);
            if !(0 != dc_pgp_handle_rpgp_error(0 as *mut dc_context_t)) {
                *ret_fingerprint_bytes = rpgp_cvec_len(fingerprint);
                *ret_fingerprint = malloc(*ret_fingerprint_bytes) as *mut uint8_t;
                memcpy(
                    *ret_fingerprint as *mut libc::c_void,
                    rpgp_cvec_data(fingerprint) as *const libc::c_void,
                    *ret_fingerprint_bytes,
                );
                success = 1i32
            }
        }
    }
    if !key.is_null() {
        rpgp_key_drop(key);
    }
    if !fingerprint.is_null() {
        rpgp_cvec_drop(fingerprint);
    }
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_split_key(
    mut context: *mut dc_context_t,
    mut private_in: *const dc_key_t,
    mut ret_public_key: *mut dc_key_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut key: *mut rpgp_signed_secret_key = 0 as *mut rpgp_signed_secret_key;
    let mut pub_key: *mut rpgp_signed_public_key = 0 as *mut rpgp_signed_public_key;
    let mut buf: *mut rpgp_cvec = 0 as *mut rpgp_cvec;
    if !(context.is_null() || private_in.is_null() || ret_public_key.is_null()) {
        if (*private_in).type_0 != 1i32 {
            dc_log_warning(
                context,
                0i32,
                b"Split key: Given key is no private key.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            key = rpgp_skey_from_bytes(
                (*private_in).binary as *const uint8_t,
                (*private_in).bytes as size_t,
            );
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                pub_key = rpgp_skey_public_key(key);
                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                    buf = rpgp_pkey_to_bytes(pub_key);
                    if !(0 != dc_pgp_handle_rpgp_error(context)) {
                        dc_key_set_from_binary(
                            ret_public_key,
                            rpgp_cvec_data(buf) as *const libc::c_void,
                            rpgp_cvec_len(buf) as libc::c_int,
                            0i32,
                        );
                        success = 1i32
                    }
                }
            }
        }
    }
    if !key.is_null() {
        rpgp_skey_drop(key);
    }
    if !pub_key.is_null() {
        rpgp_pkey_drop(pub_key);
    }
    if !buf.is_null() {
        rpgp_cvec_drop(buf);
    }
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_pk_encrypt(
    mut context: *mut dc_context_t,
    mut plain_text: *const libc::c_void,
    mut plain_bytes: size_t,
    mut raw_public_keys_for_encryption: *const dc_keyring_t,
    mut raw_private_key_for_signing: *const dc_key_t,
    mut use_armor: libc::c_int,
    mut ret_ctext: *mut *mut libc::c_void,
    mut ret_ctext_bytes: *mut size_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut i: libc::c_int = 0i32;
    let mut success: libc::c_int = 0i32;
    let mut public_keys_len: libc::c_int = 0i32;
    let mut public_keys: *mut *mut rpgp_signed_public_key = 0 as *mut *mut rpgp_signed_public_key;
    let mut private_key: *mut rpgp_signed_secret_key = 0 as *mut rpgp_signed_secret_key;
    let mut encrypted: *mut rpgp_message = 0 as *mut rpgp_message;
    if !(context.is_null()
        || plain_text == 0 as *mut libc::c_void
        || plain_bytes == 0i32 as libc::c_ulong
        || ret_ctext.is_null()
        || ret_ctext_bytes.is_null()
        || raw_public_keys_for_encryption.is_null()
        || (*raw_public_keys_for_encryption).count <= 0i32
        || use_armor == 0i32)
    {
        /* only support use_armor=1 */
        *ret_ctext = 0 as *mut libc::c_void;
        *ret_ctext_bytes = 0i32 as size_t;
        public_keys_len = (*raw_public_keys_for_encryption).count;
        public_keys = malloc(
            (::std::mem::size_of::<*mut rpgp_signed_public_key>() as libc::c_ulong)
                .wrapping_mul(public_keys_len as libc::c_ulong),
        ) as *mut *mut rpgp_signed_public_key;
        /* setup secret key for signing */
        if !raw_private_key_for_signing.is_null() {
            private_key = rpgp_skey_from_bytes(
                (*raw_private_key_for_signing).binary as *const uint8_t,
                (*raw_private_key_for_signing).bytes as size_t,
            );
            if private_key.is_null() || 0 != dc_pgp_handle_rpgp_error(context) {
                dc_log_warning(
                    context,
                    0i32,
                    b"No key for signing found.\x00" as *const u8 as *const libc::c_char,
                );
                current_block = 2132137392766895896;
            } else {
                current_block = 12800627514080957624;
            }
        } else {
            current_block = 12800627514080957624;
        }
        match current_block {
            2132137392766895896 => {}
            _ => {
                /* setup public keys for encryption */
                i = 0i32;
                loop {
                    if !(i < public_keys_len) {
                        current_block = 6057473163062296781;
                        break;
                    }
                    let ref mut fresh0 = *public_keys.offset(i as isize);
                    *fresh0 = rpgp_pkey_from_bytes(
                        (**(*raw_public_keys_for_encryption).keys.offset(i as isize)).binary
                            as *const uint8_t,
                        (**(*raw_public_keys_for_encryption).keys.offset(i as isize)).bytes
                            as size_t,
                    );
                    if 0 != dc_pgp_handle_rpgp_error(context) {
                        current_block = 2132137392766895896;
                        break;
                    }
                    i += 1
                }
                match current_block {
                    2132137392766895896 => {}
                    _ => {
                        /* sign & encrypt */
                        let mut op_clocks: clock_t = 0i32 as clock_t;
                        let mut start: clock_t = clock();
                        if private_key.is_null() {
                            encrypted = rpgp_encrypt_bytes_to_keys(
                                plain_text as *const uint8_t,
                                plain_bytes,
                                public_keys as *const *const rpgp_signed_public_key,
                                public_keys_len as size_t,
                            );
                            if 0 != dc_pgp_handle_rpgp_error(context) {
                                dc_log_warning(
                                    context,
                                    0i32,
                                    b"Encryption failed.\x00" as *const u8 as *const libc::c_char,
                                );
                                current_block = 2132137392766895896;
                            } else {
                                op_clocks = clock().wrapping_sub(start);
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"Message encrypted in %.3f ms.\x00" as *const u8
                                        as *const libc::c_char,
                                    op_clocks as libc::c_double * 1000.0f64
                                        / 1000000i32 as libc::c_double,
                                );
                                current_block = 1538046216550696469;
                            }
                        } else {
                            encrypted = rpgp_sign_encrypt_bytes_to_keys(
                                plain_text as *const uint8_t,
                                plain_bytes,
                                public_keys as *const *const rpgp_signed_public_key,
                                public_keys_len as size_t,
                                private_key,
                            );
                            if 0 != dc_pgp_handle_rpgp_error(context) {
                                dc_log_warning(
                                    context,
                                    0i32,
                                    b"Signing and encrypting failed.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                current_block = 2132137392766895896;
                            } else {
                                op_clocks = clock().wrapping_sub(start);
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"Message signed and encrypted in %.3f ms.\x00" as *const u8
                                        as *const libc::c_char,
                                    op_clocks as libc::c_double * 1000.0f64
                                        / 1000000i32 as libc::c_double,
                                );
                                current_block = 1538046216550696469;
                            }
                        }
                        match current_block {
                            2132137392766895896 => {}
                            _ => {
                                /* convert message to armored bytes and return values */
                                let mut armored: *mut rpgp_cvec = rpgp_msg_to_armored(encrypted);
                                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                                    *ret_ctext = rpgp_cvec_data(armored) as *mut libc::c_void;
                                    *ret_ctext_bytes = rpgp_cvec_len(armored);
                                    free(armored as *mut libc::c_void);
                                    success = 1i32
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if !private_key.is_null() {
        rpgp_skey_drop(private_key);
    }
    i = 0i32;
    while i < public_keys_len {
        rpgp_pkey_drop(*public_keys.offset(i as isize));
        i += 1
    }
    if !encrypted.is_null() {
        rpgp_msg_drop(encrypted);
    }
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_pk_decrypt(
    mut context: *mut dc_context_t,
    mut ctext: *const libc::c_void,
    mut ctext_bytes: size_t,
    mut raw_private_keys_for_decryption: *const dc_keyring_t,
    mut raw_public_keys_for_validation: *const dc_keyring_t,
    mut use_armor: libc::c_int,
    mut ret_plain: *mut *mut libc::c_void,
    mut ret_plain_bytes: *mut size_t,
    mut ret_signature_fingerprints: *mut dc_hash_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut i: libc::c_int = 0i32;
    let mut success: libc::c_int = 0i32;
    let mut encrypted: *mut rpgp_message = 0 as *mut rpgp_message;
    let mut decrypted: *mut rpgp_message_decrypt_result = 0 as *mut rpgp_message_decrypt_result;
    let mut private_keys_len: libc::c_int = 0i32;
    let mut public_keys_len: libc::c_int = 0i32;
    let mut private_keys: *mut *mut rpgp_signed_secret_key = 0 as *mut *mut rpgp_signed_secret_key;
    let mut public_keys: *mut *mut rpgp_signed_public_key = 0 as *mut *mut rpgp_signed_public_key;
    if !(context.is_null()
        || ctext == 0 as *mut libc::c_void
        || ctext_bytes == 0i32 as libc::c_ulong
        || ret_plain.is_null()
        || ret_plain_bytes.is_null()
        || raw_private_keys_for_decryption.is_null()
        || (*raw_private_keys_for_decryption).count <= 0i32
        || use_armor == 0i32)
    {
        /* only support use_armor=1 */
        *ret_plain = 0 as *mut libc::c_void;
        *ret_plain_bytes = 0i32 as size_t;
        private_keys_len = (*raw_private_keys_for_decryption).count;
        private_keys = malloc(
            (::std::mem::size_of::<*mut rpgp_signed_secret_key>() as libc::c_ulong)
                .wrapping_mul(private_keys_len as libc::c_ulong),
        ) as *mut *mut rpgp_signed_secret_key;
        if !raw_public_keys_for_validation.is_null() {
            public_keys_len = (*raw_public_keys_for_validation).count;
            public_keys = malloc(
                (::std::mem::size_of::<*mut rpgp_signed_public_key>() as libc::c_ulong)
                    .wrapping_mul(public_keys_len as libc::c_ulong),
            ) as *mut *mut rpgp_signed_public_key
        }
        /* setup secret keys for decryption */
        i = 0i32;
        loop {
            if !(i < (*raw_private_keys_for_decryption).count) {
                current_block = 15904375183555213903;
                break;
            }
            let ref mut fresh1 = *private_keys.offset(i as isize);
            *fresh1 = rpgp_skey_from_bytes(
                (**(*raw_private_keys_for_decryption).keys.offset(i as isize)).binary
                    as *const uint8_t,
                (**(*raw_private_keys_for_decryption).keys.offset(i as isize)).bytes as size_t,
            );
            if 0 != dc_pgp_handle_rpgp_error(context) {
                current_block = 11904635156640512504;
                break;
            }
            i += 1
        }
        match current_block {
            11904635156640512504 => {}
            _ => {
                /* setup public keys for validation */
                if !raw_public_keys_for_validation.is_null() {
                    i = 0i32;
                    loop {
                        if !(i < (*raw_public_keys_for_validation).count) {
                            current_block = 7172762164747879670;
                            break;
                        }
                        let ref mut fresh2 = *public_keys.offset(i as isize);
                        *fresh2 = rpgp_pkey_from_bytes(
                            (**(*raw_public_keys_for_validation).keys.offset(i as isize)).binary
                                as *const uint8_t,
                            (**(*raw_public_keys_for_validation).keys.offset(i as isize)).bytes
                                as size_t,
                        );
                        if 0 != dc_pgp_handle_rpgp_error(context) {
                            current_block = 11904635156640512504;
                            break;
                        }
                        i += 1
                    }
                } else {
                    current_block = 7172762164747879670;
                }
                match current_block {
                    11904635156640512504 => {}
                    _ => {
                        /* decrypt */
                        encrypted = rpgp_msg_from_armor(ctext as *const uint8_t, ctext_bytes);
                        if !(0 != dc_pgp_handle_rpgp_error(context)) {
                            decrypted = rpgp_msg_decrypt_no_pw(
                                encrypted,
                                private_keys as *const *const rpgp_signed_secret_key,
                                private_keys_len as size_t,
                                public_keys as *const *const rpgp_signed_public_key,
                                public_keys_len as size_t,
                            );
                            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                                let mut decrypted_bytes: *mut rpgp_cvec =
                                    rpgp_msg_to_bytes((*decrypted).message_ptr);
                                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                                    *ret_plain_bytes = rpgp_cvec_len(decrypted_bytes);
                                    *ret_plain =
                                        rpgp_cvec_data(decrypted_bytes) as *mut libc::c_void;
                                    free(decrypted_bytes as *mut libc::c_void);
                                    if !ret_signature_fingerprints.is_null() {
                                        let mut j: uint32_t = 0i32 as uint32_t;
                                        let mut len: uint32_t =
                                            (*decrypted).valid_ids_len as uint32_t;
                                        while j < len {
                                            let mut fingerprint_hex: *mut libc::c_char =
                                                *(*decrypted).valid_ids_ptr.offset(j as isize);
                                            if !fingerprint_hex.is_null() {
                                                dc_hash_insert(
                                                    ret_signature_fingerprints,
                                                    fingerprint_hex as *const libc::c_void,
                                                    strlen(fingerprint_hex) as libc::c_int,
                                                    1i32 as *mut libc::c_void,
                                                );
                                                free(fingerprint_hex as *mut libc::c_void);
                                            }
                                            j = j.wrapping_add(1)
                                        }
                                    }
                                    success = 1i32
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    i = 0i32;
    while i < private_keys_len {
        rpgp_skey_drop(*private_keys.offset(i as isize));
        i += 1
    }
    i = 0i32;
    while i < public_keys_len {
        rpgp_pkey_drop(*public_keys.offset(i as isize));
        i += 1
    }
    if !encrypted.is_null() {
        rpgp_msg_drop(encrypted);
    }
    if !decrypted.is_null() {
        rpgp_message_decrypt_result_drop(decrypted);
    }
    return success;
}
/* symm. encryption */
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_symm_encrypt(
    mut context: *mut dc_context_t,
    mut passphrase: *const libc::c_char,
    mut plain: *const libc::c_void,
    mut plain_bytes: size_t,
    mut ret_ctext_armored: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut decrypted: *mut rpgp_message = 0 as *mut rpgp_message;
    if !(context.is_null()
        || passphrase.is_null()
        || plain == 0 as *mut libc::c_void
        || plain_bytes == 0i32 as libc::c_ulong
        || ret_ctext_armored.is_null())
    {
        decrypted =
            rpgp_encrypt_bytes_with_password(plain as *const uint8_t, plain_bytes, passphrase);
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            *ret_ctext_armored = rpgp_msg_to_armored_str(decrypted);
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                success = 1i32
            }
        }
    }
    if !decrypted.is_null() {
        rpgp_msg_drop(decrypted);
    }
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_pgp_symm_decrypt(
    mut context: *mut dc_context_t,
    mut passphrase: *const libc::c_char,
    mut ctext: *const libc::c_void,
    mut ctext_bytes: size_t,
    mut ret_plain_text: *mut *mut libc::c_void,
    mut ret_plain_bytes: *mut size_t,
) -> libc::c_int {
    let mut decrypted_bytes: *mut rpgp_cvec = 0 as *mut rpgp_cvec;
    let mut success: libc::c_int = 0i32;
    let mut encrypted: *mut rpgp_message = 0 as *mut rpgp_message;
    let mut decrypted: *mut rpgp_message = 0 as *mut rpgp_message;
    encrypted = rpgp_msg_from_bytes(ctext as *const uint8_t, ctext_bytes);
    if !(0 != dc_pgp_handle_rpgp_error(context)) {
        decrypted = rpgp_msg_decrypt_with_password(encrypted, passphrase);
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            decrypted_bytes = rpgp_msg_to_bytes(decrypted);
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                *ret_plain_text = rpgp_cvec_data(decrypted_bytes) as *mut libc::c_void;
                *ret_plain_bytes = rpgp_cvec_len(decrypted_bytes);
                free(decrypted_bytes as *mut libc::c_void);
                success = 1i32
            }
        }
    }
    if !encrypted.is_null() {
        rpgp_msg_drop(encrypted);
    }
    if !decrypted.is_null() {
        rpgp_msg_drop(decrypted);
    }
    return success;
}

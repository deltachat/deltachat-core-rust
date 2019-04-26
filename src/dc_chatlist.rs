use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_context::dc_context_t;
use crate::dc_lot::dc_lot_t;

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
    fn dc_array_new(_: *mut dc_context_t, initsize: size_t) -> *mut dc_array_t;
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
    fn dc_array_empty(_: *mut dc_array_t);
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_array_get_cnt(_: *const dc_array_t) -> size_t;
    #[no_mangle]
    fn dc_array_add_id(_: *mut dc_array_t, _: uint32_t);
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_text(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_char,
        _: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_array_get_id(_: *const dc_array_t, index: size_t) -> uint32_t;
    /* *
     * @class dc_lot_t
     *
     * An object containing a set of values.
     * The meaning of the values is defined by the function returning the object.
     * Lot objects are created
     * eg. by dc_chatlist_get_summary() or dc_msg_get_summary().
     *
     * NB: _Lot_ is used in the meaning _heap_ here.
     */
    #[no_mangle]
    fn dc_lot_new() -> *mut dc_lot_t;
    #[no_mangle]
    fn dc_chat_unref(_: *mut dc_chat_t);
    #[no_mangle]
    fn dc_contact_unref(_: *mut dc_contact_t);
    #[no_mangle]
    fn dc_msg_unref(_: *mut dc_msg_t);
    /* library-internal */
    /* in practice, the user additionally cuts the string himself pixel-accurate */
    #[no_mangle]
    fn dc_lot_fill(
        _: *mut dc_lot_t,
        _: *const dc_msg_t,
        _: *const dc_chat_t,
        _: *const dc_contact_t,
        _: *mut dc_context_t,
    );
    /* Return the string with the given ID by calling DC_EVENT_GET_STRING.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str(_: *mut dc_context_t, id: libc::c_int) -> *mut libc::c_char;
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
    fn dc_contact_load_from_db(
        _: *mut dc_contact_t,
        _: *mut dc_sqlite3_t,
        contact_id: uint32_t,
    ) -> libc::c_int;
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
    fn dc_contact_new(_: *mut dc_context_t) -> *mut dc_contact_t;
    #[no_mangle]
    fn dc_msg_load_from_db(_: *mut dc_msg_t, _: *mut dc_context_t, id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_new_untyped(_: *mut dc_context_t) -> *mut dc_msg_t;
    #[no_mangle]
    fn dc_chat_load_from_db(_: *mut dc_chat_t, id: uint32_t) -> libc::c_int;
    /* *
     * @class dc_chat_t
     *
     * An object representing a single chat in memory.
     * Chat objects are created using eg. dc_get_chat()
     * and are not updated on database changes;
     * if you want an update, you have to recreate the object.
     */
    // virtual chat showing all messages belonging to chats flagged with chats.blocked=2
    // messages that should be deleted get this chat_id; the messages are deleted from the working thread later then. This is also needed as rfc724_mid should be preset as long as the message is not deleted on the server (otherwise it is downloaded again)
    // a message is just in creation but not yet assigned to a chat (eg. we may need the message ID to set up blobs; this avoids unready message to be sent and shown)
    // virtual chat showing all messages flagged with msgs.starred=2
    // only an indicator in a chatlist
    // only an indicator in a chatlist
    // larger chat IDs are "real" chats, their messages are "real" messages.
    #[no_mangle]
    fn dc_chat_new(_: *mut dc_context_t) -> *mut dc_chat_t;
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
/* * the structure behind dc_chatlist_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_chatlist {
    pub magic: uint32_t,
    pub context: *mut dc_context_t,
    pub cnt: size_t,
    pub chatNlastmsg_ids: *mut dc_array_t,
}
pub type dc_chatlist_t = _dc_chatlist;
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
// handle chatlists
#[no_mangle]
pub unsafe extern "C" fn dc_get_chatlist(
    mut context: *mut dc_context_t,
    mut listflags: libc::c_int,
    mut query_str: *const libc::c_char,
    mut query_id: uint32_t,
) -> *mut dc_chatlist_t {
    let mut success: libc::c_int = 0i32;
    let mut obj: *mut dc_chatlist_t = dc_chatlist_new(context);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(0 == dc_chatlist_load_from_db(obj, listflags, query_str, query_id)) {
            success = 1i32
        }
    }
    if 0 != success {
        return obj;
    } else {
        dc_chatlist_unref(obj);
        return 0 as *mut dc_chatlist_t;
    };
}
/* *
 * @class dc_chatlist_t
 *
 * An object representing a single chatlist in memory.
 * Chatlist objects contain chat IDs
 * and, if possible, message IDs belonging to them.
 * The chatlist object is not updated;
 * if you want an update, you have to recreate the object.
 *
 * For a **typical chat overview**,
 * the idea is to get the list of all chats via dc_get_chatlist()
 * without any listflags (see below)
 * and to implement a "virtual list" or so
 * (the count of chats is known by dc_chatlist_get_cnt()).
 *
 * Only for the items that are in view
 * (the list may have several hundreds chats),
 * the UI should call dc_chatlist_get_summary() then.
 * dc_chatlist_get_summary() provides all elements needed for painting the item.
 *
 * On a click of such an item,
 * the UI should change to the chat view
 * and get all messages from this view via dc_get_chat_msgs().
 * Again, a "virtual list" is created
 * (the count of messages is known)
 * and for each messages that is scrolled into view, dc_get_msg() is called then.
 *
 * Why no listflags?
 * Without listflags, dc_get_chatlist() adds the deaddrop
 * and the archive "link" automatically as needed.
 * The UI can just render these items differently then.
 * Although the deaddrop link is currently always the first entry
 * and only present on new messages,
 * there is the rough idea that it can be optionally always present
 * and sorted into the list by date.
 * Rendering the deaddrop in the described way
 * would not add extra work in the UI then.
 */
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_new(mut context: *mut dc_context_t) -> *mut dc_chatlist_t {
    let mut chatlist: *mut dc_chatlist_t = 0 as *mut dc_chatlist_t;
    chatlist = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_chatlist_t>() as libc::c_ulong,
    ) as *mut dc_chatlist_t;
    if chatlist.is_null() {
        exit(20i32);
    }
    (*chatlist).magic = 0xc4a71157u32;
    (*chatlist).context = context;
    (*chatlist).chatNlastmsg_ids = dc_array_new(context, 128i32 as size_t);
    if (*chatlist).chatNlastmsg_ids.is_null() {
        exit(32i32);
    }
    return chatlist;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_unref(mut chatlist: *mut dc_chatlist_t) {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return;
    }
    dc_chatlist_empty(chatlist);
    dc_array_unref((*chatlist).chatNlastmsg_ids);
    (*chatlist).magic = 0i32 as uint32_t;
    free(chatlist as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_empty(mut chatlist: *mut dc_chatlist_t) {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return;
    }
    (*chatlist).cnt = 0i32 as size_t;
    dc_array_empty((*chatlist).chatNlastmsg_ids);
}
/* *
 * Load a chatlist from the database to the chatlist object.
 *
 * @private @memberof dc_chatlist_t
 */
unsafe extern "C" fn dc_chatlist_load_from_db(
    mut chatlist: *mut dc_chatlist_t,
    mut listflags: libc::c_int,
    mut query__: *const libc::c_char,
    mut query_contact_id: uint32_t,
) -> libc::c_int {
    let mut current_block: u64;
    //clock_t       start = clock();
    let mut success: libc::c_int = 0i32;
    let mut add_archived_link_item: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut strLikeCmd: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut query: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 || (*chatlist).context.is_null())
    {
        dc_chatlist_empty(chatlist);
        // select with left join and minimum:
        // - the inner select must use `hidden` and _not_ `m.hidden`
        //   which would refer the outer select and take a lot of time
        // - `GROUP BY` is needed several messages may have the same timestamp
        // - the list starts with the newest chats
        // nb: the query currently shows messages from blocked contacts in groups.
        // however, for normal-groups, this is okay as the message is also returned by dc_get_chat_msgs()
        // (otherwise it would be hard to follow conversations, wa and tg do the same)
        // for the deaddrop, however, they should really be hidden, however, _currently_ the deaddrop is not
        // shown at all permanent in the chatlist.
        if 0 != query_contact_id {
            stmt =
                dc_sqlite3_prepare((*(*chatlist).context).sql,
                                   b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?)  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, query_contact_id as libc::c_int);
            current_block = 3437258052017859086;
        } else if 0 != listflags & 0x1i32 {
            stmt =
                dc_sqlite3_prepare((*(*chatlist).context).sql,
                                   b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.archived=1  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                       as *const u8 as *const libc::c_char);
            current_block = 3437258052017859086;
        } else if query__.is_null() {
            if 0 == listflags & 0x2i32 {
                let mut last_deaddrop_fresh_msg_id: uint32_t =
                    get_last_deaddrop_fresh_msg((*chatlist).context);
                if last_deaddrop_fresh_msg_id > 0i32 as libc::c_uint {
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, 1i32 as uint32_t);
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, last_deaddrop_fresh_msg_id);
                }
                add_archived_link_item = 1i32
            }
            stmt =
                dc_sqlite3_prepare((*(*chatlist).context).sql,
                                   b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.archived=0  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                       as *const u8 as *const libc::c_char);
            current_block = 3437258052017859086;
        } else {
            query = dc_strdup(query__);
            dc_trim(query);
            if *query.offset(0isize) as libc::c_int == 0i32 {
                success = 1i32;
                current_block = 15179736777190528364;
            } else {
                strLikeCmd = dc_mprintf(b"%%%s%%\x00" as *const u8 as *const libc::c_char, query);
                stmt =
                    dc_sqlite3_prepare((*(*chatlist).context).sql,
                                       b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.name LIKE ?  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_text(stmt, 1i32, strLikeCmd, -1i32, None);
                current_block = 3437258052017859086;
            }
        }
        match current_block {
            15179736777190528364 => {}
            _ => {
                while sqlite3_step(stmt) == 100i32 {
                    dc_array_add_id(
                        (*chatlist).chatNlastmsg_ids,
                        sqlite3_column_int(stmt, 0i32) as uint32_t,
                    );
                    dc_array_add_id(
                        (*chatlist).chatNlastmsg_ids,
                        sqlite3_column_int(stmt, 1i32) as uint32_t,
                    );
                }
                if 0 != add_archived_link_item && dc_get_archived_cnt((*chatlist).context) > 0i32 {
                    if dc_array_get_cnt((*chatlist).chatNlastmsg_ids) == 0i32 as libc::c_ulong
                        && 0 != listflags & 0x4i32
                    {
                        dc_array_add_id((*chatlist).chatNlastmsg_ids, 7i32 as uint32_t);
                        dc_array_add_id((*chatlist).chatNlastmsg_ids, 0i32 as uint32_t);
                    }
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, 6i32 as uint32_t);
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, 0i32 as uint32_t);
                }
                (*chatlist).cnt = dc_array_get_cnt((*chatlist).chatNlastmsg_ids)
                    .wrapping_div(2i32 as libc::c_ulong);
                success = 1i32
            }
        }
    }
    sqlite3_finalize(stmt);
    free(query as *mut libc::c_void);
    free(strLikeCmd as *mut libc::c_void);
    return success;
}
// Context functions to work with chatlist
#[no_mangle]
pub unsafe extern "C" fn dc_get_archived_cnt(mut context: *mut dc_context_t) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT COUNT(*) FROM chats WHERE blocked=0 AND archived=1;\x00" as *const u8
            as *const libc::c_char,
    );
    if sqlite3_step(stmt) == 100i32 {
        ret = sqlite3_column_int(stmt, 0i32)
    }
    sqlite3_finalize(stmt);
    return ret;
}
unsafe extern "C" fn get_last_deaddrop_fresh_msg(mut context: *mut dc_context_t) -> uint32_t {
    let mut ret: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    stmt =
        dc_sqlite3_prepare((*context).sql,
                           b"SELECT m.id  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  WHERE m.state=10   AND m.hidden=0    AND c.blocked=2 ORDER BY m.timestamp DESC, m.id DESC;\x00"
                               as *const u8 as *const libc::c_char);
    /* we have an index over the state-column, this should be sufficient as there are typically only few fresh messages */
    if !(sqlite3_step(stmt) != 100i32) {
        ret = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_cnt(mut chatlist: *const dc_chatlist_t) -> size_t {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return 0i32 as size_t;
    }
    return (*chatlist).cnt;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_chat_id(
    mut chatlist: *const dc_chatlist_t,
    mut index: size_t,
) -> uint32_t {
    if chatlist.is_null()
        || (*chatlist).magic != 0xc4a71157u32
        || (*chatlist).chatNlastmsg_ids.is_null()
        || index >= (*chatlist).cnt
    {
        return 0i32 as uint32_t;
    }
    return dc_array_get_id(
        (*chatlist).chatNlastmsg_ids,
        index.wrapping_mul(2i32 as libc::c_ulong),
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_msg_id(
    mut chatlist: *const dc_chatlist_t,
    mut index: size_t,
) -> uint32_t {
    if chatlist.is_null()
        || (*chatlist).magic != 0xc4a71157u32
        || (*chatlist).chatNlastmsg_ids.is_null()
        || index >= (*chatlist).cnt
    {
        return 0i32 as uint32_t;
    }
    return dc_array_get_id(
        (*chatlist).chatNlastmsg_ids,
        index
            .wrapping_mul(2i32 as libc::c_ulong)
            .wrapping_add(1i32 as libc::c_ulong),
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_summary(
    mut chatlist: *const dc_chatlist_t,
    mut index: size_t,
    mut chat: *mut dc_chat_t,
) -> *mut dc_lot_t {
    let mut current_block: u64;
    /* The summary is created by the chat, not by the last message.
    This is because we may want to display drafts here or stuff as
    "is typing".
    Also, sth. as "No messages" would not work if the summary comes from a
    message. */
    /* the function never returns NULL */
    let mut ret: *mut dc_lot_t = dc_lot_new();
    let mut lastmsg_id: uint32_t = 0i32 as uint32_t;
    let mut lastmsg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut lastcontact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut chat_to_delete: *mut dc_chat_t = 0 as *mut dc_chat_t;
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 || index >= (*chatlist).cnt {
        (*ret).text2 = dc_strdup(b"ErrBadChatlistIndex\x00" as *const u8 as *const libc::c_char)
    } else {
        lastmsg_id = dc_array_get_id(
            (*chatlist).chatNlastmsg_ids,
            index
                .wrapping_mul(2i32 as libc::c_ulong)
                .wrapping_add(1i32 as libc::c_ulong),
        );
        if chat.is_null() {
            chat = dc_chat_new((*chatlist).context);
            chat_to_delete = chat;
            if 0 == dc_chat_load_from_db(
                chat,
                dc_array_get_id(
                    (*chatlist).chatNlastmsg_ids,
                    index.wrapping_mul(2i32 as libc::c_ulong),
                ),
            ) {
                (*ret).text2 =
                    dc_strdup(b"ErrCannotReadChat\x00" as *const u8 as *const libc::c_char);
                current_block = 3777403817673069519;
            } else {
                current_block = 7651349459974463963;
            }
        } else {
            current_block = 7651349459974463963;
        }
        match current_block {
            3777403817673069519 => {}
            _ => {
                if 0 != lastmsg_id {
                    lastmsg = dc_msg_new_untyped((*chatlist).context);
                    dc_msg_load_from_db(lastmsg, (*chatlist).context, lastmsg_id);
                    if (*lastmsg).from_id != 1i32 as libc::c_uint
                        && ((*chat).type_0 == 120i32 || (*chat).type_0 == 130i32)
                    {
                        lastcontact = dc_contact_new((*chatlist).context);
                        dc_contact_load_from_db(
                            lastcontact,
                            (*(*chatlist).context).sql,
                            (*lastmsg).from_id,
                        );
                    }
                }
                if (*chat).id == 6i32 as libc::c_uint {
                    (*ret).text2 = dc_strdup(0 as *const libc::c_char)
                } else if lastmsg.is_null() || (*lastmsg).from_id == 0i32 as libc::c_uint {
                    (*ret).text2 = dc_stock_str((*chatlist).context, 1i32)
                } else {
                    dc_lot_fill(ret, lastmsg, chat, lastcontact, (*chatlist).context);
                }
            }
        }
    }
    dc_msg_unref(lastmsg);
    dc_contact_unref(lastcontact);
    dc_chat_unref(chat_to_delete);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_context(
    mut chatlist: *mut dc_chatlist_t,
) -> *mut dc_context_t {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return 0 as *mut dc_context_t;
    }
    return (*chatlist).context;
}

use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    #[no_mangle]
    fn usleep(_: useconds_t) -> libc::c_int;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn clock() -> clock_t;
    #[no_mangle]
    fn pthread_cond_destroy(_: *mut pthread_cond_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_cond_init(_: *mut pthread_cond_t, _: *const pthread_condattr_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_cond_signal(_: *mut pthread_cond_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_cond_wait(_: *mut pthread_cond_t, _: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_destroy(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_init(_: *mut pthread_mutex_t, _: *const pthread_mutexattr_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_lock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_unlock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_sqlite3_get_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: int32_t,
    ) -> int32_t;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_imap_interrupt_idle(_: *mut dc_imap_t);
    #[no_mangle]
    fn dc_imap_fetch(_: *mut dc_imap_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_set_watch_folder(_: *mut dc_imap_t, watch_folder: *const libc::c_char);
    #[no_mangle]
    fn dc_imap_disconnect(_: *mut dc_imap_t);
    #[no_mangle]
    fn dc_configure_folders(_: *mut dc_context_t, _: *mut dc_imap_t, flags: libc::c_int);
    #[no_mangle]
    fn dc_connect_to_configured_imap(_: *mut dc_context_t, _: *mut dc_imap_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_is_connected(_: *const dc_imap_t) -> libc::c_int;
    #[no_mangle]
    fn dc_imap_idle(_: *mut dc_imap_t);
}
pub type __uint32_t = libc::c_uint;
pub type __darwin_size_t = libc::c_ulong;
pub type __darwin_clock_t = libc::c_ulong;
pub type __darwin_ssize_t = libc::c_long;
pub type __darwin_time_t = libc::c_long;
pub type __darwin_useconds_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_cond_t {
    pub __sig: libc::c_long,
    pub __opaque: [libc::c_char; 40],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_condattr_t {
    pub __sig: libc::c_long,
    pub __opaque: [libc::c_char; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_mutex_t {
    pub __sig: libc::c_long,
    pub __opaque: [libc::c_char; 56],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_mutexattr_t {
    pub __sig: libc::c_long,
    pub __opaque: [libc::c_char; 8],
}
pub type __darwin_pthread_cond_t = _opaque_pthread_cond_t;
pub type __darwin_pthread_condattr_t = _opaque_pthread_condattr_t;
pub type __darwin_pthread_mutex_t = _opaque_pthread_mutex_t;
pub type __darwin_pthread_mutexattr_t = _opaque_pthread_mutexattr_t;
pub type size_t = __darwin_size_t;
pub type uint32_t = libc::c_uint;
pub type int32_t = libc::c_int;
pub type uintptr_t = libc::c_ulong;
pub type ssize_t = __darwin_ssize_t;
pub type useconds_t = __darwin_useconds_t;
pub type time_t = __darwin_time_t;
pub type uint8_t = libc::c_uchar;
pub type clock_t = __darwin_clock_t;
pub type pthread_cond_t = __darwin_pthread_cond_t;
pub type pthread_condattr_t = __darwin_pthread_condattr_t;
pub type pthread_mutex_t = __darwin_pthread_mutex_t;
pub type pthread_mutexattr_t = __darwin_pthread_mutexattr_t;
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
#[no_mangle]
pub unsafe extern "C" fn dc_jobthread_init(
    mut jobthread: *mut dc_jobthread_t,
    mut context: *mut dc_context_t,
    mut name: *const libc::c_char,
    mut folder_config_name: *const libc::c_char,
) {
    if jobthread.is_null() || context.is_null() || name.is_null() {
        return;
    }
    (*jobthread).context = context;
    (*jobthread).name = dc_strdup(name);
    (*jobthread).folder_config_name = dc_strdup(folder_config_name);
    (*jobthread).imap = 0 as *mut _dc_imap;
    pthread_mutex_init(&mut (*jobthread).mutex, 0 as *const pthread_mutexattr_t);
    pthread_cond_init(&mut (*jobthread).idle_cond, 0 as *const pthread_condattr_t);
    (*jobthread).idle_condflag = 0i32;
    (*jobthread).jobs_needed = 0i32;
    (*jobthread).suspended = 0i32;
    (*jobthread).using_handle = 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_jobthread_exit(mut jobthread: *mut dc_jobthread_t) {
    if jobthread.is_null() {
        return;
    }
    pthread_cond_destroy(&mut (*jobthread).idle_cond);
    pthread_mutex_destroy(&mut (*jobthread).mutex);
    free((*jobthread).name as *mut libc::c_void);
    (*jobthread).name = 0 as *mut libc::c_char;
    free((*jobthread).folder_config_name as *mut libc::c_void);
    (*jobthread).folder_config_name = 0 as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn dc_jobthread_suspend(
    mut jobthread: *mut dc_jobthread_t,
    mut suspend: libc::c_int,
) {
    if jobthread.is_null() {
        return;
    }
    if 0 != suspend {
        dc_log_info(
            (*jobthread).context,
            0i32,
            b"Suspending %s-thread.\x00" as *const u8 as *const libc::c_char,
            (*jobthread).name,
        );
        pthread_mutex_lock(&mut (*jobthread).mutex);
        (*jobthread).suspended = 1i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        dc_jobthread_interrupt_idle(jobthread);
        loop {
            pthread_mutex_lock(&mut (*jobthread).mutex);
            if (*jobthread).using_handle == 0i32 {
                pthread_mutex_unlock(&mut (*jobthread).mutex);
                return;
            }
            pthread_mutex_unlock(&mut (*jobthread).mutex);
            usleep((300i32 * 1000i32) as useconds_t);
        }
    } else {
        dc_log_info(
            (*jobthread).context,
            0i32,
            b"Unsuspending %s-thread.\x00" as *const u8 as *const libc::c_char,
            (*jobthread).name,
        );
        pthread_mutex_lock(&mut (*jobthread).mutex);
        (*jobthread).suspended = 0i32;
        (*jobthread).idle_condflag = 1i32;
        pthread_cond_signal(&mut (*jobthread).idle_cond);
        pthread_mutex_unlock(&mut (*jobthread).mutex);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_jobthread_interrupt_idle(mut jobthread: *mut dc_jobthread_t) {
    if jobthread.is_null() {
        return;
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).jobs_needed = 1i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
    dc_log_info(
        (*jobthread).context,
        0i32,
        b"Interrupting %s-IDLE...\x00" as *const u8 as *const libc::c_char,
        (*jobthread).name,
    );
    if !(*jobthread).imap.is_null() {
        dc_imap_interrupt_idle((*jobthread).imap);
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).idle_condflag = 1i32;
    pthread_cond_signal(&mut (*jobthread).idle_cond);
    pthread_mutex_unlock(&mut (*jobthread).mutex);
}
#[no_mangle]
pub unsafe extern "C" fn dc_jobthread_fetch(
    mut jobthread: *mut dc_jobthread_t,
    mut use_network: libc::c_int,
) {
    let mut start: clock_t = 0;
    if jobthread.is_null() {
        return;
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    if 0 != (*jobthread).suspended {
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    (*jobthread).using_handle = 1i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
    if !(0 == use_network || (*jobthread).imap.is_null()) {
        start = clock();
        if !(0 == connect_to_imap(jobthread)) {
            dc_log_info(
                (*jobthread).context,
                0i32,
                b"%s-fetch started...\x00" as *const u8 as *const libc::c_char,
                (*jobthread).name,
            );
            dc_imap_fetch((*jobthread).imap);
            if 0 != (*(*jobthread).imap).should_reconnect {
                dc_log_info(
                    (*jobthread).context,
                    0i32,
                    b"%s-fetch aborted, starting over...\x00" as *const u8 as *const libc::c_char,
                    (*jobthread).name,
                );
                dc_imap_fetch((*jobthread).imap);
            }
            dc_log_info(
                (*jobthread).context,
                0i32,
                b"%s-fetch done in %.0f ms.\x00" as *const u8 as *const libc::c_char,
                (*jobthread).name,
                clock().wrapping_sub(start) as libc::c_double * 1000.0f64
                    / 1000000i32 as libc::c_double,
            );
        }
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).using_handle = 0i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
}
/* ******************************************************************************
 * the typical fetch, idle, interrupt-idle
 ******************************************************************************/
unsafe extern "C" fn connect_to_imap(mut jobthread: *mut dc_jobthread_t) -> libc::c_int {
    let mut ret_connected: libc::c_int = 0i32;
    let mut mvbox_name: *mut libc::c_char = 0 as *mut libc::c_char;
    if 0 != dc_imap_is_connected((*jobthread).imap) {
        ret_connected = 1i32
    } else {
        ret_connected = dc_connect_to_configured_imap((*jobthread).context, (*jobthread).imap);
        if !(0 == ret_connected) {
            if dc_sqlite3_get_config_int(
                (*(*jobthread).context).sql,
                b"folders_configured\x00" as *const u8 as *const libc::c_char,
                0i32,
            ) < 3i32
            {
                dc_configure_folders((*jobthread).context, (*jobthread).imap, 0x1i32);
            }
            mvbox_name = dc_sqlite3_get_config(
                (*(*jobthread).context).sql,
                (*jobthread).folder_config_name,
                0 as *const libc::c_char,
            );
            if mvbox_name.is_null() {
                dc_imap_disconnect((*jobthread).imap);
                ret_connected = 0i32
            } else {
                dc_imap_set_watch_folder((*jobthread).imap, mvbox_name);
            }
        }
    }
    free(mvbox_name as *mut libc::c_void);
    return ret_connected;
}
#[no_mangle]
pub unsafe extern "C" fn dc_jobthread_idle(
    mut jobthread: *mut dc_jobthread_t,
    mut use_network: libc::c_int,
) {
    if jobthread.is_null() {
        return;
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    if 0 != (*jobthread).jobs_needed {
        dc_log_info(
            (*jobthread).context,
            0i32,
            b"%s-IDLE will not be started as it was interrupted while not ideling.\x00" as *const u8
                as *const libc::c_char,
            (*jobthread).name,
        );
        (*jobthread).jobs_needed = 0i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    if 0 != (*jobthread).suspended {
        while (*jobthread).idle_condflag == 0i32 {
            pthread_cond_wait(&mut (*jobthread).idle_cond, &mut (*jobthread).mutex);
        }
        (*jobthread).idle_condflag = 0i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    (*jobthread).using_handle = 1i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
    if 0 == use_network || (*jobthread).imap.is_null() {
        pthread_mutex_lock(&mut (*jobthread).mutex);
        (*jobthread).using_handle = 0i32;
        while (*jobthread).idle_condflag == 0i32 {
            pthread_cond_wait(&mut (*jobthread).idle_cond, &mut (*jobthread).mutex);
        }
        (*jobthread).idle_condflag = 0i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    connect_to_imap(jobthread);
    dc_log_info(
        (*jobthread).context,
        0i32,
        b"%s-IDLE started...\x00" as *const u8 as *const libc::c_char,
        (*jobthread).name,
    );
    dc_imap_idle((*jobthread).imap);
    dc_log_info(
        (*jobthread).context,
        0i32,
        b"%s-IDLE ended.\x00" as *const u8 as *const libc::c_char,
        (*jobthread).name,
    );
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).using_handle = 0i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
}

use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    #[no_mangle]
    fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn realloc(_: *mut libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn qsort(
        __base: *mut libc::c_void,
        __nel: size_t,
        __width: size_t,
        __compar: Option<
            unsafe extern "C" fn(_: *const libc::c_void, _: *const libc::c_void) -> libc::c_int,
        >,
    );
    #[no_mangle]
    fn memcpy(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strcat(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn sprintf(_: *mut libc::c_char, _: *const libc::c_char, _: ...) -> libc::c_int;
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
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
/* *
 * @class dc_array_t
 *
 * An object containing a simple array.
 * This object is used in several places where functions need to return an array.
 * The items of the array are typically IDs.
 * To free an array object, use dc_array_unref().
 */
#[no_mangle]
pub unsafe extern "C" fn dc_array_unref(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    if (*array).type_0 == 1i32 {
        dc_array_free_ptr(array);
    }
    free((*array).array as *mut libc::c_void);
    (*array).magic = 0i32 as uint32_t;
    free(array as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_free_ptr(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    let mut i: size_t = 0i32 as size_t;
    while i < (*array).count {
        if (*array).type_0 == 1i32 {
            free(
                (*(*(*array).array.offset(i as isize) as *mut _dc_location)).marker
                    as *mut libc::c_void,
            );
        }
        free(*(*array).array.offset(i as isize) as *mut libc::c_void);
        *(*array).array.offset(i as isize) = 0i32 as uintptr_t;
        i = i.wrapping_add(1)
    }
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_add_uint(mut array: *mut dc_array_t, mut item: uintptr_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    if (*array).count == (*array).allocated {
        let mut newsize: libc::c_int = (*array)
            .allocated
            .wrapping_mul(2i32 as libc::c_ulong)
            .wrapping_add(10i32 as libc::c_ulong)
            as libc::c_int;
        (*array).array = realloc(
            (*array).array as *mut libc::c_void,
            (newsize as libc::c_ulong)
                .wrapping_mul(::std::mem::size_of::<uintptr_t>() as libc::c_ulong),
        ) as *mut uintptr_t;
        if (*array).array.is_null() {
            exit(49i32);
        }
        (*array).allocated = newsize as size_t
    }
    *(*array).array.offset((*array).count as isize) = item;
    (*array).count = (*array).count.wrapping_add(1);
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_add_id(mut array: *mut dc_array_t, mut item: uint32_t) {
    dc_array_add_uint(array, item as uintptr_t);
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_add_ptr(mut array: *mut dc_array_t, mut item: *mut libc::c_void) {
    dc_array_add_uint(array, item as uintptr_t);
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_cnt(mut array: *const dc_array_t) -> size_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return 0i32 as size_t;
    }
    return (*array).count;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_uint(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> uintptr_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || index >= (*array).count {
        return 0i32 as uintptr_t;
    }
    return *(*array).array.offset(index as isize);
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_id(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> uint32_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || index >= (*array).count {
        return 0i32 as uint32_t;
    }
    if (*array).type_0 == 1i32 {
        return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).location_id;
    }
    return *(*array).array.offset(index as isize) as uint32_t;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_ptr(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> *mut libc::c_void {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || index >= (*array).count {
        return 0 as *mut libc::c_void;
    }
    return *(*array).array.offset(index as isize) as *mut libc::c_void;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_latitude(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> libc::c_double {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0i32 as libc::c_double;
    }
    return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).latitude;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_longitude(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> libc::c_double {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0i32 as libc::c_double;
    }
    return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).longitude;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_accuracy(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> libc::c_double {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0i32 as libc::c_double;
    }
    return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).accuracy;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_timestamp(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> time_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0i32 as time_t;
    }
    return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).timestamp;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_chat_id(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> uint32_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0i32 as uint32_t;
    }
    return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).chat_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_contact_id(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> uint32_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0i32 as uint32_t;
    }
    return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).contact_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_msg_id(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> uint32_t {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0i32 as uint32_t;
    }
    return (*(*(*array).array.offset(index as isize) as *mut _dc_location)).msg_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_marker(
    mut array: *const dc_array_t,
    mut index: size_t,
) -> *mut libc::c_char {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || index >= (*array).count
        || (*array).type_0 != 1i32
        || *(*array).array.offset(index as isize) == 0i32 as libc::c_ulong
    {
        return 0 as *mut libc::c_char;
    }
    return dc_strdup_keep_null(
        (*(*(*array).array.offset(index as isize) as *mut _dc_location)).marker,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_search_id(
    mut array: *const dc_array_t,
    mut needle: uint32_t,
    mut ret_index: *mut size_t,
) -> libc::c_int {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return 0i32;
    }
    let mut data: *mut uintptr_t = (*array).array;
    let mut i: size_t = 0;
    let mut cnt: size_t = (*array).count;
    i = 0i32 as size_t;
    while i < cnt {
        if *data.offset(i as isize) == needle as libc::c_ulong {
            if !ret_index.is_null() {
                *ret_index = i
            }
            return 1i32;
        }
        i = i.wrapping_add(1)
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_raw(mut array: *const dc_array_t) -> *const uintptr_t {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return 0 as *const uintptr_t;
    }
    return (*array).array;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_new(
    mut context: *mut dc_context_t,
    mut initsize: size_t,
) -> *mut dc_array_t {
    return dc_array_new_typed(context, 0i32, initsize);
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_new_typed(
    mut context: *mut dc_context_t,
    mut type_0: libc::c_int,
    mut initsize: size_t,
) -> *mut dc_array_t {
    let mut array: *mut dc_array_t = 0 as *mut dc_array_t;
    array = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_array_t>() as libc::c_ulong,
    ) as *mut dc_array_t;
    if array.is_null() {
        exit(47i32);
    }
    (*array).magic = 0xa11aai32 as uint32_t;
    (*array).context = context;
    (*array).count = 0i32 as size_t;
    (*array).allocated = if initsize < 1i32 as libc::c_ulong {
        1i32 as libc::c_ulong
    } else {
        initsize
    };
    (*array).type_0 = type_0;
    (*array).array = malloc(
        (*array)
            .allocated
            .wrapping_mul(::std::mem::size_of::<uintptr_t>() as libc::c_ulong),
    ) as *mut uintptr_t;
    if (*array).array.is_null() {
        exit(48i32);
    }
    return array;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_empty(mut array: *mut dc_array_t) {
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return;
    }
    (*array).count = 0i32 as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_duplicate(mut array: *const dc_array_t) -> *mut dc_array_t {
    let mut ret: *mut dc_array_t = 0 as *mut dc_array_t;
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint {
        return 0 as *mut dc_array_t;
    }
    ret = dc_array_new((*array).context, (*array).allocated);
    (*ret).count = (*array).count;
    memcpy(
        (*ret).array as *mut libc::c_void,
        (*array).array as *const libc::c_void,
        (*array)
            .count
            .wrapping_mul(::std::mem::size_of::<uintptr_t>() as libc::c_ulong),
    );
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_sort_ids(mut array: *mut dc_array_t) {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || (*array).count <= 1i32 as libc::c_ulong
    {
        return;
    }
    qsort(
        (*array).array as *mut libc::c_void,
        (*array).count,
        ::std::mem::size_of::<uintptr_t>() as libc::c_ulong,
        Some(cmp_intptr_t),
    );
}
unsafe extern "C" fn cmp_intptr_t(
    mut p1: *const libc::c_void,
    mut p2: *const libc::c_void,
) -> libc::c_int {
    let mut v1: uintptr_t = *(p1 as *mut uintptr_t);
    let mut v2: uintptr_t = *(p2 as *mut uintptr_t);
    return if v1 < v2 {
        -1i32
    } else if v1 > v2 {
        1i32
    } else {
        0i32
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_sort_strings(mut array: *mut dc_array_t) {
    if array.is_null()
        || (*array).magic != 0xa11aai32 as libc::c_uint
        || (*array).count <= 1i32 as libc::c_ulong
    {
        return;
    }
    qsort(
        (*array).array as *mut libc::c_void,
        (*array).count,
        ::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong,
        Some(cmp_strings_t),
    );
}
unsafe extern "C" fn cmp_strings_t(
    mut p1: *const libc::c_void,
    mut p2: *const libc::c_void,
) -> libc::c_int {
    let mut v1: *const libc::c_char = *(p1 as *mut *const libc::c_char);
    let mut v2: *const libc::c_char = *(p2 as *mut *const libc::c_char);
    return strcmp(v1, v2);
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_string(
    mut array: *const dc_array_t,
    mut sep: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if array.is_null() || (*array).magic != 0xa11aai32 as libc::c_uint || sep.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let mut i: libc::c_int = 0;
    ret = malloc(
        (*array)
            .count
            .wrapping_mul((11i32 as libc::c_ulong).wrapping_add(strlen(sep)))
            .wrapping_add(1i32 as libc::c_ulong),
    ) as *mut libc::c_char;
    if ret.is_null() {
        exit(35i32);
    }
    *ret.offset(0isize) = 0i32 as libc::c_char;
    i = 0i32;
    while (i as libc::c_ulong) < (*array).count {
        if 0 != i {
            strcat(ret, sep);
        }
        sprintf(
            &mut *ret.offset(strlen(ret) as isize) as *mut libc::c_char,
            b"%lu\x00" as *const u8 as *const libc::c_char,
            *(*array).array.offset(i as isize) as libc::c_ulong,
        );
        i += 1
    }
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_arr_to_string(
    mut arr: *const uint32_t,
    mut cnt: libc::c_int,
) -> *mut libc::c_char {
    /* return comma-separated value-string from integer array */
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut sep: *const libc::c_char = b",\x00" as *const u8 as *const libc::c_char;
    if arr.is_null() || cnt <= 0i32 {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let mut i: libc::c_int = 0;
    ret = malloc(
        (cnt as libc::c_ulong)
            .wrapping_mul((11i32 as libc::c_ulong).wrapping_add(strlen(sep)))
            .wrapping_add(1i32 as libc::c_ulong),
    ) as *mut libc::c_char;
    if ret.is_null() {
        exit(35i32);
    }
    *ret.offset(0isize) = 0i32 as libc::c_char;
    i = 0i32;
    while i < cnt {
        if 0 != i {
            strcat(ret, sep);
        }
        sprintf(
            &mut *ret.offset(strlen(ret) as isize) as *mut libc::c_char,
            b"%lu\x00" as *const u8 as *const libc::c_char,
            *arr.offset(i as isize) as libc::c_ulong,
        );
        i += 1
    }
    return ret;
}

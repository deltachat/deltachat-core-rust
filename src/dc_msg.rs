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
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_get_chat(_: *mut dc_context_t, chat_id: uint32_t) -> *mut dc_chat_t;
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
    fn dc_contact_unref(_: *mut dc_contact_t);
    /* library-private */
    #[no_mangle]
    fn dc_param_new() -> *mut dc_param_t;
    #[no_mangle]
    fn dc_param_unref(_: *mut dc_param_t);
    #[no_mangle]
    fn dc_param_set_packed(_: *mut dc_param_t, _: *const libc::c_char);
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_strbuilder_catf(_: *mut dc_strbuilder_t, format: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_get_int(_: *const dc_param_t, key: libc::c_int, def: int32_t) -> int32_t;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_filesuffix_lc(pathNfilename: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_get(
        _: *const dc_param_t,
        key: libc::c_int,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_filebytes(_: *mut dc_context_t, pathNfilename: *const libc::c_char) -> uint64_t;
    #[no_mangle]
    fn dc_get_abs_path(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_contact_get_name_n_addr(_: *const dc_contact_t) -> *mut libc::c_char;
    #[no_mangle]
    fn sqlite3_column_int64(_: *mut sqlite3_stmt, iCol: libc::c_int) -> sqlite3_int64;
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
    #[no_mangle]
    fn dc_timestamp_to_str(_: time_t) -> *mut libc::c_char;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    #[no_mangle]
    fn dc_truncate_str(_: *mut libc::c_char, approx_characters: libc::c_int);
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    #[no_mangle]
    fn dc_truncate_n_unwrap_str(
        _: *mut libc::c_char,
        approx_characters: libc::c_int,
        do_unwrap: libc::c_int,
    );
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
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
    fn dc_sqlite3_commit(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_begin_transaction(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_rollback(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn sqlite3_reset(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_get_contact(_: *mut dc_context_t, contact_id: uint32_t) -> *mut dc_contact_t;
    #[no_mangle]
    fn dc_chat_unref(_: *mut dc_chat_t);
    #[no_mangle]
    fn dc_get_filename(pathNfilename: *const libc::c_char) -> *mut libc::c_char;
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
    #[no_mangle]
    fn dc_gm2local_offset() -> libc::c_long;
    #[no_mangle]
    fn dc_split_armored_data(
        buf: *mut libc::c_char,
        ret_headerline: *mut *const libc::c_char,
        ret_setupcodebegin: *mut *const libc::c_char,
        ret_preferencrypt: *mut *const libc::c_char,
        ret_base64: *mut *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_read_file(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
        buf: *mut *mut libc::c_void,
        buf_bytes: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
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
    fn dc_sqlite3_log_error(_: *mut dc_sqlite3_t, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_get_chat_contact_cnt(_: *mut dc_context_t, chat_id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
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
pub type dc_strbuilder_t = _dc_strbuilder;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type sqlite3_int64 = sqlite_int64;
pub type sqlite_int64 = libc::c_longlong;
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
// handle messages
#[no_mangle]
pub unsafe extern "C" fn dc_get_msg_info(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) -> *mut libc::c_char {
    let mut e2ee_errors: libc::c_int = 0;
    let mut w: libc::c_int = 0;
    let mut h: libc::c_int = 0;
    let mut duration: libc::c_int = 0;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut contact_from: *mut dc_contact_t = dc_contact_new(context);
    let mut rawtxt: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        dc_msg_load_from_db(msg, context, msg_id);
        dc_contact_load_from_db(contact_from, (*context).sql, (*msg).from_id);
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT txt_raw FROM msgs WHERE id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, msg_id as libc::c_int);
        if sqlite3_step(stmt) != 100i32 {
            p = dc_mprintf(
                b"Cannot load message #%i.\x00" as *const u8 as *const libc::c_char,
                msg_id as libc::c_int,
            );
            dc_strbuilder_cat(&mut ret, p);
            free(p as *mut libc::c_void);
        } else {
            rawtxt = dc_strdup(sqlite3_column_text(stmt, 0i32) as *mut libc::c_char);
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            dc_trim(rawtxt);
            dc_truncate_str(rawtxt, 100000i32);
            dc_strbuilder_cat(&mut ret, b"Sent: \x00" as *const u8 as *const libc::c_char);
            p = dc_timestamp_to_str(dc_msg_get_timestamp(msg));
            dc_strbuilder_cat(&mut ret, p);
            free(p as *mut libc::c_void);
            p = dc_contact_get_name_n_addr(contact_from);
            dc_strbuilder_catf(
                &mut ret as *mut dc_strbuilder_t,
                b" by %s\x00" as *const u8 as *const libc::c_char,
                p,
            );
            free(p as *mut libc::c_void);
            dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
            if (*msg).from_id != 1i32 as libc::c_uint {
                dc_strbuilder_cat(
                    &mut ret,
                    b"Received: \x00" as *const u8 as *const libc::c_char,
                );
                p = dc_timestamp_to_str(if 0 != (*msg).timestamp_rcvd {
                    (*msg).timestamp_rcvd
                } else {
                    (*msg).timestamp_sort
                });
                dc_strbuilder_cat(&mut ret, p);
                free(p as *mut libc::c_void);
                dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
            }
            if !((*msg).from_id == 2i32 as libc::c_uint || (*msg).to_id == 2i32 as libc::c_uint) {
                // device-internal message, no further details needed
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"SELECT contact_id, timestamp_sent FROM msgs_mdns WHERE msg_id=?;\x00"
                        as *const u8 as *const libc::c_char,
                );
                sqlite3_bind_int(stmt, 1i32, msg_id as libc::c_int);
                while sqlite3_step(stmt) == 100i32 {
                    dc_strbuilder_cat(&mut ret, b"Read: \x00" as *const u8 as *const libc::c_char);
                    p = dc_timestamp_to_str(sqlite3_column_int64(stmt, 1i32) as time_t);
                    dc_strbuilder_cat(&mut ret, p);
                    free(p as *mut libc::c_void);
                    dc_strbuilder_cat(&mut ret, b" by \x00" as *const u8 as *const libc::c_char);
                    let mut contact: *mut dc_contact_t = dc_contact_new(context);
                    dc_contact_load_from_db(
                        contact,
                        (*context).sql,
                        sqlite3_column_int64(stmt, 0i32) as uint32_t,
                    );
                    p = dc_contact_get_name_n_addr(contact);
                    dc_strbuilder_cat(&mut ret, p);
                    free(p as *mut libc::c_void);
                    dc_contact_unref(contact);
                    dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
                }
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                p = 0 as *mut libc::c_char;
                match (*msg).state {
                    10 => p = dc_strdup(b"Fresh\x00" as *const u8 as *const libc::c_char),
                    13 => p = dc_strdup(b"Noticed\x00" as *const u8 as *const libc::c_char),
                    16 => p = dc_strdup(b"Seen\x00" as *const u8 as *const libc::c_char),
                    26 => p = dc_strdup(b"Delivered\x00" as *const u8 as *const libc::c_char),
                    24 => p = dc_strdup(b"Failed\x00" as *const u8 as *const libc::c_char),
                    28 => p = dc_strdup(b"Read\x00" as *const u8 as *const libc::c_char),
                    20 => p = dc_strdup(b"Pending\x00" as *const u8 as *const libc::c_char),
                    18 => p = dc_strdup(b"Preparing\x00" as *const u8 as *const libc::c_char),
                    _ => {
                        p = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, (*msg).state)
                    }
                }
                dc_strbuilder_catf(
                    &mut ret as *mut dc_strbuilder_t,
                    b"State: %s\x00" as *const u8 as *const libc::c_char,
                    p,
                );
                free(p as *mut libc::c_void);
                if 0 != dc_msg_has_location(msg) {
                    dc_strbuilder_cat(
                        &mut ret,
                        b", Location sent\x00" as *const u8 as *const libc::c_char,
                    );
                }
                p = 0 as *mut libc::c_char;
                e2ee_errors = 0;
                e2ee_errors = dc_param_get_int((*msg).param, 'e' as i32, 0i32);
                if 0 != e2ee_errors {
                    if 0 != e2ee_errors & 0x2i32 {
                        p = dc_strdup(
                            b"Encrypted, no valid signature\x00" as *const u8
                                as *const libc::c_char,
                        )
                    }
                } else if 0 != dc_param_get_int((*msg).param, 'c' as i32, 0i32) {
                    p = dc_strdup(b"Encrypted\x00" as *const u8 as *const libc::c_char)
                }
                if !p.is_null() {
                    dc_strbuilder_catf(
                        &mut ret as *mut dc_strbuilder_t,
                        b", %s\x00" as *const u8 as *const libc::c_char,
                        p,
                    );
                    free(p as *mut libc::c_void);
                }
                dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
                p = dc_param_get((*msg).param, 'L' as i32, 0 as *const libc::c_char);
                if !p.is_null() {
                    dc_strbuilder_catf(
                        &mut ret as *mut dc_strbuilder_t,
                        b"Error: %s\n\x00" as *const u8 as *const libc::c_char,
                        p,
                    );
                    free(p as *mut libc::c_void);
                }
                p = dc_msg_get_file(msg);
                if !p.is_null() && 0 != *p.offset(0isize) as libc::c_int {
                    dc_strbuilder_catf(
                        &mut ret as *mut dc_strbuilder_t,
                        b"\nFile: %s, %i bytes\n\x00" as *const u8 as *const libc::c_char,
                        p,
                        dc_get_filebytes(context, p) as libc::c_int,
                    );
                }
                free(p as *mut libc::c_void);
                if (*msg).type_0 != 10i32 {
                    p = 0 as *mut libc::c_char;
                    match (*msg).type_0 {
                        40 => p = dc_strdup(b"Audio\x00" as *const u8 as *const libc::c_char),
                        60 => p = dc_strdup(b"File\x00" as *const u8 as *const libc::c_char),
                        21 => p = dc_strdup(b"GIF\x00" as *const u8 as *const libc::c_char),
                        20 => p = dc_strdup(b"Image\x00" as *const u8 as *const libc::c_char),
                        50 => p = dc_strdup(b"Video\x00" as *const u8 as *const libc::c_char),
                        41 => p = dc_strdup(b"Voice\x00" as *const u8 as *const libc::c_char),
                        _ => {
                            p = dc_mprintf(
                                b"%i\x00" as *const u8 as *const libc::c_char,
                                (*msg).type_0,
                            )
                        }
                    }
                    dc_strbuilder_catf(
                        &mut ret as *mut dc_strbuilder_t,
                        b"Type: %s\n\x00" as *const u8 as *const libc::c_char,
                        p,
                    );
                    free(p as *mut libc::c_void);
                    p = dc_msg_get_filemime(msg);
                    dc_strbuilder_catf(
                        &mut ret as *mut dc_strbuilder_t,
                        b"Mimetype: %s\n\x00" as *const u8 as *const libc::c_char,
                        p,
                    );
                    free(p as *mut libc::c_void);
                }
                w = dc_param_get_int((*msg).param, 'w' as i32, 0i32);
                h = dc_param_get_int((*msg).param, 'h' as i32, 0i32);
                if w != 0i32 || h != 0i32 {
                    p = dc_mprintf(
                        b"Dimension: %i x %i\n\x00" as *const u8 as *const libc::c_char,
                        w,
                        h,
                    );
                    dc_strbuilder_cat(&mut ret, p);
                    free(p as *mut libc::c_void);
                }
                duration = dc_param_get_int((*msg).param, 'd' as i32, 0i32);
                if duration != 0i32 {
                    p = dc_mprintf(
                        b"Duration: %i ms\n\x00" as *const u8 as *const libc::c_char,
                        duration,
                    );
                    dc_strbuilder_cat(&mut ret, p);
                    free(p as *mut libc::c_void);
                }
                if !rawtxt.is_null() && 0 != *rawtxt.offset(0isize) as libc::c_int {
                    dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
                    dc_strbuilder_cat(&mut ret, rawtxt);
                    dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
                }
                if !(*msg).rfc724_mid.is_null()
                    && 0 != *(*msg).rfc724_mid.offset(0isize) as libc::c_int
                {
                    dc_strbuilder_catf(
                        &mut ret as *mut dc_strbuilder_t,
                        b"\nMessage-ID: %s\x00" as *const u8 as *const libc::c_char,
                        (*msg).rfc724_mid,
                    );
                }
                if !(*msg).server_folder.is_null()
                    && 0 != *(*msg).server_folder.offset(0isize) as libc::c_int
                {
                    dc_strbuilder_catf(
                        &mut ret as *mut dc_strbuilder_t,
                        b"\nLast seen as: %s/%i\x00" as *const u8 as *const libc::c_char,
                        (*msg).server_folder,
                        (*msg).server_uid as libc::c_int,
                    );
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    dc_msg_unref(msg);
    dc_contact_unref(contact_from);
    free(rawtxt as *mut libc::c_void);
    return ret.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_new_untyped(mut context: *mut dc_context_t) -> *mut dc_msg_t {
    return dc_msg_new(context, 0i32);
}
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
pub unsafe extern "C" fn dc_msg_new(
    mut context: *mut dc_context_t,
    mut viewtype: libc::c_int,
) -> *mut dc_msg_t {
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    msg = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_msg_t>() as libc::c_ulong,
    ) as *mut dc_msg_t;
    if msg.is_null() {
        exit(15i32);
    }
    (*msg).context = context;
    (*msg).magic = 0x11561156i32 as uint32_t;
    (*msg).type_0 = viewtype;
    (*msg).state = 0i32;
    (*msg).param = dc_param_new();
    return msg;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_unref(mut msg: *mut dc_msg_t) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_msg_empty(msg);
    dc_param_unref((*msg).param);
    (*msg).magic = 0i32 as uint32_t;
    free(msg as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_empty(mut msg: *mut dc_msg_t) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    free((*msg).text as *mut libc::c_void);
    (*msg).text = 0 as *mut libc::c_char;
    free((*msg).rfc724_mid as *mut libc::c_void);
    (*msg).rfc724_mid = 0 as *mut libc::c_char;
    free((*msg).in_reply_to as *mut libc::c_void);
    (*msg).in_reply_to = 0 as *mut libc::c_char;
    free((*msg).server_folder as *mut libc::c_void);
    (*msg).server_folder = 0 as *mut libc::c_char;
    dc_param_set_packed((*msg).param, 0 as *const libc::c_char);
    (*msg).hidden = 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filemime(mut msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut file: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        ret = dc_param_get((*msg).param, 'm' as i32, 0 as *const libc::c_char);
        if ret.is_null() {
            file = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
            if !file.is_null() {
                dc_msg_guess_msgtype_from_suffix(file, 0 as *mut libc::c_int, &mut ret);
                if ret.is_null() {
                    ret = dc_strdup(
                        b"application/octet-stream\x00" as *const u8 as *const libc::c_char,
                    )
                }
            }
        }
    }
    free(file as *mut libc::c_void);
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_guess_msgtype_from_suffix(
    mut pathNfilename: *const libc::c_char,
    mut ret_msgtype: *mut libc::c_int,
    mut ret_mime: *mut *mut libc::c_char,
) {
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dummy_msgtype: libc::c_int = 0i32;
    let mut dummy_buf: *mut libc::c_char = 0 as *mut libc::c_char;
    if !pathNfilename.is_null() {
        if ret_msgtype.is_null() {
            ret_msgtype = &mut dummy_msgtype
        }
        if ret_mime.is_null() {
            ret_mime = &mut dummy_buf
        }
        *ret_msgtype = 0i32;
        *ret_mime = 0 as *mut libc::c_char;
        suffix = dc_get_filesuffix_lc(pathNfilename);
        if !suffix.is_null() {
            if strcmp(suffix, b"mp3\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = 40i32;
                *ret_mime = dc_strdup(b"audio/mpeg\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"aac\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = 40i32;
                *ret_mime = dc_strdup(b"audio/aac\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"mp4\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = 50i32;
                *ret_mime = dc_strdup(b"video/mp4\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"jpg\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"jpeg\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                *ret_msgtype = 20i32;
                *ret_mime = dc_strdup(b"image/jpeg\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"png\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = 20i32;
                *ret_mime = dc_strdup(b"image/png\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"webp\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = 20i32;
                *ret_mime = dc_strdup(b"image/webp\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"gif\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = 21i32;
                *ret_mime = dc_strdup(b"image/gif\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"vcf\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"vcard\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                *ret_msgtype = 60i32;
                *ret_mime = dc_strdup(b"text/vcard\x00" as *const u8 as *const libc::c_char)
            }
        }
    }
    free(suffix as *mut libc::c_void);
    free(dummy_buf as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_file(mut msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut file_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut file_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        file_rel = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
        if !file_rel.is_null() {
            file_abs = dc_get_abs_path((*msg).context, file_rel)
        }
    }
    free(file_rel as *mut libc::c_void);
    return if !file_abs.is_null() {
        file_abs
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_location(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return ((*msg).location_id != 0i32 as libc::c_uint) as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_timestamp(mut msg: *const dc_msg_t) -> time_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as time_t;
    }
    return if 0 != (*msg).timestamp_sent {
        (*msg).timestamp_sent
    } else {
        (*msg).timestamp_sort
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_load_from_db(
    mut msg: *mut dc_msg_t,
    mut context: *mut dc_context_t,
    mut id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(msg.is_null()
        || (*msg).magic != 0x11561156i32 as libc::c_uint
        || context.is_null()
        || (*context).sql.is_null())
    {
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT  m.id,rfc724_mid,m.mime_in_reply_to,m.server_folder,m.server_uid,m.move_state,m.chat_id,  m.from_id,m.to_id,m.timestamp,m.timestamp_sent,m.timestamp_rcvd, m.type,m.state,m.msgrmsg,m.txt,  m.param,m.starred,m.hidden,m.location_id, c.blocked  FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id WHERE m.id=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, id as libc::c_int);
        if !(sqlite3_step(stmt) != 100i32) {
            if !(0 == dc_msg_set_from_stmt(msg, stmt, 0i32)) {
                /* also calls dc_msg_empty() */
                (*msg).context = context;
                success = 1i32
            }
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
unsafe extern "C" fn dc_msg_set_from_stmt(
    mut msg: *mut dc_msg_t,
    mut row: *mut sqlite3_stmt,
    mut row_offset: libc::c_int,
) -> libc::c_int {
    dc_msg_empty(msg);
    let fresh0 = row_offset;
    row_offset = row_offset + 1;
    (*msg).id = sqlite3_column_int(row, fresh0) as uint32_t;
    let fresh1 = row_offset;
    row_offset = row_offset + 1;
    (*msg).rfc724_mid = dc_strdup(sqlite3_column_text(row, fresh1) as *mut libc::c_char);
    let fresh2 = row_offset;
    row_offset = row_offset + 1;
    (*msg).in_reply_to = dc_strdup(sqlite3_column_text(row, fresh2) as *mut libc::c_char);
    let fresh3 = row_offset;
    row_offset = row_offset + 1;
    (*msg).server_folder = dc_strdup(sqlite3_column_text(row, fresh3) as *mut libc::c_char);
    let fresh4 = row_offset;
    row_offset = row_offset + 1;
    (*msg).server_uid = sqlite3_column_int(row, fresh4) as uint32_t;
    let fresh5 = row_offset;
    row_offset = row_offset + 1;
    (*msg).move_state = sqlite3_column_int(row, fresh5) as dc_move_state_t;
    let fresh6 = row_offset;
    row_offset = row_offset + 1;
    (*msg).chat_id = sqlite3_column_int(row, fresh6) as uint32_t;
    let fresh7 = row_offset;
    row_offset = row_offset + 1;
    (*msg).from_id = sqlite3_column_int(row, fresh7) as uint32_t;
    let fresh8 = row_offset;
    row_offset = row_offset + 1;
    (*msg).to_id = sqlite3_column_int(row, fresh8) as uint32_t;
    let fresh9 = row_offset;
    row_offset = row_offset + 1;
    (*msg).timestamp_sort = sqlite3_column_int64(row, fresh9) as time_t;
    let fresh10 = row_offset;
    row_offset = row_offset + 1;
    (*msg).timestamp_sent = sqlite3_column_int64(row, fresh10) as time_t;
    let fresh11 = row_offset;
    row_offset = row_offset + 1;
    (*msg).timestamp_rcvd = sqlite3_column_int64(row, fresh11) as time_t;
    let fresh12 = row_offset;
    row_offset = row_offset + 1;
    (*msg).type_0 = sqlite3_column_int(row, fresh12);
    let fresh13 = row_offset;
    row_offset = row_offset + 1;
    (*msg).state = sqlite3_column_int(row, fresh13);
    let fresh14 = row_offset;
    row_offset = row_offset + 1;
    (*msg).is_dc_message = sqlite3_column_int(row, fresh14);
    let fresh15 = row_offset;
    row_offset = row_offset + 1;
    (*msg).text = dc_strdup(sqlite3_column_text(row, fresh15) as *mut libc::c_char);
    let fresh16 = row_offset;
    row_offset = row_offset + 1;
    dc_param_set_packed(
        (*msg).param,
        sqlite3_column_text(row, fresh16) as *mut libc::c_char,
    );
    let fresh17 = row_offset;
    row_offset = row_offset + 1;
    (*msg).starred = sqlite3_column_int(row, fresh17);
    let fresh18 = row_offset;
    row_offset = row_offset + 1;
    (*msg).hidden = sqlite3_column_int(row, fresh18);
    let fresh19 = row_offset;
    row_offset = row_offset + 1;
    (*msg).location_id = sqlite3_column_int(row, fresh19) as uint32_t;
    let fresh20 = row_offset;
    row_offset = row_offset + 1;
    (*msg).chat_blocked = sqlite3_column_int(row, fresh20);
    if (*msg).chat_blocked == 2i32 {
        dc_truncate_n_unwrap_str((*msg).text, 256i32, 0i32);
    }
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_mime_headers(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) -> *mut libc::c_char {
    let mut eml: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT mime_headers FROM msgs WHERE id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, msg_id as libc::c_int);
        if sqlite3_step(stmt) == 100i32 {
            eml = dc_strdup_keep_null(sqlite3_column_text(stmt, 0i32) as *const libc::c_char)
        }
    }
    sqlite3_finalize(stmt);
    return eml;
}
#[no_mangle]
pub unsafe extern "C" fn dc_delete_msgs(
    mut context: *mut dc_context_t,
    mut msg_ids: *const uint32_t,
    mut msg_cnt: libc::c_int,
) {
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg_ids.is_null()
        || msg_cnt <= 0i32
    {
        return;
    }
    dc_sqlite3_begin_transaction((*context).sql);
    let mut i: libc::c_int = 0i32;
    while i < msg_cnt {
        dc_update_msg_chat_id(context, *msg_ids.offset(i as isize), 3i32 as uint32_t);
        dc_job_add(
            context,
            110i32,
            *msg_ids.offset(i as isize) as libc::c_int,
            0 as *const libc::c_char,
            0i32,
        );
        i += 1
    }
    dc_sqlite3_commit((*context).sql);
    if 0 != msg_cnt {
        (*context).cb.expect("non-null function pointer")(
            context,
            2000i32,
            0i32 as uintptr_t,
            0i32 as uintptr_t,
        );
        dc_job_kill_action(context, 105i32);
        dc_job_add(context, 105i32, 0i32, 0 as *const libc::c_char, 10i32);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_update_msg_chat_id(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
    mut chat_id: uint32_t,
) {
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE msgs SET chat_id=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, msg_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_markseen_msgs(
    mut context: *mut dc_context_t,
    mut msg_ids: *const uint32_t,
    mut msg_cnt: libc::c_int,
) {
    let mut transaction_pending: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    let mut send_event: libc::c_int = 0i32;
    let mut curr_state: libc::c_int = 0i32;
    let mut curr_blocked: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg_ids.is_null()
        || msg_cnt <= 0i32)
    {
        dc_sqlite3_begin_transaction((*context).sql);
        transaction_pending = 1i32;
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT m.state, c.blocked  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  WHERE m.id=? AND m.chat_id>9\x00"
                                   as *const u8 as *const libc::c_char);
        i = 0i32;
        while i < msg_cnt {
            sqlite3_reset(stmt);
            sqlite3_bind_int(stmt, 1i32, *msg_ids.offset(i as isize) as libc::c_int);
            if !(sqlite3_step(stmt) != 100i32) {
                curr_state = sqlite3_column_int(stmt, 0i32);
                curr_blocked = sqlite3_column_int(stmt, 1i32);
                if curr_blocked == 0i32 {
                    if curr_state == 10i32 || curr_state == 13i32 {
                        dc_update_msg_state(context, *msg_ids.offset(i as isize), 16i32);
                        dc_log_info(
                            context,
                            0i32,
                            b"Seen message #%i.\x00" as *const u8 as *const libc::c_char,
                            *msg_ids.offset(i as isize),
                        );
                        dc_job_add(
                            context,
                            130i32,
                            *msg_ids.offset(i as isize) as libc::c_int,
                            0 as *const libc::c_char,
                            0i32,
                        );
                        send_event = 1i32
                    }
                } else if curr_state == 10i32 {
                    dc_update_msg_state(context, *msg_ids.offset(i as isize), 13i32);
                    send_event = 1i32
                }
            }
            i += 1
        }
        dc_sqlite3_commit((*context).sql);
        transaction_pending = 0i32;
        if 0 != send_event {
            (*context).cb.expect("non-null function pointer")(
                context,
                2000i32,
                0i32 as uintptr_t,
                0i32 as uintptr_t,
            );
        }
    }
    if 0 != transaction_pending {
        dc_sqlite3_rollback((*context).sql);
    }
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_update_msg_state(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
    mut state: libc::c_int,
) {
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE msgs SET state=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, state);
    sqlite3_bind_int(stmt, 2i32, msg_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_star_msgs(
    mut context: *mut dc_context_t,
    mut msg_ids: *const uint32_t,
    mut msg_cnt: libc::c_int,
    mut star: libc::c_int,
) {
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg_ids.is_null()
        || msg_cnt <= 0i32
        || star != 0i32 && star != 1i32
    {
        return;
    }
    dc_sqlite3_begin_transaction((*context).sql);
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE msgs SET starred=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    let mut i: libc::c_int = 0i32;
    while i < msg_cnt {
        sqlite3_reset(stmt);
        sqlite3_bind_int(stmt, 1i32, star);
        sqlite3_bind_int(stmt, 2i32, *msg_ids.offset(i as isize) as libc::c_int);
        sqlite3_step(stmt);
        i += 1
    }
    sqlite3_finalize(stmt);
    dc_sqlite3_commit((*context).sql);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_msg(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) -> *mut dc_msg_t {
    let mut success: libc::c_int = 0i32;
    let mut obj: *mut dc_msg_t = dc_msg_new_untyped(context);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(0 == dc_msg_load_from_db(obj, context, msg_id)) {
            success = 1i32
        }
    }
    if 0 != success {
        return obj;
    } else {
        dc_msg_unref(obj);
        return 0 as *mut dc_msg_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_id(mut msg: *const dc_msg_t) -> uint32_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    return (*msg).id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_from_id(mut msg: *const dc_msg_t) -> uint32_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    return (*msg).from_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_chat_id(mut msg: *const dc_msg_t) -> uint32_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    return if 0 != (*msg).chat_blocked {
        1i32 as libc::c_uint
    } else {
        (*msg).chat_id
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_viewtype(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return (*msg).type_0;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_state(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return (*msg).state;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_received_timestamp(mut msg: *const dc_msg_t) -> time_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as time_t;
    }
    return (*msg).timestamp_rcvd;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_sort_timestamp(mut msg: *const dc_msg_t) -> time_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as time_t;
    }
    return (*msg).timestamp_sort;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_text(mut msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    ret = dc_strdup((*msg).text);
    dc_truncate_str(ret, 30000i32);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filename(mut msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        pathNfilename = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
        if !pathNfilename.is_null() {
            ret = dc_get_filename(pathNfilename)
        }
    }
    free(pathNfilename as *mut libc::c_void);
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filebytes(mut msg: *const dc_msg_t) -> uint64_t {
    let mut ret: uint64_t = 0i32 as uint64_t;
    let mut file: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        file = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
        if !file.is_null() {
            ret = dc_get_filebytes((*msg).context, file)
        }
    }
    free(file as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_width(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return dc_param_get_int((*msg).param, 'w' as i32, 0i32);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_height(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return dc_param_get_int((*msg).param, 'h' as i32, 0i32);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_duration(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return dc_param_get_int((*msg).param, 'd' as i32, 0i32);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_showpadlock(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint || (*msg).context.is_null() {
        return 0i32;
    }
    if dc_param_get_int((*msg).param, 'c' as i32, 0i32) != 0i32 {
        return 1i32;
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summary(
    mut msg: *const dc_msg_t,
    mut chat: *const dc_chat_t,
) -> *mut dc_lot_t {
    let mut current_block: u64;
    let mut ret: *mut dc_lot_t = dc_lot_new();
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut chat_to_delete: *mut dc_chat_t = 0 as *mut dc_chat_t;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        if chat.is_null() {
            chat_to_delete = dc_get_chat((*msg).context, (*msg).chat_id);
            if chat_to_delete.is_null() {
                current_block = 15204159476013091401;
            } else {
                chat = chat_to_delete;
                current_block = 7815301370352969686;
            }
        } else {
            current_block = 7815301370352969686;
        }
        match current_block {
            15204159476013091401 => {}
            _ => {
                if (*msg).from_id != 1i32 as libc::c_uint
                    && ((*chat).type_0 == 120i32 || (*chat).type_0 == 130i32)
                {
                    contact = dc_get_contact((*chat).context, (*msg).from_id)
                }
                dc_lot_fill(ret, msg, chat, contact, (*msg).context);
            }
        }
    }
    dc_contact_unref(contact);
    dc_chat_unref(chat_to_delete);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summarytext(
    mut msg: *const dc_msg_t,
    mut approx_characters: libc::c_int,
) -> *mut libc::c_char {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    return dc_msg_get_summarytext_by_raw(
        (*msg).type_0,
        (*msg).text,
        (*msg).param,
        approx_characters,
        (*msg).context,
    );
}
/* the returned value must be free()'d */
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summarytext_by_raw(
    mut type_0: libc::c_int,
    mut text: *const libc::c_char,
    mut param: *mut dc_param_t,
    mut approx_characters: libc::c_int,
    mut context: *mut dc_context_t,
) -> *mut libc::c_char {
    /* get a summary text, result must be free()'d, never returns NULL. */
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut prefix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut label: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut append_text: libc::c_int = 1i32;
    match type_0 {
        20 => prefix = dc_stock_str(context, 9i32),
        21 => prefix = dc_stock_str(context, 23i32),
        50 => prefix = dc_stock_str(context, 10i32),
        41 => prefix = dc_stock_str(context, 7i32),
        40 | 60 => {
            if dc_param_get_int(param, 'S' as i32, 0i32) == 6i32 {
                prefix = dc_stock_str(context, 42i32);
                append_text = 0i32
            } else {
                pathNfilename = dc_param_get(
                    param,
                    'f' as i32,
                    b"ErrFilename\x00" as *const u8 as *const libc::c_char,
                );
                value = dc_get_filename(pathNfilename);
                label = dc_stock_str(context, if type_0 == 40i32 { 11i32 } else { 12i32 });
                prefix = dc_mprintf(
                    b"%s \xe2\x80\x93 %s\x00" as *const u8 as *const libc::c_char,
                    label,
                    value,
                )
            }
        }
        _ => {
            if dc_param_get_int(param, 'S' as i32, 0i32) == 9i32 {
                prefix = dc_stock_str(context, 66i32);
                append_text = 0i32
            }
        }
    }
    if 0 != append_text
        && !prefix.is_null()
        && !text.is_null()
        && 0 != *text.offset(0isize) as libc::c_int
    {
        ret = dc_mprintf(
            b"%s \xe2\x80\x93 %s\x00" as *const u8 as *const libc::c_char,
            prefix,
            text,
        );
        dc_truncate_n_unwrap_str(ret, approx_characters, 1i32);
    } else if 0 != append_text && !text.is_null() && 0 != *text.offset(0isize) as libc::c_int {
        ret = dc_strdup(text);
        dc_truncate_n_unwrap_str(ret, approx_characters, 1i32);
    } else {
        ret = prefix;
        prefix = 0 as *mut libc::c_char
    }
    free(prefix as *mut libc::c_void);
    free(pathNfilename as *mut libc::c_void);
    free(label as *mut libc::c_void);
    free(value as *mut libc::c_void);
    if ret.is_null() {
        ret = dc_strdup(0 as *const libc::c_char)
    }
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_deviating_timestamp(mut msg: *const dc_msg_t) -> libc::c_int {
    let mut cnv_to_local: libc::c_long = dc_gm2local_offset();
    let mut sort_timestamp: time_t = dc_msg_get_sort_timestamp(msg) + cnv_to_local;
    let mut send_timestamp: time_t = dc_msg_get_timestamp(msg) + cnv_to_local;
    return (sort_timestamp / 86400i32 as libc::c_long != send_timestamp / 86400i32 as libc::c_long)
        as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_sent(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return if (*msg).state >= 26i32 { 1i32 } else { 0i32 };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_starred(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return if 0 != (*msg).starred { 1i32 } else { 0i32 };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_forwarded(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return if 0 != dc_param_get_int((*msg).param, 'a' as i32, 0i32) {
        1i32
    } else {
        0i32
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_info(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    let mut cmd: libc::c_int = dc_param_get_int((*msg).param, 'S' as i32, 0i32);
    if (*msg).from_id == 2i32 as libc::c_uint
        || (*msg).to_id == 2i32 as libc::c_uint
        || 0 != cmd && cmd != 6i32
    {
        return 1i32;
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_increation(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint || (*msg).context.is_null() {
        return 0i32;
    }
    return (((*msg).type_0 == 20i32
        || (*msg).type_0 == 21i32
        || (*msg).type_0 == 40i32
        || (*msg).type_0 == 41i32
        || (*msg).type_0 == 50i32
        || (*msg).type_0 == 60i32)
        && (*msg).state == 18i32) as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_setupmessage(mut msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint || (*msg).type_0 != 60i32 {
        return 0i32;
    }
    return if dc_param_get_int((*msg).param, 'S' as i32, 0i32) == 6i32 {
        1i32
    } else {
        0i32
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_setupcodebegin(mut msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf_bytes: size_t = 0i32 as size_t;
    // just a pointer inside buf, MUST NOT be free()'d
    let mut buf_headerline: *const libc::c_char = 0 as *const libc::c_char;
    // just a pointer inside buf, MUST NOT be free()'d
    let mut buf_setupcodebegin: *const libc::c_char = 0 as *const libc::c_char;
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(0 == dc_msg_is_setupmessage(msg)) {
        filename = dc_msg_get_file(msg);
        if !(filename.is_null() || *filename.offset(0isize) as libc::c_int == 0i32) {
            if !(0
                == dc_read_file(
                    (*msg).context,
                    filename,
                    &mut buf as *mut *mut libc::c_char as *mut *mut libc::c_void,
                    &mut buf_bytes,
                )
                || buf.is_null()
                || buf_bytes <= 0i32 as libc::c_ulong)
            {
                if !(0
                    == dc_split_armored_data(
                        buf,
                        &mut buf_headerline,
                        &mut buf_setupcodebegin,
                        0 as *mut *const libc::c_char,
                        0 as *mut *const libc::c_char,
                    )
                    || strcmp(
                        buf_headerline,
                        b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
                    ) != 0i32
                    || buf_setupcodebegin.is_null())
                {
                    ret = dc_strdup(buf_setupcodebegin)
                }
            }
        }
    }
    free(filename as *mut libc::c_void);
    free(buf as *mut libc::c_void);
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_text(mut msg: *mut dc_msg_t, mut text: *const libc::c_char) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    free((*msg).text as *mut libc::c_void);
    (*msg).text = dc_strdup(text);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_file(
    mut msg: *mut dc_msg_t,
    mut file: *const libc::c_char,
    mut filemime: *const libc::c_char,
) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_param_set((*msg).param, 'f' as i32, file);
    dc_param_set((*msg).param, 'm' as i32, filemime);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_dimension(
    mut msg: *mut dc_msg_t,
    mut width: libc::c_int,
    mut height: libc::c_int,
) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_param_set_int((*msg).param, 'w' as i32, width);
    dc_param_set_int((*msg).param, 'h' as i32, height);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_duration(mut msg: *mut dc_msg_t, mut duration: libc::c_int) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_param_set_int((*msg).param, 'd' as i32, duration);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_latefiling_mediasize(
    mut msg: *mut dc_msg_t,
    mut width: libc::c_int,
    mut height: libc::c_int,
    mut duration: libc::c_int,
) {
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        if width > 0i32 && height > 0i32 {
            dc_param_set_int((*msg).param, 'w' as i32, width);
            dc_param_set_int((*msg).param, 'h' as i32, height);
        }
        if duration > 0i32 {
            dc_param_set_int((*msg).param, 'd' as i32, duration);
        }
        dc_msg_save_param_to_disk(msg);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_save_param_to_disk(mut msg: *mut dc_msg_t) {
    if msg.is_null()
        || (*msg).magic != 0x11561156i32 as libc::c_uint
        || (*msg).context.is_null()
        || (*(*msg).context).sql.is_null()
    {
        return;
    }
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*(*msg).context).sql,
        b"UPDATE msgs SET param=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, (*(*msg).param).packed, -1i32, None);
    sqlite3_bind_int(stmt, 2i32, (*msg).id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_msg_new_load(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) -> *mut dc_msg_t {
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    dc_msg_load_from_db(msg, context, msg_id);
    return msg;
}
#[no_mangle]
pub unsafe extern "C" fn dc_delete_msg_from_db(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) {
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(0 == dc_msg_load_from_db(msg, context, msg_id)) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"DELETE FROM msgs WHERE id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, (*msg).id as libc::c_int);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"DELETE FROM msgs_mdns WHERE msg_id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, (*msg).id as libc::c_int);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt
    }
    sqlite3_finalize(stmt);
    dc_msg_unref(msg);
}
/* as we do not cut inside words, this results in about 32-42 characters.
Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
It should also be very clear, the subject is _not_ the whole message.
The value is also used for CC:-summaries */
// Context functions to work with messages
#[no_mangle]
pub unsafe extern "C" fn dc_msg_exists(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) -> libc::c_int {
    let mut msg_exists: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg_id <= 9i32 as libc::c_uint)
    {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT chat_id FROM msgs WHERE id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, msg_id as libc::c_int);
        if sqlite3_step(stmt) == 100i32 {
            let mut chat_id: uint32_t = sqlite3_column_int(stmt, 0i32) as uint32_t;
            if chat_id != 3i32 as libc::c_uint {
                msg_exists = 1i32
            }
        }
    }
    sqlite3_finalize(stmt);
    return msg_exists;
}
#[no_mangle]
pub unsafe extern "C" fn dc_update_msg_move_state(
    mut context: *mut dc_context_t,
    mut rfc724_mid: *const libc::c_char,
    mut state: dc_move_state_t,
) {
    // we update the move_state for all messages belonging to a given Message-ID
    // so that the state stay intact when parts are deleted
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE msgs SET move_state=? WHERE rfc724_mid=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, state as libc::c_int);
    sqlite3_bind_text(stmt, 2i32, rfc724_mid, -1i32, None);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_msg_failed(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
    mut error: *const libc::c_char,
) {
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(0 == dc_msg_load_from_db(msg, context, msg_id)) {
        if 18i32 == (*msg).state || 20i32 == (*msg).state || 26i32 == (*msg).state {
            (*msg).state = 24i32
        }
        if !error.is_null() {
            dc_param_set((*msg).param, 'L' as i32, error);
            dc_log_error(
                context,
                0i32,
                b"%s\x00" as *const u8 as *const libc::c_char,
                error,
            );
        }
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"UPDATE msgs SET state=?, param=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, (*msg).state);
        sqlite3_bind_text(stmt, 2i32, (*(*msg).param).packed, -1i32, None);
        sqlite3_bind_int(stmt, 3i32, msg_id as libc::c_int);
        sqlite3_step(stmt);
        (*context).cb.expect("non-null function pointer")(
            context,
            2012i32,
            (*msg).chat_id as uintptr_t,
            msg_id as uintptr_t,
        );
    }
    sqlite3_finalize(stmt);
    dc_msg_unref(msg);
}
/* returns 1 if an event should be send */
#[no_mangle]
pub unsafe extern "C" fn dc_mdn_from_ext(
    mut context: *mut dc_context_t,
    mut from_id: uint32_t,
    mut rfc724_mid: *const libc::c_char,
    mut timestamp_sent: time_t,
    mut ret_chat_id: *mut uint32_t,
    mut ret_msg_id: *mut uint32_t,
) -> libc::c_int {
    let mut chat_type: libc::c_int = 0;
    let mut msg_state: libc::c_int = 0;
    let mut mdn_already_in_table: libc::c_int = 0;
    let mut ist_cnt: libc::c_int = 0;
    let mut soll_cnt: libc::c_int = 0;
    let mut read_by_all: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || from_id <= 9i32 as libc::c_uint
        || rfc724_mid.is_null()
        || ret_chat_id.is_null()
        || ret_msg_id.is_null()
        || *ret_chat_id != 0i32 as libc::c_uint
        || *ret_msg_id != 0i32 as libc::c_uint)
    {
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT m.id, c.id, c.type, m.state FROM msgs m  LEFT JOIN chats c ON m.chat_id=c.id  WHERE rfc724_mid=? AND from_id=1  ORDER BY m.id;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1i32, rfc724_mid, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            *ret_msg_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
            *ret_chat_id = sqlite3_column_int(stmt, 1i32) as uint32_t;
            chat_type = sqlite3_column_int(stmt, 2i32);
            msg_state = sqlite3_column_int(stmt, 3i32);
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            if !(msg_state != 18i32 && msg_state != 20i32 && msg_state != 26i32) {
                /* eg. already marked as MDNS_RCVD. however, it is importent, that the message ID is set above as this will allow the caller eg. to move the message away */
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"SELECT contact_id FROM msgs_mdns WHERE msg_id=? AND contact_id=?;\x00"
                        as *const u8 as *const libc::c_char,
                );
                sqlite3_bind_int(stmt, 1i32, *ret_msg_id as libc::c_int);
                sqlite3_bind_int(stmt, 2i32, from_id as libc::c_int);
                mdn_already_in_table = if sqlite3_step(stmt) == 100i32 {
                    1i32
                } else {
                    0i32
                };
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                if 0 == mdn_already_in_table {
                    stmt =
                        dc_sqlite3_prepare((*context).sql,
                                           b"INSERT INTO msgs_mdns (msg_id, contact_id, timestamp_sent) VALUES (?, ?, ?);\x00"
                                               as *const u8 as
                                               *const libc::c_char);
                    sqlite3_bind_int(stmt, 1i32, *ret_msg_id as libc::c_int);
                    sqlite3_bind_int(stmt, 2i32, from_id as libc::c_int);
                    sqlite3_bind_int64(stmt, 3i32, timestamp_sent as sqlite3_int64);
                    sqlite3_step(stmt);
                    sqlite3_finalize(stmt);
                    stmt = 0 as *mut sqlite3_stmt
                }
                // Normal chat? that's quite easy.
                if chat_type == 100i32 {
                    dc_update_msg_state(context, *ret_msg_id, 28i32);
                    read_by_all = 1i32
                } else {
                    /* send event about new state */
                    stmt = dc_sqlite3_prepare(
                        (*context).sql,
                        b"SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=?;\x00" as *const u8
                            as *const libc::c_char,
                    );
                    sqlite3_bind_int(stmt, 1i32, *ret_msg_id as libc::c_int);
                    if !(sqlite3_step(stmt) != 100i32) {
                        /* error */
                        ist_cnt = sqlite3_column_int(stmt, 0i32);
                        sqlite3_finalize(stmt);
                        stmt = 0 as *mut sqlite3_stmt;
                        /*
                        Groupsize:  Min. MDNs

                        1 S         n/a
                        2 SR        1
                        3 SRR       2
                        4 SRRR      2
                        5 SRRRR     3
                        6 SRRRRR    3

                        (S=Sender, R=Recipient)
                        */
                        /*for rounding, SELF is already included!*/
                        soll_cnt = (dc_get_chat_contact_cnt(context, *ret_chat_id) + 1i32) / 2i32;
                        if !(ist_cnt < soll_cnt) {
                            /* wait for more receipts */
                            dc_update_msg_state(context, *ret_msg_id, 28i32);
                            read_by_all = 1i32
                        }
                    }
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    return read_by_all;
}
/* the number of messages assigned to real chat (!=deaddrop, !=trash) */
#[no_mangle]
pub unsafe extern "C" fn dc_get_real_msg_cnt(mut context: *mut dc_context_t) -> size_t {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut ret: size_t = 0i32 as size_t;
    if !(*(*context).sql).cobj.is_null() {
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT COUNT(*)  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  WHERE m.id>9 AND m.chat_id>9 AND c.blocked=0;\x00"
                                   as *const u8 as *const libc::c_char);
        if sqlite3_step(stmt) != 100i32 {
            dc_sqlite3_log_error(
                (*context).sql,
                b"dc_get_real_msg_cnt() failed.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            ret = sqlite3_column_int(stmt, 0i32) as size_t
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_deaddrop_msg_cnt(mut context: *mut dc_context_t) -> size_t {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut ret: size_t = 0i32 as size_t;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null())
    {
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT COUNT(*) FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id WHERE c.blocked=2;\x00"
                                   as *const u8 as *const libc::c_char);
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32) as size_t
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_rfc724_mid_cnt(
    mut context: *mut dc_context_t,
    mut rfc724_mid: *const libc::c_char,
) -> libc::c_int {
    /* check the number of messages with the same rfc724_mid */
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null())
    {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT COUNT(*) FROM msgs WHERE rfc724_mid=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, rfc724_mid, -1i32, None);
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32)
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_rfc724_mid_exists(
    mut context: *mut dc_context_t,
    mut rfc724_mid: *const libc::c_char,
    mut ret_server_folder: *mut *mut libc::c_char,
    mut ret_server_uid: *mut uint32_t,
) -> uint32_t {
    let mut ret: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || rfc724_mid.is_null()
        || *rfc724_mid.offset(0isize) as libc::c_int == 0i32)
    {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT server_folder, server_uid, id FROM msgs WHERE rfc724_mid=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, rfc724_mid, -1i32, None);
        if sqlite3_step(stmt) != 100i32 {
            if !ret_server_folder.is_null() {
                *ret_server_folder = 0 as *mut libc::c_char
            }
            if !ret_server_uid.is_null() {
                *ret_server_uid = 0i32 as uint32_t
            }
        } else {
            if !ret_server_folder.is_null() {
                *ret_server_folder = dc_strdup(sqlite3_column_text(stmt, 0i32) as *mut libc::c_char)
            }
            if !ret_server_uid.is_null() {
                *ret_server_uid = sqlite3_column_int(stmt, 1i32) as uint32_t
            }
            ret = sqlite3_column_int(stmt, 2i32) as uint32_t
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_update_server_uid(
    mut context: *mut dc_context_t,
    mut rfc724_mid: *const libc::c_char,
    mut server_folder: *const libc::c_char,
    mut server_uid: uint32_t,
) {
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE msgs SET server_folder=?, server_uid=? WHERE rfc724_mid=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, server_folder, -1i32, None);
    sqlite3_bind_int(stmt, 2i32, server_uid as libc::c_int);
    sqlite3_bind_text(stmt, 3i32, rfc724_mid, -1i32, None);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

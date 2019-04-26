use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type sqlite3_stmt;
    #[no_mangle]
    fn __assert_rtn(
        _: *const libc::c_char,
        _: *const libc::c_char,
        _: libc::c_int,
        _: *const libc::c_char,
    ) -> !;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn strtol(_: *const libc::c_char, _: *mut *mut libc::c_char, _: libc::c_int) -> libc::c_long;
    #[no_mangle]
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn carray_new(initsize: libc::c_uint) -> *mut carray;
    #[no_mangle]
    fn carray_add(
        array: *mut carray,
        data: *mut libc::c_void,
        indx: *mut libc::c_uint,
    ) -> libc::c_int;
    #[no_mangle]
    fn carray_free(array: *mut carray);
    /* library-private */
    #[no_mangle]
    fn dc_param_new() -> *mut dc_param_t;
    #[no_mangle]
    fn dc_param_unref(_: *mut dc_param_t);
    #[no_mangle]
    fn dc_param_set_packed(_: *mut dc_param_t, _: *const libc::c_char);
    #[no_mangle]
    fn dc_msg_new_untyped(_: *mut dc_context_t) -> *mut dc_msg_t;
    #[no_mangle]
    fn dc_msg_unref(_: *mut dc_msg_t);
    #[no_mangle]
    fn dc_scaleup_contact_origin(_: *mut dc_context_t, contact_id: uint32_t, origin: libc::c_int);
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    /* Return the string with the given ID by calling DC_EVENT_GET_STRING.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str(_: *mut dc_context_t, id: libc::c_int) -> *mut libc::c_char;
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
    // Context functions to work with chatlist
    #[no_mangle]
    fn dc_get_archived_cnt(_: *mut dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn sqlite3_column_int64(_: *mut sqlite3_stmt, iCol: libc::c_int) -> sqlite3_int64;
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_msg_load_from_db(_: *mut dc_msg_t, _: *mut dc_context_t, id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_contact_unref(_: *mut dc_contact_t);
    #[no_mangle]
    fn sqlite3_free(_: *mut libc::c_void);
    #[no_mangle]
    fn sqlite3_mprintf(_: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_sqlite3_get_rowid(
        _: *mut dc_sqlite3_t,
        table: *const libc::c_char,
        field: *const libc::c_char,
        value: *const libc::c_char,
    ) -> uint32_t;
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
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_real_contact_exists(_: *mut dc_context_t, contact_id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_create_smeared_timestamp(_: *mut dc_context_t) -> time_t;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
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
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn sqlite3_column_type(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn dc_param_get_int(_: *const dc_param_t, key: libc::c_int, def: int32_t) -> int32_t;
    #[no_mangle]
    fn dc_sqlite3_get_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: int32_t,
    ) -> int32_t;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_create_outgoing_rfc724_mid(
        grpid: *const libc::c_char,
        addr: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_log_event(
        _: *mut dc_context_t,
        event_code: libc::c_int,
        data1: libc::c_int,
        msg: *const libc::c_char,
        _: ...
    );
    #[no_mangle]
    fn dc_msg_guess_msgtype_from_suffix(
        pathNfilename: *const libc::c_char,
        ret_msgtype: *mut libc::c_int,
        ret_mime: *mut *mut libc::c_char,
    );
    #[no_mangle]
    fn dc_make_rel_and_copy(
        _: *mut dc_context_t,
        pathNfilename: *mut *mut libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_is_blobdir_path(_: *mut dc_context_t, path: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_param_get(
        _: *const dc_param_t,
        key: libc::c_int,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_msg_save_param_to_disk(_: *mut dc_msg_t);
    #[no_mangle]
    fn dc_get_msg(_: *mut dc_context_t, msg_id: uint32_t) -> *mut dc_msg_t;
    #[no_mangle]
    fn dc_job_send_msg(_: *mut dc_context_t, msg_id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_update_msg_state(_: *mut dc_context_t, msg_id: uint32_t, state: libc::c_int);
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
    fn dc_msg_is_increation(_: *const dc_msg_t) -> libc::c_int;
    #[no_mangle]
    fn dc_delete_msg_from_db(_: *mut dc_context_t, _: uint32_t);
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
    fn dc_array_add_id(_: *mut dc_array_t, _: uint32_t);
    #[no_mangle]
    fn dc_gm2local_offset() -> libc::c_long;
    #[no_mangle]
    fn dc_array_get_id(_: *const dc_array_t, index: size_t) -> uint32_t;
    #[no_mangle]
    fn dc_array_get_cnt(_: *const dc_array_t) -> size_t;
    #[no_mangle]
    fn dc_sqlite3_rollback(_: *mut dc_sqlite3_t);
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
    fn dc_sqlite3_execute(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_begin_transaction(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_msg_set_text(_: *mut dc_msg_t, text: *const libc::c_char);
    /* Message-ID tools */
    #[no_mangle]
    fn dc_create_id() -> *mut libc::c_char;
    /* Replaces the first `%1$s` in the given String-ID by the given value.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str_repl_string(
        _: *mut dc_context_t,
        id: libc::c_int,
        value: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_contact(_: *mut dc_context_t, contact_id: uint32_t) -> *mut dc_contact_t;
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
    fn dc_contact_is_verified(_: *mut dc_contact_t) -> libc::c_int;
    #[no_mangle]
    fn dc_arr_to_string(arr: *const uint32_t, cnt: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_create_smeared_timestamps(_: *mut dc_context_t, count: libc::c_int) -> time_t;
    #[no_mangle]
    fn dc_stock_str_repl_int(
        _: *mut dc_context_t,
        id: libc::c_int,
        value: libc::c_int,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_contact_get_profile_image(_: *const dc_contact_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_abs_path(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
    ) -> *mut libc::c_char;
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
pub type int32_t = libc::c_int;
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
pub type sqlite3_int64 = sqlite_int64;
pub type sqlite_int64 = libc::c_longlong;
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
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
// handle chats
#[no_mangle]
pub unsafe extern "C" fn dc_create_chat_by_msg_id(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut send_event: libc::c_int = 0i32;
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut chat: *mut dc_chat_t = dc_chat_new(context);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(0 == dc_msg_load_from_db(msg, context, msg_id)
            || 0 == dc_chat_load_from_db(chat, (*msg).chat_id)
            || (*chat).id <= 9i32 as libc::c_uint)
        {
            chat_id = (*chat).id;
            if 0 != (*chat).blocked {
                dc_unblock_chat(context, (*chat).id);
                send_event = 1i32
            }
            dc_scaleup_contact_origin(context, (*msg).from_id, 0x800i32);
        }
    }
    dc_msg_unref(msg);
    dc_chat_unref(chat);
    if 0 != send_event {
        (*context).cb.expect("non-null function pointer")(
            context,
            2000i32,
            0i32 as uintptr_t,
            0i32 as uintptr_t,
        );
    }
    return chat_id;
}
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
pub unsafe extern "C" fn dc_chat_new(mut context: *mut dc_context_t) -> *mut dc_chat_t {
    let mut chat: *mut dc_chat_t = 0 as *mut dc_chat_t;
    if context.is_null() || {
        chat = calloc(
            1i32 as libc::c_ulong,
            ::std::mem::size_of::<dc_chat_t>() as libc::c_ulong,
        ) as *mut dc_chat_t;
        chat.is_null()
    } {
        exit(14i32);
    }
    (*chat).magic = 0xc4a7c4a7u32;
    (*chat).context = context;
    (*chat).type_0 = 0i32;
    (*chat).param = dc_param_new();
    return chat;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_unref(mut chat: *mut dc_chat_t) {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return;
    }
    dc_chat_empty(chat);
    dc_param_unref((*chat).param);
    (*chat).magic = 0i32 as uint32_t;
    free(chat as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_empty(mut chat: *mut dc_chat_t) {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return;
    }
    free((*chat).name as *mut libc::c_void);
    (*chat).name = 0 as *mut libc::c_char;
    (*chat).type_0 = 0i32;
    (*chat).id = 0i32 as uint32_t;
    free((*chat).grpid as *mut libc::c_void);
    (*chat).grpid = 0 as *mut libc::c_char;
    (*chat).blocked = 0i32;
    (*chat).gossiped_timestamp = 0i32 as time_t;
    dc_param_set_packed((*chat).param, 0 as *const libc::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn dc_unblock_chat(mut context: *mut dc_context_t, mut chat_id: uint32_t) {
    dc_block_chat(context, chat_id, 0i32);
}
#[no_mangle]
pub unsafe extern "C" fn dc_block_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut new_blocking: libc::c_int,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE chats SET blocked=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, new_blocking);
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_load_from_db(
    mut chat: *mut dc_chat_t,
    mut chat_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        dc_chat_empty(chat);
        stmt =
            dc_sqlite3_prepare((*(*chat).context).sql,
                               b"SELECT  c.id,c.type,c.name, c.grpid,c.param,c.archived, c.blocked, c.gossiped_timestamp, c.locations_send_until  FROM chats c WHERE c.id=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        if !(sqlite3_step(stmt) != 100i32) {
            if !(0 == set_from_stmt(chat, stmt)) {
                success = 1i32
            }
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
unsafe extern "C" fn set_from_stmt(
    mut chat: *mut dc_chat_t,
    mut row: *mut sqlite3_stmt,
) -> libc::c_int {
    let mut row_offset: libc::c_int = 0i32;
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 || row.is_null() {
        return 0i32;
    }
    dc_chat_empty(chat);
    let fresh0 = row_offset;
    row_offset = row_offset + 1;
    (*chat).id = sqlite3_column_int(row, fresh0) as uint32_t;
    let fresh1 = row_offset;
    row_offset = row_offset + 1;
    (*chat).type_0 = sqlite3_column_int(row, fresh1);
    let fresh2 = row_offset;
    row_offset = row_offset + 1;
    (*chat).name = dc_strdup(sqlite3_column_text(row, fresh2) as *mut libc::c_char);
    let fresh3 = row_offset;
    row_offset = row_offset + 1;
    (*chat).grpid = dc_strdup(sqlite3_column_text(row, fresh3) as *mut libc::c_char);
    let fresh4 = row_offset;
    row_offset = row_offset + 1;
    dc_param_set_packed(
        (*chat).param,
        sqlite3_column_text(row, fresh4) as *mut libc::c_char,
    );
    let fresh5 = row_offset;
    row_offset = row_offset + 1;
    (*chat).archived = sqlite3_column_int(row, fresh5);
    let fresh6 = row_offset;
    row_offset = row_offset + 1;
    (*chat).blocked = sqlite3_column_int(row, fresh6);
    let fresh7 = row_offset;
    row_offset = row_offset + 1;
    (*chat).gossiped_timestamp = sqlite3_column_int64(row, fresh7) as time_t;
    let fresh8 = row_offset;
    row_offset = row_offset + 1;
    (*chat).is_sending_locations = (sqlite3_column_int64(row, fresh8)
        > time(0 as *mut time_t) as libc::c_longlong)
        as libc::c_int;
    if (*chat).id == 1i32 as libc::c_uint {
        free((*chat).name as *mut libc::c_void);
        (*chat).name = dc_stock_str((*chat).context, 8i32)
    } else if (*chat).id == 6i32 as libc::c_uint {
        free((*chat).name as *mut libc::c_void);
        let mut tempname: *mut libc::c_char = dc_stock_str((*chat).context, 40i32);
        (*chat).name = dc_mprintf(
            b"%s (%i)\x00" as *const u8 as *const libc::c_char,
            tempname,
            dc_get_archived_cnt((*chat).context),
        );
        free(tempname as *mut libc::c_void);
    } else if (*chat).id == 5i32 as libc::c_uint {
        free((*chat).name as *mut libc::c_void);
        (*chat).name = dc_stock_str((*chat).context, 41i32)
    } else if 0 != dc_param_exists((*chat).param, 'K' as i32) {
        free((*chat).name as *mut libc::c_void);
        (*chat).name = dc_stock_str((*chat).context, 2i32)
    }
    return row_offset;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_chat_by_contact_id(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_blocked: libc::c_int = 0i32;
    let mut send_event: libc::c_int = 0i32;
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_blocked);
    if 0 != chat_id {
        if 0 != chat_blocked {
            dc_unblock_chat(context, chat_id);
            send_event = 1i32
        }
    } else if 0i32 == dc_real_contact_exists(context, contact_id)
        && contact_id != 1i32 as libc::c_uint
    {
        dc_log_warning(
            context,
            0i32,
            b"Cannot create chat, contact %i does not exist.\x00" as *const u8
                as *const libc::c_char,
            contact_id as libc::c_int,
        );
    } else {
        dc_create_or_lookup_nchat_by_contact_id(
            context,
            contact_id,
            0i32,
            &mut chat_id,
            0 as *mut libc::c_int,
        );
        if 0 != chat_id {
            send_event = 1i32
        }
        dc_scaleup_contact_origin(context, contact_id, 0x800i32);
    }
    if 0 != send_event {
        (*context).cb.expect("non-null function pointer")(
            context,
            2000i32,
            0i32 as uintptr_t,
            0i32 as uintptr_t,
        );
    }
    return chat_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_or_lookup_nchat_by_contact_id(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
    mut create_blocked: libc::c_int,
    mut ret_chat_id: *mut uint32_t,
    mut ret_chat_blocked: *mut libc::c_int,
) {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_blocked: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut chat_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut q: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !ret_chat_id.is_null() {
        *ret_chat_id = 0i32 as uint32_t
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = 0i32
    }
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null()
    {
        return;
    }
    if contact_id == 0i32 as libc::c_uint {
        return;
    }
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_blocked);
    if chat_id != 0i32 as libc::c_uint {
        if !ret_chat_id.is_null() {
            *ret_chat_id = chat_id
        }
        if !ret_chat_blocked.is_null() {
            *ret_chat_blocked = chat_blocked
        }
        return;
    }
    contact = dc_contact_new(context);
    if !(0 == dc_contact_load_from_db(contact, (*context).sql, contact_id)) {
        chat_name =
            if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
                (*contact).name
            } else {
                (*contact).addr
            };
        q = sqlite3_mprintf(
            b"INSERT INTO chats (type, name, param, blocked, grpid) VALUES(%i, %Q, %Q, %i, %Q)\x00"
                as *const u8 as *const libc::c_char,
            100i32,
            chat_name,
            if contact_id == 1i32 as libc::c_uint {
                b"K=1\x00" as *const u8 as *const libc::c_char
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            create_blocked,
            (*contact).addr,
        );
        if 0 != !('K' as i32 == 'K' as i32) as libc::c_int as libc::c_long {
            __assert_rtn(
                (*::std::mem::transmute::<&[u8; 40], &[libc::c_char; 40]>(
                    b"dc_create_or_lookup_nchat_by_contact_id\x00",
                ))
                .as_ptr(),
                b"../src/dc_chat.c\x00" as *const u8 as *const libc::c_char,
                1386i32,
                b"DC_PARAM_SELFTALK==\'K\'\x00" as *const u8 as *const libc::c_char,
            );
        } else {
        };
        stmt = dc_sqlite3_prepare((*context).sql, q);
        if !stmt.is_null() {
            if !(sqlite3_step(stmt) != 101i32) {
                chat_id = dc_sqlite3_get_rowid(
                    (*context).sql,
                    b"chats\x00" as *const u8 as *const libc::c_char,
                    b"grpid\x00" as *const u8 as *const libc::c_char,
                    (*contact).addr,
                );
                sqlite3_free(q as *mut libc::c_void);
                q = 0 as *mut libc::c_char;
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                q = sqlite3_mprintf(
                    b"INSERT INTO chats_contacts (chat_id, contact_id) VALUES(%i, %i)\x00"
                        as *const u8 as *const libc::c_char,
                    chat_id,
                    contact_id,
                );
                stmt = dc_sqlite3_prepare((*context).sql, q);
                if !(sqlite3_step(stmt) != 101i32) {
                    sqlite3_free(q as *mut libc::c_void);
                    q = 0 as *mut libc::c_char;
                    sqlite3_finalize(stmt);
                    stmt = 0 as *mut sqlite3_stmt
                }
            }
        }
    }
    sqlite3_free(q as *mut libc::c_void);
    sqlite3_finalize(stmt);
    dc_contact_unref(contact);
    if !ret_chat_id.is_null() {
        *ret_chat_id = chat_id
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = create_blocked
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_lookup_real_nchat_by_contact_id(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
    mut ret_chat_id: *mut uint32_t,
    mut ret_chat_blocked: *mut libc::c_int,
) {
    /* checks for "real" chats or self-chat */
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !ret_chat_id.is_null() {
        *ret_chat_id = 0i32 as uint32_t
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = 0i32
    }
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null()
    {
        return;
    }
    stmt =
        dc_sqlite3_prepare((*context).sql,
                           b"SELECT c.id, c.blocked FROM chats c INNER JOIN chats_contacts j ON c.id=j.chat_id WHERE c.type=100 AND c.id>9 AND j.contact_id=?;\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        if !ret_chat_id.is_null() {
            *ret_chat_id = sqlite3_column_int(stmt, 0i32) as uint32_t
        }
        if !ret_chat_blocked.is_null() {
            *ret_chat_blocked = sqlite3_column_int(stmt, 1i32)
        }
    }
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_id_by_contact_id(
    mut context: *mut dc_context_t,
    mut contact_id: uint32_t,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0i32;
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_id_blocked);
    return if 0 != chat_id_blocked {
        0i32 as libc::c_uint
    } else {
        chat_id
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_prepare_msg(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut msg: *mut dc_msg_t,
) -> uint32_t {
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg.is_null()
        || chat_id <= 9i32 as libc::c_uint
    {
        return 0i32 as uint32_t;
    }
    (*msg).state = 18i32;
    let mut msg_id: uint32_t = prepare_msg_common(context, chat_id, msg);
    (*context).cb.expect("non-null function pointer")(
        context,
        2000i32,
        (*msg).chat_id as uintptr_t,
        (*msg).id as uintptr_t,
    );
    return msg_id;
}
unsafe extern "C" fn prepare_msg_common(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut msg: *mut dc_msg_t,
) -> uint32_t {
    let mut current_block: u64;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut chat: *mut dc_chat_t = 0 as *mut dc_chat_t;
    (*msg).id = 0i32 as uint32_t;
    (*msg).context = context;
    if (*msg).type_0 == 10i32 {
        current_block = 17281240262373992796;
    } else if (*msg).type_0 == 20i32
        || (*msg).type_0 == 21i32
        || (*msg).type_0 == 40i32
        || (*msg).type_0 == 41i32
        || (*msg).type_0 == 50i32
        || (*msg).type_0 == 60i32
    {
        pathNfilename = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
        if pathNfilename.is_null() {
            dc_log_error(
                context,
                0i32,
                b"Attachment missing for message of type #%i.\x00" as *const u8
                    as *const libc::c_char,
                (*msg).type_0 as libc::c_int,
            );
            current_block = 2171833246886114521;
        } else if (*msg).state == 18i32 && 0 == dc_is_blobdir_path(context, pathNfilename) {
            dc_log_error(
                context,
                0i32,
                b"Files must be created in the blob-directory.\x00" as *const u8
                    as *const libc::c_char,
            );
            current_block = 2171833246886114521;
        } else if 0 == dc_make_rel_and_copy(context, &mut pathNfilename) {
            current_block = 2171833246886114521;
        } else {
            dc_param_set((*msg).param, 'f' as i32, pathNfilename);
            if (*msg).type_0 == 60i32 || (*msg).type_0 == 20i32 {
                let mut better_type: libc::c_int = 0i32;
                let mut better_mime: *mut libc::c_char = 0 as *mut libc::c_char;
                dc_msg_guess_msgtype_from_suffix(pathNfilename, &mut better_type, &mut better_mime);
                if 0 != better_type {
                    (*msg).type_0 = better_type;
                    dc_param_set((*msg).param, 'm' as i32, better_mime);
                }
                free(better_mime as *mut libc::c_void);
            } else if 0 == dc_param_exists((*msg).param, 'm' as i32) {
                let mut better_mime_0: *mut libc::c_char = 0 as *mut libc::c_char;
                dc_msg_guess_msgtype_from_suffix(
                    pathNfilename,
                    0 as *mut libc::c_int,
                    &mut better_mime_0,
                );
                dc_param_set((*msg).param, 'm' as i32, better_mime_0);
                free(better_mime_0 as *mut libc::c_void);
            }
            dc_log_info(
                context,
                0i32,
                b"Attaching \"%s\" for message type #%i.\x00" as *const u8 as *const libc::c_char,
                pathNfilename,
                (*msg).type_0 as libc::c_int,
            );
            current_block = 17281240262373992796;
        }
    } else {
        dc_log_error(
            context,
            0i32,
            b"Cannot send messages of type #%i.\x00" as *const u8 as *const libc::c_char,
            (*msg).type_0 as libc::c_int,
        );
        current_block = 2171833246886114521;
    }
    match current_block {
        17281240262373992796 => {
            dc_unarchive_chat(context, chat_id);
            (*(*context).smtp).log_connect_errors = 1i32;
            chat = dc_chat_new(context);
            if 0 != dc_chat_load_from_db(chat, chat_id) {
                if (*msg).state != 18i32 {
                    (*msg).state = 20i32
                }
                (*msg).id =
                    prepare_msg_raw(context, chat, msg, dc_create_smeared_timestamp(context));
                (*msg).chat_id = chat_id
            }
        }
        _ => {}
    }
    /* potential error already logged */
    dc_chat_unref(chat);
    free(pathNfilename as *mut libc::c_void);
    return (*msg).id;
}
unsafe extern "C" fn prepare_msg_raw(
    mut context: *mut dc_context_t,
    mut chat: *mut dc_chat_t,
    mut msg: *const dc_msg_t,
    mut timestamp: time_t,
) -> uint32_t {
    let mut do_guarantee_e2ee: libc::c_int = 0;
    let mut e2ee_enabled: libc::c_int = 0;
    let mut current_block: u64;
    let mut parent_rfc724_mid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parent_references: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parent_in_reply_to: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut new_rfc724_mid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut new_references: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut new_in_reply_to: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut msg_id: uint32_t = 0i32 as uint32_t;
    let mut to_id: uint32_t = 0i32 as uint32_t;
    if !((*chat).type_0 == 100i32 || (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32) {
        dc_log_error(
            context,
            0i32,
            b"Cannot send to chat type #%i.\x00" as *const u8 as *const libc::c_char,
            (*chat).type_0,
        );
    } else if ((*chat).type_0 == 120i32 || (*chat).type_0 == 130i32)
        && 0 == dc_is_contact_in_chat(context, (*chat).id, 1i32 as uint32_t)
    {
        dc_log_event(
            context,
            410i32,
            0i32,
            b"Cannot send message; self not in group.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        let mut from: *mut libc::c_char = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if from.is_null() {
            dc_log_error(
                context,
                0i32,
                b"Cannot send message, not configured.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            new_rfc724_mid = dc_create_outgoing_rfc724_mid(
                if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
                    (*chat).grpid
                } else {
                    0 as *mut libc::c_char
                },
                from,
            );
            free(from as *mut libc::c_void);
            if (*chat).type_0 == 100i32 {
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"SELECT contact_id FROM chats_contacts WHERE chat_id=?;\x00" as *const u8
                        as *const libc::c_char,
                );
                sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
                if sqlite3_step(stmt) != 100i32 {
                    dc_log_error(
                        context,
                        0i32,
                        b"Cannot send message, contact for chat #%i not found.\x00" as *const u8
                            as *const libc::c_char,
                        (*chat).id,
                    );
                    current_block = 10477488590406205504;
                } else {
                    to_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
                    sqlite3_finalize(stmt);
                    stmt = 0 as *mut sqlite3_stmt;
                    current_block = 5689316957504528238;
                }
            } else {
                if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
                    if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 1i32 {
                        dc_param_set((*chat).param, 'U' as i32, 0 as *const libc::c_char);
                        dc_chat_update_param(chat);
                    }
                }
                current_block = 5689316957504528238;
            }
            match current_block {
                10477488590406205504 => {}
                _ => {
                    /* check if we can guarantee E2EE for this message.
                    if we guarantee E2EE, and circumstances change
                    so that E2EE is no longer available at a later point (reset, changed settings),
                    we do not send the message out at all */
                    do_guarantee_e2ee = 0i32;
                    e2ee_enabled = dc_sqlite3_get_config_int(
                        (*context).sql,
                        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                        1i32,
                    );
                    if 0 != e2ee_enabled && dc_param_get_int((*msg).param, 'u' as i32, 0i32) == 0i32
                    {
                        let mut can_encrypt: libc::c_int = 1i32;
                        let mut all_mutual: libc::c_int = 1i32;
                        stmt =
                            dc_sqlite3_prepare((*context).sql,
                                               b"SELECT ps.prefer_encrypted, c.addr FROM chats_contacts cc  LEFT JOIN contacts c ON cc.contact_id=c.id  LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE cc.chat_id=?  AND cc.contact_id>9;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                        sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
                        while sqlite3_step(stmt) == 100i32 {
                            if sqlite3_column_type(stmt, 0i32) == 5i32 {
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"[autocrypt] no peerstate for %s\x00" as *const u8
                                        as *const libc::c_char,
                                    sqlite3_column_text(stmt, 1i32),
                                );
                                can_encrypt = 0i32;
                                all_mutual = 0i32
                            } else {
                                let mut prefer_encrypted: libc::c_int =
                                    sqlite3_column_int(stmt, 0i32);
                                if prefer_encrypted != 1i32 {
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"[autocrypt] peerstate for %s is %s\x00" as *const u8
                                            as *const libc::c_char,
                                        sqlite3_column_text(stmt, 1i32),
                                        if prefer_encrypted == 0i32 {
                                            b"NOPREFERENCE\x00" as *const u8 as *const libc::c_char
                                        } else {
                                            b"RESET\x00" as *const u8 as *const libc::c_char
                                        },
                                    );
                                    all_mutual = 0i32
                                }
                            }
                        }
                        sqlite3_finalize(stmt);
                        stmt = 0 as *mut sqlite3_stmt;
                        if 0 != can_encrypt {
                            if 0 != all_mutual {
                                do_guarantee_e2ee = 1i32
                            } else if 0 != last_msg_in_chat_encrypted((*context).sql, (*chat).id) {
                                do_guarantee_e2ee = 1i32
                            }
                        }
                    }
                    if 0 != do_guarantee_e2ee {
                        dc_param_set_int((*msg).param, 'c' as i32, 1i32);
                    }
                    dc_param_set((*msg).param, 'e' as i32, 0 as *const libc::c_char);
                    if 0 == dc_chat_is_self_talk(chat)
                        && 0 != get_parent_mime_headers(
                            chat,
                            &mut parent_rfc724_mid,
                            &mut parent_in_reply_to,
                            &mut parent_references,
                        )
                    {
                        if !parent_rfc724_mid.is_null()
                            && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                        {
                            new_in_reply_to = dc_strdup(parent_rfc724_mid)
                        }
                        if !parent_references.is_null() {
                            let mut space: *mut libc::c_char = 0 as *mut libc::c_char;
                            space = strchr(parent_references, ' ' as i32);
                            if !space.is_null() {
                                *space = 0i32 as libc::c_char
                            }
                        }
                        if !parent_references.is_null()
                            && 0 != *parent_references.offset(0isize) as libc::c_int
                            && !parent_rfc724_mid.is_null()
                            && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                        {
                            new_references = dc_mprintf(
                                b"%s %s\x00" as *const u8 as *const libc::c_char,
                                parent_references,
                                parent_rfc724_mid,
                            )
                        } else if !parent_references.is_null()
                            && 0 != *parent_references.offset(0isize) as libc::c_int
                        {
                            new_references = dc_strdup(parent_references)
                        } else if !parent_in_reply_to.is_null()
                            && 0 != *parent_in_reply_to.offset(0isize) as libc::c_int
                            && !parent_rfc724_mid.is_null()
                            && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                        {
                            new_references = dc_mprintf(
                                b"%s %s\x00" as *const u8 as *const libc::c_char,
                                parent_in_reply_to,
                                parent_rfc724_mid,
                            )
                        } else if !parent_in_reply_to.is_null()
                            && 0 != *parent_in_reply_to.offset(0isize) as libc::c_int
                        {
                            new_references = dc_strdup(parent_in_reply_to)
                        }
                    }
                    stmt =
                        dc_sqlite3_prepare((*context).sql,
                                           b"INSERT INTO msgs (rfc724_mid, chat_id, from_id, to_id, timestamp, type, state, txt, param, hidden, mime_in_reply_to, mime_references) VALUES (?,?,?,?,?, ?,?,?,?,?, ?,?);\x00"
                                               as *const u8 as
                                               *const libc::c_char);
                    sqlite3_bind_text(stmt, 1i32, new_rfc724_mid, -1i32, None);
                    sqlite3_bind_int(stmt, 2i32, (*chat).id as libc::c_int);
                    sqlite3_bind_int(stmt, 3i32, 1i32);
                    sqlite3_bind_int(stmt, 4i32, to_id as libc::c_int);
                    sqlite3_bind_int64(stmt, 5i32, timestamp as sqlite3_int64);
                    sqlite3_bind_int(stmt, 6i32, (*msg).type_0);
                    sqlite3_bind_int(stmt, 7i32, (*msg).state);
                    sqlite3_bind_text(
                        stmt,
                        8i32,
                        if !(*msg).text.is_null() {
                            (*msg).text
                        } else {
                            b"\x00" as *const u8 as *const libc::c_char
                        },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_text(stmt, 9i32, (*(*msg).param).packed, -1i32, None);
                    sqlite3_bind_int(stmt, 10i32, (*msg).hidden);
                    sqlite3_bind_text(stmt, 11i32, new_in_reply_to, -1i32, None);
                    sqlite3_bind_text(stmt, 12i32, new_references, -1i32, None);
                    if sqlite3_step(stmt) != 101i32 {
                        dc_log_error(
                            context,
                            0i32,
                            b"Cannot send message, cannot insert to database.\x00" as *const u8
                                as *const libc::c_char,
                            (*chat).id,
                        );
                    } else {
                        msg_id = dc_sqlite3_get_rowid(
                            (*context).sql,
                            b"msgs\x00" as *const u8 as *const libc::c_char,
                            b"rfc724_mid\x00" as *const u8 as *const libc::c_char,
                            new_rfc724_mid,
                        )
                    }
                }
            }
        }
    }
    free(parent_rfc724_mid as *mut libc::c_void);
    free(parent_in_reply_to as *mut libc::c_void);
    free(parent_references as *mut libc::c_void);
    free(new_rfc724_mid as *mut libc::c_void);
    free(new_in_reply_to as *mut libc::c_void);
    free(new_references as *mut libc::c_void);
    sqlite3_finalize(stmt);
    return msg_id;
}
unsafe extern "C" fn get_parent_mime_headers(
    mut chat: *const dc_chat_t,
    mut parent_rfc724_mid: *mut *mut libc::c_char,
    mut parent_in_reply_to: *mut *mut libc::c_char,
    mut parent_references: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(chat.is_null()
        || parent_rfc724_mid.is_null()
        || parent_in_reply_to.is_null()
        || parent_references.is_null())
    {
        stmt =
            dc_sqlite3_prepare((*(*chat).context).sql,
                               b"SELECT rfc724_mid, mime_in_reply_to, mime_references FROM msgs WHERE timestamp=(SELECT max(timestamp) FROM msgs WHERE chat_id=? AND from_id!=?);\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
        sqlite3_bind_int(stmt, 2i32, 1i32);
        if sqlite3_step(stmt) == 100i32 {
            *parent_rfc724_mid = dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char);
            *parent_in_reply_to = dc_strdup(sqlite3_column_text(stmt, 1i32) as *const libc::c_char);
            *parent_references = dc_strdup(sqlite3_column_text(stmt, 2i32) as *const libc::c_char);
            success = 1i32
        }
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if 0 == success {
            stmt =
                dc_sqlite3_prepare((*(*chat).context).sql,
                                   b"SELECT rfc724_mid, mime_in_reply_to, mime_references FROM msgs WHERE timestamp=(SELECT min(timestamp) FROM msgs WHERE chat_id=? AND from_id==?);\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
            sqlite3_bind_int(stmt, 2i32, 1i32);
            if sqlite3_step(stmt) == 100i32 {
                *parent_rfc724_mid =
                    dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char);
                *parent_in_reply_to =
                    dc_strdup(sqlite3_column_text(stmt, 1i32) as *const libc::c_char);
                *parent_references =
                    dc_strdup(sqlite3_column_text(stmt, 2i32) as *const libc::c_char);
                success = 1i32
            }
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_self_talk(mut chat: *const dc_chat_t) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    return dc_param_exists((*chat).param, 'K' as i32);
}
/* ******************************************************************************
 * Sending messages
 ******************************************************************************/
unsafe extern "C" fn last_msg_in_chat_encrypted(
    mut sql: *mut dc_sqlite3_t,
    mut chat_id: uint32_t,
) -> libc::c_int {
    let mut last_is_encrypted: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt =
        dc_sqlite3_prepare(sql,
                           b"SELECT param  FROM msgs  WHERE timestamp=(SELECT MAX(timestamp) FROM msgs WHERE chat_id=?)  ORDER BY id DESC;\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        let mut msg_param: *mut dc_param_t = dc_param_new();
        dc_param_set_packed(
            msg_param,
            sqlite3_column_text(stmt, 0i32) as *mut libc::c_char,
        );
        if 0 != dc_param_exists(msg_param, 'c' as i32) {
            last_is_encrypted = 1i32
        }
        dc_param_unref(msg_param);
    }
    sqlite3_finalize(stmt);
    return last_is_encrypted;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_update_param(mut chat: *mut dc_chat_t) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*(*chat).context).sql,
        b"UPDATE chats SET param=? WHERE id=?\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, (*(*chat).param).packed, -1i32, None);
    sqlite3_bind_int(stmt, 2i32, (*chat).id as libc::c_int);
    success = if sqlite3_step(stmt) == 101i32 {
        1i32
    } else {
        0i32
    };
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_contact_in_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    /* this function works for group and for normal chats, however, it is more useful for group chats.
    DC_CONTACT_ID_SELF may be used to check, if the user itself is in a group chat (DC_CONTACT_ID_SELF is not added to normal chats) */
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT contact_id FROM chats_contacts WHERE chat_id=? AND contact_id=?;\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
        ret = if sqlite3_step(stmt) == 100i32 {
            1i32
        } else {
            0i32
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_unarchive_chat(mut context: *mut dc_context_t, mut chat_id: uint32_t) {
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE chats SET archived=0 WHERE id=?\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_send_msg(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut msg: *mut dc_msg_t,
) -> uint32_t {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || msg.is_null() {
        return 0i32 as uint32_t;
    }
    if (*msg).state != 18i32 {
        if 0 == prepare_msg_common(context, chat_id, msg) {
            return 0i32 as uint32_t;
        }
    } else {
        if chat_id != 0i32 as libc::c_uint && chat_id != (*msg).chat_id {
            return 0i32 as uint32_t;
        }
        dc_update_msg_state(context, (*msg).id, 20i32);
    }
    if 0 == dc_job_send_msg(context, (*msg).id) {
        return 0i32 as uint32_t;
    }
    (*context).cb.expect("non-null function pointer")(
        context,
        2000i32,
        (*msg).chat_id as uintptr_t,
        (*msg).id as uintptr_t,
    );
    if 0 == chat_id {
        let mut forwards: *mut libc::c_char =
            dc_param_get((*msg).param, 'P' as i32, 0 as *const libc::c_char);
        if !forwards.is_null() {
            let mut p: *mut libc::c_char = forwards;
            while 0 != *p {
                let mut id: int32_t = strtol(p, &mut p, 10i32) as int32_t;
                if 0 == id {
                    // avoid hanging if user tampers with db
                    break;
                } else {
                    let mut copy: *mut dc_msg_t = dc_get_msg(context, id as uint32_t);
                    if !copy.is_null() {
                        dc_send_msg(context, 0i32 as uint32_t, copy);
                    }
                    dc_msg_unref(copy);
                }
            }
            dc_param_set((*msg).param, 'P' as i32, 0 as *const libc::c_char);
            dc_msg_save_param_to_disk(msg);
        }
        free(forwards as *mut libc::c_void);
    }
    return (*msg).id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_send_text_msg(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut text_to_send: *const libc::c_char,
) -> uint32_t {
    let mut msg: *mut dc_msg_t = dc_msg_new(context, 10i32);
    let mut ret: uint32_t = 0i32 as uint32_t;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint
        || text_to_send.is_null())
    {
        (*msg).text = dc_strdup(text_to_send);
        ret = dc_send_msg(context, chat_id, msg)
    }
    dc_msg_unref(msg);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_draft(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut msg: *mut dc_msg_t,
) {
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint
    {
        return;
    }
    if 0 != set_draft_raw(context, chat_id, msg) {
        (*context).cb.expect("non-null function pointer")(
            context,
            2000i32,
            chat_id as uintptr_t,
            0i32 as uintptr_t,
        );
    };
}
unsafe extern "C" fn set_draft_raw(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut msg: *mut dc_msg_t,
) -> libc::c_int {
    let mut current_block: u64;
    // similar to as dc_set_draft() but does not emit an event
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut prev_draft_msg_id: uint32_t = 0i32 as uint32_t;
    let mut sth_changed: libc::c_int = 0i32;
    prev_draft_msg_id = get_draft_msg_id(context, chat_id);
    if 0 != prev_draft_msg_id {
        dc_delete_msg_from_db(context, prev_draft_msg_id);
        sth_changed = 1i32
    }
    // save new draft
    if !msg.is_null() {
        if (*msg).type_0 == 10i32 {
            if (*msg).text.is_null() || *(*msg).text.offset(0isize) as libc::c_int == 0i32 {
                current_block = 14513523936503887211;
            } else {
                current_block = 4495394744059808450;
            }
        } else if (*msg).type_0 == 20i32
            || (*msg).type_0 == 21i32
            || (*msg).type_0 == 40i32
            || (*msg).type_0 == 41i32
            || (*msg).type_0 == 50i32
            || (*msg).type_0 == 60i32
        {
            pathNfilename = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
            if pathNfilename.is_null() {
                current_block = 14513523936503887211;
            } else if 0 != dc_msg_is_increation(msg)
                && 0 == dc_is_blobdir_path(context, pathNfilename)
            {
                current_block = 14513523936503887211;
            } else if 0 == dc_make_rel_and_copy(context, &mut pathNfilename) {
                current_block = 14513523936503887211;
            } else {
                dc_param_set((*msg).param, 'f' as i32, pathNfilename);
                current_block = 4495394744059808450;
            }
        } else {
            current_block = 14513523936503887211;
        }
        match current_block {
            14513523936503887211 => {}
            _ => {
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"INSERT INTO msgs (chat_id, from_id, timestamp, type, state, txt, param, hidden) VALUES (?,?,?, ?,?,?,?,?);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                sqlite3_bind_int(stmt, 2i32, 1i32);
                sqlite3_bind_int64(stmt, 3i32, time(0 as *mut time_t) as sqlite3_int64);
                sqlite3_bind_int(stmt, 4i32, (*msg).type_0);
                sqlite3_bind_int(stmt, 5i32, 19i32);
                sqlite3_bind_text(
                    stmt,
                    6i32,
                    if !(*msg).text.is_null() {
                        (*msg).text
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    -1i32,
                    None,
                );
                sqlite3_bind_text(stmt, 7i32, (*(*msg).param).packed, -1i32, None);
                sqlite3_bind_int(stmt, 8i32, 1i32);
                if !(sqlite3_step(stmt) != 101i32) {
                    sth_changed = 1i32
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    free(pathNfilename as *mut libc::c_void);
    return sth_changed;
}
unsafe extern "C" fn get_draft_msg_id(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> uint32_t {
    let mut draft_msg_id: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT id FROM msgs WHERE chat_id=? AND state=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, 19i32);
    if sqlite3_step(stmt) == 100i32 {
        draft_msg_id = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    return draft_msg_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_draft(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> *mut dc_msg_t {
    let mut draft_msg_id: uint32_t = 0i32 as uint32_t;
    let mut draft_msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint
    {
        return 0 as *mut dc_msg_t;
    }
    draft_msg_id = get_draft_msg_id(context, chat_id);
    if draft_msg_id == 0i32 as libc::c_uint {
        return 0 as *mut dc_msg_t;
    }
    draft_msg = dc_msg_new_untyped(context);
    if 0 == dc_msg_load_from_db(draft_msg, context, draft_msg_id) {
        dc_msg_unref(draft_msg);
        return 0 as *mut dc_msg_t;
    }
    return draft_msg;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_msgs(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut flags: uint32_t,
    mut marker1before: uint32_t,
) -> *mut dc_array_t {
    //clock_t       start = clock();
    let mut success: libc::c_int = 0i32;
    let mut ret: *mut dc_array_t = dc_array_new(context, 512i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut curr_id: uint32_t = 0;
    let mut curr_local_timestamp: time_t = 0;
    let mut curr_day: libc::c_int = 0;
    let mut last_day: libc::c_int = 0i32;
    let mut cnv_to_local: libc::c_long = dc_gm2local_offset();
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || ret.is_null()) {
        if chat_id == 1i32 as libc::c_uint {
            let mut show_emails: libc::c_int = dc_sqlite3_get_config_int(
                (*context).sql,
                b"show_emails\x00" as *const u8 as *const libc::c_char,
                0i32,
            );
            stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN chats ON m.chat_id=chats.id LEFT JOIN contacts ON m.from_id=contacts.id WHERE m.from_id!=1   AND m.from_id!=2   AND m.hidden=0    AND chats.blocked=2   AND contacts.blocked=0   AND m.msgrmsg>=?  ORDER BY m.timestamp,m.id;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, if show_emails == 2i32 { 0i32 } else { 1i32 });
        } else if chat_id == 5i32 as libc::c_uint {
            stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.starred=1    AND m.hidden=0    AND ct.blocked=0 ORDER BY m.timestamp,m.id;\x00"
                                       as *const u8 as *const libc::c_char)
        } else {
            stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT m.id, m.timestamp FROM msgs m WHERE m.chat_id=?    AND m.hidden=0  ORDER BY m.timestamp,m.id;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        }
        while sqlite3_step(stmt) == 100i32 {
            curr_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
            if curr_id == marker1before {
                dc_array_add_id(ret, 1i32 as uint32_t);
            }
            if 0 != flags & 0x1i32 as libc::c_uint {
                curr_local_timestamp = sqlite3_column_int64(stmt, 1i32) as time_t + cnv_to_local;
                curr_day = (curr_local_timestamp / 86400i32 as libc::c_long) as libc::c_int;
                if curr_day != last_day {
                    dc_array_add_id(ret, 9i32 as uint32_t);
                    last_day = curr_day
                }
            }
            dc_array_add_id(ret, curr_id);
        }
        success = 1i32
    }
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
pub unsafe extern "C" fn dc_get_msg_cnt(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT COUNT(*) FROM msgs WHERE chat_id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32)
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_fresh_msg_cnt(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT COUNT(*) FROM msgs  WHERE state=10   AND hidden=0    AND chat_id=?;\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32)
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) {
    let mut check: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut update: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        check = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id FROM msgs  WHERE chat_id=? AND state=10;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(check, 1i32, chat_id as libc::c_int);
        if !(sqlite3_step(check) != 100i32) {
            update = dc_sqlite3_prepare(
                (*context).sql,
                b"UPDATE msgs    SET state=13 WHERE chat_id=? AND state=10;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_int(update, 1i32, chat_id as libc::c_int);
            sqlite3_step(update);
            (*context).cb.expect("non-null function pointer")(
                context,
                2000i32,
                0i32 as uintptr_t,
                0i32 as uintptr_t,
            );
        }
    }
    sqlite3_finalize(check);
    sqlite3_finalize(update);
}
#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_all_chats(mut context: *mut dc_context_t) {
    let mut check: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut update: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        check = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id FROM msgs  WHERE state=10;\x00" as *const u8 as *const libc::c_char,
        );
        if !(sqlite3_step(check) != 100i32) {
            update = dc_sqlite3_prepare(
                (*context).sql,
                b"UPDATE msgs    SET state=13 WHERE state=10;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_step(update);
            (*context).cb.expect("non-null function pointer")(
                context,
                2000i32,
                0i32 as uintptr_t,
                0i32 as uintptr_t,
            );
        }
    }
    sqlite3_finalize(check);
    sqlite3_finalize(update);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_media(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut msg_type: libc::c_int,
    mut msg_type2: libc::c_int,
    mut msg_type3: libc::c_int,
) -> *mut dc_array_t {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0 as *mut dc_array_t;
    }
    let mut ret: *mut dc_array_t = dc_array_new(context, 100i32 as size_t);
    let mut stmt: *mut sqlite3_stmt =
        dc_sqlite3_prepare((*context).sql,
                           b"SELECT id FROM msgs WHERE chat_id=? AND (type=? OR type=? OR type=?) ORDER BY timestamp, id;\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, msg_type);
    sqlite3_bind_int(
        stmt,
        3i32,
        if msg_type2 > 0i32 {
            msg_type2
        } else {
            msg_type
        },
    );
    sqlite3_bind_int(
        stmt,
        4i32,
        if msg_type3 > 0i32 {
            msg_type3
        } else {
            msg_type
        },
    );
    while sqlite3_step(stmt) == 100i32 {
        dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_next_media(
    mut context: *mut dc_context_t,
    mut curr_msg_id: uint32_t,
    mut dir: libc::c_int,
    mut msg_type: libc::c_int,
    mut msg_type2: libc::c_int,
    mut msg_type3: libc::c_int,
) -> uint32_t {
    let mut ret_msg_id: uint32_t = 0i32 as uint32_t;
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut list: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut i: libc::c_int = 0i32;
    let mut cnt: libc::c_int = 0i32;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(0 == dc_msg_load_from_db(msg, context, curr_msg_id)) {
            list = dc_get_chat_media(
                context,
                (*msg).chat_id,
                if msg_type > 0i32 {
                    msg_type
                } else {
                    (*msg).type_0
                },
                msg_type2,
                msg_type3,
            );
            if !list.is_null() {
                cnt = dc_array_get_cnt(list) as libc::c_int;
                i = 0i32;
                while i < cnt {
                    if curr_msg_id == dc_array_get_id(list, i as size_t) {
                        if dir > 0i32 {
                            if i + 1i32 < cnt {
                                ret_msg_id = dc_array_get_id(list, (i + 1i32) as size_t)
                            }
                        } else if dir < 0i32 {
                            if i - 1i32 >= 0i32 {
                                ret_msg_id = dc_array_get_id(list, (i - 1i32) as size_t)
                            }
                        }
                        break;
                    } else {
                        i += 1
                    }
                }
            }
        }
    }
    dc_array_unref(list);
    dc_msg_unref(msg);
    return ret_msg_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_archive_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut archive: libc::c_int,
) {
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint
        || archive != 0i32 && archive != 1i32
    {
        return;
    }
    if 0 != archive {
        let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"UPDATE msgs SET state=13 WHERE chat_id=? AND state=10;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    }
    let mut stmt_0: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"UPDATE chats SET archived=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt_0, 1i32, archive);
    sqlite3_bind_int(stmt_0, 2i32, chat_id as libc::c_int);
    sqlite3_step(stmt_0);
    sqlite3_finalize(stmt_0);
    (*context).cb.expect("non-null function pointer")(
        context,
        2000i32,
        0i32 as uintptr_t,
        0i32 as uintptr_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_delete_chat(mut context: *mut dc_context_t, mut chat_id: uint32_t) {
    /* Up to 2017-11-02 deleting a group also implied leaving it, see above why we have changed this. */
    let mut pending_transaction: libc::c_int = 0i32;
    let mut obj: *mut dc_chat_t = dc_chat_new(context);
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint)
    {
        if !(0 == dc_chat_load_from_db(obj, chat_id)) {
            dc_sqlite3_begin_transaction((*context).sql);
            pending_transaction = 1i32;
            q3 = sqlite3_mprintf(
                b"DELETE FROM msgs_mdns WHERE msg_id IN (SELECT id FROM msgs WHERE chat_id=%i);\x00"
                    as *const u8 as *const libc::c_char,
                chat_id,
            );
            if !(0 == dc_sqlite3_execute((*context).sql, q3)) {
                sqlite3_free(q3 as *mut libc::c_void);
                q3 = 0 as *mut libc::c_char;
                q3 = sqlite3_mprintf(
                    b"DELETE FROM msgs WHERE chat_id=%i;\x00" as *const u8 as *const libc::c_char,
                    chat_id,
                );
                if !(0 == dc_sqlite3_execute((*context).sql, q3)) {
                    sqlite3_free(q3 as *mut libc::c_void);
                    q3 = 0 as *mut libc::c_char;
                    q3 = sqlite3_mprintf(
                        b"DELETE FROM chats_contacts WHERE chat_id=%i;\x00" as *const u8
                            as *const libc::c_char,
                        chat_id,
                    );
                    if !(0 == dc_sqlite3_execute((*context).sql, q3)) {
                        sqlite3_free(q3 as *mut libc::c_void);
                        q3 = 0 as *mut libc::c_char;
                        q3 = sqlite3_mprintf(
                            b"DELETE FROM chats WHERE id=%i;\x00" as *const u8
                                as *const libc::c_char,
                            chat_id,
                        );
                        if !(0 == dc_sqlite3_execute((*context).sql, q3)) {
                            sqlite3_free(q3 as *mut libc::c_void);
                            q3 = 0 as *mut libc::c_char;
                            dc_sqlite3_commit((*context).sql);
                            pending_transaction = 0i32;
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2000i32,
                                0i32 as uintptr_t,
                                0i32 as uintptr_t,
                            );
                            dc_job_kill_action(context, 105i32);
                            dc_job_add(context, 105i32, 0i32, 0 as *const libc::c_char, 10i32);
                        }
                    }
                }
            }
        }
    }
    if 0 != pending_transaction {
        dc_sqlite3_rollback((*context).sql);
    }
    dc_chat_unref(obj);
    sqlite3_free(q3 as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_contacts(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> *mut dc_array_t {
    /* Normal chats do not include SELF.  Group chats do (as it may happen that one is deleted from a
    groupchat but the chats stays visible, moreover, this makes displaying lists easier) */
    let mut ret: *mut dc_array_t = dc_array_new(context, 100i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(chat_id == 1i32 as libc::c_uint) {
            /* we could also create a list for all contacts in the deaddrop by searching contacts belonging to chats with chats.blocked=2, however, currently this is not needed */
            stmt =
                dc_sqlite3_prepare((*context).sql,
                                   b"SELECT cc.contact_id FROM chats_contacts cc LEFT JOIN contacts c ON c.id=cc.contact_id WHERE cc.chat_id=? ORDER BY c.id=1, LOWER(c.name||c.addr), c.id;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
            while sqlite3_step(stmt) == 100i32 {
                dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
            }
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> *mut dc_chat_t {
    let mut success: libc::c_int = 0i32;
    let mut obj: *mut dc_chat_t = dc_chat_new(context);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(0 == dc_chat_load_from_db(obj, chat_id)) {
            success = 1i32
        }
    }
    if 0 != success {
        return obj;
    } else {
        dc_chat_unref(obj);
        return 0 as *mut dc_chat_t;
    };
}
// handle group chats
#[no_mangle]
pub unsafe extern "C" fn dc_create_group_chat(
    mut context: *mut dc_context_t,
    mut verified: libc::c_int,
    mut chat_name: *const libc::c_char,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut draft_txt: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut draft_msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_name.is_null()
        || *chat_name.offset(0isize) as libc::c_int == 0i32
    {
        return 0i32 as uint32_t;
    }
    draft_txt = dc_stock_str_repl_string(context, 14i32, chat_name);
    grpid = dc_create_id();
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"INSERT INTO chats (type, name, grpid, param) VALUES(?, ?, ?, \'U=1\');\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, if 0 != verified { 130i32 } else { 120i32 });
    sqlite3_bind_text(stmt, 2i32, chat_name, -1i32, None);
    sqlite3_bind_text(stmt, 3i32, grpid, -1i32, None);
    if !(sqlite3_step(stmt) != 101i32) {
        chat_id = dc_sqlite3_get_rowid(
            (*context).sql,
            b"chats\x00" as *const u8 as *const libc::c_char,
            b"grpid\x00" as *const u8 as *const libc::c_char,
            grpid,
        );
        if !(chat_id == 0i32 as libc::c_uint) {
            if !(0 == dc_add_to_chat_contacts_table(context, chat_id, 1i32 as uint32_t)) {
                draft_msg = dc_msg_new(context, 10i32);
                dc_msg_set_text(draft_msg, draft_txt);
                set_draft_raw(context, chat_id, draft_msg);
            }
        }
    }
    sqlite3_finalize(stmt);
    free(draft_txt as *mut libc::c_void);
    dc_msg_unref(draft_msg);
    free(grpid as *mut libc::c_void);
    if 0 != chat_id {
        (*context).cb.expect("non-null function pointer")(
            context,
            2000i32,
            0i32 as uintptr_t,
            0i32 as uintptr_t,
        );
    }
    return chat_id;
}
/* you MUST NOT modify this or the following strings */
// Context functions to work with chats
#[no_mangle]
pub unsafe extern "C" fn dc_add_to_chat_contacts_table(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    /* add a contact to a chat; the function does not check the type or if any of the record exist or are already added to the chat! */
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"INSERT INTO chats_contacts (chat_id, contact_id) VALUES(?, ?)\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
    ret = if sqlite3_step(stmt) == 101i32 {
        1i32
    } else {
        0i32
    };
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_add_contact_to_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    return dc_add_contact_to_chat_ex(context, chat_id, contact_id, 0i32);
}
#[no_mangle]
pub unsafe extern "C" fn dc_add_contact_to_chat_ex(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut contact_id: uint32_t,
    mut flags: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let mut chat: *mut dc_chat_t = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || contact.is_null()
        || chat_id <= 9i32 as libc::c_uint)
    {
        dc_reset_gossiped_timestamp(context, chat_id);
        /*this also makes sure, not contacts are added to special or normal chats*/
        if !(0i32 == real_group_exists(context, chat_id)
            || 0i32 == dc_real_contact_exists(context, contact_id)
                && contact_id != 1i32 as libc::c_uint
            || 0i32 == dc_chat_load_from_db(chat, chat_id))
        {
            if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    410i32,
                    0i32,
                    b"Cannot add contact to group; self not in group.\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                if 0 != flags & 0x1i32 && dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 1i32
                {
                    dc_param_set((*chat).param, 'U' as i32, 0 as *const libc::c_char);
                    dc_chat_update_param(chat);
                }
                self_addr = dc_sqlite3_get_config(
                    (*context).sql,
                    b"configured_addr\x00" as *const u8 as *const libc::c_char,
                    b"\x00" as *const u8 as *const libc::c_char,
                );
                if !(strcasecmp((*contact).addr, self_addr) == 0i32) {
                    /* ourself is added using DC_CONTACT_ID_SELF, do not add it explicitly. if SELF is not in the group, members cannot be added at all. */
                    if 0 != dc_is_contact_in_chat(context, chat_id, contact_id) {
                        if 0 == flags & 0x1i32 {
                            success = 1i32;
                            current_block = 12326129973959287090;
                        } else {
                            current_block = 15125582407903384992;
                        }
                    } else {
                        // else continue and send status mail
                        if (*chat).type_0 == 130i32 {
                            if dc_contact_is_verified(contact) != 2i32 {
                                dc_log_error(context, 0i32,
                                             b"Only bidirectional verified contacts can be added to verified groups.\x00"
                                                 as *const u8 as
                                                 *const libc::c_char);
                                current_block = 12326129973959287090;
                            } else {
                                current_block = 13472856163611868459;
                            }
                        } else {
                            current_block = 13472856163611868459;
                        }
                        match current_block {
                            12326129973959287090 => {}
                            _ => {
                                if 0i32
                                    == dc_add_to_chat_contacts_table(context, chat_id, contact_id)
                                {
                                    current_block = 12326129973959287090;
                                } else {
                                    current_block = 15125582407903384992;
                                }
                            }
                        }
                    }
                    match current_block {
                        12326129973959287090 => {}
                        _ => {
                            if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                                (*msg).type_0 = 10i32;
                                (*msg).text = dc_stock_system_msg(
                                    context,
                                    17i32,
                                    (*contact).addr,
                                    0 as *const libc::c_char,
                                    1i32 as uint32_t,
                                );
                                dc_param_set_int((*msg).param, 'S' as i32, 4i32);
                                dc_param_set((*msg).param, 'E' as i32, (*contact).addr);
                                dc_param_set_int((*msg).param, 'F' as i32, flags);
                                (*msg).id = dc_send_msg(context, chat_id, msg);
                                (*context).cb.expect("non-null function pointer")(
                                    context,
                                    2000i32,
                                    chat_id as uintptr_t,
                                    (*msg).id as uintptr_t,
                                );
                            }
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2020i32,
                                chat_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                            success = 1i32
                        }
                    }
                }
            }
        }
    }
    dc_chat_unref(chat);
    dc_contact_unref(contact);
    dc_msg_unref(msg);
    free(self_addr as *mut libc::c_void);
    return success;
}
unsafe extern "C" fn real_group_exists(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> libc::c_int {
    // check if a group or a verified group exists under the given ID
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut ret: libc::c_int = 0i32;
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null()
        || chat_id <= 9i32 as libc::c_uint
    {
        return 0i32;
    }
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT id FROM chats  WHERE id=?    AND (type=120 OR type=130);\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        ret = 1i32
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_reset_gossiped_timestamp(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) {
    dc_set_gossiped_timestamp(context, chat_id, 0i32 as time_t);
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_gossiped_timestamp(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut timestamp: time_t,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if 0 != chat_id {
        dc_log_info(
            context,
            0i32,
            b"set gossiped_timestamp for chat #%i to %i.\x00" as *const u8 as *const libc::c_char,
            chat_id as libc::c_int,
            timestamp as libc::c_int,
        );
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"UPDATE chats SET gossiped_timestamp=? WHERE id=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
        sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    } else {
        dc_log_info(
            context,
            0i32,
            b"set gossiped_timestamp for all chats to %i.\x00" as *const u8 as *const libc::c_char,
            timestamp as libc::c_int,
        );
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"UPDATE chats SET gossiped_timestamp=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
    }
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
#[no_mangle]
pub unsafe extern "C" fn dc_remove_contact_from_chat(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let mut chat: *mut dc_chat_t = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint
        || contact_id <= 9i32 as libc::c_uint && contact_id != 1i32 as libc::c_uint)
    {
        /* we do not check if "contact_id" exists but just delete all records with the id from chats_contacts */
        /* this allows to delete pending references to deleted contacts.  Of course, this should _not_ happen. */
        if !(0i32 == real_group_exists(context, chat_id)
            || 0i32 == dc_chat_load_from_db(chat, chat_id))
        {
            if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    410i32,
                    0i32,
                    b"Cannot remove contact from chat; self not in group.\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                if !contact.is_null() {
                    if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                        (*msg).type_0 = 10i32;
                        if (*contact).id == 1i32 as libc::c_uint {
                            dc_set_group_explicitly_left(context, (*chat).grpid);
                            (*msg).text = dc_stock_system_msg(
                                context,
                                19i32,
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                                1i32 as uint32_t,
                            )
                        } else {
                            (*msg).text = dc_stock_system_msg(
                                context,
                                18i32,
                                (*contact).addr,
                                0 as *const libc::c_char,
                                1i32 as uint32_t,
                            )
                        }
                        dc_param_set_int((*msg).param, 'S' as i32, 5i32);
                        dc_param_set((*msg).param, 'E' as i32, (*contact).addr);
                        (*msg).id = dc_send_msg(context, chat_id, msg);
                        (*context).cb.expect("non-null function pointer")(
                            context,
                            2000i32,
                            chat_id as uintptr_t,
                            (*msg).id as uintptr_t,
                        );
                    }
                }
                q3 = sqlite3_mprintf(
                    b"DELETE FROM chats_contacts WHERE chat_id=%i AND contact_id=%i;\x00"
                        as *const u8 as *const libc::c_char,
                    chat_id,
                    contact_id,
                );
                if !(0 == dc_sqlite3_execute((*context).sql, q3)) {
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2020i32,
                        chat_id as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    success = 1i32
                }
            }
        }
    }
    sqlite3_free(q3 as *mut libc::c_void);
    dc_chat_unref(chat);
    dc_contact_unref(contact);
    dc_msg_unref(msg);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_group_explicitly_left(
    mut context: *mut dc_context_t,
    mut grpid: *const libc::c_char,
) {
    if 0 == dc_is_group_explicitly_left(context, grpid) {
        let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"INSERT INTO leftgrps (grpid) VALUES(?);\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, grpid, -1i32, None);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_group_explicitly_left(
    mut context: *mut dc_context_t,
    mut grpid: *const libc::c_char,
) -> libc::c_int {
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT id FROM leftgrps WHERE grpid=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, grpid, -1i32, None);
    let mut ret: libc::c_int = (sqlite3_step(stmt) == 100i32) as libc::c_int;
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_name(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut new_name: *const libc::c_char,
) -> libc::c_int {
    /* the function only sets the names of group chats; normal chats get their names from the contacts */
    let mut success: libc::c_int = 0i32;
    let mut chat: *mut dc_chat_t = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || new_name.is_null()
        || *new_name.offset(0isize) as libc::c_int == 0i32
        || chat_id <= 9i32 as libc::c_uint)
    {
        if !(0i32 == real_group_exists(context, chat_id)
            || 0i32 == dc_chat_load_from_db(chat, chat_id))
        {
            if strcmp((*chat).name, new_name) == 0i32 {
                success = 1i32
            } else if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    410i32,
                    0i32,
                    b"Cannot set chat name; self not in group\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                q3 = sqlite3_mprintf(
                    b"UPDATE chats SET name=%Q WHERE id=%i;\x00" as *const u8
                        as *const libc::c_char,
                    new_name,
                    chat_id,
                );
                if !(0 == dc_sqlite3_execute((*context).sql, q3)) {
                    if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                        (*msg).type_0 = 10i32;
                        (*msg).text = dc_stock_system_msg(
                            context,
                            15i32,
                            (*chat).name,
                            new_name,
                            1i32 as uint32_t,
                        );
                        dc_param_set_int((*msg).param, 'S' as i32, 2i32);
                        dc_param_set((*msg).param, 'E' as i32, (*chat).name);
                        (*msg).id = dc_send_msg(context, chat_id, msg);
                        (*context).cb.expect("non-null function pointer")(
                            context,
                            2000i32,
                            chat_id as uintptr_t,
                            (*msg).id as uintptr_t,
                        );
                    }
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2020i32,
                        chat_id as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    success = 1i32
                }
            }
        }
    }
    sqlite3_free(q3 as *mut libc::c_void);
    dc_chat_unref(chat);
    dc_msg_unref(msg);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_profile_image(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut new_image: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut chat: *mut dc_chat_t = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut new_image_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || chat_id <= 9i32 as libc::c_uint)
    {
        if !(0i32 == real_group_exists(context, chat_id)
            || 0i32 == dc_chat_load_from_db(chat, chat_id))
        {
            if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    410i32,
                    0i32,
                    b"Cannot set chat profile image; self not in group.\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                if !new_image.is_null() {
                    new_image_rel = dc_strdup(new_image);
                    if 0 == dc_make_rel_and_copy(context, &mut new_image_rel) {
                        current_block = 14766584022300871387;
                    } else {
                        current_block = 1856101646708284338;
                    }
                } else {
                    current_block = 1856101646708284338;
                }
                match current_block {
                    14766584022300871387 => {}
                    _ => {
                        dc_param_set((*chat).param, 'i' as i32, new_image_rel);
                        if !(0 == dc_chat_update_param(chat)) {
                            if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                                dc_param_set_int((*msg).param, 'S' as i32, 3i32);
                                dc_param_set((*msg).param, 'E' as i32, new_image_rel);
                                (*msg).type_0 = 10i32;
                                (*msg).text = dc_stock_system_msg(
                                    context,
                                    if !new_image_rel.is_null() {
                                        16i32
                                    } else {
                                        33i32
                                    },
                                    0 as *const libc::c_char,
                                    0 as *const libc::c_char,
                                    1i32 as uint32_t,
                                );
                                (*msg).id = dc_send_msg(context, chat_id, msg);
                                (*context).cb.expect("non-null function pointer")(
                                    context,
                                    2000i32,
                                    chat_id as uintptr_t,
                                    (*msg).id as uintptr_t,
                                );
                            }
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2020i32,
                                chat_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                            success = 1i32
                        }
                    }
                }
            }
        }
    }
    dc_chat_unref(chat);
    dc_msg_unref(msg);
    free(new_image_rel as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_forward_msgs(
    mut context: *mut dc_context_t,
    mut msg_ids: *const uint32_t,
    mut msg_cnt: libc::c_int,
    mut chat_id: uint32_t,
) {
    let mut current_block: u64;
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut chat: *mut dc_chat_t = dc_chat_new(context);
    let mut contact: *mut dc_contact_t = dc_contact_new(context);
    let mut transaction_pending: libc::c_int = 0i32;
    let mut created_db_entries: *mut carray = carray_new(16i32 as libc::c_uint);
    let mut idsstr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut curr_timestamp: time_t = 0i32 as time_t;
    let mut original_param: *mut dc_param_t = dc_param_new();
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg_ids.is_null()
        || msg_cnt <= 0i32
        || chat_id <= 9i32 as libc::c_uint)
    {
        dc_sqlite3_begin_transaction((*context).sql);
        transaction_pending = 1i32;
        dc_unarchive_chat(context, chat_id);
        (*(*context).smtp).log_connect_errors = 1i32;
        if !(0 == dc_chat_load_from_db(chat, chat_id)) {
            curr_timestamp = dc_create_smeared_timestamps(context, msg_cnt);
            idsstr = dc_arr_to_string(msg_ids, msg_cnt);
            q3 = sqlite3_mprintf(
                b"SELECT id FROM msgs WHERE id IN(%s) ORDER BY timestamp,id\x00" as *const u8
                    as *const libc::c_char,
                idsstr,
            );
            stmt = dc_sqlite3_prepare((*context).sql, q3);
            loop {
                if !(sqlite3_step(stmt) == 100i32) {
                    current_block = 10758786907990354186;
                    break;
                }
                let mut src_msg_id: libc::c_int = sqlite3_column_int(stmt, 0i32);
                if 0 == dc_msg_load_from_db(msg, context, src_msg_id as uint32_t) {
                    current_block = 2015322633586469911;
                    break;
                }
                dc_param_set_packed(original_param, (*(*msg).param).packed);
                if (*msg).from_id != 1i32 as libc::c_uint {
                    dc_param_set_int((*msg).param, 'a' as i32, 1i32);
                }
                dc_param_set((*msg).param, 'c' as i32, 0 as *const libc::c_char);
                dc_param_set((*msg).param, 'u' as i32, 0 as *const libc::c_char);
                dc_param_set((*msg).param, 'S' as i32, 0 as *const libc::c_char);
                let mut new_msg_id: uint32_t = 0;
                if (*msg).state == 18i32 {
                    let fresh9 = curr_timestamp;
                    curr_timestamp = curr_timestamp + 1;
                    new_msg_id = prepare_msg_raw(context, chat, msg, fresh9);
                    let mut save_param: *mut dc_param_t = (*msg).param;
                    (*msg).param = original_param;
                    (*msg).id = src_msg_id as uint32_t;
                    let mut old_fwd: *mut libc::c_char = dc_param_get(
                        (*msg).param,
                        'P' as i32,
                        b"\x00" as *const u8 as *const libc::c_char,
                    );
                    let mut new_fwd: *mut libc::c_char = dc_mprintf(
                        b"%s %d\x00" as *const u8 as *const libc::c_char,
                        old_fwd,
                        new_msg_id,
                    );
                    dc_param_set((*msg).param, 'P' as i32, new_fwd);
                    dc_msg_save_param_to_disk(msg);
                    free(new_fwd as *mut libc::c_void);
                    free(old_fwd as *mut libc::c_void);
                    (*msg).param = save_param
                } else {
                    (*msg).state = 20i32;
                    let fresh10 = curr_timestamp;
                    curr_timestamp = curr_timestamp + 1;
                    new_msg_id = prepare_msg_raw(context, chat, msg, fresh10);
                    dc_job_send_msg(context, new_msg_id);
                }
                carray_add(
                    created_db_entries,
                    chat_id as uintptr_t as *mut libc::c_void,
                    0 as *mut libc::c_uint,
                );
                carray_add(
                    created_db_entries,
                    new_msg_id as uintptr_t as *mut libc::c_void,
                    0 as *mut libc::c_uint,
                );
            }
            match current_block {
                2015322633586469911 => {}
                _ => {
                    dc_sqlite3_commit((*context).sql);
                    transaction_pending = 0i32
                }
            }
        }
    }
    if 0 != transaction_pending {
        dc_sqlite3_rollback((*context).sql);
    }
    if !created_db_entries.is_null() {
        let mut i: size_t = 0;
        let mut icnt: size_t = carray_count(created_db_entries) as size_t;
        i = 0i32 as size_t;
        while i < icnt {
            (*context).cb.expect("non-null function pointer")(
                context,
                2000i32,
                carray_get(created_db_entries, i as libc::c_uint) as uintptr_t,
                carray_get(
                    created_db_entries,
                    i.wrapping_add(1i32 as libc::c_ulong) as libc::c_uint,
                ) as uintptr_t,
            );
            i = (i as libc::c_ulong).wrapping_add(2i32 as libc::c_ulong) as size_t as size_t
        }
        carray_free(created_db_entries);
    }
    dc_contact_unref(contact);
    dc_msg_unref(msg);
    dc_chat_unref(chat);
    sqlite3_finalize(stmt);
    free(idsstr as *mut libc::c_void);
    sqlite3_free(q3 as *mut libc::c_void);
    dc_param_unref(original_param);
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_id(mut chat: *const dc_chat_t) -> uint32_t {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32 as uint32_t;
    }
    return (*chat).id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_type(mut chat: *const dc_chat_t) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    return (*chat).type_0;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_name(mut chat: *const dc_chat_t) -> *mut libc::c_char {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char);
    }
    return dc_strdup((*chat).name);
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_subtitle(mut chat: *const dc_chat_t) -> *mut libc::c_char {
    /* returns either the address or the number of chat members */
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char);
    }
    if (*chat).type_0 == 100i32 && 0 != dc_param_exists((*chat).param, 'K' as i32) {
        ret = dc_stock_str((*chat).context, 50i32)
    } else if (*chat).type_0 == 100i32 {
        let mut r: libc::c_int = 0;
        let mut stmt: *mut sqlite3_stmt =
            dc_sqlite3_prepare((*(*chat).context).sql,
                               b"SELECT c.addr FROM chats_contacts cc  LEFT JOIN contacts c ON c.id=cc.contact_id  WHERE cc.chat_id=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
        r = sqlite3_step(stmt);
        if r == 100i32 {
            ret = dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char)
        }
        sqlite3_finalize(stmt);
    } else if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
        let mut cnt: libc::c_int = 0i32;
        if (*chat).id == 1i32 as libc::c_uint {
            ret = dc_stock_str((*chat).context, 8i32)
        } else {
            cnt = dc_get_chat_contact_cnt((*chat).context, (*chat).id);
            ret = dc_stock_str_repl_int((*chat).context, 4i32, cnt)
        }
    }
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_contact_cnt(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT COUNT(*) FROM chats_contacts WHERE chat_id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        ret = sqlite3_column_int(stmt, 0i32)
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_profile_image(
    mut chat: *const dc_chat_t,
) -> *mut libc::c_char {
    let mut image_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut image_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contacts: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        image_rel = dc_param_get((*chat).param, 'i' as i32, 0 as *const libc::c_char);
        if !image_rel.is_null() && 0 != *image_rel.offset(0isize) as libc::c_int {
            image_abs = dc_get_abs_path((*chat).context, image_rel)
        } else if (*chat).type_0 == 100i32 {
            contacts = dc_get_chat_contacts((*chat).context, (*chat).id);
            if (*contacts).count >= 1i32 as libc::c_ulong {
                contact = dc_get_contact(
                    (*chat).context,
                    *(*contacts).array.offset(0isize) as uint32_t,
                );
                image_abs = dc_contact_get_profile_image(contact)
            }
        }
    }
    free(image_rel as *mut libc::c_void);
    dc_array_unref(contacts);
    dc_contact_unref(contact);
    return image_abs;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_color(mut chat: *const dc_chat_t) -> uint32_t {
    let mut color: uint32_t = 0i32 as uint32_t;
    let mut contacts: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        if (*chat).type_0 == 100i32 {
            contacts = dc_get_chat_contacts((*chat).context, (*chat).id);
            if (*contacts).count >= 1i32 as libc::c_ulong {
                contact = dc_get_contact(
                    (*chat).context,
                    *(*contacts).array.offset(0isize) as uint32_t,
                );
                color = dc_str_to_color((*contact).addr) as uint32_t
            }
        } else {
            color = dc_str_to_color((*chat).name) as uint32_t
        }
    }
    dc_array_unref(contacts);
    dc_contact_unref(contact);
    return color;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_archived(mut chat: *const dc_chat_t) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    return (*chat).archived;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_unpromoted(mut chat: *const dc_chat_t) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    return dc_param_get_int((*chat).param, 'U' as i32, 0i32);
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_verified(mut chat: *const dc_chat_t) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    return ((*chat).type_0 == 130i32) as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_sending_locations(mut chat: *const dc_chat_t) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    return (*chat).is_sending_locations;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_cnt(mut context: *mut dc_context_t) -> size_t {
    let mut ret: size_t = 0i32 as size_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*(*context).sql).cobj.is_null())
    {
        /* no database, no chats - this is no error (needed eg. for information) */
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT COUNT(*) FROM chats WHERE id>9 AND blocked=0;\x00" as *const u8
                as *const libc::c_char,
        );
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32) as size_t
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_id_by_grpid(
    mut context: *mut dc_context_t,
    mut grpid: *const libc::c_char,
    mut ret_blocked: *mut libc::c_int,
    mut ret_verified: *mut libc::c_int,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !ret_blocked.is_null() {
        *ret_blocked = 0i32
    }
    if !ret_verified.is_null() {
        *ret_verified = 0i32
    }
    if !(context.is_null() || grpid.is_null()) {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id, blocked, type FROM chats WHERE grpid=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, grpid, -1i32, None);
        if sqlite3_step(stmt) == 100i32 {
            chat_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
            if !ret_blocked.is_null() {
                *ret_blocked = sqlite3_column_int(stmt, 1i32)
            }
            if !ret_verified.is_null() {
                *ret_verified = (sqlite3_column_int(stmt, 2i32) == 130i32) as libc::c_int
            }
        }
    }
    sqlite3_finalize(stmt);
    return chat_id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_add_device_msg(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut text: *const libc::c_char,
) {
    let mut msg_id: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut rfc724_mid: *mut libc::c_char = dc_create_outgoing_rfc724_mid(
        0 as *const libc::c_char,
        b"@device\x00" as *const u8 as *const libc::c_char,
    );
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || text.is_null()) {
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"INSERT INTO msgs (chat_id,from_id,to_id, timestamp,type,state, txt,rfc724_mid) VALUES (?,?,?, ?,?,?, ?,?);\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        sqlite3_bind_int(stmt, 2i32, 2i32);
        sqlite3_bind_int(stmt, 3i32, 2i32);
        sqlite3_bind_int64(
            stmt,
            4i32,
            dc_create_smeared_timestamp(context) as sqlite3_int64,
        );
        sqlite3_bind_int(stmt, 5i32, 10i32);
        sqlite3_bind_int(stmt, 6i32, 13i32);
        sqlite3_bind_text(stmt, 7i32, text, -1i32, None);
        sqlite3_bind_text(stmt, 8i32, rfc724_mid, -1i32, None);
        if !(sqlite3_step(stmt) != 101i32) {
            msg_id = dc_sqlite3_get_rowid(
                (*context).sql,
                b"msgs\x00" as *const u8 as *const libc::c_char,
                b"rfc724_mid\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            (*context).cb.expect("non-null function pointer")(
                context,
                2000i32,
                chat_id as uintptr_t,
                msg_id as uintptr_t,
            );
        }
    }
    free(rfc724_mid as *mut libc::c_void);
    sqlite3_finalize(stmt);
}

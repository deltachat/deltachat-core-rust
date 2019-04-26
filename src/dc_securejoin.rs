use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type sqlite3_stmt;
    #[no_mangle]
    fn usleep(_: useconds_t) -> libc::c_int;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_lock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_unlock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn dc_create_chat_by_contact_id(_: *mut dc_context_t, contact_id: uint32_t) -> uint32_t;
    #[no_mangle]
    fn dc_send_msg(_: *mut dc_context_t, chat_id: uint32_t, _: *mut dc_msg_t) -> uint32_t;
    #[no_mangle]
    fn dc_get_chat_contacts(_: *mut dc_context_t, chat_id: uint32_t) -> *mut dc_array_t;
    #[no_mangle]
    fn dc_get_chat(_: *mut dc_context_t, chat_id: uint32_t) -> *mut dc_chat_t;
    #[no_mangle]
    fn dc_get_contact(_: *mut dc_context_t, contact_id: uint32_t) -> *mut dc_contact_t;
    #[no_mangle]
    fn dc_stop_ongoing_process(_: *mut dc_context_t);
    // out-of-band verification
    // id=contact
    // text1=groupname
    // id=contact
    // id=contact
    // test1=formatted fingerprint
    // id=contact
    // text1=text
    // text1=URL
    // text1=error string
    #[no_mangle]
    fn dc_check_qr(_: *mut dc_context_t, qr: *const libc::c_char) -> *mut dc_lot_t;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_chat_unref(_: *mut dc_chat_t);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_urlencode(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_chat_get_name(_: *const dc_chat_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_key_new() -> *mut dc_key_t;
    #[no_mangle]
    fn dc_key_unref(_: *mut dc_key_t);
    #[no_mangle]
    fn dc_key_get_fingerprint(_: *const dc_key_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_key_load_self_public(
        _: *mut dc_key_t,
        self_addr: *const libc::c_char,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_get_config(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    // Functions to read/write token from/to the database. A token is any string associated with a key.
    #[no_mangle]
    fn dc_token_save(
        _: *mut dc_context_t,
        _: dc_tokennamespc_t,
        foreign_id: uint32_t,
        token: *const libc::c_char,
    );
    /* Message-ID tools */
    #[no_mangle]
    fn dc_create_id() -> *mut libc::c_char;
    #[no_mangle]
    fn dc_token_lookup(
        _: *mut dc_context_t,
        _: dc_tokennamespc_t,
        foreign_id: uint32_t,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_ensure_secret_key_exists(_: *mut dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_free_ongoing(_: *mut dc_context_t);
    #[no_mangle]
    fn dc_lot_unref(_: *mut dc_lot_t);
    #[no_mangle]
    fn dc_get_chat_id_by_grpid(
        _: *mut dc_context_t,
        grpid: *const libc::c_char,
        ret_blocked: *mut libc::c_int,
        ret_verified: *mut libc::c_int,
    ) -> uint32_t;
    #[no_mangle]
    fn dc_msg_new_untyped(_: *mut dc_context_t) -> *mut dc_msg_t;
    #[no_mangle]
    fn dc_msg_unref(_: *mut dc_msg_t);
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
    #[no_mangle]
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
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
    fn dc_array_get_id(_: *const dc_array_t, index: size_t) -> uint32_t;
    #[no_mangle]
    fn dc_array_get_cnt(_: *const dc_array_t) -> size_t;
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
    #[no_mangle]
    fn dc_apeerstate_new(_: *mut dc_context_t) -> *mut dc_apeerstate_t;
    #[no_mangle]
    fn dc_normalize_fingerprint(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_apeerstate_load_by_addr(
        _: *mut dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        addr: *const libc::c_char,
    ) -> libc::c_int;
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
    fn dc_alloc_ongoing(_: *mut dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_contact_is_verified(_: *mut dc_contact_t) -> libc::c_int;
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
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    /* Replaces the first `%1$s` in the given String-ID by the given value.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str_repl_string(
        _: *mut dc_context_t,
        id: libc::c_int,
        value: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_create_or_lookup_nchat_by_contact_id(
        _: *mut dc_context_t,
        contact_id: uint32_t,
        create_blocked: libc::c_int,
        ret_chat_id: *mut uint32_t,
        ret_chat_blocked: *mut libc::c_int,
    );
    #[no_mangle]
    fn dc_unblock_chat(_: *mut dc_context_t, chat_id: uint32_t);
    #[no_mangle]
    fn dc_add_device_msg(_: *mut dc_context_t, chat_id: uint32_t, text: *const libc::c_char);
    #[no_mangle]
    fn dc_add_contact_to_chat_ex(
        _: *mut dc_context_t,
        chat_id: uint32_t,
        contact_id: uint32_t,
        flags: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_hash_find(
        _: *const dc_hash_t,
        pKey: *const libc::c_void,
        nKey: libc::c_int,
    ) -> *mut libc::c_void;
    #[no_mangle]
    fn dc_apeerstate_unref(_: *mut dc_apeerstate_t);
    #[no_mangle]
    fn dc_apeerstate_set_verified(
        _: *mut dc_apeerstate_t,
        which_key: libc::c_int,
        fingerprint: *const libc::c_char,
        verified: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_load_by_fingerprint(
        _: *mut dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        fingerprint: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_save_to_db(
        _: *const dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        create: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_addr_equals_self(_: *mut dc_context_t, addr: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_scaleup_contact_origin(_: *mut dc_context_t, contact_id: uint32_t, origin: libc::c_int);
    /* the following functions can be used only after a call to dc_mimeparser_parse() */
    #[no_mangle]
    fn dc_mimeparser_lookup_field(
        _: *mut dc_mimeparser_t,
        field_name: *const libc::c_char,
    ) -> *mut mailimf_field;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_token_exists(
        _: *mut dc_context_t,
        _: dc_tokennamespc_t,
        token: *const libc::c_char,
    ) -> libc::c_int;
}
pub type __uint32_t = libc::c_uint;
pub type __darwin_size_t = libc::c_ulong;
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
pub struct _opaque_pthread_mutex_t {
    pub __sig: libc::c_long,
    pub __opaque: [libc::c_char; 56],
}
pub type __darwin_pthread_cond_t = _opaque_pthread_cond_t;
pub type __darwin_pthread_mutex_t = _opaque_pthread_mutex_t;
pub type size_t = __darwin_size_t;
pub type uint32_t = libc::c_uint;
pub type int32_t = libc::c_int;
pub type uintptr_t = libc::c_ulong;
pub type ssize_t = __darwin_ssize_t;
pub type useconds_t = __darwin_useconds_t;
pub type time_t = __darwin_time_t;
pub type uint8_t = libc::c_uchar;
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
pub type clistiter = clistcell;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_date_time {
    pub dt_day: libc::c_int,
    pub dt_month: libc::c_int,
    pub dt_year: libc::c_int,
    pub dt_hour: libc::c_int,
    pub dt_min: libc::c_int,
    pub dt_sec: libc::c_int,
    pub dt_zone: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_mailbox_list {
    pub mb_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_mailbox {
    pub mb_display_name: *mut libc::c_char,
    pub mb_addr_spec: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_address_list {
    pub ad_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_fields {
    pub fld_list: *mut clist,
}
pub type unnamed = libc::c_uint;
pub const MAILIMF_FIELD_OPTIONAL_FIELD: unnamed = 22;
pub const MAILIMF_FIELD_KEYWORDS: unnamed = 21;
pub const MAILIMF_FIELD_COMMENTS: unnamed = 20;
pub const MAILIMF_FIELD_SUBJECT: unnamed = 19;
pub const MAILIMF_FIELD_REFERENCES: unnamed = 18;
pub const MAILIMF_FIELD_IN_REPLY_TO: unnamed = 17;
pub const MAILIMF_FIELD_MESSAGE_ID: unnamed = 16;
pub const MAILIMF_FIELD_BCC: unnamed = 15;
pub const MAILIMF_FIELD_CC: unnamed = 14;
pub const MAILIMF_FIELD_TO: unnamed = 13;
pub const MAILIMF_FIELD_REPLY_TO: unnamed = 12;
pub const MAILIMF_FIELD_SENDER: unnamed = 11;
pub const MAILIMF_FIELD_FROM: unnamed = 10;
pub const MAILIMF_FIELD_ORIG_DATE: unnamed = 9;
pub const MAILIMF_FIELD_RESENT_MSG_ID: unnamed = 8;
pub const MAILIMF_FIELD_RESENT_BCC: unnamed = 7;
pub const MAILIMF_FIELD_RESENT_CC: unnamed = 6;
pub const MAILIMF_FIELD_RESENT_TO: unnamed = 5;
pub const MAILIMF_FIELD_RESENT_SENDER: unnamed = 4;
pub const MAILIMF_FIELD_RESENT_FROM: unnamed = 3;
pub const MAILIMF_FIELD_RESENT_DATE: unnamed = 2;
pub const MAILIMF_FIELD_RETURN_PATH: unnamed = 1;
pub const MAILIMF_FIELD_NONE: unnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_0 {
    pub fld_return_path: *mut mailimf_return,
    pub fld_resent_date: *mut mailimf_orig_date,
    pub fld_resent_from: *mut mailimf_from,
    pub fld_resent_sender: *mut mailimf_sender,
    pub fld_resent_to: *mut mailimf_to,
    pub fld_resent_cc: *mut mailimf_cc,
    pub fld_resent_bcc: *mut mailimf_bcc,
    pub fld_resent_msg_id: *mut mailimf_message_id,
    pub fld_orig_date: *mut mailimf_orig_date,
    pub fld_from: *mut mailimf_from,
    pub fld_sender: *mut mailimf_sender,
    pub fld_reply_to: *mut mailimf_reply_to,
    pub fld_to: *mut mailimf_to,
    pub fld_cc: *mut mailimf_cc,
    pub fld_bcc: *mut mailimf_bcc,
    pub fld_message_id: *mut mailimf_message_id,
    pub fld_in_reply_to: *mut mailimf_in_reply_to,
    pub fld_references: *mut mailimf_references,
    pub fld_subject: *mut mailimf_subject,
    pub fld_comments: *mut mailimf_comments,
    pub fld_keywords: *mut mailimf_keywords,
    pub fld_optional_field: *mut mailimf_optional_field,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_optional_field {
    pub fld_name: *mut libc::c_char,
    pub fld_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_keywords {
    pub kw_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_comments {
    pub cm_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_subject {
    pub sbj_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_references {
    pub mid_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_in_reply_to {
    pub mid_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_message_id {
    pub mid_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_bcc {
    pub bcc_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_cc {
    pub cc_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_to {
    pub to_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_reply_to {
    pub rt_addr_list: *mut mailimf_address_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_sender {
    pub snd_mb: *mut mailimf_mailbox,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_from {
    pub frm_mb_list: *mut mailimf_mailbox_list,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_orig_date {
    pub dt_date_time: *mut mailimf_date_time,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_return {
    pub ret_path: *mut mailimf_path,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_path {
    pub pt_addr_spec: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_composite_type {
    pub ct_type: libc::c_int,
    pub ct_token: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_content {
    pub ct_type: *mut mailmime_type,
    pub ct_subtype: *mut libc::c_char,
    pub ct_parameters: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_type {
    pub tp_type: libc::c_int,
    pub tp_data: unnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_1 {
    pub tp_discrete_type: *mut mailmime_discrete_type,
    pub tp_composite_type: *mut mailmime_composite_type,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_discrete_type {
    pub dt_type: libc::c_int,
    pub dt_extension: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_fields {
    pub fld_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_data {
    pub dt_type: libc::c_int,
    pub dt_encoding: libc::c_int,
    pub dt_encoded: libc::c_int,
    pub dt_data: unnamed_2,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_2 {
    pub dt_text: unnamed_3,
    pub dt_filename: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_3 {
    pub dt_data: *const libc::c_char,
    pub dt_length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime {
    pub mm_parent_type: libc::c_int,
    pub mm_parent: *mut mailmime,
    pub mm_multipart_pos: *mut clistiter,
    pub mm_type: libc::c_int,
    pub mm_mime_start: *const libc::c_char,
    pub mm_length: size_t,
    pub mm_mime_fields: *mut mailmime_fields,
    pub mm_content_type: *mut mailmime_content,
    pub mm_body: *mut mailmime_data,
    pub mm_data: unnamed_4,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_4 {
    pub mm_single: *mut mailmime_data,
    pub mm_multipart: unnamed_6,
    pub mm_message: unnamed_5,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_5 {
    pub mm_fields: *mut mailimf_fields,
    pub mm_msg_mime: *mut mailmime,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_6 {
    pub mm_preamble: *mut mailmime_data,
    pub mm_epilogue: *mut mailmime_data,
    pub mm_mp_list: *mut clist,
}
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
    pub smtp_sasl: unnamed_7,
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
pub struct unnamed_7 {
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
    pub sec_data: unnamed_8,
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
pub union unnamed_8 {
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
    pub ft_data: unnamed_9,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_9 {
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
    pub imap_sasl: unnamed_10,
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
pub struct unnamed_10 {
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
// Token namespaces
pub type dc_tokennamespc_t = libc::c_uint;
pub const DC_TOKEN_AUTH: dc_tokennamespc_t = 110;
pub const DC_TOKEN_INVITENUMBER: dc_tokennamespc_t = 100;
pub type dc_apeerstate_t = _dc_apeerstate;
/* prefer-encrypt states */
/* *
 * @class dc_apeerstate_t
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_apeerstate {
    pub context: *mut dc_context_t,
    pub addr: *mut libc::c_char,
    pub last_seen: time_t,
    pub last_seen_autocrypt: time_t,
    pub prefer_encrypt: libc::c_int,
    pub public_key: *mut dc_key_t,
    pub public_key_fingerprint: *mut libc::c_char,
    pub gossip_key: *mut dc_key_t,
    pub gossip_timestamp: time_t,
    pub gossip_key_fingerprint: *mut libc::c_char,
    pub verified_key: *mut dc_key_t,
    pub verified_key_fingerprint: *mut libc::c_char,
    pub to_save: libc::c_int,
    pub degrade_event: libc::c_int,
}
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
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
/* *
 * @class dc_mimeparser_t
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_mimeparser {
    pub parts: *mut carray,
    pub mimeroot: *mut mailmime,
    pub header: dc_hash_t,
    pub header_root: *mut mailimf_fields,
    pub header_protected: *mut mailimf_fields,
    pub subject: *mut libc::c_char,
    pub is_send_by_messenger: libc::c_int,
    pub decrypting_failed: libc::c_int,
    pub e2ee_helper: *mut _dc_e2ee_helper,
    pub blobdir: *const libc::c_char,
    pub is_forwarded: libc::c_int,
    pub context: *mut dc_context_t,
    pub reports: *mut carray,
    pub is_system_message: libc::c_int,
    pub kml: *mut _dc_kml,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_kml {
    pub addr: *mut libc::c_char,
    pub locations: *mut dc_array_t,
    pub tag: libc::c_int,
    pub curr: dc_location_t,
}
pub type dc_location_t = _dc_location;
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
/* library private: end-to-end-encryption */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_e2ee_helper {
    pub encryption_successfull: libc::c_int,
    pub cdata_to_free: *mut libc::c_void,
    pub encrypted: libc::c_int,
    pub signatures: *mut dc_hash_t,
    pub gossipped_addr: *mut dc_hash_t,
}
pub type dc_mimeparser_t = _dc_mimeparser;
#[no_mangle]
pub unsafe extern "C" fn dc_get_securejoin_qr(
    mut context: *mut dc_context_t,
    mut group_chat_id: uint32_t,
) -> *mut libc::c_char {
    let mut current_block: u64;
    /* =========================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    ========================================================= */
    let mut qr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_addr_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_name_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut invitenumber: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut auth: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut chat: *mut dc_chat_t = 0 as *mut dc_chat_t;
    let mut group_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut group_name_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        dc_ensure_secret_key_exists(context);
        invitenumber = dc_token_lookup(context, DC_TOKEN_INVITENUMBER, group_chat_id);
        if invitenumber.is_null() {
            invitenumber = dc_create_id();
            dc_token_save(context, DC_TOKEN_INVITENUMBER, group_chat_id, invitenumber);
        }
        auth = dc_token_lookup(context, DC_TOKEN_AUTH, group_chat_id);
        if auth.is_null() {
            auth = dc_create_id();
            dc_token_save(context, DC_TOKEN_AUTH, group_chat_id, auth);
        }
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if self_addr.is_null() {
            dc_log_error(
                context,
                0i32,
                b"Not configured, cannot generate QR code.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            self_name = dc_sqlite3_get_config(
                (*context).sql,
                b"displayname\x00" as *const u8 as *const libc::c_char,
                b"\x00" as *const u8 as *const libc::c_char,
            );
            fingerprint = get_self_fingerprint(context);
            if !fingerprint.is_null() {
                self_addr_urlencoded = dc_urlencode(self_addr);
                self_name_urlencoded = dc_urlencode(self_name);
                if 0 != group_chat_id {
                    chat = dc_get_chat(context, group_chat_id);
                    if chat.is_null() {
                        dc_log_error(
                            context,
                            0i32,
                            b"Cannot get QR-code for chat-id %i\x00" as *const u8
                                as *const libc::c_char,
                            group_chat_id,
                        );
                        current_block = 9531737720721467826;
                    } else {
                        group_name = dc_chat_get_name(chat);
                        group_name_urlencoded = dc_urlencode(group_name);
                        qr = dc_mprintf(
                            b"OPENPGP4FPR:%s#a=%s&g=%s&x=%s&i=%s&s=%s\x00" as *const u8
                                as *const libc::c_char,
                            fingerprint,
                            self_addr_urlencoded,
                            group_name_urlencoded,
                            (*chat).grpid,
                            invitenumber,
                            auth,
                        );
                        current_block = 1118134448028020070;
                    }
                } else {
                    qr = dc_mprintf(
                        b"OPENPGP4FPR:%s#a=%s&n=%s&i=%s&s=%s\x00" as *const u8
                            as *const libc::c_char,
                        fingerprint,
                        self_addr_urlencoded,
                        self_name_urlencoded,
                        invitenumber,
                        auth,
                    );
                    current_block = 1118134448028020070;
                }
                match current_block {
                    9531737720721467826 => {}
                    _ => {
                        dc_log_info(
                            context,
                            0i32,
                            b"Generated QR code: %s\x00" as *const u8 as *const libc::c_char,
                            qr,
                        );
                    }
                }
            }
        }
    }
    free(self_addr_urlencoded as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    free(self_name as *mut libc::c_void);
    free(self_name_urlencoded as *mut libc::c_void);
    free(fingerprint as *mut libc::c_void);
    free(invitenumber as *mut libc::c_void);
    free(auth as *mut libc::c_void);
    dc_chat_unref(chat);
    free(group_name as *mut libc::c_void);
    free(group_name_urlencoded as *mut libc::c_void);
    return if !qr.is_null() {
        qr
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}
unsafe extern "C" fn get_self_fingerprint(mut context: *mut dc_context_t) -> *mut libc::c_char {
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_key: *mut dc_key_t = dc_key_new();
    let mut fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    self_addr = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    if !(self_addr.is_null() || 0 == dc_key_load_self_public(self_key, self_addr, (*context).sql)) {
        fingerprint = dc_key_get_fingerprint(self_key);
        fingerprint.is_null();
    }
    free(self_addr as *mut libc::c_void);
    dc_key_unref(self_key);
    return fingerprint;
}
#[no_mangle]
pub unsafe extern "C" fn dc_join_securejoin(
    mut context: *mut dc_context_t,
    mut qr: *const libc::c_char,
) -> uint32_t {
    /* ==========================================================
    ====             Bob - the joiner's side             =====
    ====   Step 2 in "Setup verified contact" protocol   =====
    ========================================================== */
    let mut ret_chat_id: libc::c_int = 0i32;
    let mut ongoing_allocated: libc::c_int = 0i32;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;
    let mut join_vg: libc::c_int = 0i32;
    let mut qr_scan: *mut dc_lot_t = 0 as *mut dc_lot_t;
    let mut qr_locked: libc::c_int = 0i32;
    dc_log_info(
        context,
        0i32,
        b"Requesting secure-join ...\x00" as *const u8 as *const libc::c_char,
    );
    dc_ensure_secret_key_exists(context);
    ongoing_allocated = dc_alloc_ongoing(context);
    if !(ongoing_allocated == 0i32) {
        qr_scan = dc_check_qr(context, qr);
        if qr_scan.is_null() || (*qr_scan).state != 200i32 && (*qr_scan).state != 202i32 {
            dc_log_error(
                context,
                0i32,
                b"Unknown QR code.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            contact_chat_id = dc_create_chat_by_contact_id(context, (*qr_scan).id);
            if contact_chat_id == 0i32 as libc::c_uint {
                dc_log_error(
                    context,
                    0i32,
                    b"Unknown contact.\x00" as *const u8 as *const libc::c_char,
                );
            } else if !(0 != (*context).shall_stop_ongoing) {
                join_vg = ((*qr_scan).state == 202i32) as libc::c_int;
                (*context).bobs_status = 0i32;
                pthread_mutex_lock(&mut (*context).bobs_qr_critical);
                qr_locked = 1i32;
                (*context).bobs_qr_scan = qr_scan;
                if 0 != qr_locked {
                    pthread_mutex_unlock(&mut (*context).bobs_qr_critical);
                    qr_locked = 0i32
                }
                if 0 != fingerprint_equals_sender(context, (*qr_scan).fingerprint, contact_chat_id)
                {
                    dc_log_info(
                        context,
                        0i32,
                        b"Taking protocol shortcut.\x00" as *const u8 as *const libc::c_char,
                    );
                    (*context).bob_expects = 6i32;
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2061i32,
                        chat_id_2_contact_id(context, contact_chat_id) as uintptr_t,
                        400i32 as uintptr_t,
                    );
                    let mut own_fingerprint: *mut libc::c_char = get_self_fingerprint(context);
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char
                        },
                        (*qr_scan).auth,
                        own_fingerprint,
                        if 0 != join_vg {
                            (*qr_scan).text2
                        } else {
                            0 as *mut libc::c_char
                        },
                    );
                    free(own_fingerprint as *mut libc::c_void);
                } else {
                    (*context).bob_expects = 2i32;
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-request\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-request\x00" as *const u8 as *const libc::c_char
                        },
                        (*qr_scan).invitenumber,
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                }
                // Bob -> Alice
                while !(0 != (*context).shall_stop_ongoing) {
                    usleep((300i32 * 1000i32) as useconds_t);
                }
            }
        }
    }
    (*context).bob_expects = 0i32;
    if (*context).bobs_status == 1i32 {
        if 0 != join_vg {
            ret_chat_id = dc_get_chat_id_by_grpid(
                context,
                (*qr_scan).text2,
                0 as *mut libc::c_int,
                0 as *mut libc::c_int,
            ) as libc::c_int
        } else {
            ret_chat_id = contact_chat_id as libc::c_int
        }
    }
    pthread_mutex_lock(&mut (*context).bobs_qr_critical);
    qr_locked = 1i32;
    (*context).bobs_qr_scan = 0 as *mut dc_lot_t;
    if 0 != qr_locked {
        pthread_mutex_unlock(&mut (*context).bobs_qr_critical);
        qr_locked = 0i32
    }
    dc_lot_unref(qr_scan);
    if 0 != ongoing_allocated {
        dc_free_ongoing(context);
    }
    return ret_chat_id as uint32_t;
}
unsafe extern "C" fn send_handshake_msg(
    mut context: *mut dc_context_t,
    mut contact_chat_id: uint32_t,
    mut step: *const libc::c_char,
    mut param2: *const libc::c_char,
    mut fingerprint: *const libc::c_char,
    mut grpid: *const libc::c_char,
) {
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    (*msg).type_0 = 10i32;
    (*msg).text = dc_mprintf(
        b"Secure-Join: %s\x00" as *const u8 as *const libc::c_char,
        step,
    );
    (*msg).hidden = 1i32;
    dc_param_set_int((*msg).param, 'S' as i32, 7i32);
    dc_param_set((*msg).param, 'E' as i32, step);
    if !param2.is_null() {
        dc_param_set((*msg).param, 'F' as i32, param2);
    }
    if !fingerprint.is_null() {
        dc_param_set((*msg).param, 'G' as i32, fingerprint);
    }
    if !grpid.is_null() {
        dc_param_set((*msg).param, 'H' as i32, grpid);
    }
    if strcmp(step, b"vg-request\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(step, b"vc-request\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_param_set_int((*msg).param, 'u' as i32, 1i32);
    } else {
        dc_param_set_int((*msg).param, 'c' as i32, 1i32);
    }
    dc_send_msg(context, contact_chat_id, msg);
    dc_msg_unref(msg);
}
unsafe extern "C" fn chat_id_2_contact_id(
    mut context: *mut dc_context_t,
    mut contact_chat_id: uint32_t,
) -> uint32_t {
    let mut contact_id: uint32_t = 0i32 as uint32_t;
    let mut contacts: *mut dc_array_t = dc_get_chat_contacts(context, contact_chat_id);
    if !(dc_array_get_cnt(contacts) != 1i32 as libc::c_ulong) {
        contact_id = dc_array_get_id(contacts, 0i32 as size_t)
    }
    dc_array_unref(contacts);
    return contact_id;
}
unsafe extern "C" fn fingerprint_equals_sender(
    mut context: *mut dc_context_t,
    mut fingerprint: *const libc::c_char,
    mut contact_chat_id: uint32_t,
) -> libc::c_int {
    let mut fingerprint_equal: libc::c_int = 0i32;
    let mut contacts: *mut dc_array_t = dc_get_chat_contacts(context, contact_chat_id);
    let mut contact: *mut dc_contact_t = dc_contact_new(context);
    let mut peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
    let mut fingerprint_normalized: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(dc_array_get_cnt(contacts) != 1i32 as libc::c_ulong) {
        if !(0
            == dc_contact_load_from_db(
                contact,
                (*context).sql,
                dc_array_get_id(contacts, 0i32 as size_t),
            )
            || 0 == dc_apeerstate_load_by_addr(peerstate, (*context).sql, (*contact).addr))
        {
            fingerprint_normalized = dc_normalize_fingerprint(fingerprint);
            if strcasecmp(fingerprint_normalized, (*peerstate).public_key_fingerprint) == 0i32 {
                fingerprint_equal = 1i32
            }
        }
    }
    free(fingerprint_normalized as *mut libc::c_void);
    dc_contact_unref(contact);
    dc_array_unref(contacts);
    return fingerprint_equal;
}
/* library private: secure-join */
#[no_mangle]
pub unsafe extern "C" fn dc_handle_securejoin_handshake(
    mut context: *mut dc_context_t,
    mut mimeparser: *mut dc_mimeparser_t,
    mut contact_id: uint32_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut qr_locked: libc::c_int = 0i32;
    let mut step: *const libc::c_char = 0 as *const libc::c_char;
    let mut join_vg: libc::c_int = 0i32;
    let mut scanned_fingerprint_of_alice: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut auth: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut own_fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;
    let mut contact_chat_id_blocked: libc::c_int = 0i32;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(context.is_null() || mimeparser.is_null() || contact_id <= 9i32 as libc::c_uint) {
        step = lookup_field(
            mimeparser,
            b"Secure-Join\x00" as *const u8 as *const libc::c_char,
        );
        if !step.is_null() {
            dc_log_info(
                context,
                0i32,
                b">>>>>>>>>>>>>>>>>>>>>>>>> secure-join message \'%s\' received\x00" as *const u8
                    as *const libc::c_char,
                step,
            );
            join_vg = (strncmp(
                step,
                b"vg-\x00" as *const u8 as *const libc::c_char,
                3i32 as libc::c_ulong,
            ) == 0i32) as libc::c_int;
            dc_create_or_lookup_nchat_by_contact_id(
                context,
                contact_id,
                0i32,
                &mut contact_chat_id,
                &mut contact_chat_id_blocked,
            );
            if 0 != contact_chat_id_blocked {
                dc_unblock_chat(context, contact_chat_id);
            }
            ret = 0x2i32;
            if strcmp(step, b"vg-request\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(step, b"vc-request\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                /* =========================================================
                ====             Alice - the inviter side            ====
                ====   Step 3 in "Setup verified contact" protocol   ====
                ========================================================= */
                // this message may be unencrypted (Bob, the joinder and the sender, might not have Alice's key yet)
                // it just ensures, we have Bobs key now. If we do _not_ have the key because eg. MitM has removed it,
                // send_message() will fail with the error "End-to-end-encryption unavailable unexpectedly.", so, there is no additional check needed here.
                // verify that the `Secure-Join-Invitenumber:`-header matches invitenumber written to the QR code
                let mut invitenumber: *const libc::c_char = 0 as *const libc::c_char;
                invitenumber = lookup_field(
                    mimeparser,
                    b"Secure-Join-Invitenumber\x00" as *const u8 as *const libc::c_char,
                );
                if invitenumber.is_null() {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Secure-join denied (invitenumber missing).\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if dc_token_exists(context, DC_TOKEN_INVITENUMBER, invitenumber) == 0i32 {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Secure-join denied (bad invitenumber).\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    dc_log_info(
                        context,
                        0i32,
                        b"Secure-join requested.\x00" as *const u8 as *const libc::c_char,
                    );
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2060i32,
                        contact_id as uintptr_t,
                        300i32 as uintptr_t,
                    );
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-auth-required\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-auth-required\x00" as *const u8 as *const libc::c_char
                        },
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                    current_block = 10256747982273457880;
                }
            } else if strcmp(
                step,
                b"vg-auth-required\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-auth-required\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                pthread_mutex_lock(&mut (*context).bobs_qr_critical);
                qr_locked = 1i32;
                if (*context).bobs_qr_scan.is_null()
                    || (*context).bob_expects != 2i32
                    || 0 != join_vg && (*(*context).bobs_qr_scan).state != 202i32
                {
                    dc_log_warning(
                        context,
                        0i32,
                        b"auth-required message out of sync.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    // no error, just aborted somehow or a mail from another handshake
                    current_block = 4378276786830486580;
                } else {
                    scanned_fingerprint_of_alice =
                        dc_strdup((*(*context).bobs_qr_scan).fingerprint);
                    auth = dc_strdup((*(*context).bobs_qr_scan).auth);
                    if 0 != join_vg {
                        grpid = dc_strdup((*(*context).bobs_qr_scan).text2)
                    }
                    if 0 != qr_locked {
                        pthread_mutex_unlock(&mut (*context).bobs_qr_critical);
                        qr_locked = 0i32
                    }
                    if 0 == encrypted_and_signed(mimeparser, scanned_fingerprint_of_alice) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            if 0 != (*(*mimeparser).e2ee_helper).encrypted {
                                b"No valid signature.\x00" as *const u8 as *const libc::c_char
                            } else {
                                b"Not encrypted.\x00" as *const u8 as *const libc::c_char
                            },
                        );
                        end_bobs_joining(context, 0i32);
                        current_block = 4378276786830486580;
                    } else if 0
                        == fingerprint_equals_sender(
                            context,
                            scanned_fingerprint_of_alice,
                            contact_chat_id,
                        )
                    {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Fingerprint mismatch on joiner-side.\x00" as *const u8
                                as *const libc::c_char,
                        );
                        end_bobs_joining(context, 0i32);
                        current_block = 4378276786830486580;
                    } else {
                        dc_log_info(
                            context,
                            0i32,
                            b"Fingerprint verified.\x00" as *const u8 as *const libc::c_char,
                        );
                        own_fingerprint = get_self_fingerprint(context);
                        (*context).cb.expect("non-null function pointer")(
                            context,
                            2061i32,
                            contact_id as uintptr_t,
                            400i32 as uintptr_t,
                        );
                        (*context).bob_expects = 6i32;
                        send_handshake_msg(
                            context,
                            contact_chat_id,
                            if 0 != join_vg {
                                b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char
                            } else {
                                b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char
                            },
                            auth,
                            own_fingerprint,
                            grpid,
                        );
                        current_block = 10256747982273457880;
                    }
                }
            } else if strcmp(
                step,
                b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                /* ============================================================
                ====              Alice - the inviter side              ====
                ====   Steps 5+6 in "Setup verified contact" protocol   ====
                ====  Step 6 in "Out-of-band verified groups" protocol  ====
                ============================================================ */
                // verify that Secure-Join-Fingerprint:-header matches the fingerprint of Bob
                let mut fingerprint: *const libc::c_char = 0 as *const libc::c_char;
                fingerprint = lookup_field(
                    mimeparser,
                    b"Secure-Join-Fingerprint\x00" as *const u8 as *const libc::c_char,
                );
                if fingerprint.is_null() {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint not provided.\x00" as *const u8 as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if 0 == encrypted_and_signed(mimeparser, fingerprint) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Auth not encrypted.\x00" as *const u8 as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if 0 == fingerprint_equals_sender(context, fingerprint, contact_chat_id) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    dc_log_info(
                        context,
                        0i32,
                        b"Fingerprint verified.\x00" as *const u8 as *const libc::c_char,
                    );
                    // verify that the `Secure-Join-Auth:`-header matches the secret written to the QR code
                    let mut auth_0: *const libc::c_char = 0 as *const libc::c_char;
                    auth_0 = lookup_field(
                        mimeparser,
                        b"Secure-Join-Auth\x00" as *const u8 as *const libc::c_char,
                    );
                    if auth_0.is_null() {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Auth not provided.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else if dc_token_exists(context, DC_TOKEN_AUTH, auth_0) == 0i32 {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Auth invalid.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else if 0 == mark_peer_as_verified(context, fingerprint) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                                as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else {
                        dc_scaleup_contact_origin(context, contact_id, 0x1000000i32);
                        dc_log_info(
                            context,
                            0i32,
                            b"Auth verified.\x00" as *const u8 as *const libc::c_char,
                        );
                        secure_connection_established(context, contact_chat_id);
                        (*context).cb.expect("non-null function pointer")(
                            context,
                            2030i32,
                            contact_id as uintptr_t,
                            0i32 as uintptr_t,
                        );
                        (*context).cb.expect("non-null function pointer")(
                            context,
                            2060i32,
                            contact_id as uintptr_t,
                            600i32 as uintptr_t,
                        );
                        if 0 != join_vg {
                            grpid = dc_strdup(lookup_field(
                                mimeparser,
                                b"Secure-Join-Group\x00" as *const u8 as *const libc::c_char,
                            ));
                            let mut group_chat_id: uint32_t = dc_get_chat_id_by_grpid(
                                context,
                                grpid,
                                0 as *mut libc::c_int,
                                0 as *mut libc::c_int,
                            );
                            if group_chat_id == 0i32 as libc::c_uint {
                                dc_log_error(
                                    context,
                                    0i32,
                                    b"Chat %s not found.\x00" as *const u8 as *const libc::c_char,
                                    grpid,
                                );
                                current_block = 4378276786830486580;
                            } else {
                                dc_add_contact_to_chat_ex(
                                    context,
                                    group_chat_id,
                                    contact_id,
                                    0x1i32,
                                );
                                current_block = 10256747982273457880;
                            }
                        } else {
                            send_handshake_msg(
                                context,
                                contact_chat_id,
                                b"vc-contact-confirm\x00" as *const u8 as *const libc::c_char,
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                            );
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2060i32,
                                contact_id as uintptr_t,
                                1000i32 as uintptr_t,
                            );
                            current_block = 10256747982273457880;
                        }
                    }
                }
            } else if strcmp(
                step,
                b"vg-member-added\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-contact-confirm\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                if 0 != join_vg {
                    ret = 0x1i32
                }
                if (*context).bob_expects != 6i32 {
                    dc_log_info(
                        context,
                        0i32,
                        b"Message belongs to a different handshake.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    pthread_mutex_lock(&mut (*context).bobs_qr_critical);
                    qr_locked = 1i32;
                    if (*context).bobs_qr_scan.is_null()
                        || 0 != join_vg && (*(*context).bobs_qr_scan).state != 202i32
                    {
                        dc_log_warning(
                            context,
                            0i32,
                            b"Message out of sync or belongs to a different handshake.\x00"
                                as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else {
                        scanned_fingerprint_of_alice =
                            dc_strdup((*(*context).bobs_qr_scan).fingerprint);
                        if 0 != join_vg {
                            grpid = dc_strdup((*(*context).bobs_qr_scan).text2)
                        }
                        if 0 != qr_locked {
                            pthread_mutex_unlock(&mut (*context).bobs_qr_critical);
                            qr_locked = 0i32
                        }
                        let mut vg_expect_encrypted: libc::c_int = 1i32;
                        if 0 != join_vg {
                            let mut is_verified_group: libc::c_int = 0i32;
                            dc_get_chat_id_by_grpid(
                                context,
                                grpid,
                                0 as *mut libc::c_int,
                                &mut is_verified_group,
                            );
                            if 0 == is_verified_group {
                                vg_expect_encrypted = 0i32
                            }
                        }
                        if 0 != vg_expect_encrypted {
                            if 0 == encrypted_and_signed(mimeparser, scanned_fingerprint_of_alice) {
                                could_not_establish_secure_connection(
                                    context,
                                    contact_chat_id,
                                    b"Contact confirm message not encrypted.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                end_bobs_joining(context, 0i32);
                                current_block = 4378276786830486580;
                            } else {
                                current_block = 5195798230510548452;
                            }
                        } else {
                            current_block = 5195798230510548452;
                        }
                        match current_block {
                            4378276786830486580 => {}
                            _ => {
                                if 0 == mark_peer_as_verified(context, scanned_fingerprint_of_alice)
                                {
                                    could_not_establish_secure_connection(
                                        context,
                                        contact_chat_id,
                                        b"Fingerprint mismatch on joiner-side.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    current_block = 4378276786830486580;
                                } else {
                                    dc_scaleup_contact_origin(context, contact_id, 0x2000000i32);
                                    (*context).cb.expect("non-null function pointer")(
                                        context,
                                        2030i32,
                                        0i32 as uintptr_t,
                                        0i32 as uintptr_t,
                                    );
                                    if 0 != join_vg {
                                        if 0 == dc_addr_equals_self(
                                            context,
                                            lookup_field(
                                                mimeparser,
                                                b"Chat-Group-Member-Added\x00" as *const u8
                                                    as *const libc::c_char,
                                            ),
                                        ) {
                                            dc_log_info(context, 0i32,
                                                        b"Message belongs to a different handshake (scaled up contact anyway to allow creation of group).\x00"
                                                            as *const u8 as
                                                            *const libc::c_char);
                                            current_block = 4378276786830486580;
                                        } else {
                                            current_block = 9180031981464905198;
                                        }
                                    } else {
                                        current_block = 9180031981464905198;
                                    }
                                    match current_block {
                                        4378276786830486580 => {}
                                        _ => {
                                            secure_connection_established(context, contact_chat_id);
                                            (*context).bob_expects = 0i32;
                                            if 0 != join_vg {
                                                send_handshake_msg(
                                                    context,
                                                    contact_chat_id,
                                                    b"vg-member-added-received\x00" as *const u8
                                                        as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                );
                                            }
                                            end_bobs_joining(context, 1i32);
                                            current_block = 10256747982273457880;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if strcmp(
                step,
                b"vg-member-added-received\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
            {
                /* ============================================================
                ====              Alice - the inviter side              ====
                ====  Step 8 in "Out-of-band verified groups" protocol  ====
                ============================================================ */
                contact = dc_get_contact(context, contact_id);
                if contact.is_null() || 0 == dc_contact_is_verified(contact) {
                    dc_log_warning(
                        context,
                        0i32,
                        b"vg-member-added-received invalid.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2060i32,
                        contact_id as uintptr_t,
                        800i32 as uintptr_t,
                    );
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2060i32,
                        contact_id as uintptr_t,
                        1000i32 as uintptr_t,
                    );
                    current_block = 10256747982273457880;
                }
            } else {
                current_block = 10256747982273457880;
            }
            match current_block {
                4378276786830486580 => {}
                _ => {
                    if 0 != ret & 0x2i32 {
                        ret |= 0x4i32
                    }
                }
            }
        }
    }
    if 0 != qr_locked {
        pthread_mutex_unlock(&mut (*context).bobs_qr_critical);
        qr_locked = 0i32
    }
    dc_contact_unref(contact);
    free(scanned_fingerprint_of_alice as *mut libc::c_void);
    free(auth as *mut libc::c_void);
    free(own_fingerprint as *mut libc::c_void);
    free(grpid as *mut libc::c_void);
    return ret;
}
unsafe extern "C" fn end_bobs_joining(mut context: *mut dc_context_t, mut status: libc::c_int) {
    (*context).bobs_status = status;
    dc_stop_ongoing_process(context);
}
unsafe extern "C" fn secure_connection_established(
    mut context: *mut dc_context_t,
    mut contact_chat_id: uint32_t,
) {
    let mut contact_id: uint32_t = chat_id_2_contact_id(context, contact_chat_id);
    let mut contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let mut msg: *mut libc::c_char = dc_stock_str_repl_string(
        context,
        35i32,
        if !contact.is_null() {
            (*contact).addr
        } else {
            b"?\x00" as *const u8 as *const libc::c_char
        },
    );
    dc_add_device_msg(context, contact_chat_id, msg);
    (*context).cb.expect("non-null function pointer")(
        context,
        2020i32,
        contact_chat_id as uintptr_t,
        0i32 as uintptr_t,
    );
    free(msg as *mut libc::c_void);
    dc_contact_unref(contact);
}
unsafe extern "C" fn lookup_field(
    mut mimeparser: *mut dc_mimeparser_t,
    mut key: *const libc::c_char,
) -> *const libc::c_char {
    let mut value: *const libc::c_char = 0 as *const libc::c_char;
    let mut field: *mut mailimf_field = dc_mimeparser_lookup_field(mimeparser, key);
    if field.is_null()
        || (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
        || (*field).fld_data.fld_optional_field.is_null()
        || {
            value = (*(*field).fld_data.fld_optional_field).fld_value;
            value.is_null()
        }
    {
        return 0 as *const libc::c_char;
    }
    return value;
}
unsafe extern "C" fn could_not_establish_secure_connection(
    mut context: *mut dc_context_t,
    mut contact_chat_id: uint32_t,
    mut details: *const libc::c_char,
) {
    let mut contact_id: uint32_t = chat_id_2_contact_id(context, contact_chat_id);
    let mut contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let mut msg: *mut libc::c_char = dc_stock_str_repl_string(
        context,
        36i32,
        if !contact.is_null() {
            (*contact).addr
        } else {
            b"?\x00" as *const u8 as *const libc::c_char
        },
    );
    dc_add_device_msg(context, contact_chat_id, msg);
    dc_log_error(
        context,
        0i32,
        b"%s (%s)\x00" as *const u8 as *const libc::c_char,
        msg,
        details,
    );
    free(msg as *mut libc::c_void);
    dc_contact_unref(contact);
}
unsafe extern "C" fn mark_peer_as_verified(
    mut context: *mut dc_context_t,
    mut fingerprint: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
    if !(0 == dc_apeerstate_load_by_fingerprint(peerstate, (*context).sql, fingerprint)) {
        if !(0 == dc_apeerstate_set_verified(peerstate, 1i32, fingerprint, 2i32)) {
            (*peerstate).prefer_encrypt = 1i32;
            (*peerstate).to_save |= 0x2i32;
            dc_apeerstate_save_to_db(peerstate, (*context).sql, 0i32);
            success = 1i32
        }
    }
    dc_apeerstate_unref(peerstate);
    return success;
}
/* ******************************************************************************
 * Tools: Misc.
 ******************************************************************************/
unsafe extern "C" fn encrypted_and_signed(
    mut mimeparser: *mut dc_mimeparser_t,
    mut expected_fingerprint: *const libc::c_char,
) -> libc::c_int {
    if 0 == (*(*mimeparser).e2ee_helper).encrypted {
        dc_log_warning(
            (*mimeparser).context,
            0i32,
            b"Message not encrypted.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if (*(*(*mimeparser).e2ee_helper).signatures).count <= 0i32 {
        dc_log_warning(
            (*mimeparser).context,
            0i32,
            b"Message not signed.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if expected_fingerprint.is_null() {
        dc_log_warning(
            (*mimeparser).context,
            0i32,
            b"Fingerprint for comparison missing.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if dc_hash_find(
        (*(*mimeparser).e2ee_helper).signatures,
        expected_fingerprint as *const libc::c_void,
        strlen(expected_fingerprint) as libc::c_int,
    )
    .is_null()
    {
        dc_log_warning(
            (*mimeparser).context,
            0i32,
            b"Message does not match expected fingerprint %s.\x00" as *const u8
                as *const libc::c_char,
            expected_fingerprint,
        );
        return 0i32;
    }
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_handle_degrade_event(
    mut context: *mut dc_context_t,
    mut peerstate: *mut dc_apeerstate_t,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut contact_id: uint32_t = 0i32 as uint32_t;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;
    if !(context.is_null() || peerstate.is_null()) {
        // - we do not issue an warning for DC_DE_ENCRYPTION_PAUSED as this is quite normal
        // - currently, we do not issue an extra warning for DC_DE_VERIFICATION_LOST - this always comes
        //   together with DC_DE_FINGERPRINT_CHANGED which is logged, the idea is not to bother
        //   with things they cannot fix, so the user is just kicked from the verified group
        //   (and he will know this and can fix this)
        if 0 != (*peerstate).degrade_event & 0x2i32 {
            stmt = dc_sqlite3_prepare(
                (*context).sql,
                b"SELECT id FROM contacts WHERE addr=?;\x00" as *const u8 as *const libc::c_char,
            );
            sqlite3_bind_text(stmt, 1i32, (*peerstate).addr, -1i32, None);
            sqlite3_step(stmt);
            contact_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
            sqlite3_finalize(stmt);
            if !(contact_id == 0i32 as libc::c_uint) {
                dc_create_or_lookup_nchat_by_contact_id(
                    context,
                    contact_id,
                    2i32,
                    &mut contact_chat_id,
                    0 as *mut libc::c_int,
                );
                let mut msg: *mut libc::c_char =
                    dc_stock_str_repl_string(context, 37i32, (*peerstate).addr);
                dc_add_device_msg(context, contact_chat_id, msg);
                free(msg as *mut libc::c_void);
                (*context).cb.expect("non-null function pointer")(
                    context,
                    2020i32,
                    contact_chat_id as uintptr_t,
                    0i32 as uintptr_t,
                );
            }
        }
    };
}

use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type sqlite3_stmt;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn memcpy(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn localtime(_: *const time_t) -> *mut tm;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn mmap_string_new(init: *const libc::c_char) -> *mut MMAPString;
    #[no_mangle]
    fn mmap_string_free(string: *mut MMAPString);
    #[no_mangle]
    fn clist_new() -> *mut clist;
    #[no_mangle]
    fn clist_free(_: *mut clist);
    #[no_mangle]
    fn clist_insert_after(_: *mut clist, _: *mut clistiter, _: *mut libc::c_void) -> libc::c_int;
    #[no_mangle]
    fn mailimf_address_new(
        ad_type: libc::c_int,
        ad_mailbox: *mut mailimf_mailbox,
        ad_group: *mut mailimf_group,
    ) -> *mut mailimf_address;
    #[no_mangle]
    fn mailimf_mailbox_new(
        mb_display_name: *mut libc::c_char,
        mb_addr_spec: *mut libc::c_char,
    ) -> *mut mailimf_mailbox;
    #[no_mangle]
    fn mailimf_field_new(
        fld_type: libc::c_int,
        fld_return_path: *mut mailimf_return,
        fld_resent_date: *mut mailimf_orig_date,
        fld_resent_from: *mut mailimf_from,
        fld_resent_sender: *mut mailimf_sender,
        fld_resent_to: *mut mailimf_to,
        fld_resent_cc: *mut mailimf_cc,
        fld_resent_bcc: *mut mailimf_bcc,
        fld_resent_msg_id: *mut mailimf_message_id,
        fld_orig_date: *mut mailimf_orig_date,
        fld_from: *mut mailimf_from,
        fld_sender: *mut mailimf_sender,
        fld_reply_to: *mut mailimf_reply_to,
        fld_to: *mut mailimf_to,
        fld_cc: *mut mailimf_cc,
        fld_bcc: *mut mailimf_bcc,
        fld_message_id: *mut mailimf_message_id,
        fld_in_reply_to: *mut mailimf_in_reply_to,
        fld_references: *mut mailimf_references,
        fld_subject: *mut mailimf_subject,
        fld_comments: *mut mailimf_comments,
        fld_keywords: *mut mailimf_keywords,
        fld_optional_field: *mut mailimf_optional_field,
    ) -> *mut mailimf_field;
    #[no_mangle]
    fn mailimf_subject_new(sbj_value: *mut libc::c_char) -> *mut mailimf_subject;
    #[no_mangle]
    fn mailimf_mailbox_list_new_empty() -> *mut mailimf_mailbox_list;
    #[no_mangle]
    fn mailimf_mailbox_list_add(
        mailbox_list: *mut mailimf_mailbox_list,
        mb: *mut mailimf_mailbox,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailimf_address_list_new_empty() -> *mut mailimf_address_list;
    #[no_mangle]
    fn mailimf_address_list_add(
        address_list: *mut mailimf_address_list,
        addr: *mut mailimf_address,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailimf_fields_add(fields: *mut mailimf_fields, field: *mut mailimf_field) -> libc::c_int;
    #[no_mangle]
    fn mailimf_fields_new_with_data_all(
        date: *mut mailimf_date_time,
        from: *mut mailimf_mailbox_list,
        sender: *mut mailimf_mailbox,
        reply_to: *mut mailimf_address_list,
        to: *mut mailimf_address_list,
        cc: *mut mailimf_address_list,
        bcc: *mut mailimf_address_list,
        message_id: *mut libc::c_char,
        in_reply_to: *mut clist,
        references: *mut clist,
        subject: *mut libc::c_char,
    ) -> *mut mailimf_fields;
    #[no_mangle]
    fn mailimf_get_date(time_0: time_t) -> *mut mailimf_date_time;
    #[no_mangle]
    fn mailimf_field_new_custom(
        name: *mut libc::c_char,
        value: *mut libc::c_char,
    ) -> *mut mailimf_field;
    #[no_mangle]
    fn mailmime_parameter_new(
        pa_name: *mut libc::c_char,
        pa_value: *mut libc::c_char,
    ) -> *mut mailmime_parameter;
    #[no_mangle]
    fn mailmime_free(mime: *mut mailmime);
    #[no_mangle]
    fn mailmime_disposition_parm_new(
        pa_type: libc::c_int,
        pa_filename: *mut libc::c_char,
        pa_creation_date: *mut libc::c_char,
        pa_modification_date: *mut libc::c_char,
        pa_read_date: *mut libc::c_char,
        pa_size: size_t,
        pa_parameter: *mut mailmime_parameter,
    ) -> *mut mailmime_disposition_parm;
    #[no_mangle]
    fn mailmime_new_message_data(msg_mime: *mut mailmime) -> *mut mailmime;
    #[no_mangle]
    fn mailmime_new_empty(
        content: *mut mailmime_content,
        mime_fields: *mut mailmime_fields,
    ) -> *mut mailmime;
    #[no_mangle]
    fn mailmime_set_body_file(
        build_info: *mut mailmime,
        filename: *mut libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_set_body_text(
        build_info: *mut mailmime,
        data_str: *mut libc::c_char,
        length: size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_add_part(build_info: *mut mailmime, part: *mut mailmime) -> libc::c_int;
    #[no_mangle]
    fn mailmime_set_imf_fields(build_info: *mut mailmime, fields: *mut mailimf_fields);
    #[no_mangle]
    fn mailmime_smart_add_part(mime: *mut mailmime, mime_sub: *mut mailmime) -> libc::c_int;
    #[no_mangle]
    fn mailmime_content_new_with_str(str: *const libc::c_char) -> *mut mailmime_content;
    #[no_mangle]
    fn mailmime_fields_new_encoding(type_0: libc::c_int) -> *mut mailmime_fields;
    #[no_mangle]
    fn mailmime_multiple_new(type_0: *const libc::c_char) -> *mut mailmime;
    #[no_mangle]
    fn mailmime_fields_new_filename(
        dsp_type: libc::c_int,
        filename: *mut libc::c_char,
        encoding_type: libc::c_int,
    ) -> *mut mailmime_fields;
    #[no_mangle]
    fn mailmime_param_new_with_data(
        name: *mut libc::c_char,
        value: *mut libc::c_char,
    ) -> *mut mailmime_parameter;
    #[no_mangle]
    fn mailmime_write_mem(
        f: *mut MMAPString,
        col: *mut libc::c_int,
        build_info: *mut mailmime,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_is_sending_locations_to_chat(_: *mut dc_context_t, chat_id: uint32_t) -> libc::c_int;
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
    #[no_mangle]
    fn dc_chat_unref(_: *mut dc_chat_t);
    #[no_mangle]
    fn dc_chat_is_self_talk(_: *const dc_chat_t) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_unref(_: *mut dc_msg_t);
    #[no_mangle]
    fn dc_msg_get_summarytext(
        _: *const dc_msg_t,
        approx_characters: libc::c_int,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_msg_is_increation(_: *const dc_msg_t) -> libc::c_int;
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
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
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
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_str_to_clist(_: *const libc::c_char, delimiter: *const libc::c_char) -> *mut clist;
    /* clist tools */
    #[no_mangle]
    fn clist_free_content(_: *const clist);
    #[no_mangle]
    fn clist_search_string_nocase(_: *const clist, str: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_create_smeared_timestamp(_: *mut dc_context_t) -> time_t;
    #[no_mangle]
    fn dc_create_outgoing_rfc724_mid(
        grpid: *const libc::c_char,
        addr: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_filename(pathNfilename: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_filesuffix_lc(pathNfilename: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_abs_path(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_get_filebytes(_: *mut dc_context_t, pathNfilename: *const libc::c_char) -> uint64_t;
    #[no_mangle]
    fn dc_encode_header_words(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_needs_ext_header(_: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_encode_ext_header(_: *const libc::c_char) -> *mut libc::c_char;
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
    fn dc_param_get(
        _: *const dc_param_t,
        key: libc::c_int,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_get_int(_: *const dc_param_t, key: libc::c_int, def: int32_t) -> int32_t;
    #[no_mangle]
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
    /* Return the string with the given ID by calling DC_EVENT_GET_STRING.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str(_: *mut dc_context_t, id: libc::c_int) -> *mut libc::c_char;
    /* Replaces the first `%1$s` in the given String-ID by the given value.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str_repl_string(
        _: *mut dc_context_t,
        id: libc::c_int,
        value: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_chat_load_from_db(_: *mut dc_chat_t, id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_new_untyped(_: *mut dc_context_t) -> *mut dc_msg_t;
    #[no_mangle]
    fn dc_msg_load_from_db(_: *mut dc_msg_t, _: *mut dc_context_t, id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_get_summarytext_by_raw(
        type_0: libc::c_int,
        text: *const libc::c_char,
        _: *mut dc_param_t,
        approx_bytes: libc::c_int,
        _: *mut dc_context_t,
    ) -> *mut libc::c_char;
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
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_get_location_kml(
        _: *mut dc_context_t,
        chat_id: uint32_t,
        last_added_location_id: *mut uint32_t,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_e2ee_encrypt(
        _: *mut dc_context_t,
        recipients_addr: *const clist,
        force_plaintext: libc::c_int,
        e2ee_guaranteed: libc::c_int,
        min_verified: libc::c_int,
        do_gossip: libc::c_int,
        in_out_message: *mut mailmime,
        _: *mut dc_e2ee_helper_t,
    );
    #[no_mangle]
    fn dc_e2ee_thanks(_: *mut dc_e2ee_helper_t);
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
pub type unnamed = libc::c_uint;
pub const MAILIMF_ADDRESS_GROUP: unnamed = 2;
pub const MAILIMF_ADDRESS_MAILBOX: unnamed = 1;
pub const MAILIMF_ADDRESS_ERROR: unnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_address {
    pub ad_type: libc::c_int,
    pub ad_data: unnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_0 {
    pub ad_mailbox: *mut mailimf_mailbox,
    pub ad_group: *mut mailimf_group,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_group {
    pub grp_display_name: *mut libc::c_char,
    pub grp_mb_list: *mut mailimf_mailbox_list,
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
pub type unnamed_1 = libc::c_uint;
pub const MAILIMF_FIELD_OPTIONAL_FIELD: unnamed_1 = 22;
pub const MAILIMF_FIELD_KEYWORDS: unnamed_1 = 21;
pub const MAILIMF_FIELD_COMMENTS: unnamed_1 = 20;
pub const MAILIMF_FIELD_SUBJECT: unnamed_1 = 19;
pub const MAILIMF_FIELD_REFERENCES: unnamed_1 = 18;
pub const MAILIMF_FIELD_IN_REPLY_TO: unnamed_1 = 17;
pub const MAILIMF_FIELD_MESSAGE_ID: unnamed_1 = 16;
pub const MAILIMF_FIELD_BCC: unnamed_1 = 15;
pub const MAILIMF_FIELD_CC: unnamed_1 = 14;
pub const MAILIMF_FIELD_TO: unnamed_1 = 13;
pub const MAILIMF_FIELD_REPLY_TO: unnamed_1 = 12;
pub const MAILIMF_FIELD_SENDER: unnamed_1 = 11;
pub const MAILIMF_FIELD_FROM: unnamed_1 = 10;
pub const MAILIMF_FIELD_ORIG_DATE: unnamed_1 = 9;
pub const MAILIMF_FIELD_RESENT_MSG_ID: unnamed_1 = 8;
pub const MAILIMF_FIELD_RESENT_BCC: unnamed_1 = 7;
pub const MAILIMF_FIELD_RESENT_CC: unnamed_1 = 6;
pub const MAILIMF_FIELD_RESENT_TO: unnamed_1 = 5;
pub const MAILIMF_FIELD_RESENT_SENDER: unnamed_1 = 4;
pub const MAILIMF_FIELD_RESENT_FROM: unnamed_1 = 3;
pub const MAILIMF_FIELD_RESENT_DATE: unnamed_1 = 2;
pub const MAILIMF_FIELD_RETURN_PATH: unnamed_1 = 1;
pub const MAILIMF_FIELD_NONE: unnamed_1 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailimf_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_2,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_2 {
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
    pub tp_data: unnamed_3,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_3 {
    pub tp_discrete_type: *mut mailmime_discrete_type,
    pub tp_composite_type: *mut mailmime_composite_type,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_discrete_type {
    pub dt_type: libc::c_int,
    pub dt_extension: *mut libc::c_char,
}
pub type unnamed_4 = libc::c_uint;
pub const MAILMIME_FIELD_LOCATION: unnamed_4 = 8;
pub const MAILMIME_FIELD_LANGUAGE: unnamed_4 = 7;
pub const MAILMIME_FIELD_DISPOSITION: unnamed_4 = 6;
pub const MAILMIME_FIELD_VERSION: unnamed_4 = 5;
pub const MAILMIME_FIELD_DESCRIPTION: unnamed_4 = 4;
pub const MAILMIME_FIELD_ID: unnamed_4 = 3;
pub const MAILMIME_FIELD_TRANSFER_ENCODING: unnamed_4 = 2;
pub const MAILMIME_FIELD_TYPE: unnamed_4 = 1;
pub const MAILMIME_FIELD_NONE: unnamed_4 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_5,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_5 {
    pub fld_content: *mut mailmime_content,
    pub fld_encoding: *mut mailmime_mechanism,
    pub fld_id: *mut libc::c_char,
    pub fld_description: *mut libc::c_char,
    pub fld_version: uint32_t,
    pub fld_disposition: *mut mailmime_disposition,
    pub fld_language: *mut mailmime_language,
    pub fld_location: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_language {
    pub lg_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition {
    pub dsp_type: *mut mailmime_disposition_type,
    pub dsp_parms: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition_type {
    pub dsp_type: libc::c_int,
    pub dsp_extension: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_mechanism {
    pub enc_type: libc::c_int,
    pub enc_token: *mut libc::c_char,
}
pub type unnamed_6 = libc::c_uint;
pub const MAILMIME_MECHANISM_TOKEN: unnamed_6 = 6;
pub const MAILMIME_MECHANISM_BASE64: unnamed_6 = 5;
pub const MAILMIME_MECHANISM_QUOTED_PRINTABLE: unnamed_6 = 4;
pub const MAILMIME_MECHANISM_BINARY: unnamed_6 = 3;
pub const MAILMIME_MECHANISM_8BIT: unnamed_6 = 2;
pub const MAILMIME_MECHANISM_7BIT: unnamed_6 = 1;
pub const MAILMIME_MECHANISM_ERROR: unnamed_6 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_fields {
    pub fld_list: *mut clist,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_parameter {
    pub pa_name: *mut libc::c_char,
    pub pa_value: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_data {
    pub dt_type: libc::c_int,
    pub dt_encoding: libc::c_int,
    pub dt_encoded: libc::c_int,
    pub dt_data: unnamed_7,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_7 {
    pub dt_text: unnamed_8,
    pub dt_filename: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_8 {
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
    pub mm_data: unnamed_9,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_9 {
    pub mm_single: *mut mailmime_data,
    pub mm_multipart: unnamed_11,
    pub mm_message: unnamed_10,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_10 {
    pub mm_fields: *mut mailimf_fields,
    pub mm_msg_mime: *mut mailmime,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_11 {
    pub mm_preamble: *mut mailmime_data,
    pub mm_epilogue: *mut mailmime_data,
    pub mm_mp_list: *mut clist,
}
pub type unnamed_12 = libc::c_uint;
pub const MAILMIME_DISPOSITION_TYPE_EXTENSION: unnamed_12 = 3;
pub const MAILMIME_DISPOSITION_TYPE_ATTACHMENT: unnamed_12 = 2;
pub const MAILMIME_DISPOSITION_TYPE_INLINE: unnamed_12 = 1;
pub const MAILMIME_DISPOSITION_TYPE_ERROR: unnamed_12 = 0;
pub type unnamed_13 = libc::c_uint;
pub const MAILMIME_DISPOSITION_PARM_PARAMETER: unnamed_13 = 5;
pub const MAILMIME_DISPOSITION_PARM_SIZE: unnamed_13 = 4;
pub const MAILMIME_DISPOSITION_PARM_READ_DATE: unnamed_13 = 3;
pub const MAILMIME_DISPOSITION_PARM_MODIFICATION_DATE: unnamed_13 = 2;
pub const MAILMIME_DISPOSITION_PARM_CREATION_DATE: unnamed_13 = 1;
pub const MAILMIME_DISPOSITION_PARM_FILENAME: unnamed_13 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition_parm {
    pub pa_type: libc::c_int,
    pub pa_data: unnamed_14,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_14 {
    pub pa_filename: *mut libc::c_char,
    pub pa_creation_date: *mut libc::c_char,
    pub pa_modification_date: *mut libc::c_char,
    pub pa_read_date: *mut libc::c_char,
    pub pa_size: size_t,
    pub pa_parameter: *mut mailmime_parameter,
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
    pub smtp_sasl: unnamed_15,
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
pub struct unnamed_15 {
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
    pub sec_data: unnamed_16,
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
pub union unnamed_16 {
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
    pub ft_data: unnamed_17,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_17 {
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
    pub imap_sasl: unnamed_18,
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
pub struct unnamed_18 {
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
// backups
// attachments of 25 mb brutto should work on the majority of providers
// (brutto examples: web.de=50, 1&1=40, t-online.de=32, gmail=25, posteo=50, yahoo=25, all-inkl=100).
// as an upper limit, we double the size; the core won't send messages larger than this
// to get the netto sizes, we substract 1 mb header-overhead and the base64-overhead.
// some defaults
pub type dc_e2ee_helper_t = _dc_e2ee_helper;
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
pub type dc_mimefactory_t = _dc_mimefactory;
#[no_mangle]
pub unsafe extern "C" fn dc_mimefactory_init(
    mut factory: *mut dc_mimefactory_t,
    mut context: *mut dc_context_t,
) {
    if factory.is_null() || context.is_null() {
        return;
    }
    memset(
        factory as *mut libc::c_void,
        0i32,
        ::std::mem::size_of::<dc_mimefactory_t>() as libc::c_ulong,
    );
    (*factory).context = context;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimefactory_empty(mut factory: *mut dc_mimefactory_t) {
    if factory.is_null() {
        return;
    }
    free((*factory).from_addr as *mut libc::c_void);
    (*factory).from_addr = 0 as *mut libc::c_char;
    free((*factory).from_displayname as *mut libc::c_void);
    (*factory).from_displayname = 0 as *mut libc::c_char;
    free((*factory).selfstatus as *mut libc::c_void);
    (*factory).selfstatus = 0 as *mut libc::c_char;
    free((*factory).rfc724_mid as *mut libc::c_void);
    (*factory).rfc724_mid = 0 as *mut libc::c_char;
    if !(*factory).recipients_names.is_null() {
        clist_free_content((*factory).recipients_names);
        clist_free((*factory).recipients_names);
        (*factory).recipients_names = 0 as *mut clist
    }
    if !(*factory).recipients_addr.is_null() {
        clist_free_content((*factory).recipients_addr);
        clist_free((*factory).recipients_addr);
        (*factory).recipients_addr = 0 as *mut clist
    }
    dc_msg_unref((*factory).msg);
    (*factory).msg = 0 as *mut dc_msg_t;
    dc_chat_unref((*factory).chat);
    (*factory).chat = 0 as *mut dc_chat_t;
    free((*factory).in_reply_to as *mut libc::c_void);
    (*factory).in_reply_to = 0 as *mut libc::c_char;
    free((*factory).references as *mut libc::c_void);
    (*factory).references = 0 as *mut libc::c_char;
    if !(*factory).out.is_null() {
        mmap_string_free((*factory).out);
        (*factory).out = 0 as *mut MMAPString
    }
    (*factory).out_encrypted = 0i32;
    (*factory).loaded = DC_MF_NOTHING_LOADED;
    free((*factory).error as *mut libc::c_void);
    (*factory).error = 0 as *mut libc::c_char;
    (*factory).timestamp = 0i32 as time_t;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimefactory_load_msg(
    mut factory: *mut dc_mimefactory_t,
    mut msg_id: uint32_t,
) -> libc::c_int {
    let mut context: *mut dc_context_t = 0 as *mut dc_context_t;
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(factory.is_null()
        || msg_id <= 9i32 as libc::c_uint
        || (*factory).context.is_null()
        || !(*factory).msg.is_null())
    {
        /*call empty() before */
        context = (*factory).context;
        (*factory).recipients_names = clist_new();
        (*factory).recipients_addr = clist_new();
        (*factory).msg = dc_msg_new_untyped(context);
        (*factory).chat = dc_chat_new(context);
        if 0 != dc_msg_load_from_db((*factory).msg, context, msg_id)
            && 0 != dc_chat_load_from_db((*factory).chat, (*(*factory).msg).chat_id)
        {
            load_from(factory);
            (*factory).req_mdn = 0i32;
            if 0 != dc_chat_is_self_talk((*factory).chat) {
                clist_insert_after(
                    (*factory).recipients_names,
                    (*(*factory).recipients_names).last,
                    dc_strdup_keep_null((*factory).from_displayname) as *mut libc::c_void,
                );
                clist_insert_after(
                    (*factory).recipients_addr,
                    (*(*factory).recipients_addr).last,
                    dc_strdup((*factory).from_addr) as *mut libc::c_void,
                );
            } else {
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"SELECT c.authname, c.addr  FROM chats_contacts cc  LEFT JOIN contacts c ON cc.contact_id=c.id  WHERE cc.chat_id=? AND cc.contact_id>9;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, (*(*factory).msg).chat_id as libc::c_int);
                while sqlite3_step(stmt) == 100i32 {
                    let mut authname: *const libc::c_char =
                        sqlite3_column_text(stmt, 0i32) as *const libc::c_char;
                    let mut addr: *const libc::c_char =
                        sqlite3_column_text(stmt, 1i32) as *const libc::c_char;
                    if clist_search_string_nocase((*factory).recipients_addr, addr) == 0i32 {
                        clist_insert_after(
                            (*factory).recipients_names,
                            (*(*factory).recipients_names).last,
                            (if !authname.is_null() && 0 != *authname.offset(0isize) as libc::c_int
                            {
                                dc_strdup(authname)
                            } else {
                                0 as *mut libc::c_char
                            }) as *mut libc::c_void,
                        );
                        clist_insert_after(
                            (*factory).recipients_addr,
                            (*(*factory).recipients_addr).last,
                            dc_strdup(addr) as *mut libc::c_void,
                        );
                    }
                }
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                let mut command: libc::c_int =
                    dc_param_get_int((*(*factory).msg).param, 'S' as i32, 0i32);
                if command == 5i32 {
                    let mut email_to_remove: *mut libc::c_char = dc_param_get(
                        (*(*factory).msg).param,
                        'E' as i32,
                        0 as *const libc::c_char,
                    );
                    let mut self_addr: *mut libc::c_char = dc_sqlite3_get_config(
                        (*context).sql,
                        b"configured_addr\x00" as *const u8 as *const libc::c_char,
                        b"\x00" as *const u8 as *const libc::c_char,
                    );
                    if !email_to_remove.is_null() && strcasecmp(email_to_remove, self_addr) != 0i32
                    {
                        if clist_search_string_nocase((*factory).recipients_addr, email_to_remove)
                            == 0i32
                        {
                            clist_insert_after(
                                (*factory).recipients_names,
                                (*(*factory).recipients_names).last,
                                0 as *mut libc::c_void,
                            );
                            clist_insert_after(
                                (*factory).recipients_addr,
                                (*(*factory).recipients_addr).last,
                                email_to_remove as *mut libc::c_void,
                            );
                        }
                    }
                    free(self_addr as *mut libc::c_void);
                }
                if command != 6i32
                    && command != 7i32
                    && 0 != dc_sqlite3_get_config_int(
                        (*context).sql,
                        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                        1i32,
                    )
                {
                    (*factory).req_mdn = 1i32
                }
            }
            stmt = dc_sqlite3_prepare(
                (*context).sql,
                b"SELECT mime_in_reply_to, mime_references FROM msgs WHERE id=?\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_int(stmt, 1i32, (*(*factory).msg).id as libc::c_int);
            if sqlite3_step(stmt) == 100i32 {
                (*factory).in_reply_to =
                    dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char);
                (*factory).references =
                    dc_strdup(sqlite3_column_text(stmt, 1i32) as *const libc::c_char)
            }
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            success = 1i32;
            (*factory).loaded = DC_MF_MSG_LOADED;
            (*factory).timestamp = (*(*factory).msg).timestamp_sort;
            (*factory).rfc724_mid = dc_strdup((*(*factory).msg).rfc724_mid)
        }
        if 0 != success {
            (*factory).increation = dc_msg_is_increation((*factory).msg)
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
unsafe extern "C" fn load_from(mut factory: *mut dc_mimefactory_t) {
    (*factory).from_addr = dc_sqlite3_get_config(
        (*(*factory).context).sql,
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    (*factory).from_displayname = dc_sqlite3_get_config(
        (*(*factory).context).sql,
        b"displayname\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    (*factory).selfstatus = dc_sqlite3_get_config(
        (*(*factory).context).sql,
        b"selfstatus\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    if (*factory).selfstatus.is_null() {
        (*factory).selfstatus = dc_stock_str((*factory).context, 13i32)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimefactory_load_mdn(
    mut factory: *mut dc_mimefactory_t,
    mut msg_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !factory.is_null() {
        (*factory).recipients_names = clist_new();
        (*factory).recipients_addr = clist_new();
        (*factory).msg = dc_msg_new_untyped((*factory).context);
        if !(0
            == dc_sqlite3_get_config_int(
                (*(*factory).context).sql,
                b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                1i32,
            ))
        {
            /* MDNs not enabled - check this is late, in the job. the use may have changed its choice while offline ... */
            contact = dc_contact_new((*factory).context);
            if !(0 == dc_msg_load_from_db((*factory).msg, (*factory).context, msg_id)
                || 0 == dc_contact_load_from_db(
                    contact,
                    (*(*factory).context).sql,
                    (*(*factory).msg).from_id,
                ))
            {
                if !(0 != (*contact).blocked || (*(*factory).msg).chat_id <= 9i32 as libc::c_uint) {
                    /* Do not send MDNs trash etc.; chats.blocked is already checked by the caller in dc_markseen_msgs() */
                    if !((*(*factory).msg).from_id <= 9i32 as libc::c_uint) {
                        clist_insert_after(
                            (*factory).recipients_names,
                            (*(*factory).recipients_names).last,
                            (if !(*contact).authname.is_null()
                                && 0 != *(*contact).authname.offset(0isize) as libc::c_int
                            {
                                dc_strdup((*contact).authname)
                            } else {
                                0 as *mut libc::c_char
                            }) as *mut libc::c_void,
                        );
                        clist_insert_after(
                            (*factory).recipients_addr,
                            (*(*factory).recipients_addr).last,
                            dc_strdup((*contact).addr) as *mut libc::c_void,
                        );
                        load_from(factory);
                        (*factory).timestamp = dc_create_smeared_timestamp((*factory).context);
                        (*factory).rfc724_mid = dc_create_outgoing_rfc724_mid(
                            0 as *const libc::c_char,
                            (*factory).from_addr,
                        );
                        success = 1i32;
                        (*factory).loaded = DC_MF_MDN_LOADED
                    }
                }
            }
        }
    }
    dc_contact_unref(contact);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimefactory_render(mut factory: *mut dc_mimefactory_t) -> libc::c_int {
    let mut subject: *mut mailimf_subject = 0 as *mut mailimf_subject;
    let mut current_block: u64;
    let mut imf_fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut message: *mut mailmime = 0 as *mut mailmime;
    let mut message_text: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut message_text2: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut subject_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut afwd_email: libc::c_int = 0i32;
    let mut col: libc::c_int = 0i32;
    let mut success: libc::c_int = 0i32;
    let mut parts: libc::c_int = 0i32;
    let mut e2ee_guaranteed: libc::c_int = 0i32;
    let mut min_verified: libc::c_int = 0i32;
    // 1=add Autocrypt-header (needed eg. for handshaking), 2=no Autocrypte-header (used for MDN)
    let mut force_plaintext: libc::c_int = 0i32;
    let mut do_gossip: libc::c_int = 0i32;
    let mut grpimage: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut e2ee_helper: dc_e2ee_helper_t = _dc_e2ee_helper {
        encryption_successfull: 0,
        cdata_to_free: 0 as *mut libc::c_void,
        encrypted: 0,
        signatures: 0 as *mut dc_hash_t,
        gossipped_addr: 0 as *mut dc_hash_t,
    };
    memset(
        &mut e2ee_helper as *mut dc_e2ee_helper_t as *mut libc::c_void,
        0i32,
        ::std::mem::size_of::<dc_e2ee_helper_t>() as libc::c_ulong,
    );
    if factory.is_null()
        || (*factory).loaded as libc::c_uint == DC_MF_NOTHING_LOADED as libc::c_int as libc::c_uint
        || !(*factory).out.is_null()
    {
        /*call empty() before*/
        set_error(
            factory,
            b"Invalid use of mimefactory-object.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        let mut from: *mut mailimf_mailbox_list = mailimf_mailbox_list_new_empty();
        mailimf_mailbox_list_add(
            from,
            mailimf_mailbox_new(
                if !(*factory).from_displayname.is_null() {
                    dc_encode_header_words((*factory).from_displayname)
                } else {
                    0 as *mut libc::c_char
                },
                dc_strdup((*factory).from_addr),
            ),
        );
        let mut to: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
        if !(*factory).recipients_names.is_null()
            && !(*factory).recipients_addr.is_null()
            && (*(*factory).recipients_addr).count > 0i32
        {
            let mut iter1: *mut clistiter = 0 as *mut clistiter;
            let mut iter2: *mut clistiter = 0 as *mut clistiter;
            to = mailimf_address_list_new_empty();
            iter1 = (*(*factory).recipients_names).first;
            iter2 = (*(*factory).recipients_addr).first;
            while !iter1.is_null() && !iter2.is_null() {
                let mut name: *const libc::c_char = (if !iter1.is_null() {
                    (*iter1).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char;
                let mut addr: *const libc::c_char = (if !iter2.is_null() {
                    (*iter2).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char;
                mailimf_address_list_add(
                    to,
                    mailimf_address_new(
                        MAILIMF_ADDRESS_MAILBOX as libc::c_int,
                        mailimf_mailbox_new(
                            if !name.is_null() {
                                dc_encode_header_words(name)
                            } else {
                                0 as *mut libc::c_char
                            },
                            dc_strdup(addr),
                        ),
                        0 as *mut mailimf_group,
                    ),
                );
                iter1 = if !iter1.is_null() {
                    (*iter1).next
                } else {
                    0 as *mut clistcell_s
                };
                iter2 = if !iter2.is_null() {
                    (*iter2).next
                } else {
                    0 as *mut clistcell_s
                }
            }
        }
        let mut references_list: *mut clist = 0 as *mut clist;
        if !(*factory).references.is_null()
            && 0 != *(*factory).references.offset(0isize) as libc::c_int
        {
            references_list = dc_str_to_clist(
                (*factory).references,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
        let mut in_reply_to_list: *mut clist = 0 as *mut clist;
        if !(*factory).in_reply_to.is_null()
            && 0 != *(*factory).in_reply_to.offset(0isize) as libc::c_int
        {
            in_reply_to_list = dc_str_to_clist(
                (*factory).in_reply_to,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
        imf_fields = mailimf_fields_new_with_data_all(
            mailimf_get_date((*factory).timestamp),
            from,
            0 as *mut mailimf_mailbox,
            0 as *mut mailimf_address_list,
            to,
            0 as *mut mailimf_address_list,
            0 as *mut mailimf_address_list,
            dc_strdup((*factory).rfc724_mid),
            in_reply_to_list,
            references_list,
            0 as *mut libc::c_char,
        );
        mailimf_fields_add(
            imf_fields,
            mailimf_field_new_custom(
                strdup(b"X-Mailer\x00" as *const u8 as *const libc::c_char),
                dc_mprintf(
                    b"Delta Chat Core %s%s%s\x00" as *const u8 as *const libc::c_char,
                    b"0.42.0\x00" as *const u8 as *const libc::c_char,
                    if !(*(*factory).context).os_name.is_null() {
                        b"/\x00" as *const u8 as *const libc::c_char
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    if !(*(*factory).context).os_name.is_null() {
                        (*(*factory).context).os_name
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                ),
            ),
        );
        mailimf_fields_add(
            imf_fields,
            mailimf_field_new_custom(
                strdup(b"Chat-Version\x00" as *const u8 as *const libc::c_char),
                strdup(b"1.0\x00" as *const u8 as *const libc::c_char),
            ),
        );
        if 0 != (*factory).req_mdn {
            mailimf_fields_add(
                imf_fields,
                mailimf_field_new_custom(
                    strdup(
                        b"Chat-Disposition-Notification-To\x00" as *const u8 as *const libc::c_char,
                    ),
                    strdup((*factory).from_addr),
                ),
            );
        }
        message = mailmime_new_message_data(0 as *mut mailmime);
        mailmime_set_imf_fields(message, imf_fields);
        if (*factory).loaded as libc::c_uint == DC_MF_MSG_LOADED as libc::c_int as libc::c_uint {
            /* Render a normal message
             *********************************************************************/
            let mut chat: *mut dc_chat_t = (*factory).chat;
            let mut msg: *mut dc_msg_t = (*factory).msg;
            let mut meta_part: *mut mailmime = 0 as *mut mailmime;
            let mut placeholdertext: *mut libc::c_char = 0 as *mut libc::c_char;
            if (*chat).type_0 == 130i32 {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Verified\x00" as *const u8 as *const libc::c_char),
                        strdup(b"1\x00" as *const u8 as *const libc::c_char),
                    ),
                );
                force_plaintext = 0i32;
                e2ee_guaranteed = 1i32;
                min_verified = 2i32
            } else {
                force_plaintext = dc_param_get_int((*(*factory).msg).param, 'u' as i32, 0i32);
                if force_plaintext == 0i32 {
                    e2ee_guaranteed = dc_param_get_int((*(*factory).msg).param, 'c' as i32, 0i32)
                }
            }
            if (*chat).gossiped_timestamp == 0i32 as libc::c_long
                || ((*chat).gossiped_timestamp + (2i32 * 24i32 * 60i32 * 60i32) as libc::c_long)
                    < time(0 as *mut time_t)
            {
                do_gossip = 1i32
            }
            /* build header etc. */
            let mut command: libc::c_int = dc_param_get_int((*msg).param, 'S' as i32, 0i32);
            if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Group-ID\x00" as *const u8 as *const libc::c_char),
                        dc_strdup((*chat).grpid),
                    ),
                );
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Group-Name\x00" as *const u8 as *const libc::c_char),
                        dc_encode_header_words((*chat).name),
                    ),
                );
                if command == 5i32 {
                    let mut email_to_remove: *mut libc::c_char =
                        dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                    if !email_to_remove.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Chat-Group-Member-Removed\x00" as *const u8
                                        as *const libc::c_char,
                                ),
                                email_to_remove,
                            ),
                        );
                    }
                } else if command == 4i32 {
                    do_gossip = 1i32;
                    let mut email_to_add: *mut libc::c_char =
                        dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                    if !email_to_add.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Chat-Group-Member-Added\x00" as *const u8
                                        as *const libc::c_char,
                                ),
                                email_to_add,
                            ),
                        );
                        grpimage = dc_param_get((*chat).param, 'i' as i32, 0 as *const libc::c_char)
                    }
                    if 0 != dc_param_get_int((*msg).param, 'F' as i32, 0i32) & 0x1i32 {
                        dc_log_info(
                            (*msg).context,
                            0i32,
                            b"sending secure-join message \'%s\' >>>>>>>>>>>>>>>>>>>>>>>>>\x00"
                                as *const u8 as *const libc::c_char,
                            b"vg-member-added\x00" as *const u8 as *const libc::c_char,
                        );
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(b"Secure-Join\x00" as *const u8 as *const libc::c_char),
                                strdup(b"vg-member-added\x00" as *const u8 as *const libc::c_char),
                            ),
                        );
                    }
                } else if command == 2i32 {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(
                                b"Chat-Group-Name-Changed\x00" as *const u8 as *const libc::c_char,
                            ),
                            dc_param_get(
                                (*msg).param,
                                'E' as i32,
                                b"\x00" as *const u8 as *const libc::c_char,
                            ),
                        ),
                    );
                } else if command == 3i32 {
                    grpimage = dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                    if grpimage.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char),
                                dc_strdup(b"0\x00" as *const u8 as *const libc::c_char),
                            ),
                        );
                    }
                }
            }
            if command == 8i32 {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Content\x00" as *const u8 as *const libc::c_char),
                        strdup(
                            b"location-streaming-enabled\x00" as *const u8 as *const libc::c_char,
                        ),
                    ),
                );
            }
            if command == 6i32 {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Autocrypt-Setup-Message\x00" as *const u8 as *const libc::c_char),
                        strdup(b"v1\x00" as *const u8 as *const libc::c_char),
                    ),
                );
                placeholdertext = dc_stock_str((*factory).context, 43i32)
            }
            if command == 7i32 {
                let mut step: *mut libc::c_char =
                    dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                if !step.is_null() {
                    dc_log_info(
                        (*msg).context,
                        0i32,
                        b"sending secure-join message \'%s\' >>>>>>>>>>>>>>>>>>>>>>>>>\x00"
                            as *const u8 as *const libc::c_char,
                        step,
                    );
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Secure-Join\x00" as *const u8 as *const libc::c_char),
                            step,
                        ),
                    );
                    let mut param2: *mut libc::c_char =
                        dc_param_get((*msg).param, 'F' as i32, 0 as *const libc::c_char);
                    if !param2.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                if strcmp(
                                    step,
                                    b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char,
                                ) == 0i32
                                    || strcmp(
                                        step,
                                        b"vc-request-with-auth\x00" as *const u8
                                            as *const libc::c_char,
                                    ) == 0i32
                                {
                                    strdup(
                                        b"Secure-Join-Auth\x00" as *const u8 as *const libc::c_char,
                                    )
                                } else {
                                    strdup(
                                        b"Secure-Join-Invitenumber\x00" as *const u8
                                            as *const libc::c_char,
                                    )
                                },
                                param2,
                            ),
                        );
                    }
                    let mut fingerprint: *mut libc::c_char =
                        dc_param_get((*msg).param, 'G' as i32, 0 as *const libc::c_char);
                    if !fingerprint.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Secure-Join-Fingerprint\x00" as *const u8
                                        as *const libc::c_char,
                                ),
                                fingerprint,
                            ),
                        );
                    }
                    let mut grpid: *mut libc::c_char =
                        dc_param_get((*msg).param, 'H' as i32, 0 as *const libc::c_char);
                    if !grpid.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Secure-Join-Group\x00" as *const u8 as *const libc::c_char,
                                ),
                                grpid,
                            ),
                        );
                    }
                }
            }
            if !grpimage.is_null() {
                let mut meta: *mut dc_msg_t = dc_msg_new_untyped((*factory).context);
                (*meta).type_0 = 20i32;
                dc_param_set((*meta).param, 'f' as i32, grpimage);
                let mut filename_as_sent: *mut libc::c_char = 0 as *mut libc::c_char;
                meta_part = build_body_file(
                    meta,
                    b"group-image\x00" as *const u8 as *const libc::c_char,
                    &mut filename_as_sent,
                );
                if !meta_part.is_null() {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char),
                            filename_as_sent,
                        ),
                    );
                }
                dc_msg_unref(meta);
            }
            if (*msg).type_0 == 41i32 || (*msg).type_0 == 40i32 || (*msg).type_0 == 50i32 {
                if (*msg).type_0 == 41i32 {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Chat-Voice-Message\x00" as *const u8 as *const libc::c_char),
                            strdup(b"1\x00" as *const u8 as *const libc::c_char),
                        ),
                    );
                }
                let mut duration_ms: libc::c_int = dc_param_get_int((*msg).param, 'd' as i32, 0i32);
                if duration_ms > 0i32 {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Chat-Duration\x00" as *const u8 as *const libc::c_char),
                            dc_mprintf(
                                b"%i\x00" as *const u8 as *const libc::c_char,
                                duration_ms as libc::c_int,
                            ),
                        ),
                    );
                }
            }
            afwd_email = dc_param_exists((*msg).param, 'a' as i32);
            let mut fwdhint: *mut libc::c_char = 0 as *mut libc::c_char;
            if 0 != afwd_email {
                fwdhint = dc_strdup(
                    b"---------- Forwarded message ----------\r\nFrom: Delta Chat\r\n\r\n\x00"
                        as *const u8 as *const libc::c_char,
                )
            }
            let mut final_text: *const libc::c_char = 0 as *const libc::c_char;
            if !placeholdertext.is_null() {
                final_text = placeholdertext
            } else if !(*msg).text.is_null() && 0 != *(*msg).text.offset(0isize) as libc::c_int {
                final_text = (*msg).text
            }
            let mut footer: *mut libc::c_char = (*factory).selfstatus;
            message_text = dc_mprintf(
                b"%s%s%s%s%s\x00" as *const u8 as *const libc::c_char,
                if !fwdhint.is_null() {
                    fwdhint
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !final_text.is_null() {
                    final_text
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !final_text.is_null()
                    && !footer.is_null()
                    && 0 != *footer.offset(0isize) as libc::c_int
                {
                    b"\r\n\r\n\x00" as *const u8 as *const libc::c_char
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !footer.is_null() && 0 != *footer.offset(0isize) as libc::c_int {
                    b"-- \r\n\x00" as *const u8 as *const libc::c_char
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !footer.is_null() && 0 != *footer.offset(0isize) as libc::c_int {
                    footer
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
            );
            let mut text_part: *mut mailmime = build_body_text(message_text);
            mailmime_smart_add_part(message, text_part);
            parts += 1;
            free(fwdhint as *mut libc::c_void);
            free(placeholdertext as *mut libc::c_void);
            /* add attachment part */
            if (*msg).type_0 == 20i32
                || (*msg).type_0 == 21i32
                || (*msg).type_0 == 40i32
                || (*msg).type_0 == 41i32
                || (*msg).type_0 == 50i32
                || (*msg).type_0 == 60i32
            {
                if 0 == is_file_size_okay(msg) {
                    let mut error: *mut libc::c_char = dc_mprintf(
                        b"Message exceeds the recommended %i MB.\x00" as *const u8
                            as *const libc::c_char,
                        24i32 * 1024i32 * 1024i32 / 4i32 * 3i32 / 1000i32 / 1000i32,
                    );
                    set_error(factory, error);
                    free(error as *mut libc::c_void);
                    current_block = 11328123142868406523;
                } else {
                    let mut file_part: *mut mailmime =
                        build_body_file(msg, 0 as *const libc::c_char, 0 as *mut *mut libc::c_char);
                    if !file_part.is_null() {
                        mailmime_smart_add_part(message, file_part);
                        parts += 1
                    }
                    current_block = 13000670339742628194;
                }
            } else {
                current_block = 13000670339742628194;
            }
            match current_block {
                11328123142868406523 => {}
                _ => {
                    if parts == 0i32 {
                        set_error(
                            factory,
                            b"Empty message.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 11328123142868406523;
                    } else {
                        if !meta_part.is_null() {
                            mailmime_smart_add_part(message, meta_part);
                            parts += 1
                        }
                        if 0 != dc_is_sending_locations_to_chat((*msg).context, (*msg).chat_id) {
                            let mut last_added_location_id: uint32_t = 0i32 as uint32_t;
                            let mut kml_file: *mut libc::c_char = dc_get_location_kml(
                                (*msg).context,
                                (*msg).chat_id,
                                &mut last_added_location_id,
                            );
                            if !kml_file.is_null() {
                                let mut content_type: *mut mailmime_content =
                                    mailmime_content_new_with_str(
                                        b"application/vnd.google-earth.kml+xml\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                let mut mime_fields: *mut mailmime_fields =
                                    mailmime_fields_new_filename(
                                        MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                                        dc_strdup(
                                            b"location.kml\x00" as *const u8 as *const libc::c_char,
                                        ),
                                        MAILMIME_MECHANISM_8BIT as libc::c_int,
                                    );
                                let mut kml_mime_part: *mut mailmime =
                                    mailmime_new_empty(content_type, mime_fields);
                                mailmime_set_body_text(kml_mime_part, kml_file, strlen(kml_file));
                                mailmime_smart_add_part(message, kml_mime_part);
                                parts += 1;
                                (*factory).out_last_added_location_id = last_added_location_id
                            }
                        }
                        current_block = 9952640327414195044;
                    }
                }
            }
        } else if (*factory).loaded as libc::c_uint
            == DC_MF_MDN_LOADED as libc::c_int as libc::c_uint
        {
            let mut multipart: *mut mailmime =
                mailmime_multiple_new(b"multipart/report\x00" as *const u8 as *const libc::c_char);
            let mut content: *mut mailmime_content = (*multipart).mm_content_type;
            clist_insert_after(
                (*content).ct_parameters,
                (*(*content).ct_parameters).last,
                mailmime_param_new_with_data(
                    b"report-type\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                    b"disposition-notification\x00" as *const u8 as *const libc::c_char
                        as *mut libc::c_char,
                ) as *mut libc::c_void,
            );
            mailmime_add_part(message, multipart);
            let mut p1: *mut libc::c_char = 0 as *mut libc::c_char;
            let mut p2: *mut libc::c_char = 0 as *mut libc::c_char;
            if 0 != dc_param_get_int((*(*factory).msg).param, 'c' as i32, 0i32) {
                p1 = dc_stock_str((*factory).context, 24i32)
            } else {
                p1 = dc_msg_get_summarytext((*factory).msg, 32i32)
            }
            p2 = dc_stock_str_repl_string((*factory).context, 32i32, p1);
            message_text = dc_mprintf(b"%s\r\n\x00" as *const u8 as *const libc::c_char, p2);
            free(p2 as *mut libc::c_void);
            free(p1 as *mut libc::c_void);
            let mut human_mime_part: *mut mailmime = build_body_text(message_text);
            mailmime_add_part(multipart, human_mime_part);
            message_text2 =
                dc_mprintf(b"Reporting-UA: Delta Chat %s\r\nOriginal-Recipient: rfc822;%s\r\nFinal-Recipient: rfc822;%s\r\nOriginal-Message-ID: <%s>\r\nDisposition: manual-action/MDN-sent-automatically; displayed\r\n\x00"
                               as *const u8 as *const libc::c_char,
                           b"0.42.0\x00" as *const u8 as *const libc::c_char,
                           (*factory).from_addr, (*factory).from_addr,
                           (*(*factory).msg).rfc724_mid);
            let mut content_type_0: *mut mailmime_content = mailmime_content_new_with_str(
                b"message/disposition-notification\x00" as *const u8 as *const libc::c_char,
            );
            let mut mime_fields_0: *mut mailmime_fields =
                mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
            let mut mach_mime_part: *mut mailmime =
                mailmime_new_empty(content_type_0, mime_fields_0);
            mailmime_set_body_text(mach_mime_part, message_text2, strlen(message_text2));
            mailmime_add_part(multipart, mach_mime_part);
            force_plaintext = 2i32;
            current_block = 9952640327414195044;
        } else {
            set_error(
                factory,
                b"No message loaded.\x00" as *const u8 as *const libc::c_char,
            );
            current_block = 11328123142868406523;
        }
        match current_block {
            11328123142868406523 => {}
            _ => {
                if (*factory).loaded as libc::c_uint
                    == DC_MF_MDN_LOADED as libc::c_int as libc::c_uint
                {
                    let mut e: *mut libc::c_char = dc_stock_str((*factory).context, 31i32);
                    subject_str =
                        dc_mprintf(b"Chat: %s\x00" as *const u8 as *const libc::c_char, e);
                    free(e as *mut libc::c_void);
                } else {
                    subject_str = get_subject((*factory).chat, (*factory).msg, afwd_email)
                }
                subject = mailimf_subject_new(dc_encode_header_words(subject_str));
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new(
                        MAILIMF_FIELD_SUBJECT as libc::c_int,
                        0 as *mut mailimf_return,
                        0 as *mut mailimf_orig_date,
                        0 as *mut mailimf_from,
                        0 as *mut mailimf_sender,
                        0 as *mut mailimf_to,
                        0 as *mut mailimf_cc,
                        0 as *mut mailimf_bcc,
                        0 as *mut mailimf_message_id,
                        0 as *mut mailimf_orig_date,
                        0 as *mut mailimf_from,
                        0 as *mut mailimf_sender,
                        0 as *mut mailimf_reply_to,
                        0 as *mut mailimf_to,
                        0 as *mut mailimf_cc,
                        0 as *mut mailimf_bcc,
                        0 as *mut mailimf_message_id,
                        0 as *mut mailimf_in_reply_to,
                        0 as *mut mailimf_references,
                        subject,
                        0 as *mut mailimf_comments,
                        0 as *mut mailimf_keywords,
                        0 as *mut mailimf_optional_field,
                    ),
                );
                if force_plaintext != 2i32 {
                    dc_e2ee_encrypt(
                        (*factory).context,
                        (*factory).recipients_addr,
                        force_plaintext,
                        e2ee_guaranteed,
                        min_verified,
                        do_gossip,
                        message,
                        &mut e2ee_helper,
                    );
                }
                if 0 != e2ee_helper.encryption_successfull {
                    (*factory).out_encrypted = 1i32;
                    if 0 != do_gossip {
                        (*factory).out_gossiped = 1i32
                    }
                }
                (*factory).out = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
                mailmime_write_mem((*factory).out, &mut col, message);
                success = 1i32
            }
        }
    }
    if !message.is_null() {
        mailmime_free(message);
    }
    dc_e2ee_thanks(&mut e2ee_helper);
    free(message_text as *mut libc::c_void);
    free(message_text2 as *mut libc::c_void);
    free(subject_str as *mut libc::c_void);
    free(grpimage as *mut libc::c_void);
    return success;
}
unsafe extern "C" fn get_subject(
    mut chat: *const dc_chat_t,
    mut msg: *const dc_msg_t,
    mut afwd_email: libc::c_int,
) -> *mut libc::c_char {
    let mut context: *mut dc_context_t = if !chat.is_null() {
        (*chat).context
    } else {
        0 as *mut dc_context_t
    };
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut raw_subject: *mut libc::c_char =
        dc_msg_get_summarytext_by_raw((*msg).type_0, (*msg).text, (*msg).param, 32i32, context);
    let mut fwd: *const libc::c_char = if 0 != afwd_email {
        b"Fwd: \x00" as *const u8 as *const libc::c_char
    } else {
        b"\x00" as *const u8 as *const libc::c_char
    };
    if dc_param_get_int((*msg).param, 'S' as i32, 0i32) == 6i32 {
        ret = dc_stock_str(context, 42i32)
    } else if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
        ret = dc_mprintf(
            b"Chat: %s: %s%s\x00" as *const u8 as *const libc::c_char,
            (*chat).name,
            fwd,
            raw_subject,
        )
    } else {
        ret = dc_mprintf(
            b"Chat: %s%s\x00" as *const u8 as *const libc::c_char,
            fwd,
            raw_subject,
        )
    }
    free(raw_subject as *mut libc::c_void);
    return ret;
}
unsafe extern "C" fn set_error(mut factory: *mut dc_mimefactory_t, mut text: *const libc::c_char) {
    if factory.is_null() {
        return;
    }
    free((*factory).error as *mut libc::c_void);
    (*factory).error = dc_strdup_keep_null(text);
}
unsafe extern "C" fn build_body_text(mut text: *mut libc::c_char) -> *mut mailmime {
    let mut mime_fields: *mut mailmime_fields = 0 as *mut mailmime_fields;
    let mut message_part: *mut mailmime = 0 as *mut mailmime;
    let mut content: *mut mailmime_content = 0 as *mut mailmime_content;
    content = mailmime_content_new_with_str(b"text/plain\x00" as *const u8 as *const libc::c_char);
    clist_insert_after(
        (*content).ct_parameters,
        (*(*content).ct_parameters).last,
        mailmime_param_new_with_data(
            b"charset\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            b"utf-8\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        ) as *mut libc::c_void,
    );
    mime_fields = mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
    message_part = mailmime_new_empty(content, mime_fields);
    mailmime_set_body_text(message_part, text, strlen(text));
    return message_part;
}
unsafe extern "C" fn build_body_file(
    mut msg: *const dc_msg_t,
    mut base_name: *const libc::c_char,
    mut ret_file_name_as_sent: *mut *mut libc::c_char,
) -> *mut mailmime {
    let mut needs_ext: libc::c_int = 0;
    let mut mime_fields: *mut mailmime_fields = 0 as *mut mailmime_fields;
    let mut mime_sub: *mut mailmime = 0 as *mut mailmime;
    let mut content: *mut mailmime_content = 0 as *mut mailmime_content;
    let mut pathNfilename: *mut libc::c_char =
        dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
    let mut mimetype: *mut libc::c_char =
        dc_param_get((*msg).param, 'm' as i32, 0 as *const libc::c_char);
    let mut suffix: *mut libc::c_char = dc_get_filesuffix_lc(pathNfilename);
    let mut filename_to_send: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filename_encoded: *mut libc::c_char = 0 as *mut libc::c_char;
    if !pathNfilename.is_null() {
        if (*msg).type_0 == 41i32 {
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
                localtime(&(*msg).timestamp_sort) as *const libc::c_void,
                ::std::mem::size_of::<tm>() as libc::c_ulong,
            );
            filename_to_send = dc_mprintf(
                b"voice-message_%04i-%02i-%02i_%02i-%02i-%02i.%s\x00" as *const u8
                    as *const libc::c_char,
                wanted_struct.tm_year as libc::c_int + 1900i32,
                wanted_struct.tm_mon as libc::c_int + 1i32,
                wanted_struct.tm_mday as libc::c_int,
                wanted_struct.tm_hour as libc::c_int,
                wanted_struct.tm_min as libc::c_int,
                wanted_struct.tm_sec as libc::c_int,
                if !suffix.is_null() {
                    suffix
                } else {
                    b"dat\x00" as *const u8 as *const libc::c_char
                },
            )
        } else if (*msg).type_0 == 40i32 {
            filename_to_send = dc_get_filename(pathNfilename)
        } else if (*msg).type_0 == 20i32 || (*msg).type_0 == 21i32 {
            if base_name.is_null() {
                base_name = b"image\x00" as *const u8 as *const libc::c_char
            }
            filename_to_send = dc_mprintf(
                b"%s.%s\x00" as *const u8 as *const libc::c_char,
                base_name,
                if !suffix.is_null() {
                    suffix
                } else {
                    b"dat\x00" as *const u8 as *const libc::c_char
                },
            )
        } else if (*msg).type_0 == 50i32 {
            filename_to_send = dc_mprintf(
                b"video.%s\x00" as *const u8 as *const libc::c_char,
                if !suffix.is_null() {
                    suffix
                } else {
                    b"dat\x00" as *const u8 as *const libc::c_char
                },
            )
        } else {
            filename_to_send = dc_get_filename(pathNfilename)
        }
        if mimetype.is_null() {
            if suffix.is_null() {
                mimetype =
                    dc_strdup(b"application/octet-stream\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"png\x00" as *const u8 as *const libc::c_char) == 0i32 {
                mimetype = dc_strdup(b"image/png\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"jpg\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"jpeg\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"jpe\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                mimetype = dc_strdup(b"image/jpeg\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"gif\x00" as *const u8 as *const libc::c_char) == 0i32 {
                mimetype = dc_strdup(b"image/gif\x00" as *const u8 as *const libc::c_char)
            } else {
                mimetype =
                    dc_strdup(b"application/octet-stream\x00" as *const u8 as *const libc::c_char)
            }
        }
        if !mimetype.is_null() {
            /* create mime part, for Content-Disposition, see RFC 2183.
            `Content-Disposition: attachment` seems not to make a difference to `Content-Disposition: inline` at least on tested Thunderbird and Gma'l in 2017.
            But I've heard about problems with inline and outl'k, so we just use the attachment-type until we run into other problems ... */
            needs_ext = dc_needs_ext_header(filename_to_send);
            mime_fields = mailmime_fields_new_filename(
                MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                if 0 != needs_ext {
                    0 as *mut libc::c_char
                } else {
                    dc_strdup(filename_to_send)
                },
                MAILMIME_MECHANISM_BASE64 as libc::c_int,
            );
            if 0 != needs_ext {
                let mut cur1: *mut clistiter = (*(*mime_fields).fld_list).first;
                while !cur1.is_null() {
                    let mut field: *mut mailmime_field = (if !cur1.is_null() {
                        (*cur1).data
                    } else {
                        0 as *mut libc::c_void
                    })
                        as *mut mailmime_field;
                    if !field.is_null()
                        && (*field).fld_type == MAILMIME_FIELD_DISPOSITION as libc::c_int
                        && !(*field).fld_data.fld_disposition.is_null()
                    {
                        let mut file_disposition: *mut mailmime_disposition =
                            (*field).fld_data.fld_disposition;
                        if !file_disposition.is_null() {
                            let mut parm: *mut mailmime_disposition_parm =
                                mailmime_disposition_parm_new(
                                    MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int,
                                    0 as *mut libc::c_char,
                                    0 as *mut libc::c_char,
                                    0 as *mut libc::c_char,
                                    0 as *mut libc::c_char,
                                    0i32 as size_t,
                                    mailmime_parameter_new(
                                        strdup(
                                            b"filename*\x00" as *const u8 as *const libc::c_char,
                                        ),
                                        dc_encode_ext_header(filename_to_send),
                                    ),
                                );
                            if !parm.is_null() {
                                clist_insert_after(
                                    (*file_disposition).dsp_parms,
                                    (*(*file_disposition).dsp_parms).last,
                                    parm as *mut libc::c_void,
                                );
                            }
                        }
                        break;
                    } else {
                        cur1 = if !cur1.is_null() {
                            (*cur1).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                }
            }
            content = mailmime_content_new_with_str(mimetype);
            filename_encoded = dc_encode_header_words(filename_to_send);
            clist_insert_after(
                (*content).ct_parameters,
                (*(*content).ct_parameters).last,
                mailmime_param_new_with_data(
                    b"name\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                    filename_encoded,
                ) as *mut libc::c_void,
            );
            mime_sub = mailmime_new_empty(content, mime_fields);
            mailmime_set_body_file(mime_sub, dc_get_abs_path((*msg).context, pathNfilename));
            if !ret_file_name_as_sent.is_null() {
                *ret_file_name_as_sent = dc_strdup(filename_to_send)
            }
        }
    }
    free(pathNfilename as *mut libc::c_void);
    free(mimetype as *mut libc::c_void);
    free(filename_to_send as *mut libc::c_void);
    free(filename_encoded as *mut libc::c_void);
    free(suffix as *mut libc::c_void);
    return mime_sub;
}
/* ******************************************************************************
 * Render
 ******************************************************************************/
unsafe extern "C" fn is_file_size_okay(mut msg: *const dc_msg_t) -> libc::c_int {
    let mut file_size_okay: libc::c_int = 1i32;
    let mut pathNfilename: *mut libc::c_char =
        dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
    let mut bytes: uint64_t = dc_get_filebytes((*msg).context, pathNfilename);
    if bytes > (49i32 * 1024i32 * 1024i32 / 4i32 * 3i32) as libc::c_ulonglong {
        file_size_okay = 0i32
    }
    free(pathNfilename as *mut libc::c_void);
    return file_size_okay;
}

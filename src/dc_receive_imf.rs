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
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strstr(_: *const libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
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
    #[no_mangle]
    fn mmap_string_unref(str: *mut libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn sprintf(_: *mut libc::c_char, _: *const libc::c_char, _: ...) -> libc::c_int;
    #[no_mangle]
    fn mailimf_msg_id_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_free(mime: *mut mailmime);
    #[no_mangle]
    fn mailmime_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailmime,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_is_contact_in_chat(
        _: *mut dc_context_t,
        chat_id: uint32_t,
        contact_id: uint32_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_lookup_contact_id_by_addr(_: *mut dc_context_t, addr: *const libc::c_char) -> uint32_t;
    #[no_mangle]
    fn dc_get_contact(_: *mut dc_context_t, contact_id: uint32_t) -> *mut dc_contact_t;
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
    fn dc_array_add_ptr(_: *mut dc_array_t, _: *mut libc::c_void);
    #[no_mangle]
    fn dc_array_get_cnt(_: *const dc_array_t) -> size_t;
    #[no_mangle]
    fn dc_array_get_id(_: *const dc_array_t, index: size_t) -> uint32_t;
    #[no_mangle]
    fn dc_array_get_ptr(_: *const dc_array_t, index: size_t) -> *mut libc::c_void;
    #[no_mangle]
    fn dc_array_search_id(_: *const dc_array_t, needle: uint32_t, indx: *mut size_t)
        -> libc::c_int;
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
    fn sqlite3_mprintf(_: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn sqlite3_free(_: *mut libc::c_void);
    #[no_mangle]
    fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_int64(_: *mut sqlite3_stmt, _: libc::c_int, _: sqlite3_int64) -> libc::c_int;
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
    fn sqlite3_column_int64(_: *mut sqlite3_stmt, iCol: libc::c_int) -> sqlite3_int64;
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_reset(pStmt: *mut sqlite3_stmt) -> libc::c_int;
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
    #[no_mangle]
    fn dc_sqlite3_get_rowid(
        _: *mut dc_sqlite3_t,
        table: *const libc::c_char,
        field: *const libc::c_char,
        value: *const libc::c_char,
    ) -> uint32_t;
    #[no_mangle]
    fn dc_sqlite3_begin_transaction(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_commit(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_rollback(_: *mut dc_sqlite3_t);
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strlower_in_place(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_str_from_clist(_: *const clist, delimiter: *const libc::c_char) -> *mut libc::c_char;
    /* date/time tools */
    #[no_mangle]
    fn dc_timestamp_from_date(date_time: *mut mailimf_date_time) -> time_t;
    /* timesmearing */
    #[no_mangle]
    fn dc_smeared_time(_: *mut dc_context_t) -> time_t;
    #[no_mangle]
    fn dc_create_smeared_timestamp(_: *mut dc_context_t) -> time_t;
    #[no_mangle]
    fn dc_create_incoming_rfc724_mid(
        message_timestamp: time_t,
        contact_id_from: uint32_t,
        contact_ids_to: *mut dc_array_t,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_extract_grpid_from_rfc724_mid(rfc724_mid: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_extract_grpid_from_rfc724_mid_list(rfc724_mid_list: *const clist) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_decode_header_words(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_get(
        _: *const dc_param_t,
        key: libc::c_int,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
    /* library-private */
    #[no_mangle]
    fn dc_param_new() -> *mut dc_param_t;
    #[no_mangle]
    fn dc_param_unref(_: *mut dc_param_t);
    #[no_mangle]
    fn dc_stock_str_repl_int(
        _: *mut dc_context_t,
        id: libc::c_int,
        value: libc::c_int,
    ) -> *mut libc::c_char;
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
    fn dc_array_new(_: *mut dc_context_t, initsize: size_t) -> *mut dc_array_t;
    #[no_mangle]
    fn dc_array_free_ptr(_: *mut dc_array_t);
    #[no_mangle]
    fn dc_array_duplicate(_: *const dc_array_t) -> *mut dc_array_t;
    #[no_mangle]
    fn dc_array_sort_ids(_: *mut dc_array_t);
    #[no_mangle]
    fn dc_array_sort_strings(_: *mut dc_array_t);
    #[no_mangle]
    fn dc_array_get_string(_: *const dc_array_t, sep: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_chat_load_from_db(_: *mut dc_chat_t, id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_chat_update_param(_: *mut dc_chat_t) -> libc::c_int;
    /* you MUST NOT modify this or the following strings */
    // Context functions to work with chats
    #[no_mangle]
    fn dc_add_to_chat_contacts_table(
        _: *mut dc_context_t,
        chat_id: uint32_t,
        contact_id: uint32_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_get_chat_id_by_grpid(
        _: *mut dc_context_t,
        grpid: *const libc::c_char,
        ret_blocked: *mut libc::c_int,
        ret_verified: *mut libc::c_int,
    ) -> uint32_t;
    #[no_mangle]
    fn dc_create_or_lookup_nchat_by_contact_id(
        _: *mut dc_context_t,
        contact_id: uint32_t,
        create_blocked: libc::c_int,
        ret_chat_id: *mut uint32_t,
        ret_chat_blocked: *mut libc::c_int,
    );
    #[no_mangle]
    fn dc_lookup_real_nchat_by_contact_id(
        _: *mut dc_context_t,
        contact_id: uint32_t,
        ret_chat_id: *mut uint32_t,
        ret_chat_blocked: *mut libc::c_int,
    );
    #[no_mangle]
    fn dc_unarchive_chat(_: *mut dc_context_t, chat_id: uint32_t);
    #[no_mangle]
    fn dc_unblock_chat(_: *mut dc_context_t, chat_id: uint32_t);
    #[no_mangle]
    fn dc_get_chat_contact_cnt(_: *mut dc_context_t, chat_id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_is_group_explicitly_left(_: *mut dc_context_t, grpid: *const libc::c_char)
        -> libc::c_int;
    #[no_mangle]
    fn dc_reset_gossiped_timestamp(_: *mut dc_context_t, chat_id: uint32_t);
    #[no_mangle]
    fn dc_mdn_from_ext(
        _: *mut dc_context_t,
        from_id: uint32_t,
        rfc724_mid: *const libc::c_char,
        _: time_t,
        ret_chat_id: *mut uint32_t,
        ret_msg_id: *mut uint32_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_rfc724_mid_exists(
        _: *mut dc_context_t,
        rfc724_mid: *const libc::c_char,
        ret_server_folder: *mut *mut libc::c_char,
        ret_server_uid: *mut uint32_t,
    ) -> uint32_t;
    #[no_mangle]
    fn dc_update_server_uid(
        _: *mut dc_context_t,
        rfc724_mid: *const libc::c_char,
        server_folder: *const libc::c_char,
        server_uid: uint32_t,
    );
    #[no_mangle]
    fn dc_hash_find(
        _: *const dc_hash_t,
        pKey: *const libc::c_void,
        nKey: libc::c_int,
    ) -> *mut libc::c_void;
    #[no_mangle]
    fn dc_apeerstate_new(_: *mut dc_context_t) -> *mut dc_apeerstate_t;
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
    fn dc_apeerstate_load_by_addr(
        _: *mut dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        addr: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_save_to_db(
        _: *const dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        create: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_has_verified_key(
        _: *const dc_apeerstate_t,
        fingerprints: *const dc_hash_t,
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
    fn dc_contact_is_verified_ex(_: *mut dc_contact_t, _: *const dc_apeerstate_t) -> libc::c_int;
    // Working with names
    #[no_mangle]
    fn dc_normalize_name(full_name: *mut libc::c_char);
    // Working with e-mail-addresses
    #[no_mangle]
    fn dc_addr_cmp(addr1: *const libc::c_char, addr2: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_addr_equals_contact(
        _: *mut dc_context_t,
        addr: *const libc::c_char,
        contact_id: uint32_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_add_or_lookup_contact(
        _: *mut dc_context_t,
        display_name: *const libc::c_char,
        addr_spec: *const libc::c_char,
        origin: libc::c_int,
        sth_modified: *mut libc::c_int,
    ) -> uint32_t;
    #[no_mangle]
    fn dc_get_contact_origin(
        _: *mut dc_context_t,
        contact_id: uint32_t,
        ret_blocked: *mut libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_is_contact_blocked(_: *mut dc_context_t, contact_id: uint32_t) -> libc::c_int;
    #[no_mangle]
    fn dc_scaleup_contact_origin(_: *mut dc_context_t, contact_id: uint32_t, origin: libc::c_int);
    #[no_mangle]
    fn dc_job_add(
        _: *mut dc_context_t,
        action: libc::c_int,
        foreign_id: libc::c_int,
        param: *const libc::c_char,
        delay: libc::c_int,
    );
    #[no_mangle]
    fn dc_mimeparser_new(
        blobdir: *const libc::c_char,
        _: *mut dc_context_t,
    ) -> *mut dc_mimeparser_t;
    #[no_mangle]
    fn dc_mimeparser_unref(_: *mut dc_mimeparser_t);
    #[no_mangle]
    fn dc_mimeparser_parse(
        _: *mut dc_mimeparser_t,
        body_not_terminated: *const libc::c_char,
        body_bytes: size_t,
    );
    /* the following functions can be used only after a call to dc_mimeparser_parse() */
    #[no_mangle]
    fn dc_mimeparser_lookup_field(
        _: *mut dc_mimeparser_t,
        field_name: *const libc::c_char,
    ) -> *mut mailimf_field;
    #[no_mangle]
    fn dc_mimeparser_lookup_optional_field(
        _: *mut dc_mimeparser_t,
        field_name: *const libc::c_char,
    ) -> *mut mailimf_optional_field;
    #[no_mangle]
    fn dc_mimeparser_get_last_nonmeta(_: *mut dc_mimeparser_t) -> *mut dc_mimepart_t;
    #[no_mangle]
    fn dc_mimeparser_is_mailinglist_message(_: *mut dc_mimeparser_t) -> libc::c_int;
    #[no_mangle]
    fn dc_mimeparser_sender_equals_recipient(_: *mut dc_mimeparser_t) -> libc::c_int;
    #[no_mangle]
    fn dc_mimeparser_repl_msg_by_error(_: *mut dc_mimeparser_t, error_msg: *const libc::c_char);
    /* low-level-tools for working with mailmime structures directly */
    #[no_mangle]
    fn mailmime_find_ct_parameter(
        _: *mut mailmime,
        name: *const libc::c_char,
    ) -> *mut mailmime_parameter;
    #[no_mangle]
    fn mailmime_transfer_decode(
        _: *mut mailmime,
        ret_decoded_data: *mut *const libc::c_char,
        ret_decoded_data_bytes: *mut size_t,
        ret_to_mmap_string_unref: *mut *mut libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_find_mailimf_fields(_: *mut mailmime) -> *mut mailimf_fields;
    #[no_mangle]
    fn mailimf_find_optional_field(
        _: *mut mailimf_fields,
        wanted_fld_name: *const libc::c_char,
    ) -> *mut mailimf_optional_field;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_save_locations(
        _: *mut dc_context_t,
        chat_id: uint32_t,
        contact_id: uint32_t,
        _: *const dc_array_t,
    ) -> uint32_t;
    #[no_mangle]
    fn dc_set_msg_location_id(_: *mut dc_context_t, msg_id: uint32_t, location_id: uint32_t);
    #[no_mangle]
    fn dc_do_heuristics_moves(_: *mut dc_context_t, folder: *const libc::c_char, msg_id: uint32_t);
    #[no_mangle]
    fn rpgp_hash_sha256(bytes_ptr: *const uint8_t, bytes_len: size_t) -> *mut rpgp_cvec;
    #[no_mangle]
    fn rpgp_cvec_drop(cvec_ptr: *mut rpgp_cvec);
    #[no_mangle]
    fn rpgp_cvec_data(cvec_ptr: *mut rpgp_cvec) -> *const uint8_t;
    /* library private: secure-join */
    #[no_mangle]
    fn dc_handle_securejoin_handshake(
        _: *mut dc_context_t,
        _: *mut dc_mimeparser_t,
        contact_id: uint32_t,
    ) -> libc::c_int;
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
pub type unnamed_3 = libc::c_uint;
pub const MAILMIME_COMPOSITE_TYPE_EXTENSION: unnamed_3 = 3;
pub const MAILMIME_COMPOSITE_TYPE_MULTIPART: unnamed_3 = 2;
pub const MAILMIME_COMPOSITE_TYPE_MESSAGE: unnamed_3 = 1;
pub const MAILMIME_COMPOSITE_TYPE_ERROR: unnamed_3 = 0;
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
    pub tp_data: unnamed_4,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_4 {
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
pub struct mailmime_parameter {
    pub pa_name: *mut libc::c_char,
    pub pa_value: *mut libc::c_char,
}
pub type unnamed_5 = libc::c_uint;
pub const MAILMIME_TYPE_COMPOSITE_TYPE: unnamed_5 = 2;
pub const MAILMIME_TYPE_DISCRETE_TYPE: unnamed_5 = 1;
pub const MAILMIME_TYPE_ERROR: unnamed_5 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_data {
    pub dt_type: libc::c_int,
    pub dt_encoding: libc::c_int,
    pub dt_encoded: libc::c_int,
    pub dt_data: unnamed_6,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_6 {
    pub dt_text: unnamed_7,
    pub dt_filename: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_7 {
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
    pub mm_data: unnamed_8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_8 {
    pub mm_single: *mut mailmime_data,
    pub mm_multipart: unnamed_10,
    pub mm_message: unnamed_9,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_9 {
    pub mm_fields: *mut mailimf_fields,
    pub mm_msg_mime: *mut mailmime,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_10 {
    pub mm_preamble: *mut mailmime_data,
    pub mm_epilogue: *mut mailmime_data,
    pub mm_mp_list: *mut clist,
}
pub type unnamed_11 = libc::c_uint;
pub const MAIL_ERROR_SSL: unnamed_11 = 58;
pub const MAIL_ERROR_FOLDER: unnamed_11 = 57;
pub const MAIL_ERROR_UNABLE: unnamed_11 = 56;
pub const MAIL_ERROR_SYSTEM: unnamed_11 = 55;
pub const MAIL_ERROR_COMMAND: unnamed_11 = 54;
pub const MAIL_ERROR_SEND: unnamed_11 = 53;
pub const MAIL_ERROR_CHAR_ENCODING_FAILED: unnamed_11 = 52;
pub const MAIL_ERROR_SUBJECT_NOT_FOUND: unnamed_11 = 51;
pub const MAIL_ERROR_PROGRAM_ERROR: unnamed_11 = 50;
pub const MAIL_ERROR_NO_PERMISSION: unnamed_11 = 49;
pub const MAIL_ERROR_COMMAND_NOT_SUPPORTED: unnamed_11 = 48;
pub const MAIL_ERROR_NO_APOP: unnamed_11 = 47;
pub const MAIL_ERROR_READONLY: unnamed_11 = 46;
pub const MAIL_ERROR_FATAL: unnamed_11 = 45;
pub const MAIL_ERROR_CLOSE: unnamed_11 = 44;
pub const MAIL_ERROR_CAPABILITY: unnamed_11 = 43;
pub const MAIL_ERROR_PROTOCOL: unnamed_11 = 42;
pub const MAIL_ERROR_MISC: unnamed_11 = 41;
pub const MAIL_ERROR_EXPUNGE: unnamed_11 = 40;
pub const MAIL_ERROR_NO_TLS: unnamed_11 = 39;
pub const MAIL_ERROR_CACHE_MISS: unnamed_11 = 38;
pub const MAIL_ERROR_STARTTLS: unnamed_11 = 37;
pub const MAIL_ERROR_MOVE: unnamed_11 = 36;
pub const MAIL_ERROR_FOLDER_NOT_FOUND: unnamed_11 = 35;
pub const MAIL_ERROR_REMOVE: unnamed_11 = 34;
pub const MAIL_ERROR_PART_NOT_FOUND: unnamed_11 = 33;
pub const MAIL_ERROR_INVAL: unnamed_11 = 32;
pub const MAIL_ERROR_PARSE: unnamed_11 = 31;
pub const MAIL_ERROR_MSG_NOT_FOUND: unnamed_11 = 30;
pub const MAIL_ERROR_DISKSPACE: unnamed_11 = 29;
pub const MAIL_ERROR_SEARCH: unnamed_11 = 28;
pub const MAIL_ERROR_STORE: unnamed_11 = 27;
pub const MAIL_ERROR_FETCH: unnamed_11 = 26;
pub const MAIL_ERROR_COPY: unnamed_11 = 25;
pub const MAIL_ERROR_APPEND: unnamed_11 = 24;
pub const MAIL_ERROR_LSUB: unnamed_11 = 23;
pub const MAIL_ERROR_LIST: unnamed_11 = 22;
pub const MAIL_ERROR_UNSUBSCRIBE: unnamed_11 = 21;
pub const MAIL_ERROR_SUBSCRIBE: unnamed_11 = 20;
pub const MAIL_ERROR_STATUS: unnamed_11 = 19;
pub const MAIL_ERROR_MEMORY: unnamed_11 = 18;
pub const MAIL_ERROR_SELECT: unnamed_11 = 17;
pub const MAIL_ERROR_EXAMINE: unnamed_11 = 16;
pub const MAIL_ERROR_CHECK: unnamed_11 = 15;
pub const MAIL_ERROR_RENAME: unnamed_11 = 14;
pub const MAIL_ERROR_NOOP: unnamed_11 = 13;
pub const MAIL_ERROR_LOGOUT: unnamed_11 = 12;
pub const MAIL_ERROR_DELETE: unnamed_11 = 11;
pub const MAIL_ERROR_CREATE: unnamed_11 = 10;
pub const MAIL_ERROR_LOGIN: unnamed_11 = 9;
pub const MAIL_ERROR_STREAM: unnamed_11 = 8;
pub const MAIL_ERROR_FILE: unnamed_11 = 7;
pub const MAIL_ERROR_BAD_STATE: unnamed_11 = 6;
pub const MAIL_ERROR_CONNECT: unnamed_11 = 5;
pub const MAIL_ERROR_UNKNOWN: unnamed_11 = 4;
pub const MAIL_ERROR_NOT_IMPLEMENTED: unnamed_11 = 3;
pub const MAIL_NO_ERROR_NON_AUTHENTICATED: unnamed_11 = 2;
pub const MAIL_NO_ERROR_AUTHENTICATED: unnamed_11 = 1;
pub const MAIL_NO_ERROR: unnamed_11 = 0;
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
    pub smtp_sasl: unnamed_12,
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
pub struct unnamed_12 {
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
    pub sec_data: unnamed_13,
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
pub union unnamed_13 {
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
    pub ft_data: unnamed_14,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_14 {
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
    pub imap_sasl: unnamed_15,
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
pub type sqlite_int64 = libc::c_longlong;
pub type sqlite3_int64 = sqlite_int64;
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type dc_strbuilder_t = _dc_strbuilder;
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
pub type dc_apeerstate_t = _dc_apeerstate;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_mimepart {
    pub type_0: libc::c_int,
    pub is_meta: libc::c_int,
    pub int_mimetype: libc::c_int,
    pub msg: *mut libc::c_char,
    pub msg_raw: *mut libc::c_char,
    pub bytes: libc::c_int,
    pub param: *mut dc_param_t,
}
/* Parse MIME body; this is the text part of an IMF, see https://tools.ietf.org/html/rfc5322
dc_mimeparser_t has no deep dependencies to dc_context_t or to the database
(dc_context_t is used for logging only). */
pub type dc_mimepart_t = _dc_mimepart;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rpgp_cvec {
    pub data: *mut uint8_t,
    pub len: size_t,
}
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
#[no_mangle]
pub unsafe extern "C" fn dc_receive_imf(
    mut context: *mut dc_context_t,
    mut imf_raw_not_terminated: *const libc::c_char,
    mut imf_raw_bytes: size_t,
    mut server_folder: *const libc::c_char,
    mut server_uid: uint32_t,
    mut flags: uint32_t,
) {
    let mut current_block: u64;
    /* the function returns the number of created messages in the database */
    let mut incoming: libc::c_int = 1i32;
    let mut incoming_origin: libc::c_int = 0i32;
    let mut to_ids: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut to_self: libc::c_int = 0i32;
    let mut from_id: uint32_t = 0i32 as uint32_t;
    let mut from_id_blocked: libc::c_int = 0i32;
    let mut to_id: uint32_t = 0i32 as uint32_t;
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0i32;
    let mut state: libc::c_int = 0i32;
    let mut hidden: libc::c_int = 0i32;
    let mut msgrmsg: libc::c_int = 0i32;
    let mut add_delete_job: libc::c_int = 0i32;
    let mut insert_msg_id: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut i: size_t = 0i32 as size_t;
    let mut icnt: size_t = 0i32 as size_t;
    /* Message-ID from the header */
    let mut rfc724_mid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut sort_timestamp: time_t = -1i32 as time_t;
    let mut sent_timestamp: time_t = -1i32 as time_t;
    let mut rcvd_timestamp: time_t = -1i32 as time_t;
    let mut mime_parser: *mut dc_mimeparser_t = dc_mimeparser_new((*context).blobdir, context);
    let mut transaction_pending: libc::c_int = 0i32;
    let mut field: *const mailimf_field = 0 as *const mailimf_field;
    let mut mime_in_reply_to: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut mime_references: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut created_db_entries: *mut carray = carray_new(16i32 as libc::c_uint);
    let mut create_event_to_send: libc::c_int = 2000i32;
    let mut rr_event_to_send: *mut carray = carray_new(16i32 as libc::c_uint);
    let mut txt_raw: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_log_info(
        context,
        0i32,
        b"Receiving message %s/%lu...\x00" as *const u8 as *const libc::c_char,
        if !server_folder.is_null() {
            server_folder
        } else {
            b"?\x00" as *const u8 as *const libc::c_char
        },
        server_uid,
    );
    to_ids = dc_array_new(context, 16i32 as size_t);
    if to_ids.is_null()
        || created_db_entries.is_null()
        || rr_event_to_send.is_null()
        || mime_parser.is_null()
    {
        dc_log_info(
            context,
            0i32,
            b"Bad param.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        dc_mimeparser_parse(mime_parser, imf_raw_not_terminated, imf_raw_bytes);
        if (*mime_parser).header.count == 0i32 {
            dc_log_info(
                context,
                0i32,
                b"No header.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            /* Error - even adding an empty record won't help as we do not know the message ID */
            field = dc_mimeparser_lookup_field(
                mime_parser,
                b"Date\x00" as *const u8 as *const libc::c_char,
            );
            if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_ORIG_DATE as libc::c_int {
                let mut orig_date: *mut mailimf_orig_date = (*field).fld_data.fld_orig_date;
                if !orig_date.is_null() {
                    sent_timestamp = dc_timestamp_from_date((*orig_date).dt_date_time)
                }
            }
            dc_sqlite3_begin_transaction((*context).sql);
            transaction_pending = 1i32;
            field = dc_mimeparser_lookup_field(
                mime_parser,
                b"From\x00" as *const u8 as *const libc::c_char,
            );
            if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_FROM as libc::c_int {
                let mut fld_from: *mut mailimf_from = (*field).fld_data.fld_from;
                if !fld_from.is_null() {
                    let mut check_self: libc::c_int = 0;
                    let mut from_list: *mut dc_array_t = dc_array_new(context, 16i32 as size_t);
                    dc_add_or_lookup_contacts_by_mailbox_list(
                        context,
                        (*fld_from).frm_mb_list,
                        0x10i32,
                        from_list,
                        &mut check_self,
                    );
                    if 0 != check_self {
                        incoming = 0i32;
                        if 0 != dc_mimeparser_sender_equals_recipient(mime_parser) {
                            from_id = 1i32 as uint32_t
                        }
                    } else if dc_array_get_cnt(from_list) >= 1i32 as libc::c_ulong {
                        from_id = dc_array_get_id(from_list, 0i32 as size_t);
                        incoming_origin =
                            dc_get_contact_origin(context, from_id, &mut from_id_blocked)
                    }
                    dc_array_unref(from_list);
                }
            }
            field = dc_mimeparser_lookup_field(
                mime_parser,
                b"To\x00" as *const u8 as *const libc::c_char,
            );
            if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_TO as libc::c_int {
                let mut fld_to: *mut mailimf_to = (*field).fld_data.fld_to;
                if !fld_to.is_null() {
                    dc_add_or_lookup_contacts_by_address_list(
                        context,
                        (*fld_to).to_addr_list,
                        if 0 == incoming {
                            0x4000i32
                        } else if incoming_origin >= 0x100i32 {
                            0x400i32
                        } else {
                            0x40i32
                        },
                        to_ids,
                        &mut to_self,
                    );
                }
            }
            if !dc_mimeparser_get_last_nonmeta(mime_parser).is_null() {
                field = dc_mimeparser_lookup_field(
                    mime_parser,
                    b"Cc\x00" as *const u8 as *const libc::c_char,
                );
                if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_CC as libc::c_int {
                    let mut fld_cc: *mut mailimf_cc = (*field).fld_data.fld_cc;
                    if !fld_cc.is_null() {
                        dc_add_or_lookup_contacts_by_address_list(
                            context,
                            (*fld_cc).cc_addr_list,
                            if 0 == incoming {
                                0x2000i32
                            } else if incoming_origin >= 0x100i32 {
                                0x200i32
                            } else {
                                0x20i32
                            },
                            to_ids,
                            0 as *mut libc::c_int,
                        );
                    }
                }
                field = dc_mimeparser_lookup_field(
                    mime_parser,
                    b"Message-ID\x00" as *const u8 as *const libc::c_char,
                );
                if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_MESSAGE_ID as libc::c_int
                {
                    let mut fld_message_id: *mut mailimf_message_id =
                        (*field).fld_data.fld_message_id;
                    if !fld_message_id.is_null() {
                        rfc724_mid = dc_strdup((*fld_message_id).mid_value)
                    }
                }
                if rfc724_mid.is_null() {
                    rfc724_mid = dc_create_incoming_rfc724_mid(sent_timestamp, from_id, to_ids);
                    if rfc724_mid.is_null() {
                        dc_log_info(
                            context,
                            0i32,
                            b"Cannot create Message-ID.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 16282941964262048061;
                    } else {
                        current_block = 777662472977924419;
                    }
                } else {
                    current_block = 777662472977924419;
                }
                match current_block {
                    16282941964262048061 => {}
                    _ => {
                        /* check, if the mail is already in our database - if so, just update the folder/uid (if the mail was moved around) and finish.
                        (we may get a mail twice eg. if it is moved between folders. make sure, this check is done eg. before securejoin-processing) */
                        let mut old_server_folder: *mut libc::c_char = 0 as *mut libc::c_char;
                        let mut old_server_uid: uint32_t = 0i32 as uint32_t;
                        if 0 != dc_rfc724_mid_exists(
                            context,
                            rfc724_mid,
                            &mut old_server_folder,
                            &mut old_server_uid,
                        ) {
                            if strcmp(old_server_folder, server_folder) != 0i32
                                || old_server_uid != server_uid
                            {
                                dc_sqlite3_rollback((*context).sql);
                                transaction_pending = 0i32;
                                dc_update_server_uid(
                                    context,
                                    rfc724_mid,
                                    server_folder,
                                    server_uid,
                                );
                            }
                            free(old_server_folder as *mut libc::c_void);
                            dc_log_info(
                                context,
                                0i32,
                                b"Message already in DB.\x00" as *const u8 as *const libc::c_char,
                            );
                            current_block = 16282941964262048061;
                        } else {
                            msgrmsg = (*mime_parser).is_send_by_messenger;
                            if msgrmsg == 0i32
                                && 0 != dc_is_reply_to_messenger_message(context, mime_parser)
                            {
                                msgrmsg = 2i32
                            }
                            /* incoming non-chat messages may be discarded;
                            maybe this can be optimized later,
                            by checking the state before the message body is downloaded */
                            let mut allow_creation: libc::c_int = 1i32;
                            if msgrmsg == 0i32 {
                                let mut show_emails: libc::c_int = dc_sqlite3_get_config_int(
                                    (*context).sql,
                                    b"show_emails\x00" as *const u8 as *const libc::c_char,
                                    0i32,
                                );
                                if show_emails == 0i32 {
                                    chat_id = 3i32 as uint32_t;
                                    allow_creation = 0i32
                                } else if show_emails == 1i32 {
                                    allow_creation = 0i32
                                }
                            }
                            if 0 != incoming {
                                state = if 0 != flags as libc::c_long & 0x1i64 {
                                    16i32
                                } else {
                                    10i32
                                };
                                to_id = 1i32 as uint32_t;
                                if !dc_mimeparser_lookup_field(
                                    mime_parser,
                                    b"Secure-Join\x00" as *const u8 as *const libc::c_char,
                                )
                                .is_null()
                                {
                                    msgrmsg = 1i32;
                                    chat_id = 0i32 as uint32_t;
                                    allow_creation = 1i32;
                                    dc_sqlite3_commit((*context).sql);
                                    let mut handshake: libc::c_int = dc_handle_securejoin_handshake(
                                        context,
                                        mime_parser,
                                        from_id,
                                    );
                                    if 0 != handshake & 0x2i32 {
                                        hidden = 1i32;
                                        add_delete_job = handshake & 0x4i32;
                                        state = 16i32
                                    }
                                    dc_sqlite3_begin_transaction((*context).sql);
                                }
                                let mut test_normal_chat_id: uint32_t = 0i32 as uint32_t;
                                let mut test_normal_chat_id_blocked: libc::c_int = 0i32;
                                dc_lookup_real_nchat_by_contact_id(
                                    context,
                                    from_id,
                                    &mut test_normal_chat_id,
                                    &mut test_normal_chat_id_blocked,
                                );
                                if chat_id == 0i32 as libc::c_uint {
                                    let mut create_blocked: libc::c_int = if 0
                                        != test_normal_chat_id
                                        && test_normal_chat_id_blocked == 0i32
                                        || incoming_origin >= 0x7fffffffi32
                                    {
                                        0i32
                                    } else {
                                        2i32
                                    };
                                    create_or_lookup_group(
                                        context,
                                        mime_parser,
                                        allow_creation,
                                        create_blocked,
                                        from_id as int32_t,
                                        to_ids,
                                        &mut chat_id,
                                        &mut chat_id_blocked,
                                    );
                                    if 0 != chat_id && 0 != chat_id_blocked && 0 == create_blocked {
                                        dc_unblock_chat(context, chat_id);
                                        chat_id_blocked = 0i32
                                    }
                                }
                                if chat_id == 0i32 as libc::c_uint {
                                    if 0 != dc_mimeparser_is_mailinglist_message(mime_parser) {
                                        chat_id = 3i32 as uint32_t;
                                        dc_log_info(
                                            context,
                                            0i32,
                                            b"Message belongs to a mailing list and is ignored.\x00"
                                                as *const u8
                                                as *const libc::c_char,
                                        );
                                    }
                                }
                                if chat_id == 0i32 as libc::c_uint {
                                    let mut create_blocked_0: libc::c_int =
                                        if incoming_origin >= 0x7fffffffi32 || from_id == to_id {
                                            0i32
                                        } else {
                                            2i32
                                        };
                                    if 0 != test_normal_chat_id {
                                        chat_id = test_normal_chat_id;
                                        chat_id_blocked = test_normal_chat_id_blocked
                                    } else if 0 != allow_creation {
                                        dc_create_or_lookup_nchat_by_contact_id(
                                            context,
                                            from_id,
                                            create_blocked_0,
                                            &mut chat_id,
                                            &mut chat_id_blocked,
                                        );
                                    }
                                    if 0 != chat_id && 0 != chat_id_blocked {
                                        if 0 == create_blocked_0 {
                                            dc_unblock_chat(context, chat_id);
                                            chat_id_blocked = 0i32
                                        } else if 0
                                            != dc_is_reply_to_known_message(context, mime_parser)
                                        {
                                            dc_scaleup_contact_origin(context, from_id, 0x100i32);
                                            dc_log_info(context, 0i32,
                                                        b"Message is a reply to a known message, mark sender as known.\x00"
                                                            as *const u8 as
                                                            *const libc::c_char);
                                            incoming_origin = if incoming_origin > 0x100i32 {
                                                incoming_origin
                                            } else {
                                                0x100i32
                                            }
                                        }
                                    }
                                }
                                if chat_id == 0i32 as libc::c_uint {
                                    chat_id = 3i32 as uint32_t
                                }
                                if 0 != chat_id_blocked && state == 10i32 {
                                    if incoming_origin < 0x100i32 && msgrmsg == 0i32 {
                                        state = 13i32
                                    }
                                }
                            } else {
                                state = 26i32;
                                from_id = 1i32 as uint32_t;
                                if dc_array_get_cnt(to_ids) >= 1i32 as libc::c_ulong {
                                    to_id = dc_array_get_id(to_ids, 0i32 as size_t);
                                    if chat_id == 0i32 as libc::c_uint {
                                        create_or_lookup_group(
                                            context,
                                            mime_parser,
                                            allow_creation,
                                            0i32,
                                            from_id as int32_t,
                                            to_ids,
                                            &mut chat_id,
                                            &mut chat_id_blocked,
                                        );
                                        if 0 != chat_id && 0 != chat_id_blocked {
                                            dc_unblock_chat(context, chat_id);
                                            chat_id_blocked = 0i32
                                        }
                                    }
                                    if chat_id == 0i32 as libc::c_uint && 0 != allow_creation {
                                        let mut create_blocked_1: libc::c_int = if 0 != msgrmsg
                                            && 0 == dc_is_contact_blocked(context, to_id)
                                        {
                                            0i32
                                        } else {
                                            2i32
                                        };
                                        dc_create_or_lookup_nchat_by_contact_id(
                                            context,
                                            to_id,
                                            create_blocked_1,
                                            &mut chat_id,
                                            &mut chat_id_blocked,
                                        );
                                        if 0 != chat_id
                                            && 0 != chat_id_blocked
                                            && 0 == create_blocked_1
                                        {
                                            dc_unblock_chat(context, chat_id);
                                            chat_id_blocked = 0i32
                                        }
                                    }
                                }
                                if chat_id == 0i32 as libc::c_uint {
                                    if dc_array_get_cnt(to_ids) == 0i32 as libc::c_ulong
                                        && 0 != to_self
                                    {
                                        dc_create_or_lookup_nchat_by_contact_id(
                                            context,
                                            1i32 as uint32_t,
                                            0i32,
                                            &mut chat_id,
                                            &mut chat_id_blocked,
                                        );
                                        if 0 != chat_id && 0 != chat_id_blocked {
                                            dc_unblock_chat(context, chat_id);
                                            chat_id_blocked = 0i32
                                        }
                                    }
                                }
                                if chat_id == 0i32 as libc::c_uint {
                                    chat_id = 3i32 as uint32_t
                                }
                            }
                            calc_timestamps(
                                context,
                                chat_id,
                                from_id,
                                sent_timestamp,
                                if 0 != flags as libc::c_long & 0x1i64 {
                                    0i32
                                } else {
                                    1i32
                                },
                                &mut sort_timestamp,
                                &mut sent_timestamp,
                                &mut rcvd_timestamp,
                            );
                            dc_unarchive_chat(context, chat_id);
                            // if the mime-headers should be saved, find out its size
                            // (the mime-header ends with an empty line)
                            let mut save_mime_headers: libc::c_int = dc_sqlite3_get_config_int(
                                (*context).sql,
                                b"save_mime_headers\x00" as *const u8 as *const libc::c_char,
                                0i32,
                            );
                            let mut header_bytes: libc::c_int = imf_raw_bytes as libc::c_int;
                            if 0 != save_mime_headers {
                                let mut p: *mut libc::c_char = 0 as *mut libc::c_char;
                                p = strstr(
                                    imf_raw_not_terminated,
                                    b"\r\n\r\n\x00" as *const u8 as *const libc::c_char,
                                );
                                if !p.is_null() {
                                    header_bytes = (p.wrapping_offset_from(imf_raw_not_terminated)
                                        as libc::c_long
                                        + 4i32 as libc::c_long)
                                        as libc::c_int
                                } else {
                                    p = strstr(
                                        imf_raw_not_terminated,
                                        b"\n\n\x00" as *const u8 as *const libc::c_char,
                                    );
                                    if !p.is_null() {
                                        header_bytes = (p
                                            .wrapping_offset_from(imf_raw_not_terminated)
                                            as libc::c_long
                                            + 2i32 as libc::c_long)
                                            as libc::c_int
                                    }
                                }
                            }
                            field = dc_mimeparser_lookup_field(
                                mime_parser,
                                b"In-Reply-To\x00" as *const u8 as *const libc::c_char,
                            );
                            if !field.is_null()
                                && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int
                            {
                                let mut fld_in_reply_to: *mut mailimf_in_reply_to =
                                    (*field).fld_data.fld_in_reply_to;
                                if !fld_in_reply_to.is_null() {
                                    mime_in_reply_to = dc_str_from_clist(
                                        (*(*field).fld_data.fld_in_reply_to).mid_list,
                                        b" \x00" as *const u8 as *const libc::c_char,
                                    )
                                }
                            }
                            field = dc_mimeparser_lookup_field(
                                mime_parser,
                                b"References\x00" as *const u8 as *const libc::c_char,
                            );
                            if !field.is_null()
                                && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int
                            {
                                let mut fld_references: *mut mailimf_references =
                                    (*field).fld_data.fld_references;
                                if !fld_references.is_null() {
                                    mime_references = dc_str_from_clist(
                                        (*(*field).fld_data.fld_references).mid_list,
                                        b" \x00" as *const u8 as *const libc::c_char,
                                    )
                                }
                            }
                            icnt = carray_count((*mime_parser).parts) as size_t;
                            stmt =
                                dc_sqlite3_prepare((*context).sql,
                                                   b"INSERT INTO msgs (rfc724_mid, server_folder, server_uid, chat_id, from_id, to_id, timestamp, timestamp_sent, timestamp_rcvd, type, state, msgrmsg,  txt, txt_raw, param, bytes, hidden, mime_headers,  mime_in_reply_to, mime_references) VALUES (?,?,?,?,?,?, ?,?,?,?,?,?, ?,?,?,?,?,?, ?,?);\x00"
                                                       as *const u8 as
                                                       *const libc::c_char);
                            i = 0i32 as size_t;
                            loop {
                                if !(i < icnt) {
                                    current_block = 2756754640271984560;
                                    break;
                                }
                                let mut part: *mut dc_mimepart_t =
                                    carray_get((*mime_parser).parts, i as libc::c_uint)
                                        as *mut dc_mimepart_t;
                                if !(0 != (*part).is_meta) {
                                    if !(*mime_parser).kml.is_null()
                                        && icnt == 1i32 as libc::c_ulong
                                        && !(*part).msg.is_null()
                                        && (strcmp(
                                            (*part).msg,
                                            b"-location-\x00" as *const u8 as *const libc::c_char,
                                        ) == 0i32
                                            || *(*part).msg.offset(0isize) as libc::c_int == 0i32)
                                    {
                                        hidden = 1i32;
                                        if state == 10i32 {
                                            state = 13i32
                                        }
                                    }
                                    if (*part).type_0 == 10i32 {
                                        txt_raw = dc_mprintf(
                                            b"%s\n\n%s\x00" as *const u8 as *const libc::c_char,
                                            if !(*mime_parser).subject.is_null() {
                                                (*mime_parser).subject
                                            } else {
                                                b"\x00" as *const u8 as *const libc::c_char
                                            },
                                            (*part).msg_raw,
                                        )
                                    }
                                    if 0 != (*mime_parser).is_system_message {
                                        dc_param_set_int(
                                            (*part).param,
                                            'S' as i32,
                                            (*mime_parser).is_system_message,
                                        );
                                    }
                                    sqlite3_reset(stmt);
                                    sqlite3_bind_text(stmt, 1i32, rfc724_mid, -1i32, None);
                                    sqlite3_bind_text(stmt, 2i32, server_folder, -1i32, None);
                                    sqlite3_bind_int(stmt, 3i32, server_uid as libc::c_int);
                                    sqlite3_bind_int(stmt, 4i32, chat_id as libc::c_int);
                                    sqlite3_bind_int(stmt, 5i32, from_id as libc::c_int);
                                    sqlite3_bind_int(stmt, 6i32, to_id as libc::c_int);
                                    sqlite3_bind_int64(stmt, 7i32, sort_timestamp as sqlite3_int64);
                                    sqlite3_bind_int64(stmt, 8i32, sent_timestamp as sqlite3_int64);
                                    sqlite3_bind_int64(stmt, 9i32, rcvd_timestamp as sqlite3_int64);
                                    sqlite3_bind_int(stmt, 10i32, (*part).type_0);
                                    sqlite3_bind_int(stmt, 11i32, state);
                                    sqlite3_bind_int(stmt, 12i32, msgrmsg);
                                    sqlite3_bind_text(
                                        stmt,
                                        13i32,
                                        if !(*part).msg.is_null() {
                                            (*part).msg
                                        } else {
                                            b"\x00" as *const u8 as *const libc::c_char
                                        },
                                        -1i32,
                                        None,
                                    );
                                    sqlite3_bind_text(
                                        stmt,
                                        14i32,
                                        if !txt_raw.is_null() {
                                            txt_raw
                                        } else {
                                            b"\x00" as *const u8 as *const libc::c_char
                                        },
                                        -1i32,
                                        None,
                                    );
                                    sqlite3_bind_text(
                                        stmt,
                                        15i32,
                                        (*(*part).param).packed,
                                        -1i32,
                                        None,
                                    );
                                    sqlite3_bind_int(stmt, 16i32, (*part).bytes);
                                    sqlite3_bind_int(stmt, 17i32, hidden);
                                    sqlite3_bind_text(
                                        stmt,
                                        18i32,
                                        if 0 != save_mime_headers {
                                            imf_raw_not_terminated
                                        } else {
                                            0 as *const libc::c_char
                                        },
                                        header_bytes,
                                        None,
                                    );
                                    sqlite3_bind_text(stmt, 19i32, mime_in_reply_to, -1i32, None);
                                    sqlite3_bind_text(stmt, 20i32, mime_references, -1i32, None);
                                    if sqlite3_step(stmt) != 101i32 {
                                        dc_log_info(
                                            context,
                                            0i32,
                                            b"Cannot write DB.\x00" as *const u8
                                                as *const libc::c_char,
                                        );
                                        /* i/o error - there is nothing more we can do - in other cases, we try to write at least an empty record */
                                        current_block = 16282941964262048061;
                                        break;
                                    } else {
                                        free(txt_raw as *mut libc::c_void);
                                        txt_raw = 0 as *mut libc::c_char;
                                        insert_msg_id = dc_sqlite3_get_rowid(
                                            (*context).sql,
                                            b"msgs\x00" as *const u8 as *const libc::c_char,
                                            b"rfc724_mid\x00" as *const u8 as *const libc::c_char,
                                            rfc724_mid,
                                        );
                                        carray_add(
                                            created_db_entries,
                                            chat_id as uintptr_t as *mut libc::c_void,
                                            0 as *mut libc::c_uint,
                                        );
                                        carray_add(
                                            created_db_entries,
                                            insert_msg_id as uintptr_t as *mut libc::c_void,
                                            0 as *mut libc::c_uint,
                                        );
                                    }
                                }
                                i = i.wrapping_add(1)
                            }
                            match current_block {
                                16282941964262048061 => {}
                                _ => {
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"Message has %i parts and is assigned to chat #%i.\x00"
                                            as *const u8
                                            as *const libc::c_char,
                                        icnt,
                                        chat_id,
                                    );
                                    if chat_id == 3i32 as libc::c_uint {
                                        create_event_to_send = 0i32
                                    } else if 0 != incoming && state == 10i32 {
                                        if 0 != from_id_blocked {
                                            create_event_to_send = 0i32
                                        } else if 0 != chat_id_blocked {
                                            create_event_to_send = 2000i32
                                        } else {
                                            create_event_to_send = 2005i32
                                        }
                                    }
                                    dc_do_heuristics_moves(context, server_folder, insert_msg_id);
                                    current_block = 18330534242458572360;
                                }
                            }
                        }
                    }
                }
            } else {
                if sent_timestamp > time(0 as *mut time_t) {
                    sent_timestamp = time(0 as *mut time_t)
                }
                current_block = 18330534242458572360;
            }
            match current_block {
                16282941964262048061 => {}
                _ => {
                    if carray_count((*mime_parser).reports) > 0i32 as libc::c_uint {
                        let mut mdns_enabled: libc::c_int = dc_sqlite3_get_config_int(
                            (*context).sql,
                            b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                            1i32,
                        );
                        icnt = carray_count((*mime_parser).reports) as size_t;
                        i = 0i32 as size_t;
                        while i < icnt {
                            let mut mdn_consumed: libc::c_int = 0i32;
                            let mut report_root: *mut mailmime =
                                carray_get((*mime_parser).reports, i as libc::c_uint)
                                    as *mut mailmime;
                            let mut report_type: *mut mailmime_parameter =
                                mailmime_find_ct_parameter(
                                    report_root,
                                    b"report-type\x00" as *const u8 as *const libc::c_char,
                                );
                            if !(report_root.is_null()
                                || report_type.is_null()
                                || (*report_type).pa_value.is_null())
                            {
                                if strcmp(
                                    (*report_type).pa_value,
                                    b"disposition-notification\x00" as *const u8
                                        as *const libc::c_char,
                                ) == 0i32
                                    && (*(*report_root).mm_data.mm_multipart.mm_mp_list).count
                                        >= 2i32
                                {
                                    if 0 != mdns_enabled {
                                        let mut report_data: *mut mailmime =
                                            (if !if !(*(*report_root)
                                                .mm_data
                                                .mm_multipart
                                                .mm_mp_list)
                                                .first
                                                .is_null()
                                            {
                                                (*(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                                                    .first)
                                                    .next
                                            } else {
                                                0 as *mut clistcell_s
                                            }
                                            .is_null()
                                            {
                                                (*if !(*(*report_root)
                                                    .mm_data
                                                    .mm_multipart
                                                    .mm_mp_list)
                                                    .first
                                                    .is_null()
                                                {
                                                    (*(*(*report_root)
                                                        .mm_data
                                                        .mm_multipart
                                                        .mm_mp_list)
                                                        .first)
                                                        .next
                                                } else {
                                                    0 as *mut clistcell_s
                                                })
                                                .data
                                            } else {
                                                0 as *mut libc::c_void
                                            })
                                                as *mut mailmime;
                                        if !report_data.is_null()
                                            && (*(*(*report_data).mm_content_type).ct_type).tp_type
                                                == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int
                                            && (*(*(*(*report_data).mm_content_type).ct_type)
                                                .tp_data
                                                .tp_composite_type)
                                                .ct_type
                                                == MAILMIME_COMPOSITE_TYPE_MESSAGE as libc::c_int
                                            && strcmp(
                                                (*(*report_data).mm_content_type).ct_subtype,
                                                b"disposition-notification\x00" as *const u8
                                                    as *const libc::c_char,
                                            ) == 0i32
                                        {
                                            let mut report_body: *const libc::c_char =
                                                0 as *const libc::c_char;
                                            let mut report_body_bytes: size_t = 0i32 as size_t;
                                            let mut to_mmap_string_unref: *mut libc::c_char =
                                                0 as *mut libc::c_char;
                                            if 0 != mailmime_transfer_decode(
                                                report_data,
                                                &mut report_body,
                                                &mut report_body_bytes,
                                                &mut to_mmap_string_unref,
                                            ) {
                                                let mut report_parsed: *mut mailmime =
                                                    0 as *mut mailmime;
                                                let mut dummy: size_t = 0i32 as size_t;
                                                if mailmime_parse(
                                                    report_body,
                                                    report_body_bytes,
                                                    &mut dummy,
                                                    &mut report_parsed,
                                                ) == MAIL_NO_ERROR as libc::c_int
                                                    && !report_parsed.is_null()
                                                {
                                                    let mut report_fields: *mut mailimf_fields =
                                                        mailmime_find_mailimf_fields(report_parsed);
                                                    if !report_fields.is_null() {
                                                        let mut of_disposition:
                                                                *mut mailimf_optional_field =
                                                            mailimf_find_optional_field(report_fields,
                                                                                        b"Disposition\x00"
                                                                                            as
                                                                                            *const u8
                                                                                            as
                                                                                            *const libc::c_char);
                                                        let mut of_org_msgid:
                                                                *mut mailimf_optional_field =
                                                            mailimf_find_optional_field(report_fields,
                                                                                        b"Original-Message-ID\x00"
                                                                                            as
                                                                                            *const u8
                                                                                            as
                                                                                            *const libc::c_char);
                                                        if !of_disposition.is_null()
                                                            && !(*of_disposition)
                                                                .fld_value
                                                                .is_null()
                                                            && !of_org_msgid.is_null()
                                                            && !(*of_org_msgid).fld_value.is_null()
                                                        {
                                                            let mut rfc724_mid_0:
                                                                    *mut libc::c_char =
                                                                0 as
                                                                    *mut libc::c_char;
                                                            dummy = 0i32 as size_t;
                                                            if mailimf_msg_id_parse(
                                                                (*of_org_msgid).fld_value,
                                                                strlen((*of_org_msgid).fld_value),
                                                                &mut dummy,
                                                                &mut rfc724_mid_0,
                                                            ) == MAIL_NO_ERROR as libc::c_int
                                                                && !rfc724_mid_0.is_null()
                                                            {
                                                                let mut chat_id_0: uint32_t =
                                                                    0i32 as uint32_t;
                                                                let mut msg_id: uint32_t =
                                                                    0i32 as uint32_t;
                                                                if 0 != dc_mdn_from_ext(
                                                                    context,
                                                                    from_id,
                                                                    rfc724_mid_0,
                                                                    sent_timestamp,
                                                                    &mut chat_id_0,
                                                                    &mut msg_id,
                                                                ) {
                                                                    carray_add(
                                                                        rr_event_to_send,
                                                                        chat_id_0 as uintptr_t
                                                                            as *mut libc::c_void,
                                                                        0 as *mut libc::c_uint,
                                                                    );
                                                                    carray_add(
                                                                        rr_event_to_send,
                                                                        msg_id as uintptr_t
                                                                            as *mut libc::c_void,
                                                                        0 as *mut libc::c_uint,
                                                                    );
                                                                }
                                                                mdn_consumed = (msg_id
                                                                    != 0i32 as libc::c_uint)
                                                                    as libc::c_int;
                                                                free(
                                                                    rfc724_mid_0
                                                                        as *mut libc::c_void,
                                                                );
                                                            }
                                                        }
                                                    }
                                                    mailmime_free(report_parsed);
                                                }
                                                if !to_mmap_string_unref.is_null() {
                                                    mmap_string_unref(to_mmap_string_unref);
                                                }
                                            }
                                        }
                                    }
                                    if 0 != (*mime_parser).is_send_by_messenger || 0 != mdn_consumed
                                    {
                                        let mut param: *mut dc_param_t = dc_param_new();
                                        dc_param_set(param, 'Z' as i32, server_folder);
                                        dc_param_set_int(param, 'z' as i32, server_uid as int32_t);
                                        if 0 != (*mime_parser).is_send_by_messenger
                                            && 0 != dc_sqlite3_get_config_int(
                                                (*context).sql,
                                                b"mvbox_move\x00" as *const u8
                                                    as *const libc::c_char,
                                                1i32,
                                            )
                                        {
                                            dc_param_set_int(param, 'M' as i32, 1i32);
                                        }
                                        dc_job_add(context, 120i32, 0i32, (*param).packed, 0i32);
                                        dc_param_unref(param);
                                    }
                                }
                            }
                            i = i.wrapping_add(1)
                        }
                    }
                    if !(*mime_parser).kml.is_null() && chat_id > 9i32 as libc::c_uint {
                        let mut contact: *mut dc_contact_t = dc_get_contact(context, from_id);
                        if !(*(*mime_parser).kml).addr.is_null()
                            && !contact.is_null()
                            && !(*contact).addr.is_null()
                            && strcasecmp((*contact).addr, (*(*mime_parser).kml).addr) == 0i32
                        {
                            let mut newest_location_id: uint32_t = dc_save_locations(
                                context,
                                chat_id,
                                from_id,
                                (*(*mime_parser).kml).locations,
                            );
                            if 0 != newest_location_id && 0 == hidden {
                                dc_set_msg_location_id(context, insert_msg_id, newest_location_id);
                            }
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2035i32,
                                from_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                        }
                        dc_contact_unref(contact);
                    }
                    if 0 != add_delete_job
                        && carray_count(created_db_entries) >= 2i32 as libc::c_uint
                    {
                        dc_job_add(
                            context,
                            110i32,
                            carray_get(created_db_entries, 1i32 as libc::c_uint) as uintptr_t
                                as libc::c_int,
                            0 as *const libc::c_char,
                            0i32,
                        );
                    }
                    dc_sqlite3_commit((*context).sql);
                    transaction_pending = 0i32
                }
            }
        }
    }
    if 0 != transaction_pending {
        dc_sqlite3_rollback((*context).sql);
    }
    dc_mimeparser_unref(mime_parser);
    free(rfc724_mid as *mut libc::c_void);
    free(mime_in_reply_to as *mut libc::c_void);
    free(mime_references as *mut libc::c_void);
    dc_array_unref(to_ids);
    if !created_db_entries.is_null() {
        if 0 != create_event_to_send {
            let mut i_0: size_t = 0;
            let mut icnt_0: size_t = carray_count(created_db_entries) as size_t;
            i_0 = 0i32 as size_t;
            while i_0 < icnt_0 {
                (*context).cb.expect("non-null function pointer")(
                    context,
                    create_event_to_send,
                    carray_get(created_db_entries, i_0 as libc::c_uint) as uintptr_t,
                    carray_get(
                        created_db_entries,
                        i_0.wrapping_add(1i32 as libc::c_ulong) as libc::c_uint,
                    ) as uintptr_t,
                );
                i_0 = (i_0 as libc::c_ulong).wrapping_add(2i32 as libc::c_ulong) as size_t as size_t
            }
        }
        carray_free(created_db_entries);
    }
    if !rr_event_to_send.is_null() {
        let mut i_1: size_t = 0;
        let mut icnt_1: size_t = carray_count(rr_event_to_send) as size_t;
        i_1 = 0i32 as size_t;
        while i_1 < icnt_1 {
            (*context).cb.expect("non-null function pointer")(
                context,
                2015i32,
                carray_get(rr_event_to_send, i_1 as libc::c_uint) as uintptr_t,
                carray_get(
                    rr_event_to_send,
                    i_1.wrapping_add(1i32 as libc::c_ulong) as libc::c_uint,
                ) as uintptr_t,
            );
            i_1 = (i_1 as libc::c_ulong).wrapping_add(2i32 as libc::c_ulong) as size_t as size_t
        }
        carray_free(rr_event_to_send);
    }
    free(txt_raw as *mut libc::c_void);
    sqlite3_finalize(stmt);
}
/* ******************************************************************************
 * Misc. Tools
 ******************************************************************************/
unsafe extern "C" fn calc_timestamps(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut from_id: uint32_t,
    mut message_timestamp: time_t,
    mut is_fresh_msg: libc::c_int,
    mut sort_timestamp: *mut time_t,
    mut sent_timestamp: *mut time_t,
    mut rcvd_timestamp: *mut time_t,
) {
    *rcvd_timestamp = time(0 as *mut time_t);
    *sent_timestamp = message_timestamp;
    if *sent_timestamp > *rcvd_timestamp {
        *sent_timestamp = *rcvd_timestamp
    }
    *sort_timestamp = message_timestamp;
    if 0 != is_fresh_msg {
        let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT MAX(timestamp) FROM msgs WHERE chat_id=? and from_id!=? AND timestamp>=?\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        sqlite3_bind_int(stmt, 2i32, from_id as libc::c_int);
        sqlite3_bind_int64(stmt, 3i32, *sort_timestamp as sqlite3_int64);
        if sqlite3_step(stmt) == 100i32 {
            let mut last_msg_time: time_t = sqlite3_column_int64(stmt, 0i32) as time_t;
            if last_msg_time > 0i32 as libc::c_long {
                if *sort_timestamp <= last_msg_time {
                    *sort_timestamp = last_msg_time + 1i32 as libc::c_long
                }
            }
        }
        sqlite3_finalize(stmt);
    }
    if *sort_timestamp >= dc_smeared_time(context) {
        *sort_timestamp = dc_create_smeared_timestamp(context)
    };
}
/* the function tries extracts the group-id from the message and returns the
corresponding chat_id.  If the chat_id is not existant, it is created.
If the message contains groups commands (name, profile image, changed members),
they are executed as well.

if no group-id could be extracted from the message, create_or_lookup_adhoc_group() is called
which tries to create or find out the chat_id by:
- is there a group with the same recipients? if so, use this (if there are multiple, use the most recent one)
- create an ad-hoc group based on the recipient list

So when the function returns, the caller has the group id matching the current
state of the group. */
unsafe extern "C" fn create_or_lookup_group(
    mut context: *mut dc_context_t,
    mut mime_parser: *mut dc_mimeparser_t,
    mut allow_creation: libc::c_int,
    mut create_blocked: libc::c_int,
    mut from_id: int32_t,
    mut to_ids: *const dc_array_t,
    mut ret_chat_id: *mut uint32_t,
    mut ret_chat_id_blocked: *mut libc::c_int,
) {
    let mut group_explicitly_left: libc::c_int = 0;
    let mut current_block: u64;
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0i32;
    let mut chat_id_verified: libc::c_int = 0i32;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpname: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut i: libc::c_int = 0i32;
    let mut to_ids_cnt: libc::c_int = dc_array_get_cnt(to_ids) as libc::c_int;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut recreate_member_list: libc::c_int = 0i32;
    let mut send_EVENT_CHAT_MODIFIED: libc::c_int = 0i32;
    /* pointer somewhere into mime_parser, must not be freed */
    let mut X_MrRemoveFromGrp: *mut libc::c_char = 0 as *mut libc::c_char;
    /* pointer somewhere into mime_parser, must not be freed */
    let mut X_MrAddToGrp: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut X_MrGrpNameChanged: libc::c_int = 0i32;
    let mut X_MrGrpImageChanged: *const libc::c_char = 0 as *const libc::c_char;
    let mut better_msg: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut failure_reason: *mut libc::c_char = 0 as *mut libc::c_char;
    if (*mime_parser).is_system_message == 8i32 {
        better_msg = dc_stock_system_msg(
            context,
            64i32,
            0 as *const libc::c_char,
            0 as *const libc::c_char,
            from_id as uint32_t,
        )
    }
    set_better_msg(mime_parser, &mut better_msg);
    /* search the grpid in the header */
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    optional_field = dc_mimeparser_lookup_optional_field(
        mime_parser,
        b"Chat-Group-ID\x00" as *const u8 as *const libc::c_char,
    );
    if !optional_field.is_null() {
        grpid = dc_strdup((*optional_field).fld_value)
    }
    if grpid.is_null() {
        field = dc_mimeparser_lookup_field(
            mime_parser,
            b"Message-ID\x00" as *const u8 as *const libc::c_char,
        );
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_MESSAGE_ID as libc::c_int {
            let mut fld_message_id: *mut mailimf_message_id = (*field).fld_data.fld_message_id;
            if !fld_message_id.is_null() {
                grpid = dc_extract_grpid_from_rfc724_mid((*fld_message_id).mid_value)
            }
        }
        if grpid.is_null() {
            field = dc_mimeparser_lookup_field(
                mime_parser,
                b"In-Reply-To\x00" as *const u8 as *const libc::c_char,
            );
            if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
                let mut fld_in_reply_to: *mut mailimf_in_reply_to =
                    (*field).fld_data.fld_in_reply_to;
                if !fld_in_reply_to.is_null() {
                    grpid = dc_extract_grpid_from_rfc724_mid_list((*fld_in_reply_to).mid_list)
                }
            }
            if grpid.is_null() {
                field = dc_mimeparser_lookup_field(
                    mime_parser,
                    b"References\x00" as *const u8 as *const libc::c_char,
                );
                if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int
                {
                    let mut fld_references: *mut mailimf_references =
                        (*field).fld_data.fld_references;
                    if !fld_references.is_null() {
                        grpid = dc_extract_grpid_from_rfc724_mid_list((*fld_references).mid_list)
                    }
                }
                if grpid.is_null() {
                    create_or_lookup_adhoc_group(
                        context,
                        mime_parser,
                        allow_creation,
                        create_blocked,
                        from_id,
                        to_ids,
                        &mut chat_id,
                        &mut chat_id_blocked,
                    );
                    current_block = 281803052766328415;
                } else {
                    current_block = 18435049525520518667;
                }
            } else {
                current_block = 18435049525520518667;
            }
        } else {
            current_block = 18435049525520518667;
        }
    } else {
        current_block = 18435049525520518667;
    }
    match current_block {
        18435049525520518667 => {
            optional_field = dc_mimeparser_lookup_optional_field(
                mime_parser,
                b"Chat-Group-Name\x00" as *const u8 as *const libc::c_char,
            );
            if !optional_field.is_null() {
                grpname = dc_decode_header_words((*optional_field).fld_value)
            }
            optional_field = dc_mimeparser_lookup_optional_field(
                mime_parser,
                b"Chat-Group-Member-Removed\x00" as *const u8 as *const libc::c_char,
            );
            if !optional_field.is_null() {
                X_MrRemoveFromGrp = (*optional_field).fld_value;
                (*mime_parser).is_system_message = 5i32;
                let mut left_group: libc::c_int =
                    (dc_lookup_contact_id_by_addr(context, X_MrRemoveFromGrp)
                        == from_id as libc::c_uint) as libc::c_int;
                better_msg = dc_stock_system_msg(
                    context,
                    if 0 != left_group { 19i32 } else { 18i32 },
                    X_MrRemoveFromGrp,
                    0 as *const libc::c_char,
                    from_id as uint32_t,
                )
            } else {
                optional_field = dc_mimeparser_lookup_optional_field(
                    mime_parser,
                    b"Chat-Group-Member-Added\x00" as *const u8 as *const libc::c_char,
                );
                if !optional_field.is_null() {
                    X_MrAddToGrp = (*optional_field).fld_value;
                    (*mime_parser).is_system_message = 4i32;
                    optional_field = dc_mimeparser_lookup_optional_field(
                        mime_parser,
                        b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char,
                    );
                    if !optional_field.is_null() {
                        X_MrGrpImageChanged = (*optional_field).fld_value
                    }
                    better_msg = dc_stock_system_msg(
                        context,
                        17i32,
                        X_MrAddToGrp,
                        0 as *const libc::c_char,
                        from_id as uint32_t,
                    )
                } else {
                    optional_field = dc_mimeparser_lookup_optional_field(
                        mime_parser,
                        b"Chat-Group-Name-Changed\x00" as *const u8 as *const libc::c_char,
                    );
                    if !optional_field.is_null() {
                        X_MrGrpNameChanged = 1i32;
                        (*mime_parser).is_system_message = 2i32;
                        better_msg = dc_stock_system_msg(
                            context,
                            15i32,
                            (*optional_field).fld_value,
                            grpname,
                            from_id as uint32_t,
                        )
                    } else {
                        optional_field = dc_mimeparser_lookup_optional_field(
                            mime_parser,
                            b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char,
                        );
                        if !optional_field.is_null() {
                            X_MrGrpImageChanged = (*optional_field).fld_value;
                            (*mime_parser).is_system_message = 3i32;
                            better_msg = dc_stock_system_msg(
                                context,
                                if strcmp(
                                    X_MrGrpImageChanged,
                                    b"0\x00" as *const u8 as *const libc::c_char,
                                ) == 0i32
                                {
                                    33i32
                                } else {
                                    16i32
                                },
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                                from_id as uint32_t,
                            )
                        }
                    }
                }
            }
            set_better_msg(mime_parser, &mut better_msg);
            chat_id = dc_get_chat_id_by_grpid(
                context,
                grpid,
                &mut chat_id_blocked,
                &mut chat_id_verified,
            );
            if chat_id != 0i32 as libc::c_uint {
                if 0 != chat_id_verified
                    && 0 == check_verified_properties(
                        context,
                        mime_parser,
                        from_id as uint32_t,
                        to_ids,
                        &mut failure_reason,
                    )
                {
                    dc_mimeparser_repl_msg_by_error(mime_parser, failure_reason);
                }
            }
            if chat_id != 0i32 as libc::c_uint
                && 0 == dc_is_contact_in_chat(context, chat_id, from_id as uint32_t)
            {
                recreate_member_list = 1i32
            }
            /* check if the group does not exist but should be created */
            group_explicitly_left = dc_is_group_explicitly_left(context, grpid);
            self_addr = dc_sqlite3_get_config(
                (*context).sql,
                b"configured_addr\x00" as *const u8 as *const libc::c_char,
                b"\x00" as *const u8 as *const libc::c_char,
            );
            if chat_id == 0i32 as libc::c_uint
                && 0 == dc_mimeparser_is_mailinglist_message(mime_parser)
                && !grpid.is_null()
                && !grpname.is_null()
                && X_MrRemoveFromGrp.is_null()
                && (0 == group_explicitly_left
                    || !X_MrAddToGrp.is_null() && dc_addr_cmp(self_addr, X_MrAddToGrp) == 0i32)
            {
                /*otherwise, a pending "quit" message may pop up*/
                /*re-create explicitly left groups only if ourself is re-added*/
                let mut create_verified: libc::c_int = 0i32;
                if !dc_mimeparser_lookup_field(
                    mime_parser,
                    b"Chat-Verified\x00" as *const u8 as *const libc::c_char,
                )
                .is_null()
                {
                    create_verified = 1i32;
                    if 0 == check_verified_properties(
                        context,
                        mime_parser,
                        from_id as uint32_t,
                        to_ids,
                        &mut failure_reason,
                    ) {
                        dc_mimeparser_repl_msg_by_error(mime_parser, failure_reason);
                    }
                }
                if 0 == allow_creation {
                    current_block = 281803052766328415;
                } else {
                    chat_id = create_group_record(
                        context,
                        grpid,
                        grpname,
                        create_blocked,
                        create_verified,
                    );
                    chat_id_blocked = create_blocked;
                    chat_id_verified = create_verified;
                    recreate_member_list = 1i32;
                    current_block = 200744462051969938;
                }
            } else {
                current_block = 200744462051969938;
            }
            match current_block {
                281803052766328415 => {}
                _ => {
                    /* again, check chat_id */
                    if chat_id <= 9i32 as libc::c_uint {
                        chat_id = 0i32 as uint32_t;
                        if 0 != group_explicitly_left {
                            chat_id = 3i32 as uint32_t
                        } else {
                            create_or_lookup_adhoc_group(
                                context,
                                mime_parser,
                                allow_creation,
                                create_blocked,
                                from_id,
                                to_ids,
                                &mut chat_id,
                                &mut chat_id_blocked,
                            );
                        }
                    } else {
                        if !X_MrAddToGrp.is_null() || !X_MrRemoveFromGrp.is_null() {
                            recreate_member_list = 1i32
                        } else if 0 != X_MrGrpNameChanged
                            && !grpname.is_null()
                            && strlen(grpname) < 200i32 as libc::c_ulong
                        {
                            stmt = dc_sqlite3_prepare(
                                (*context).sql,
                                b"UPDATE chats SET name=? WHERE id=?;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            sqlite3_bind_text(stmt, 1i32, grpname, -1i32, None);
                            sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
                            sqlite3_step(stmt);
                            sqlite3_finalize(stmt);
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2020i32,
                                chat_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                        }
                        if !X_MrGrpImageChanged.is_null() {
                            let mut ok: libc::c_int = 0i32;
                            let mut grpimage: *mut libc::c_char = 0 as *mut libc::c_char;
                            if strcmp(
                                X_MrGrpImageChanged,
                                b"0\x00" as *const u8 as *const libc::c_char,
                            ) == 0i32
                            {
                                ok = 1i32
                            } else {
                                let mut i_0: libc::c_int = 0i32;
                                while (i_0 as libc::c_uint) < carray_count((*mime_parser).parts) {
                                    let mut part: *mut dc_mimepart_t =
                                        carray_get((*mime_parser).parts, i_0 as libc::c_uint)
                                            as *mut dc_mimepart_t;
                                    if (*part).type_0 == 20i32 {
                                        grpimage = dc_param_get(
                                            (*part).param,
                                            'f' as i32,
                                            0 as *const libc::c_char,
                                        );
                                        ok = 1i32
                                    }
                                    i_0 += 1
                                }
                            }
                            if 0 != ok {
                                let mut chat: *mut dc_chat_t = dc_chat_new(context);
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"New group image set to %s.\x00" as *const u8
                                        as *const libc::c_char,
                                    if !grpimage.is_null() {
                                        b"DELETED\x00" as *const u8 as *const libc::c_char
                                    } else {
                                        grpimage
                                    },
                                );
                                dc_chat_load_from_db(chat, chat_id);
                                dc_param_set((*chat).param, 'i' as i32, grpimage);
                                dc_chat_update_param(chat);
                                dc_chat_unref(chat);
                                free(grpimage as *mut libc::c_void);
                                send_EVENT_CHAT_MODIFIED = 1i32
                            }
                        }
                        if 0 != recreate_member_list {
                            let mut skip: *const libc::c_char = if !X_MrRemoveFromGrp.is_null() {
                                X_MrRemoveFromGrp
                            } else {
                                0 as *mut libc::c_char
                            };
                            stmt = dc_sqlite3_prepare(
                                (*context).sql,
                                b"DELETE FROM chats_contacts WHERE chat_id=?;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                            sqlite3_step(stmt);
                            sqlite3_finalize(stmt);
                            if skip.is_null() || dc_addr_cmp(self_addr, skip) != 0i32 {
                                dc_add_to_chat_contacts_table(context, chat_id, 1i32 as uint32_t);
                            }
                            if from_id > 9i32 {
                                if dc_addr_equals_contact(context, self_addr, from_id as uint32_t)
                                    == 0i32
                                    && (skip.is_null()
                                        || dc_addr_equals_contact(
                                            context,
                                            skip,
                                            from_id as uint32_t,
                                        ) == 0i32)
                                {
                                    dc_add_to_chat_contacts_table(
                                        context,
                                        chat_id,
                                        from_id as uint32_t,
                                    );
                                }
                            }
                            i = 0i32;
                            while i < to_ids_cnt {
                                let mut to_id: uint32_t = dc_array_get_id(to_ids, i as size_t);
                                if dc_addr_equals_contact(context, self_addr, to_id) == 0i32
                                    && (skip.is_null()
                                        || dc_addr_equals_contact(context, skip, to_id) == 0i32)
                                {
                                    dc_add_to_chat_contacts_table(context, chat_id, to_id);
                                }
                                i += 1
                            }
                            send_EVENT_CHAT_MODIFIED = 1i32;
                            dc_reset_gossiped_timestamp(context, chat_id);
                        }
                        if 0 != send_EVENT_CHAT_MODIFIED {
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2020i32,
                                chat_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                        }
                        /* check the number of receivers -
                        the only critical situation is if the user hits "Reply" instead of "Reply all" in a non-messenger-client */
                        if to_ids_cnt == 1i32 && (*mime_parser).is_send_by_messenger == 0i32 {
                            let mut is_contact_cnt: libc::c_int =
                                dc_get_chat_contact_cnt(context, chat_id);
                            if is_contact_cnt > 3i32 {
                                /* to_ids_cnt==1 may be "From: A, To: B, SELF" as SELF is not counted in to_ids_cnt. So everything up to 3 is no error. */
                                chat_id = 0i32 as uint32_t;
                                create_or_lookup_adhoc_group(
                                    context,
                                    mime_parser,
                                    allow_creation,
                                    create_blocked,
                                    from_id,
                                    to_ids,
                                    &mut chat_id,
                                    &mut chat_id_blocked,
                                );
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    free(grpid as *mut libc::c_void);
    free(grpname as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    free(better_msg as *mut libc::c_void);
    free(failure_reason as *mut libc::c_void);
    if !ret_chat_id.is_null() {
        *ret_chat_id = chat_id
    }
    if !ret_chat_id_blocked.is_null() {
        *ret_chat_id_blocked = if 0 != chat_id { chat_id_blocked } else { 0i32 }
    };
}
/* ******************************************************************************
 * Handle groups for received messages
 ******************************************************************************/
unsafe extern "C" fn create_or_lookup_adhoc_group(
    mut context: *mut dc_context_t,
    mut mime_parser: *mut dc_mimeparser_t,
    mut allow_creation: libc::c_int,
    mut create_blocked: libc::c_int,
    mut from_id: int32_t,
    mut to_ids: *const dc_array_t,
    mut ret_chat_id: *mut uint32_t,
    mut ret_chat_id_blocked: *mut libc::c_int,
) {
    let mut current_block: u64;
    /* if we're here, no grpid was found, check there is an existing ad-hoc
    group matching the to-list or if we can create one */
    let mut member_ids: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    let mut chat_ids: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut chat_ids_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpname: *mut libc::c_char = 0 as *mut libc::c_char;
    /* build member list from the given ids */
    if !(dc_array_get_cnt(to_ids) == 0i32 as libc::c_ulong
        || 0 != dc_mimeparser_is_mailinglist_message(mime_parser))
    {
        /* too few contacts or a mailinglist */
        member_ids = dc_array_duplicate(to_ids);
        if 0 == dc_array_search_id(member_ids, from_id as uint32_t, 0 as *mut size_t) {
            dc_array_add_id(member_ids, from_id as uint32_t);
        }
        if 0 == dc_array_search_id(member_ids, 1i32 as uint32_t, 0 as *mut size_t) {
            dc_array_add_id(member_ids, 1i32 as uint32_t);
        }
        if !(dc_array_get_cnt(member_ids) < 3i32 as libc::c_ulong) {
            /* too few contacts given */
            chat_ids = search_chat_ids_by_contact_ids(context, member_ids);
            if dc_array_get_cnt(chat_ids) > 0i32 as libc::c_ulong {
                chat_ids_str =
                    dc_array_get_string(chat_ids, b",\x00" as *const u8 as *const libc::c_char);
                q3 =
                    sqlite3_mprintf(b"SELECT c.id, c.blocked  FROM chats c  LEFT JOIN msgs m ON m.chat_id=c.id  WHERE c.id IN(%s)  ORDER BY m.timestamp DESC, m.id DESC  LIMIT 1;\x00"
                                        as *const u8 as *const libc::c_char,
                                    chat_ids_str);
                stmt = dc_sqlite3_prepare((*context).sql, q3);
                if sqlite3_step(stmt) == 100i32 {
                    chat_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
                    chat_id_blocked = sqlite3_column_int(stmt, 1i32);
                    /* success, chat found */
                    current_block = 11334989263469503965;
                } else {
                    current_block = 11194104282611034094;
                }
            } else {
                current_block = 11194104282611034094;
            }
            match current_block {
                11334989263469503965 => {}
                _ => {
                    if !(0 == allow_creation) {
                        /* we do not check if the message is a reply to another group, this may result in
                        chats with unclear member list. instead we create a new group in the following lines ... */
                        /* create a new ad-hoc group
                        - there is no need to check if this group exists; otherwise we would have catched it above */
                        grpid = create_adhoc_grp_id(context, member_ids);
                        if !grpid.is_null() {
                            if !(*mime_parser).subject.is_null()
                                && 0 != *(*mime_parser).subject.offset(0isize) as libc::c_int
                            {
                                grpname = dc_strdup((*mime_parser).subject)
                            } else {
                                grpname = dc_stock_str_repl_int(
                                    context,
                                    4i32,
                                    dc_array_get_cnt(member_ids) as libc::c_int,
                                )
                            }
                            chat_id =
                                create_group_record(context, grpid, grpname, create_blocked, 0i32);
                            chat_id_blocked = create_blocked;
                            i = 0i32;
                            while (i as libc::c_ulong) < dc_array_get_cnt(member_ids) {
                                dc_add_to_chat_contacts_table(
                                    context,
                                    chat_id,
                                    dc_array_get_id(member_ids, i as size_t),
                                );
                                i += 1
                            }
                            (*context).cb.expect("non-null function pointer")(
                                context,
                                2020i32,
                                chat_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                        }
                    }
                }
            }
        }
    }
    dc_array_unref(member_ids);
    dc_array_unref(chat_ids);
    free(chat_ids_str as *mut libc::c_void);
    free(grpid as *mut libc::c_void);
    free(grpname as *mut libc::c_void);
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    if !ret_chat_id.is_null() {
        *ret_chat_id = chat_id
    }
    if !ret_chat_id_blocked.is_null() {
        *ret_chat_id_blocked = chat_id_blocked
    };
}
unsafe extern "C" fn create_group_record(
    mut context: *mut dc_context_t,
    mut grpid: *const libc::c_char,
    mut grpname: *const libc::c_char,
    mut create_blocked: libc::c_int,
    mut create_verified: libc::c_int,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"INSERT INTO chats (type, name, grpid, blocked) VALUES(?, ?, ?, ?);\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(
        stmt,
        1i32,
        if 0 != create_verified { 130i32 } else { 120i32 },
    );
    sqlite3_bind_text(stmt, 2i32, grpname, -1i32, None);
    sqlite3_bind_text(stmt, 3i32, grpid, -1i32, None);
    sqlite3_bind_int(stmt, 4i32, create_blocked);
    if !(sqlite3_step(stmt) != 101i32) {
        chat_id = dc_sqlite3_get_rowid(
            (*context).sql,
            b"chats\x00" as *const u8 as *const libc::c_char,
            b"grpid\x00" as *const u8 as *const libc::c_char,
            grpid,
        )
    }
    sqlite3_finalize(stmt);
    return chat_id;
}
unsafe extern "C" fn create_adhoc_grp_id(
    mut context: *mut dc_context_t,
    mut member_ids: *mut dc_array_t,
) -> *mut libc::c_char {
    /* algorithm:
    - sort normalized, lowercased, e-mail addresses alphabetically
    - put all e-mail addresses into a single string, separate the addresss by a single comma
    - sha-256 this string (without possibly terminating null-characters)
    - encode the first 64 bits of the sha-256 output as lowercase hex (results in 16 characters from the set [0-9a-f])
     */
    let mut member_addrs: *mut dc_array_t = dc_array_new(context, 23i32 as size_t);
    let mut member_ids_str: *mut libc::c_char =
        dc_array_get_string(member_ids, b",\x00" as *const u8 as *const libc::c_char);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut i: libc::c_int = 0i32;
    let mut iCnt: libc::c_int = 0i32;
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut member_cs: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut member_cs, 0i32);
    q3 = sqlite3_mprintf(
        b"SELECT addr FROM contacts WHERE id IN(%s) AND id!=1\x00" as *const u8
            as *const libc::c_char,
        member_ids_str,
    );
    stmt = dc_sqlite3_prepare((*context).sql, q3);
    addr = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        b"no-self\x00" as *const u8 as *const libc::c_char,
    );
    dc_strlower_in_place(addr);
    dc_array_add_ptr(member_addrs, addr as *mut libc::c_void);
    while sqlite3_step(stmt) == 100i32 {
        addr = dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char);
        dc_strlower_in_place(addr);
        dc_array_add_ptr(member_addrs, addr as *mut libc::c_void);
    }
    dc_array_sort_strings(member_addrs);
    iCnt = dc_array_get_cnt(member_addrs) as libc::c_int;
    i = 0i32;
    while i < iCnt {
        if 0 != i {
            dc_strbuilder_cat(&mut member_cs, b",\x00" as *const u8 as *const libc::c_char);
        }
        dc_strbuilder_cat(
            &mut member_cs,
            dc_array_get_ptr(member_addrs, i as size_t) as *const libc::c_char,
        );
        i += 1
    }
    /* make sha-256 from the string */
    let mut binary_hash: *mut rpgp_cvec =
        rpgp_hash_sha256(member_cs.buf as *const uint8_t, strlen(member_cs.buf));
    if !binary_hash.is_null() {
        ret = calloc(1i32 as libc::c_ulong, 256i32 as libc::c_ulong) as *mut libc::c_char;
        if !ret.is_null() {
            i = 0i32;
            while i < 8i32 {
                sprintf(
                    &mut *ret.offset((i * 2i32) as isize) as *mut libc::c_char,
                    b"%02x\x00" as *const u8 as *const libc::c_char,
                    *rpgp_cvec_data(binary_hash).offset(i as isize) as libc::c_int,
                );
                i += 1
            }
            rpgp_cvec_drop(binary_hash);
        }
    }
    dc_array_free_ptr(member_addrs);
    dc_array_unref(member_addrs);
    free(member_ids_str as *mut libc::c_void);
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    free(member_cs.buf as *mut libc::c_void);
    return ret;
}
unsafe extern "C" fn search_chat_ids_by_contact_ids(
    mut context: *mut dc_context_t,
    mut unsorted_contact_ids: *const dc_array_t,
) -> *mut dc_array_t {
    /* searches chat_id's by the given contact IDs, may return zero, one or more chat_id's */
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut contact_ids: *mut dc_array_t = dc_array_new(context, 23i32 as size_t);
    let mut contact_ids_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut chat_ids: *mut dc_array_t = dc_array_new(context, 23i32 as size_t);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        /* copy array, remove duplicates and SELF, sort by ID */
        let mut i: libc::c_int = 0;
        let mut iCnt: libc::c_int = dc_array_get_cnt(unsorted_contact_ids) as libc::c_int;
        if !(iCnt <= 0i32) {
            i = 0i32;
            while i < iCnt {
                let mut curr_id: uint32_t = dc_array_get_id(unsorted_contact_ids, i as size_t);
                if curr_id != 1i32 as libc::c_uint
                    && 0 == dc_array_search_id(contact_ids, curr_id, 0 as *mut size_t)
                {
                    dc_array_add_id(contact_ids, curr_id);
                }
                i += 1
            }
            if !(dc_array_get_cnt(contact_ids) == 0i32 as libc::c_ulong) {
                dc_array_sort_ids(contact_ids);
                contact_ids_str =
                    dc_array_get_string(contact_ids, b",\x00" as *const u8 as *const libc::c_char);
                q3 =
                    sqlite3_mprintf(b"SELECT DISTINCT cc.chat_id, cc.contact_id  FROM chats_contacts cc  LEFT JOIN chats c ON c.id=cc.chat_id  WHERE cc.chat_id IN(SELECT chat_id FROM chats_contacts WHERE contact_id IN(%s))   AND c.type=120   AND cc.contact_id!=1 ORDER BY cc.chat_id, cc.contact_id;\x00"
                                        as *const u8 as *const libc::c_char,
                                    contact_ids_str);
                stmt = dc_sqlite3_prepare((*context).sql, q3);
                let mut last_chat_id: uint32_t = 0i32 as uint32_t;
                let mut matches: uint32_t = 0i32 as uint32_t;
                let mut mismatches: uint32_t = 0i32 as uint32_t;
                while sqlite3_step(stmt) == 100i32 {
                    let mut chat_id: uint32_t = sqlite3_column_int(stmt, 0i32) as uint32_t;
                    let mut contact_id: uint32_t = sqlite3_column_int(stmt, 1i32) as uint32_t;
                    if chat_id != last_chat_id {
                        if matches as libc::c_ulong == dc_array_get_cnt(contact_ids)
                            && mismatches == 0i32 as libc::c_uint
                        {
                            dc_array_add_id(chat_ids, last_chat_id);
                        }
                        last_chat_id = chat_id;
                        matches = 0i32 as uint32_t;
                        mismatches = 0i32 as uint32_t
                    }
                    if contact_id == dc_array_get_id(contact_ids, matches as size_t) {
                        matches = matches.wrapping_add(1)
                    } else {
                        mismatches = mismatches.wrapping_add(1)
                    }
                }
                if matches as libc::c_ulong == dc_array_get_cnt(contact_ids)
                    && mismatches == 0i32 as libc::c_uint
                {
                    dc_array_add_id(chat_ids, last_chat_id);
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    free(contact_ids_str as *mut libc::c_void);
    dc_array_unref(contact_ids);
    sqlite3_free(q3 as *mut libc::c_void);
    return chat_ids;
}
unsafe extern "C" fn check_verified_properties(
    mut context: *mut dc_context_t,
    mut mimeparser: *mut dc_mimeparser_t,
    mut from_id: uint32_t,
    mut to_ids: *const dc_array_t,
    mut failure_reason: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut everythings_okay: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = dc_contact_new(context);
    let mut peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
    let mut to_ids_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if 0 == dc_contact_load_from_db(contact, (*context).sql, from_id) {
        *failure_reason = dc_mprintf(
            b"%s. See \"Info\" for details.\x00" as *const u8 as *const libc::c_char,
            b"Internal Error; cannot load contact.\x00" as *const u8 as *const libc::c_char,
        );
        dc_log_warning(context, 0i32, *failure_reason);
    } else if 0 == (*(*mimeparser).e2ee_helper).encrypted {
        *failure_reason = dc_mprintf(
            b"%s. See \"Info\" for details.\x00" as *const u8 as *const libc::c_char,
            b"This message is not encrypted.\x00" as *const u8 as *const libc::c_char,
        );
        dc_log_warning(context, 0i32, *failure_reason);
    } else {
        // ensure, the contact is verified
        // and the message is signed with a verified key of the sender.
        // this check is skipped for SELF as there is no proper SELF-peerstate
        // and results in group-splits otherwise.
        if from_id != 1i32 as libc::c_uint {
            if 0 == dc_apeerstate_load_by_addr(peerstate, (*context).sql, (*contact).addr)
                || dc_contact_is_verified_ex(contact, peerstate) != 2i32
            {
                *failure_reason = dc_mprintf(
                    b"%s. See \"Info\" for details.\x00" as *const u8 as *const libc::c_char,
                    b"The sender of this message is not verified.\x00" as *const u8
                        as *const libc::c_char,
                );
                dc_log_warning(context, 0i32, *failure_reason);
                current_block = 14837890932895028253;
            } else if 0
                == dc_apeerstate_has_verified_key(
                    peerstate,
                    (*(*mimeparser).e2ee_helper).signatures,
                )
            {
                *failure_reason = dc_mprintf(
                    b"%s. See \"Info\" for details.\x00" as *const u8 as *const libc::c_char,
                    b"The message was sent with non-verified encryption.\x00" as *const u8
                        as *const libc::c_char,
                );
                dc_log_warning(context, 0i32, *failure_reason);
                current_block = 14837890932895028253;
            } else {
                current_block = 15904375183555213903;
            }
        } else {
            current_block = 15904375183555213903;
        }
        match current_block {
            14837890932895028253 => {}
            _ => {
                to_ids_str =
                    dc_array_get_string(to_ids, b",\x00" as *const u8 as *const libc::c_char);
                q3 =
                    sqlite3_mprintf(b"SELECT c.addr, LENGTH(ps.verified_key_fingerprint)  FROM contacts c  LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.id IN(%s) \x00"
                                        as *const u8 as *const libc::c_char,
                                    to_ids_str);
                stmt = dc_sqlite3_prepare((*context).sql, q3);
                loop {
                    if !(sqlite3_step(stmt) == 100i32) {
                        current_block = 2604890879466389055;
                        break;
                    }
                    let mut to_addr: *const libc::c_char =
                        sqlite3_column_text(stmt, 0i32) as *const libc::c_char;
                    let mut is_verified: libc::c_int = sqlite3_column_int(stmt, 1i32);
                    if !dc_hash_find(
                        (*(*mimeparser).e2ee_helper).gossipped_addr,
                        to_addr as *const libc::c_void,
                        strlen(to_addr) as libc::c_int,
                    )
                    .is_null()
                        && 0 != dc_apeerstate_load_by_addr(peerstate, (*context).sql, to_addr)
                    {
                        if 0 == is_verified
                            || strcmp(
                                (*peerstate).verified_key_fingerprint,
                                (*peerstate).public_key_fingerprint,
                            ) != 0i32
                                && strcmp(
                                    (*peerstate).verified_key_fingerprint,
                                    (*peerstate).gossip_key_fingerprint,
                                ) != 0i32
                        {
                            dc_log_info(
                                context,
                                0i32,
                                b"%s has verfied %s.\x00" as *const u8 as *const libc::c_char,
                                (*contact).addr,
                                to_addr,
                            );
                            dc_apeerstate_set_verified(
                                peerstate,
                                0i32,
                                (*peerstate).gossip_key_fingerprint,
                                2i32,
                            );
                            dc_apeerstate_save_to_db(peerstate, (*context).sql, 0i32);
                            is_verified = 1i32
                        }
                    }
                    if !(0 == is_verified) {
                        continue;
                    }
                    let mut err: *mut libc::c_char = dc_mprintf(
                        b"%s is not a member of this verified group.\x00" as *const u8
                            as *const libc::c_char,
                        to_addr,
                    );
                    *failure_reason = dc_mprintf(
                        b"%s. See \"Info\" for details.\x00" as *const u8 as *const libc::c_char,
                        err,
                    );
                    dc_log_warning(context, 0i32, *failure_reason);
                    free(err as *mut libc::c_void);
                    current_block = 14837890932895028253;
                    break;
                }
                match current_block {
                    14837890932895028253 => {}
                    _ => everythings_okay = 1i32,
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    dc_contact_unref(contact);
    dc_apeerstate_unref(peerstate);
    free(to_ids_str as *mut libc::c_void);
    sqlite3_free(q3 as *mut libc::c_void);
    return everythings_okay;
}
unsafe extern "C" fn set_better_msg(
    mut mime_parser: *mut dc_mimeparser_t,
    mut better_msg: *mut *mut libc::c_char,
) {
    if !(*better_msg).is_null() && carray_count((*mime_parser).parts) > 0i32 as libc::c_uint {
        let mut part: *mut dc_mimepart_t =
            carray_get((*mime_parser).parts, 0i32 as libc::c_uint) as *mut dc_mimepart_t;
        if (*part).type_0 == 10i32 {
            free((*part).msg as *mut libc::c_void);
            (*part).msg = *better_msg;
            *better_msg = 0 as *mut libc::c_char
        }
    };
}
unsafe extern "C" fn dc_is_reply_to_known_message(
    mut context: *mut dc_context_t,
    mut mime_parser: *mut dc_mimeparser_t,
) -> libc::c_int {
    /* check if the message is a reply to a known message; the replies are identified by the Message-ID from
    `In-Reply-To`/`References:` (to support non-Delta-Clients) or from `Chat-Predecessor:` (Delta clients, see comment in dc_chat.c) */
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    optional_field = dc_mimeparser_lookup_optional_field(
        mime_parser,
        b"Chat-Predecessor\x00" as *const u8 as *const libc::c_char,
    );
    if !optional_field.is_null() {
        if 0 != is_known_rfc724_mid(context, (*optional_field).fld_value) {
            return 1i32;
        }
    }
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    field = dc_mimeparser_lookup_field(
        mime_parser,
        b"In-Reply-To\x00" as *const u8 as *const libc::c_char,
    );
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
        let mut fld_in_reply_to: *mut mailimf_in_reply_to = (*field).fld_data.fld_in_reply_to;
        if !fld_in_reply_to.is_null() {
            if 0 != is_known_rfc724_mid_in_list(
                context,
                (*(*field).fld_data.fld_in_reply_to).mid_list,
            ) {
                return 1i32;
            }
        }
    }
    field = dc_mimeparser_lookup_field(
        mime_parser,
        b"References\x00" as *const u8 as *const libc::c_char,
    );
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int {
        let mut fld_references: *mut mailimf_references = (*field).fld_data.fld_references;
        if !fld_references.is_null() {
            if 0 != is_known_rfc724_mid_in_list(
                context,
                (*(*field).fld_data.fld_references).mid_list,
            ) {
                return 1i32;
            }
        }
    }
    return 0i32;
}
unsafe extern "C" fn is_known_rfc724_mid_in_list(
    mut context: *mut dc_context_t,
    mut mid_list: *const clist,
) -> libc::c_int {
    if !mid_list.is_null() {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*mid_list).first;
        while !cur.is_null() {
            if 0 != is_known_rfc724_mid(
                context,
                (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char,
            ) {
                return 1i32;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell_s
            }
        }
    }
    return 0i32;
}
/* ******************************************************************************
 * Check if a message is a reply to a known message (messenger or non-messenger)
 ******************************************************************************/
unsafe extern "C" fn is_known_rfc724_mid(
    mut context: *mut dc_context_t,
    mut rfc724_mid: *const libc::c_char,
) -> libc::c_int {
    let mut is_known: libc::c_int = 0i32;
    if !rfc724_mid.is_null() {
        let mut stmt: *mut sqlite3_stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT m.id FROM msgs m  LEFT JOIN chats c ON m.chat_id=c.id  WHERE m.rfc724_mid=?  AND m.chat_id>9 AND c.blocked=0;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_text(stmt, 1i32, rfc724_mid, -1i32, None);
        if sqlite3_step(stmt) == 100i32 {
            is_known = 1i32
        }
        sqlite3_finalize(stmt);
    }
    return is_known;
}
unsafe extern "C" fn dc_is_reply_to_messenger_message(
    mut context: *mut dc_context_t,
    mut mime_parser: *mut dc_mimeparser_t,
) -> libc::c_int {
    /* function checks, if the message defined by mime_parser references a message send by us from Delta Chat.
    This is similar to is_reply_to_known_message() but
    - checks also if any of the referenced IDs are send by a messenger
    - it is okay, if the referenced messages are moved to trash here
    - no check for the Chat-* headers (function is only called if it is no messenger message itself) */
    let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
    field = dc_mimeparser_lookup_field(
        mime_parser,
        b"In-Reply-To\x00" as *const u8 as *const libc::c_char,
    );
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
        let mut fld_in_reply_to: *mut mailimf_in_reply_to = (*field).fld_data.fld_in_reply_to;
        if !fld_in_reply_to.is_null() {
            if 0 != is_msgrmsg_rfc724_mid_in_list(
                context,
                (*(*field).fld_data.fld_in_reply_to).mid_list,
            ) {
                return 1i32;
            }
        }
    }
    field = dc_mimeparser_lookup_field(
        mime_parser,
        b"References\x00" as *const u8 as *const libc::c_char,
    );
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int {
        let mut fld_references: *mut mailimf_references = (*field).fld_data.fld_references;
        if !fld_references.is_null() {
            if 0 != is_msgrmsg_rfc724_mid_in_list(
                context,
                (*(*field).fld_data.fld_references).mid_list,
            ) {
                return 1i32;
            }
        }
    }
    return 0i32;
}
unsafe extern "C" fn is_msgrmsg_rfc724_mid_in_list(
    mut context: *mut dc_context_t,
    mut mid_list: *const clist,
) -> libc::c_int {
    if !mid_list.is_null() {
        let mut cur: *mut clistiter = (*mid_list).first;
        while !cur.is_null() {
            if 0 != is_msgrmsg_rfc724_mid(
                context,
                (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char,
            ) {
                return 1i32;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell_s
            }
        }
    }
    return 0i32;
}
/* ******************************************************************************
 * Check if a message is a reply to any messenger message
 ******************************************************************************/
unsafe extern "C" fn is_msgrmsg_rfc724_mid(
    mut context: *mut dc_context_t,
    mut rfc724_mid: *const libc::c_char,
) -> libc::c_int {
    let mut is_msgrmsg: libc::c_int = 0i32;
    if !rfc724_mid.is_null() {
        let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"SELECT id FROM msgs  WHERE rfc724_mid=?  AND msgrmsg!=0  AND chat_id>9;\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, rfc724_mid, -1i32, None);
        if sqlite3_step(stmt) == 100i32 {
            is_msgrmsg = 1i32
        }
        sqlite3_finalize(stmt);
    }
    return is_msgrmsg;
}
unsafe extern "C" fn dc_add_or_lookup_contacts_by_address_list(
    mut context: *mut dc_context_t,
    mut adr_list: *const mailimf_address_list,
    mut origin: libc::c_int,
    mut ids: *mut dc_array_t,
    mut check_self: *mut libc::c_int,
) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || adr_list.is_null()
    {
        return;
    }
    let mut cur: *mut clistiter = (*(*adr_list).ad_list).first;
    while !cur.is_null() {
        let mut adr: *mut mailimf_address = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_address;
        if !adr.is_null() {
            if (*adr).ad_type == MAILIMF_ADDRESS_MAILBOX as libc::c_int {
                let mut mb: *mut mailimf_mailbox = (*adr).ad_data.ad_mailbox;
                if !mb.is_null() {
                    add_or_lookup_contact_by_addr(
                        context,
                        (*mb).mb_display_name,
                        (*mb).mb_addr_spec,
                        origin,
                        ids,
                        check_self,
                    );
                }
            } else if (*adr).ad_type == MAILIMF_ADDRESS_GROUP as libc::c_int {
                let mut group: *mut mailimf_group = (*adr).ad_data.ad_group;
                if !group.is_null() && !(*group).grp_mb_list.is_null() {
                    dc_add_or_lookup_contacts_by_mailbox_list(
                        context,
                        (*group).grp_mb_list,
                        origin,
                        ids,
                        check_self,
                    );
                }
            }
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell_s
        }
    }
}
unsafe extern "C" fn dc_add_or_lookup_contacts_by_mailbox_list(
    mut context: *mut dc_context_t,
    mut mb_list: *const mailimf_mailbox_list,
    mut origin: libc::c_int,
    mut ids: *mut dc_array_t,
    mut check_self: *mut libc::c_int,
) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || mb_list.is_null() {
        return;
    }
    let mut cur: *mut clistiter = (*(*mb_list).mb_list).first;
    while !cur.is_null() {
        let mut mb: *mut mailimf_mailbox = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_mailbox;
        if !mb.is_null() {
            add_or_lookup_contact_by_addr(
                context,
                (*mb).mb_display_name,
                (*mb).mb_addr_spec,
                origin,
                ids,
                check_self,
            );
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell_s
        }
    }
}
/* ******************************************************************************
 * Add contacts to database on receiving messages
 ******************************************************************************/
unsafe extern "C" fn add_or_lookup_contact_by_addr(
    mut context: *mut dc_context_t,
    mut display_name_enc: *const libc::c_char,
    mut addr_spec: *const libc::c_char,
    mut origin: libc::c_int,
    mut ids: *mut dc_array_t,
    mut check_self: *mut libc::c_int,
) {
    /* is addr_spec equal to SELF? */
    let mut dummy: libc::c_int = 0i32;
    if check_self.is_null() {
        check_self = &mut dummy
    }
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || addr_spec.is_null()
    {
        return;
    }
    *check_self = 0i32;
    let mut self_addr: *mut libc::c_char = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        b"\x00" as *const u8 as *const libc::c_char,
    );
    if dc_addr_cmp(self_addr, addr_spec) == 0i32 {
        *check_self = 1i32
    }
    free(self_addr as *mut libc::c_void);
    if 0 != *check_self {
        return;
    }
    /* add addr_spec if missing, update otherwise */
    let mut display_name_dec: *mut libc::c_char = 0 as *mut libc::c_char;
    if !display_name_enc.is_null() {
        display_name_dec = dc_decode_header_words(display_name_enc);
        dc_normalize_name(display_name_dec);
    }
    /*can be NULL*/
    let mut row_id: uint32_t = dc_add_or_lookup_contact(
        context,
        display_name_dec,
        addr_spec,
        origin,
        0 as *mut libc::c_int,
    );
    free(display_name_dec as *mut libc::c_void);
    if 0 != row_id {
        if 0 == dc_array_search_id(ids, row_id, 0 as *mut size_t) {
            dc_array_add_id(ids, row_id);
        }
    };
}

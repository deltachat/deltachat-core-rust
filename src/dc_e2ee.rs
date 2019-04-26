use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type mailstream_cancel;
    pub type sqlite3;
    #[no_mangle]
    fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn clock() -> clock_t;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn pthread_self() -> pthread_t;
    #[no_mangle]
    fn mmap_string_new(init: *const libc::c_char) -> *mut MMAPString;
    #[no_mangle]
    fn mmap_string_free(string: *mut MMAPString);
    #[no_mangle]
    fn mmap_string_unref(str: *mut libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn clist_insert_after(_: *mut clist, _: *mut clistiter, _: *mut libc::c_void) -> libc::c_int;
    #[no_mangle]
    fn clist_delete(_: *mut clist, _: *mut clistiter) -> *mut clistiter;
    #[no_mangle]
    fn mailimf_fields_free(fields: *mut mailimf_fields);
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
    fn mailimf_fields_new_empty() -> *mut mailimf_fields;
    #[no_mangle]
    fn mailimf_fields_add(fields: *mut mailimf_fields, field: *mut mailimf_field) -> libc::c_int;
    #[no_mangle]
    fn mailimf_field_new_custom(
        name: *mut libc::c_char,
        value: *mut libc::c_char,
    ) -> *mut mailimf_field;
    #[no_mangle]
    fn mailimf_envelope_and_optional_fields_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailimf_fields,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_content_free(content: *mut mailmime_content);
    #[no_mangle]
    fn mailmime_mechanism_new(
        enc_type: libc::c_int,
        enc_token: *mut libc::c_char,
    ) -> *mut mailmime_mechanism;
    #[no_mangle]
    fn mailmime_mechanism_free(mechanism: *mut mailmime_mechanism);
    #[no_mangle]
    fn mailmime_fields_free(fields: *mut mailmime_fields);
    #[no_mangle]
    fn mailmime_new(
        mm_type: libc::c_int,
        mm_mime_start: *const libc::c_char,
        mm_length: size_t,
        mm_mime_fields: *mut mailmime_fields,
        mm_content_type: *mut mailmime_content,
        mm_body: *mut mailmime_data,
        mm_preamble: *mut mailmime_data,
        mm_epilogue: *mut mailmime_data,
        mm_mp_list: *mut clist,
        mm_fields: *mut mailimf_fields,
        mm_msg_mime: *mut mailmime,
    ) -> *mut mailmime;
    #[no_mangle]
    fn mailmime_free(mime: *mut mailmime);
    #[no_mangle]
    fn mailmime_fields_new_empty() -> *mut mailmime_fields;
    #[no_mangle]
    fn mailmime_fields_new_with_data(
        encoding: *mut mailmime_mechanism,
        id: *mut libc::c_char,
        description: *mut libc::c_char,
        disposition: *mut mailmime_disposition,
        language: *mut mailmime_language,
    ) -> *mut mailmime_fields;
    #[no_mangle]
    fn mailmime_get_content_message() -> *mut mailmime_content;
    #[no_mangle]
    fn mailmime_new_empty(
        content: *mut mailmime_content,
        mime_fields: *mut mailmime_fields,
    ) -> *mut mailmime;
    #[no_mangle]
    fn mailmime_set_body_text(
        build_info: *mut mailmime,
        data_str: *mut libc::c_char,
        length: size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_smart_add_part(mime: *mut mailmime, mime_sub: *mut mailmime) -> libc::c_int;
    #[no_mangle]
    fn mailmime_content_new_with_str(str: *const libc::c_char) -> *mut mailmime_content;
    #[no_mangle]
    fn mailmime_param_new_with_data(
        name: *mut libc::c_char,
        value: *mut libc::c_char,
    ) -> *mut mailmime_parameter;
    #[no_mangle]
    fn mailmime_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailmime,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_part_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        encoding: libc::c_int,
        result: *mut *mut libc::c_char,
        result_len: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_write_mem(
        f: *mut MMAPString,
        col: *mut libc::c_int,
        build_info: *mut mailmime,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_substitute(old_mime: *mut mailmime, new_mime: *mut mailmime) -> libc::c_int;
    #[no_mangle]
    fn mailprivacy_prepare_mime(mime: *mut mailmime);
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
    fn dc_array_add_ptr(_: *mut dc_array_t, _: *mut libc::c_void);
    #[no_mangle]
    fn dc_array_get_cnt(_: *const dc_array_t) -> size_t;
    #[no_mangle]
    fn dc_array_get_ptr(_: *const dc_array_t, index: size_t) -> *mut libc::c_void;
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
    /* date/time tools */
    #[no_mangle]
    fn dc_timestamp_from_date(date_time: *mut mailimf_date_time) -> time_t;
    #[no_mangle]
    fn dc_array_new(_: *mut dc_context_t, initsize: size_t) -> *mut dc_array_t;
    #[no_mangle]
    fn dc_key_new() -> *mut dc_key_t;
    #[no_mangle]
    fn dc_key_unref(_: *mut dc_key_t);
    #[no_mangle]
    fn dc_key_save_self_keypair(
        public_key: *const dc_key_t,
        private_key: *const dc_key_t,
        addr: *const libc::c_char,
        is_default: libc::c_int,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_key_load_self_public(
        _: *mut dc_key_t,
        self_addr: *const libc::c_char,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_key_load_self_private(
        _: *mut dc_key_t,
        self_addr: *const libc::c_char,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
    /* the returned pointer is ref'd and must be unref'd after usage */
    #[no_mangle]
    fn dc_aheader_new() -> *mut dc_aheader_t;
    #[no_mangle]
    fn dc_aheader_new_from_imffields(
        wanted_from: *const libc::c_char,
        mime: *const mailimf_fields,
    ) -> *mut dc_aheader_t;
    #[no_mangle]
    fn dc_aheader_unref(_: *mut dc_aheader_t);
    #[no_mangle]
    fn dc_aheader_set_from_string(
        _: *mut dc_aheader_t,
        header_str: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_aheader_render(_: *const dc_aheader_t) -> *mut libc::c_char;
    /*
     * There are 4 different modes of operation for a hash table:
     *
     *   DC_HASH_INT         nKey is used as the key and pKey is ignored.
     *
     *   DC_HASH_POINTER     pKey is used as the key and nKey is ignored.
     *
     *   DC_HASH_STRING      pKey points to a string that is nKey bytes long
     *                      (including the null-terminator, if any).  Case
     *                      is ignored in comparisons.
     *
     *   DC_HASH_BINARY      pKey points to binary data nKey bytes long.
     *                      memcmp() is used to compare keys.
     *
     * A copy of the key is made for DC_HASH_STRING and DC_HASH_BINARY
     * if the copyKey parameter to dc_hash_init() is 1.
     */
    /*
     * Just to make the last parameter of dc_hash_init() more readable.
     */
    /*
     * Access routines.  To delete an element, insert a NULL pointer.
     */
    #[no_mangle]
    fn dc_hash_init(_: *mut dc_hash_t, keytype: libc::c_int, copyKey: libc::c_int);
    #[no_mangle]
    fn dc_hash_insert(
        _: *mut dc_hash_t,
        pKey: *const libc::c_void,
        nKey: libc::c_int,
        pData: *mut libc::c_void,
    ) -> *mut libc::c_void;
    #[no_mangle]
    fn dc_hash_find(
        _: *const dc_hash_t,
        pKey: *const libc::c_void,
        nKey: libc::c_int,
    ) -> *mut libc::c_void;
    #[no_mangle]
    fn dc_hash_clear(_: *mut dc_hash_t);
    #[no_mangle]
    fn dc_apeerstate_new(_: *mut dc_context_t) -> *mut dc_apeerstate_t;
    #[no_mangle]
    fn dc_apeerstate_unref(_: *mut dc_apeerstate_t);
    #[no_mangle]
    fn dc_apeerstate_init_from_header(
        _: *mut dc_apeerstate_t,
        _: *const dc_aheader_t,
        message_time: time_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_init_from_gossip(
        _: *mut dc_apeerstate_t,
        _: *const dc_aheader_t,
        message_time: time_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_degrade_encryption(
        _: *mut dc_apeerstate_t,
        message_time: time_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_apply_header(
        _: *mut dc_apeerstate_t,
        _: *const dc_aheader_t,
        message_time: time_t,
    );
    #[no_mangle]
    fn dc_apeerstate_apply_gossip(
        _: *mut dc_apeerstate_t,
        _: *const dc_aheader_t,
        message_time: time_t,
    );
    #[no_mangle]
    fn dc_apeerstate_render_gossip_header(
        _: *const dc_apeerstate_t,
        min_verified: libc::c_int,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_apeerstate_peek_key(
        _: *const dc_apeerstate_t,
        min_verified: libc::c_int,
    ) -> *mut dc_key_t;
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
    fn mailmime_find_mailimf_fields(_: *mut mailmime) -> *mut mailimf_fields;
    #[no_mangle]
    fn mailimf_find_first_addr(_: *const mailimf_mailbox_list) -> *mut libc::c_char;
    #[no_mangle]
    fn mailimf_find_field(
        _: *mut mailimf_fields,
        wanted_fld_type: libc::c_int,
    ) -> *mut mailimf_field;
    #[no_mangle]
    fn mailimf_get_recipients(_: *mut mailimf_fields) -> *mut dc_hash_t;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_keyring_new() -> *mut dc_keyring_t;
    #[no_mangle]
    fn dc_keyring_unref(_: *mut dc_keyring_t);
    #[no_mangle]
    fn dc_pgp_pk_encrypt(
        _: *mut dc_context_t,
        plain: *const libc::c_void,
        plain_bytes: size_t,
        _: *const dc_keyring_t,
        sign_key: *const dc_key_t,
        use_armor: libc::c_int,
        ret_ctext: *mut *mut libc::c_void,
        ret_ctext_bytes: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_keyring_add(_: *mut dc_keyring_t, _: *mut dc_key_t);
    #[no_mangle]
    fn dc_pgp_is_valid_key(_: *mut dc_context_t, _: *const dc_key_t) -> libc::c_int;
    /* public key encryption */
    #[no_mangle]
    fn dc_pgp_create_keypair(
        _: *mut dc_context_t,
        addr: *const libc::c_char,
        public_key: *mut dc_key_t,
        private_key: *mut dc_key_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_pgp_rand_seed(_: *mut dc_context_t, buf: *const libc::c_void, bytes: size_t);
    #[no_mangle]
    fn dc_handle_degrade_event(_: *mut dc_context_t, _: *mut dc_apeerstate_t);
    #[no_mangle]
    fn dc_pgp_pk_decrypt(
        _: *mut dc_context_t,
        ctext: *const libc::c_void,
        ctext_bytes: size_t,
        _: *const dc_keyring_t,
        validate_keys: *const dc_keyring_t,
        use_armor: libc::c_int,
        plain: *mut *mut libc::c_void,
        plain_bytes: *mut size_t,
        ret_signature_fingerprints: *mut dc_hash_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_keyring_load_self_private_for_decrypting(
        _: *mut dc_keyring_t,
        self_addr: *const libc::c_char,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
}
pub type __darwin_size_t = libc::c_ulong;
pub type __darwin_clock_t = libc::c_ulong;
pub type __darwin_ssize_t = libc::c_long;
pub type __darwin_time_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __darwin_pthread_handler_rec {
    pub __routine: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    pub __arg: *mut libc::c_void,
    pub __next: *mut __darwin_pthread_handler_rec,
}
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _opaque_pthread_t {
    pub __sig: libc::c_long,
    pub __cleanup_stack: *mut __darwin_pthread_handler_rec,
    pub __opaque: [libc::c_char; 8176],
}
pub type __darwin_pthread_cond_t = _opaque_pthread_cond_t;
pub type __darwin_pthread_mutex_t = _opaque_pthread_mutex_t;
pub type __darwin_pthread_t = *mut _opaque_pthread_t;
pub type int32_t = libc::c_int;
pub type uintptr_t = libc::c_ulong;
pub type size_t = __darwin_size_t;
pub type uint8_t = libc::c_uchar;
pub type uint32_t = libc::c_uint;
pub type ssize_t = __darwin_ssize_t;
pub type clock_t = __darwin_clock_t;
pub type time_t = __darwin_time_t;
pub type pthread_cond_t = __darwin_pthread_cond_t;
pub type pthread_mutex_t = __darwin_pthread_mutex_t;
pub type pthread_t = __darwin_pthread_t;
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
pub type unnamed_1 = libc::c_uint;
pub const MAILIMF_ERROR_FILE: unnamed_1 = 4;
pub const MAILIMF_ERROR_INVAL: unnamed_1 = 3;
pub const MAILIMF_ERROR_MEMORY: unnamed_1 = 2;
pub const MAILIMF_ERROR_PARSE: unnamed_1 = 1;
pub const MAILIMF_NO_ERROR: unnamed_1 = 0;
pub type unnamed_2 = libc::c_uint;
pub const MAILMIME_COMPOSITE_TYPE_EXTENSION: unnamed_2 = 3;
pub const MAILMIME_COMPOSITE_TYPE_MULTIPART: unnamed_2 = 2;
pub const MAILMIME_COMPOSITE_TYPE_MESSAGE: unnamed_2 = 1;
pub const MAILMIME_COMPOSITE_TYPE_ERROR: unnamed_2 = 0;
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
pub type unnamed_7 = libc::c_uint;
pub const MAILMIME_TYPE_COMPOSITE_TYPE: unnamed_7 = 2;
pub const MAILMIME_TYPE_DISCRETE_TYPE: unnamed_7 = 1;
pub const MAILMIME_TYPE_ERROR: unnamed_7 = 0;
pub type unnamed_8 = libc::c_uint;
pub const MAILMIME_DATA_FILE: unnamed_8 = 1;
pub const MAILMIME_DATA_TEXT: unnamed_8 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_data {
    pub dt_type: libc::c_int,
    pub dt_encoding: libc::c_int,
    pub dt_encoded: libc::c_int,
    pub dt_data: unnamed_9,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_9 {
    pub dt_text: unnamed_10,
    pub dt_filename: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_10 {
    pub dt_data: *const libc::c_char,
    pub dt_length: size_t,
}
pub type unnamed_11 = libc::c_uint;
pub const MAILMIME_MESSAGE: unnamed_11 = 3;
pub const MAILMIME_MULTIPLE: unnamed_11 = 2;
pub const MAILMIME_SINGLE: unnamed_11 = 1;
pub const MAILMIME_NONE: unnamed_11 = 0;
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
    pub mm_data: unnamed_12,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_12 {
    pub mm_single: *mut mailmime_data,
    pub mm_multipart: unnamed_14,
    pub mm_message: unnamed_13,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_13 {
    pub mm_fields: *mut mailimf_fields,
    pub mm_msg_mime: *mut mailmime,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_14 {
    pub mm_preamble: *mut mailmime_data,
    pub mm_epilogue: *mut mailmime_data,
    pub mm_mp_list: *mut clist,
}
pub type unnamed_15 = libc::c_uint;
pub const MAIL_ERROR_SSL: unnamed_15 = 58;
pub const MAIL_ERROR_FOLDER: unnamed_15 = 57;
pub const MAIL_ERROR_UNABLE: unnamed_15 = 56;
pub const MAIL_ERROR_SYSTEM: unnamed_15 = 55;
pub const MAIL_ERROR_COMMAND: unnamed_15 = 54;
pub const MAIL_ERROR_SEND: unnamed_15 = 53;
pub const MAIL_ERROR_CHAR_ENCODING_FAILED: unnamed_15 = 52;
pub const MAIL_ERROR_SUBJECT_NOT_FOUND: unnamed_15 = 51;
pub const MAIL_ERROR_PROGRAM_ERROR: unnamed_15 = 50;
pub const MAIL_ERROR_NO_PERMISSION: unnamed_15 = 49;
pub const MAIL_ERROR_COMMAND_NOT_SUPPORTED: unnamed_15 = 48;
pub const MAIL_ERROR_NO_APOP: unnamed_15 = 47;
pub const MAIL_ERROR_READONLY: unnamed_15 = 46;
pub const MAIL_ERROR_FATAL: unnamed_15 = 45;
pub const MAIL_ERROR_CLOSE: unnamed_15 = 44;
pub const MAIL_ERROR_CAPABILITY: unnamed_15 = 43;
pub const MAIL_ERROR_PROTOCOL: unnamed_15 = 42;
pub const MAIL_ERROR_MISC: unnamed_15 = 41;
pub const MAIL_ERROR_EXPUNGE: unnamed_15 = 40;
pub const MAIL_ERROR_NO_TLS: unnamed_15 = 39;
pub const MAIL_ERROR_CACHE_MISS: unnamed_15 = 38;
pub const MAIL_ERROR_STARTTLS: unnamed_15 = 37;
pub const MAIL_ERROR_MOVE: unnamed_15 = 36;
pub const MAIL_ERROR_FOLDER_NOT_FOUND: unnamed_15 = 35;
pub const MAIL_ERROR_REMOVE: unnamed_15 = 34;
pub const MAIL_ERROR_PART_NOT_FOUND: unnamed_15 = 33;
pub const MAIL_ERROR_INVAL: unnamed_15 = 32;
pub const MAIL_ERROR_PARSE: unnamed_15 = 31;
pub const MAIL_ERROR_MSG_NOT_FOUND: unnamed_15 = 30;
pub const MAIL_ERROR_DISKSPACE: unnamed_15 = 29;
pub const MAIL_ERROR_SEARCH: unnamed_15 = 28;
pub const MAIL_ERROR_STORE: unnamed_15 = 27;
pub const MAIL_ERROR_FETCH: unnamed_15 = 26;
pub const MAIL_ERROR_COPY: unnamed_15 = 25;
pub const MAIL_ERROR_APPEND: unnamed_15 = 24;
pub const MAIL_ERROR_LSUB: unnamed_15 = 23;
pub const MAIL_ERROR_LIST: unnamed_15 = 22;
pub const MAIL_ERROR_UNSUBSCRIBE: unnamed_15 = 21;
pub const MAIL_ERROR_SUBSCRIBE: unnamed_15 = 20;
pub const MAIL_ERROR_STATUS: unnamed_15 = 19;
pub const MAIL_ERROR_MEMORY: unnamed_15 = 18;
pub const MAIL_ERROR_SELECT: unnamed_15 = 17;
pub const MAIL_ERROR_EXAMINE: unnamed_15 = 16;
pub const MAIL_ERROR_CHECK: unnamed_15 = 15;
pub const MAIL_ERROR_RENAME: unnamed_15 = 14;
pub const MAIL_ERROR_NOOP: unnamed_15 = 13;
pub const MAIL_ERROR_LOGOUT: unnamed_15 = 12;
pub const MAIL_ERROR_DELETE: unnamed_15 = 11;
pub const MAIL_ERROR_CREATE: unnamed_15 = 10;
pub const MAIL_ERROR_LOGIN: unnamed_15 = 9;
pub const MAIL_ERROR_STREAM: unnamed_15 = 8;
pub const MAIL_ERROR_FILE: unnamed_15 = 7;
pub const MAIL_ERROR_BAD_STATE: unnamed_15 = 6;
pub const MAIL_ERROR_CONNECT: unnamed_15 = 5;
pub const MAIL_ERROR_UNKNOWN: unnamed_15 = 4;
pub const MAIL_ERROR_NOT_IMPLEMENTED: unnamed_15 = 3;
pub const MAIL_NO_ERROR_NON_AUTHENTICATED: unnamed_15 = 2;
pub const MAIL_NO_ERROR_AUTHENTICATED: unnamed_15 = 1;
pub const MAIL_NO_ERROR: unnamed_15 = 0;
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
    pub smtp_sasl: unnamed_16,
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
pub struct unnamed_16 {
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
    pub sec_data: unnamed_17,
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
pub union unnamed_17 {
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
    pub ft_data: unnamed_18,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_18 {
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
    pub imap_sasl: unnamed_19,
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
pub struct unnamed_19 {
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
/* *
 * @class dc_aheader_t
 * Library-internal. Parse and create [Autocrypt-headers](https://autocrypt.org/en/latest/level1.html#the-autocrypt-header).
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_aheader {
    pub addr: *mut libc::c_char,
    pub public_key: *mut dc_key_t,
    pub prefer_encrypt: libc::c_int,
}
pub type dc_aheader_t = _dc_aheader;
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
pub type dc_keyring_t = _dc_keyring;
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
#[no_mangle]
pub unsafe extern "C" fn dc_e2ee_encrypt(
    mut context: *mut dc_context_t,
    mut recipients_addr: *const clist,
    mut force_unencrypted: libc::c_int,
    mut e2ee_guaranteed: libc::c_int,
    mut min_verified: libc::c_int,
    mut do_gossip: libc::c_int,
    mut in_out_message: *mut mailmime,
    mut helper: *mut dc_e2ee_helper_t,
) {
    let mut p_0: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut current_block: u64;
    let mut col: libc::c_int = 0i32;
    let mut do_encrypt: libc::c_int = 0i32;
    let mut autocryptheader: *mut dc_aheader_t = dc_aheader_new();
    /*just a pointer into mailmime structure, must not be freed*/
    let mut imffields_unprotected: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut keyring: *mut dc_keyring_t = dc_keyring_new();
    let mut sign_key: *mut dc_key_t = dc_key_new();
    let mut plain: *mut MMAPString = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
    let mut ctext: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ctext_bytes: size_t = 0i32 as size_t;
    let mut peerstates: *mut dc_array_t = dc_array_new(0 as *mut dc_context_t, 10i32 as size_t);
    if !helper.is_null() {
        memset(
            helper as *mut libc::c_void,
            0i32,
            ::std::mem::size_of::<dc_e2ee_helper_t>() as libc::c_ulong,
        );
    }
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || recipients_addr.is_null()
        || in_out_message.is_null()
        || !(*in_out_message).mm_parent.is_null()
        || autocryptheader.is_null()
        || keyring.is_null()
        || sign_key.is_null()
        || plain.is_null()
        || helper.is_null())
    {
        /* libEtPan's pgp_encrypt_mime() takes the parent as the new root. We just expect the root as being given to this function. */
        (*autocryptheader).prefer_encrypt = 0i32;
        if 0 != dc_sqlite3_get_config_int(
            (*context).sql,
            b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
            1i32,
        ) {
            (*autocryptheader).prefer_encrypt = 1i32
        }
        (*autocryptheader).addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if !(*autocryptheader).addr.is_null() {
            if !(0
                == load_or_generate_self_public_key(
                    context,
                    (*autocryptheader).public_key,
                    (*autocryptheader).addr,
                    in_out_message,
                ))
            {
                /*only for random-seed*/
                if (*autocryptheader).prefer_encrypt == 1i32 || 0 != e2ee_guaranteed {
                    do_encrypt = 1i32;
                    let mut iter1: *mut clistiter = 0 as *mut clistiter;
                    iter1 = (*recipients_addr).first;
                    while !iter1.is_null() {
                        let mut recipient_addr: *const libc::c_char = (if !iter1.is_null() {
                            (*iter1).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *const libc::c_char;
                        let mut peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
                        let mut key_to_use: *mut dc_key_t = 0 as *mut dc_key_t;
                        if !(strcasecmp(recipient_addr, (*autocryptheader).addr) == 0i32) {
                            if 0 != dc_apeerstate_load_by_addr(
                                peerstate,
                                (*context).sql,
                                recipient_addr,
                            ) && {
                                key_to_use = dc_apeerstate_peek_key(peerstate, min_verified);
                                !key_to_use.is_null()
                            } && ((*peerstate).prefer_encrypt == 1i32 || 0 != e2ee_guaranteed)
                            {
                                dc_keyring_add(keyring, key_to_use);
                                dc_array_add_ptr(peerstates, peerstate as *mut libc::c_void);
                            } else {
                                dc_apeerstate_unref(peerstate);
                                do_encrypt = 0i32;
                                /* if we cannot encrypt to a single recipient, we cannot encrypt the message at all */
                                break;
                            }
                        }
                        iter1 = if !iter1.is_null() {
                            (*iter1).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                }
                if 0 != do_encrypt {
                    dc_keyring_add(keyring, (*autocryptheader).public_key);
                    if 0 == dc_key_load_self_private(
                        sign_key,
                        (*autocryptheader).addr,
                        (*context).sql,
                    ) {
                        do_encrypt = 0i32
                    }
                }
                if 0 != force_unencrypted {
                    do_encrypt = 0i32
                }
                imffields_unprotected = mailmime_find_mailimf_fields(in_out_message);
                if !imffields_unprotected.is_null() {
                    /* encrypt message, if possible */
                    if 0 != do_encrypt {
                        mailprivacy_prepare_mime(in_out_message);
                        let mut part_to_encrypt: *mut mailmime =
                            (*in_out_message).mm_data.mm_message.mm_msg_mime;
                        (*part_to_encrypt).mm_parent = 0 as *mut mailmime;
                        let mut imffields_encrypted: *mut mailimf_fields =
                            mailimf_fields_new_empty();
                        /* mailmime_new_message_data() calls mailmime_fields_new_with_version() which would add the unwanted MIME-Version:-header */
                        let mut message_to_encrypt: *mut mailmime = mailmime_new(
                            MAILMIME_MESSAGE as libc::c_int,
                            0 as *const libc::c_char,
                            0i32 as size_t,
                            mailmime_fields_new_empty(),
                            mailmime_get_content_message(),
                            0 as *mut mailmime_data,
                            0 as *mut mailmime_data,
                            0 as *mut mailmime_data,
                            0 as *mut clist,
                            imffields_encrypted,
                            part_to_encrypt,
                        );
                        if 0 != do_gossip {
                            let mut iCnt: libc::c_int = dc_array_get_cnt(peerstates) as libc::c_int;
                            if iCnt > 1i32 {
                                let mut i: libc::c_int = 0i32;
                                while i < iCnt {
                                    let mut p: *mut libc::c_char =
                                        dc_apeerstate_render_gossip_header(
                                            dc_array_get_ptr(peerstates, i as size_t)
                                                as *mut dc_apeerstate_t,
                                            min_verified,
                                        );
                                    if !p.is_null() {
                                        mailimf_fields_add(
                                            imffields_encrypted,
                                            mailimf_field_new_custom(
                                                strdup(
                                                    b"Autocrypt-Gossip\x00" as *const u8
                                                        as *const libc::c_char,
                                                ),
                                                p,
                                            ),
                                        );
                                    }
                                    i += 1
                                }
                            }
                        }
                        /* memoryhole headers */
                        let mut cur: *mut clistiter = (*(*imffields_unprotected).fld_list).first;
                        while !cur.is_null() {
                            let mut move_to_encrypted: libc::c_int = 0i32;
                            let mut field: *mut mailimf_field = (if !cur.is_null() {
                                (*cur).data
                            } else {
                                0 as *mut libc::c_void
                            })
                                as *mut mailimf_field;
                            if !field.is_null() {
                                if (*field).fld_type == MAILIMF_FIELD_SUBJECT as libc::c_int {
                                    move_to_encrypted = 1i32
                                } else if (*field).fld_type
                                    == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
                                {
                                    let mut opt_field: *mut mailimf_optional_field =
                                        (*field).fld_data.fld_optional_field;
                                    if !opt_field.is_null() && !(*opt_field).fld_name.is_null() {
                                        if strncmp(
                                            (*opt_field).fld_name,
                                            b"Secure-Join\x00" as *const u8 as *const libc::c_char,
                                            11i32 as libc::c_ulong,
                                        ) == 0i32
                                            || strncmp(
                                                (*opt_field).fld_name,
                                                b"Chat-\x00" as *const u8 as *const libc::c_char,
                                                5i32 as libc::c_ulong,
                                            ) == 0i32
                                                && strcmp(
                                                    (*opt_field).fld_name,
                                                    b"Chat-Version\x00" as *const u8
                                                        as *const libc::c_char,
                                                ) != 0i32
                                        {
                                            move_to_encrypted = 1i32
                                        }
                                    }
                                }
                            }
                            if 0 != move_to_encrypted {
                                mailimf_fields_add(imffields_encrypted, field);
                                cur = clist_delete((*imffields_unprotected).fld_list, cur)
                            } else {
                                cur = if !cur.is_null() {
                                    (*cur).next
                                } else {
                                    0 as *mut clistcell_s
                                }
                            }
                        }
                        let mut subject: *mut mailimf_subject = mailimf_subject_new(dc_strdup(
                            b"...\x00" as *const u8 as *const libc::c_char,
                        ));
                        mailimf_fields_add(
                            imffields_unprotected,
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
                        clist_insert_after(
                            (*(*part_to_encrypt).mm_content_type).ct_parameters,
                            (*(*(*part_to_encrypt).mm_content_type).ct_parameters).last,
                            mailmime_param_new_with_data(
                                b"protected-headers\x00" as *const u8 as *const libc::c_char
                                    as *mut libc::c_char,
                                b"v1\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                            ) as *mut libc::c_void,
                        );
                        mailmime_write_mem(plain, &mut col, message_to_encrypt);
                        if (*plain).str_0.is_null() || (*plain).len <= 0i32 as libc::c_ulong {
                            current_block = 14181132614457621749;
                        } else if 0
                            == dc_pgp_pk_encrypt(
                                context,
                                (*plain).str_0 as *const libc::c_void,
                                (*plain).len,
                                keyring,
                                sign_key,
                                1i32,
                                &mut ctext as *mut *mut libc::c_char as *mut *mut libc::c_void,
                                &mut ctext_bytes,
                            )
                        {
                            /*use_armor*/
                            current_block = 14181132614457621749;
                        } else {
                            (*helper).cdata_to_free = ctext as *mut libc::c_void;
                            //char* t2=dc_null_terminate(ctext,ctext_bytes);printf("ENCRYPTED:\n%s\n",t2);free(t2); // DEBUG OUTPUT
                            /* create MIME-structure that will contain the encrypted text */
                            let mut encrypted_part: *mut mailmime = new_data_part(
                                0 as *mut libc::c_void,
                                0i32 as size_t,
                                b"multipart/encrypted\x00" as *const u8 as *const libc::c_char
                                    as *mut libc::c_char,
                                -1i32,
                            );
                            let mut content: *mut mailmime_content =
                                (*encrypted_part).mm_content_type;
                            clist_insert_after(
                                (*content).ct_parameters,
                                (*(*content).ct_parameters).last,
                                mailmime_param_new_with_data(
                                    b"protocol\x00" as *const u8 as *const libc::c_char
                                        as *mut libc::c_char,
                                    b"application/pgp-encrypted\x00" as *const u8
                                        as *const libc::c_char
                                        as *mut libc::c_char,
                                ) as *mut libc::c_void,
                            );
                            static mut version_content: [libc::c_char; 13] =
                                [86, 101, 114, 115, 105, 111, 110, 58, 32, 49, 13, 10, 0];
                            let mut version_mime: *mut mailmime = new_data_part(
                                version_content.as_mut_ptr() as *mut libc::c_void,
                                strlen(version_content.as_mut_ptr()),
                                b"application/pgp-encrypted\x00" as *const u8 as *const libc::c_char
                                    as *mut libc::c_char,
                                MAILMIME_MECHANISM_7BIT as libc::c_int,
                            );
                            mailmime_smart_add_part(encrypted_part, version_mime);
                            let mut ctext_part: *mut mailmime = new_data_part(
                                ctext as *mut libc::c_void,
                                ctext_bytes,
                                b"application/octet-stream\x00" as *const u8 as *const libc::c_char
                                    as *mut libc::c_char,
                                MAILMIME_MECHANISM_7BIT as libc::c_int,
                            );
                            mailmime_smart_add_part(encrypted_part, ctext_part);
                            (*in_out_message).mm_data.mm_message.mm_msg_mime = encrypted_part;
                            (*encrypted_part).mm_parent = in_out_message;
                            mailmime_free(message_to_encrypt);
                            (*helper).encryption_successfull = 1i32;
                            current_block = 13824533195664196414;
                        }
                    } else {
                        current_block = 13824533195664196414;
                    }
                    match current_block {
                        14181132614457621749 => {}
                        _ => {
                            p_0 = dc_aheader_render(autocryptheader);
                            if !p_0.is_null() {
                                mailimf_fields_add(
                                    imffields_unprotected,
                                    mailimf_field_new_custom(
                                        strdup(
                                            b"Autocrypt\x00" as *const u8 as *const libc::c_char,
                                        ),
                                        p_0,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    dc_aheader_unref(autocryptheader);
    dc_keyring_unref(keyring);
    dc_key_unref(sign_key);
    if !plain.is_null() {
        mmap_string_free(plain);
    }
    let mut i_0: libc::c_int =
        dc_array_get_cnt(peerstates).wrapping_sub(1i32 as libc::c_ulong) as libc::c_int;
    while i_0 >= 0i32 {
        dc_apeerstate_unref(dc_array_get_ptr(peerstates, i_0 as size_t) as *mut dc_apeerstate_t);
        i_0 -= 1
    }
    dc_array_unref(peerstates);
}
/* ******************************************************************************
 * Tools
 ******************************************************************************/
unsafe extern "C" fn new_data_part(
    mut data: *mut libc::c_void,
    mut data_bytes: size_t,
    mut default_content_type: *mut libc::c_char,
    mut default_encoding: libc::c_int,
) -> *mut mailmime {
    let mut current_block: u64;
    //char basename_buf[PATH_MAX];
    let mut encoding: *mut mailmime_mechanism = 0 as *mut mailmime_mechanism;
    let mut content: *mut mailmime_content = 0 as *mut mailmime_content;
    let mut mime: *mut mailmime = 0 as *mut mailmime;
    //int r;
    //char * dup_filename;
    let mut mime_fields: *mut mailmime_fields = 0 as *mut mailmime_fields;
    let mut encoding_type: libc::c_int = 0;
    let mut content_type_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut do_encoding: libc::c_int = 0;
    encoding = 0 as *mut mailmime_mechanism;
    if default_content_type.is_null() {
        content_type_str =
            b"application/octet-stream\x00" as *const u8 as *const libc::c_char as *mut libc::c_char
    } else {
        content_type_str = default_content_type
    }
    content = mailmime_content_new_with_str(content_type_str);
    if content.is_null() {
        current_block = 16266721588079097885;
    } else {
        do_encoding = 1i32;
        if (*(*content).ct_type).tp_type == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int {
            let mut composite: *mut mailmime_composite_type = 0 as *mut mailmime_composite_type;
            composite = (*(*content).ct_type).tp_data.tp_composite_type;
            match (*composite).ct_type {
                1 => {
                    if strcasecmp(
                        (*content).ct_subtype,
                        b"rfc822\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                    {
                        do_encoding = 0i32
                    }
                }
                2 => do_encoding = 0i32,
                _ => {}
            }
        }
        if 0 != do_encoding {
            if default_encoding == -1i32 {
                encoding_type = MAILMIME_MECHANISM_BASE64 as libc::c_int
            } else {
                encoding_type = default_encoding
            }
            encoding = mailmime_mechanism_new(encoding_type, 0 as *mut libc::c_char);
            if encoding.is_null() {
                current_block = 16266721588079097885;
            } else {
                current_block = 11057878835866523405;
            }
        } else {
            current_block = 11057878835866523405;
        }
        match current_block {
            16266721588079097885 => {}
            _ => {
                mime_fields = mailmime_fields_new_with_data(
                    encoding,
                    0 as *mut libc::c_char,
                    0 as *mut libc::c_char,
                    0 as *mut mailmime_disposition,
                    0 as *mut mailmime_language,
                );
                if mime_fields.is_null() {
                    current_block = 16266721588079097885;
                } else {
                    mime = mailmime_new_empty(content, mime_fields);
                    if mime.is_null() {
                        mailmime_fields_free(mime_fields);
                        mailmime_content_free(content);
                    } else {
                        if !data.is_null()
                            && data_bytes > 0i32 as libc::c_ulong
                            && (*mime).mm_type == MAILMIME_SINGLE as libc::c_int
                        {
                            mailmime_set_body_text(mime, data as *mut libc::c_char, data_bytes);
                        }
                        return mime;
                    }
                    current_block = 13668317689588454213;
                }
            }
        }
    }
    match current_block {
        16266721588079097885 => {
            if !encoding.is_null() {
                mailmime_mechanism_free(encoding);
            }
            if !content.is_null() {
                mailmime_content_free(content);
            }
        }
        _ => {}
    }
    return 0 as *mut mailmime;
}
/* ******************************************************************************
 * Generate Keypairs
 ******************************************************************************/
unsafe extern "C" fn load_or_generate_self_public_key(
    mut context: *mut dc_context_t,
    mut public_key: *mut dc_key_t,
    mut self_addr: *const libc::c_char,
    mut random_data_mime: *mut mailmime,
) -> libc::c_int {
    let mut current_block: u64;
    /* avoid double creation (we unlock the database during creation) */
    static mut s_in_key_creation: libc::c_int = 0i32;
    let mut key_created: libc::c_int = 0i32;
    let mut success: libc::c_int = 0i32;
    let mut key_creation_here: libc::c_int = 0i32;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || public_key.is_null())
    {
        if 0 == dc_key_load_self_public(public_key, self_addr, (*context).sql) {
            /* create the keypair - this may take a moment, however, as this is in a thread, this is no big deal */
            if 0 != s_in_key_creation {
                current_block = 10496152961502316708;
            } else {
                key_creation_here = 1i32;
                s_in_key_creation = 1i32;
                /* seed the random generator */
                let mut seed: [uintptr_t; 4] = [0; 4];
                seed[0usize] = time(0 as *mut time_t) as uintptr_t;
                seed[1usize] = seed.as_mut_ptr() as uintptr_t;
                seed[2usize] = public_key as uintptr_t;
                seed[3usize] = pthread_self() as uintptr_t;
                dc_pgp_rand_seed(
                    context,
                    seed.as_mut_ptr() as *const libc::c_void,
                    ::std::mem::size_of::<[uintptr_t; 4]>() as libc::c_ulong,
                );
                if !random_data_mime.is_null() {
                    let mut random_data_mmap: *mut MMAPString = 0 as *mut MMAPString;
                    let mut col: libc::c_int = 0i32;
                    random_data_mmap = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
                    if random_data_mmap.is_null() {
                        current_block = 10496152961502316708;
                    } else {
                        mailmime_write_mem(random_data_mmap, &mut col, random_data_mime);
                        dc_pgp_rand_seed(
                            context,
                            (*random_data_mmap).str_0 as *const libc::c_void,
                            (*random_data_mmap).len,
                        );
                        mmap_string_free(random_data_mmap);
                        current_block = 26972500619410423;
                    }
                } else {
                    current_block = 26972500619410423;
                }
                match current_block {
                    10496152961502316708 => {}
                    _ => {
                        let mut private_key: *mut dc_key_t = dc_key_new();
                        let mut start: clock_t = clock();
                        dc_log_info(
                            context,
                            0i32,
                            b"Generating keypair with %i bits, e=%i ...\x00" as *const u8
                                as *const libc::c_char,
                            2048i32,
                            65537i32,
                        );
                        key_created =
                            dc_pgp_create_keypair(context, self_addr, public_key, private_key);
                        if 0 == key_created {
                            dc_log_warning(
                                context,
                                0i32,
                                b"Cannot create keypair.\x00" as *const u8 as *const libc::c_char,
                            );
                            current_block = 10496152961502316708;
                        } else if 0 == dc_pgp_is_valid_key(context, public_key)
                            || 0 == dc_pgp_is_valid_key(context, private_key)
                        {
                            dc_log_warning(
                                context,
                                0i32,
                                b"Generated keys are not valid.\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            current_block = 10496152961502316708;
                        } else if 0
                            == dc_key_save_self_keypair(
                                public_key,
                                private_key,
                                self_addr,
                                1i32,
                                (*context).sql,
                            )
                        {
                            /*set default*/
                            dc_log_warning(
                                context,
                                0i32,
                                b"Cannot save keypair.\x00" as *const u8 as *const libc::c_char,
                            );
                            current_block = 10496152961502316708;
                        } else {
                            dc_log_info(
                                context,
                                0i32,
                                b"Keypair generated in %.3f s.\x00" as *const u8
                                    as *const libc::c_char,
                                clock().wrapping_sub(start) as libc::c_double
                                    / 1000000i32 as libc::c_double,
                            );
                            dc_key_unref(private_key);
                            current_block = 1118134448028020070;
                        }
                    }
                }
            }
        } else {
            current_block = 1118134448028020070;
        }
        match current_block {
            10496152961502316708 => {}
            _ => success = 1i32,
        }
    }
    if 0 != key_creation_here {
        s_in_key_creation = 0i32
    }
    return success;
}
/* returns 1 if sth. was decrypted, 0 in other cases */
#[no_mangle]
pub unsafe extern "C" fn dc_e2ee_decrypt(
    mut context: *mut dc_context_t,
    mut in_out_message: *mut mailmime,
    mut helper: *mut dc_e2ee_helper_t,
) {
    let mut iterations: libc::c_int = 0;
    /* return values: 0=nothing to decrypt/cannot decrypt, 1=sth. decrypted
    (to detect parts that could not be decrypted, simply look for left "multipart/encrypted" MIME types */
    /*just a pointer into mailmime structure, must not be freed*/
    let mut imffields: *mut mailimf_fields = mailmime_find_mailimf_fields(in_out_message);
    let mut autocryptheader: *mut dc_aheader_t = 0 as *mut dc_aheader_t;
    let mut message_time: time_t = 0i32 as time_t;
    let mut peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
    let mut from: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut private_keyring: *mut dc_keyring_t = dc_keyring_new();
    let mut public_keyring_for_validate: *mut dc_keyring_t = dc_keyring_new();
    let mut gossip_headers: *mut mailimf_fields = 0 as *mut mailimf_fields;
    if !helper.is_null() {
        memset(
            helper as *mut libc::c_void,
            0i32,
            ::std::mem::size_of::<dc_e2ee_helper_t>() as libc::c_ulong,
        );
    }
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || in_out_message.is_null()
        || helper.is_null()
        || imffields.is_null())
    {
        if !imffields.is_null() {
            let mut field: *mut mailimf_field =
                mailimf_find_field(imffields, MAILIMF_FIELD_FROM as libc::c_int);
            if !field.is_null() && !(*field).fld_data.fld_from.is_null() {
                from = mailimf_find_first_addr((*(*field).fld_data.fld_from).frm_mb_list)
            }
            field = mailimf_find_field(imffields, MAILIMF_FIELD_ORIG_DATE as libc::c_int);
            if !field.is_null() && !(*field).fld_data.fld_orig_date.is_null() {
                let mut orig_date: *mut mailimf_orig_date = (*field).fld_data.fld_orig_date;
                if !orig_date.is_null() {
                    message_time = dc_timestamp_from_date((*orig_date).dt_date_time);
                    if message_time != -1i32 as libc::c_long
                        && message_time > time(0 as *mut time_t)
                    {
                        message_time = time(0 as *mut time_t)
                    }
                }
            }
        }
        autocryptheader = dc_aheader_new_from_imffields(from, imffields);
        if !autocryptheader.is_null() {
            if 0 == dc_pgp_is_valid_key(context, (*autocryptheader).public_key) {
                dc_aheader_unref(autocryptheader);
                autocryptheader = 0 as *mut dc_aheader_t
            }
        }
        if message_time > 0i32 as libc::c_long && !from.is_null() {
            if 0 != dc_apeerstate_load_by_addr(peerstate, (*context).sql, from) {
                if !autocryptheader.is_null() {
                    dc_apeerstate_apply_header(peerstate, autocryptheader, message_time);
                    dc_apeerstate_save_to_db(peerstate, (*context).sql, 0i32);
                } else if message_time > (*peerstate).last_seen_autocrypt
                    && 0 == contains_report(in_out_message)
                {
                    dc_apeerstate_degrade_encryption(peerstate, message_time);
                    dc_apeerstate_save_to_db(peerstate, (*context).sql, 0i32);
                }
            } else if !autocryptheader.is_null() {
                dc_apeerstate_init_from_header(peerstate, autocryptheader, message_time);
                dc_apeerstate_save_to_db(peerstate, (*context).sql, 1i32);
            }
        }
        /* load private key for decryption */
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if !self_addr.is_null() {
            if !(0
                == dc_keyring_load_self_private_for_decrypting(
                    private_keyring,
                    self_addr,
                    (*context).sql,
                ))
            {
                if (*peerstate).last_seen == 0i32 as libc::c_long {
                    dc_apeerstate_load_by_addr(peerstate, (*context).sql, from);
                }
                if 0 != (*peerstate).degrade_event {
                    dc_handle_degrade_event(context, peerstate);
                }
                dc_keyring_add(public_keyring_for_validate, (*peerstate).gossip_key);
                dc_keyring_add(public_keyring_for_validate, (*peerstate).public_key);
                (*helper).signatures =
                    malloc(::std::mem::size_of::<dc_hash_t>() as libc::c_ulong) as *mut dc_hash_t;
                dc_hash_init((*helper).signatures, 3i32, 1i32);
                iterations = 0i32;
                while iterations < 10i32 {
                    let mut has_unencrypted_parts: libc::c_int = 0i32;
                    if 0 == decrypt_recursive(
                        context,
                        in_out_message,
                        private_keyring,
                        public_keyring_for_validate,
                        (*helper).signatures,
                        &mut gossip_headers,
                        &mut has_unencrypted_parts,
                    ) {
                        break;
                    }
                    if iterations == 0i32 && 0 == has_unencrypted_parts {
                        (*helper).encrypted = 1i32
                    }
                    iterations += 1
                }
                if !gossip_headers.is_null() {
                    (*helper).gossipped_addr =
                        update_gossip_peerstates(context, message_time, imffields, gossip_headers)
                }
            }
        }
    }
    //mailmime_print(in_out_message);
    if !gossip_headers.is_null() {
        mailimf_fields_free(gossip_headers);
    }
    dc_aheader_unref(autocryptheader);
    dc_apeerstate_unref(peerstate);
    dc_keyring_unref(private_keyring);
    dc_keyring_unref(public_keyring_for_validate);
    free(from as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
}
unsafe extern "C" fn update_gossip_peerstates(
    mut context: *mut dc_context_t,
    mut message_time: time_t,
    mut imffields: *mut mailimf_fields,
    mut gossip_headers: *const mailimf_fields,
) -> *mut dc_hash_t {
    let mut cur1: *mut clistiter = 0 as *mut clistiter;
    let mut recipients: *mut dc_hash_t = 0 as *mut dc_hash_t;
    let mut gossipped_addr: *mut dc_hash_t = 0 as *mut dc_hash_t;
    cur1 = (*(*gossip_headers).fld_list).first;
    while !cur1.is_null() {
        let mut field: *mut mailimf_field = (if !cur1.is_null() {
            (*cur1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        if (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            let mut optional_field: *const mailimf_optional_field =
                (*field).fld_data.fld_optional_field;
            if !optional_field.is_null()
                && !(*optional_field).fld_name.is_null()
                && strcasecmp(
                    (*optional_field).fld_name,
                    b"Autocrypt-Gossip\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                let mut gossip_header: *mut dc_aheader_t = dc_aheader_new();
                if 0 != dc_aheader_set_from_string(gossip_header, (*optional_field).fld_value)
                    && 0 != dc_pgp_is_valid_key(context, (*gossip_header).public_key)
                {
                    if recipients.is_null() {
                        recipients = mailimf_get_recipients(imffields)
                    }
                    if !dc_hash_find(
                        recipients,
                        (*gossip_header).addr as *const libc::c_void,
                        strlen((*gossip_header).addr) as libc::c_int,
                    )
                    .is_null()
                    {
                        let mut peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
                        if 0 == dc_apeerstate_load_by_addr(
                            peerstate,
                            (*context).sql,
                            (*gossip_header).addr,
                        ) {
                            dc_apeerstate_init_from_gossip(peerstate, gossip_header, message_time);
                            dc_apeerstate_save_to_db(peerstate, (*context).sql, 1i32);
                        } else {
                            dc_apeerstate_apply_gossip(peerstate, gossip_header, message_time);
                            dc_apeerstate_save_to_db(peerstate, (*context).sql, 0i32);
                        }
                        if 0 != (*peerstate).degrade_event {
                            dc_handle_degrade_event(context, peerstate);
                        }
                        dc_apeerstate_unref(peerstate);
                        if gossipped_addr.is_null() {
                            gossipped_addr =
                                malloc(::std::mem::size_of::<dc_hash_t>() as libc::c_ulong)
                                    as *mut dc_hash_t;
                            dc_hash_init(gossipped_addr, 3i32, 1i32);
                        }
                        dc_hash_insert(
                            gossipped_addr,
                            (*gossip_header).addr as *const libc::c_void,
                            strlen((*gossip_header).addr) as libc::c_int,
                            1i32 as *mut libc::c_void,
                        );
                    } else {
                        dc_log_info(
                            context,
                            0i32,
                            b"Ignoring gossipped \"%s\" as the address is not in To/Cc list.\x00"
                                as *const u8 as *const libc::c_char,
                            (*gossip_header).addr,
                        );
                    }
                }
                dc_aheader_unref(gossip_header);
            }
        }
        cur1 = if !cur1.is_null() {
            (*cur1).next
        } else {
            0 as *mut clistcell_s
        }
    }
    if !recipients.is_null() {
        dc_hash_clear(recipients);
        free(recipients as *mut libc::c_void);
    }
    return gossipped_addr;
}
unsafe extern "C" fn decrypt_recursive(
    mut context: *mut dc_context_t,
    mut mime: *mut mailmime,
    mut private_keyring: *const dc_keyring_t,
    mut public_keyring_for_validate: *const dc_keyring_t,
    mut ret_valid_signatures: *mut dc_hash_t,
    mut ret_gossip_headers: *mut *mut mailimf_fields,
    mut ret_has_unencrypted_parts: *mut libc::c_int,
) -> libc::c_int {
    let mut ct: *mut mailmime_content = 0 as *mut mailmime_content;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    if context.is_null() || mime.is_null() {
        return 0i32;
    }
    if (*mime).mm_type == MAILMIME_MULTIPLE as libc::c_int {
        ct = (*mime).mm_content_type;
        if !ct.is_null()
            && !(*ct).ct_subtype.is_null()
            && strcmp(
                (*ct).ct_subtype,
                b"encrypted\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
        {
            cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
            while !cur.is_null() {
                let mut decrypted_mime: *mut mailmime = 0 as *mut mailmime;
                if 0 != decrypt_part(
                    context,
                    (if !cur.is_null() {
                        (*cur).data
                    } else {
                        0 as *mut libc::c_void
                    }) as *mut mailmime,
                    private_keyring,
                    public_keyring_for_validate,
                    ret_valid_signatures,
                    &mut decrypted_mime,
                ) {
                    if (*ret_gossip_headers).is_null() && (*ret_valid_signatures).count > 0i32 {
                        let mut dummy: size_t = 0i32 as size_t;
                        let mut test: *mut mailimf_fields = 0 as *mut mailimf_fields;
                        if mailimf_envelope_and_optional_fields_parse(
                            (*decrypted_mime).mm_mime_start,
                            (*decrypted_mime).mm_length,
                            &mut dummy,
                            &mut test,
                        ) == MAILIMF_NO_ERROR as libc::c_int
                            && !test.is_null()
                        {
                            *ret_gossip_headers = test
                        }
                    }
                    mailmime_substitute(mime, decrypted_mime);
                    mailmime_free(mime);
                    return 1i32;
                }
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell_s
                }
            }
            *ret_has_unencrypted_parts = 1i32
        } else {
            cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
            while !cur.is_null() {
                if 0 != decrypt_recursive(
                    context,
                    (if !cur.is_null() {
                        (*cur).data
                    } else {
                        0 as *mut libc::c_void
                    }) as *mut mailmime,
                    private_keyring,
                    public_keyring_for_validate,
                    ret_valid_signatures,
                    ret_gossip_headers,
                    ret_has_unencrypted_parts,
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
    } else if (*mime).mm_type == MAILMIME_MESSAGE as libc::c_int {
        if 0 != decrypt_recursive(
            context,
            (*mime).mm_data.mm_message.mm_msg_mime,
            private_keyring,
            public_keyring_for_validate,
            ret_valid_signatures,
            ret_gossip_headers,
            ret_has_unencrypted_parts,
        ) {
            return 1i32;
        }
    } else {
        *ret_has_unencrypted_parts = 1i32
    }
    return 0i32;
}
unsafe extern "C" fn decrypt_part(
    mut context: *mut dc_context_t,
    mut mime: *mut mailmime,
    mut private_keyring: *const dc_keyring_t,
    mut public_keyring_for_validate: *const dc_keyring_t,
    mut ret_valid_signatures: *mut dc_hash_t,
    mut ret_decrypted_mime: *mut *mut mailmime,
) -> libc::c_int {
    let mut add_signatures: *mut dc_hash_t = 0 as *mut dc_hash_t;
    let mut current_block: u64;
    let mut mime_data: *mut mailmime_data = 0 as *mut mailmime_data;
    let mut mime_transfer_encoding: libc::c_int = MAILMIME_MECHANISM_BINARY as libc::c_int;
    /* mmap_string_unref()'d if set */
    let mut transfer_decoding_buffer: *mut libc::c_char = 0 as *mut libc::c_char;
    /* must not be free()'d */
    let mut decoded_data: *const libc::c_char = 0 as *const libc::c_char;
    let mut decoded_data_bytes: size_t = 0i32 as size_t;
    let mut plain_buf: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut plain_bytes: size_t = 0i32 as size_t;
    let mut sth_decrypted: libc::c_int = 0i32;
    *ret_decrypted_mime = 0 as *mut mailmime;
    mime_data = (*mime).mm_data.mm_single;
    /* MAILMIME_DATA_FILE indicates, the data is in a file; AFAIK this is not used on parsing */
    if !((*mime_data).dt_type != MAILMIME_DATA_TEXT as libc::c_int
        || (*mime_data).dt_data.dt_text.dt_data.is_null()
        || (*mime_data).dt_data.dt_text.dt_length <= 0i32 as libc::c_ulong)
    {
        if !(*mime).mm_mime_fields.is_null() {
            let mut cur: *mut clistiter = 0 as *mut clistiter;
            cur = (*(*(*mime).mm_mime_fields).fld_list).first;
            while !cur.is_null() {
                let mut field: *mut mailmime_field = (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime_field;
                if !field.is_null() {
                    if (*field).fld_type == MAILMIME_FIELD_TRANSFER_ENCODING as libc::c_int
                        && !(*field).fld_data.fld_encoding.is_null()
                    {
                        mime_transfer_encoding = (*(*field).fld_data.fld_encoding).enc_type
                    }
                }
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell_s
                }
            }
        }
        /* regard `Content-Transfer-Encoding:` */
        if mime_transfer_encoding == MAILMIME_MECHANISM_7BIT as libc::c_int
            || mime_transfer_encoding == MAILMIME_MECHANISM_8BIT as libc::c_int
            || mime_transfer_encoding == MAILMIME_MECHANISM_BINARY as libc::c_int
        {
            decoded_data = (*mime_data).dt_data.dt_text.dt_data;
            decoded_data_bytes = (*mime_data).dt_data.dt_text.dt_length;
            if decoded_data.is_null() || decoded_data_bytes <= 0i32 as libc::c_ulong {
                /* no error - but no data */
                current_block = 2554982661806928548;
            } else {
                current_block = 4488286894823169796;
            }
        } else {
            let mut r: libc::c_int = 0;
            let mut current_index: size_t = 0i32 as size_t;
            r = mailmime_part_parse(
                (*mime_data).dt_data.dt_text.dt_data,
                (*mime_data).dt_data.dt_text.dt_length,
                &mut current_index,
                mime_transfer_encoding,
                &mut transfer_decoding_buffer,
                &mut decoded_data_bytes,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int
                || transfer_decoding_buffer.is_null()
                || decoded_data_bytes <= 0i32 as libc::c_ulong
            {
                current_block = 2554982661806928548;
            } else {
                decoded_data = transfer_decoding_buffer;
                current_block = 4488286894823169796;
            }
        }
        match current_block {
            2554982661806928548 => {}
            _ => {
                /* encrypted, decoded data in decoded_data now ... */
                if !(0 == has_decrypted_pgp_armor(decoded_data, decoded_data_bytes as libc::c_int))
                {
                    add_signatures = if (*ret_valid_signatures).count <= 0i32 {
                        ret_valid_signatures
                    } else {
                        0 as *mut dc_hash_t
                    };
                    /*if we already have fingerprints, do not add more; this ensures, only the fingerprints from the outer-most part are collected */
                    if !(0
                        == dc_pgp_pk_decrypt(
                            context,
                            decoded_data as *const libc::c_void,
                            decoded_data_bytes,
                            private_keyring,
                            public_keyring_for_validate,
                            1i32,
                            &mut plain_buf,
                            &mut plain_bytes,
                            add_signatures,
                        )
                        || plain_buf.is_null()
                        || plain_bytes <= 0i32 as libc::c_ulong)
                    {
                        //{char* t1=dc_null_terminate(plain_buf,plain_bytes);printf("\n**********\n%s\n**********\n",t1);free(t1);}
                        let mut index: size_t = 0i32 as size_t;
                        let mut decrypted_mime: *mut mailmime = 0 as *mut mailmime;
                        if mailmime_parse(
                            plain_buf as *const libc::c_char,
                            plain_bytes,
                            &mut index,
                            &mut decrypted_mime,
                        ) != MAIL_NO_ERROR as libc::c_int
                            || decrypted_mime.is_null()
                        {
                            if !decrypted_mime.is_null() {
                                mailmime_free(decrypted_mime);
                            }
                        } else {
                            *ret_decrypted_mime = decrypted_mime;
                            sth_decrypted = 1i32
                        }
                    }
                }
            }
        }
    }
    //mailmime_substitute(mime, new_mime);
    //s. mailprivacy_gnupg.c::pgp_decrypt()
    if !transfer_decoding_buffer.is_null() {
        mmap_string_unref(transfer_decoding_buffer);
    }
    return sth_decrypted;
}
/* ******************************************************************************
 * Decrypt
 ******************************************************************************/
unsafe extern "C" fn has_decrypted_pgp_armor(
    mut str__: *const libc::c_char,
    mut str_bytes: libc::c_int,
) -> libc::c_int {
    let mut str_end: *const libc::c_uchar =
        (str__ as *const libc::c_uchar).offset(str_bytes as isize);
    let mut p: *const libc::c_uchar = str__ as *const libc::c_uchar;
    while p < str_end {
        if *p as libc::c_int > ' ' as i32 {
            break;
        }
        p = p.offset(1isize);
        str_bytes -= 1
    }
    if str_bytes > 27i32
        && strncmp(
            p as *const libc::c_char,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
            27i32 as libc::c_ulong,
        ) == 0i32
    {
        return 1i32;
    }
    return 0i32;
}
/* *
 * Check if a MIME structure contains a multipart/report part.
 *
 * As reports are often unencrypted, we do not reset the Autocrypt header in
 * this case.
 *
 * However, Delta Chat itself has no problem with encrypted multipart/report
 * parts and MUAs should be encouraged to encrpyt multipart/reports as well so
 * that we could use the normal Autocrypt processing.
 *
 * @private
 * @param mime The mime struture to check
 * @return 1=multipart/report found in MIME, 0=no multipart/report found
 */
unsafe extern "C" fn contains_report(mut mime: *mut mailmime) -> libc::c_int {
    if (*mime).mm_type == MAILMIME_MULTIPLE as libc::c_int {
        if (*(*(*mime).mm_content_type).ct_type).tp_type
            == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int
            && (*(*(*(*mime).mm_content_type).ct_type)
                .tp_data
                .tp_composite_type)
                .ct_type
                == MAILMIME_COMPOSITE_TYPE_MULTIPART as libc::c_int
            && strcmp(
                (*(*mime).mm_content_type).ct_subtype,
                b"report\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
        {
            return 1i32;
        }
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
        while !cur.is_null() {
            if 0 != contains_report(
                (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime,
            ) {
                return 1i32;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell_s
            }
        }
    } else if (*mime).mm_type == MAILMIME_MESSAGE as libc::c_int {
        if 0 != contains_report((*mime).mm_data.mm_message.mm_msg_mime) {
            return 1i32;
        }
    }
    return 0i32;
}
/* frees data referenced by "mailmime" but not freed by mailmime_free(). After calling this function, in_out_message cannot be used any longer! */
#[no_mangle]
pub unsafe extern "C" fn dc_e2ee_thanks(mut helper: *mut dc_e2ee_helper_t) {
    if helper.is_null() {
        return;
    }
    free((*helper).cdata_to_free);
    (*helper).cdata_to_free = 0 as *mut libc::c_void;
    if !(*helper).gossipped_addr.is_null() {
        dc_hash_clear((*helper).gossipped_addr);
        free((*helper).gossipped_addr as *mut libc::c_void);
        (*helper).gossipped_addr = 0 as *mut dc_hash_t
    }
    if !(*helper).signatures.is_null() {
        dc_hash_clear((*helper).signatures);
        free((*helper).signatures as *mut libc::c_void);
        (*helper).signatures = 0 as *mut dc_hash_t
    };
}
/* makes sure, the private key exists, needed only for exporting keys and the case no message was sent before */
#[no_mangle]
pub unsafe extern "C" fn dc_ensure_secret_key_exists(
    mut context: *mut dc_context_t,
) -> libc::c_int {
    /* normally, the key is generated as soon as the first mail is send
    (this is to gain some extra-random-seed by the message content and the timespan between program start and message sending) */
    let mut success: libc::c_int = 0i32;
    let mut public_key: *mut dc_key_t = dc_key_new();
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || public_key.is_null())
    {
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if self_addr.is_null() {
            dc_log_warning(
                context,
                0i32,
                b"Cannot ensure secret key if context is not configured.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else if !(0
            == load_or_generate_self_public_key(context, public_key, self_addr, 0 as *mut mailmime))
        {
            /*no random text data for seeding available*/
            success = 1i32
        }
    }
    dc_key_unref(public_key);
    free(self_addr as *mut libc::c_void);
    return success;
}

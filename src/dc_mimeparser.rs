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
    fn atoi(_: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strstr(_: *const libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strndup(_: *const libc::c_char, _: libc::c_ulong) -> *mut libc::c_char;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strncasecmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong)
        -> libc::c_int;
    #[no_mangle]
    fn carray_new(initsize: libc::c_uint) -> *mut carray;
    #[no_mangle]
    fn carray_add(
        array: *mut carray,
        data: *mut libc::c_void,
        indx: *mut libc::c_uint,
    ) -> libc::c_int;
    #[no_mangle]
    fn carray_set_size(array: *mut carray, new_size: libc::c_uint) -> libc::c_int;
    #[no_mangle]
    fn carray_delete_slow(array: *mut carray, indx: libc::c_uint) -> libc::c_int;
    #[no_mangle]
    fn carray_free(array: *mut carray);
    #[no_mangle]
    fn mmap_string_unref(str: *mut libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn mailimf_mailbox_list_free(mb_list: *mut mailimf_mailbox_list);
    #[no_mangle]
    fn mailimf_fields_free(fields: *mut mailimf_fields);
    #[no_mangle]
    fn mailimf_mailbox_list_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailimf_mailbox_list,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailimf_envelope_and_optional_fields_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailimf_fields,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailmime_free(mime: *mut mailmime);
    #[no_mangle]
    fn mailmime_content_charset_get(content: *mut mailmime_content) -> *mut libc::c_char;
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
    fn charconv_buffer(
        tocode: *const libc::c_char,
        fromcode: *const libc::c_char,
        str: *const libc::c_char,
        length: size_t,
        result: *mut *mut libc::c_char,
        result_len: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn charconv_buffer_free(str: *mut libc::c_char);
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_replace_bad_utf8_chars(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_get_filemeta(
        buf: *const libc::c_void,
        buf_bytes: size_t,
        ret_width: *mut uint32_t,
        ret_height: *mut uint32_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_write_file(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
        buf: *const libc::c_void,
        buf_bytes: size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_get_fine_pathNfilename(
        _: *mut dc_context_t,
        pathNfolder: *const libc::c_char,
        desired_name: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_decode_header_words(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_decode_ext_header(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
    /* library-private */
    #[no_mangle]
    fn dc_param_new() -> *mut dc_param_t;
    #[no_mangle]
    fn dc_param_unref(_: *mut dc_param_t);
    /* Return the string with the given ID by calling DC_EVENT_GET_STRING.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str(_: *mut dc_context_t, id: libc::c_int) -> *mut libc::c_char;
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
    fn dc_addr_normalize(addr: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_kml_unref(_: *mut dc_kml_t);
    #[no_mangle]
    fn dc_e2ee_thanks(_: *mut dc_e2ee_helper_t);
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_simplify_unref(_: *mut dc_simplify_t);
    #[no_mangle]
    fn dc_kml_parse(
        _: *mut dc_context_t,
        content: *const libc::c_char,
        content_bytes: size_t,
    ) -> *mut dc_kml_t;
    /* Simplify and normalise text: Remove quotes, signatures, unnecessary
    lineends etc.
    The data returned from Simplify() must be free()'d when no longer used, private */
    #[no_mangle]
    fn dc_simplify_simplify(
        _: *mut dc_simplify_t,
        txt_unterminated: *const libc::c_char,
        txt_bytes: libc::c_int,
        is_html: libc::c_int,
        is_msgrmsg: libc::c_int,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_simplify_new() -> *mut dc_simplify_t;
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_e2ee_decrypt(
        _: *mut dc_context_t,
        in_out_message: *mut mailmime,
        _: *mut dc_e2ee_helper_t,
    );
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
pub const MAILIMF_ERROR_FILE: unnamed_3 = 4;
pub const MAILIMF_ERROR_INVAL: unnamed_3 = 3;
pub const MAILIMF_ERROR_MEMORY: unnamed_3 = 2;
pub const MAILIMF_ERROR_PARSE: unnamed_3 = 1;
pub const MAILIMF_NO_ERROR: unnamed_3 = 0;
pub type unnamed_4 = libc::c_uint;
pub const MAILMIME_COMPOSITE_TYPE_EXTENSION: unnamed_4 = 3;
pub const MAILMIME_COMPOSITE_TYPE_MULTIPART: unnamed_4 = 2;
pub const MAILMIME_COMPOSITE_TYPE_MESSAGE: unnamed_4 = 1;
pub const MAILMIME_COMPOSITE_TYPE_ERROR: unnamed_4 = 0;
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
    pub tp_data: unnamed_5,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_5 {
    pub tp_discrete_type: *mut mailmime_discrete_type,
    pub tp_composite_type: *mut mailmime_composite_type,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_discrete_type {
    pub dt_type: libc::c_int,
    pub dt_extension: *mut libc::c_char,
}
pub type unnamed_6 = libc::c_uint;
pub const MAILMIME_DISCRETE_TYPE_EXTENSION: unnamed_6 = 6;
pub const MAILMIME_DISCRETE_TYPE_APPLICATION: unnamed_6 = 5;
pub const MAILMIME_DISCRETE_TYPE_VIDEO: unnamed_6 = 4;
pub const MAILMIME_DISCRETE_TYPE_AUDIO: unnamed_6 = 3;
pub const MAILMIME_DISCRETE_TYPE_IMAGE: unnamed_6 = 2;
pub const MAILMIME_DISCRETE_TYPE_TEXT: unnamed_6 = 1;
pub const MAILMIME_DISCRETE_TYPE_ERROR: unnamed_6 = 0;
pub type unnamed_7 = libc::c_uint;
pub const MAILMIME_FIELD_LOCATION: unnamed_7 = 8;
pub const MAILMIME_FIELD_LANGUAGE: unnamed_7 = 7;
pub const MAILMIME_FIELD_DISPOSITION: unnamed_7 = 6;
pub const MAILMIME_FIELD_VERSION: unnamed_7 = 5;
pub const MAILMIME_FIELD_DESCRIPTION: unnamed_7 = 4;
pub const MAILMIME_FIELD_ID: unnamed_7 = 3;
pub const MAILMIME_FIELD_TRANSFER_ENCODING: unnamed_7 = 2;
pub const MAILMIME_FIELD_TYPE: unnamed_7 = 1;
pub const MAILMIME_FIELD_NONE: unnamed_7 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_field {
    pub fld_type: libc::c_int,
    pub fld_data: unnamed_8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_8 {
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
pub type unnamed_9 = libc::c_uint;
pub const MAILMIME_MECHANISM_TOKEN: unnamed_9 = 6;
pub const MAILMIME_MECHANISM_BASE64: unnamed_9 = 5;
pub const MAILMIME_MECHANISM_QUOTED_PRINTABLE: unnamed_9 = 4;
pub const MAILMIME_MECHANISM_BINARY: unnamed_9 = 3;
pub const MAILMIME_MECHANISM_8BIT: unnamed_9 = 2;
pub const MAILMIME_MECHANISM_7BIT: unnamed_9 = 1;
pub const MAILMIME_MECHANISM_ERROR: unnamed_9 = 0;
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
pub type unnamed_10 = libc::c_uint;
pub const MAILMIME_TYPE_COMPOSITE_TYPE: unnamed_10 = 2;
pub const MAILMIME_TYPE_DISCRETE_TYPE: unnamed_10 = 1;
pub const MAILMIME_TYPE_ERROR: unnamed_10 = 0;
pub type unnamed_11 = libc::c_uint;
pub const MAILMIME_DATA_FILE: unnamed_11 = 1;
pub const MAILMIME_DATA_TEXT: unnamed_11 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_data {
    pub dt_type: libc::c_int,
    pub dt_encoding: libc::c_int,
    pub dt_encoded: libc::c_int,
    pub dt_data: unnamed_12,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_12 {
    pub dt_text: unnamed_13,
    pub dt_filename: *mut libc::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_13 {
    pub dt_data: *const libc::c_char,
    pub dt_length: size_t,
}
pub type unnamed_14 = libc::c_uint;
pub const MAILMIME_MESSAGE: unnamed_14 = 3;
pub const MAILMIME_MULTIPLE: unnamed_14 = 2;
pub const MAILMIME_SINGLE: unnamed_14 = 1;
pub const MAILMIME_NONE: unnamed_14 = 0;
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
    pub mm_data: unnamed_15,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_15 {
    pub mm_single: *mut mailmime_data,
    pub mm_multipart: unnamed_17,
    pub mm_message: unnamed_16,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_16 {
    pub mm_fields: *mut mailimf_fields,
    pub mm_msg_mime: *mut mailmime,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed_17 {
    pub mm_preamble: *mut mailmime_data,
    pub mm_epilogue: *mut mailmime_data,
    pub mm_mp_list: *mut clist,
}
pub type unnamed_18 = libc::c_uint;
pub const MAILMIME_DISPOSITION_TYPE_EXTENSION: unnamed_18 = 3;
pub const MAILMIME_DISPOSITION_TYPE_ATTACHMENT: unnamed_18 = 2;
pub const MAILMIME_DISPOSITION_TYPE_INLINE: unnamed_18 = 1;
pub const MAILMIME_DISPOSITION_TYPE_ERROR: unnamed_18 = 0;
pub type unnamed_19 = libc::c_uint;
pub const MAILMIME_DISPOSITION_PARM_PARAMETER: unnamed_19 = 5;
pub const MAILMIME_DISPOSITION_PARM_SIZE: unnamed_19 = 4;
pub const MAILMIME_DISPOSITION_PARM_READ_DATE: unnamed_19 = 3;
pub const MAILMIME_DISPOSITION_PARM_MODIFICATION_DATE: unnamed_19 = 2;
pub const MAILMIME_DISPOSITION_PARM_CREATION_DATE: unnamed_19 = 1;
pub const MAILMIME_DISPOSITION_PARM_FILENAME: unnamed_19 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct mailmime_disposition_parm {
    pub pa_type: libc::c_int,
    pub pa_data: unnamed_20,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_20 {
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
    pub smtp_sasl: unnamed_21,
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
pub struct unnamed_21 {
    pub sasl_conn: *mut libc::c_void,
    pub sasl_server_fqdn: *const libc::c_char,
    pub sasl_login: *const libc::c_char,
    pub sasl_auth_name: *const libc::c_char,
    pub sasl_password: *const libc::c_char,
    pub sasl_realm: *const libc::c_char,
    pub sasl_secret: *mut libc::c_void,
}
pub type unnamed_22 = libc::c_uint;
pub const MAIL_CHARCONV_ERROR_CONV: unnamed_22 = 3;
pub const MAIL_CHARCONV_ERROR_MEMORY: unnamed_22 = 2;
pub const MAIL_CHARCONV_ERROR_UNKNOWN_CHARSET: unnamed_22 = 1;
pub const MAIL_CHARCONV_NO_ERROR: unnamed_22 = 0;
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
    pub sec_data: unnamed_23,
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
pub union unnamed_23 {
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
    pub ft_data: unnamed_24,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_24 {
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
    pub imap_sasl: unnamed_25,
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
pub struct unnamed_25 {
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type dc_strbuilder_t = _dc_strbuilder;
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
// backups
// attachments of 25 mb brutto should work on the majority of providers
// (brutto examples: web.de=50, 1&1=40, t-online.de=32, gmail=25, posteo=50, yahoo=25, all-inkl=100).
// as an upper limit, we double the size; the core won't send messages larger than this
// to get the netto sizes, we substract 1 mb header-overhead and the base64-overhead.
// some defaults
pub type dc_e2ee_helper_t = _dc_e2ee_helper;
pub type dc_kml_t = _dc_kml;
/* ** library-private **********************************************************/
pub type dc_simplify_t = _dc_simplify;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_simplify {
    pub is_forwarded: libc::c_int,
    pub is_cut_at_begin: libc::c_int,
    pub is_cut_at_end: libc::c_int,
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
// deprecated
#[no_mangle]
pub unsafe extern "C" fn dc_no_compound_msgs() {
    s_generate_compound_msgs = 0i32;
}
// deprecated: flag to switch generation of compound messages on and off.
static mut s_generate_compound_msgs: libc::c_int = 1i32;
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_new(
    mut blobdir: *const libc::c_char,
    mut context: *mut dc_context_t,
) -> *mut dc_mimeparser_t {
    let mut mimeparser: *mut dc_mimeparser_t = 0 as *mut dc_mimeparser_t;
    mimeparser = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_mimeparser_t>() as libc::c_ulong,
    ) as *mut dc_mimeparser_t;
    if mimeparser.is_null() {
        exit(30i32);
    }
    (*mimeparser).context = context;
    (*mimeparser).parts = carray_new(16i32 as libc::c_uint);
    (*mimeparser).blobdir = blobdir;
    (*mimeparser).reports = carray_new(16i32 as libc::c_uint);
    (*mimeparser).e2ee_helper = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_e2ee_helper_t>() as libc::c_ulong,
    ) as *mut _dc_e2ee_helper;
    dc_hash_init(&mut (*mimeparser).header, 3i32, 0i32);
    return mimeparser;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_unref(mut mimeparser: *mut dc_mimeparser_t) {
    if mimeparser.is_null() {
        return;
    }
    dc_mimeparser_empty(mimeparser);
    if !(*mimeparser).parts.is_null() {
        carray_free((*mimeparser).parts);
    }
    if !(*mimeparser).reports.is_null() {
        carray_free((*mimeparser).reports);
    }
    free((*mimeparser).e2ee_helper as *mut libc::c_void);
    free(mimeparser as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_empty(mut mimeparser: *mut dc_mimeparser_t) {
    if mimeparser.is_null() {
        return;
    }
    if !(*mimeparser).parts.is_null() {
        let mut i: libc::c_int = 0;
        let mut cnt: libc::c_int = carray_count((*mimeparser).parts) as libc::c_int;
        i = 0i32;
        while i < cnt {
            let mut part: *mut dc_mimepart_t =
                carray_get((*mimeparser).parts, i as libc::c_uint) as *mut dc_mimepart_t;
            if !part.is_null() {
                dc_mimepart_unref(part);
            }
            i += 1
        }
        carray_set_size((*mimeparser).parts, 0i32 as libc::c_uint);
    }
    (*mimeparser).header_root = 0 as *mut mailimf_fields;
    dc_hash_clear(&mut (*mimeparser).header);
    if !(*mimeparser).header_protected.is_null() {
        mailimf_fields_free((*mimeparser).header_protected);
        (*mimeparser).header_protected = 0 as *mut mailimf_fields
    }
    (*mimeparser).is_send_by_messenger = 0i32;
    (*mimeparser).is_system_message = 0i32;
    free((*mimeparser).subject as *mut libc::c_void);
    (*mimeparser).subject = 0 as *mut libc::c_char;
    if !(*mimeparser).mimeroot.is_null() {
        mailmime_free((*mimeparser).mimeroot);
        (*mimeparser).mimeroot = 0 as *mut mailmime
    }
    (*mimeparser).is_forwarded = 0i32;
    if !(*mimeparser).reports.is_null() {
        carray_set_size((*mimeparser).reports, 0i32 as libc::c_uint);
    }
    (*mimeparser).decrypting_failed = 0i32;
    dc_e2ee_thanks((*mimeparser).e2ee_helper);
    dc_kml_unref((*mimeparser).kml);
    (*mimeparser).kml = 0 as *mut _dc_kml;
}
unsafe extern "C" fn dc_mimepart_unref(mut mimepart: *mut dc_mimepart_t) {
    if mimepart.is_null() {
        return;
    }
    free((*mimepart).msg as *mut libc::c_void);
    (*mimepart).msg = 0 as *mut libc::c_char;
    free((*mimepart).msg_raw as *mut libc::c_void);
    (*mimepart).msg_raw = 0 as *mut libc::c_char;
    dc_param_unref((*mimepart).param);
    free(mimepart as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_parse(
    mut mimeparser: *mut dc_mimeparser_t,
    mut body_not_terminated: *const libc::c_char,
    mut body_bytes: size_t,
) {
    let mut r: libc::c_int = 0i32;
    let mut index: size_t = 0i32 as size_t;
    let mut optional_field: *mut mailimf_optional_field = 0 as *mut mailimf_optional_field;
    dc_mimeparser_empty(mimeparser);
    r = mailmime_parse(
        body_not_terminated,
        body_bytes,
        &mut index,
        &mut (*mimeparser).mimeroot,
    );
    if !(r != MAILIMF_NO_ERROR as libc::c_int || (*mimeparser).mimeroot.is_null()) {
        dc_e2ee_decrypt(
            (*mimeparser).context,
            (*mimeparser).mimeroot,
            (*mimeparser).e2ee_helper,
        );
        dc_mimeparser_parse_mime_recursive(mimeparser, (*mimeparser).mimeroot);
        let mut field: *mut mailimf_field = dc_mimeparser_lookup_field(
            mimeparser,
            b"Subject\x00" as *const u8 as *const libc::c_char,
        );
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_SUBJECT as libc::c_int {
            (*mimeparser).subject =
                dc_decode_header_words((*(*field).fld_data.fld_subject).sbj_value)
        }
        if !dc_mimeparser_lookup_optional_field(
            mimeparser,
            b"Chat-Version\x00" as *const u8 as *const libc::c_char,
        )
        .is_null()
        {
            (*mimeparser).is_send_by_messenger = 1i32
        }
        if !dc_mimeparser_lookup_field(
            mimeparser,
            b"Autocrypt-Setup-Message\x00" as *const u8 as *const libc::c_char,
        )
        .is_null()
        {
            let mut i: libc::c_int = 0;
            let mut has_setup_file: libc::c_int = 0i32;
            i = 0i32;
            while (i as libc::c_uint) < carray_count((*mimeparser).parts) {
                let mut part: *mut dc_mimepart_t =
                    carray_get((*mimeparser).parts, i as libc::c_uint) as *mut dc_mimepart_t;
                if (*part).int_mimetype == 111i32 {
                    has_setup_file = 1i32
                }
                i += 1
            }
            if 0 != has_setup_file {
                (*mimeparser).is_system_message = 6i32;
                i = 0i32;
                while (i as libc::c_uint) < carray_count((*mimeparser).parts) {
                    let mut part_0: *mut dc_mimepart_t =
                        carray_get((*mimeparser).parts, i as libc::c_uint) as *mut dc_mimepart_t;
                    if (*part_0).int_mimetype != 111i32 {
                        dc_mimepart_unref(part_0);
                        carray_delete_slow((*mimeparser).parts, i as libc::c_uint);
                        i -= 1
                    }
                    i += 1
                }
            }
        } else {
            optional_field = dc_mimeparser_lookup_optional_field(
                mimeparser,
                b"Chat-Content\x00" as *const u8 as *const libc::c_char,
            );
            if !optional_field.is_null() && !(*optional_field).fld_value.is_null() {
                if strcmp(
                    (*optional_field).fld_value,
                    b"location-streaming-enabled\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    (*mimeparser).is_system_message = 8i32
                }
            }
        }
        if !dc_mimeparser_lookup_field(
            mimeparser,
            b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char,
        )
        .is_null()
            && carray_count((*mimeparser).parts) >= 1i32 as libc::c_uint
        {
            let mut textpart: *mut dc_mimepart_t =
                carray_get((*mimeparser).parts, 0i32 as libc::c_uint) as *mut dc_mimepart_t;
            if (*textpart).type_0 == 10i32 {
                if carray_count((*mimeparser).parts) >= 2i32 as libc::c_uint {
                    let mut imgpart: *mut dc_mimepart_t =
                        carray_get((*mimeparser).parts, 1i32 as libc::c_uint) as *mut dc_mimepart_t;
                    if (*imgpart).type_0 == 20i32 {
                        (*imgpart).is_meta = 1i32
                    }
                }
            }
        }
        if 0 != (*mimeparser).is_send_by_messenger
            && 0 != s_generate_compound_msgs
            && carray_count((*mimeparser).parts) == 2i32 as libc::c_uint
        {
            let mut textpart_0: *mut dc_mimepart_t =
                carray_get((*mimeparser).parts, 0i32 as libc::c_uint) as *mut dc_mimepart_t;
            let mut filepart: *mut dc_mimepart_t =
                carray_get((*mimeparser).parts, 1i32 as libc::c_uint) as *mut dc_mimepart_t;
            if (*textpart_0).type_0 == 10i32
                && ((*filepart).type_0 == 20i32
                    || (*filepart).type_0 == 21i32
                    || (*filepart).type_0 == 40i32
                    || (*filepart).type_0 == 41i32
                    || (*filepart).type_0 == 50i32
                    || (*filepart).type_0 == 60i32)
                && 0 == (*filepart).is_meta
            {
                free((*filepart).msg as *mut libc::c_void);
                (*filepart).msg = (*textpart_0).msg;
                (*textpart_0).msg = 0 as *mut libc::c_char;
                dc_mimepart_unref(textpart_0);
                carray_delete_slow((*mimeparser).parts, 0i32 as libc::c_uint);
            }
        }
        if !(*mimeparser).subject.is_null() {
            let mut prepend_subject: libc::c_int = 1i32;
            if 0 == (*mimeparser).decrypting_failed {
                let mut p: *mut libc::c_char = strchr((*mimeparser).subject, ':' as i32);
                if p.wrapping_offset_from((*mimeparser).subject) as libc::c_long
                    == 2i32 as libc::c_long
                    || p.wrapping_offset_from((*mimeparser).subject) as libc::c_long
                        == 3i32 as libc::c_long
                    || 0 != (*mimeparser).is_send_by_messenger
                    || !strstr(
                        (*mimeparser).subject,
                        b"Chat:\x00" as *const u8 as *const libc::c_char,
                    )
                    .is_null()
                {
                    prepend_subject = 0i32
                }
            }
            if 0 != prepend_subject {
                let mut subj: *mut libc::c_char = dc_strdup((*mimeparser).subject);
                let mut p_0: *mut libc::c_char = strchr(subj, '[' as i32);
                if !p_0.is_null() {
                    *p_0 = 0i32 as libc::c_char
                }
                dc_trim(subj);
                if 0 != *subj.offset(0isize) {
                    let mut i_0: libc::c_int = 0;
                    let mut icnt: libc::c_int = carray_count((*mimeparser).parts) as libc::c_int;
                    i_0 = 0i32;
                    while i_0 < icnt {
                        let mut part_1: *mut dc_mimepart_t =
                            carray_get((*mimeparser).parts, i_0 as libc::c_uint)
                                as *mut dc_mimepart_t;
                        if (*part_1).type_0 == 10i32 {
                            let mut new_txt: *mut libc::c_char = dc_mprintf(
                                b"%s \xe2\x80\x93 %s\x00" as *const u8 as *const libc::c_char,
                                subj,
                                (*part_1).msg,
                            );
                            free((*part_1).msg as *mut libc::c_void);
                            (*part_1).msg = new_txt;
                            break;
                        } else {
                            i_0 += 1
                        }
                    }
                }
                free(subj as *mut libc::c_void);
            }
        }
        if 0 != (*mimeparser).is_forwarded {
            let mut i_1: libc::c_int = 0;
            let mut icnt_0: libc::c_int = carray_count((*mimeparser).parts) as libc::c_int;
            i_1 = 0i32;
            while i_1 < icnt_0 {
                let mut part_2: *mut dc_mimepart_t =
                    carray_get((*mimeparser).parts, i_1 as libc::c_uint) as *mut dc_mimepart_t;
                dc_param_set_int((*part_2).param, 'a' as i32, 1i32);
                i_1 += 1
            }
        }
        if carray_count((*mimeparser).parts) == 1i32 as libc::c_uint {
            let mut part_3: *mut dc_mimepart_t =
                carray_get((*mimeparser).parts, 0i32 as libc::c_uint) as *mut dc_mimepart_t;
            if (*part_3).type_0 == 40i32 {
                if !dc_mimeparser_lookup_optional_field(
                    mimeparser,
                    b"Chat-Voice-Message\x00" as *const u8 as *const libc::c_char,
                )
                .is_null()
                {
                    (*part_3).type_0 = 41i32
                }
            }
            if (*part_3).type_0 == 40i32 || (*part_3).type_0 == 41i32 || (*part_3).type_0 == 50i32 {
                let mut field_0: *const mailimf_optional_field =
                    dc_mimeparser_lookup_optional_field(
                        mimeparser,
                        b"Chat-Duration\x00" as *const u8 as *const libc::c_char,
                    );
                if !field_0.is_null() {
                    let mut duration_ms: libc::c_int = atoi((*field_0).fld_value);
                    if duration_ms > 0i32 && duration_ms < 24i32 * 60i32 * 60i32 * 1000i32 {
                        dc_param_set_int((*part_3).param, 'd' as i32, duration_ms);
                    }
                }
            }
        }
        if 0 == (*mimeparser).decrypting_failed {
            let mut dn_field: *const mailimf_optional_field = dc_mimeparser_lookup_optional_field(
                mimeparser,
                b"Chat-Disposition-Notification-To\x00" as *const u8 as *const libc::c_char,
            );
            if !dn_field.is_null() && !dc_mimeparser_get_last_nonmeta(mimeparser).is_null() {
                let mut mb_list: *mut mailimf_mailbox_list = 0 as *mut mailimf_mailbox_list;
                let mut index_0: size_t = 0i32 as size_t;
                if mailimf_mailbox_list_parse(
                    (*dn_field).fld_value,
                    strlen((*dn_field).fld_value),
                    &mut index_0,
                    &mut mb_list,
                ) == MAILIMF_NO_ERROR as libc::c_int
                    && !mb_list.is_null()
                {
                    let mut dn_to_addr: *mut libc::c_char = mailimf_find_first_addr(mb_list);
                    if !dn_to_addr.is_null() {
                        let mut from_field: *mut mailimf_field = dc_mimeparser_lookup_field(
                            mimeparser,
                            b"From\x00" as *const u8 as *const libc::c_char,
                        );
                        if !from_field.is_null()
                            && (*from_field).fld_type == MAILIMF_FIELD_FROM as libc::c_int
                            && !(*from_field).fld_data.fld_from.is_null()
                        {
                            let mut from_addr: *mut libc::c_char = mailimf_find_first_addr(
                                (*(*from_field).fld_data.fld_from).frm_mb_list,
                            );
                            if !from_addr.is_null() {
                                if strcmp(from_addr, dn_to_addr) == 0i32 {
                                    let mut part_4: *mut dc_mimepart_t =
                                        dc_mimeparser_get_last_nonmeta(mimeparser);
                                    if !part_4.is_null() {
                                        dc_param_set_int((*part_4).param, 'r' as i32, 1i32);
                                    }
                                }
                                free(from_addr as *mut libc::c_void);
                            }
                        }
                        free(dn_to_addr as *mut libc::c_void);
                    }
                    mailimf_mailbox_list_free(mb_list);
                }
            }
        }
    }
    /* Cleanup - and try to create at least an empty part if there are no parts yet */
    if dc_mimeparser_get_last_nonmeta(mimeparser).is_null()
        && carray_count((*mimeparser).reports) == 0i32 as libc::c_uint
    {
        let mut part_5: *mut dc_mimepart_t = dc_mimepart_new();
        (*part_5).type_0 = 10i32;
        if !(*mimeparser).subject.is_null() && 0 == (*mimeparser).is_send_by_messenger {
            (*part_5).msg = dc_strdup((*mimeparser).subject)
        } else {
            (*part_5).msg = dc_strdup(b"\x00" as *const u8 as *const libc::c_char)
        }
        carray_add(
            (*mimeparser).parts,
            part_5 as *mut libc::c_void,
            0 as *mut libc::c_uint,
        );
    };
}
/* ******************************************************************************
 * a MIME part
 ******************************************************************************/
unsafe extern "C" fn dc_mimepart_new() -> *mut dc_mimepart_t {
    let mut mimepart: *mut dc_mimepart_t = 0 as *mut dc_mimepart_t;
    mimepart = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_mimepart_t>() as libc::c_ulong,
    ) as *mut dc_mimepart_t;
    if mimepart.is_null() {
        exit(33i32);
    }
    (*mimepart).type_0 = 0i32;
    (*mimepart).param = dc_param_new();
    return mimepart;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_get_last_nonmeta(
    mut mimeparser: *mut dc_mimeparser_t,
) -> *mut dc_mimepart_t {
    if !mimeparser.is_null() && !(*mimeparser).parts.is_null() {
        let mut i: libc::c_int = 0;
        let mut icnt: libc::c_int = carray_count((*mimeparser).parts) as libc::c_int;
        i = icnt - 1i32;
        while i >= 0i32 {
            let mut part: *mut dc_mimepart_t =
                carray_get((*mimeparser).parts, i as libc::c_uint) as *mut dc_mimepart_t;
            if !part.is_null() && 0 == (*part).is_meta {
                return part;
            }
            i -= 1
        }
    }
    return 0 as *mut dc_mimepart_t;
}
/*the result must be freed*/
#[no_mangle]
pub unsafe extern "C" fn mailimf_find_first_addr(
    mut mb_list: *const mailimf_mailbox_list,
) -> *mut libc::c_char {
    if mb_list.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut cur: *mut clistiter = (*(*mb_list).mb_list).first;
    while !cur.is_null() {
        let mut mb: *mut mailimf_mailbox = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_mailbox;
        if !mb.is_null() && !(*mb).mb_addr_spec.is_null() {
            return dc_addr_normalize((*mb).mb_addr_spec);
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return 0 as *mut libc::c_char;
}
/* the following functions can be used only after a call to dc_mimeparser_parse() */
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_lookup_field(
    mut mimeparser: *mut dc_mimeparser_t,
    mut field_name: *const libc::c_char,
) -> *mut mailimf_field {
    return dc_hash_find(
        &mut (*mimeparser).header,
        field_name as *const libc::c_void,
        strlen(field_name) as libc::c_int,
    ) as *mut mailimf_field;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_lookup_optional_field(
    mut mimeparser: *mut dc_mimeparser_t,
    mut field_name: *const libc::c_char,
) -> *mut mailimf_optional_field {
    let mut field: *mut mailimf_field = dc_hash_find(
        &mut (*mimeparser).header,
        field_name as *const libc::c_void,
        strlen(field_name) as libc::c_int,
    ) as *mut mailimf_field;
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
        return (*field).fld_data.fld_optional_field;
    }
    return 0 as *mut mailimf_optional_field;
}
unsafe extern "C" fn dc_mimeparser_parse_mime_recursive(
    mut mimeparser: *mut dc_mimeparser_t,
    mut mime: *mut mailmime,
) -> libc::c_int {
    let mut any_part_added: libc::c_int = 0i32;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    if mimeparser.is_null() || mime.is_null() {
        return 0i32;
    }
    if !mailmime_find_ct_parameter(
        mime,
        b"protected-headers\x00" as *const u8 as *const libc::c_char,
    )
    .is_null()
    {
        if (*mime).mm_type == MAILMIME_SINGLE as libc::c_int
            && (*(*(*mime).mm_content_type).ct_type).tp_type
                == MAILMIME_TYPE_DISCRETE_TYPE as libc::c_int
            && (*(*(*(*mime).mm_content_type).ct_type)
                .tp_data
                .tp_discrete_type)
                .dt_type
                == MAILMIME_DISCRETE_TYPE_TEXT as libc::c_int
            && !(*(*mime).mm_content_type).ct_subtype.is_null()
            && strcmp(
                (*(*mime).mm_content_type).ct_subtype,
                b"rfc822-headers\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
        {
            dc_log_info(
                (*mimeparser).context,
                0i32,
                b"Protected headers found in text/rfc822-headers attachment: Will be ignored.\x00"
                    as *const u8 as *const libc::c_char,
            );
            return 0i32;
        }
        if (*mimeparser).header_protected.is_null() {
            let mut dummy: size_t = 0i32 as size_t;
            if mailimf_envelope_and_optional_fields_parse(
                (*mime).mm_mime_start,
                (*mime).mm_length,
                &mut dummy,
                &mut (*mimeparser).header_protected,
            ) != MAILIMF_NO_ERROR as libc::c_int
                || (*mimeparser).header_protected.is_null()
            {
                dc_log_warning(
                    (*mimeparser).context,
                    0i32,
                    b"Protected headers parsing error.\x00" as *const u8 as *const libc::c_char,
                );
            } else {
                hash_header(
                    &mut (*mimeparser).header,
                    (*mimeparser).header_protected,
                    (*mimeparser).context,
                );
            }
        } else {
            dc_log_info((*mimeparser).context, 0i32,
                        b"Protected headers found in MIME header: Will be ignored as we already found an outer one.\x00"
                            as *const u8 as *const libc::c_char);
        }
    }
    match (*mime).mm_type {
        1 => any_part_added = dc_mimeparser_add_single_part_if_known(mimeparser, mime),
        2 => {
            match mailmime_get_mime_type(mime, 0 as *mut libc::c_int, 0 as *mut *mut libc::c_char) {
                10 => {
                    cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                    while !cur.is_null() {
                        let mut childmime: *mut mailmime = (if !cur.is_null() {
                            (*cur).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *mut mailmime;
                        if mailmime_get_mime_type(
                            childmime,
                            0 as *mut libc::c_int,
                            0 as *mut *mut libc::c_char,
                        ) == 30i32
                        {
                            any_part_added =
                                dc_mimeparser_parse_mime_recursive(mimeparser, childmime);
                            break;
                        } else {
                            cur = if !cur.is_null() {
                                (*cur).next
                            } else {
                                0 as *mut clistcell_s
                            }
                        }
                    }
                    if 0 == any_part_added {
                        cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                        while !cur.is_null() {
                            let mut childmime_0: *mut mailmime = (if !cur.is_null() {
                                (*cur).data
                            } else {
                                0 as *mut libc::c_void
                            })
                                as *mut mailmime;
                            if mailmime_get_mime_type(
                                childmime_0,
                                0 as *mut libc::c_int,
                                0 as *mut *mut libc::c_char,
                            ) == 60i32
                            {
                                any_part_added =
                                    dc_mimeparser_parse_mime_recursive(mimeparser, childmime_0);
                                break;
                            } else {
                                cur = if !cur.is_null() {
                                    (*cur).next
                                } else {
                                    0 as *mut clistcell_s
                                }
                            }
                        }
                    }
                    if 0 == any_part_added {
                        cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                        while !cur.is_null() {
                            if 0 != dc_mimeparser_parse_mime_recursive(
                                mimeparser,
                                (if !cur.is_null() {
                                    (*cur).data
                                } else {
                                    0 as *mut libc::c_void
                                }) as *mut mailmime,
                            ) {
                                any_part_added = 1i32;
                                /* out of for() */
                                break;
                            } else {
                                cur = if !cur.is_null() {
                                    (*cur).next
                                } else {
                                    0 as *mut clistcell_s
                                }
                            }
                        }
                    }
                }
                20 => {
                    cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                    if !cur.is_null() {
                        any_part_added = dc_mimeparser_parse_mime_recursive(
                            mimeparser,
                            (if !cur.is_null() {
                                (*cur).data
                            } else {
                                0 as *mut libc::c_void
                            }) as *mut mailmime,
                        )
                    }
                }
                40 => {
                    let mut part: *mut dc_mimepart_t = dc_mimepart_new();
                    (*part).type_0 = 10i32;
                    let mut msg_body: *mut libc::c_char =
                        dc_stock_str((*mimeparser).context, 29i32);
                    (*part).msg =
                        dc_mprintf(b"[%s]\x00" as *const u8 as *const libc::c_char, msg_body);
                    (*part).msg_raw = dc_strdup((*part).msg);
                    free(msg_body as *mut libc::c_void);
                    carray_add(
                        (*mimeparser).parts,
                        part as *mut libc::c_void,
                        0 as *mut libc::c_uint,
                    );
                    any_part_added = 1i32;
                    (*mimeparser).decrypting_failed = 1i32
                }
                46 => {
                    cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                    if !cur.is_null() {
                        any_part_added = dc_mimeparser_parse_mime_recursive(
                            mimeparser,
                            (if !cur.is_null() {
                                (*cur).data
                            } else {
                                0 as *mut libc::c_void
                            }) as *mut mailmime,
                        )
                    }
                }
                45 => {
                    if (*(*mime).mm_data.mm_multipart.mm_mp_list).count >= 2i32 {
                        let mut report_type: *mut mailmime_parameter = mailmime_find_ct_parameter(
                            mime,
                            b"report-type\x00" as *const u8 as *const libc::c_char,
                        );
                        if !report_type.is_null()
                            && !(*report_type).pa_value.is_null()
                            && strcmp(
                                (*report_type).pa_value,
                                b"disposition-notification\x00" as *const u8 as *const libc::c_char,
                            ) == 0i32
                        {
                            carray_add(
                                (*mimeparser).reports,
                                mime as *mut libc::c_void,
                                0 as *mut libc::c_uint,
                            );
                        } else {
                            any_part_added = dc_mimeparser_parse_mime_recursive(
                                mimeparser,
                                (if !(*(*mime).mm_data.mm_multipart.mm_mp_list).first.is_null() {
                                    (*(*(*mime).mm_data.mm_multipart.mm_mp_list).first).data
                                } else {
                                    0 as *mut libc::c_void
                                }) as *mut mailmime,
                            )
                        }
                    }
                }
                _ => {
                    let mut skip_part: *mut mailmime = 0 as *mut mailmime;
                    let mut html_part: *mut mailmime = 0 as *mut mailmime;
                    let mut plain_cnt: libc::c_int = 0i32;
                    let mut html_cnt: libc::c_int = 0i32;
                    cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                    while !cur.is_null() {
                        let mut childmime_1: *mut mailmime = (if !cur.is_null() {
                            (*cur).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *mut mailmime;
                        if mailmime_get_mime_type(
                            childmime_1,
                            0 as *mut libc::c_int,
                            0 as *mut *mut libc::c_char,
                        ) == 60i32
                        {
                            plain_cnt += 1
                        } else if mailmime_get_mime_type(
                            childmime_1,
                            0 as *mut libc::c_int,
                            0 as *mut *mut libc::c_char,
                        ) == 70i32
                        {
                            html_part = childmime_1;
                            html_cnt += 1
                        }
                        cur = if !cur.is_null() {
                            (*cur).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                    if plain_cnt == 1i32 && html_cnt == 1i32 {
                        dc_log_warning((*mimeparser).context, 0i32,
                                       b"HACK: multipart/mixed message found with PLAIN and HTML, we\'ll skip the HTML part as this seems to be unwanted.\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                        skip_part = html_part
                    }
                    cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                    while !cur.is_null() {
                        let mut childmime_2: *mut mailmime = (if !cur.is_null() {
                            (*cur).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *mut mailmime;
                        if childmime_2 != skip_part {
                            if 0 != dc_mimeparser_parse_mime_recursive(mimeparser, childmime_2) {
                                any_part_added = 1i32
                            }
                        }
                        cur = if !cur.is_null() {
                            (*cur).next
                        } else {
                            0 as *mut clistcell_s
                        }
                    }
                }
            }
        }
        3 => {
            if (*mimeparser).header_root.is_null() {
                (*mimeparser).header_root = (*mime).mm_data.mm_message.mm_fields;
                hash_header(
                    &mut (*mimeparser).header,
                    (*mimeparser).header_root,
                    (*mimeparser).context,
                );
            }
            if !(*mime).mm_data.mm_message.mm_msg_mime.is_null() {
                any_part_added = dc_mimeparser_parse_mime_recursive(
                    mimeparser,
                    (*mime).mm_data.mm_message.mm_msg_mime,
                )
            }
        }
        _ => {}
    }
    return any_part_added;
}
unsafe extern "C" fn hash_header(
    mut out: *mut dc_hash_t,
    mut in_0: *const mailimf_fields,
    mut context: *mut dc_context_t,
) {
    if in_0.is_null() {
        return;
    }
    let mut cur1: *mut clistiter = (*(*in_0).fld_list).first;
    while !cur1.is_null() {
        let mut field: *mut mailimf_field = (if !cur1.is_null() {
            (*cur1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        let mut key: *const libc::c_char = 0 as *const libc::c_char;
        match (*field).fld_type {
            1 => key = b"Return-Path\x00" as *const u8 as *const libc::c_char,
            9 => key = b"Date\x00" as *const u8 as *const libc::c_char,
            10 => key = b"From\x00" as *const u8 as *const libc::c_char,
            11 => key = b"Sender\x00" as *const u8 as *const libc::c_char,
            12 => key = b"Reply-To\x00" as *const u8 as *const libc::c_char,
            13 => key = b"To\x00" as *const u8 as *const libc::c_char,
            14 => key = b"Cc\x00" as *const u8 as *const libc::c_char,
            15 => key = b"Bcc\x00" as *const u8 as *const libc::c_char,
            16 => key = b"Message-ID\x00" as *const u8 as *const libc::c_char,
            17 => key = b"In-Reply-To\x00" as *const u8 as *const libc::c_char,
            18 => key = b"References\x00" as *const u8 as *const libc::c_char,
            19 => key = b"Subject\x00" as *const u8 as *const libc::c_char,
            22 => {
                let mut optional_field: *const mailimf_optional_field =
                    (*field).fld_data.fld_optional_field;
                if !optional_field.is_null() {
                    key = (*optional_field).fld_name
                }
            }
            _ => {}
        }
        if !key.is_null() {
            let mut key_len: libc::c_int = strlen(key) as libc::c_int;
            if !dc_hash_find(out, key as *const libc::c_void, key_len).is_null() {
                if (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
                    || key_len > 5i32
                        && strncasecmp(
                            key,
                            b"Chat-\x00" as *const u8 as *const libc::c_char,
                            5i32 as libc::c_ulong,
                        ) == 0i32
                {
                    dc_hash_insert(
                        out,
                        key as *const libc::c_void,
                        key_len,
                        field as *mut libc::c_void,
                    );
                }
            } else {
                dc_hash_insert(
                    out,
                    key as *const libc::c_void,
                    key_len,
                    field as *mut libc::c_void,
                );
            }
        }
        cur1 = if !cur1.is_null() {
            (*cur1).next
        } else {
            0 as *mut clistcell_s
        }
    }
}
unsafe extern "C" fn mailmime_get_mime_type(
    mut mime: *mut mailmime,
    mut msg_type: *mut libc::c_int,
    mut raw_mime: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut c: *mut mailmime_content = (*mime).mm_content_type;
    let mut dummy: libc::c_int = 0i32;
    if msg_type.is_null() {
        msg_type = &mut dummy
    }
    *msg_type = 0i32;
    if c.is_null() || (*c).ct_type.is_null() {
        return 0i32;
    }
    match (*(*c).ct_type).tp_type {
        1 => match (*(*(*c).ct_type).tp_data.tp_discrete_type).dt_type {
            1 => {
                if !(0 != mailmime_is_attachment_disposition(mime)) {
                    if strcmp(
                        (*c).ct_subtype,
                        b"plain\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                    {
                        *msg_type = 10i32;
                        return 60i32;
                    } else {
                        if strcmp(
                            (*c).ct_subtype,
                            b"html\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        {
                            *msg_type = 10i32;
                            return 70i32;
                        }
                    }
                }
                *msg_type = 60i32;
                reconcat_mime(
                    raw_mime,
                    b"text\x00" as *const u8 as *const libc::c_char,
                    (*c).ct_subtype,
                );
                return 110i32;
            }
            2 => {
                if strcmp(
                    (*c).ct_subtype,
                    b"gif\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    *msg_type = 21i32
                } else if strcmp(
                    (*c).ct_subtype,
                    b"svg+xml\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    *msg_type = 60i32;
                    reconcat_mime(
                        raw_mime,
                        b"image\x00" as *const u8 as *const libc::c_char,
                        (*c).ct_subtype,
                    );
                    return 110i32;
                } else {
                    *msg_type = 20i32
                }
                reconcat_mime(
                    raw_mime,
                    b"image\x00" as *const u8 as *const libc::c_char,
                    (*c).ct_subtype,
                );
                return 80i32;
            }
            3 => {
                *msg_type = 40i32;
                reconcat_mime(
                    raw_mime,
                    b"audio\x00" as *const u8 as *const libc::c_char,
                    (*c).ct_subtype,
                );
                return 90i32;
            }
            4 => {
                *msg_type = 50i32;
                reconcat_mime(
                    raw_mime,
                    b"video\x00" as *const u8 as *const libc::c_char,
                    (*c).ct_subtype,
                );
                return 100i32;
            }
            _ => {
                *msg_type = 60i32;
                if (*(*(*c).ct_type).tp_data.tp_discrete_type).dt_type
                    == MAILMIME_DISCRETE_TYPE_APPLICATION as libc::c_int
                    && strcmp(
                        (*c).ct_subtype,
                        b"autocrypt-setup\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                {
                    reconcat_mime(
                        raw_mime,
                        b"application\x00" as *const u8 as *const libc::c_char,
                        (*c).ct_subtype,
                    );
                    return 111i32;
                }
                reconcat_mime(
                    raw_mime,
                    (*(*(*c).ct_type).tp_data.tp_discrete_type).dt_extension,
                    (*c).ct_subtype,
                );
                return 110i32;
            }
        },
        2 => {
            if (*(*(*c).ct_type).tp_data.tp_composite_type).ct_type
                == MAILMIME_COMPOSITE_TYPE_MULTIPART as libc::c_int
            {
                if strcmp(
                    (*c).ct_subtype,
                    b"alternative\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return 10i32;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"related\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return 20i32;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"encrypted\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return 40i32;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"signed\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return 46i32;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"mixed\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return 30i32;
                } else if strcmp(
                    (*c).ct_subtype,
                    b"report\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    return 45i32;
                } else {
                    return 50i32;
                }
            } else {
                if (*(*(*c).ct_type).tp_data.tp_composite_type).ct_type
                    == MAILMIME_COMPOSITE_TYPE_MESSAGE as libc::c_int
                {
                    return 0i32;
                }
            }
        }
        _ => {}
    }
    return 0i32;
}
unsafe extern "C" fn reconcat_mime(
    mut raw_mime: *mut *mut libc::c_char,
    mut type_0: *const libc::c_char,
    mut subtype: *const libc::c_char,
) {
    if !raw_mime.is_null() {
        *raw_mime = dc_mprintf(
            b"%s/%s\x00" as *const u8 as *const libc::c_char,
            if !type_0.is_null() {
                type_0
            } else {
                b"application\x00" as *const u8 as *const libc::c_char
            },
            if !subtype.is_null() {
                subtype
            } else {
                b"octet-stream\x00" as *const u8 as *const libc::c_char
            },
        )
    };
}
unsafe extern "C" fn mailmime_is_attachment_disposition(mut mime: *mut mailmime) -> libc::c_int {
    if !(*mime).mm_mime_fields.is_null() {
        let mut cur: *mut clistiter = (*(*(*mime).mm_mime_fields).fld_list).first;
        while !cur.is_null() {
            let mut field: *mut mailmime_field = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailmime_field;
            if !field.is_null()
                && (*field).fld_type == MAILMIME_FIELD_DISPOSITION as libc::c_int
                && !(*field).fld_data.fld_disposition.is_null()
            {
                if !(*(*field).fld_data.fld_disposition).dsp_type.is_null()
                    && (*(*(*field).fld_data.fld_disposition).dsp_type).dsp_type
                        == MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int
                {
                    return 1i32;
                }
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
/* low-level-tools for working with mailmime structures directly */
#[no_mangle]
pub unsafe extern "C" fn mailmime_find_ct_parameter(
    mut mime: *mut mailmime,
    mut name: *const libc::c_char,
) -> *mut mailmime_parameter {
    if mime.is_null()
        || name.is_null()
        || (*mime).mm_content_type.is_null()
        || (*(*mime).mm_content_type).ct_parameters.is_null()
    {
        return 0 as *mut mailmime_parameter;
    }
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*(*(*mime).mm_content_type).ct_parameters).first;
    while !cur.is_null() {
        let mut param: *mut mailmime_parameter = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailmime_parameter;
        if !param.is_null() && !(*param).pa_name.is_null() {
            if strcmp((*param).pa_name, name) == 0i32 {
                return param;
            }
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return 0 as *mut mailmime_parameter;
}
unsafe extern "C" fn dc_mimeparser_add_single_part_if_known(
    mut mimeparser: *mut dc_mimeparser_t,
    mut mime: *mut mailmime,
) -> libc::c_int {
    let mut current_block: u64;
    let mut part: *mut dc_mimepart_t = 0 as *mut dc_mimepart_t;
    let mut old_part_count: libc::c_int = carray_count((*mimeparser).parts) as libc::c_int;
    let mut mime_type: libc::c_int = 0;
    let mut mime_data: *mut mailmime_data = 0 as *mut mailmime_data;
    let mut file_suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut desired_filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut msg_type: libc::c_int = 0i32;
    let mut raw_mime: *mut libc::c_char = 0 as *mut libc::c_char;
    /* mmap_string_unref()'d if set */
    let mut transfer_decoding_buffer: *mut libc::c_char = 0 as *mut libc::c_char;
    /* charconv_buffer_free()'d if set (just calls mmap_string_unref()) */
    let mut charset_buffer: *mut libc::c_char = 0 as *mut libc::c_char;
    /* must not be free()'d */
    let mut decoded_data: *const libc::c_char = 0 as *const libc::c_char;
    let mut decoded_data_bytes: size_t = 0i32 as size_t;
    let mut simplifier: *mut dc_simplify_t = 0 as *mut dc_simplify_t;
    if !(mime.is_null() || (*mime).mm_data.mm_single.is_null()) {
        mime_type = mailmime_get_mime_type(mime, &mut msg_type, &mut raw_mime);
        mime_data = (*mime).mm_data.mm_single;
        /* MAILMIME_DATA_FILE indicates, the data is in a file; AFAIK this is not used on parsing */
        if !((*mime_data).dt_type != MAILMIME_DATA_TEXT as libc::c_int
            || (*mime_data).dt_data.dt_text.dt_data.is_null()
            || (*mime_data).dt_data.dt_text.dt_length <= 0i32 as libc::c_ulong)
        {
            /* regard `Content-Transfer-Encoding:` */
            if !(0
                == mailmime_transfer_decode(
                    mime,
                    &mut decoded_data,
                    &mut decoded_data_bytes,
                    &mut transfer_decoding_buffer,
                ))
            {
                /* no always error - but no data */
                match mime_type {
                    60 | 70 => {
                        if simplifier.is_null() {
                            simplifier = dc_simplify_new();
                            if simplifier.is_null() {
                                current_block = 8795901732489102124;
                            } else {
                                current_block = 13797916685926291137;
                            }
                        } else {
                            current_block = 13797916685926291137;
                        }
                        match current_block {
                            8795901732489102124 => {}
                            _ => {
                                /* get from `Content-Type: text/...; charset=utf-8`; must not be free()'d */
                                let mut charset: *const libc::c_char =
                                    mailmime_content_charset_get((*mime).mm_content_type);
                                if !charset.is_null()
                                    && strcmp(
                                        charset,
                                        b"utf-8\x00" as *const u8 as *const libc::c_char,
                                    ) != 0i32
                                    && strcmp(
                                        charset,
                                        b"UTF-8\x00" as *const u8 as *const libc::c_char,
                                    ) != 0i32
                                {
                                    let mut ret_bytes: size_t = 0i32 as size_t;
                                    let mut r: libc::c_int = charconv_buffer(
                                        b"utf-8\x00" as *const u8 as *const libc::c_char,
                                        charset,
                                        decoded_data,
                                        decoded_data_bytes,
                                        &mut charset_buffer,
                                        &mut ret_bytes,
                                    );
                                    if r != MAIL_CHARCONV_NO_ERROR as libc::c_int {
                                        dc_log_warning((*mimeparser).context,
                                                       0i32,
                                                       b"Cannot convert %i bytes from \"%s\" to \"utf-8\"; errorcode is %i.\x00"
                                                           as *const u8 as
                                                           *const libc::c_char,
                                                       decoded_data_bytes as
                                                           libc::c_int,
                                                       charset,
                                                       r as libc::c_int);
                                        current_block = 17788412896529399552;
                                    } else if charset_buffer.is_null()
                                        || ret_bytes <= 0i32 as libc::c_ulong
                                    {
                                        /* no error - but nothing to add */
                                        current_block = 8795901732489102124;
                                    } else {
                                        decoded_data = charset_buffer;
                                        decoded_data_bytes = ret_bytes;
                                        current_block = 17788412896529399552;
                                    }
                                } else {
                                    current_block = 17788412896529399552;
                                }
                                match current_block {
                                    8795901732489102124 => {}
                                    _ => {
                                        /* check header directly as is_send_by_messenger is not yet set up */
                                        let mut is_msgrmsg: libc::c_int =
                                            (dc_mimeparser_lookup_optional_field(
                                                mimeparser,
                                                b"Chat-Version\x00" as *const u8
                                                    as *const libc::c_char,
                                            ) != 0 as *mut libc::c_void
                                                as *mut mailimf_optional_field)
                                                as libc::c_int;
                                        let mut simplified_txt: *mut libc::c_char =
                                            dc_simplify_simplify(
                                                simplifier,
                                                decoded_data,
                                                decoded_data_bytes as libc::c_int,
                                                if mime_type == 70i32 { 1i32 } else { 0i32 },
                                                is_msgrmsg,
                                            );
                                        if !simplified_txt.is_null()
                                            && 0 != *simplified_txt.offset(0isize) as libc::c_int
                                        {
                                            part = dc_mimepart_new();
                                            (*part).type_0 = 10i32;
                                            (*part).int_mimetype = mime_type;
                                            (*part).msg = simplified_txt;
                                            (*part).msg_raw =
                                                strndup(decoded_data, decoded_data_bytes);
                                            do_add_single_part(mimeparser, part);
                                            part = 0 as *mut dc_mimepart_t
                                        } else {
                                            free(simplified_txt as *mut libc::c_void);
                                        }
                                        if 0 != (*simplifier).is_forwarded {
                                            (*mimeparser).is_forwarded = 1i32
                                        }
                                        current_block = 10261677128829721533;
                                    }
                                }
                            }
                        }
                    }
                    80 | 90 | 100 | 110 | 111 => {
                        /* try to get file name from
                           `Content-Disposition: ... filename*=...`
                        or `Content-Disposition: ... filename*0*=... filename*1*=... filename*2*=...`
                        or `Content-Disposition: ... filename=...` */
                        let mut filename_parts: dc_strbuilder_t = _dc_strbuilder {
                            buf: 0 as *mut libc::c_char,
                            allocated: 0,
                            free: 0,
                            eos: 0 as *mut libc::c_char,
                        };
                        dc_strbuilder_init(&mut filename_parts, 0i32);
                        let mut cur1: *mut clistiter = (*(*(*mime).mm_mime_fields).fld_list).first;
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
                                    let mut cur2: *mut clistiter =
                                        (*(*file_disposition).dsp_parms).first;
                                    while !cur2.is_null() {
                                        let mut dsp_param: *mut mailmime_disposition_parm =
                                            (if !cur2.is_null() {
                                                (*cur2).data
                                            } else {
                                                0 as *mut libc::c_void
                                            })
                                                as *mut mailmime_disposition_parm;
                                        if !dsp_param.is_null() {
                                            if (*dsp_param).pa_type
                                                == MAILMIME_DISPOSITION_PARM_PARAMETER
                                                    as libc::c_int
                                                && !(*dsp_param).pa_data.pa_parameter.is_null()
                                                && !(*(*dsp_param).pa_data.pa_parameter)
                                                    .pa_name
                                                    .is_null()
                                                && strncmp(
                                                    (*(*dsp_param).pa_data.pa_parameter).pa_name,
                                                    b"filename*\x00" as *const u8
                                                        as *const libc::c_char,
                                                    9i32 as libc::c_ulong,
                                                ) == 0i32
                                            {
                                                dc_strbuilder_cat(
                                                    &mut filename_parts,
                                                    (*(*dsp_param).pa_data.pa_parameter).pa_value,
                                                );
                                            } else if (*dsp_param).pa_type
                                                == MAILMIME_DISPOSITION_PARM_FILENAME as libc::c_int
                                            {
                                                desired_filename = dc_decode_header_words(
                                                    (*dsp_param).pa_data.pa_filename,
                                                )
                                            }
                                        }
                                        cur2 = if !cur2.is_null() {
                                            (*cur2).next
                                        } else {
                                            0 as *mut clistcell_s
                                        }
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
                        if 0 != strlen(filename_parts.buf) {
                            free(desired_filename as *mut libc::c_void);
                            desired_filename = dc_decode_ext_header(filename_parts.buf)
                        }
                        free(filename_parts.buf as *mut libc::c_void);
                        if desired_filename.is_null() {
                            let mut param: *mut mailmime_parameter = mailmime_find_ct_parameter(
                                mime,
                                b"name\x00" as *const u8 as *const libc::c_char,
                            );
                            if !param.is_null()
                                && !(*param).pa_value.is_null()
                                && 0 != *(*param).pa_value.offset(0isize) as libc::c_int
                            {
                                desired_filename = dc_strdup((*param).pa_value)
                            }
                        }
                        /* if there is still no filename, guess one */
                        if desired_filename.is_null() {
                            if !(*mime).mm_content_type.is_null()
                                && !(*(*mime).mm_content_type).ct_subtype.is_null()
                            {
                                desired_filename = dc_mprintf(
                                    b"file.%s\x00" as *const u8 as *const libc::c_char,
                                    (*(*mime).mm_content_type).ct_subtype,
                                );
                                current_block = 17019156190352891614;
                            } else {
                                current_block = 8795901732489102124;
                            }
                        } else {
                            current_block = 17019156190352891614;
                        }
                        match current_block {
                            8795901732489102124 => {}
                            _ => {
                                if strncmp(
                                    desired_filename,
                                    b"location\x00" as *const u8 as *const libc::c_char,
                                    8i32 as libc::c_ulong,
                                ) == 0i32
                                    && strncmp(
                                        desired_filename
                                            .offset(strlen(desired_filename) as isize)
                                            .offset(-4isize),
                                        b".kml\x00" as *const u8 as *const libc::c_char,
                                        4i32 as libc::c_ulong,
                                    ) == 0i32
                                {
                                    (*mimeparser).kml = dc_kml_parse(
                                        (*mimeparser).context,
                                        decoded_data,
                                        decoded_data_bytes,
                                    );
                                    current_block = 8795901732489102124;
                                } else {
                                    dc_replace_bad_utf8_chars(desired_filename);
                                    do_add_single_file_part(
                                        mimeparser,
                                        msg_type,
                                        mime_type,
                                        raw_mime,
                                        decoded_data,
                                        decoded_data_bytes,
                                        desired_filename,
                                    );
                                    current_block = 10261677128829721533;
                                }
                            }
                        }
                    }
                    _ => {
                        current_block = 10261677128829721533;
                    }
                }
                match current_block {
                    8795901732489102124 => {}
                    _ => {}
                }
            }
        }
    }
    /* add object? (we do not add all objetcs, eg. signatures etc. are ignored) */
    dc_simplify_unref(simplifier);
    if !charset_buffer.is_null() {
        charconv_buffer_free(charset_buffer);
    }
    if !transfer_decoding_buffer.is_null() {
        mmap_string_unref(transfer_decoding_buffer);
    }
    free(file_suffix as *mut libc::c_void);
    free(desired_filename as *mut libc::c_void);
    dc_mimepart_unref(part);
    free(raw_mime as *mut libc::c_void);
    return if carray_count((*mimeparser).parts) > old_part_count as libc::c_uint {
        1i32
    } else {
        0i32
    };
}
unsafe extern "C" fn do_add_single_file_part(
    mut parser: *mut dc_mimeparser_t,
    mut msg_type: libc::c_int,
    mut mime_type: libc::c_int,
    mut raw_mime: *const libc::c_char,
    mut decoded_data: *const libc::c_char,
    mut decoded_data_bytes: size_t,
    mut desired_filename: *const libc::c_char,
) {
    let mut part: *mut dc_mimepart_t = 0 as *mut dc_mimepart_t;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    /* create a free file name to use */
    pathNfilename = dc_get_fine_pathNfilename(
        (*parser).context,
        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
        desired_filename,
    );
    if !pathNfilename.is_null() {
        /* copy data to file */
        if !(dc_write_file(
            (*parser).context,
            pathNfilename,
            decoded_data as *const libc::c_void,
            decoded_data_bytes,
        ) == 0i32)
        {
            part = dc_mimepart_new();
            (*part).type_0 = msg_type;
            (*part).int_mimetype = mime_type;
            (*part).bytes = decoded_data_bytes as libc::c_int;
            dc_param_set((*part).param, 'f' as i32, pathNfilename);
            dc_param_set((*part).param, 'm' as i32, raw_mime);
            if mime_type == 80i32 {
                let mut w: uint32_t = 0i32 as uint32_t;
                let mut h: uint32_t = 0i32 as uint32_t;
                if 0 != dc_get_filemeta(
                    decoded_data as *const libc::c_void,
                    decoded_data_bytes,
                    &mut w,
                    &mut h,
                ) {
                    dc_param_set_int((*part).param, 'w' as i32, w as int32_t);
                    dc_param_set_int((*part).param, 'h' as i32, h as int32_t);
                }
            }
            do_add_single_part(parser, part);
            part = 0 as *mut dc_mimepart_t
        }
    }
    free(pathNfilename as *mut libc::c_void);
    dc_mimepart_unref(part);
}
unsafe extern "C" fn do_add_single_part(
    mut parser: *mut dc_mimeparser_t,
    mut part: *mut dc_mimepart_t,
) {
    if 0 != (*(*parser).e2ee_helper).encrypted
        && (*(*(*parser).e2ee_helper).signatures).count > 0i32
    {
        dc_param_set_int((*part).param, 'c' as i32, 1i32);
    } else if 0 != (*(*parser).e2ee_helper).encrypted {
        dc_param_set_int((*part).param, 'e' as i32, 0x2i32);
    }
    carray_add(
        (*parser).parts,
        part as *mut libc::c_void,
        0 as *mut libc::c_uint,
    );
}
#[no_mangle]
pub unsafe extern "C" fn mailmime_transfer_decode(
    mut mime: *mut mailmime,
    mut ret_decoded_data: *mut *const libc::c_char,
    mut ret_decoded_data_bytes: *mut size_t,
    mut ret_to_mmap_string_unref: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut mime_transfer_encoding: libc::c_int = MAILMIME_MECHANISM_BINARY as libc::c_int;
    let mut mime_data: *mut mailmime_data = 0 as *mut mailmime_data;
    /* must not be free()'d */
    let mut decoded_data: *const libc::c_char = 0 as *const libc::c_char;
    let mut decoded_data_bytes: size_t = 0i32 as size_t;
    /* mmap_string_unref()'d if set */
    let mut transfer_decoding_buffer: *mut libc::c_char = 0 as *mut libc::c_char;
    if mime.is_null()
        || ret_decoded_data.is_null()
        || ret_decoded_data_bytes.is_null()
        || ret_to_mmap_string_unref.is_null()
        || !(*ret_decoded_data).is_null()
        || *ret_decoded_data_bytes != 0i32 as libc::c_ulong
        || !(*ret_to_mmap_string_unref).is_null()
    {
        return 0i32;
    }
    mime_data = (*mime).mm_data.mm_single;
    if !(*mime).mm_mime_fields.is_null() {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*(*mime).mm_mime_fields).fld_list).first;
        while !cur.is_null() {
            let mut field: *mut mailmime_field = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailmime_field;
            if !field.is_null()
                && (*field).fld_type == MAILMIME_FIELD_TRANSFER_ENCODING as libc::c_int
                && !(*field).fld_data.fld_encoding.is_null()
            {
                mime_transfer_encoding = (*(*field).fld_data.fld_encoding).enc_type;
                break;
            } else {
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell_s
                }
            }
        }
    }
    if mime_transfer_encoding == MAILMIME_MECHANISM_7BIT as libc::c_int
        || mime_transfer_encoding == MAILMIME_MECHANISM_8BIT as libc::c_int
        || mime_transfer_encoding == MAILMIME_MECHANISM_BINARY as libc::c_int
    {
        decoded_data = (*mime_data).dt_data.dt_text.dt_data;
        decoded_data_bytes = (*mime_data).dt_data.dt_text.dt_length;
        if decoded_data.is_null() || decoded_data_bytes <= 0i32 as libc::c_ulong {
            return 0i32;
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
            return 0i32;
        }
        decoded_data = transfer_decoding_buffer
    }
    *ret_decoded_data = decoded_data;
    *ret_decoded_data_bytes = decoded_data_bytes;
    *ret_to_mmap_string_unref = transfer_decoding_buffer;
    return 1i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_is_mailinglist_message(
    mut mimeparser: *mut dc_mimeparser_t,
) -> libc::c_int {
    if mimeparser.is_null() {
        return 0i32;
    }
    if !dc_mimeparser_lookup_field(
        mimeparser,
        b"List-Id\x00" as *const u8 as *const libc::c_char,
    )
    .is_null()
    {
        return 1i32;
    }
    let mut precedence: *mut mailimf_optional_field = dc_mimeparser_lookup_optional_field(
        mimeparser,
        b"Precedence\x00" as *const u8 as *const libc::c_char,
    );
    if !precedence.is_null() {
        if strcasecmp(
            (*precedence).fld_value,
            b"list\x00" as *const u8 as *const libc::c_char,
        ) == 0i32
            || strcasecmp(
                (*precedence).fld_value,
                b"bulk\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
        {
            return 1i32;
        }
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_sender_equals_recipient(
    mut mimeparser: *mut dc_mimeparser_t,
) -> libc::c_int {
    let mut sender_equals_recipient: libc::c_int = 0i32;
    let mut fld: *const mailimf_field = 0 as *const mailimf_field;
    let mut fld_from: *const mailimf_from = 0 as *const mailimf_from;
    let mut mb: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
    let mut from_addr_norm: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut recipients: *mut dc_hash_t = 0 as *mut dc_hash_t;
    if !(mimeparser.is_null() || (*mimeparser).header_root.is_null()) {
        /* get From: and check there is exactly one sender */
        fld = mailimf_find_field((*mimeparser).header_root, MAILIMF_FIELD_FROM as libc::c_int);
        if !(fld.is_null()
            || {
                fld_from = (*fld).fld_data.fld_from;
                fld_from.is_null()
            }
            || (*fld_from).frm_mb_list.is_null()
            || (*(*fld_from).frm_mb_list).mb_list.is_null()
            || (*(*(*fld_from).frm_mb_list).mb_list).count != 1i32)
        {
            mb = (if !(*(*(*fld_from).frm_mb_list).mb_list).first.is_null() {
                (*(*(*(*fld_from).frm_mb_list).mb_list).first).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailimf_mailbox;
            if !mb.is_null() {
                from_addr_norm = dc_addr_normalize((*mb).mb_addr_spec);
                recipients = mailimf_get_recipients((*mimeparser).header_root);
                if !((*recipients).count != 1i32) {
                    if !dc_hash_find(
                        recipients,
                        from_addr_norm as *const libc::c_void,
                        strlen(from_addr_norm) as libc::c_int,
                    )
                    .is_null()
                    {
                        sender_equals_recipient = 1i32
                    }
                }
            }
        }
    }
    dc_hash_clear(recipients);
    free(recipients as *mut libc::c_void);
    free(from_addr_norm as *mut libc::c_void);
    return sender_equals_recipient;
}
#[no_mangle]
pub unsafe extern "C" fn mailimf_get_recipients(
    mut imffields: *mut mailimf_fields,
) -> *mut dc_hash_t {
    /* the returned value must be dc_hash_clear()'d and free()'d. returned addresses are normalized. */
    let mut recipients: *mut dc_hash_t =
        malloc(::std::mem::size_of::<dc_hash_t>() as libc::c_ulong) as *mut dc_hash_t;
    dc_hash_init(recipients, 3i32, 1i32);
    let mut cur1: *mut clistiter = 0 as *mut clistiter;
    cur1 = (*(*imffields).fld_list).first;
    while !cur1.is_null() {
        let mut fld: *mut mailimf_field = (if !cur1.is_null() {
            (*cur1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        let mut fld_to: *mut mailimf_to = 0 as *mut mailimf_to;
        let mut fld_cc: *mut mailimf_cc = 0 as *mut mailimf_cc;
        let mut addr_list: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
        match (*fld).fld_type {
            13 => {
                fld_to = (*fld).fld_data.fld_to;
                if !fld_to.is_null() {
                    addr_list = (*fld_to).to_addr_list
                }
            }
            14 => {
                fld_cc = (*fld).fld_data.fld_cc;
                if !fld_cc.is_null() {
                    addr_list = (*fld_cc).cc_addr_list
                }
            }
            _ => {}
        }
        if !addr_list.is_null() {
            let mut cur2: *mut clistiter = 0 as *mut clistiter;
            cur2 = (*(*addr_list).ad_list).first;
            while !cur2.is_null() {
                let mut adr: *mut mailimf_address = (if !cur2.is_null() {
                    (*cur2).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailimf_address;
                if !adr.is_null() {
                    if (*adr).ad_type == MAILIMF_ADDRESS_MAILBOX as libc::c_int {
                        mailimf_get_recipients__add_addr(recipients, (*adr).ad_data.ad_mailbox);
                    } else if (*adr).ad_type == MAILIMF_ADDRESS_GROUP as libc::c_int {
                        let mut group: *mut mailimf_group = (*adr).ad_data.ad_group;
                        if !group.is_null() && !(*group).grp_mb_list.is_null() {
                            let mut cur3: *mut clistiter = 0 as *mut clistiter;
                            cur3 = (*(*(*group).grp_mb_list).mb_list).first;
                            while !cur3.is_null() {
                                mailimf_get_recipients__add_addr(
                                    recipients,
                                    (if !cur3.is_null() {
                                        (*cur3).data
                                    } else {
                                        0 as *mut libc::c_void
                                    }) as *mut mailimf_mailbox,
                                );
                                cur3 = if !cur3.is_null() {
                                    (*cur3).next
                                } else {
                                    0 as *mut clistcell_s
                                }
                            }
                        }
                    }
                }
                cur2 = if !cur2.is_null() {
                    (*cur2).next
                } else {
                    0 as *mut clistcell_s
                }
            }
        }
        cur1 = if !cur1.is_null() {
            (*cur1).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return recipients;
}
/* ******************************************************************************
 * debug output
 ******************************************************************************/
/* DEBUG_MIME_OUTPUT */
/* ******************************************************************************
 * low-level-tools for getting a list of all recipients
 ******************************************************************************/
unsafe extern "C" fn mailimf_get_recipients__add_addr(
    mut recipients: *mut dc_hash_t,
    mut mb: *mut mailimf_mailbox,
) {
    if !mb.is_null() {
        let mut addr_norm: *mut libc::c_char = dc_addr_normalize((*mb).mb_addr_spec);
        dc_hash_insert(
            recipients,
            addr_norm as *const libc::c_void,
            strlen(addr_norm) as libc::c_int,
            1i32 as *mut libc::c_void,
        );
        free(addr_norm as *mut libc::c_void);
    };
}
/*the result is a pointer to mime, must not be freed*/
#[no_mangle]
pub unsafe extern "C" fn mailimf_find_field(
    mut header: *mut mailimf_fields,
    mut wanted_fld_type: libc::c_int,
) -> *mut mailimf_field {
    if header.is_null() || (*header).fld_list.is_null() {
        return 0 as *mut mailimf_field;
    }
    let mut cur1: *mut clistiter = (*(*header).fld_list).first;
    while !cur1.is_null() {
        let mut field: *mut mailimf_field = (if !cur1.is_null() {
            (*cur1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        if !field.is_null() {
            if (*field).fld_type == wanted_fld_type {
                return field;
            }
        }
        cur1 = if !cur1.is_null() {
            (*cur1).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return 0 as *mut mailimf_field;
}
#[no_mangle]
pub unsafe extern "C" fn dc_mimeparser_repl_msg_by_error(
    mut mimeparser: *mut dc_mimeparser_t,
    mut error_msg: *const libc::c_char,
) {
    let mut part: *mut dc_mimepart_t = 0 as *mut dc_mimepart_t;
    let mut i: libc::c_int = 0i32;
    if mimeparser.is_null()
        || (*mimeparser).parts.is_null()
        || carray_count((*mimeparser).parts) <= 0i32 as libc::c_uint
    {
        return;
    }
    part = carray_get((*mimeparser).parts, 0i32 as libc::c_uint) as *mut dc_mimepart_t;
    (*part).type_0 = 10i32;
    free((*part).msg as *mut libc::c_void);
    (*part).msg = dc_mprintf(b"[%s]\x00" as *const u8 as *const libc::c_char, error_msg);
    i = 1i32;
    while (i as libc::c_uint) < carray_count((*mimeparser).parts) {
        part = carray_get((*mimeparser).parts, i as libc::c_uint) as *mut dc_mimepart_t;
        if !part.is_null() {
            dc_mimepart_unref(part);
        }
        i += 1
    }
    carray_set_size((*mimeparser).parts, 1i32 as libc::c_uint);
}
/*the result is a pointer to mime, must not be freed*/
#[no_mangle]
pub unsafe extern "C" fn mailmime_find_mailimf_fields(
    mut mime: *mut mailmime,
) -> *mut mailimf_fields {
    if mime.is_null() {
        return 0 as *mut mailimf_fields;
    }
    match (*mime).mm_type {
        2 => {
            let mut cur: *mut clistiter = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
            while !cur.is_null() {
                let mut header: *mut mailimf_fields = mailmime_find_mailimf_fields(
                    (if !cur.is_null() {
                        (*cur).data
                    } else {
                        0 as *mut libc::c_void
                    }) as *mut mailmime,
                );
                if !header.is_null() {
                    return header;
                }
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell_s
                }
            }
        }
        3 => return (*mime).mm_data.mm_message.mm_fields,
        _ => {}
    }
    return 0 as *mut mailimf_fields;
}
#[no_mangle]
pub unsafe extern "C" fn mailimf_find_optional_field(
    mut header: *mut mailimf_fields,
    mut wanted_fld_name: *const libc::c_char,
) -> *mut mailimf_optional_field {
    if header.is_null() || (*header).fld_list.is_null() {
        return 0 as *mut mailimf_optional_field;
    }
    let mut cur1: *mut clistiter = (*(*header).fld_list).first;
    while !cur1.is_null() {
        let mut field: *mut mailimf_field = (if !cur1.is_null() {
            (*cur1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            let mut optional_field: *mut mailimf_optional_field =
                (*field).fld_data.fld_optional_field;
            if !optional_field.is_null()
                && !(*optional_field).fld_name.is_null()
                && !(*optional_field).fld_value.is_null()
                && strcasecmp((*optional_field).fld_name, wanted_fld_name) == 0i32
            {
                return optional_field;
            }
        }
        cur1 = if !cur1.is_null() {
            (*cur1).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return 0 as *mut mailimf_optional_field;
}

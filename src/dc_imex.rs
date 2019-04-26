use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type _telldir;
    pub type mailstream_cancel;
    pub type sqlite3;
    pub type sqlite3_stmt;
    #[no_mangle]
    fn closedir(_: *mut DIR) -> libc::c_int;
    #[no_mangle]
    fn opendir(_: *const libc::c_char) -> *mut DIR;
    #[no_mangle]
    fn readdir(_: *mut DIR) -> *mut dirent;
    #[no_mangle]
    fn sleep(_: libc::c_uint) -> libc::c_uint;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn RAND_bytes(buf: *mut libc::c_uchar, num: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn RAND_pseudo_bytes(buf: *mut libc::c_uchar, num: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn mmap_string_unref(str: *mut libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strncpy(_: *mut libc::c_char, _: *const libc::c_char, _: libc::c_ulong)
        -> *mut libc::c_char;
    #[no_mangle]
    fn strstr(_: *const libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strndup(_: *const libc::c_char, _: libc::c_ulong) -> *mut libc::c_char;
    #[no_mangle]
    fn localtime(_: *const time_t) -> *mut tm;
    #[no_mangle]
    fn strftime(_: *mut libc::c_char, _: size_t, _: *const libc::c_char, _: *const tm) -> size_t;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn mailmime_base64_body_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut libc::c_char,
        result_len: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_is_configured(_: *const dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_create_chat_by_contact_id(_: *mut dc_context_t, contact_id: uint32_t) -> uint32_t;
    #[no_mangle]
    fn dc_send_msg(_: *mut dc_context_t, chat_id: uint32_t, _: *mut dc_msg_t) -> uint32_t;
    #[no_mangle]
    fn dc_get_msg(_: *mut dc_context_t, msg_id: uint32_t) -> *mut dc_msg_t;
    /* library-private */
    #[no_mangle]
    fn dc_param_new() -> *mut dc_param_t;
    #[no_mangle]
    fn dc_param_unref(_: *mut dc_param_t);
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
    fn dc_param_set(_: *mut dc_param_t, key: libc::c_int, value: *const libc::c_char);
    #[no_mangle]
    fn dc_param_set_int(_: *mut dc_param_t, key: libc::c_int, value: int32_t);
    #[no_mangle]
    fn dc_sqlite3_unref(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_get_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        def: int32_t,
    ) -> int32_t;
    #[no_mangle]
    fn dc_sqlite3_open(
        _: *mut dc_sqlite3_t,
        dbfile: *const libc::c_char,
        flags: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_new(_: *mut dc_context_t) -> *mut dc_sqlite3_t;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_loginparam_new() -> *mut dc_loginparam_t;
    #[no_mangle]
    fn dc_loginparam_unref(_: *mut dc_loginparam_t);
    #[no_mangle]
    fn dc_loginparam_read(
        _: *mut dc_loginparam_t,
        _: *mut dc_sqlite3_t,
        prefix: *const libc::c_char,
    );
    #[no_mangle]
    fn dc_free_ongoing(_: *mut dc_context_t);
    #[no_mangle]
    fn dc_msg_unref(_: *mut dc_msg_t);
    #[no_mangle]
    fn dc_msg_is_sent(_: *const dc_msg_t) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_new_untyped(_: *mut dc_context_t) -> *mut dc_msg_t;
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
    fn dc_key_new() -> *mut dc_key_t;
    #[no_mangle]
    fn dc_key_unref(_: *mut dc_key_t);
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    /* Return the string with the given ID by calling DC_EVENT_GET_STRING.
    The result must be free()'d! */
    #[no_mangle]
    fn dc_stock_str(_: *mut dc_context_t, id: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_str_replace(
        haystack: *mut *mut libc::c_char,
        needle: *const libc::c_char,
        replacement: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_key_render_asc(
        _: *const dc_key_t,
        add_header_lines: *const libc::c_char,
    ) -> *mut libc::c_char;
    /* symm. encryption */
    #[no_mangle]
    fn dc_pgp_symm_encrypt(
        context: *mut dc_context_t,
        passphrase: *const libc::c_char,
        plain: *const libc::c_void,
        plain_bytes: size_t,
        ret_ctext_armored: *mut *mut libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_key_load_self_private(
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
    #[no_mangle]
    fn dc_ensure_secret_key_exists(_: *mut dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_strbuilder_catf(_: *mut dc_strbuilder_t, format: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_alloc_ongoing(_: *mut dc_context_t) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_set_config_int(
        _: *mut dc_sqlite3_t,
        key: *const libc::c_char,
        value: int32_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_key_save_self_keypair(
        public_key: *const dc_key_t,
        private_key: *const dc_key_t,
        addr: *const libc::c_char,
        is_default: libc::c_int,
        sql: *mut dc_sqlite3_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_execute(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_blob(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_void,
        n: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    /* tools, these functions are compatible to the corresponding sqlite3_* functions */
    #[no_mangle]
    fn dc_sqlite3_prepare(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> *mut sqlite3_stmt;
    #[no_mangle]
    fn dc_pgp_split_key(
        _: *mut dc_context_t,
        private_in: *const dc_key_t,
        public_out: *mut dc_key_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_pgp_is_valid_key(_: *mut dc_context_t, _: *const dc_key_t) -> libc::c_int;
    #[no_mangle]
    fn dc_key_set_from_base64(
        _: *mut dc_key_t,
        base64: *const libc::c_char,
        type_0: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_split_armored_data(
        buf: *mut libc::c_char,
        ret_headerline: *mut *const libc::c_char,
        ret_setupcodebegin: *mut *const libc::c_char,
        ret_preferencrypt: *mut *const libc::c_char,
        ret_base64: *mut *const libc::c_char,
    ) -> libc::c_int;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_pgp_symm_decrypt(
        context: *mut dc_context_t,
        passphrase: *const libc::c_char,
        ctext: *const libc::c_void,
        ctext_bytes: size_t,
        ret_plain_text: *mut *mut libc::c_void,
        ret_plain_bytes: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_read_file(
        _: *mut dc_context_t,
        pathNfilename: *const libc::c_char,
        buf: *mut *mut libc::c_void,
        buf_bytes: *mut size_t,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_msg_get_file(_: *const dc_msg_t) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_msg_is_setupmessage(_: *const dc_msg_t) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_bind_text(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_char,
        _: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_blob(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_void;
    #[no_mangle]
    fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    #[no_mangle]
    fn sqlite3_column_bytes(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_reset(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_close(_: *mut dc_sqlite3_t);
    #[no_mangle]
    fn dc_sqlite3_is_open(_: *const dc_sqlite3_t) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_try_execute(_: *mut dc_sqlite3_t, sql: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_sqlite3_table_exists(_: *mut dc_sqlite3_t, name: *const libc::c_char) -> libc::c_int;
    /* housekeeping */
    #[no_mangle]
    fn dc_housekeeping(_: *mut dc_context_t);
    #[no_mangle]
    fn dc_get_filesuffix_lc(pathNfilename: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_file_exist(_: *mut dc_context_t, pathNfilename: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_delete_file(_: *mut dc_context_t, pathNFilename: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_copy_file(
        _: *mut dc_context_t,
        pathNFilename: *const libc::c_char,
        dest_pathNFilename: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_create_folder(_: *mut dc_context_t, pathNfilename: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_param_get(
        _: *const dc_param_t,
        key: libc::c_int,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_get_int(_: *const dc_param_t, key: libc::c_int, def: int32_t) -> int32_t;
    #[no_mangle]
    fn dc_key_set_from_stmt(
        _: *mut dc_key_t,
        _: *mut sqlite3_stmt,
        index: libc::c_int,
        type_0: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_key_render_asc_to_file(
        _: *const dc_key_t,
        file: *const libc::c_char,
        _: *mut dc_context_t,
    ) -> libc::c_int;
}
pub type __uint8_t = libc::c_uchar;
pub type __uint16_t = libc::c_ushort;
pub type __uint64_t = libc::c_ulonglong;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dirent {
    pub d_ino: __uint64_t,
    pub d_seekoff: __uint64_t,
    pub d_reclen: __uint16_t,
    pub d_namlen: __uint16_t,
    pub d_type: __uint8_t,
    pub d_name: [libc::c_char; 1024],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DIR {
    pub __dd_fd: libc::c_int,
    pub __dd_loc: libc::c_long,
    pub __dd_size: libc::c_long,
    pub __dd_buf: *mut libc::c_char,
    pub __dd_len: libc::c_int,
    pub __dd_seek: libc::c_long,
    pub __padding: libc::c_long,
    pub __dd_flags: libc::c_int,
    pub __dd_lock: __darwin_pthread_mutex_t,
    pub __dd_td: *mut _telldir,
}
pub type size_t = __darwin_size_t;
pub type uint32_t = libc::c_uint;
pub type int32_t = libc::c_int;
pub type uintptr_t = libc::c_ulong;
pub type ssize_t = __darwin_ssize_t;
pub type time_t = __darwin_time_t;
pub type uint8_t = libc::c_uchar;
pub type uint16_t = libc::c_ushort;
pub type pthread_cond_t = __darwin_pthread_cond_t;
pub type pthread_mutex_t = __darwin_pthread_mutex_t;
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
pub type unnamed = libc::c_uint;
pub const MAILIMF_ERROR_FILE: unnamed = 4;
pub const MAILIMF_ERROR_INVAL: unnamed = 3;
pub const MAILIMF_ERROR_MEMORY: unnamed = 2;
pub const MAILIMF_ERROR_PARSE: unnamed = 1;
pub const MAILIMF_NO_ERROR: unnamed = 0;
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
    pub smtp_sasl: unnamed_0,
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
pub struct unnamed_0 {
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
    pub sec_data: unnamed_1,
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
pub union unnamed_1 {
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
    pub ft_data: unnamed_2,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union unnamed_2 {
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
    pub imap_sasl: unnamed_3,
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
pub struct unnamed_3 {
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
pub type dc_loginparam_t = _dc_loginparam;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_loginparam {
    pub addr: *mut libc::c_char,
    pub mail_server: *mut libc::c_char,
    pub mail_user: *mut libc::c_char,
    pub mail_pw: *mut libc::c_char,
    pub mail_port: uint16_t,
    pub send_server: *mut libc::c_char,
    pub send_user: *mut libc::c_char,
    pub send_pw: *mut libc::c_char,
    pub send_port: libc::c_int,
    pub server_flags: libc::c_int,
}
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
pub type dc_strbuilder_t = _dc_strbuilder;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
/* *
 * Library-internal.
 */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_job {
    pub job_id: uint32_t,
    pub action: libc::c_int,
    pub foreign_id: uint32_t,
    pub desired_timestamp: time_t,
    pub added_timestamp: time_t,
    pub tries: libc::c_int,
    pub param: *mut dc_param_t,
    pub try_again: libc::c_int,
    pub pending_error: *mut libc::c_char,
}
// thread IDs
// jobs in the INBOX-thread, range from DC_IMAP_THREAD..DC_IMAP_THREAD+999
// low priority ...
// ... high priority
// jobs in the SMTP-thread, range from DC_SMTP_THREAD..DC_SMTP_THREAD+999
// low priority ...
// ... high priority
// timeouts until actions are aborted.
// this may also affects IDLE to return, so a re-connect may take this time.
// mailcore2 uses 30 seconds, k-9 uses 10 seconds
pub type dc_job_t = _dc_job;
// import/export and tools
// param1 is a directory where the keys are written to
// param1 is a directory where the keys are searched in and read from
// param1 is a directory where the backup is written to
// param1 is the file with the backup to import
#[no_mangle]
pub unsafe extern "C" fn dc_imex(
    mut context: *mut dc_context_t,
    mut what: libc::c_int,
    mut param1: *const libc::c_char,
    mut param2: *const libc::c_char,
) {
    let mut param: *mut dc_param_t = dc_param_new();
    dc_param_set_int(param, 'S' as i32, what);
    dc_param_set(param, 'E' as i32, param1);
    dc_param_set(param, 'F' as i32, param2);
    dc_job_kill_action(context, 910i32);
    dc_job_add(context, 910i32, 0i32, (*param).packed, 0i32);
    dc_param_unref(param);
}
#[no_mangle]
pub unsafe extern "C" fn dc_imex_has_backup(
    mut context: *mut dc_context_t,
    mut dir_name: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret_backup_time: time_t = 0i32 as time_t;
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut prefix_len: libc::c_int =
        strlen(b"delta-chat\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut suffix_len: libc::c_int =
        strlen(b"bak\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut curr_pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut test_sql: *mut dc_sqlite3_t = 0 as *mut dc_sqlite3_t;
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0 as *mut libc::c_char;
    }
    dir_handle = opendir(dir_name);
    if dir_handle.is_null() {
        dc_log_info(
            context,
            0i32,
            b"Backup check: Cannot open directory \"%s\".\x00" as *const u8 as *const libc::c_char,
            dir_name,
        );
    } else {
        loop {
            dir_entry = readdir(dir_handle);
            if dir_entry.is_null() {
                break;
            }
            let mut name: *const libc::c_char = (*dir_entry).d_name.as_mut_ptr();
            let mut name_len: libc::c_int = strlen(name) as libc::c_int;
            if name_len > prefix_len
                && strncmp(
                    name,
                    b"delta-chat\x00" as *const u8 as *const libc::c_char,
                    prefix_len as libc::c_ulong,
                ) == 0i32
                && name_len > suffix_len
                && strncmp(
                    &*name.offset((name_len - suffix_len - 1i32) as isize),
                    b".bak\x00" as *const u8 as *const libc::c_char,
                    suffix_len as libc::c_ulong,
                ) == 0i32
            {
                free(curr_pathNfilename as *mut libc::c_void);
                curr_pathNfilename = dc_mprintf(
                    b"%s/%s\x00" as *const u8 as *const libc::c_char,
                    dir_name,
                    name,
                );
                dc_sqlite3_unref(test_sql);
                test_sql = dc_sqlite3_new(context);
                if !test_sql.is_null() && 0 != dc_sqlite3_open(test_sql, curr_pathNfilename, 0x1i32)
                {
                    let mut curr_backup_time: time_t = dc_sqlite3_get_config_int(
                        test_sql,
                        b"backup_time\x00" as *const u8 as *const libc::c_char,
                        0i32,
                    ) as time_t;
                    if curr_backup_time > 0i32 as libc::c_long && curr_backup_time > ret_backup_time
                    {
                        free(ret as *mut libc::c_void);
                        ret = curr_pathNfilename;
                        ret_backup_time = curr_backup_time;
                        curr_pathNfilename = 0 as *mut libc::c_char
                    }
                }
            }
        }
    }
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    free(curr_pathNfilename as *mut libc::c_void);
    dc_sqlite3_unref(test_sql);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_check_password(
    mut context: *mut dc_context_t,
    mut test_pw: *const libc::c_char,
) -> libc::c_int {
    /* Check if the given password matches the configured mail_pw.
    This is to prompt the user before starting eg. an export; this is mainly to avoid doing people bad thinkgs if they have short access to the device.
    When we start supporting OAuth some day, we should think this over, maybe force the user to re-authenticate himself with the Android password. */
    let mut loginparam: *mut dc_loginparam_t = dc_loginparam_new();
    let mut success: libc::c_int = 0i32;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        dc_loginparam_read(
            loginparam,
            (*context).sql,
            b"configured_\x00" as *const u8 as *const libc::c_char,
        );
        if ((*loginparam).mail_pw.is_null()
            || *(*loginparam).mail_pw.offset(0isize) as libc::c_int == 0i32)
            && (test_pw.is_null() || *test_pw.offset(0isize) as libc::c_int == 0i32)
        {
            success = 1i32
        } else if (*loginparam).mail_pw.is_null() || test_pw.is_null() {
            success = 0i32
        } else if strcmp((*loginparam).mail_pw, test_pw) == 0i32 {
            success = 1i32
        }
    }
    dc_loginparam_unref(loginparam);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_initiate_key_transfer(
    mut context: *mut dc_context_t,
) -> *mut libc::c_char {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut setup_code: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut setup_file_content: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut setup_file_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut msg_id: uint32_t = 0i32 as uint32_t;
    if 0 == dc_alloc_ongoing(context) {
        return 0 as *mut libc::c_char;
    }
    setup_code = dc_create_setup_code(context);
    if !setup_code.is_null() {
        /* this may require a keypair to be created. this may take a second ... */
        if !(0 != (*context).shall_stop_ongoing) {
            setup_file_content = dc_render_setup_file(context, setup_code);
            if !setup_file_content.is_null() {
                /* encrypting may also take a while ... */
                if !(0 != (*context).shall_stop_ongoing) {
                    setup_file_name = dc_get_fine_pathNfilename(
                        context,
                        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
                        b"autocrypt-setup-message.html\x00" as *const u8 as *const libc::c_char,
                    );
                    if !(setup_file_name.is_null()
                        || 0 == dc_write_file(
                            context,
                            setup_file_name,
                            setup_file_content as *const libc::c_void,
                            strlen(setup_file_content),
                        ))
                    {
                        chat_id = dc_create_chat_by_contact_id(context, 1i32 as uint32_t);
                        if !(chat_id == 0i32 as libc::c_uint) {
                            msg = dc_msg_new_untyped(context);
                            (*msg).type_0 = 60i32;
                            dc_param_set((*msg).param, 'f' as i32, setup_file_name);
                            dc_param_set(
                                (*msg).param,
                                'm' as i32,
                                b"application/autocrypt-setup\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_param_set_int((*msg).param, 'S' as i32, 6i32);
                            dc_param_set_int((*msg).param, 'u' as i32, 2i32);
                            if !(0 != (*context).shall_stop_ongoing) {
                                msg_id = dc_send_msg(context, chat_id, msg);
                                if !(msg_id == 0i32 as libc::c_uint) {
                                    dc_msg_unref(msg);
                                    msg = 0 as *mut dc_msg_t;
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"Wait for setup message being sent ...\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    loop {
                                        if 0 != (*context).shall_stop_ongoing {
                                            current_block = 6116957410927263949;
                                            break;
                                        }
                                        sleep(1i32 as libc::c_uint);
                                        msg = dc_get_msg(context, msg_id);
                                        if 0 != dc_msg_is_sent(msg) {
                                            current_block = 6450636197030046351;
                                            break;
                                        }
                                        dc_msg_unref(msg);
                                        msg = 0 as *mut dc_msg_t
                                    }
                                    match current_block {
                                        6116957410927263949 => {}
                                        _ => {
                                            dc_log_info(
                                                context,
                                                0i32,
                                                b"... setup message sent.\x00" as *const u8
                                                    as *const libc::c_char,
                                            );
                                            success = 1i32
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if 0 == success {
        free(setup_code as *mut libc::c_void);
        setup_code = 0 as *mut libc::c_char
    }
    free(setup_file_name as *mut libc::c_void);
    free(setup_file_content as *mut libc::c_void);
    dc_msg_unref(msg);
    dc_free_ongoing(context);
    return setup_code;
}
#[no_mangle]
pub unsafe extern "C" fn dc_render_setup_file(
    mut context: *mut dc_context_t,
    mut passphrase: *const libc::c_char,
) -> *mut libc::c_char {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut curr_private_key: *mut dc_key_t = dc_key_new();
    let mut passphrase_begin: [libc::c_char; 8] = [0; 8];
    let mut encr_string: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret_setupfilecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || passphrase.is_null()
        || strlen(passphrase) < 2i32 as libc::c_ulong
        || curr_private_key.is_null())
    {
        strncpy(
            passphrase_begin.as_mut_ptr(),
            passphrase,
            2i32 as libc::c_ulong,
        );
        passphrase_begin[2usize] = 0i32 as libc::c_char;
        /* create the payload */
        if !(0 == dc_ensure_secret_key_exists(context)) {
            self_addr = dc_sqlite3_get_config(
                (*context).sql,
                b"configured_addr\x00" as *const u8 as *const libc::c_char,
                0 as *const libc::c_char,
            );
            dc_key_load_self_private(curr_private_key, self_addr, (*context).sql);
            let mut e2ee_enabled: libc::c_int = dc_sqlite3_get_config_int(
                (*context).sql,
                b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                1i32,
            );
            let mut payload_key_asc: *mut libc::c_char = dc_key_render_asc(
                curr_private_key,
                if 0 != e2ee_enabled {
                    b"Autocrypt-Prefer-Encrypt: mutual\r\n\x00" as *const u8 as *const libc::c_char
                } else {
                    0 as *const libc::c_char
                },
            );
            if !payload_key_asc.is_null() {
                if !(0
                    == dc_pgp_symm_encrypt(
                        context,
                        passphrase,
                        payload_key_asc as *const libc::c_void,
                        strlen(payload_key_asc),
                        &mut encr_string,
                    ))
                {
                    free(payload_key_asc as *mut libc::c_void);
                    let mut replacement: *mut libc::c_char =
                        dc_mprintf(b"-----BEGIN PGP MESSAGE-----\r\nPassphrase-Format: numeric9x4\r\nPassphrase-Begin: %s\x00"
                                       as *const u8 as *const libc::c_char,
                                   passphrase_begin.as_mut_ptr());
                    dc_str_replace(
                        &mut encr_string,
                        b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
                        replacement,
                    );
                    free(replacement as *mut libc::c_void);
                    let mut setup_message_title: *mut libc::c_char = dc_stock_str(context, 42i32);
                    let mut setup_message_body: *mut libc::c_char = dc_stock_str(context, 43i32);
                    dc_str_replace(
                        &mut setup_message_body,
                        b"\r\x00" as *const u8 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                    dc_str_replace(
                        &mut setup_message_body,
                        b"\n\x00" as *const u8 as *const libc::c_char,
                        b"<br>\x00" as *const u8 as *const libc::c_char,
                    );
                    ret_setupfilecontent =
                        dc_mprintf(b"<!DOCTYPE html>\r\n<html>\r\n<head>\r\n<title>%s</title>\r\n</head>\r\n<body>\r\n<h1>%s</h1>\r\n<p>%s</p>\r\n<pre>\r\n%s\r\n</pre>\r\n</body>\r\n</html>\r\n\x00"
                                       as *const u8 as *const libc::c_char,
                                   setup_message_title, setup_message_title,
                                   setup_message_body, encr_string);
                    free(setup_message_title as *mut libc::c_void);
                    free(setup_message_body as *mut libc::c_void);
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    dc_key_unref(curr_private_key);
    free(encr_string as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    return ret_setupfilecontent;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_setup_code(mut context: *mut dc_context_t) -> *mut libc::c_char {
    let mut random_val: uint16_t = 0i32 as uint16_t;
    let mut i: libc::c_int = 0i32;
    let mut ret: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    i = 0i32;
    while i < 9i32 {
        loop {
            if 0 == RAND_bytes(
                &mut random_val as *mut uint16_t as *mut libc::c_uchar,
                ::std::mem::size_of::<uint16_t>() as libc::c_ulong as libc::c_int,
            ) {
                dc_log_warning(
                    context,
                    0i32,
                    b"Falling back to pseudo-number generation for the setup code.\x00" as *const u8
                        as *const libc::c_char,
                );
                RAND_pseudo_bytes(
                    &mut random_val as *mut uint16_t as *mut libc::c_uchar,
                    ::std::mem::size_of::<uint16_t>() as libc::c_ulong as libc::c_int,
                );
            }
            if !(random_val as libc::c_int > 60000i32) {
                break;
            }
        }
        random_val = (random_val as libc::c_int % 10000i32) as uint16_t;
        dc_strbuilder_catf(
            &mut ret as *mut dc_strbuilder_t,
            b"%s%04i\x00" as *const u8 as *const libc::c_char,
            if 0 != i {
                b"-\x00" as *const u8 as *const libc::c_char
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            random_val as libc::c_int,
        );
        i += 1
    }
    return ret.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_continue_key_transfer(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
    mut setup_code: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filebytes: size_t = 0i32 as size_t;
    let mut armored_key: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut norm_sc: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg_id <= 9i32 as libc::c_uint
        || setup_code.is_null())
    {
        msg = dc_get_msg(context, msg_id);
        if msg.is_null()
            || 0 == dc_msg_is_setupmessage(msg)
            || {
                filename = dc_msg_get_file(msg);
                filename.is_null()
            }
            || *filename.offset(0isize) as libc::c_int == 0i32
        {
            dc_log_error(
                context,
                0i32,
                b"Message is no Autocrypt Setup Message.\x00" as *const u8 as *const libc::c_char,
            );
        } else if 0
            == dc_read_file(
                context,
                filename,
                &mut filecontent as *mut *mut libc::c_char as *mut *mut libc::c_void,
                &mut filebytes,
            )
            || filecontent.is_null()
            || filebytes <= 0i32 as libc::c_ulong
        {
            dc_log_error(
                context,
                0i32,
                b"Cannot read Autocrypt Setup Message file.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
            norm_sc = dc_normalize_setup_code(context, setup_code);
            if norm_sc.is_null() {
                dc_log_warning(
                    context,
                    0i32,
                    b"Cannot normalize Setup Code.\x00" as *const u8 as *const libc::c_char,
                );
            } else {
                armored_key = dc_decrypt_setup_file(context, norm_sc, filecontent);
                if armored_key.is_null() {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Cannot decrypt Autocrypt Setup Message.\x00" as *const u8
                            as *const libc::c_char,
                    );
                } else if !(0 == set_self_key(context, armored_key, 1i32)) {
                    /*set default*/
                    /* error already logged */
                    success = 1i32
                }
            }
        }
    }
    free(armored_key as *mut libc::c_void);
    free(filecontent as *mut libc::c_void);
    free(filename as *mut libc::c_void);
    dc_msg_unref(msg);
    free(norm_sc as *mut libc::c_void);
    return success;
}
unsafe extern "C" fn set_self_key(
    mut context: *mut dc_context_t,
    mut armored: *const libc::c_char,
    mut set_default: libc::c_int,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    // pointer inside buf, MUST NOT be free()'d
    let mut buf_headerline: *const libc::c_char = 0 as *const libc::c_char;
    //   - " -
    let mut buf_preferencrypt: *const libc::c_char = 0 as *const libc::c_char;
    //   - " -
    let mut buf_base64: *const libc::c_char = 0 as *const libc::c_char;
    let mut private_key: *mut dc_key_t = dc_key_new();
    let mut public_key: *mut dc_key_t = dc_key_new();
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    buf = dc_strdup(armored);
    if 0 == dc_split_armored_data(
        buf,
        &mut buf_headerline,
        0 as *mut *const libc::c_char,
        &mut buf_preferencrypt,
        &mut buf_base64,
    ) || strcmp(
        buf_headerline,
        b"-----BEGIN PGP PRIVATE KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
    ) != 0i32
        || buf_base64.is_null()
    {
        dc_log_warning(
            context,
            0i32,
            b"File does not contain a private key.\x00" as *const u8 as *const libc::c_char,
        );
    } else if 0 == dc_key_set_from_base64(private_key, buf_base64, 1i32)
        || 0 == dc_pgp_is_valid_key(context, private_key)
        || 0 == dc_pgp_split_key(context, private_key, public_key)
    {
        dc_log_error(
            context,
            0i32,
            b"File does not contain a valid private key.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"DELETE FROM keypairs WHERE public_key=? OR private_key=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_blob(stmt, 1i32, (*public_key).binary, (*public_key).bytes, None);
        sqlite3_bind_blob(
            stmt,
            2i32,
            (*private_key).binary,
            (*private_key).bytes,
            None,
        );
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if 0 != set_default {
            dc_sqlite3_execute(
                (*context).sql,
                b"UPDATE keypairs SET is_default=0;\x00" as *const u8 as *const libc::c_char,
            );
        }
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if 0 == dc_key_save_self_keypair(
            public_key,
            private_key,
            self_addr,
            set_default,
            (*context).sql,
        ) {
            dc_log_error(
                context,
                0i32,
                b"Cannot save keypair.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            if !buf_preferencrypt.is_null() {
                if strcmp(
                    buf_preferencrypt,
                    b"nopreference\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    dc_sqlite3_set_config_int(
                        (*context).sql,
                        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                        0i32,
                    );
                } else if strcmp(
                    buf_preferencrypt,
                    b"mutual\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    dc_sqlite3_set_config_int(
                        (*context).sql,
                        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                        1i32,
                    );
                }
            }
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    free(buf as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    dc_key_unref(private_key);
    dc_key_unref(public_key);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_decrypt_setup_file(
    mut context: *mut dc_context_t,
    mut passphrase: *const libc::c_char,
    mut filecontent: *const libc::c_char,
) -> *mut libc::c_char {
    let mut fc_buf: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fc_headerline: *const libc::c_char = 0 as *const libc::c_char;
    let mut fc_base64: *const libc::c_char = 0 as *const libc::c_char;
    let mut binary: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut binary_bytes: size_t = 0i32 as size_t;
    let mut indx: size_t = 0i32 as size_t;
    let mut plain: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut plain_bytes: size_t = 0i32 as size_t;
    let mut payload: *mut libc::c_char = 0 as *mut libc::c_char;
    fc_buf = dc_strdup(filecontent);
    if !(0
        == dc_split_armored_data(
            fc_buf,
            &mut fc_headerline,
            0 as *mut *const libc::c_char,
            0 as *mut *const libc::c_char,
            &mut fc_base64,
        )
        || fc_headerline.is_null()
        || strcmp(
            fc_headerline,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) != 0i32
        || fc_base64.is_null())
    {
        /* convert base64 to binary */
        /*must be freed using mmap_string_unref()*/
        if !(mailmime_base64_body_parse(
            fc_base64,
            strlen(fc_base64),
            &mut indx,
            &mut binary,
            &mut binary_bytes,
        ) != MAILIMF_NO_ERROR as libc::c_int
            || binary.is_null()
            || binary_bytes == 0i32 as libc::c_ulong)
        {
            /* decrypt symmetrically */
            if !(0
                == dc_pgp_symm_decrypt(
                    context,
                    passphrase,
                    binary as *const libc::c_void,
                    binary_bytes,
                    &mut plain,
                    &mut plain_bytes,
                ))
            {
                payload = strndup(plain as *const libc::c_char, plain_bytes)
            }
        }
    }
    free(plain);
    free(fc_buf as *mut libc::c_void);
    if !binary.is_null() {
        mmap_string_unref(binary);
    }
    return payload;
}
#[no_mangle]
pub unsafe extern "C" fn dc_normalize_setup_code(
    mut context: *mut dc_context_t,
    mut in_0: *const libc::c_char,
) -> *mut libc::c_char {
    if in_0.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut out: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut out, 0i32);
    let mut outlen: libc::c_int = 0i32;
    let mut p1: *const libc::c_char = in_0;
    while 0 != *p1 {
        if *p1 as libc::c_int >= '0' as i32 && *p1 as libc::c_int <= '9' as i32 {
            dc_strbuilder_catf(
                &mut out as *mut dc_strbuilder_t,
                b"%c\x00" as *const u8 as *const libc::c_char,
                *p1 as libc::c_int,
            );
            outlen = strlen(out.buf) as libc::c_int;
            if outlen == 4i32
                || outlen == 9i32
                || outlen == 14i32
                || outlen == 19i32
                || outlen == 24i32
                || outlen == 29i32
                || outlen == 34i32
                || outlen == 39i32
            {
                dc_strbuilder_cat(&mut out, b"-\x00" as *const u8 as *const libc::c_char);
            }
        }
        p1 = p1.offset(1isize)
    }
    return out.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_job_do_DC_JOB_IMEX_IMAP(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut ongoing_allocated_here: libc::c_int = 0i32;
    let mut what: libc::c_int = 0i32;
    let mut param1: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param2: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*context).sql.is_null())
    {
        if !(0 == dc_alloc_ongoing(context)) {
            ongoing_allocated_here = 1i32;
            what = dc_param_get_int((*job).param, 'S' as i32, 0i32);
            param1 = dc_param_get((*job).param, 'E' as i32, 0 as *const libc::c_char);
            param2 = dc_param_get((*job).param, 'F' as i32, 0 as *const libc::c_char);
            if param1.is_null() {
                dc_log_error(
                    context,
                    0i32,
                    b"No Import/export dir/file given.\x00" as *const u8 as *const libc::c_char,
                );
            } else {
                dc_log_info(
                    context,
                    0i32,
                    b"Import/export process started.\x00" as *const u8 as *const libc::c_char,
                );
                (*context).cb.expect("non-null function pointer")(
                    context,
                    2051i32,
                    10i32 as uintptr_t,
                    0i32 as uintptr_t,
                );
                if 0 == dc_sqlite3_is_open((*context).sql) {
                    dc_log_error(
                        context,
                        0i32,
                        b"Import/export: Database not opened.\x00" as *const u8
                            as *const libc::c_char,
                    );
                } else {
                    if what == 1i32 || what == 11i32 {
                        /* before we export anything, make sure the private key exists */
                        if 0 == dc_ensure_secret_key_exists(context) {
                            dc_log_error(context, 0i32,
                                         b"Import/export: Cannot create private key or private key not available.\x00"
                                             as *const u8 as
                                             *const libc::c_char);
                            current_block = 3568988166330621280;
                        } else {
                            dc_create_folder(context, param1);
                            current_block = 4495394744059808450;
                        }
                    } else {
                        current_block = 4495394744059808450;
                    }
                    match current_block {
                        3568988166330621280 => {}
                        _ => match what {
                            1 => {
                                current_block = 10991094515395304355;
                                match current_block {
                                    2973387206439775448 => {
                                        if 0 == import_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    11250025114629486028 => {
                                        if 0 == import_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    12669919903773909120 => {
                                        if 0 == export_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    _ => {
                                        if 0 == export_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                }
                                match current_block {
                                    3568988166330621280 => {}
                                    _ => {
                                        dc_log_info(
                                            context,
                                            0i32,
                                            b"Import/export completed.\x00" as *const u8
                                                as *const libc::c_char,
                                        );
                                        success = 1i32
                                    }
                                }
                            }
                            2 => {
                                current_block = 11250025114629486028;
                                match current_block {
                                    2973387206439775448 => {
                                        if 0 == import_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    11250025114629486028 => {
                                        if 0 == import_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    12669919903773909120 => {
                                        if 0 == export_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    _ => {
                                        if 0 == export_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                }
                                match current_block {
                                    3568988166330621280 => {}
                                    _ => {
                                        dc_log_info(
                                            context,
                                            0i32,
                                            b"Import/export completed.\x00" as *const u8
                                                as *const libc::c_char,
                                        );
                                        success = 1i32
                                    }
                                }
                            }
                            11 => {
                                current_block = 12669919903773909120;
                                match current_block {
                                    2973387206439775448 => {
                                        if 0 == import_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    11250025114629486028 => {
                                        if 0 == import_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    12669919903773909120 => {
                                        if 0 == export_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    _ => {
                                        if 0 == export_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                }
                                match current_block {
                                    3568988166330621280 => {}
                                    _ => {
                                        dc_log_info(
                                            context,
                                            0i32,
                                            b"Import/export completed.\x00" as *const u8
                                                as *const libc::c_char,
                                        );
                                        success = 1i32
                                    }
                                }
                            }
                            12 => {
                                current_block = 2973387206439775448;
                                match current_block {
                                    2973387206439775448 => {
                                        if 0 == import_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    11250025114629486028 => {
                                        if 0 == import_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    12669919903773909120 => {
                                        if 0 == export_backup(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                    _ => {
                                        if 0 == export_self_keys(context, param1) {
                                            current_block = 3568988166330621280;
                                        } else {
                                            current_block = 1118134448028020070;
                                        }
                                    }
                                }
                                match current_block {
                                    3568988166330621280 => {}
                                    _ => {
                                        dc_log_info(
                                            context,
                                            0i32,
                                            b"Import/export completed.\x00" as *const u8
                                                as *const libc::c_char,
                                        );
                                        success = 1i32
                                    }
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }
    free(param1 as *mut libc::c_void);
    free(param2 as *mut libc::c_void);
    if 0 != ongoing_allocated_here {
        dc_free_ongoing(context);
    }
    (*context).cb.expect("non-null function pointer")(
        context,
        2051i32,
        (if 0 != success { 1000i32 } else { 0i32 }) as uintptr_t,
        0i32 as uintptr_t,
    );
}
/* ******************************************************************************
 * Import backup
 ******************************************************************************/
unsafe extern "C" fn import_backup(
    mut context: *mut dc_context_t,
    mut backup_to_import: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut processed_files_cnt: libc::c_int = 0i32;
    let mut total_files_cnt: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut repl_from: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut repl_to: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_log_info(
        context,
        0i32,
        b"Import \"%s\" to \"%s\".\x00" as *const u8 as *const libc::c_char,
        backup_to_import,
        (*context).dbfile,
    );
    if 0 != dc_is_configured(context) {
        dc_log_error(
            context,
            0i32,
            b"Cannot import backups to accounts in use.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        if 0 != dc_sqlite3_is_open((*context).sql) {
            dc_sqlite3_close((*context).sql);
        }
        dc_delete_file(context, (*context).dbfile);
        if 0 != dc_file_exist(context, (*context).dbfile) {
            dc_log_error(
                context,
                0i32,
                b"Cannot import backups: Cannot delete the old file.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else if !(0 == dc_copy_file(context, backup_to_import, (*context).dbfile)) {
            /* error already logged */
            /* re-open copied database file */
            if !(0 == dc_sqlite3_open((*context).sql, (*context).dbfile, 0i32)) {
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"SELECT COUNT(*) FROM backup_blobs;\x00" as *const u8 as *const libc::c_char,
                );
                sqlite3_step(stmt);
                total_files_cnt = sqlite3_column_int(stmt, 0i32);
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"SELECT file_name, file_content FROM backup_blobs ORDER BY id;\x00"
                        as *const u8 as *const libc::c_char,
                );
                loop {
                    if !(sqlite3_step(stmt) == 100i32) {
                        current_block = 10891380440665537214;
                        break;
                    }
                    if 0 != (*context).shall_stop_ongoing {
                        current_block = 8648553629232744886;
                        break;
                    }
                    processed_files_cnt += 1;
                    let mut permille: libc::c_int = processed_files_cnt * 1000i32 / total_files_cnt;
                    if permille < 10i32 {
                        permille = 10i32
                    }
                    if permille > 990i32 {
                        permille = 990i32
                    }
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2051i32,
                        permille as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    let mut file_name: *const libc::c_char =
                        sqlite3_column_text(stmt, 0i32) as *const libc::c_char;
                    let mut file_bytes: libc::c_int = sqlite3_column_bytes(stmt, 1i32);
                    let mut file_content: *const libc::c_void = sqlite3_column_blob(stmt, 1i32);
                    if !(file_bytes > 0i32 && !file_content.is_null()) {
                        continue;
                    }
                    free(pathNfilename as *mut libc::c_void);
                    pathNfilename = dc_mprintf(
                        b"%s/%s\x00" as *const u8 as *const libc::c_char,
                        (*context).blobdir,
                        file_name,
                    );
                    if !(0
                        == dc_write_file(
                            context,
                            pathNfilename,
                            file_content,
                            file_bytes as size_t,
                        ))
                    {
                        continue;
                    }
                    dc_log_error(
                        context,
                        0i32,
                        b"Storage full? Cannot write file %s with %i bytes.\x00" as *const u8
                            as *const libc::c_char,
                        pathNfilename,
                        file_bytes,
                    );
                    /* otherwise the user may believe the stuff is imported correctly, but there are files missing ... */
                    current_block = 8648553629232744886;
                    break;
                }
                match current_block {
                    8648553629232744886 => {}
                    _ => {
                        sqlite3_finalize(stmt);
                        stmt = 0 as *mut sqlite3_stmt;
                        dc_sqlite3_execute(
                            (*context).sql,
                            b"DROP TABLE backup_blobs;\x00" as *const u8 as *const libc::c_char,
                        );
                        dc_sqlite3_try_execute(
                            (*context).sql,
                            b"VACUUM;\x00" as *const u8 as *const libc::c_char,
                        );
                        success = 1i32
                    }
                }
            }
        }
    }
    free(pathNfilename as *mut libc::c_void);
    free(repl_from as *mut libc::c_void);
    free(repl_to as *mut libc::c_void);
    sqlite3_finalize(stmt);
    return success;
}
/* ******************************************************************************
 * Export backup
 ******************************************************************************/
/* the FILE_PROGRESS macro calls the callback with the permille of files processed.
The macro avoids weird values of 0% or 100% while still working. */
unsafe extern "C" fn export_backup(
    mut context: *mut dc_context_t,
    mut dir: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut closed: libc::c_int = 0i32;
    let mut dest_pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_sql: *mut dc_sqlite3_t = 0 as *mut dc_sqlite3_t;
    let mut now: time_t = time(0 as *mut time_t);
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut prefix_len: libc::c_int =
        strlen(b"delta-chat\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut suffix_len: libc::c_int =
        strlen(b"bak\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut curr_pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut buf_bytes: size_t = 0i32 as size_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut total_files_cnt: libc::c_int = 0i32;
    let mut processed_files_cnt: libc::c_int = 0i32;
    let mut delete_dest_file: libc::c_int = 0i32;
    /* get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    FIXME: we should write to a temporary file first and rename it on success. this would guarantee the backup is complete. however, currently it is not clear it the import exists in the long run (may be replaced by a restore-from-imap)*/
    let mut timeinfo: *mut tm = 0 as *mut tm;
    let mut buffer: [libc::c_char; 256] = [0; 256];
    timeinfo = localtime(&mut now);
    strftime(
        buffer.as_mut_ptr(),
        256i32 as size_t,
        b"delta-chat-%Y-%m-%d.bak\x00" as *const u8 as *const libc::c_char,
        timeinfo,
    );
    dest_pathNfilename = dc_get_fine_pathNfilename(context, dir, buffer.as_mut_ptr());
    if dest_pathNfilename.is_null() {
        dc_log_error(
            context,
            0i32,
            b"Cannot get backup file name.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        dc_housekeeping(context);
        dc_sqlite3_try_execute(
            (*context).sql,
            b"VACUUM;\x00" as *const u8 as *const libc::c_char,
        );
        dc_sqlite3_close((*context).sql);
        closed = 1i32;
        dc_log_info(
            context,
            0i32,
            b"Backup \"%s\" to \"%s\".\x00" as *const u8 as *const libc::c_char,
            (*context).dbfile,
            dest_pathNfilename,
        );
        if !(0 == dc_copy_file(context, (*context).dbfile, dest_pathNfilename)) {
            /* error already logged */
            dc_sqlite3_open((*context).sql, (*context).dbfile, 0i32);
            closed = 0i32;
            /* add all files as blobs to the database copy (this does not require the source to be locked, neigher the destination as it is used only here) */
            /*for logging only*/
            dest_sql = dc_sqlite3_new(context);
            if !(dest_sql.is_null() || 0 == dc_sqlite3_open(dest_sql, dest_pathNfilename, 0i32)) {
                /* error already logged */
                if 0 == dc_sqlite3_table_exists(
                    dest_sql,
                    b"backup_blobs\x00" as *const u8 as *const libc::c_char,
                ) {
                    if 0 ==
                           dc_sqlite3_execute(dest_sql,
                                              b"CREATE TABLE backup_blobs (id INTEGER PRIMARY KEY, file_name, file_content);\x00"
                                                  as *const u8 as
                                                  *const libc::c_char) {
                        /* error already logged */
                        current_block = 11487273724841241105;
                    } else { current_block = 14648156034262866959; }
                } else {
                    current_block = 14648156034262866959;
                }
                match current_block {
                    11487273724841241105 => {}
                    _ => {
                        total_files_cnt = 0i32;
                        dir_handle = opendir((*context).blobdir);
                        if dir_handle.is_null() {
                            dc_log_error(
                                context,
                                0i32,
                                b"Backup: Cannot get info for blob-directory \"%s\".\x00"
                                    as *const u8
                                    as *const libc::c_char,
                                (*context).blobdir,
                            );
                        } else {
                            loop {
                                dir_entry = readdir(dir_handle);
                                if dir_entry.is_null() {
                                    break;
                                }
                                total_files_cnt += 1
                            }
                            closedir(dir_handle);
                            dir_handle = 0 as *mut DIR;
                            if total_files_cnt > 0i32 {
                                /* scan directory, pass 2: copy files */
                                dir_handle = opendir((*context).blobdir);
                                if dir_handle.is_null() {
                                    dc_log_error(
                                        context,
                                        0i32,
                                        b"Backup: Cannot copy from blob-directory \"%s\".\x00"
                                            as *const u8
                                            as *const libc::c_char,
                                        (*context).blobdir,
                                    );
                                    current_block = 11487273724841241105;
                                } else {
                                    stmt =
                                        dc_sqlite3_prepare(dest_sql,
                                                           b"INSERT INTO backup_blobs (file_name, file_content) VALUES (?, ?);\x00"
                                                               as *const u8 as
                                                               *const libc::c_char);
                                    loop {
                                        dir_entry = readdir(dir_handle);
                                        if dir_entry.is_null() {
                                            current_block = 2631791190359682872;
                                            break;
                                        }
                                        if 0 != (*context).shall_stop_ongoing {
                                            delete_dest_file = 1i32;
                                            current_block = 11487273724841241105;
                                            break;
                                        } else {
                                            processed_files_cnt += 1;
                                            let mut permille: libc::c_int =
                                                processed_files_cnt * 1000i32 / total_files_cnt;
                                            if permille < 10i32 {
                                                permille = 10i32
                                            }
                                            if permille > 990i32 {
                                                permille = 990i32
                                            }
                                            (*context).cb.expect("non-null function pointer")(
                                                context,
                                                2051i32,
                                                permille as uintptr_t,
                                                0i32 as uintptr_t,
                                            );
                                            /* name without path; may also be `.` or `..` */
                                            let mut name: *mut libc::c_char =
                                                (*dir_entry).d_name.as_mut_ptr();
                                            let mut name_len: libc::c_int =
                                                strlen(name) as libc::c_int;
                                            if !(name_len == 1i32
                                                && *name.offset(0isize) as libc::c_int
                                                    == '.' as i32
                                                || name_len == 2i32
                                                    && *name.offset(0isize) as libc::c_int
                                                        == '.' as i32
                                                    && *name.offset(1isize) as libc::c_int
                                                        == '.' as i32
                                                || name_len > prefix_len
                                                    && strncmp(
                                                        name,
                                                        b"delta-chat\x00" as *const u8
                                                            as *const libc::c_char,
                                                        prefix_len as libc::c_ulong,
                                                    ) == 0i32
                                                    && name_len > suffix_len
                                                    && strncmp(
                                                        &mut *name.offset(
                                                            (name_len - suffix_len - 1i32) as isize,
                                                        ),
                                                        b".bak\x00" as *const u8
                                                            as *const libc::c_char,
                                                        suffix_len as libc::c_ulong,
                                                    ) == 0i32)
                                            {
                                                //dc_log_info(context, 0, "Backup: Skipping \"%s\".", name);
                                                free(curr_pathNfilename as *mut libc::c_void);
                                                curr_pathNfilename = dc_mprintf(
                                                    b"%s/%s\x00" as *const u8
                                                        as *const libc::c_char,
                                                    (*context).blobdir,
                                                    name,
                                                );
                                                free(buf);
                                                if 0 == dc_read_file(
                                                    context,
                                                    curr_pathNfilename,
                                                    &mut buf,
                                                    &mut buf_bytes,
                                                ) || buf.is_null()
                                                    || buf_bytes <= 0i32 as libc::c_ulong
                                                {
                                                    continue;
                                                }
                                                sqlite3_bind_text(stmt, 1i32, name, -1i32, None);
                                                sqlite3_bind_blob(
                                                    stmt,
                                                    2i32,
                                                    buf,
                                                    buf_bytes as libc::c_int,
                                                    None,
                                                );
                                                if sqlite3_step(stmt) != 101i32 {
                                                    dc_log_error(context,
                                                                 0i32,
                                                                 b"Disk full? Cannot add file \"%s\" to backup.\x00"
                                                                     as
                                                                     *const u8
                                                                     as
                                                                     *const libc::c_char,
                                                                 curr_pathNfilename);
                                                    /* this is not recoverable! writing to the sqlite database should work! */
                                                    current_block = 11487273724841241105;
                                                    break;
                                                } else {
                                                    sqlite3_reset(stmt);
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"Backup: No files to copy.\x00" as *const u8
                                        as *const libc::c_char,
                                    (*context).blobdir,
                                );
                                current_block = 2631791190359682872;
                            }
                            match current_block {
                                11487273724841241105 => {}
                                _ => {
                                    dc_sqlite3_set_config_int(
                                        dest_sql,
                                        b"backup_time\x00" as *const u8 as *const libc::c_char,
                                        now as int32_t,
                                    );
                                    (*context).cb.expect("non-null function pointer")(
                                        context,
                                        2052i32,
                                        dest_pathNfilename as uintptr_t,
                                        0i32 as uintptr_t,
                                    );
                                    success = 1i32
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    if 0 != closed {
        dc_sqlite3_open((*context).sql, (*context).dbfile, 0i32);
    }
    sqlite3_finalize(stmt);
    dc_sqlite3_close(dest_sql);
    dc_sqlite3_unref(dest_sql);
    if 0 != delete_dest_file {
        dc_delete_file(context, dest_pathNfilename);
    }
    free(dest_pathNfilename as *mut libc::c_void);
    free(curr_pathNfilename as *mut libc::c_void);
    free(buf);
    return success;
}
/* ******************************************************************************
 * Classic key import
 ******************************************************************************/
unsafe extern "C" fn import_self_keys(
    mut context: *mut dc_context_t,
    mut dir_name: *const libc::c_char,
) -> libc::c_int {
    /* hint: even if we switch to import Autocrypt Setup Files, we should leave the possibility to import
    plain ASC keys, at least keys without a password, if we do not want to implement a password entry function.
    Importing ASC keys is useful to use keys in Delta Chat used by any other non-Autocrypt-PGP implementation.

    Maybe we should make the "default" key handlong also a little bit smarter
    (currently, the last imported key is the standard key unless it contains the string "legacy" in its name) */
    let mut imported_cnt: libc::c_int = 0i32;
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut path_plus_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut set_default: libc::c_int = 0i32;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf_bytes: size_t = 0i32 as size_t;
    // a pointer inside buf, MUST NOT be free()'d
    let mut private_key: *const libc::c_char = 0 as *const libc::c_char;
    let mut buf2: *mut libc::c_char = 0 as *mut libc::c_char;
    // a pointer inside buf2, MUST NOT be free()'d
    let mut buf2_headerline: *const libc::c_char = 0 as *const libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || dir_name.is_null())
    {
        dir_handle = opendir(dir_name);
        if dir_handle.is_null() {
            dc_log_error(
                context,
                0i32,
                b"Import: Cannot open directory \"%s\".\x00" as *const u8 as *const libc::c_char,
                dir_name,
            );
        } else {
            loop {
                dir_entry = readdir(dir_handle);
                if dir_entry.is_null() {
                    break;
                }
                free(suffix as *mut libc::c_void);
                suffix = dc_get_filesuffix_lc((*dir_entry).d_name.as_mut_ptr());
                if suffix.is_null()
                    || strcmp(suffix, b"asc\x00" as *const u8 as *const libc::c_char) != 0i32
                {
                    continue;
                }
                free(path_plus_name as *mut libc::c_void);
                path_plus_name = dc_mprintf(
                    b"%s/%s\x00" as *const u8 as *const libc::c_char,
                    dir_name,
                    (*dir_entry).d_name.as_mut_ptr(),
                );
                dc_log_info(
                    context,
                    0i32,
                    b"Checking: %s\x00" as *const u8 as *const libc::c_char,
                    path_plus_name,
                );
                free(buf as *mut libc::c_void);
                buf = 0 as *mut libc::c_char;
                if 0 == dc_read_file(
                    context,
                    path_plus_name,
                    &mut buf as *mut *mut libc::c_char as *mut *mut libc::c_void,
                    &mut buf_bytes,
                ) || buf_bytes < 50i32 as libc::c_ulong
                {
                    continue;
                }
                private_key = buf;
                free(buf2 as *mut libc::c_void);
                buf2 = dc_strdup(buf);
                if 0 != dc_split_armored_data(
                    buf2,
                    &mut buf2_headerline,
                    0 as *mut *const libc::c_char,
                    0 as *mut *const libc::c_char,
                    0 as *mut *const libc::c_char,
                ) && strcmp(
                    buf2_headerline,
                    b"-----BEGIN PGP PUBLIC KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    private_key = strstr(
                        buf,
                        b"-----BEGIN PGP PRIVATE KEY BLOCK\x00" as *const u8 as *const libc::c_char,
                    );
                    if private_key.is_null() {
                        /* this is no error but quite normal as we always export the public keys together with the private ones */
                        continue;
                    }
                }
                set_default = 1i32;
                if !strstr(
                    (*dir_entry).d_name.as_mut_ptr(),
                    b"legacy\x00" as *const u8 as *const libc::c_char,
                )
                .is_null()
                {
                    dc_log_info(
                        context,
                        0i32,
                        b"Treating \"%s\" as a legacy private key.\x00" as *const u8
                            as *const libc::c_char,
                        path_plus_name,
                    );
                    set_default = 0i32
                }
                if 0 == set_self_key(context, private_key, set_default) {
                    continue;
                }
                imported_cnt += 1
            }
            if imported_cnt == 0i32 {
                dc_log_error(
                    context,
                    0i32,
                    b"No private keys found in \"%s\".\x00" as *const u8 as *const libc::c_char,
                    dir_name,
                );
            }
        }
    }
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    free(suffix as *mut libc::c_void);
    free(path_plus_name as *mut libc::c_void);
    free(buf as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    return imported_cnt;
}
unsafe extern "C" fn export_self_keys(
    mut context: *mut dc_context_t,
    mut dir: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut export_errors: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut id: libc::c_int = 0i32;
    let mut is_default: libc::c_int = 0i32;
    let mut public_key: *mut dc_key_t = dc_key_new();
    let mut private_key: *mut dc_key_t = dc_key_new();
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT id, public_key, private_key, is_default FROM keypairs;\x00" as *const u8
            as *const libc::c_char,
    );
    if !stmt.is_null() {
        while sqlite3_step(stmt) == 100i32 {
            id = sqlite3_column_int(stmt, 0i32);
            dc_key_set_from_stmt(public_key, stmt, 1i32, 0i32);
            dc_key_set_from_stmt(private_key, stmt, 2i32, 1i32);
            is_default = sqlite3_column_int(stmt, 3i32);
            if 0 == export_key_to_asc_file(context, dir, id, public_key, is_default) {
                export_errors += 1
            }
            if 0 == export_key_to_asc_file(context, dir, id, private_key, is_default) {
                export_errors += 1
            }
        }
        if export_errors == 0i32 {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    dc_key_unref(public_key);
    dc_key_unref(private_key);
    return success;
}
/* ******************************************************************************
 * Classic key export
 ******************************************************************************/
unsafe extern "C" fn export_key_to_asc_file(
    mut context: *mut dc_context_t,
    mut dir: *const libc::c_char,
    mut id: libc::c_int,
    mut key: *const dc_key_t,
    mut is_default: libc::c_int,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut file_name: *mut libc::c_char = 0 as *mut libc::c_char;
    if 0 != is_default {
        file_name = dc_mprintf(
            b"%s/%s-key-default.asc\x00" as *const u8 as *const libc::c_char,
            dir,
            if (*key).type_0 == 0i32 {
                b"public\x00" as *const u8 as *const libc::c_char
            } else {
                b"private\x00" as *const u8 as *const libc::c_char
            },
        )
    } else {
        file_name = dc_mprintf(
            b"%s/%s-key-%i.asc\x00" as *const u8 as *const libc::c_char,
            dir,
            if (*key).type_0 == 0i32 {
                b"public\x00" as *const u8 as *const libc::c_char
            } else {
                b"private\x00" as *const u8 as *const libc::c_char
            },
            id,
        )
    }
    dc_log_info(
        context,
        0i32,
        b"Exporting key %s\x00" as *const u8 as *const libc::c_char,
        file_name,
    );
    dc_delete_file(context, file_name);
    if 0 == dc_key_render_asc_to_file(key, file_name, context) {
        dc_log_error(
            context,
            0i32,
            b"Cannot write key to %s\x00" as *const u8 as *const libc::c_char,
            file_name,
        );
    } else {
        (*context).cb.expect("non-null function pointer")(
            context,
            2052i32,
            file_name as uintptr_t,
            0i32 as uintptr_t,
        );
        success = 1i32
    }
    free(file_name as *mut libc::c_void);
    return success;
}

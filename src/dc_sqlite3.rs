use c2rust_bitfields::BitfieldStruct;
use libc;
extern "C" {
    pub type _telldir;
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
    fn closedir(_: *mut DIR) -> libc::c_int;
    #[no_mangle]
    fn opendir(_: *const libc::c_char) -> *mut DIR;
    #[no_mangle]
    fn readdir(_: *mut DIR) -> *mut dirent;
    #[no_mangle]
    fn stat(_: *const libc::c_char, _: *mut stat) -> libc::c_int;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn atol(_: *const libc::c_char) -> libc::c_long;
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn sscanf(_: *const libc::c_char, _: *const libc::c_char, _: ...) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_threadsafe() -> libc::c_int;
    #[no_mangle]
    fn sqlite3_close(_: *mut sqlite3) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_busy_timeout(_: *mut sqlite3, ms: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_mprintf(_: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn sqlite3_vmprintf(_: *const libc::c_char, _: ::std::ffi::VaList) -> *mut libc::c_char;
    #[no_mangle]
    fn sqlite3_free(_: *mut libc::c_void);
    #[no_mangle]
    fn sqlite3_open_v2(
        filename: *const libc::c_char,
        ppDb: *mut *mut sqlite3,
        flags: libc::c_int,
        zVfs: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn sqlite3_errmsg(_: *mut sqlite3) -> *const libc::c_char;
    #[no_mangle]
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        zSql: *const libc::c_char,
        nByte: libc::c_int,
        ppStmt: *mut *mut sqlite3_stmt,
        pzTail: *mut *const libc::c_char,
    ) -> libc::c_int;
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
    fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    #[no_mangle]
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    #[no_mangle]
    fn dc_log_info(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    /* file tools */
    #[no_mangle]
    fn dc_ensure_no_slash(pathNfilename: *mut libc::c_char);
    #[no_mangle]
    fn dc_apeerstate_new(_: *mut dc_context_t) -> *mut dc_apeerstate_t;
    #[no_mangle]
    fn dc_apeerstate_unref(_: *mut dc_apeerstate_t);
    #[no_mangle]
    fn dc_apeerstate_save_to_db(
        _: *const dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        create: libc::c_int,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_recalc_fingerprint(_: *mut dc_apeerstate_t) -> libc::c_int;
    #[no_mangle]
    fn dc_apeerstate_load_by_addr(
        _: *mut dc_apeerstate_t,
        _: *mut dc_sqlite3_t,
        addr: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_hash_clear(_: *mut dc_hash_t);
    #[no_mangle]
    fn dc_delete_file(_: *mut dc_context_t, pathNFilename: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn dc_hash_find(
        _: *const dc_hash_t,
        pKey: *const libc::c_void,
        nKey: libc::c_int,
    ) -> *mut libc::c_void;
    #[no_mangle]
    fn dc_hash_insert(
        _: *mut dc_hash_t,
        pKey: *const libc::c_void,
        nKey: libc::c_int,
        pData: *mut libc::c_void,
    ) -> *mut libc::c_void;
    /* library-private */
    #[no_mangle]
    fn dc_param_new() -> *mut dc_param_t;
    #[no_mangle]
    fn dc_param_unref(_: *mut dc_param_t);
    #[no_mangle]
    fn dc_param_get(
        _: *const dc_param_t,
        key: libc::c_int,
        def: *const libc::c_char,
    ) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_param_set_packed(_: *mut dc_param_t, _: *const libc::c_char);
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
}
pub type __builtin_va_list = [__va_list_tag; 1];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list_tag {
    pub gp_offset: libc::c_uint,
    pub fp_offset: libc::c_uint,
    pub overflow_arg_area: *mut libc::c_void,
    pub reg_save_area: *mut libc::c_void,
}
pub type __uint8_t = libc::c_uchar;
pub type __uint16_t = libc::c_ushort;
pub type __int32_t = libc::c_int;
pub type __uint32_t = libc::c_uint;
pub type __int64_t = libc::c_longlong;
pub type __uint64_t = libc::c_ulonglong;
pub type __darwin_size_t = libc::c_ulong;
pub type __darwin_va_list = __builtin_va_list;
pub type __darwin_ssize_t = libc::c_long;
pub type __darwin_time_t = libc::c_long;
pub type __darwin_blkcnt_t = __int64_t;
pub type __darwin_blksize_t = __int32_t;
pub type __darwin_dev_t = __int32_t;
pub type __darwin_gid_t = __uint32_t;
pub type __darwin_ino64_t = __uint64_t;
pub type __darwin_mode_t = __uint16_t;
pub type __darwin_off_t = __int64_t;
pub type __darwin_uid_t = __uint32_t;
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
pub type int32_t = libc::c_int;
pub type int64_t = libc::c_longlong;
pub type uintptr_t = libc::c_ulong;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __darwin_time_t,
    pub tv_nsec: libc::c_long,
}
pub type blkcnt_t = __darwin_blkcnt_t;
pub type blksize_t = __darwin_blksize_t;
pub type dev_t = __darwin_dev_t;
pub type mode_t = __darwin_mode_t;
pub type nlink_t = __uint16_t;
pub type uid_t = __darwin_uid_t;
pub type gid_t = __darwin_gid_t;
pub type off_t = __darwin_off_t;
pub type time_t = __darwin_time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: dev_t,
    pub st_mode: mode_t,
    pub st_nlink: nlink_t,
    pub st_ino: __darwin_ino64_t,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    pub st_rdev: dev_t,
    pub st_atimespec: timespec,
    pub st_mtimespec: timespec,
    pub st_ctimespec: timespec,
    pub st_birthtimespec: timespec,
    pub st_size: off_t,
    pub st_blocks: blkcnt_t,
    pub st_blksize: blksize_t,
    pub st_flags: __uint32_t,
    pub st_gen: __uint32_t,
    pub st_lspare: __int32_t,
    pub st_qspare: [__int64_t; 2],
}
pub type size_t = __darwin_size_t;
pub type uint8_t = libc::c_uchar;
pub type uint32_t = libc::c_uint;
pub type uint64_t = libc::c_ulonglong;
pub type ssize_t = __darwin_ssize_t;
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
pub type va_list = __darwin_va_list;
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
pub type unnamed_3 = libc::c_uint;
pub const DC_MOVE_STATE_MOVING: unnamed_3 = 3;
pub const DC_MOVE_STATE_STAY: unnamed_3 = 2;
pub const DC_MOVE_STATE_PENDING: unnamed_3 = 1;
pub const DC_MOVE_STATE_UNDEFINED: unnamed_3 = 0;
pub type sqlite3_destructor_type = Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>;
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
/* Forward declarations of structures.
 */
pub type dc_hash_t = _dc_hash;
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
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_new(mut context: *mut dc_context_t) -> *mut dc_sqlite3_t {
    let mut sql: *mut dc_sqlite3_t = 0 as *mut dc_sqlite3_t;
    sql = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_sqlite3_t>() as libc::c_ulong,
    ) as *mut dc_sqlite3_t;
    if sql.is_null() {
        exit(24i32);
    }
    (*sql).context = context;
    return sql;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_unref(mut sql: *mut dc_sqlite3_t) {
    if sql.is_null() {
        return;
    }
    if !(*sql).cobj.is_null() {
        dc_sqlite3_close(sql);
    }
    free(sql as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_close(mut sql: *mut dc_sqlite3_t) {
    if sql.is_null() {
        return;
    }
    if !(*sql).cobj.is_null() {
        sqlite3_close((*sql).cobj);
        (*sql).cobj = 0 as *mut sqlite3
    }
    dc_log_info(
        (*sql).context,
        0i32,
        b"Database closed.\x00" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_open(
    mut sql: *mut dc_sqlite3_t,
    mut dbfile: *const libc::c_char,
    mut flags: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    if 0 != dc_sqlite3_is_open(sql) {
        return 0i32;
    }
    if !(sql.is_null() || dbfile.is_null()) {
        if sqlite3_threadsafe() == 0i32 {
            dc_log_error(
                (*sql).context,
                0i32,
                b"Sqlite3 compiled thread-unsafe; this is not supported.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else if !(*sql).cobj.is_null() {
            dc_log_error(
                (*sql).context,
                0i32,
                b"Cannot open, database \"%s\" already opened.\x00" as *const u8
                    as *const libc::c_char,
                dbfile,
            );
        } else if sqlite3_open_v2(
            dbfile,
            &mut (*sql).cobj,
            0x10000i32
                | if 0 != flags & 0x1i32 {
                    0x1i32
                } else {
                    0x2i32 | 0x4i32
                },
            0 as *const libc::c_char,
        ) != 0i32
        {
            dc_sqlite3_log_error(
                sql,
                b"Cannot open database \"%s\".\x00" as *const u8 as *const libc::c_char,
                dbfile,
            );
        } else {
            dc_sqlite3_execute(
                sql,
                b"PRAGMA secure_delete=on;\x00" as *const u8 as *const libc::c_char,
            );
            sqlite3_busy_timeout((*sql).cobj, 10i32 * 1000i32);
            if 0 == flags & 0x1i32 {
                let mut exists_before_update: libc::c_int = 0i32;
                let mut dbversion_before_update: libc::c_int = 0i32;
                /* Init tables to dbversion=0 */
                if 0 == dc_sqlite3_table_exists(
                    sql,
                    b"config\x00" as *const u8 as *const libc::c_char,
                ) {
                    dc_log_info(
                        (*sql).context,
                        0i32,
                        b"First time init: creating tables in \"%s\".\x00" as *const u8
                            as *const libc::c_char,
                        dbfile,
                    );
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE config (id INTEGER PRIMARY KEY, keyname TEXT, value TEXT);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX config_index1 ON config (keyname);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE contacts (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT DEFAULT \'\', addr TEXT DEFAULT \'\' COLLATE NOCASE, origin INTEGER DEFAULT 0, blocked INTEGER DEFAULT 0, last_seen INTEGER DEFAULT 0, param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX contacts_index1 ON contacts (name COLLATE NOCASE);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX contacts_index2 ON contacts (addr COLLATE NOCASE);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"INSERT INTO contacts (id,name,origin) VALUES (1,\'self\',262144), (2,\'device\',262144), (3,\'rsvd\',262144), (4,\'rsvd\',262144), (5,\'rsvd\',262144), (6,\'rsvd\',262144), (7,\'rsvd\',262144), (8,\'rsvd\',262144), (9,\'rsvd\',262144);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE chats (id INTEGER PRIMARY KEY AUTOINCREMENT,  type INTEGER DEFAULT 0, name TEXT DEFAULT \'\', draft_timestamp INTEGER DEFAULT 0, draft_txt TEXT DEFAULT \'\', blocked INTEGER DEFAULT 0, grpid TEXT DEFAULT \'\', param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX chats_index1 ON chats (grpid);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);\x00"
                            as *const u8 as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"INSERT INTO chats (id,type,name) VALUES (1,120,\'deaddrop\'), (2,120,\'rsvd\'), (3,120,\'trash\'), (4,120,\'msgs_in_creation\'), (5,120,\'starred\'), (6,120,\'archivedlink\'), (7,100,\'rsvd\'), (8,100,\'rsvd\'), (9,100,\'rsvd\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE msgs (id INTEGER PRIMARY KEY AUTOINCREMENT, rfc724_mid TEXT DEFAULT \'\', server_folder TEXT DEFAULT \'\', server_uid INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0, to_id INTEGER DEFAULT 0, timestamp INTEGER DEFAULT 0, type INTEGER DEFAULT 0, state INTEGER DEFAULT 0, msgrmsg INTEGER DEFAULT 1, bytes INTEGER DEFAULT 0, txt TEXT DEFAULT \'\', txt_raw TEXT DEFAULT \'\', param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index1 ON msgs (rfc724_mid);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index2 ON msgs (chat_id);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index3 ON msgs (timestamp);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX msgs_index4 ON msgs (state);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    dc_sqlite3_execute(sql,
                                       b"INSERT INTO msgs (id,msgrmsg,txt) VALUES (1,0,\'marker1\'), (2,0,\'rsvd\'), (3,0,\'rsvd\'), (4,0,\'rsvd\'), (5,0,\'rsvd\'), (6,0,\'rsvd\'), (7,0,\'rsvd\'), (8,0,\'rsvd\'), (9,0,\'daymarker\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(sql,
                                       b"CREATE TABLE jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, added_timestamp INTEGER, desired_timestamp INTEGER DEFAULT 0, action INTEGER, foreign_id INTEGER, param TEXT DEFAULT \'\');\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                    dc_sqlite3_execute(
                        sql,
                        b"CREATE INDEX jobs_index1 ON jobs (desired_timestamp);\x00" as *const u8
                            as *const libc::c_char,
                    );
                    if 0 == dc_sqlite3_table_exists(
                        sql,
                        b"config\x00" as *const u8 as *const libc::c_char,
                    ) || 0
                        == dc_sqlite3_table_exists(
                            sql,
                            b"contacts\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"chats\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"chats_contacts\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"msgs\x00" as *const u8 as *const libc::c_char,
                        )
                        || 0 == dc_sqlite3_table_exists(
                            sql,
                            b"jobs\x00" as *const u8 as *const libc::c_char,
                        )
                    {
                        dc_sqlite3_log_error(
                            sql,
                            b"Cannot create tables in new database \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            dbfile,
                        );
                        /* cannot create the tables - maybe we cannot write? */
                        current_block = 13628706266672894061;
                    } else {
                        dc_sqlite3_set_config_int(
                            sql,
                            b"dbversion\x00" as *const u8 as *const libc::c_char,
                            0i32,
                        );
                        current_block = 14072441030219150333;
                    }
                } else {
                    exists_before_update = 1i32;
                    dbversion_before_update = dc_sqlite3_get_config_int(
                        sql,
                        b"dbversion\x00" as *const u8 as *const libc::c_char,
                        0i32,
                    );
                    current_block = 14072441030219150333;
                }
                match current_block {
                    13628706266672894061 => {}
                    _ => {
                        // (1) update low-level database structure.
                        // this should be done before updates that use high-level objects that
                        // rely themselves on the low-level structure.
                        // --------------------------------------------------------------------
                        let mut dbversion: libc::c_int = dbversion_before_update;
                        let mut recalc_fingerprints: libc::c_int = 0i32;
                        let mut update_file_paths: libc::c_int = 0i32;
                        if dbversion < 1i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT \'\');\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX leftgrps_index1 ON leftgrps (grpid);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 1i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                1i32,
                            );
                        }
                        if dbversion < 2i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT \'\';\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 2i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                2i32,
                            );
                        }
                        if dbversion < 7i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE keypairs ( id INTEGER PRIMARY KEY, addr TEXT DEFAULT \'\' COLLATE NOCASE, is_default INTEGER DEFAULT 0, private_key, public_key, created INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dbversion = 7i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                7i32,
                            );
                        }
                        if dbversion < 10i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE acpeerstates ( id INTEGER PRIMARY KEY, addr TEXT DEFAULT \'\' COLLATE NOCASE, last_seen INTEGER DEFAULT 0, last_seen_autocrypt INTEGER DEFAULT 0, public_key, prefer_encrypted INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 10i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                10i32,
                            );
                        }
                        if dbversion < 12i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 12i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                12i32,
                            );
                        }
                        if dbversion < 17i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX chats_index2 ON chats (archived);\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX msgs_index5 ON msgs (starred);\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 17i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                17i32,
                            );
                        }
                        if dbversion < 18i32 {
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN gossip_key;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 18i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                18i32,
                            );
                        }
                        if dbversion < 27i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 27i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                27i32,
                            );
                        }
                        if dbversion < 34i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT \'\';\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT \'\';\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            recalc_fingerprints = 1i32;
                            dbversion = 34i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                34i32,
                            );
                        }
                        if dbversion < 39i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT \'\', timestamp INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE acpeerstates ADD COLUMN verified_key;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT \'\';\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            if dbversion_before_update == 34i32 {
                                dc_sqlite3_execute(sql,
                                                   b"UPDATE acpeerstates SET verified_key=gossip_key, verified_key_fingerprint=gossip_key_fingerprint WHERE gossip_key_verified=2;\x00"
                                                       as *const u8 as
                                                       *const libc::c_char);
                                dc_sqlite3_execute(sql,
                                                   b"UPDATE acpeerstates SET verified_key=public_key, verified_key_fingerprint=public_key_fingerprint WHERE public_key_verified=2;\x00"
                                                       as *const u8 as
                                                       *const libc::c_char);
                            }
                            dbversion = 39i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                39i32,
                            );
                        }
                        if dbversion < 40i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 40i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                40i32,
                            );
                        }
                        if dbversion < 41i32 {
                            update_file_paths = 1i32;
                            dbversion = 41i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                41i32,
                            );
                        }
                        if dbversion < 42i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"UPDATE msgs SET txt=\'\' WHERE type!=10\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 42i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                42i32,
                            );
                        }
                        if dbversion < 44i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN mime_headers TEXT;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 44i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                44i32,
                            );
                        }
                        if dbversion < 46i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN mime_references TEXT;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 46i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                46i32,
                            );
                        }
                        if dbversion < 47i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 47i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                47i32,
                            );
                        }
                        if dbversion < 48i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            if 0 != !(DC_MOVE_STATE_UNDEFINED as libc::c_int == 0i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    559i32,
                                    b"DC_MOVE_STATE_UNDEFINED == 0\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_PENDING as libc::c_int == 1i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    560i32,
                                    b"DC_MOVE_STATE_PENDING == 1\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_STAY as libc::c_int == 2i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    561i32,
                                    b"DC_MOVE_STATE_STAY == 2\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            if 0 != !(DC_MOVE_STATE_MOVING as libc::c_int == 3i32) as libc::c_int
                                as libc::c_long
                            {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    562i32,
                                    b"DC_MOVE_STATE_MOVING == 3\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            dbversion = 48i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                48i32,
                            );
                        }
                        if dbversion < 49i32 {
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dbversion = 49i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                49i32,
                            );
                        }
                        if dbversion < 50i32 {
                            if 0 != exists_before_update {
                                dc_sqlite3_set_config_int(
                                    sql,
                                    b"show_emails\x00" as *const u8 as *const libc::c_char,
                                    2i32,
                                );
                            }
                            dbversion = 50i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                50i32,
                            );
                        }
                        if dbversion < 53i32 {
                            dc_sqlite3_execute(sql,
                                               b"CREATE TABLE locations ( id INTEGER PRIMARY KEY AUTOINCREMENT, latitude REAL DEFAULT 0.0, longitude REAL DEFAULT 0.0, accuracy REAL DEFAULT 0.0, timestamp INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0);\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX locations_index1 ON locations (from_id);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX locations_index2 ON locations (timestamp);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(sql,
                                               b"ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX chats_index3 ON chats (locations_send_until);\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 53i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                53i32,
                            );
                        }
                        if dbversion < 54i32 {
                            dc_sqlite3_execute(
                                sql,
                                b"ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;\x00"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                            dc_sqlite3_execute(
                                sql,
                                b"CREATE INDEX msgs_index6 ON msgs (location_id);\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dbversion = 54i32;
                            dc_sqlite3_set_config_int(
                                sql,
                                b"dbversion\x00" as *const u8 as *const libc::c_char,
                                54i32,
                            );
                        }
                        if 0 != recalc_fingerprints {
                            let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
                                sql,
                                b"SELECT addr FROM acpeerstates;\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            while sqlite3_step(stmt) == 100i32 {
                                let mut peerstate: *mut dc_apeerstate_t =
                                    dc_apeerstate_new((*sql).context);
                                if 0 != dc_apeerstate_load_by_addr(
                                    peerstate,
                                    sql,
                                    sqlite3_column_text(stmt, 0i32) as *const libc::c_char,
                                ) && 0 != dc_apeerstate_recalc_fingerprint(peerstate)
                                {
                                    dc_apeerstate_save_to_db(peerstate, sql, 0i32);
                                }
                                dc_apeerstate_unref(peerstate);
                            }
                            sqlite3_finalize(stmt);
                        }
                        if 0 != update_file_paths {
                            let mut repl_from: *mut libc::c_char = dc_sqlite3_get_config(
                                sql,
                                b"backup_for\x00" as *const u8 as *const libc::c_char,
                                (*(*sql).context).blobdir,
                            );
                            dc_ensure_no_slash(repl_from);
                            if 0 != !('f' as i32 == 'f' as i32) as libc::c_int as libc::c_long {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    656i32,
                                    b"\'f\'==DC_PARAM_FILE\x00" as *const u8 as *const libc::c_char,
                                );
                            } else {
                            };
                            let mut q3: *mut libc::c_char =
                                sqlite3_mprintf(b"UPDATE msgs SET param=replace(param, \'f=%q/\', \'f=$BLOBDIR/\');\x00"
                                                    as *const u8 as
                                                    *const libc::c_char,
                                                repl_from);
                            dc_sqlite3_execute(sql, q3);
                            sqlite3_free(q3 as *mut libc::c_void);
                            if 0 != !('i' as i32 == 'i' as i32) as libc::c_int as libc::c_long {
                                __assert_rtn(
                                    (*::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(
                                        b"dc_sqlite3_open\x00",
                                    ))
                                    .as_ptr(),
                                    b"../src/dc_sqlite3.c\x00" as *const u8 as *const libc::c_char,
                                    661i32,
                                    b"\'i\'==DC_PARAM_PROFILE_IMAGE\x00" as *const u8
                                        as *const libc::c_char,
                                );
                            } else {
                            };
                            q3 =
                                sqlite3_mprintf(b"UPDATE chats SET param=replace(param, \'i=%q/\', \'i=$BLOBDIR/\');\x00"
                                                    as *const u8 as
                                                    *const libc::c_char,
                                                repl_from);
                            dc_sqlite3_execute(sql, q3);
                            sqlite3_free(q3 as *mut libc::c_void);
                            free(repl_from as *mut libc::c_void);
                            dc_sqlite3_set_config(
                                sql,
                                b"backup_for\x00" as *const u8 as *const libc::c_char,
                                0 as *const libc::c_char,
                            );
                        }
                        current_block = 12024807525273687499;
                    }
                }
            } else {
                current_block = 12024807525273687499;
            }
            match current_block {
                13628706266672894061 => {}
                _ => {
                    dc_log_info(
                        (*sql).context,
                        0i32,
                        b"Opened \"%s\".\x00" as *const u8 as *const libc::c_char,
                        dbfile,
                    );
                    return 1i32;
                }
            }
        }
    }
    dc_sqlite3_close(sql);
    return 0i32;
}
/* handle configurations, private */
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_set_config(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut value: *const libc::c_char,
) -> libc::c_int {
    let mut state: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if key.is_null() {
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_set_config(): Bad parameter.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if 0 == dc_sqlite3_is_open(sql) {
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_set_config(): Database not ready.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if !value.is_null() {
        stmt = dc_sqlite3_prepare(
            sql,
            b"SELECT value FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        if state == 101i32 {
            stmt = dc_sqlite3_prepare(
                sql,
                b"INSERT INTO config (keyname, value) VALUES (?, ?);\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
            sqlite3_bind_text(stmt, 2i32, value, -1i32, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else if state == 100i32 {
            stmt = dc_sqlite3_prepare(
                sql,
                b"UPDATE config SET value=? WHERE keyname=?;\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_text(stmt, 1i32, value, -1i32, None);
            sqlite3_bind_text(stmt, 2i32, key, -1i32, None);
            state = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
        } else {
            dc_log_error(
                (*sql).context,
                0i32,
                b"dc_sqlite3_set_config(): Cannot read value.\x00" as *const u8
                    as *const libc::c_char,
            );
            return 0i32;
        }
    } else {
        stmt = dc_sqlite3_prepare(
            sql,
            b"DELETE FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
        state = sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    }
    if state != 101i32 {
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_set_config(): Cannot change value.\x00" as *const u8
                as *const libc::c_char,
        );
        return 0i32;
    }
    return 1i32;
}
/* tools, these functions are compatible to the corresponding sqlite3_* functions */
/* the result mus be freed using sqlite3_finalize() */
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_prepare(
    mut sql: *mut dc_sqlite3_t,
    mut querystr: *const libc::c_char,
) -> *mut sqlite3_stmt {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if sql.is_null() || querystr.is_null() || (*sql).cobj.is_null() {
        return 0 as *mut sqlite3_stmt;
    }
    if sqlite3_prepare_v2(
        (*sql).cobj,
        querystr,
        -1i32,
        &mut stmt,
        0 as *mut *const libc::c_char,
    ) != 0i32
    {
        dc_sqlite3_log_error(
            sql,
            b"Query failed: %s\x00" as *const u8 as *const libc::c_char,
            querystr,
        );
        return 0 as *mut sqlite3_stmt;
    }
    return stmt;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_log_error(
    mut sql: *mut dc_sqlite3_t,
    mut msg_format: *const libc::c_char,
    mut va: ...
) {
    let mut msg: *mut libc::c_char = 0 as *mut libc::c_char;
    if sql.is_null() || msg_format.is_null() {
        return;
    }
    msg = sqlite3_vmprintf(msg_format, va);
    dc_log_error(
        (*sql).context,
        0i32,
        b"%s SQLite says: %s\x00" as *const u8 as *const libc::c_char,
        if !msg.is_null() {
            msg
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        if !(*sql).cobj.is_null() {
            sqlite3_errmsg((*sql).cobj)
        } else {
            b"SQLite object not set up.\x00" as *const u8 as *const libc::c_char
        },
    );
    sqlite3_free(msg as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_is_open(mut sql: *const dc_sqlite3_t) -> libc::c_int {
    if sql.is_null() || (*sql).cobj.is_null() {
        return 0i32;
    }
    return 1i32;
}
/* the returned string must be free()'d, returns NULL on errors */
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_config(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut def: *const libc::c_char,
) -> *mut libc::c_char {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if 0 == dc_sqlite3_is_open(sql) || key.is_null() {
        return dc_strdup_keep_null(def);
    }
    stmt = dc_sqlite3_prepare(
        sql,
        b"SELECT value FROM config WHERE keyname=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, key, -1i32, None);
    if sqlite3_step(stmt) == 100i32 {
        let mut ptr: *const libc::c_uchar = sqlite3_column_text(stmt, 0i32);
        if !ptr.is_null() {
            let mut ret: *mut libc::c_char = dc_strdup(ptr as *const libc::c_char);
            sqlite3_finalize(stmt);
            return ret;
        }
    }
    sqlite3_finalize(stmt);
    return dc_strdup_keep_null(def);
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_execute(
    mut sql: *mut dc_sqlite3_t,
    mut querystr: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut sqlState: libc::c_int = 0i32;
    stmt = dc_sqlite3_prepare(sql, querystr);
    if !stmt.is_null() {
        sqlState = sqlite3_step(stmt);
        if sqlState != 101i32 && sqlState != 100i32 {
            dc_sqlite3_log_error(
                sql,
                b"Cannot execute \"%s\".\x00" as *const u8 as *const libc::c_char,
                querystr,
            );
        } else {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_set_config_int(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut value: int32_t,
) -> libc::c_int {
    let mut value_str: *mut libc::c_char = dc_mprintf(
        b"%i\x00" as *const u8 as *const libc::c_char,
        value as libc::c_int,
    );
    if value_str.is_null() {
        return 0i32;
    }
    let mut ret: libc::c_int = dc_sqlite3_set_config(sql, key, value_str);
    free(value_str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_config_int(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut def: int32_t,
) -> int32_t {
    let mut str: *mut libc::c_char = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    if str.is_null() {
        return def;
    }
    let mut ret: int32_t = atol(str) as int32_t;
    free(str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_table_exists(
    mut sql: *mut dc_sqlite3_t,
    mut name: *const libc::c_char,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut querystr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut sqlState: libc::c_int = 0i32;
    querystr = sqlite3_mprintf(
        b"PRAGMA table_info(%s)\x00" as *const u8 as *const libc::c_char,
        name,
    );
    if querystr.is_null() {
        /* this statement cannot be used with binded variables */
        dc_log_error(
            (*sql).context,
            0i32,
            b"dc_sqlite3_table_exists_(): Out of memory.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        stmt = dc_sqlite3_prepare(sql, querystr);
        if !stmt.is_null() {
            sqlState = sqlite3_step(stmt);
            if sqlState == 100i32 {
                ret = 1i32
            }
        }
    }
    /* error/cleanup */
    if !stmt.is_null() {
        sqlite3_finalize(stmt);
    }
    if !querystr.is_null() {
        sqlite3_free(querystr as *mut libc::c_void);
    }
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_set_config_int64(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut value: int64_t,
) -> libc::c_int {
    let mut value_str: *mut libc::c_char = dc_mprintf(
        b"%lld\x00" as *const u8 as *const libc::c_char,
        value as libc::c_long,
    );
    if value_str.is_null() {
        return 0i32;
    }
    let mut ret: libc::c_int = dc_sqlite3_set_config(sql, key, value_str);
    free(value_str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_config_int64(
    mut sql: *mut dc_sqlite3_t,
    mut key: *const libc::c_char,
    mut def: int64_t,
) -> int64_t {
    let mut str: *mut libc::c_char = dc_sqlite3_get_config(sql, key, 0 as *const libc::c_char);
    if str.is_null() {
        return def;
    }
    let mut ret: int64_t = 0i32 as int64_t;
    sscanf(
        str,
        b"%lld\x00" as *const u8 as *const libc::c_char,
        &mut ret as *mut int64_t,
    );
    free(str as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_try_execute(
    mut sql: *mut dc_sqlite3_t,
    mut querystr: *const libc::c_char,
) -> libc::c_int {
    // same as dc_sqlite3_execute() but does not pass error to ui
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut sql_state: libc::c_int = 0i32;
    stmt = dc_sqlite3_prepare(sql, querystr);
    if !stmt.is_null() {
        sql_state = sqlite3_step(stmt);
        if sql_state != 101i32 && sql_state != 100i32 {
            dc_log_warning(
                (*sql).context,
                0i32,
                b"Try-execute for \"%s\" failed: %s\x00" as *const u8 as *const libc::c_char,
                querystr,
                sqlite3_errmsg((*sql).cobj),
            );
        } else {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_rowid(
    mut sql: *mut dc_sqlite3_t,
    mut table: *const libc::c_char,
    mut field: *const libc::c_char,
    mut value: *const libc::c_char,
) -> uint32_t {
    // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    // the ORDER BY ensures, this function always returns the most recent id,
    // eg. if a Message-ID is splitted into different messages.
    let mut id: uint32_t = 0i32 as uint32_t;
    let mut q3: *mut libc::c_char = sqlite3_mprintf(
        b"SELECT id FROM %s WHERE %s=%Q ORDER BY id DESC;\x00" as *const u8 as *const libc::c_char,
        table,
        field,
        value,
    );
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(sql, q3);
    if 100i32 == sqlite3_step(stmt) {
        id = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    return id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_get_rowid2(
    mut sql: *mut dc_sqlite3_t,
    mut table: *const libc::c_char,
    mut field: *const libc::c_char,
    mut value: uint64_t,
    mut field2: *const libc::c_char,
    mut value2: uint32_t,
) -> uint32_t {
    // same as dc_sqlite3_get_rowid() with a key over two columns
    let mut id: uint32_t = 0i32 as uint32_t;
    // see https://www.sqlite.org/printf.html for sqlite-printf modifiers
    let mut q3: *mut libc::c_char = sqlite3_mprintf(
        b"SELECT id FROM %s WHERE %s=%lli AND %s=%i ORDER BY id DESC;\x00" as *const u8
            as *const libc::c_char,
        table,
        field,
        value,
        field2,
        value2,
    );
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(sql, q3);
    if 100i32 == sqlite3_step(stmt) {
        id = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    sqlite3_free(q3 as *mut libc::c_void);
    return id;
}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_begin_transaction(mut sql: *mut dc_sqlite3_t) {}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_commit(mut sql: *mut dc_sqlite3_t) {}
#[no_mangle]
pub unsafe extern "C" fn dc_sqlite3_rollback(mut sql: *mut dc_sqlite3_t) {}
/* housekeeping */
#[no_mangle]
pub unsafe extern "C" fn dc_housekeeping(mut context: *mut dc_context_t) {
    let mut keep_files_newer_than: time_t = 0;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut files_in_use: dc_hash_t = _dc_hash {
        keyClass: 0,
        copyKey: 0,
        count: 0,
        first: 0 as *mut dc_hashelem_t,
        htsize: 0,
        ht: 0 as *mut _ht,
    };
    let mut path: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut unreferenced_count: libc::c_int = 0i32;
    dc_hash_init(&mut files_in_use, 3i32, 1i32);
    dc_log_info(
        context,
        0i32,
        b"Start housekeeping...\x00" as *const u8 as *const libc::c_char,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM msgs  WHERE chat_id!=3   AND type!=10;\x00" as *const u8
            as *const libc::c_char,
        'f' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM jobs;\x00" as *const u8 as *const libc::c_char,
        'f' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM chats;\x00" as *const u8 as *const libc::c_char,
        'i' as i32,
    );
    maybe_add_from_param(
        context,
        &mut files_in_use,
        b"SELECT param FROM contacts;\x00" as *const u8 as *const libc::c_char,
        'i' as i32,
    );
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT value FROM config;\x00" as *const u8 as *const libc::c_char,
    );
    while sqlite3_step(stmt) == 100i32 {
        maybe_add_file(
            &mut files_in_use,
            sqlite3_column_text(stmt, 0i32) as *const libc::c_char,
        );
    }
    dc_log_info(
        context,
        0i32,
        b"%i files in use.\x00" as *const u8 as *const libc::c_char,
        files_in_use.count as libc::c_int,
    );
    /* go through directory and delete unused files */
    dir_handle = opendir((*context).blobdir);
    if dir_handle.is_null() {
        dc_log_warning(
            context,
            0i32,
            b"Housekeeping: Cannot open %s.\x00" as *const u8 as *const libc::c_char,
            (*context).blobdir,
        );
    } else {
        /* avoid deletion of files that are just created to build a message object */
        keep_files_newer_than = time(0 as *mut time_t) - (60i32 * 60i32) as libc::c_long;
        loop {
            dir_entry = readdir(dir_handle);
            if dir_entry.is_null() {
                break;
            }
            /* name without path or `.` or `..` */
            let mut name: *const libc::c_char = (*dir_entry).d_name.as_mut_ptr();
            let mut name_len: libc::c_int = strlen(name) as libc::c_int;
            if name_len == 1i32 && *name.offset(0isize) as libc::c_int == '.' as i32
                || name_len == 2i32
                    && *name.offset(0isize) as libc::c_int == '.' as i32
                    && *name.offset(1isize) as libc::c_int == '.' as i32
            {
                continue;
            }
            if 0 != is_file_in_use(&mut files_in_use, 0 as *const libc::c_char, name)
                || 0 != is_file_in_use(
                    &mut files_in_use,
                    b".increation\x00" as *const u8 as *const libc::c_char,
                    name,
                )
                || 0 != is_file_in_use(
                    &mut files_in_use,
                    b".waveform\x00" as *const u8 as *const libc::c_char,
                    name,
                )
                || 0 != is_file_in_use(
                    &mut files_in_use,
                    b"-preview.jpg\x00" as *const u8 as *const libc::c_char,
                    name,
                )
            {
                continue;
            }
            unreferenced_count += 1;
            free(path as *mut libc::c_void);
            path = dc_mprintf(
                b"%s/%s\x00" as *const u8 as *const libc::c_char,
                (*context).blobdir,
                name,
            );
            let mut st: stat = stat {
                st_dev: 0,
                st_mode: 0,
                st_nlink: 0,
                st_ino: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                st_atimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_mtimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_ctimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_birthtimespec: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_size: 0,
                st_blocks: 0,
                st_blksize: 0,
                st_flags: 0,
                st_gen: 0,
                st_lspare: 0,
                st_qspare: [0; 2],
            };
            if stat(path, &mut st) == 0i32 {
                if st.st_mtimespec.tv_sec > keep_files_newer_than
                    || st.st_atimespec.tv_sec > keep_files_newer_than
                    || st.st_ctimespec.tv_sec > keep_files_newer_than
                {
                    dc_log_info(
                        context,
                        0i32,
                        b"Housekeeping: Keeping new unreferenced file #%i: %s\x00" as *const u8
                            as *const libc::c_char,
                        unreferenced_count,
                        name,
                    );
                    continue;
                }
            }
            dc_log_info(
                context,
                0i32,
                b"Housekeeping: Deleting unreferenced file #%i: %s\x00" as *const u8
                    as *const libc::c_char,
                unreferenced_count,
                name,
            );
            dc_delete_file(context, path);
        }
    }
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    sqlite3_finalize(stmt);
    dc_hash_clear(&mut files_in_use);
    free(path as *mut libc::c_void);
    dc_log_info(
        context,
        0i32,
        b"Housekeeping done.\x00" as *const u8 as *const libc::c_char,
    );
}
unsafe extern "C" fn is_file_in_use(
    mut files_in_use: *mut dc_hash_t,
    mut namespc: *const libc::c_char,
    mut name: *const libc::c_char,
) -> libc::c_int {
    let mut name_to_check: *mut libc::c_char = dc_strdup(name);
    if !namespc.is_null() {
        let mut name_len: libc::c_int = strlen(name) as libc::c_int;
        let mut namespc_len: libc::c_int = strlen(namespc) as libc::c_int;
        if name_len <= namespc_len
            || strcmp(&*name.offset((name_len - namespc_len) as isize), namespc) != 0i32
        {
            return 0i32;
        }
        *name_to_check.offset((name_len - namespc_len) as isize) = 0i32 as libc::c_char
    }
    let mut ret: libc::c_int = (dc_hash_find(
        files_in_use,
        name_to_check as *const libc::c_void,
        strlen(name_to_check) as libc::c_int,
    ) != 0 as *mut libc::c_void) as libc::c_int;
    free(name_to_check as *mut libc::c_void);
    return ret;
}
/* ******************************************************************************
 * Housekeeping
 ******************************************************************************/
unsafe extern "C" fn maybe_add_file(
    mut files_in_use: *mut dc_hash_t,
    mut file: *const libc::c_char,
) {
    if strncmp(
        file,
        b"$BLOBDIR/\x00" as *const u8 as *const libc::c_char,
        9i32 as libc::c_ulong,
    ) != 0i32
    {
        return;
    }
    let mut raw_name: *const libc::c_char = &*file.offset(9isize) as *const libc::c_char;
    dc_hash_insert(
        files_in_use,
        raw_name as *const libc::c_void,
        strlen(raw_name) as libc::c_int,
        1i32 as *mut libc::c_void,
    );
}
unsafe extern "C" fn maybe_add_from_param(
    mut context: *mut dc_context_t,
    mut files_in_use: *mut dc_hash_t,
    mut query: *const libc::c_char,
    mut param_id: libc::c_int,
) {
    let mut param: *mut dc_param_t = dc_param_new();
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare((*context).sql, query);
    while sqlite3_step(stmt) == 100i32 {
        dc_param_set_packed(
            param,
            sqlite3_column_text(stmt, 0i32) as *const libc::c_char,
        );
        let mut file: *mut libc::c_char = dc_param_get(param, param_id, 0 as *const libc::c_char);
        if !file.is_null() {
            maybe_add_file(files_in_use, file);
            free(file as *mut libc::c_void);
        }
    }
    sqlite3_finalize(stmt);
    dc_param_unref(param);
}

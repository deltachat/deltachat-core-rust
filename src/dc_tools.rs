use c2rust_bitfields::BitfieldStruct;
use libc;

extern "C" {
    pub type mailstream_cancel;
    pub type __sFILEX;
    pub type sqlite3;
    #[no_mangle]
    static mut _DefaultRuneLocale: _RuneLocale;
    #[no_mangle]
    pub fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;

    #[no_mangle]
    fn __maskrune(_: __darwin_ct_rune_t, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn __tolower(_: __darwin_ct_rune_t) -> __darwin_ct_rune_t;
    #[no_mangle]
    fn open(_: *const libc::c_char, _: libc::c_int, _: ...) -> libc::c_int;
    #[no_mangle]
    fn close(_: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn read(_: libc::c_int, _: *mut libc::c_void, _: size_t) -> ssize_t;
    #[no_mangle]
    fn write(__fd: libc::c_int, __buf: *const libc::c_void, __nbyte: size_t) -> ssize_t;
    #[no_mangle]
    fn mkdir(_: *const libc::c_char, _: mode_t) -> libc::c_int;
    #[no_mangle]
    fn stat(_: *const libc::c_char, _: *mut stat) -> libc::c_int;
    #[no_mangle]
    fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn atof(_: *const libc::c_char) -> libc::c_double;
    #[no_mangle]
    fn atoi(_: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn RAND_bytes(buf: *mut libc::c_uchar, num: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn RAND_pseudo_bytes(buf: *mut libc::c_uchar, num: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn localtime(_: *const time_t) -> *mut tm;
    #[no_mangle]
    fn time(_: *mut time_t) -> time_t;
    #[no_mangle]
    fn gmtime_r(_: *const time_t, _: *mut tm) -> *mut tm;
    #[no_mangle]
    fn localtime_r(_: *const time_t, _: *mut tm) -> *mut tm;
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
    fn clist_new() -> *mut clist;
    #[no_mangle]
    fn clist_insert_after(_: *mut clist, _: *mut clistiter, _: *mut libc::c_void) -> libc::c_int;
    #[no_mangle]
    fn fclose(_: *mut FILE) -> libc::c_int;
    #[no_mangle]
    fn fopen(_: *const libc::c_char, _: *const libc::c_char) -> *mut FILE;
    #[no_mangle]
    fn fread(
        _: *mut libc::c_void,
        _: libc::c_ulong,
        _: libc::c_ulong,
        _: *mut FILE,
    ) -> libc::c_ulong;
    #[no_mangle]
    fn fseek(_: *mut FILE, _: libc::c_long, _: libc::c_int) -> libc::c_int;
    #[no_mangle]
    fn ftell(_: *mut FILE) -> libc::c_long;
    #[no_mangle]
    fn fwrite(
        _: *const libc::c_void,
        _: libc::c_ulong,
        _: libc::c_ulong,
        _: *mut FILE,
    ) -> libc::c_ulong;
    #[no_mangle]
    fn remove(_: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn snprintf(
        _: *mut libc::c_char,
        _: libc::c_ulong,
        _: *const libc::c_char,
        _: ...
    ) -> libc::c_int;
    #[no_mangle]
    fn vsnprintf(
        _: *mut libc::c_char,
        _: libc::c_ulong,
        _: *const libc::c_char,
        _: ::std::ffi::VaList,
    ) -> libc::c_int;
    #[no_mangle]
    fn mailimap_date_time_new(
        dt_day: libc::c_int,
        dt_month: libc::c_int,
        dt_year: libc::c_int,
        dt_hour: libc::c_int,
        dt_min: libc::c_int,
        dt_sec: libc::c_int,
        dt_zone: libc::c_int,
    ) -> *mut mailimap_date_time;
    #[no_mangle]
    fn memcpy(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn memmove(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong)
        -> *mut libc::c_void;
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strcat(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong) -> libc::c_int;
    #[no_mangle]
    fn strncpy(_: *mut libc::c_char, _: *const libc::c_char, _: libc::c_ulong)
        -> *mut libc::c_char;
    #[no_mangle]
    fn strrchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    #[no_mangle]
    fn strstr(_: *const libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strndup(_: *const libc::c_char, _: libc::c_ulong) -> *mut libc::c_char;
    #[no_mangle]
    fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_lock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn pthread_mutex_unlock(_: *mut pthread_mutex_t) -> libc::c_int;
    #[no_mangle]
    fn dc_array_get_cnt(_: *const dc_array_t) -> size_t;
    #[no_mangle]
    fn dc_array_get_id(_: *const dc_array_t, index: size_t) -> uint32_t;
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_log_warning(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
    #[no_mangle]
    fn dc_log_error(_: *mut dc_context_t, data1: libc::c_int, msg: *const libc::c_char, _: ...);
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
pub type va_list = __builtin_va_list;
pub type __uint16_t = libc::c_ushort;
pub type __int32_t = libc::c_int;
pub type __uint32_t = libc::c_uint;
pub type __int64_t = libc::c_longlong;
pub type __uint64_t = libc::c_ulonglong;
pub type __darwin_ct_rune_t = libc::c_int;
pub type __darwin_size_t = libc::c_ulong;
pub type __darwin_wchar_t = libc::c_int;
pub type __darwin_rune_t = __darwin_wchar_t;
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
pub type size_t = __darwin_size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneEntry {
    pub __min: __darwin_rune_t,
    pub __max: __darwin_rune_t,
    pub __map: __darwin_rune_t,
    pub __types: *mut __uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneRange {
    pub __nranges: libc::c_int,
    pub __ranges: *mut _RuneEntry,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneCharClass {
    pub __name: [libc::c_char; 14],
    pub __mask: __uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _RuneLocale {
    pub __magic: [libc::c_char; 8],
    pub __encoding: [libc::c_char; 32],
    pub __sgetrune: Option<
        unsafe extern "C" fn(
            _: *const libc::c_char,
            _: __darwin_size_t,
            _: *mut *const libc::c_char,
        ) -> __darwin_rune_t,
    >,
    pub __sputrune: Option<
        unsafe extern "C" fn(
            _: __darwin_rune_t,
            _: *mut libc::c_char,
            _: __darwin_size_t,
            _: *mut *mut libc::c_char,
        ) -> libc::c_int,
    >,
    pub __invalid_rune: __darwin_rune_t,
    pub __runetype: [__uint32_t; 256],
    pub __maplower: [__darwin_rune_t; 256],
    pub __mapupper: [__darwin_rune_t; 256],
    pub __runetype_ext: _RuneRange,
    pub __maplower_ext: _RuneRange,
    pub __mapupper_ext: _RuneRange,
    pub __variable: *mut libc::c_void,
    pub __variable_len: libc::c_int,
    pub __ncharclasses: libc::c_int,
    pub __charclasses: *mut _RuneCharClass,
}
pub type mode_t = __darwin_mode_t;
pub type off_t = __darwin_off_t;
pub type uintptr_t = libc::c_ulong;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __darwin_time_t,
    pub tv_nsec: libc::c_long,
}
pub type uint64_t = libc::c_ulonglong;
pub type uint32_t = libc::c_uint;
pub type ssize_t = __darwin_ssize_t;
pub type uid_t = __darwin_uid_t;
pub type gid_t = __darwin_gid_t;
pub type time_t = __darwin_time_t;
pub type dev_t = __darwin_dev_t;
pub type blkcnt_t = __darwin_blkcnt_t;
pub type blksize_t = __darwin_blksize_t;
pub type nlink_t = __uint16_t;
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
pub type pthread_cond_t = __darwin_pthread_cond_t;
pub type pthread_mutex_t = __darwin_pthread_mutex_t;
pub type uint8_t = libc::c_uchar;
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
pub type fpos_t = __darwin_off_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sbuf {
    pub _base: *mut libc::c_uchar,
    pub _size: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sFILE {
    pub _p: *mut libc::c_uchar,
    pub _r: libc::c_int,
    pub _w: libc::c_int,
    pub _flags: libc::c_short,
    pub _file: libc::c_short,
    pub _bf: __sbuf,
    pub _lbfsize: libc::c_int,
    pub _cookie: *mut libc::c_void,
    pub _close: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> libc::c_int>,
    pub _read: Option<
        unsafe extern "C" fn(
            _: *mut libc::c_void,
            _: *mut libc::c_char,
            _: libc::c_int,
        ) -> libc::c_int,
    >,
    pub _seek:
        Option<unsafe extern "C" fn(_: *mut libc::c_void, _: fpos_t, _: libc::c_int) -> fpos_t>,
    pub _write: Option<
        unsafe extern "C" fn(
            _: *mut libc::c_void,
            _: *const libc::c_char,
            _: libc::c_int,
        ) -> libc::c_int,
    >,
    pub _ub: __sbuf,
    pub _extra: *mut __sFILEX,
    pub _ur: libc::c_int,
    pub _ubuf: [libc::c_uchar; 3],
    pub _nbuf: [libc::c_uchar; 1],
    pub _lb: __sbuf,
    pub _blksize: libc::c_int,
    pub _offset: fpos_t,
}
pub type FILE = __sFILE;
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
pub struct mailimap_date_time {
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
pub type dc_strbuilder_t = _dc_strbuilder;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
#[inline]
unsafe extern "C" fn isascii(mut _c: libc::c_int) -> libc::c_int {
    return (_c & !0x7fi32 == 0i32) as libc::c_int;
}
#[inline]
unsafe extern "C" fn __istype(mut _c: __darwin_ct_rune_t, mut _f: libc::c_ulong) -> libc::c_int {
    return if 0 != isascii(_c) {
        (0 != _DefaultRuneLocale.__runetype[_c as usize] as libc::c_ulong & _f) as libc::c_int
    } else {
        (0 != __maskrune(_c, _f)) as libc::c_int
    };
}
#[no_mangle]
#[inline]
pub unsafe extern "C" fn isspace(mut _c: libc::c_int) -> libc::c_int {
    return __istype(_c, 0x4000i64 as libc::c_ulong);
}
#[no_mangle]
#[inline]
pub unsafe extern "C" fn tolower(mut _c: libc::c_int) -> libc::c_int {
    return __tolower(_c);
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
/* Some tools and enhancements to the used libraries, there should be
no references to dc_context_t and other "larger" classes here. */
// for carray etc.
/* ** library-private **********************************************************/
/* math tools */
#[no_mangle]
pub unsafe extern "C" fn dc_exactly_one_bit_set(mut v: libc::c_int) -> libc::c_int {
    return (0 != v && 0 == v & v - 1i32) as libc::c_int;
}
/* string tools */
/* dc_strdup() returns empty string if NULL is given, never returns NULL (exits on errors) */
#[no_mangle]
pub unsafe extern "C" fn dc_strdup(mut s: *const libc::c_char) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !s.is_null() {
        ret = strdup(s);
        if ret.is_null() {
            exit(16i32);
        }
    } else {
        ret = calloc(1i32 as libc::c_ulong, 1i32 as libc::c_ulong) as *mut libc::c_char;
        if ret.is_null() {
            exit(17i32);
        }
    }
    return ret;
}
/* strdup(NULL) is undefined, safe_strdup_keep_null(NULL) returns NULL in this case */
#[no_mangle]
pub unsafe extern "C" fn dc_strdup_keep_null(mut s: *const libc::c_char) -> *mut libc::c_char {
    return if !s.is_null() {
        dc_strdup(s)
    } else {
        0 as *mut libc::c_char
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_atoi_null_is_0(mut s: *const libc::c_char) -> libc::c_int {
    return if !s.is_null() { atoi(s) } else { 0i32 };
}
#[no_mangle]
pub unsafe extern "C" fn dc_atof(mut str: *const libc::c_char) -> libc::c_double {
    // hack around atof() that may accept only `,` as decimal point on mac
    let mut test: *mut libc::c_char =
        dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, 1.2f64);
    *test.offset(2isize) = 0i32 as libc::c_char;
    let mut str_locale: *mut libc::c_char = dc_strdup(str);
    dc_str_replace(
        &mut str_locale,
        b".\x00" as *const u8 as *const libc::c_char,
        test.offset(1isize),
    );
    let mut f: libc::c_double = atof(str_locale);
    free(test as *mut libc::c_void);
    free(str_locale as *mut libc::c_void);
    return f;
}
#[no_mangle]
pub unsafe extern "C" fn dc_str_replace(
    mut haystack: *mut *mut libc::c_char,
    mut needle: *const libc::c_char,
    mut replacement: *const libc::c_char,
) -> libc::c_int {
    let mut replacements: libc::c_int = 0i32;
    let mut start_search_pos: libc::c_int = 0i32;
    let mut needle_len: libc::c_int = 0i32;
    let mut replacement_len: libc::c_int = 0i32;
    if haystack.is_null()
        || (*haystack).is_null()
        || needle.is_null()
        || *needle.offset(0isize) as libc::c_int == 0i32
    {
        return 0i32;
    }
    needle_len = strlen(needle) as libc::c_int;
    replacement_len = (if !replacement.is_null() {
        strlen(replacement)
    } else {
        0i32 as libc::c_ulong
    }) as libc::c_int;
    loop {
        let mut p2: *mut libc::c_char =
            strstr((*haystack).offset(start_search_pos as isize), needle);
        if p2.is_null() {
            break;
        }
        start_search_pos = (p2.wrapping_offset_from(*haystack) as libc::c_long
            + replacement_len as libc::c_long) as libc::c_int;
        *p2 = 0i32 as libc::c_char;
        p2 = p2.offset(needle_len as isize);
        let mut new_string: *mut libc::c_char = dc_mprintf(
            b"%s%s%s\x00" as *const u8 as *const libc::c_char,
            *haystack,
            if !replacement.is_null() {
                replacement
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            p2,
        );
        free(*haystack as *mut libc::c_void);
        *haystack = new_string;
        replacements += 1
    }
    return replacements;
}
#[no_mangle]
pub unsafe extern "C" fn dc_ftoa(mut f: libc::c_double) -> *mut libc::c_char {
    // hack around printf(%f) that may return `,` as decimal point on mac
    let mut test: *mut libc::c_char =
        dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, 1.2f64);
    *test.offset(2isize) = 0i32 as libc::c_char;
    let mut str: *mut libc::c_char = dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, f);
    dc_str_replace(
        &mut str,
        test.offset(1isize),
        b".\x00" as *const u8 as *const libc::c_char,
    );
    free(test as *mut libc::c_void);
    return str;
}
#[no_mangle]
pub unsafe extern "C" fn dc_ltrim(mut buf: *mut libc::c_char) {
    let mut len: size_t = 0i32 as size_t;
    let mut cur: *const libc::c_uchar = 0 as *const libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = buf as *const libc::c_uchar;
        while 0 != *cur as libc::c_int && 0 != isspace(*cur as libc::c_int) {
            cur = cur.offset(1isize);
            len = len.wrapping_sub(1)
        }
        if buf as *const libc::c_uchar != cur {
            memmove(
                buf as *mut libc::c_void,
                cur as *const libc::c_void,
                len.wrapping_add(1i32 as libc::c_ulong),
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_rtrim(mut buf: *mut libc::c_char) {
    let mut len: size_t = 0i32 as size_t;
    let mut cur: *mut libc::c_uchar = 0 as *mut libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = (buf as *mut libc::c_uchar)
            .offset(len as isize)
            .offset(-1isize);
        while cur != buf as *mut libc::c_uchar && 0 != isspace(*cur as libc::c_int) {
            cur = cur.offset(-1isize);
            len = len.wrapping_sub(1)
        }
        *cur.offset(
            (if 0 != isspace(*cur as libc::c_int) {
                0i32
            } else {
                1i32
            }) as isize,
        ) = '\u{0}' as i32 as libc::c_uchar
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_trim(mut buf: *mut libc::c_char) {
    dc_ltrim(buf);
    dc_rtrim(buf);
}
/* the result must be free()'d */
#[no_mangle]
pub unsafe extern "C" fn dc_strlower(mut in_0: *const libc::c_char) -> *mut libc::c_char {
    let mut out: *mut libc::c_char = dc_strdup(in_0);
    let mut p: *mut libc::c_char = out;
    while 0 != *p {
        *p = tolower(*p as libc::c_int) as libc::c_char;
        p = p.offset(1isize)
    }
    return out;
}
#[no_mangle]
pub unsafe extern "C" fn dc_strlower_in_place(mut in_0: *mut libc::c_char) {
    let mut p: *mut libc::c_char = in_0;
    while 0 != *p {
        *p = tolower(*p as libc::c_int) as libc::c_char;
        p = p.offset(1isize)
    }
}
#[no_mangle]
pub unsafe extern "C" fn dc_str_contains(
    mut haystack: *const libc::c_char,
    mut needle: *const libc::c_char,
) -> libc::c_int {
    if haystack.is_null() || needle.is_null() {
        return 0i32;
    }
    if !strstr(haystack, needle).is_null() {
        return 1i32;
    }
    let mut haystack_lower: *mut libc::c_char = dc_strlower(haystack);
    let mut needle_lower: *mut libc::c_char = dc_strlower(needle);
    let mut ret: libc::c_int = if !strstr(haystack_lower, needle_lower).is_null() {
        1i32
    } else {
        0i32
    };
    free(haystack_lower as *mut libc::c_void);
    free(needle_lower as *mut libc::c_void);
    return ret;
}
/* the result must be free()'d */
#[no_mangle]
pub unsafe extern "C" fn dc_null_terminate(
    mut in_0: *const libc::c_char,
    mut bytes: libc::c_int,
) -> *mut libc::c_char {
    let mut out: *mut libc::c_char = malloc((bytes + 1i32) as libc::c_ulong) as *mut libc::c_char;
    if out.is_null() {
        exit(45i32);
    }
    if !in_0.is_null() && bytes > 0i32 {
        strncpy(out, in_0, bytes as libc::c_ulong);
    }
    *out.offset(bytes as isize) = 0i32 as libc::c_char;
    return out;
}
#[no_mangle]
pub unsafe extern "C" fn dc_binary_to_uc_hex(
    mut buf: *const uint8_t,
    mut bytes: size_t,
) -> *mut libc::c_char {
    let mut hex: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut i: libc::c_int = 0i32;
    if !(buf.is_null() || bytes <= 0i32 as libc::c_ulong) {
        hex = calloc(
            ::std::mem::size_of::<libc::c_char>() as libc::c_ulong,
            bytes
                .wrapping_mul(2i32 as libc::c_ulong)
                .wrapping_add(1i32 as libc::c_ulong),
        ) as *mut libc::c_char;
        if !hex.is_null() {
            i = 0i32;
            while (i as libc::c_ulong) < bytes {
                snprintf(
                    &mut *hex.offset((i * 2i32) as isize) as *mut libc::c_char,
                    3i32 as libc::c_ulong,
                    b"%02X\x00" as *const u8 as *const libc::c_char,
                    *buf.offset(i as isize) as libc::c_int,
                );
                i += 1
            }
        }
    }
    return hex;
}
/* remove all \r characters from string */
#[no_mangle]
pub unsafe extern "C" fn dc_remove_cr_chars(mut buf: *mut libc::c_char) {
    /* search for first `\r` */
    let mut p1: *const libc::c_char = buf;
    while 0 != *p1 {
        if *p1 as libc::c_int == '\r' as i32 {
            break;
        }
        p1 = p1.offset(1isize)
    }
    /* p1 is `\r` or null-byte; start removing `\r` */
    let mut p2: *mut libc::c_char = p1 as *mut libc::c_char;
    while 0 != *p1 {
        if *p1 as libc::c_int != '\r' as i32 {
            *p2 = *p1;
            p2 = p2.offset(1isize)
        }
        p1 = p1.offset(1isize)
    }
    *p2 = 0i32 as libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn dc_unify_lineends(mut buf: *mut libc::c_char) {
    dc_remove_cr_chars(buf);
}
/* replace bad UTF-8 characters by sequences of `_` (to avoid problems in filenames, we do not use eg. `?`) the function is useful if strings are unexpectingly encoded eg. as ISO-8859-1 */
#[no_mangle]
pub unsafe extern "C" fn dc_replace_bad_utf8_chars(mut buf: *mut libc::c_char) {
    let mut current_block: u64;
    if buf.is_null() {
        return;
    }
    /* force unsigned - otherwise the `> ' '` comparison will fail */
    let mut p1: *mut libc::c_uchar = buf as *mut libc::c_uchar;
    let mut p1len: libc::c_int = strlen(buf) as libc::c_int;
    let mut c: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    let mut ix: libc::c_int = 0i32;
    let mut n: libc::c_int = 0i32;
    let mut j: libc::c_int = 0i32;
    i = 0i32;
    ix = p1len;
    's_36: loop {
        if !(i < ix) {
            current_block = 13550086250199790493;
            break;
        }
        c = *p1.offset(i as isize) as libc::c_int;
        if c > 0i32 && c <= 0x7fi32 {
            n = 0i32
        } else if c & 0xe0i32 == 0xc0i32 {
            n = 1i32
        } else if c == 0xedi32
            && i < ix - 1i32
            && *p1.offset((i + 1i32) as isize) as libc::c_int & 0xa0i32 == 0xa0i32
        {
            /* U+d800 to U+dfff */
            current_block = 2775201239069267972;
            break;
        } else if c & 0xf0i32 == 0xe0i32 {
            n = 2i32
        } else if c & 0xf8i32 == 0xf0i32 {
            n = 3i32
        } else {
            //else if ((c & 0xFC) == 0xF8)                          { n=4; }        /* 111110bb - not valid in https://tools.ietf.org/html/rfc3629 */
            //else if ((c & 0xFE) == 0xFC)                          { n=5; }        /* 1111110b - not valid in https://tools.ietf.org/html/rfc3629 */
            current_block = 2775201239069267972;
            break;
        }
        j = 0i32;
        while j < n && i < ix {
            /* n bytes matching 10bbbbbb follow ? */
            i += 1;
            if i == ix || *p1.offset(i as isize) as libc::c_int & 0xc0i32 != 0x80i32 {
                current_block = 2775201239069267972;
                break 's_36;
            }
            j += 1
        }
        i += 1
    }
    match current_block {
        13550086250199790493 => return,
        _ => {
            while 0 != *p1 {
                if *p1 as libc::c_int > 0x7fi32 {
                    *p1 = '_' as i32 as libc::c_uchar
                }
                p1 = p1.offset(1isize)
            }
            return;
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_utf8_strlen(mut s: *const libc::c_char) -> size_t {
    if s.is_null() {
        return 0i32 as size_t;
    }
    let mut i: size_t = 0i32 as size_t;
    let mut j: size_t = 0i32 as size_t;
    while 0 != *s.offset(i as isize) {
        if *s.offset(i as isize) as libc::c_int & 0xc0i32 != 0x80i32 {
            j = j.wrapping_add(1)
        }
        i = i.wrapping_add(1)
    }
    return j;
}
#[no_mangle]
pub unsafe extern "C" fn dc_truncate_str(
    mut buf: *mut libc::c_char,
    mut approx_chars: libc::c_int,
) {
    if approx_chars > 0i32
        && strlen(buf)
            > (approx_chars as libc::c_ulong)
                .wrapping_add(strlen(b"[...]\x00" as *const u8 as *const libc::c_char))
    {
        let mut p: *mut libc::c_char = &mut *buf.offset(approx_chars as isize) as *mut libc::c_char;
        *p = 0i32 as libc::c_char;
        if !strchr(buf, ' ' as i32).is_null() {
            while *p.offset(-1i32 as isize) as libc::c_int != ' ' as i32
                && *p.offset(-1i32 as isize) as libc::c_int != '\n' as i32
            {
                p = p.offset(-1isize);
                *p = 0i32 as libc::c_char
            }
        }
        strcat(p, b"[...]\x00" as *const u8 as *const libc::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_truncate_n_unwrap_str(
    mut buf: *mut libc::c_char,
    mut approx_characters: libc::c_int,
    mut do_unwrap: libc::c_int,
) {
    /* Function unwraps the given string and removes unnecessary whitespace.
    Function stops processing after approx_characters are processed.
    (as we're using UTF-8, for simplicity, we cut the string only at whitespaces). */
    /* a single line is truncated `...` instead of `[...]` (the former is typically also used by the UI to fit strings in a rectangle) */
    let mut ellipse_utf8: *const libc::c_char = if 0 != do_unwrap {
        b" ...\x00" as *const u8 as *const libc::c_char
    } else {
        b" [...]\x00" as *const u8 as *const libc::c_char
    };
    let mut lastIsCharacter: libc::c_int = 0i32;
    /* force unsigned - otherwise the `> ' '` comparison will fail */
    let mut p1: *mut libc::c_uchar = buf as *mut libc::c_uchar;
    while 0 != *p1 {
        if *p1 as libc::c_int > ' ' as i32 {
            lastIsCharacter = 1i32
        } else if 0 != lastIsCharacter {
            let mut used_bytes: size_t = (p1 as uintptr_t).wrapping_sub(buf as uintptr_t) as size_t;
            if dc_utf8_strnlen(buf, used_bytes) >= approx_characters as libc::c_ulong {
                let mut buf_bytes: size_t = strlen(buf);
                if buf_bytes.wrapping_sub(used_bytes) >= strlen(ellipse_utf8) {
                    strcpy(p1 as *mut libc::c_char, ellipse_utf8);
                }
                break;
            } else {
                lastIsCharacter = 0i32;
                if 0 != do_unwrap {
                    *p1 = ' ' as i32 as libc::c_uchar
                }
            }
        } else if 0 != do_unwrap {
            *p1 = '\r' as i32 as libc::c_uchar
        }
        p1 = p1.offset(1isize)
    }
    if 0 != do_unwrap {
        dc_remove_cr_chars(buf);
    };
}
unsafe extern "C" fn dc_utf8_strnlen(mut s: *const libc::c_char, mut n: size_t) -> size_t {
    if s.is_null() {
        return 0i32 as size_t;
    }
    let mut i: size_t = 0i32 as size_t;
    let mut j: size_t = 0i32 as size_t;
    while i < n {
        if *s.offset(i as isize) as libc::c_int & 0xc0i32 != 0x80i32 {
            j = j.wrapping_add(1)
        }
        i = i.wrapping_add(1)
    }
    return j;
}
/* split string into lines*/
#[no_mangle]
pub unsafe extern "C" fn dc_split_into_lines(
    mut buf_terminated: *const libc::c_char,
) -> *mut carray {
    let mut lines: *mut carray = carray_new(1024i32 as libc::c_uint);
    let mut line_chars: size_t = 0i32 as size_t;
    let mut p1: *const libc::c_char = buf_terminated;
    let mut line_start: *const libc::c_char = p1;
    let mut l_indx: libc::c_uint = 0i32 as libc::c_uint;
    while 0 != *p1 {
        if *p1 as libc::c_int == '\n' as i32 {
            carray_add(
                lines,
                strndup(line_start, line_chars) as *mut libc::c_void,
                &mut l_indx,
            );
            p1 = p1.offset(1isize);
            line_start = p1;
            line_chars = 0i32 as size_t
        } else {
            p1 = p1.offset(1isize);
            line_chars = line_chars.wrapping_add(1)
        }
    }
    carray_add(
        lines,
        strndup(line_start, line_chars) as *mut libc::c_void,
        &mut l_indx,
    );
    return lines;
}
#[no_mangle]
pub unsafe extern "C" fn dc_free_splitted_lines(mut lines: *mut carray) {
    if !lines.is_null() {
        let mut i: libc::c_int = 0;
        let mut cnt: libc::c_int = carray_count(lines) as libc::c_int;
        i = 0i32;
        while i < cnt {
            free(carray_get(lines, i as libc::c_uint));
            i += 1
        }
        carray_free(lines);
    };
}
/* insert a break every n characters, the return must be free()'d */
#[no_mangle]
pub unsafe extern "C" fn dc_insert_breaks(
    mut in_0: *const libc::c_char,
    mut break_every: libc::c_int,
    mut break_chars: *const libc::c_char,
) -> *mut libc::c_char {
    if in_0.is_null() || break_every <= 0i32 || break_chars.is_null() {
        return dc_strdup(in_0);
    }
    let mut out_len: libc::c_int = strlen(in_0) as libc::c_int;
    let mut chars_added: libc::c_int = 0i32;
    let mut break_chars_len: libc::c_int = strlen(break_chars) as libc::c_int;
    out_len += (out_len / break_every + 1i32) * break_chars_len + 1i32;
    let mut out: *mut libc::c_char = malloc(out_len as libc::c_ulong) as *mut libc::c_char;
    if out.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut i: *const libc::c_char = in_0;
    let mut o: *mut libc::c_char = out;
    while 0 != *i {
        let fresh1 = o;
        o = o.offset(1);
        let fresh0 = i;
        i = i.offset(1);
        *fresh1 = *fresh0;
        chars_added += 1;
        if chars_added == break_every && 0 != *i as libc::c_int {
            strcpy(o, break_chars);
            o = o.offset(break_chars_len as isize);
            chars_added = 0i32
        }
    }
    *o = 0i32 as libc::c_char;
    return out;
}
#[no_mangle]
pub unsafe extern "C" fn dc_str_from_clist(
    mut list: *const clist,
    mut delimiter: *const libc::c_char,
) -> *mut libc::c_char {
    let mut str: dc_strbuilder_t = _dc_strbuilder {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut str, 256i32);
    if !list.is_null() {
        let mut cur: *mut clistiter = (*list).first;
        while !cur.is_null() {
            let mut rfc724_mid: *const libc::c_char = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *const libc::c_char;
            if !rfc724_mid.is_null() {
                if 0 != *str.buf.offset(0isize) as libc::c_int && !delimiter.is_null() {
                    dc_strbuilder_cat(&mut str, delimiter);
                }
                dc_strbuilder_cat(&mut str, rfc724_mid);
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell_s
            }
        }
    }
    return str.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_str_to_clist(
    mut str: *const libc::c_char,
    mut delimiter: *const libc::c_char,
) -> *mut clist {
    let mut list: *mut clist = clist_new();
    if list.is_null() {
        exit(54i32);
    }
    if !str.is_null() && !delimiter.is_null() && strlen(delimiter) >= 1i32 as libc::c_ulong {
        let mut p1: *const libc::c_char = str;
        loop {
            let mut p2: *const libc::c_char = strstr(p1, delimiter);
            if p2.is_null() {
                clist_insert_after(list, (*list).last, strdup(p1) as *mut libc::c_void);
                break;
            } else {
                clist_insert_after(
                    list,
                    (*list).last,
                    strndup(
                        p1,
                        p2.wrapping_offset_from(p1) as libc::c_long as libc::c_ulong,
                    ) as *mut libc::c_void,
                );
                p1 = p2.offset(strlen(delimiter) as isize)
            }
        }
    }
    return list;
}
#[no_mangle]
pub unsafe extern "C" fn dc_str_to_color(mut str: *const libc::c_char) -> libc::c_int {
    let mut str_lower: *mut libc::c_char = dc_strlower(str);
    /* the colors must fulfill some criterions as:
    - contrast to black and to white
    - work as a text-color
    - being noticable on a typical map
    - harmonize together while being different enough
    (therefore, we cannot just use random rgb colors :) */
    static mut colors: [uint32_t; 16] = [
        0xe56555i32 as uint32_t,
        0xf28c48i32 as uint32_t,
        0x8e85eei32 as uint32_t,
        0x76c84di32 as uint32_t,
        0x5bb6cci32 as uint32_t,
        0x549cddi32 as uint32_t,
        0xd25c99i32 as uint32_t,
        0xb37800i32 as uint32_t,
        0xf23030i32 as uint32_t,
        0x39b249i32 as uint32_t,
        0xbb243bi32 as uint32_t,
        0x964078i32 as uint32_t,
        0x66874fi32 as uint32_t,
        0x308ab9i32 as uint32_t,
        0x127ed0i32 as uint32_t,
        0xbe450ci32 as uint32_t,
    ];
    let mut checksum: libc::c_int = 0i32;
    let mut str_len: libc::c_int = strlen(str_lower) as libc::c_int;
    let mut i: libc::c_int = 0i32;
    while i < str_len {
        checksum += (i + 1i32) * *str_lower.offset(i as isize) as libc::c_int;
        checksum %= 0xffffffi32;
        i += 1
    }
    let mut color_index: libc::c_int = (checksum as libc::c_ulong).wrapping_rem(
        (::std::mem::size_of::<[uint32_t; 16]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<uint32_t>() as libc::c_ulong),
    ) as libc::c_int;
    free(str_lower as *mut libc::c_void);
    return colors[color_index as usize] as libc::c_int;
}
/* clist tools */
/* calls free() for each item content */
#[no_mangle]
pub unsafe extern "C" fn clist_free_content(mut haystack: *const clist) {
    let mut iter: *mut clistiter = (*haystack).first;
    while !iter.is_null() {
        free((*iter).data);
        (*iter).data = 0 as *mut libc::c_void;
        iter = if !iter.is_null() {
            (*iter).next
        } else {
            0 as *mut clistcell_s
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn clist_search_string_nocase(
    mut haystack: *const clist,
    mut needle: *const libc::c_char,
) -> libc::c_int {
    let mut iter: *mut clistiter = (*haystack).first;
    while !iter.is_null() {
        if strcasecmp((*iter).data as *const libc::c_char, needle) == 0i32 {
            return 1i32;
        }
        iter = if !iter.is_null() {
            (*iter).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return 0i32;
}
/* date/time tools */
/* the result is UTC or DC_INVALID_TIMESTAMP */
#[no_mangle]
pub unsafe extern "C" fn dc_timestamp_from_date(mut date_time: *mut mailimf_date_time) -> time_t {
    let mut tmval: tm = tm {
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
    let mut timeval: time_t = 0i32 as time_t;
    let mut zone_min: libc::c_int = 0i32;
    let mut zone_hour: libc::c_int = 0i32;
    memset(
        &mut tmval as *mut tm as *mut libc::c_void,
        0i32,
        ::std::mem::size_of::<tm>() as libc::c_ulong,
    );
    tmval.tm_sec = (*date_time).dt_sec;
    tmval.tm_min = (*date_time).dt_min;
    tmval.tm_hour = (*date_time).dt_hour;
    tmval.tm_mday = (*date_time).dt_day;
    tmval.tm_mon = (*date_time).dt_month - 1i32;
    if (*date_time).dt_year < 1000i32 {
        tmval.tm_year = (*date_time).dt_year + 2000i32 - 1900i32
    } else {
        tmval.tm_year = (*date_time).dt_year - 1900i32
    }
    timeval = mkgmtime(&mut tmval);
    if (*date_time).dt_zone >= 0i32 {
        zone_hour = (*date_time).dt_zone / 100i32;
        zone_min = (*date_time).dt_zone % 100i32
    } else {
        zone_hour = -(-(*date_time).dt_zone / 100i32);
        zone_min = -(-(*date_time).dt_zone % 100i32)
    }
    timeval -= (zone_hour * 3600i32 + zone_min * 60i32) as libc::c_long;
    return timeval;
}
#[no_mangle]
pub unsafe extern "C" fn mkgmtime(mut tmp: *mut tm) -> time_t {
    let mut dir: libc::c_int = 0i32;
    let mut bits: libc::c_int = 0i32;
    let mut saved_seconds: libc::c_int = 0i32;
    let mut t: time_t = 0i32 as time_t;
    let mut yourtm: tm = tm {
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
    let mut mytm: tm = tm {
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
    yourtm = *tmp;
    saved_seconds = yourtm.tm_sec;
    yourtm.tm_sec = 0i32;
    bits = 0i32;
    t = 1i32 as time_t;
    while t > 0i32 as libc::c_long {
        bits += 1;
        t <<= 1i32
    }
    if bits > 40i32 {
        bits = 40i32
    }
    t = if t < 0i32 as libc::c_long {
        0i32 as libc::c_long
    } else {
        (1i32 as time_t) << bits
    };
    loop {
        gmtime_r(&mut t, &mut mytm);
        dir = tmcomp(&mut mytm, &mut yourtm);
        if !(dir != 0i32) {
            break;
        }
        let fresh2 = bits;
        bits = bits - 1;
        if fresh2 < 0i32 {
            return -1i32 as time_t;
        }
        if bits < 0i32 {
            t -= 1
        } else if dir > 0i32 {
            t -= (1i32 as time_t) << bits
        } else {
            t += (1i32 as time_t) << bits
        }
    }
    t += saved_seconds as libc::c_long;
    return t;
}
/* ******************************************************************************
 * date/time tools
 ******************************************************************************/
unsafe extern "C" fn tmcomp(mut atmp: *mut tm, mut btmp: *mut tm) -> libc::c_int {
    let mut result: libc::c_int = 0i32;
    result = (*atmp).tm_year - (*btmp).tm_year;
    if result == 0i32
        && {
            result = (*atmp).tm_mon - (*btmp).tm_mon;
            result == 0i32
        }
        && {
            result = (*atmp).tm_mday - (*btmp).tm_mday;
            result == 0i32
        }
        && {
            result = (*atmp).tm_hour - (*btmp).tm_hour;
            result == 0i32
        }
        && {
            result = (*atmp).tm_min - (*btmp).tm_min;
            result == 0i32
        }
    {
        result = (*atmp).tm_sec - (*btmp).tm_sec
    }
    return result;
}
/* the return value must be free()'d */
#[no_mangle]
pub unsafe extern "C" fn dc_timestamp_to_str(mut wanted: time_t) -> *mut libc::c_char {
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
        localtime(&mut wanted) as *const libc::c_void,
        ::std::mem::size_of::<tm>() as libc::c_ulong,
    );
    return dc_mprintf(
        b"%02i.%02i.%04i %02i:%02i:%02i\x00" as *const u8 as *const libc::c_char,
        wanted_struct.tm_mday as libc::c_int,
        wanted_struct.tm_mon as libc::c_int + 1i32,
        wanted_struct.tm_year as libc::c_int + 1900i32,
        wanted_struct.tm_hour as libc::c_int,
        wanted_struct.tm_min as libc::c_int,
        wanted_struct.tm_sec as libc::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_timestamp_to_mailimap_date_time(
    mut timeval: time_t,
) -> *mut mailimap_date_time {
    let mut gmt: tm = tm {
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
    let mut lt: tm = tm {
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
    let mut off: libc::c_int = 0i32;
    let mut date_time: *mut mailimap_date_time = 0 as *mut mailimap_date_time;
    let mut sign: libc::c_int = 0i32;
    let mut hour: libc::c_int = 0i32;
    let mut min: libc::c_int = 0i32;
    gmtime_r(&mut timeval, &mut gmt);
    localtime_r(&mut timeval, &mut lt);
    off = ((mkgmtime(&mut lt) - mkgmtime(&mut gmt)) / 60i32 as libc::c_long) as libc::c_int;
    if off < 0i32 {
        sign = -1i32
    } else {
        sign = 1i32
    }
    off = off * sign;
    min = off % 60i32;
    hour = off / 60i32;
    off = hour * 100i32 + min;
    off = off * sign;
    date_time = mailimap_date_time_new(
        lt.tm_mday,
        lt.tm_mon + 1i32,
        lt.tm_year + 1900i32,
        lt.tm_hour,
        lt.tm_min,
        lt.tm_sec,
        off,
    );
    return date_time;
}
#[no_mangle]
pub unsafe extern "C" fn dc_gm2local_offset() -> libc::c_long {
    /* returns the offset that must be _added_ to an UTC/GMT-time to create the localtime.
    the function may return nagative values. */
    let mut gmtime: time_t = time(0 as *mut time_t);
    let mut timeinfo: tm = tm {
        tm_sec: 0i32,
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
    localtime_r(&mut gmtime, &mut timeinfo);
    return timeinfo.tm_gmtoff;
}
/* timesmearing */
#[no_mangle]
pub unsafe extern "C" fn dc_smeared_time(mut context: *mut dc_context_t) -> time_t {
    /* function returns a corrected time(NULL) */
    let mut now: time_t = time(0 as *mut time_t);
    pthread_mutex_lock(&mut (*context).smear_critical);
    if (*context).last_smeared_timestamp >= now {
        now = (*context).last_smeared_timestamp + 1i32 as libc::c_long
    }
    pthread_mutex_unlock(&mut (*context).smear_critical);
    return now;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_smeared_timestamp(mut context: *mut dc_context_t) -> time_t {
    let mut now: time_t = time(0 as *mut time_t);
    let mut ret: time_t = now;
    pthread_mutex_lock(&mut (*context).smear_critical);
    if ret <= (*context).last_smeared_timestamp {
        ret = (*context).last_smeared_timestamp + 1i32 as libc::c_long;
        if ret - now > 5i32 as libc::c_long {
            ret = now + 5i32 as libc::c_long
        }
    }
    (*context).last_smeared_timestamp = ret;
    pthread_mutex_unlock(&mut (*context).smear_critical);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_smeared_timestamps(
    mut context: *mut dc_context_t,
    mut count: libc::c_int,
) -> time_t {
    /* get a range to timestamps that can be used uniquely */
    let mut now: time_t = time(0 as *mut time_t);
    let mut start: time_t =
        now + (if count < 5i32 { count } else { 5i32 }) as libc::c_long - count as libc::c_long;
    pthread_mutex_lock(&mut (*context).smear_critical);
    start = if (*context).last_smeared_timestamp + 1i32 as libc::c_long > start {
        (*context).last_smeared_timestamp + 1i32 as libc::c_long
    } else {
        start
    };
    (*context).last_smeared_timestamp = start + (count - 1i32) as libc::c_long;
    pthread_mutex_unlock(&mut (*context).smear_critical);
    return start;
}
/* Message-ID tools */
#[no_mangle]
pub unsafe extern "C" fn dc_create_id() -> *mut libc::c_char {
    /* generate an id. the generated ID should be as short and as unique as possible:
    - short, because it may also used as part of Message-ID headers or in QR codes
    - unique as two IDs generated on two devices should not be the same. However, collisions are not world-wide but only by the few contacts.
    IDs generated by this function are 66 bit wide and are returned as 11 base64 characters.
    If possible, RNG of OpenSSL is used.

    Additional information when used as a message-id or group-id:
    - for OUTGOING messages this ID is written to the header as `Chat-Group-ID:` and is added to the message ID as Gr.<grpid>.<random>@<random>
    - for INCOMING messages, the ID is taken from the Chat-Group-ID-header or from the Message-ID in the In-Reply-To: or References:-Header
    - the group-id should be a string with the characters [a-zA-Z0-9\-_] */
    let mut buf: [uint32_t; 3] = [0; 3];
    if 0 == RAND_bytes(
        &mut buf as *mut [uint32_t; 3] as *mut libc::c_uchar,
        (::std::mem::size_of::<uint32_t>() as libc::c_ulong).wrapping_mul(3i32 as libc::c_ulong)
            as libc::c_int,
    ) {
        RAND_pseudo_bytes(
            &mut buf as *mut [uint32_t; 3] as *mut libc::c_uchar,
            (::std::mem::size_of::<uint32_t>() as libc::c_ulong).wrapping_mul(3i32 as libc::c_ulong)
                as libc::c_int,
        );
    }
    return encode_66bits_as_base64(buf[0usize], buf[1usize], buf[2usize]);
}
/* ******************************************************************************
 * generate Message-IDs
 ******************************************************************************/
unsafe extern "C" fn encode_66bits_as_base64(
    mut v1: uint32_t,
    mut v2: uint32_t,
    mut fill: uint32_t,
) -> *mut libc::c_char {
    /* encode 66 bits as a base64 string. This is useful for ID generating with short strings as
    we save 5 character in each id compared to 64 bit hex encoding, for a typical group ID, these are 10 characters (grpid+msgid):
    hex:    64 bit, 4 bits/character, length = 64/4 = 16 characters
    base64: 64 bit, 6 bits/character, length = 64/6 = 11 characters (plus 2 additional bits) */
    let mut ret: *mut libc::c_char = malloc(12i32 as libc::c_ulong) as *mut libc::c_char;
    if ret.is_null() {
        exit(34i32);
    }
    static mut chars: [libc::c_char; 65] = [
        65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87,
        88, 89, 90, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
        113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57,
        45, 95, 0,
    ];
    *ret.offset(0isize) = chars[(v1 >> 26i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(1isize) = chars[(v1 >> 20i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(2isize) = chars[(v1 >> 14i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(3isize) = chars[(v1 >> 8i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(4isize) = chars[(v1 >> 2i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(5isize) = chars
        [(v1 << 4i32 & 0x30i32 as libc::c_uint | v2 >> 28i32 & 0xfi32 as libc::c_uint) as usize];
    *ret.offset(6isize) = chars[(v2 >> 22i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(7isize) = chars[(v2 >> 16i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(8isize) = chars[(v2 >> 10i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(9isize) = chars[(v2 >> 4i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(10isize) =
        chars[(v2 << 2i32 & 0x3ci32 as libc::c_uint | fill & 0x3i32 as libc::c_uint) as usize];
    *ret.offset(11isize) = 0i32 as libc::c_char;
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_incoming_rfc724_mid(
    mut message_timestamp: time_t,
    mut contact_id_from: uint32_t,
    mut contact_ids_to: *mut dc_array_t,
) -> *mut libc::c_char {
    if contact_ids_to.is_null() || dc_array_get_cnt(contact_ids_to) == 0i32 as libc::c_ulong {
        return 0 as *mut libc::c_char;
    }
    /* find out the largest receiver ID (we could also take the smallest, but it should be unique) */
    let mut i: size_t = 0i32 as size_t;
    let mut icnt: size_t = dc_array_get_cnt(contact_ids_to);
    let mut largest_id_to: uint32_t = 0i32 as uint32_t;
    i = 0i32 as size_t;
    while i < icnt {
        let mut cur_id: uint32_t = dc_array_get_id(contact_ids_to, i);
        if cur_id > largest_id_to {
            largest_id_to = cur_id
        }
        i = i.wrapping_add(1)
    }
    return dc_mprintf(
        b"%lu-%lu-%lu@stub\x00" as *const u8 as *const libc::c_char,
        message_timestamp as libc::c_ulong,
        contact_id_from as libc::c_ulong,
        largest_id_to as libc::c_ulong,
    );
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_outgoing_rfc724_mid(
    mut grpid: *const libc::c_char,
    mut from_addr: *const libc::c_char,
) -> *mut libc::c_char {
    /* Function generates a Message-ID that can be used for a new outgoing message.
    - this function is called for all outgoing messages.
    - the message ID should be globally unique
    - do not add a counter or any private data as as this may give unneeded information to the receiver	*/
    let mut rand1: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut rand2: *mut libc::c_char = dc_create_id();
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut at_hostname: *const libc::c_char = strchr(from_addr, '@' as i32);
    if at_hostname.is_null() {
        at_hostname = b"@nohost\x00" as *const u8 as *const libc::c_char
    }
    if !grpid.is_null() {
        ret = dc_mprintf(
            b"Gr.%s.%s%s\x00" as *const u8 as *const libc::c_char,
            grpid,
            rand2,
            at_hostname,
        )
    } else {
        rand1 = dc_create_id();
        ret = dc_mprintf(
            b"Mr.%s.%s%s\x00" as *const u8 as *const libc::c_char,
            rand1,
            rand2,
            at_hostname,
        )
    }
    free(rand1 as *mut libc::c_void);
    free(rand2 as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_extract_grpid_from_rfc724_mid(
    mut mid: *const libc::c_char,
) -> *mut libc::c_char {
    /* extract our group ID from Message-IDs as `Gr.12345678901.morerandom@domain.de`; "12345678901" is the wanted ID in this example. */
    let mut success: libc::c_int = 0i32;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p1: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpid_len: libc::c_int = 0i32;
    if !(mid.is_null()
        || strlen(mid) < 8i32 as libc::c_ulong
        || *mid.offset(0isize) as libc::c_int != 'G' as i32
        || *mid.offset(1isize) as libc::c_int != 'r' as i32
        || *mid.offset(2isize) as libc::c_int != '.' as i32)
    {
        grpid = dc_strdup(&*mid.offset(3isize));
        p1 = strchr(grpid, '.' as i32);
        if !p1.is_null() {
            *p1 = 0i32 as libc::c_char;
            grpid_len = strlen(grpid) as libc::c_int;
            if !(grpid_len != 11i32 && grpid_len != 16i32) {
                /* strict length comparison, the 'Gr.' magic is weak enough */
                success = 1i32
            }
        }
    }
    if success == 0i32 {
        free(grpid as *mut libc::c_void);
        grpid = 0 as *mut libc::c_char
    }
    return if 0 != success {
        grpid
    } else {
        0 as *mut libc::c_char
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_extract_grpid_from_rfc724_mid_list(
    mut list: *const clist,
) -> *mut libc::c_char {
    if !list.is_null() {
        let mut cur: *mut clistiter = (*list).first;
        while !cur.is_null() {
            let mut mid: *const libc::c_char = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *const libc::c_char;
            let mut grpid: *mut libc::c_char = dc_extract_grpid_from_rfc724_mid(mid);
            if !grpid.is_null() {
                return grpid;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell_s
            }
        }
    }
    return 0 as *mut libc::c_char;
}
/* file tools */
#[no_mangle]
pub unsafe extern "C" fn dc_ensure_no_slash(mut pathNfilename: *mut libc::c_char) {
    let mut path_len: libc::c_int = strlen(pathNfilename) as libc::c_int;
    if path_len > 0i32 {
        if *pathNfilename.offset((path_len - 1i32) as isize) as libc::c_int == '/' as i32
            || *pathNfilename.offset((path_len - 1i32) as isize) as libc::c_int == '\\' as i32
        {
            *pathNfilename.offset((path_len - 1i32) as isize) = 0i32 as libc::c_char
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_validate_filename(mut filename: *mut libc::c_char) {
    /* function modifies the given buffer and replaces all characters not valid in filenames by a "-" */
    let mut p1: *mut libc::c_char = filename;
    while 0 != *p1 {
        if *p1 as libc::c_int == '/' as i32
            || *p1 as libc::c_int == '\\' as i32
            || *p1 as libc::c_int == ':' as i32
        {
            *p1 = '-' as i32 as libc::c_char
        }
        p1 = p1.offset(1isize)
    }
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_filename(
    mut pathNfilename: *const libc::c_char,
) -> *mut libc::c_char {
    let mut p: *const libc::c_char = strrchr(pathNfilename, '/' as i32);
    if p.is_null() {
        p = strrchr(pathNfilename, '\\' as i32)
    }
    if !p.is_null() {
        p = p.offset(1isize);
        return dc_strdup(p);
    } else {
        return dc_strdup(pathNfilename);
    };
}
// the case of the suffix is preserved
#[no_mangle]
pub unsafe extern "C" fn dc_split_filename(
    mut pathNfilename: *const libc::c_char,
    mut ret_basename: *mut *mut libc::c_char,
    mut ret_all_suffixes_incl_dot: *mut *mut libc::c_char,
) {
    /* splits a filename into basename and all suffixes, eg. "/path/foo.tar.gz" is split into "foo.tar" and ".gz",
    (we use the _last_ dot which allows the usage inside the filename which are very usual;
    maybe the detection could be more intelligent, however, for the moment, it is just file)
    - if there is no suffix, the returned suffix string is empty, eg. "/path/foobar" is split into "foobar" and ""
    - the case of the returned suffix is preserved; this is to allow reconstruction of (similar) names */
    let mut basename: *mut libc::c_char = dc_get_filename(pathNfilename);
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p1: *mut libc::c_char = strrchr(basename, '.' as i32);
    if !p1.is_null() {
        suffix = dc_strdup(p1);
        *p1 = 0i32 as libc::c_char
    } else {
        suffix = dc_strdup(0 as *const libc::c_char)
    }
    if !ret_basename.is_null() {
        *ret_basename = basename
    } else {
        free(basename as *mut libc::c_void);
    }
    if !ret_all_suffixes_incl_dot.is_null() {
        *ret_all_suffixes_incl_dot = suffix
    } else {
        free(suffix as *mut libc::c_void);
    };
}
// the returned suffix is lower-case
#[no_mangle]
pub unsafe extern "C" fn dc_get_filesuffix_lc(
    mut pathNfilename: *const libc::c_char,
) -> *mut libc::c_char {
    if !pathNfilename.is_null() {
        let mut p: *const libc::c_char = strrchr(pathNfilename, '.' as i32);
        if !p.is_null() {
            p = p.offset(1isize);
            return dc_strlower(p);
        }
    }
    return 0 as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_filemeta(
    mut buf_start: *const libc::c_void,
    mut buf_bytes: size_t,
    mut ret_width: *mut uint32_t,
    mut ret_height: *mut uint32_t,
) -> libc::c_int {
    /* Strategy:
    reading GIF dimensions requires the first 10 bytes of the file
    reading PNG dimensions requires the first 24 bytes of the file
    reading JPEG dimensions requires scanning through jpeg chunks
    In all formats, the file is at least 24 bytes big, so we'll read that always
    inspired by http://www.cplusplus.com/forum/beginner/45217/ */
    let mut buf: *const libc::c_uchar = buf_start as *const libc::c_uchar;
    if buf_bytes < 24i32 as libc::c_ulong {
        return 0i32;
    }
    if *buf.offset(0isize) as libc::c_int == 0xffi32
        && *buf.offset(1isize) as libc::c_int == 0xd8i32
        && *buf.offset(2isize) as libc::c_int == 0xffi32
    {
        let mut pos: libc::c_long = 2i32 as libc::c_long;
        while *buf.offset(pos as isize) as libc::c_int == 0xffi32 {
            if *buf.offset((pos + 1i32 as libc::c_long) as isize) as libc::c_int == 0xc0i32
                || *buf.offset((pos + 1i32 as libc::c_long) as isize) as libc::c_int == 0xc1i32
                || *buf.offset((pos + 1i32 as libc::c_long) as isize) as libc::c_int == 0xc2i32
                || *buf.offset((pos + 1i32 as libc::c_long) as isize) as libc::c_int == 0xc3i32
                || *buf.offset((pos + 1i32 as libc::c_long) as isize) as libc::c_int == 0xc9i32
                || *buf.offset((pos + 1i32 as libc::c_long) as isize) as libc::c_int == 0xcai32
                || *buf.offset((pos + 1i32 as libc::c_long) as isize) as libc::c_int == 0xcbi32
            {
                *ret_height =
                    (((*buf.offset((pos + 5i32 as libc::c_long) as isize) as libc::c_int) << 8i32)
                        + *buf.offset((pos + 6i32 as libc::c_long) as isize) as libc::c_int)
                        as uint32_t;
                *ret_width = (((*buf.offset((pos + 7i32 as libc::c_long) as isize) as libc::c_int)
                    << 8i32)
                    + *buf.offset((pos + 8i32 as libc::c_long) as isize) as libc::c_int)
                    as uint32_t;
                return 1i32;
            }
            pos += (2i32
                + ((*buf.offset((pos + 2i32 as libc::c_long) as isize) as libc::c_int) << 8i32)
                + *buf.offset((pos + 3i32 as libc::c_long) as isize) as libc::c_int)
                as libc::c_long;
            if (pos + 12i32 as libc::c_long) as libc::c_ulong > buf_bytes {
                break;
            }
        }
    }
    if *buf.offset(0isize) as libc::c_int == 'G' as i32
        && *buf.offset(1isize) as libc::c_int == 'I' as i32
        && *buf.offset(2isize) as libc::c_int == 'F' as i32
    {
        *ret_width = (*buf.offset(6isize) as libc::c_int
            + ((*buf.offset(7isize) as libc::c_int) << 8i32)) as uint32_t;
        *ret_height = (*buf.offset(8isize) as libc::c_int
            + ((*buf.offset(9isize) as libc::c_int) << 8i32)) as uint32_t;
        return 1i32;
    }
    if *buf.offset(0isize) as libc::c_int == 0x89i32
        && *buf.offset(1isize) as libc::c_int == 'P' as i32
        && *buf.offset(2isize) as libc::c_int == 'N' as i32
        && *buf.offset(3isize) as libc::c_int == 'G' as i32
        && *buf.offset(4isize) as libc::c_int == 0xdi32
        && *buf.offset(5isize) as libc::c_int == 0xai32
        && *buf.offset(6isize) as libc::c_int == 0x1ai32
        && *buf.offset(7isize) as libc::c_int == 0xai32
        && *buf.offset(12isize) as libc::c_int == 'I' as i32
        && *buf.offset(13isize) as libc::c_int == 'H' as i32
        && *buf.offset(14isize) as libc::c_int == 'D' as i32
        && *buf.offset(15isize) as libc::c_int == 'R' as i32
    {
        *ret_width = (((*buf.offset(16isize) as libc::c_int) << 24i32)
            + ((*buf.offset(17isize) as libc::c_int) << 16i32)
            + ((*buf.offset(18isize) as libc::c_int) << 8i32)
            + ((*buf.offset(19isize) as libc::c_int) << 0i32)) as uint32_t;
        *ret_height = (((*buf.offset(20isize) as libc::c_int) << 24i32)
            + ((*buf.offset(21isize) as libc::c_int) << 16i32)
            + ((*buf.offset(22isize) as libc::c_int) << 8i32)
            + ((*buf.offset(23isize) as libc::c_int) << 0i32)) as uint32_t;
        return 1i32;
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_abs_path(
    mut context: *mut dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> *mut libc::c_char {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null() || pathNfilename.is_null()) {
        pathNfilename_abs = dc_strdup(pathNfilename);
        if strncmp(
            pathNfilename_abs,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            8i32 as libc::c_ulong,
        ) == 0i32
        {
            if (*context).blobdir.is_null() {
                current_block = 3805228753452640762;
            } else {
                dc_str_replace(
                    &mut pathNfilename_abs,
                    b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
                    (*context).blobdir,
                );
                current_block = 6937071982253665452;
            }
        } else {
            current_block = 6937071982253665452;
        }
        match current_block {
            3805228753452640762 => {}
            _ => success = 1i32,
        }
    }
    if 0 == success {
        free(pathNfilename_abs as *mut libc::c_void);
        pathNfilename_abs = 0 as *mut libc::c_char
    }
    return pathNfilename_abs;
}
#[no_mangle]
pub unsafe extern "C" fn dc_file_exist(
    mut context: *mut dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> libc::c_int {
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
    let mut exist: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        st = stat {
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
        if stat(pathNfilename_abs, &mut st) == 0i32 {
            exist = 1i32
        }
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return exist;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_filebytes(
    mut context: *mut dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> uint64_t {
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
    let mut filebytes: uint64_t = 0i32 as uint64_t;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        st = stat {
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
        if stat(pathNfilename_abs, &mut st) == 0i32 {
            filebytes = st.st_size as uint64_t
        }
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return filebytes;
}
#[no_mangle]
pub unsafe extern "C" fn dc_delete_file(
    mut context: *mut dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        if remove(pathNfilename_abs) != 0i32 {
            dc_log_warning(
                context,
                0i32,
                b"Cannot delete \"%s\".\x00" as *const u8 as *const libc::c_char,
                pathNfilename,
            );
        } else {
            success = 1i32
        }
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_copy_file(
    mut context: *mut dc_context_t,
    mut src: *const libc::c_char,
    mut dest: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut src_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fd_src: libc::c_int = -1i32;
    let mut fd_dest: libc::c_int = -1i32;
    let mut buf: [libc::c_char; 4096] = [0; 4096];
    let mut bytes_read: size_t = 0i32 as size_t;
    let mut anything_copied: libc::c_int = 0i32;
    src_abs = dc_get_abs_path(context, src);
    if !(src_abs.is_null() || {
        dest_abs = dc_get_abs_path(context, dest);
        dest_abs.is_null()
    }) {
        fd_src = open(src_abs, 0i32);
        if fd_src < 0i32 {
            dc_log_error(
                context,
                0i32,
                b"Cannot open source file \"%s\".\x00" as *const u8 as *const libc::c_char,
                src,
            );
        } else {
            fd_dest = open(dest_abs, 0x1i32 | 0x200i32 | 0x800i32, 0o666i32);
            if fd_dest < 0i32 {
                dc_log_error(
                    context,
                    0i32,
                    b"Cannot open destination file \"%s\".\x00" as *const u8 as *const libc::c_char,
                    dest,
                );
            } else {
                loop {
                    bytes_read = read(
                        fd_src,
                        buf.as_mut_ptr() as *mut libc::c_void,
                        4096i32 as size_t,
                    ) as size_t;
                    if !(bytes_read > 0i32 as libc::c_ulong) {
                        break;
                    }
                    if write(fd_dest, buf.as_mut_ptr() as *const libc::c_void, bytes_read)
                        as libc::c_ulong
                        != bytes_read
                    {
                        dc_log_error(
                            context,
                            0i32,
                            b"Cannot write %i bytes to \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            bytes_read,
                            dest,
                        );
                    }
                    anything_copied = 1i32
                }
                if 0 == anything_copied {
                    close(fd_src);
                    fd_src = -1i32;
                    if dc_get_filebytes(context, src) != 0i32 as libc::c_ulonglong {
                        dc_log_error(
                            context,
                            0i32,
                            b"Different size information for \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            bytes_read,
                            dest,
                        );
                        current_block = 610040589300051390;
                    } else {
                        current_block = 5634871135123216486;
                    }
                } else {
                    current_block = 5634871135123216486;
                }
                match current_block {
                    610040589300051390 => {}
                    _ => success = 1i32,
                }
            }
        }
    }
    if fd_src >= 0i32 {
        close(fd_src);
    }
    if fd_dest >= 0i32 {
        close(fd_dest);
    }
    free(src_abs as *mut libc::c_void);
    free(dest_abs as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_folder(
    mut context: *mut dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> libc::c_int {
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
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        st = stat {
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
        if stat(pathNfilename_abs, &mut st) == -1i32 {
            if mkdir(pathNfilename_abs, 0o755i32 as mode_t) != 0i32 {
                dc_log_warning(
                    context,
                    0i32,
                    b"Cannot create directory \"%s\".\x00" as *const u8 as *const libc::c_char,
                    pathNfilename,
                );
                current_block = 7696101774396965466;
            } else {
                current_block = 7815301370352969686;
            }
        } else {
            current_block = 7815301370352969686;
        }
        match current_block {
            7696101774396965466 => {}
            _ => success = 1i32,
        }
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_write_file(
    mut context: *mut dc_context_t,
    mut pathNfilename: *const libc::c_char,
    mut buf: *const libc::c_void,
    mut buf_bytes: size_t,
) -> libc::c_int {
    let mut f: *mut FILE = 0 as *mut FILE;
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        f = fopen(
            pathNfilename_abs,
            b"wb\x00" as *const u8 as *const libc::c_char,
        );
        if !f.is_null() {
            if fwrite(buf, 1i32 as libc::c_ulong, buf_bytes, f) == buf_bytes {
                success = 1i32
            } else {
                dc_log_warning(
                    context,
                    0i32,
                    b"Cannot write %lu bytes to \"%s\".\x00" as *const u8 as *const libc::c_char,
                    buf_bytes as libc::c_ulong,
                    pathNfilename,
                );
            }
            fclose(f);
        } else {
            dc_log_warning(
                context,
                0i32,
                b"Cannot open \"%s\" for writing.\x00" as *const u8 as *const libc::c_char,
                pathNfilename,
            );
        }
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_read_file(
    mut context: *mut dc_context_t,
    mut pathNfilename: *const libc::c_char,
    mut buf: *mut *mut libc::c_void,
    mut buf_bytes: *mut size_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut f: *mut FILE = 0 as *mut FILE;
    if pathNfilename.is_null() || buf.is_null() || buf_bytes.is_null() {
        return 0i32;
    }
    *buf = 0 as *mut libc::c_void;
    *buf_bytes = 0i32 as size_t;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        f = fopen(
            pathNfilename_abs,
            b"rb\x00" as *const u8 as *const libc::c_char,
        );
        if !f.is_null() {
            fseek(f, 0i32 as libc::c_long, 2i32);
            *buf_bytes = ftell(f) as size_t;
            fseek(f, 0i32 as libc::c_long, 0i32);
            if !(*buf_bytes <= 0i32 as libc::c_ulong) {
                *buf = malloc((*buf_bytes).wrapping_add(1i32 as libc::c_ulong));
                if !(*buf).is_null() {
                    *(*buf as *mut libc::c_char).offset(*buf_bytes as isize) = 0i32 as libc::c_char;
                    if !(fread(*buf, 1i32 as libc::c_ulong, *buf_bytes, f) != *buf_bytes) {
                        success = 1i32
                    }
                }
            }
        }
    }
    if !f.is_null() {
        fclose(f);
    }
    if success == 0i32 {
        free(*buf);
        *buf = 0 as *mut libc::c_void;
        *buf_bytes = 0i32 as size_t;
        dc_log_warning(
            context,
            0i32,
            b"Cannot read \"%s\" or file is empty.\x00" as *const u8 as *const libc::c_char,
            pathNfilename,
        );
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_get_fine_pathNfilename(
    mut context: *mut dc_context_t,
    mut pathNfolder: *const libc::c_char,
    mut desired_filenameNsuffix__: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut pathNfolder_wo_slash: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filenameNsuffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut basename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dotNSuffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut now: time_t = time(0 as *mut time_t);
    let mut i: libc::c_int = 0i32;
    pathNfolder_wo_slash = dc_strdup(pathNfolder);
    dc_ensure_no_slash(pathNfolder_wo_slash);
    filenameNsuffix = dc_strdup(desired_filenameNsuffix__);
    dc_validate_filename(filenameNsuffix);
    dc_split_filename(filenameNsuffix, &mut basename, &mut dotNSuffix);
    i = 0i32;
    while i < 1000i32 {
        /*no deadlocks, please*/
        if 0 != i {
            let mut idx: time_t = if i < 100i32 {
                i as libc::c_long
            } else {
                now + i as libc::c_long
            };
            ret = dc_mprintf(
                b"%s/%s-%lu%s\x00" as *const u8 as *const libc::c_char,
                pathNfolder_wo_slash,
                basename,
                idx as libc::c_ulong,
                dotNSuffix,
            )
        } else {
            ret = dc_mprintf(
                b"%s/%s%s\x00" as *const u8 as *const libc::c_char,
                pathNfolder_wo_slash,
                basename,
                dotNSuffix,
            )
        }
        if 0 == dc_file_exist(context, ret) {
            /* fine filename found */
            break;
        } else {
            free(ret as *mut libc::c_void);
            ret = 0 as *mut libc::c_char;
            i += 1
        }
    }
    free(filenameNsuffix as *mut libc::c_void);
    free(basename as *mut libc::c_void);
    free(dotNSuffix as *mut libc::c_void);
    free(pathNfolder_wo_slash as *mut libc::c_void);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_is_blobdir_path(
    mut context: *mut dc_context_t,
    mut path: *const libc::c_char,
) -> libc::c_int {
    if strncmp(path, (*context).blobdir, strlen((*context).blobdir)) == 0i32
        || strncmp(
            path,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            8i32 as libc::c_ulong,
        ) == 0i32
    {
        return 1i32;
    }
    return 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_make_rel_path(
    mut context: *mut dc_context_t,
    mut path: *mut *mut libc::c_char,
) {
    if context.is_null() || path.is_null() || (*path).is_null() {
        return;
    }
    if strncmp(*path, (*context).blobdir, strlen((*context).blobdir)) == 0i32 {
        dc_str_replace(
            path,
            (*context).blobdir,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_make_rel_and_copy(
    mut context: *mut dc_context_t,
    mut path: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut blobdir_path: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null() || path.is_null() || (*path).is_null()) {
        if 0 != dc_is_blobdir_path(context, *path) {
            dc_make_rel_path(context, path);
            success = 1i32
        } else {
            filename = dc_get_filename(*path);
            if !(filename.is_null()
                || {
                    blobdir_path = dc_get_fine_pathNfilename(
                        context,
                        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
                        filename,
                    );
                    blobdir_path.is_null()
                }
                || 0 == dc_copy_file(context, *path, blobdir_path))
            {
                free(*path as *mut libc::c_void);
                *path = blobdir_path;
                blobdir_path = 0 as *mut libc::c_char;
                dc_make_rel_path(context, path);
                success = 1i32
            }
        }
    }
    free(blobdir_path as *mut libc::c_void);
    free(filename as *mut libc::c_void);
    return success;
}

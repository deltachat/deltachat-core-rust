use libc;
extern "C" {
    #[no_mangle]
    fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn realloc(_: *mut libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn exit(_: libc::c_int) -> !;
    #[no_mangle]
    fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    #[no_mangle]
    fn vsnprintf(
        _: *mut libc::c_char,
        _: libc::c_ulong,
        _: *const libc::c_char,
        _: ::std::ffi::VaList,
    ) -> libc::c_int;
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
pub type __darwin_va_list = __builtin_va_list;
pub type va_list = __darwin_va_list;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type dc_strbuilder_t = _dc_strbuilder;
#[no_mangle]
pub unsafe extern "C" fn dc_strbuilder_init(
    mut strbuilder: *mut dc_strbuilder_t,
    mut init_bytes: libc::c_int,
) {
    if strbuilder.is_null() {
        return;
    }
    (*strbuilder).allocated = if init_bytes > 128i32 {
        init_bytes
    } else {
        128i32
    };
    (*strbuilder).buf = malloc((*strbuilder).allocated as libc::c_ulong) as *mut libc::c_char;
    if (*strbuilder).buf.is_null() {
        exit(38i32);
    }
    *(*strbuilder).buf.offset(0isize) = 0i32 as libc::c_char;
    (*strbuilder).free = (*strbuilder).allocated - 1i32;
    (*strbuilder).eos = (*strbuilder).buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_strbuilder_cat(
    mut strbuilder: *mut dc_strbuilder_t,
    mut text: *const libc::c_char,
) -> *mut libc::c_char {
    if strbuilder.is_null() || text.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut len: libc::c_int = strlen(text) as libc::c_int;
    if len > (*strbuilder).free {
        let mut add_bytes: libc::c_int = if len > (*strbuilder).allocated {
            len
        } else {
            (*strbuilder).allocated
        };
        let mut old_offset: libc::c_int = (*strbuilder).eos.wrapping_offset_from((*strbuilder).buf)
            as libc::c_long as libc::c_int;
        (*strbuilder).allocated = (*strbuilder).allocated + add_bytes;
        (*strbuilder).buf = realloc(
            (*strbuilder).buf as *mut libc::c_void,
            ((*strbuilder).allocated + add_bytes) as libc::c_ulong,
        ) as *mut libc::c_char;
        if (*strbuilder).buf.is_null() {
            exit(39i32);
        }
        (*strbuilder).free = (*strbuilder).free + add_bytes;
        (*strbuilder).eos = (*strbuilder).buf.offset(old_offset as isize)
    }
    let mut ret: *mut libc::c_char = (*strbuilder).eos;
    strcpy((*strbuilder).eos, text);
    (*strbuilder).eos = (*strbuilder).eos.offset(len as isize);
    (*strbuilder).free -= len;
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_strbuilder_empty(mut strbuilder: *mut dc_strbuilder_t) {
    *(*strbuilder).buf.offset(0isize) = 0i32 as libc::c_char;
    (*strbuilder).free = (*strbuilder).allocated - 1i32;
    (*strbuilder).eos = (*strbuilder).buf;
}

use crate::constants::Event;
use crate::dc_context::dc_context_t;

pub use libc::{dirent, tm, DIR, FILE};
pub use libsqlite3_sys::*;
pub use mmime::carray::*;
pub use mmime::clist::*;

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
pub type __int64_t = libc::c_longlong;
pub type __darwin_ct_rune_t = libc::c_int;
pub type __darwin_wchar_t = libc::c_int;
pub type __darwin_rune_t = __darwin_wchar_t;
pub type uint64_t = libc::c_ulonglong;

/**
 * Callback function that should be given to dc_context_new().
 *
 * @memberof dc_context_t
 * @param context The context object as returned by dc_context_new().
 * @param event one of the @ref DC_EVENT constants
 * @param data1 depends on the event parameter
 * @param data2 depends on the event parameter
 * @return return 0 unless stated otherwise in the event parameter documentation
 */
pub type dc_callback_t =
    unsafe extern "C" fn(_: &dc_context_t, _: Event, _: uintptr_t, _: uintptr_t) -> uintptr_t;

pub type dc_move_state_t = u32;

pub type dc_receive_imf_t = unsafe fn(
    _: &dc_context_t,
    _: *const libc::c_char,
    _: size_t,
    _: *const libc::c_char,
    _: uint32_t,
    _: uint32_t,
) -> ();

/* Purpose: Reading from IMAP servers with no dependencies to the database.
dc_context_t is only used for logging and to get information about
the online state. */

pub type dc_precheck_imf_t = unsafe fn(
    _: &dc_context_t,
    _: *const libc::c_char,
    _: *const libc::c_char,
    _: u32,
) -> libc::c_int;
pub type dc_set_config_t =
    unsafe fn(_: &dc_context_t, _: *const libc::c_char, _: *const libc::c_char) -> ();
pub type dc_get_config_t = unsafe fn(
    _: &dc_context_t,
    _: *const libc::c_char,
    _: *const libc::c_char,
) -> *mut libc::c_char;

pub type sqlite_int64 = libc::int64_t;
pub type sqlite3_int64 = sqlite_int64;

pub type int32_t = libc::int32_t;
pub type int64_t = libc::int64_t;
pub type uintptr_t = libc::uintptr_t;
pub type __uint8_t = libc::uint8_t;
pub type __uint16_t = libc::uint16_t;
pub type __int32_t = libc::int32_t;
pub type __uint64_t = libc::uint64_t;

pub type time_t = libc::time_t;
pub type pid_t = libc::pid_t;
pub type size_t = libc::size_t;
pub type ssize_t = libc::ssize_t;
pub type uint32_t = libc::c_uint;
pub type uint8_t = libc::c_uchar;
pub type uint16_t = libc::c_ushort;

pub type __uint32_t = libc::c_uint;

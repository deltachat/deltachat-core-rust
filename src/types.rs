#![allow(non_camel_case_types)]
use crate::constants::Event;
use crate::context::Context;

pub use mmime::carray::*;
pub use mmime::clist::*;
pub use rusqlite::ffi::*;

/// Callback function that should be given to dc_context_new().
///
/// @memberof Context
/// @param context The context object as returned by dc_context_new().
/// @param event one of the @ref DC_EVENT constants
/// @param data1 depends on the event parameter
/// @param data2 depends on the event parameter
/// @return return 0 unless stated otherwise in the event parameter documentation
pub type dc_callback_t =
    unsafe extern "C" fn(_: &Context, _: Event, _: uintptr_t, _: uintptr_t) -> uintptr_t;

pub type dc_move_state_t = u32;

pub type dc_receive_imf_t = unsafe fn(
    _: &Context,
    _: *const libc::c_char,
    _: size_t,
    _: &str,
    _: uint32_t,
    _: uint32_t,
) -> ();

/* Purpose: Reading from IMAP servers with no dependencies to the database.
Context is only used for logging and to get information about
the online state. */

pub type dc_precheck_imf_t =
    unsafe fn(_: &Context, _: *const libc::c_char, _: &str, _: u32) -> libc::c_int;
pub type dc_set_config_t =
    unsafe fn(_: &Context, _: *const libc::c_char, _: *const libc::c_char) -> ();
pub type dc_get_config_t = fn(_: &Context, _: &str) -> Option<String>;

pub type sqlite_int64 = i64;
pub type sqlite3_int64 = sqlite_int64;

pub type int32_t = i32;
pub type int64_t = i64;
pub type uintptr_t = libc::uintptr_t;
pub type size_t = libc::size_t;
pub type ssize_t = libc::ssize_t;
pub type uint32_t = libc::c_uint;
pub type uint8_t = libc::c_uchar;
pub type uint16_t = libc::c_ushort;
pub type uint64_t = u64;

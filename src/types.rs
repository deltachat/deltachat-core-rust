#![allow(non_camel_case_types)]
use crate::context::Context;

pub use mmime::clist::*;
pub use rusqlite::ffi::*;

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
pub type dc_set_config_t = fn(_: &Context, _: &str, _: Option<&str>) -> ();
pub type dc_get_config_t = fn(_: &Context, _: &str) -> Option<String>;

pub type int32_t = i32;
pub type uintptr_t = libc::uintptr_t;
pub type size_t = libc::size_t;
pub type uint32_t = libc::c_uint;
pub type uint8_t = libc::c_uchar;
pub type uint16_t = libc::c_ushort;
pub type uint64_t = u64;

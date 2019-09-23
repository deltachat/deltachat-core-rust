use crate::other::*;
use libc;
use std::ffi::{CStr, CString};

pub const MAIL_CHARCONV_ERROR_CONV: libc::c_uint = 3;
pub const MAIL_CHARCONV_ERROR_MEMORY: libc::c_uint = 2;
pub const MAIL_CHARCONV_ERROR_UNKNOWN_CHARSET: libc::c_uint = 1;
pub const MAIL_CHARCONV_NO_ERROR: libc::c_uint = 0;

pub unsafe fn charconv(
    tocode: *const libc::c_char,
    fromcode: *const libc::c_char,
    s: *const libc::c_char,
    length: size_t,
    result: *mut *mut libc::c_char,
) -> libc::c_int {
    assert!(!fromcode.is_null(), "invalid fromcode");
    assert!(!s.is_null(), "invalid input string");
    if let Some(encoding) =
        charset::Charset::for_label(CStr::from_ptr(fromcode).to_str().unwrap().as_bytes())
    {
        let data = std::slice::from_raw_parts(s as *const u8, strlen(s));

        let (res, _, _) = encoding.decode(data);
        let res_c = CString::new(res.as_bytes()).unwrap();
        *result = strdup(res_c.as_ptr()) as *mut _;

        MAIL_CHARCONV_NO_ERROR as libc::c_int
    } else {
        MAIL_CHARCONV_ERROR_UNKNOWN_CHARSET as libc::c_int
    }
}

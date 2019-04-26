use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_context::dc_context_t;
use crate::dc_lot::dc_lot_t;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[no_mangle]
pub unsafe extern "C" fn dc_log_event(
    mut context: *mut dc_context_t,
    mut event_code: libc::c_int,
    mut data1: libc::c_int,
    mut msg: *const libc::c_char,
    mut va: ...
) {
    log_vprintf(context, event_code, data1, msg, va);
}
/* Asynchronous "Thread-errors" are reported by the dc_log_error()
function.  These errors must be shown to the user by a bubble or so.

"Normal" errors are usually returned by a special value (null or so) and are
usually not reported using dc_log_error() - its up to the caller to
decide, what should be reported or done.  However, these "Normal" errors
are usually logged by dc_log_warning(). */
unsafe extern "C" fn log_vprintf(
    mut context: *mut dc_context_t,
    mut event: libc::c_int,
    mut data1: libc::c_int,
    mut msg_format: *const libc::c_char,
    mut va_0: ::std::ffi::VaList,
) {
    let mut msg: *mut libc::c_char = 0 as *mut libc::c_char;
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    if !msg_format.is_null() {
        let mut tempbuf: [libc::c_char; 1025] = [0; 1025];
        vsnprintf(
            tempbuf.as_mut_ptr(),
            1024i32 as libc::c_ulong,
            msg_format,
            va_0,
        );
        msg = dc_strdup(tempbuf.as_mut_ptr())
    } else {
        msg = dc_mprintf(
            b"event #%i\x00" as *const u8 as *const libc::c_char,
            event as libc::c_int,
        )
    }
    (*context).cb.expect("non-null function pointer")(
        context,
        event,
        data1 as uintptr_t,
        msg as uintptr_t,
    );
    free(msg as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn dc_log_event_seq(
    mut context: *mut dc_context_t,
    mut event_code: libc::c_int,
    mut sequence_start: *mut libc::c_int,
    mut msg: *const libc::c_char,
    mut va_0: ...
) {
    if context.is_null()
        || sequence_start.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
    {
        return;
    }
    log_vprintf(context, event_code, *sequence_start, msg, va_0);
    *sequence_start = 0i32;
}
#[no_mangle]
pub unsafe extern "C" fn dc_log_error(
    mut context: *mut dc_context_t,
    mut data1: libc::c_int,
    mut msg: *const libc::c_char,
    mut va_1: ...
) {
    log_vprintf(context, 400i32, data1, msg, va_1);
}
#[no_mangle]
pub unsafe extern "C" fn dc_log_warning(
    mut context: *mut dc_context_t,
    mut data1: libc::c_int,
    mut msg: *const libc::c_char,
    mut va_2: ...
) {
    log_vprintf(context, 300i32, data1, msg, va_2);
}
#[no_mangle]
pub unsafe extern "C" fn dc_log_info(
    mut context: *mut dc_context_t,
    mut data1: libc::c_int,
    mut msg: *const libc::c_char,
    mut va_3: ...
) {
    log_vprintf(context, 100i32, data1, msg, va_3);
}

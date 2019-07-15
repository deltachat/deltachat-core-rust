use crate::constants::Event;
use crate::context::Context;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

pub unsafe extern "C" fn dc_log_event(
    context: &Context,
    event_code: Event,
    data1: libc::c_int,
    msg: *const libc::c_char,
    va: ...
) {
    log_vprintf(context, event_code, data1, msg, va);
}

/* Asynchronous "Thread-errors" are reported by the dc_log_error()
function.  These errors must be shown to the user by a bubble or so.

"Normal" errors are usually returned by a special value (null or so) and are
usually not reported using dc_log_error() - its up to the caller to
decide, what should be reported or done.  However, these "Normal" errors
are usually logged by dc_log_warning(). */
unsafe fn log_vprintf(
    context: &Context,
    event: Event,
    data1: libc::c_int,
    msg_format: *const libc::c_char,
    va_0: ::std::ffi::VaList,
) {
    let msg: *mut libc::c_char;
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
    context.call_cb(event, data1 as uintptr_t, msg as uintptr_t);
    free(msg as *mut libc::c_void);
}

pub unsafe extern "C" fn dc_log_event_seq(
    context: &Context,
    event_code: Event,
    sequence_start: *mut libc::c_int,
    msg: *const libc::c_char,
    va_0: ...
) {
    if sequence_start.is_null() {
        return;
    }
    log_vprintf(context, event_code, *sequence_start, msg, va_0);
    *sequence_start = 0i32;
}

pub unsafe extern "C" fn dc_log_error(
    context: &Context,
    data1: libc::c_int,
    msg: *const libc::c_char,
    va_1: ...
) {
    log_vprintf(context, Event::ERROR, data1, msg, va_1);
}

pub unsafe extern "C" fn dc_log_warning(
    context: &Context,
    data1: libc::c_int,
    msg: *const libc::c_char,
    va_2: ...
) {
    log_vprintf(context, Event::WARNING, data1, msg, va_2);
}

pub unsafe extern "C" fn dc_log_info(
    context: &Context,
    data1: libc::c_int,
    msg: *const libc::c_char,
    va_3: ...
) {
    log_vprintf(context, Event::INFO, data1, msg, va_3);
}

#[macro_export]
macro_rules! info {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        info!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        println!("xxx");
        let formatted = format!($msg, $($args),*);
        let formatted_c = $crate::dc_tools::to_cstring(formatted);
        $ctx.call_cb($crate::constants::Event::INFO, $data1 as uintptr_t,
                     formatted_c.as_ptr() as uintptr_t)
    }};
}

#[macro_export]
macro_rules! warn {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        warn!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        let formatted = format!($msg, $($args),*);
        let formatted_c = $crate::dc_tools::to_cstring(formatted);
        $ctx.call_cb($crate::constants::Event::WARNING, $data1 as libc::uintptr_t,
                     formatted_c.as_ptr() as libc::uintptr_t)
    };
}

#[macro_export]
macro_rules! error {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        error!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        let formatted = format!($msg, $($args),*);
        let formatted_c = $crate::dc_tools::to_cstring(formatted);
        $ctx.call_cb($crate::constants::Event::ERROR, $data1 as uintptr_t,
                     formatted_c.as_ptr() as uintptr_t)
    };
}

#[macro_export]
macro_rules! log_event {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        log_event!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $event:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        let formatted = format!($msg, $($args),*);
        let formatted_c = $crate::dc_tools::to_cstring(formatted);
        $ctx.call_cb($event, $data1 as uintptr_t,
                     formatted_c.as_ptr() as uintptr_t)
    };
}

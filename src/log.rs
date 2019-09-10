#[macro_export]
macro_rules! info {
    ($ctx:expr,  $msg:expr) => {
        info!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {
        log_event!($ctx, $crate::constants::Event::INFO, 0, $msg, $($args),*);
    };
}

#[macro_export]
macro_rules! warn {
    ($ctx:expr, $msg:expr) => {
        warn!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {
        log_event!($ctx, $crate::constants::Event::WARNING, 0, $msg, $($args),*);
    };
}

#[macro_export]
macro_rules! error {
    ($ctx:expr, $msg:expr) => {
        error!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {
        log_event!($ctx, $crate::constants::Event::ERROR, 0, $msg, $($args),*);
    };
}

#[macro_export]
macro_rules! log_event {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        log_event!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $event:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        #[allow(unused_unsafe)]
        unsafe {
            let formatted = format!($msg, $($args),*);
            let formatted_c = std::ffi::CString::new(formatted).unwrap();
            $ctx.call_cb($event, $data1 as libc::uintptr_t,
                         formatted_c.as_ptr() as libc::uintptr_t);
    }};
}

#[macro_export]
macro_rules! emit_event {
    ($ctx:expr, $event:expr, $data1:expr, $data2:expr) => {
        $ctx.call_cb($event, $data1 as libc::uintptr_t, $data2 as libc::uintptr_t);
    };
}

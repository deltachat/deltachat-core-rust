#[macro_export]
macro_rules! info {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        info!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        #[allow(unused_unsafe)]
        unsafe {
            let formatted = format!($msg, $($args),*);
            let formatted_c =  std::ffi::CString::new(formatted).unwrap();
            $ctx.call_cb($crate::constants::Event::INFO, $data1 as libc::uintptr_t,
                     formatted_c.as_ptr() as libc::uintptr_t);
    }};
}

#[macro_export]
macro_rules! warn {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        warn!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        #[allow(unused_unsafe)]
        unsafe {
            let formatted = format!($msg, $($args),*);
            let formatted_c = std::ffi::CString::new(formatted).unwrap();
            $ctx.call_cb($crate::constants::Event::WARNING, $data1 as libc::uintptr_t,
                         formatted_c.as_ptr() as libc::uintptr_t);
        }};
}

#[macro_export]
macro_rules! error {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        error!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        #[allow(unused_unsafe)]
        unsafe {
        let formatted = format!($msg, $($args),*);
        let formatted_c = std::ffi::CString::new(formatted).unwrap();
        $ctx.call_cb($crate::constants::Event::ERROR, $data1 as libc::uintptr_t,
                     formatted_c.as_ptr() as libc::uintptr_t);
    }};
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

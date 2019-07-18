#[macro_export]
macro_rules! info {
    ($ctx:expr, $data1:expr, $msg:expr) => {
        info!($ctx, $data1, $msg,)
    };
    ($ctx:expr, $data1:expr, $msg:expr, $($args:expr),* $(,)?) => {
        #[allow(unused_unsafe)]
        unsafe {
            let formatted = format!($msg, $($args),*);
            let formatted_c =  $crate::dc_tools::to_cstring(formatted);
            $ctx.call_cb($crate::constants::Event::INFO, $data1 as libc::uintptr_t,
                     formatted_c as libc::uintptr_t);
            libc::free(formatted_c as *mut libc::c_void);
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
            let formatted_c = $crate::dc_tools::to_cstring(formatted);
            $ctx.call_cb($crate::constants::Event::WARNING, $data1 as libc::uintptr_t,
                         formatted_c as libc::uintptr_t);
            libc::free(formatted_c as *mut libc::c_void) ;
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
        let formatted_c = $crate::dc_tools::to_cstring(formatted);
        $ctx.call_cb($crate::constants::Event::ERROR, $data1 as libc::uintptr_t,
                     formatted_c as libc::uintptr_t);
        libc::free(formatted_c as *mut libc::c_void);
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
            let formatted_c = $crate::dc_tools::to_cstring(formatted);
            $ctx.call_cb($event, $data1 as libc::uintptr_t,
                         formatted_c as libc::uintptr_t);
            libc::free(formatted_c as *mut libc::c_void);
    }};
}

#[macro_export]
macro_rules! info {
    ($ctx:expr,  $msg:expr) => {
        info!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::Event::Info(formatted));
    };
}

#[macro_export]
macro_rules! warn {
    ($ctx:expr, $msg:expr) => {
        warn!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::Event::Warning(formatted));
    };
}

#[macro_export]
macro_rules! error {
    ($ctx:expr, $msg:expr) => {
        error!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::Event::Error(formatted));
    };
}

#[macro_export]
macro_rules! emit_event {
    ($ctx:expr, $event:expr) => {
        $ctx.call_cb($event);
    };
}

//! # Logging
use crate::context::Context;

#[macro_export]
macro_rules! info {
    ($ctx:expr,  $msg:expr) => {
        info!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        let formatted = format!($msg, $($args),*);
        let full = format!("{file}:{line}: {msg}",
                           file = file!(),
                           line = line!(),
                           msg = &formatted);
        emit_event!($ctx, $crate::EventType::Info(full));
    }};
}

#[macro_export]
macro_rules! warn {
    ($ctx:expr, $msg:expr) => {
        warn!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        let formatted = format!($msg, $($args),*);
        let full = format!("{file}:{line}: {msg}",
                           file = file!(),
                           line = line!(),
                           msg = &formatted);
        emit_event!($ctx, $crate::EventType::Warning(full));
    }};
}

#[macro_export]
macro_rules! error {
    ($ctx:expr, $msg:expr) => {
        error!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::EventType::Error(formatted));
    }};
}

#[macro_export]
macro_rules! error_network {
    ($ctx:expr, $msg:expr) => {
        error_network!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::EventType::ErrorNetwork(formatted));
    }};
}

#[macro_export]
macro_rules! emit_event {
    ($ctx:expr, $event:expr) => {
        $ctx.emit_event($event);
    };
}

pub trait LogExt<T, E>
where
    Self: std::marker::Sized,
{
    #[track_caller]
    fn log_err_inner(self, context: &Context, msg: Option<&str>) -> Result<T, E>;

    /// Emits a warning if the receiver contains an Err value.
    ///
    /// Thanks to the [track_caller](https://blog.rust-lang.org/2020/08/27/Rust-1.46.0.html#track_caller)
    /// feature, the location of the caller is printed to the log, just like with the warn!() macro.
    ///
    /// Unfortunately, the track_caller feature does not work on async functions (as of Rust 1.50).
    /// Once it is, you can add `#[track_caller]` to helper functions that use one of the log helpers here
    /// so that the location of the caller can be seen in the log. (this won't work with the macros,
    /// like warn!(), since the file!() and line!() macros don't work with track_caller)  
    /// See https://github.com/rust-lang/rust/issues/78840 for progress on this.
    #[track_caller]
    fn log_err(self, context: &Context, msg: &str) -> Result<T, E> {
        self.log_err_inner(context, Some(msg))
    }

    /// Emits a warning if the receiver contains an Err value and returns an [`Option<T>`].
    ///
    /// Example:
    /// ```text
    /// if let Err(e) = do_something() {
    ///     warn!(context, "{:#}", e);
    /// }
    /// ```
    /// is equivalent to:
    /// ```text
    /// do_something().ok_or_log(context);
    /// ```
    ///
    /// For a note on the `track_caller` feature, see the doc comment on `log_err()`.
    #[track_caller]
    fn ok_or_log(self, context: &Context) -> Option<T> {
        self.log_err_inner(context, None).ok()
    }

    /// Like `ok_or_log()`, but you can pass an extra message that is prepended in the log.
    ///
    /// Example:
    /// ```text
    /// if let Err(e) = do_something() {
    ///     warn!(context, "Something went wrong: {:#}", e);
    /// }
    /// ```
    /// is equivalent to:
    /// ```text
    /// do_something().ok_or_log_msg(context, "Something went wrong");
    /// ```
    /// and is also equivalent to:
    /// ```text
    /// use anyhow::Context as _;
    /// do_something().context("Something went wrong").ok_or_log(context);
    /// ```
    ///
    /// For a note on the `track_caller` feature, see the doc comment on `log_err()`.
    #[track_caller]
    fn ok_or_log_msg(self, context: &Context, msg: &'static str) -> Option<T> {
        self.log_err_inner(context, Some(msg)).ok()
    }
}

impl<T, E: std::fmt::Display> LogExt<T, E> for Result<T, E> {
    #[track_caller]
    fn log_err_inner(self, context: &Context, msg: Option<&str>) -> Result<T, E> {
        if let Err(e) = &self {
            let location = std::panic::Location::caller();

            let separator = if msg.is_none() { "" } else { ": " };
            let msg = msg.unwrap_or_default();

            // We are using Anyhow's .context() and to show the inner error, too, we need the {:#}:
            let full = format!(
                "{file}:{line}: {msg}{separator}{e:#}",
                file = location.file(),
                line = location.line(),
                msg = msg,
                separator = separator,
                e = e
            );
            // We can't use the warn!() macro here as the file!() and line!() macros
            // don't work with #[track_caller]
            emit_event!(context, crate::EventType::Warning(full));
        };
        self
    }
}

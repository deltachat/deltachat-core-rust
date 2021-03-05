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

pub trait LogExt<T> {
    /// Emits a warning if the receiver contains an Err value.
    ///
    /// Returns an [`Option<T>`] with the `Ok(_)` value, if any:
    /// - You won't get any warnings about unused results but can still use the value if you need it
    /// - This prevents the same warning from being printed to the log multiple times
    ///
    /// Example: This:
    /// ```
    /// if let Err(e) = do_something() {
    ///     warn!(context, "{:#}", e);
    /// }
    /// ```
    /// can be replaced with:
    /// ```
    /// do_something().log(context);
    /// ```
    ///
    /// Thanks to the [track_caller](https://blog.rust-lang.org/2020/08/27/Rust-1.46.0.html#track_caller)
    /// feature, the location of the caller is printed to the log, just like with the warn!() macro.
    ///
    /// Unfortunately, the track_caller feature does not work on async functions (as of Rust 1.50).
    /// This means that we can't make our logs even better by adding `#[track_caller]` to our helper
    /// functions.  
    /// See https://github.com/rust-lang/rust/issues/78840 for progress on this.
    #[track_caller]
    fn log(self, context: &Context) -> Option<T>;

    /// Like `log()`, but you can pass an extra string message.
    ///
    /// Example: This:
    /// ```
    /// if let Err(e) = do_something() {
    ///     warn!(context, "Something went wrong: {:#}", e);
    /// }
    /// ```
    /// can be replaced with:
    /// ```
    /// do_something().log_m(context, "Something went wrong");
    /// ```
    #[track_caller]
    fn log_m(self, context: &Context, msg: &str) -> Option<T>;
}

impl<T> LogExt<T> for anyhow::Result<T> {
    #[track_caller]
    fn log(self, context: &Context) -> Option<T> {
        match self {
            Err(e) => {
                let location = std::panic::Location::caller();
                // We are using Anyhow's .context() and to show the inner error, too, we need the {:#}:
                let full = format!(
                    "{file}:{line}: {e:#}",
                    file = location.file(),
                    line = location.line(),
                    e = e
                );
                // We can't use the warn!() macro here as the file!() and line!() macros
                // don't work with #[track_caller]
                emit_event!(context, crate::EventType::Warning(full));
                None
            }
            Ok(v) => Some(v),
        }
    }

    #[track_caller]
    fn log_m(self, context: &Context, msg: &str) -> Option<T> {
        match self {
            Err(e) => {
                let location = std::panic::Location::caller();
                // We are using Anyhow's .context() and to show the inner error, too, we need the {:#}:
                let full = format!(
                    "{file}:{line}: {msg}: {e:#}",
                    file = location.file(),
                    line = location.line(),
                    msg = msg,
                    e = e
                );
                // We can't use the warn!() macro here as the file!() and line!() macros
                // don't work with #[track_caller]
                emit_event!(context, crate::EventType::Warning(full));
                None
            }
            Ok(v) => Some(v),
        }
    }
}

// #[track_caller]
// fn do_something_with_sql(context: &Context) {
//     // context
//     //     .sql
//     //     .table_exists("config")
//     //     .await
//     goes_wrong().log_m(context, "Can't do something");
// }

// fn goes_wrong() -> anyhow::Result<()> {
//     Err(anyhow::format_err!("went wrong"))
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[async_std::test]
//     async fn test_log() {
//         let t = crate::test_utils::TestContext::new_alice().await;
//         t.sql.close().await;
//         do_something_with_sql(&t);
//     }
// }

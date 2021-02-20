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

pub(crate) trait LogExt<T> {
    /// Emits a warning if the receiver contained an Err value.
    ///
    /// Returns an [`Option<T>`] with the `Ok(_)` value, if any:
    /// - You won't get any warnings about unused results but can still use the value if you need it
    /// - This prevents the same warning from being printed to the log multiple times
    ///
    /// Thanks to the [track_caller](https://blog.rust-lang.org/2020/08/27/Rust-1.46.0.html#track_caller)
    /// feature, the location of the caller is printed to the log, just like with the warn!() macro.
    #[track_caller]
    fn log(self, context: &Context) -> Option<T>;
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
                // don't work well with #[track_caller]
                emit_event!(context, crate::EventType::Warning(full));
                None
            }
            Ok(v) => Some(v),
        }
    }
}

// TODO remove test or make it work OK
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;
    use anyhow::format_err;

    #[async_std::test]
    async fn test_log() {
        let t = TestContext::new_alice().await;
        let res: anyhow::Result<()> = Err(format_err!("testerror").context("Some context"));
        res.log(&t);
    }
}

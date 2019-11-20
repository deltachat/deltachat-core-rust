//! # Logging support

use std::fmt;
use std::io::prelude::*;
use std::path::PathBuf;

/// A logger for a [Context].
#[derive(Debug)]
pub struct Logger {
    logdir: PathBuf,
    logfile: std::ffi::OsString,
    file_handle: std::fs::File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Info => write!(f, "I"),
            LogLevel::Warning => write!(f, "W"),
            LogLevel::Error => write!(f, "E"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Callsite<'a> {
    pub module: &'a str,
    pub file: &'a str,
    pub line: u32,
}

impl fmt::Display for Callsite<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} in {}", self.file, self.line, self.module)
    }
}

impl Logger {
    pub fn new(logdir: PathBuf) -> Result<Logger, std::io::Error> {
        let fname = format!(
            "{}.log",
            chrono::offset::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        );
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(logdir.join(&fname))?;
        Ok(Logger {
            logdir,
            logfile: fname.into(),
            file_handle: file,
        })
    }

    pub fn log(
        &mut self,
        level: LogLevel,
        callsite: Callsite,
        msg: &str,
    ) -> Result<(), std::io::Error> {
        write!(&mut self.file_handle, "{} [{}]: {}\n", level, callsite, msg)
    }

    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        self.file_handle.flush()
    }
}

#[macro_export]
macro_rules! callsite {
    () => {
        $crate::log::Callsite {
            module: module_path!(),
            file: file!(),
            line: line!(),
        }
    };
}

#[macro_export]
macro_rules! info {
    ($ctx:expr,  $msg:expr) => {
        info!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::Event::Info(formatted));
    }};
}

#[macro_export]
macro_rules! warn {
    ($ctx:expr, $msg:expr) => {
        warn!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::Event::Warning(formatted));
    }};
}

#[macro_export]
macro_rules! error {
    ($ctx:expr, $msg:expr) => {
        error!($ctx, $msg,)
    };
    ($ctx:expr, $msg:expr, $($args:expr),* $(,)?) => {{
        let formatted = format!($msg, $($args),*);
        emit_event!($ctx, $crate::Event::Error(formatted));
    }};
}

#[macro_export]
macro_rules! emit_event {
    ($ctx:expr, $event:expr) => {
        $ctx.call_cb($event);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_logging() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let mut logger = Logger::new(dir.to_path_buf()).unwrap();
        logger.log(LogLevel::Info, callsite!(), "foo").unwrap();
        logger.log(LogLevel::Warning, callsite!(), "bar").unwrap();
        logger.log(LogLevel::Error, callsite!(), "baz").unwrap();
        logger.flush();
        let log = std::fs::read_to_string(logger.logdir.join(logger.logfile)).unwrap();
        println!("{}", log);
        let lines: Vec<&str> = log.lines().collect();

        assert!(lines[0].starts_with("I"));
        assert!(lines[0].contains("src/log.rs"));
        assert!(lines[0].contains("deltachat::log::tests"));
        assert!(lines[0].contains("foo"));

        assert!(lines[1].starts_with("W"));
        assert!(lines[1].contains("src/log.rs"));
        assert!(lines[1].contains("deltachat::log::tests"));
        assert!(lines[1].contains("bar"));

        assert!(lines[2].starts_with("E"));
        assert!(lines[2].contains("src/log.rs"));
        assert!(lines[2].contains("deltachat::log::tests"));
        assert!(lines[2].contains("baz"));
    }
}

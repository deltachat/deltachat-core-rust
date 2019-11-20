//! # Logging support

use std::io;
use std::fmt;
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

/// A logger for a [Context].
#[derive(Debug)]
pub struct Logger {
    logdir: PathBuf,
    logfile: String,
    file_handle: fs::File,
    max_files: u32,
    max_filesize: usize,
    bytes_written: usize,
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
    pub fn new(logdir: PathBuf) -> Result<Logger, io::Error> {
        let (fname, file) = Self::open(&logdir)?;
        let max_files = 5;
        Self::prune(&logdir, max_files);
        Ok(Logger {
            logdir,
            logfile: fname,
            file_handle: file,
            max_files,
            max_filesize: 4 * 1024 * 1024, // 4 Mb
            bytes_written: 0,
        })
    }

    /// Opens a new logfile, returning a tuple of (file_name, file_handle).
    ///
    /// This tries to create a new logfile based on the current time,
    /// creating .0.log, .1.log etc if this file already exists (up to 32).
    fn open(logdir: &Path) -> Result<(String, fs::File), io::Error> {
        let basename =
            chrono::offset::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let mut fname = format!("{}.log", &basename);
        let mut counter = 0;
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(logdir.join(&fname))
            {
                Ok(file) => {
                    return Ok((fname, file));
                }
                Err(e) => {
                    if counter >= 32 {
                        return Err(e);
                    } else {
                        counter += 1;
                        fname = format!("{}.{}.log", &basename, counter);
                        continue;
                    }
                }
            }
        }
    }

    /// Cleans up old logfiles.
    fn prune(logdir: &Path, max_files: u32) {
        // TODO
    }

    pub fn log(
        &mut self,
        level: LogLevel,
        callsite: Callsite,
        msg: &str,
    ) -> Result<(), std::io::Error> {
        if self.bytes_written > self.max_filesize {
            let (fname, handle) = Self::open(&self.logdir)?;
            self.logfile = fname;
            self.file_handle = handle;
            Self::prune(&self.logdir, self.max_files);
        }
        let msg = format!("{} [{}]: {}\n", level, callsite, msg);
        self.file_handle.write_all(msg.as_bytes())?;
        self.bytes_written += msg.len();
        Ok(())
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
        logger.flush().unwrap();
        let log = fs::read_to_string(logger.logdir.join(logger.logfile)).unwrap();
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

    #[test]
    fn test_reopen_logfile() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let mut logger = Logger::new(dir.to_path_buf()).unwrap();
        logger.max_filesize = 5;

        let fname0 = logger.logfile.clone();
        assert!(fname0.ends_with(".log"));
        logger
            .log(LogLevel::Info, callsite!(), "more than 5 bytes are written")
            .unwrap();
        logger.log(LogLevel::Info, callsite!(), "2nd msg").unwrap();
        let fname1 = logger.logfile.clone();
        assert!(fname1.ends_with(".1.log"));
        assert_ne!(fname0, fname1);
        let log0 = fs::read_to_string(logger.logdir.join(&fname0)).unwrap();
        assert!(log0.contains("more than 5 bytes are written"));
        let log1 = fs::read_to_string(logger.logdir.join(&fname1)).unwrap();
        assert!(log1.contains("2nd msg"));

        let mut count = 0;
        loop {
            if count > 40 {
                assert!(false, "Failed to find error");
            }
            count += 1;
            match logger.log(LogLevel::Info, callsite!(), "more reopens please") {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    }
}

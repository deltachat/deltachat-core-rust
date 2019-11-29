//! # Logging support

use std::fmt;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

/// A logger for a [Context].
#[derive(Debug)]
pub struct Logger {
    created: std::time::Instant,
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
    pub file: &'a str,
    pub line: u32,
}

impl fmt::Display for Callsite<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

impl Logger {
    pub fn new(logdir: PathBuf) -> Result<Logger, io::Error> {
        let (fname, file) = Self::open(&logdir)?;
        let max_files = 5;
        Self::prune(&logdir, max_files)?;
        Ok(Logger {
            created: std::time::Instant::now(),
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
        let mut fname = sanitize_filename::sanitize(format!("{}.log", &basename));
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
                        fname =
                            sanitize_filename::sanitize(format!("{}-{}.log", &basename, counter));
                        continue;
                    }
                }
            }
        }
    }

    /// Cleans up old logfiles.
    fn prune(logdir: &Path, max_files: u32) -> Result<(), io::Error> {
        let mut names: Vec<std::ffi::OsString> = Vec::new();
        for dirent in fs::read_dir(logdir)? {
            names.push(dirent?.file_name());
        }
        // Sorting like this sorts: 23.log, 24.1.log, 24.2.log,
        // 24.log, 25.log.  That is 24.log is out of sequence.  Oh well.
        names.sort();
        names.reverse();
        while names.len() > max_files as usize {
            if let Some(name) = names.pop() {
                fs::remove_file(logdir.join(name))?;
            }
        }
        Ok(())
    }

    pub fn log(
        &mut self,
        level: LogLevel,
        callsite: Callsite,
        msg: &str,
    ) -> Result<(), std::io::Error> {
        if self.bytes_written > self.max_filesize {
            self.flush()?;
            let (fname, handle) = Self::open(&self.logdir)?;
            self.logfile = fname;
            self.file_handle = handle;
            Self::prune(&self.logdir, self.max_files)?;
        }
        let thread = std::thread::current();
        let msg = format!(
            "{time:8.2} {level} {thid:?}/{thname} [{callsite}]: {msg}\n",
            time = self.created.elapsed().as_secs_f64(),
            level = level,
            thid = thread.id(),
            thname = thread.name().unwrap_or("unnamed"),
            callsite = callsite,
            msg = msg,
        );
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
        if let Ok(mut logger) = $ctx.logger.write() {
            logger.log($crate::log::LogLevel::Info, callsite!(), &formatted).ok();
        }
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
        if let Ok(mut logger) = $ctx.logger.write() {
            logger.log($crate::log::LogLevel::Warning, callsite!(), &formatted).ok();
        }
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
        if let Ok(mut logger) = $ctx.logger.write() {
            logger.log($crate::log::LogLevel::Error, callsite!(), &formatted).ok();
        }
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

        assert!(lines[0].contains(" I "));
        assert!(lines[0].contains(format!("{:?}", std::thread::current().id()).as_str()));
        assert!(lines[0]
            .contains(format!("{}", std::thread::current().name().unwrap_or("unnamed")).as_str()));
        assert!(lines[0].contains(&format!("src{}log.rs", std::path::MAIN_SEPARATOR)));
        assert!(lines[0].contains("foo"));

        assert!(lines[1].contains(" W "));
        assert!(lines[1].contains(format!("{:?}", std::thread::current().id()).as_str()));
        assert!(lines[1]
            .contains(format!("{}", std::thread::current().name().unwrap_or("unnamed")).as_str()));
        assert!(lines[1].contains(&format!("src{}log.rs", std::path::MAIN_SEPARATOR)));
        assert!(lines[1].contains("bar"));

        assert!(lines[2].contains(" E "));
        assert!(lines[2].contains(format!("{:?}", std::thread::current().id()).as_str()));
        assert!(lines[2]
            .contains(format!("{}", std::thread::current().name().unwrap_or("unnamed")).as_str()));
        assert!(lines[2].contains(&format!("src{}log.rs", std::path::MAIN_SEPARATOR)));
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
        assert!(fname1.ends_with("-1.log"));
        assert_ne!(fname0, fname1);
        let log0 = fs::read_to_string(logger.logdir.join(&fname0)).unwrap();
        assert!(log0.contains("more than 5 bytes are written"));
        let log1 = fs::read_to_string(logger.logdir.join(&fname1)).unwrap();
        assert!(log1.contains("2nd msg"));
    }

    #[test]
    fn test_prune() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        Logger::new(dir.to_path_buf()).unwrap();
        Logger::new(dir.to_path_buf()).unwrap();
        Logger::new(dir.to_path_buf()).unwrap();
        Logger::new(dir.to_path_buf()).unwrap();
        let dirents0: Vec<fs::DirEntry> = fs::read_dir(&dir).unwrap().map(|r| r.unwrap()).collect();
        assert_eq!(dirents0.len(), 4);
        Logger::prune(&dir, 3).unwrap();
        let dirents1: Vec<fs::DirEntry> = fs::read_dir(&dir).unwrap().map(|r| r.unwrap()).collect();
        assert_eq!(dirents1.len(), 3);
    }
}

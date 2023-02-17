use std::path::PathBuf;
use std::time::Duration;

use r2d2::ManageConnection;
use rusqlite::{Connection, Error, OpenFlags};

#[derive(Debug)]
pub struct ConnectionManager {
    /// Database file path.
    path: PathBuf,

    /// SQLite open flags.
    flags: rusqlite::OpenFlags,

    /// SQLCipher database passphrase.
    /// Empty string if database is not encrypted.
    passphrase: String,
}

impl ConnectionManager {
    /// Creates new connection manager.
    pub fn new(path: PathBuf, passphrase: String) -> Self {
        let mut flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;
        flags.insert(OpenFlags::SQLITE_OPEN_READ_WRITE);
        flags.insert(OpenFlags::SQLITE_OPEN_CREATE);

        Self {
            path,
            flags,
            passphrase,
        }
    }
}

impl ManageConnection for ConnectionManager {
    type Connection = Connection;
    type Error = Error;

    fn connect(&self) -> Result<Connection, Error> {
        let conn = Connection::open_with_flags(&self.path, self.flags)?;
        conn.execute_batch(&format!(
            "PRAGMA cipher_memory_security = OFF; -- Too slow on Android
             PRAGMA secure_delete=on;
             PRAGMA busy_timeout = {};
             PRAGMA temp_store=memory; -- Avoid SQLITE_IOERR_GETTEMPPATH errors on Android
             PRAGMA foreign_keys=on;
             ",
            Duration::from_secs(60).as_millis()
        ))?;
        conn.pragma_update(None, "key", &self.passphrase)?;
        // Try to enable auto_vacuum. This will only be
        // applied if the database is new or after successful
        // VACUUM, which usually happens before backup export.
        // When auto_vacuum is INCREMENTAL, it is possible to
        // use PRAGMA incremental_vacuum to return unused
        // database pages to the filesystem.
        conn.pragma_update(None, "auto_vacuum", "INCREMENTAL".to_string())?;

        conn.pragma_update(None, "journal_mode", "WAL".to_string())?;
        // Default synchronous=FULL is much slower. NORMAL is sufficient for WAL mode.
        conn.pragma_update(None, "synchronous", "NORMAL".to_string())?;

        Ok(conn)
    }

    fn is_valid(&self, _conn: &mut Connection) -> Result<(), Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Connection) -> bool {
        false
    }
}

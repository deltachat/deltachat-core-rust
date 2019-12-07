//! # Key-value configuration management

use derive_deref::{Deref, DerefMut};
use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumIter, EnumProperty, EnumString};

use crate::blob::BlobObject;
use crate::constants::DC_VERSION_STR;
use crate::context::Context;
use crate::dc_tools::*;
use crate::job::*;
use crate::sql;
use crate::stock::StockMessage;

/// The available configuration keys.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, AsRefStr, EnumIter, EnumProperty,
)]
#[strum(serialize_all = "snake_case")]
pub enum Config {
    Addr,
    MailServer,
    MailUser,
    MailPw,
    MailPort,
    ImapCertificateChecks,
    SendServer,
    SendUser,
    SendPw,
    SendPort,
    SmtpCertificateChecks,
    ServerFlags,

    #[strum(props(default = "INBOX"))]
    ImapFolder,

    Displayname,
    Selfstatus,
    Selfavatar,

    #[strum(props(default = "0"))]
    BccSelf,

    #[strum(props(default = "1"))]
    E2eeEnabled,

    #[strum(props(default = "1"))]
    MdnsEnabled,

    #[strum(props(default = "1"))]
    InboxWatch,

    #[strum(props(default = "1"))]
    SentboxWatch,

    #[strum(props(default = "1"))]
    MvboxWatch,

    #[strum(props(default = "1"))]
    MvboxMove,

    #[strum(props(default = "0"))] // also change ShowEmails.default() on changes
    ShowEmails,

    SaveMimeHeaders,
    ConfiguredAddr,
    ConfiguredMailServer,
    ConfiguredMailUser,
    ConfiguredMailPw,
    ConfiguredMailPort,
    ConfiguredMailSecurity,
    ConfiguredImapCertificateChecks,
    ConfiguredSendServer,
    ConfiguredSendUser,
    ConfiguredSendPw,
    ConfiguredSendPort,
    ConfiguredSmtpCertificateChecks,
    ConfiguredServerFlags,
    ConfiguredSendSecurity,
    ConfiguredE2EEEnabled,
    Configured,

    #[strum(serialize = "sys.version")]
    SysVersion,

    #[strum(serialize = "sys.msgsize_max_recommended")]
    SysMsgsizeMaxRecommended,

    #[strum(serialize = "sys.config_keys")]
    SysConfigKeys,
}

/// A trait defining a [Context]-wide configuration item.
///
/// Configuration items are stored in database of a [Context].  Most
/// configuration items are newtypes which implement [std::ops::Deref]
/// and [std::ops::DerefMut] though this is not required.  However
/// what **is required** for the struct to implement
/// [rusqlite::ToSql] and [rusqlite::types::FromSql].
pub trait ConfigItem {
    /// Returns the name of the key used in the SQLite database.
    fn keyname() -> &'static str;

    /// Loads the configuration item from the [Context]'s database.
    ///
    /// If the configuration item is not available in the database,
    /// `None` will be returned.
    fn load(context: &Context) -> Result<Option<Self>, sql::Error>
    where
        Self: std::marker::Sized + rusqlite::types::FromSql,
    {
        context
            .sql
            .query_row(
                "SELECT value FROM config WHERE keyname=?;",
                params!(Self::keyname()),
                |row| row.get(0),
            )
            .or_else(|err| match err {
                sql::Error::Sql(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                e => Err(e),
            })
    }

    /// Stores the configuration item in the [Context]'s database.
    fn store(&self, context: &Context) -> Result<(), sql::Error>
    where
        Self: rusqlite::ToSql,
    {
        if context.sql.exists(
            "select value FROM config WHERE keyname=?;",
            params!(Self::keyname()),
        )? {
            context.sql.execute(
                "UPDATE config SET value=? WHERE keyname=?",
                params![&self, Self::keyname()],
            )?;
        } else {
            context.sql.execute(
                "INSERT INTO config (keyname, value) VALUES (?, ?)",
                params![Self::keyname(), &self],
            )?;
        }
        Ok(())
    }

    /// Removes the configuration item from the [Context]'s database.
    fn delete(context: &Context) -> Result<(), sql::Error> {
        context
            .sql
            .execute(
                "DELETE FROM config WHERE keyname=?",
                params![Self::keyname()],
            )
            .and(Ok(()))
    }
}

/// Configuration item: display address for this account.
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub struct Addr(pub String);

impl rusqlite::ToSql for Addr {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        self.0.to_sql()
    }
}

impl rusqlite::types::FromSql for Addr {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        value.as_str().map(|v| Addr(v.to_string()))
    }
}

impl ConfigItem for Addr {
    fn keyname() -> &'static str {
        "addr"
    }
}

/// Configuration item:
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub struct MailServer(pub String);

impl rusqlite::ToSql for MailServer {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        self.0.to_sql()
    }
}

impl rusqlite::types::FromSql for MailServer {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        value.as_str().map(|v| MailServer(v.to_string()))
    }
}

impl ConfigItem for MailServer {
    fn keyname() -> &'static str {
        "mail_server"
    }
}

/// Configuration item:
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub struct MailUser(pub String);
// XXX TODO

/// Configuration item: whether to watch the INBOX folder for changes.
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub struct InboxWatch(pub bool);

impl Default for InboxWatch {
    fn default() -> Self {
        InboxWatch(true)
    }
}

impl rusqlite::ToSql for InboxWatch {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        // Column affinity is "text" so gets stored as string by SQLite.
        let obj = rusqlite::types::Value::Integer(self.0 as i64);
        Ok(rusqlite::types::ToSqlOutput::Owned(obj))
    }
}

impl rusqlite::types::FromSql for InboxWatch {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let str_to_int = |s: &str| {
            s.parse::<i64>()
                .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
        };
        let int_to_bool = |i| match i {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(rusqlite::types::FromSqlError::OutOfRange(v)),
        };
        value
            .as_str()
            .and_then(str_to_int)
            .and_then(int_to_bool)
            .map(InboxWatch)
    }
}

impl ConfigItem for InboxWatch {
    fn keyname() -> &'static str {
        "inbox_watch"
    }
}

impl Context {
    /// Get a configuration key. Returns `None` if no value is set, and no default value found.
    pub fn get_config(&self, key: Config) -> Option<String> {
        let value = match key {
            Config::Selfavatar => {
                let rel_path = self.sql.get_raw_config(self, key);
                rel_path.map(|p| dc_get_abs_path(self, &p).to_string_lossy().into_owned())
            }
            Config::SysVersion => Some((&*DC_VERSION_STR).clone()),
            Config::SysMsgsizeMaxRecommended => Some(format!("{}", 24 * 1024 * 1024 / 4 * 3)),
            Config::SysConfigKeys => Some(get_config_keys_string()),
            _ => self.sql.get_raw_config(self, key),
        };

        if value.is_some() {
            return value;
        }

        // Default values
        match key {
            Config::Selfstatus => Some(self.stock_str(StockMessage::StatusLine).into_owned()),
            _ => key.get_str("default").map(|s| s.to_string()),
        }
    }

    pub fn get_config_int(&self, key: Config) -> i32 {
        self.get_config(key)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default()
    }

    pub fn get_config_bool(&self, key: Config) -> bool {
        self.get_config_int(key) != 0
    }

    /// Set the given config key.
    /// If `None` is passed as a value the value is cleared and set to the default if there is one.
    pub fn set_config(&self, key: Config, value: Option<&str>) -> crate::sql::Result<()> {
        match key {
            Config::Selfavatar if value.is_some() => {
                let blob = BlobObject::create_from_path(&self, value.unwrap())?;
                self.sql.set_raw_config(self, key, Some(blob.as_name()))
            }
            Config::InboxWatch => {
                let ret = self.sql.set_raw_config(self, key, value);
                interrupt_inbox_idle(self, true);
                ret
            }
            Config::SentboxWatch => {
                let ret = self.sql.set_raw_config(self, key, value);
                interrupt_sentbox_idle(self);
                ret
            }
            Config::MvboxWatch => {
                let ret = self.sql.set_raw_config(self, key, value);
                interrupt_mvbox_idle(self);
                ret
            }
            Config::Selfstatus => {
                let def = self.stock_str(StockMessage::StatusLine);
                let val = if value.is_none() || value.unwrap() == def {
                    None
                } else {
                    value
                };

                self.sql.set_raw_config(self, key, val)
            }
            _ => self.sql.set_raw_config(self, key, value),
        }
    }
}

/// Returns all available configuration keys concated together.
fn get_config_keys_string() -> String {
    let keys = Config::iter().fold(String::new(), |mut acc, key| {
        acc += key.as_ref();
        acc += " ";
        acc
    });

    format!(" {} ", keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    use std::str::FromStr;
    use std::string::ToString;

    use lazy_static::lazy_static;

    lazy_static! {
        static ref TC: TestContext = dummy_context();
    }

    #[test]
    fn test_to_string() {
        assert_eq!(Config::MailServer.to_string(), "mail_server");
        assert_eq!(Config::from_str("mail_server"), Ok(Config::MailServer));

        assert_eq!(Config::SysConfigKeys.to_string(), "sys.config_keys");
        assert_eq!(
            Config::from_str("sys.config_keys"),
            Ok(Config::SysConfigKeys)
        );
    }

    #[test]
    fn test_default_prop() {
        assert_eq!(Config::ImapFolder.get_str("default"), Some("INBOX"));
    }

    #[test]
    fn test_selfavatar() -> failure::Fallible<()> {
        let t = dummy_context();
        let avatar_src = t.dir.path().join("avatar.jpg");
        std::fs::write(&avatar_src, b"avatar")?;
        let avatar_blob = t.ctx.get_blobdir().join("avatar.jpg");
        assert!(!avatar_blob.exists());
        t.ctx
            .set_config(Config::Selfavatar, Some(&avatar_src.to_str().unwrap()))?;
        assert!(avatar_blob.exists());
        assert_eq!(std::fs::read(&avatar_blob)?, b"avatar");
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar);
        assert_eq!(avatar_cfg, avatar_blob.to_str().map(|s| s.to_string()));
        Ok(())
    }

    #[test]
    fn test_selfavatar_in_blobdir() -> failure::Fallible<()> {
        let t = dummy_context();
        let avatar_src = t.ctx.get_blobdir().join("avatar.jpg");
        std::fs::write(&avatar_src, b"avatar")?;
        t.ctx
            .set_config(Config::Selfavatar, Some(&avatar_src.to_str().unwrap()))?;
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar);
        assert_eq!(avatar_cfg, avatar_src.to_str().map(|s| s.to_string()));
        Ok(())
    }

    #[test]
    fn test_inbox_watch() {
        // Loading from context when it is not in the DB.
        let val = InboxWatch::load(&TC.ctx).unwrap();
        assert_eq!(val, None);

        // Create in-memory from default.
        let mut val = InboxWatch::default();
        assert_eq!(*val, true);

        // Assign using deref.
        *val = false;
        assert_eq!(*val, false);

        // Construct newtype directly.
        let val = InboxWatch(false);
        assert_eq!(*val, false);

        // Helper to query raw DB value.
        let query_db_raw = || {
            TC.ctx
                .sql
                .query_row(
                    "SELECT value FROM config WHERE KEYNAME=?",
                    params![InboxWatch::keyname()],
                    |row| row.get::<_, String>(0),
                )
                .unwrap()
        };

        // Save (non-default) value to the DB.
        InboxWatch(false).store(&TC.ctx).unwrap();
        assert_eq!(query_db_raw(), "0");
        let val = InboxWatch::load(&TC.ctx).unwrap().unwrap();
        assert_eq!(val, InboxWatch(false));

        // Save true (aka default) value to the DB.
        InboxWatch(true).store(&TC.ctx).unwrap();
        assert_eq!(query_db_raw(), "1");
        let val = InboxWatch::load(&TC.ctx).unwrap().unwrap();
        assert_eq!(val, InboxWatch(true));

        // Delete the value from the DB.
        InboxWatch::delete(&TC.ctx).unwrap();
        assert!(!TC
            .ctx
            .sql
            .exists(
                "SELECT value FROM config WHERE KEYNAME=?",
                params![InboxWatch::keyname()],
            )
            .unwrap());
        let val = InboxWatch::load(&TC.ctx).unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn test_addr() {
        // In-memory creation
        let val = Addr("me@example.com".into());
        assert_eq!(*val, "me@example.com");

        // Load when DB is empty.
        let val = Addr::load(&TC.ctx).unwrap();
        assert_eq!(val, None);

        // Store and load.
        Addr("me@example.com".into()).store(&TC.ctx).unwrap();
        let val = Addr::load(&TC.ctx).unwrap();
        assert_eq!(val, Some(Addr("me@example.com".into())));

        // Delete
        Addr::delete(&TC.ctx).unwrap();
        assert_eq!(Addr::load(&TC.ctx).unwrap(), None);
    }
}

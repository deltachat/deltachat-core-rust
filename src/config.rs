//! # Key-value configuration management

use std::str::FromStr;

use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumDiscriminants, EnumIter, EnumString};

use crate::blob::BlobObject;
use crate::constants::DC_VERSION_STR;
use crate::context::Context;
use crate::error::Error;
use crate::job::*;
use crate::stock::StockMessage;

/// The available configuration items.
///
/// There is also an enum called `ConfigKey` which has the same enum
/// variants but does not carry any data.
#[derive(
    Debug,
    Clone,
    Display,
    PartialEq,
    Eq,
    EnumDiscriminants,
    EnumProperty,
    EnumString,
    EnumIter,
    AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(
    name(ConfigKey),
    derive(Display, EnumString),
    strum(serialize_all = "snake_case")
)]
pub enum ConfigItem {
    Addr(String),
    BccSelf(String),
    Configured(String),
    ConfiguredAddr(String),
    ConfiguredE2EEEnabled(String),
    ConfiguredImapCertificateChecks(String),
    ConfiguredMailPort(String),
    ConfiguredMailPw(String),
    ConfiguredMailSecurity(String),
    ConfiguredMailServer(String),
    ConfiguredMailUser(String),
    ConfiguredSendPort(String),
    ConfiguredSendPw(String),
    ConfiguredSendSecurity(String),
    ConfiguredSendServer(String),
    ConfiguredSendUser(String),
    ConfiguredServerFlags(String),
    ConfiguredSmtpCertificateChecks(String),
    Displayname(String),
    E2eeEnabled(String),
    ImapCertificateChecks(String),
    ImapFolder(String),
    InboxWatch(bool),
    MailPort(String),
    MailPw(String),
    MailServer(String),
    MailUser(String),
    MdnsEnabled(String),
    MvboxMove(String),
    MvboxWatch(String),
    SaveMimeHeaders(String),
    Selfavatar(String),
    Selfstatus(String),
    SendPort(String),
    SendPw(String),
    SendServer(String),
    SendUser(String),
    SentboxWatch(String),
    ServerFlags(String),
    ShowEmails(String),
    SmtpCertificateChecks(String),
    #[strum(serialize = "sys.config_keys")]
    SysConfigKeys(String),
    #[strum(serialize = "sys.msgsize_max_recommended")]
    SysMsgsizeMaxRecommended(String),
    #[strum(serialize = "sys.version")]
    SysVersion(String),
}

// Transitional: support the old name for ConfigKey.
pub type Config = ConfigKey;

impl ConfigKey {
    /// Creates a [ConfigKey] from a string.
    ///
    /// Use this rather than [ConfigKey::from_str].
    pub fn from_key_str(s: &str) -> Result<ConfigKey, strum::ParseError> {
        match s {
            "sys.config_keys" => Ok(ConfigKey::SysConfigKeys),
            "sys.msgsize_max_recommended" => Ok(ConfigKey::SysMsgsizeMaxRecommended),
            "sys.version" => Ok(ConfigKey::SysVersion),
            _ => ConfigKey::from_str(s),
        }
    }

    /// Default values for configuration options.
    ///
    /// These are returned in case there is no value stored in the
    /// database.
    fn default_item(&self, context: &Context) -> Option<ConfigItem> {
        match self {
            Self::BccSelf => Some(ConfigItem::BccSelf(String::from("1"))),
            Self::E2eeEnabled => Some(ConfigItem::E2eeEnabled(String::from("1"))),
            Self::ImapFolder => Some(ConfigItem::ImapFolder(String::from("INBOX"))),
            Self::InboxWatch => Some(ConfigItem::InboxWatch(true)),
            Self::MdnsEnabled => Some(ConfigItem::MdnsEnabled(String::from("1"))),
            Self::MvboxMove => Some(ConfigItem::MvboxMove(String::from("1"))),
            Self::MvboxWatch => Some(ConfigItem::MvboxWatch(String::from("1"))),
            Self::Selfstatus => Some(ConfigItem::Selfstatus(
                context.stock_str(StockMessage::StatusLine).into_owned(),
            )),
            Self::SentboxWatch => Some(ConfigItem::SentboxWatch(String::from("1"))),
            Self::ShowEmails => Some(ConfigItem::ShowEmails(String::from("0"))),
            Self::SysConfigKeys => Some(ConfigItem::SysConfigKeys(get_config_keys_string())),
            Self::SysMsgsizeMaxRecommended => Some(ConfigItem::SysMsgsizeMaxRecommended(format!(
                "{}",
                24 * 1024 * 1024 / 4 * 3
            ))),
            Self::SysVersion => Some(ConfigItem::SysVersion((&*DC_VERSION_STR).clone())),
            _ => None,
        }
    }
}

impl rusqlite::types::ToSql for ConfigItem {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let sql_obj = match self {
            ConfigItem::Selfavatar(value) => {
                let rel_path = std::fs::canonicalize(value)
                    .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
                rusqlite::types::Value::Text(rel_path.to_string_lossy().into_owned())
            }
            ConfigItem::InboxWatch(value) => rusqlite::types::Value::Integer(*value as i64),
            ConfigItem::Addr(value)
            | ConfigItem::BccSelf(value)
            | ConfigItem::Configured(value)
            | ConfigItem::ConfiguredAddr(value)
            | ConfigItem::ConfiguredE2EEEnabled(value)
            | ConfigItem::ConfiguredImapCertificateChecks(value)
            | ConfigItem::ConfiguredMailPort(value)
            | ConfigItem::ConfiguredMailPw(value)
            | ConfigItem::ConfiguredMailSecurity(value)
            | ConfigItem::ConfiguredMailServer(value)
            | ConfigItem::ConfiguredMailUser(value)
            | ConfigItem::ConfiguredSendPort(value)
            | ConfigItem::ConfiguredSendPw(value)
            | ConfigItem::ConfiguredSendSecurity(value)
            | ConfigItem::ConfiguredSendServer(value)
            | ConfigItem::ConfiguredSendUser(value)
            | ConfigItem::ConfiguredServerFlags(value)
            | ConfigItem::ConfiguredSmtpCertificateChecks(value)
            | ConfigItem::Displayname(value)
            | ConfigItem::E2eeEnabled(value)
            | ConfigItem::ImapCertificateChecks(value)
            | ConfigItem::ImapFolder(value)
            | ConfigItem::MailPort(value)
            | ConfigItem::MailPw(value)
            | ConfigItem::MailServer(value)
            | ConfigItem::MailUser(value)
            | ConfigItem::MdnsEnabled(value)
            | ConfigItem::MvboxMove(value)
            | ConfigItem::MvboxWatch(value)
            | ConfigItem::SaveMimeHeaders(value)
            | ConfigItem::Selfstatus(value)
            | ConfigItem::SendPort(value)
            | ConfigItem::SendPw(value)
            | ConfigItem::SendServer(value)
            | ConfigItem::SendUser(value)
            | ConfigItem::SentboxWatch(value)
            | ConfigItem::ServerFlags(value)
            | ConfigItem::ShowEmails(value)
            | ConfigItem::SmtpCertificateChecks(value)
            | ConfigItem::SysConfigKeys(value)
            | ConfigItem::SysMsgsizeMaxRecommended(value)
            | ConfigItem::SysVersion(value) => rusqlite::types::Value::Text(value.to_string()),
        };
        let out = rusqlite::types::ToSqlOutput::Owned(sql_obj);
        Ok(out)
    }
}

impl Context {
    /// Gets a configuration option.
    ///
    /// If there is no value set in the database returns the default
    /// value from [ConfigKey::default_item].
    pub fn get_config_item(&self, key: ConfigKey) -> Option<ConfigItem> {
        let res = self.sql.query_row(
            "SELECT value FROM config WHERE keyname=?;",
            params![key.to_string()],
            |row| {
                row.get_raw_checked(0)
                    .map(|valref| self.sql_raw_to_config_item(key, valref))
            },
        );
        match res {
            Ok(item) => item,
            Err(Error::Sql(rusqlite::Error::QueryReturnedNoRows)) => key.default_item(self),
            Err(err) => {
                warn!(self, "get_config: Failed SQL query: {}", err);
                None
            }
        }
    }

    // This is effectively the FromSql impl for ConfigItem, but we
    // need to know the ConfigKey which the trait does not give us.
    fn sql_raw_to_config_item(
        &self,
        key: ConfigKey,
        raw: rusqlite::types::ValueRef,
    ) -> Option<ConfigItem> {
        let to_string = |raw: rusqlite::types::ValueRef| -> Option<String> {
            raw.as_str()
                .map_err(|err| {
                    warn!(self, "ConfigItem {}; not a string: {}", key, err);
                })
                .map(|s| s.to_string())
                .ok()
        };
        let to_int = |raw: rusqlite::types::ValueRef| -> Option<i64> {
            match raw {
                // Current way this is stored.
                rusqlite::types::ValueRef::Integer(val) => Some(val),
                // Backward compatibility.
                rusqlite::types::ValueRef::Text(val) => std::str::from_utf8(val)
                    .map_err(|e| {
                        warn!(self, "ConfigItem {}; not UTF-8: {}", key, e);
                    })
                    .ok()
                    .and_then(|v| match v.parse::<i64>() {
                        Ok(i) => Some(i),
                        Err(e) => {
                            warn!(self, "ConfigItem {}; not parsed as int: {}", key, e);
                            None
                        }
                    }),
                _ => {
                    warn!(self, "ConfigItem {}; bad SQLite type: {:?}", key, raw);
                    None
                }
            }
        };
        let to_bool = |raw: rusqlite::types::ValueRef| -> Option<bool> {
            to_int(raw).and_then(|i| match i {
                0 => Some(false),
                1 => Some(true),
                v => {
                    warn!(self, "ConfigItem {}; bad bool value: {}", key, v);
                    None
                }
            })
        };
        match key {
            ConfigKey::Addr => to_string(raw).map(|val| ConfigItem::Addr(val)),
            ConfigKey::BccSelf => to_string(raw).map(|val| ConfigItem::BccSelf(val)),
            ConfigKey::Configured => to_string(raw).map(|val| ConfigItem::Configured(val)),
            ConfigKey::ConfiguredAddr => to_string(raw).map(|val| ConfigItem::ConfiguredAddr(val)),
            ConfigKey::ConfiguredE2EEEnabled => {
                to_string(raw).map(|val| ConfigItem::ConfiguredE2EEEnabled(val))
            }
            ConfigKey::ConfiguredImapCertificateChecks => {
                to_string(raw).map(|val| ConfigItem::ConfiguredImapCertificateChecks(val))
            }
            ConfigKey::ConfiguredMailPort => {
                to_string(raw).map(|val| ConfigItem::ConfiguredMailPort(val))
            }
            ConfigKey::ConfiguredMailPw => {
                to_string(raw).map(|val| ConfigItem::ConfiguredMailPw(val))
            }
            ConfigKey::ConfiguredMailSecurity => {
                to_string(raw).map(|val| ConfigItem::ConfiguredMailSecurity(val))
            }
            ConfigKey::ConfiguredMailServer => {
                to_string(raw).map(|val| ConfigItem::ConfiguredMailServer(val))
            }
            ConfigKey::ConfiguredMailUser => {
                to_string(raw).map(|val| ConfigItem::ConfiguredMailUser(val))
            }
            ConfigKey::ConfiguredSendPort => {
                to_string(raw).map(|val| ConfigItem::ConfiguredSendPort(val))
            }
            ConfigKey::ConfiguredSendPw => {
                to_string(raw).map(|val| ConfigItem::ConfiguredSendPw(val))
            }
            ConfigKey::ConfiguredSendSecurity => {
                to_string(raw).map(|val| ConfigItem::ConfiguredSendSecurity(val))
            }
            ConfigKey::ConfiguredSendServer => {
                to_string(raw).map(|val| ConfigItem::ConfiguredSendServer(val))
            }
            ConfigKey::ConfiguredSendUser => {
                to_string(raw).map(|val| ConfigItem::ConfiguredSendUser(val))
            }
            ConfigKey::ConfiguredServerFlags => {
                to_string(raw).map(|val| ConfigItem::ConfiguredServerFlags(val))
            }
            ConfigKey::ConfiguredSmtpCertificateChecks => {
                to_string(raw).map(|val| ConfigItem::ConfiguredSmtpCertificateChecks(val))
            }
            ConfigKey::Displayname => to_string(raw).map(|val| ConfigItem::Displayname(val)),
            ConfigKey::E2eeEnabled => to_string(raw).map(|val| ConfigItem::E2eeEnabled(val)),
            ConfigKey::ImapCertificateChecks => {
                to_string(raw).map(|val| ConfigItem::ImapCertificateChecks(val))
            }
            ConfigKey::ImapFolder => to_string(raw).map(|val| ConfigItem::ImapFolder(val)),
            ConfigKey::InboxWatch => to_bool(raw).map(|val| ConfigItem::InboxWatch(val)),
            ConfigKey::MailPort => to_string(raw).map(|val| ConfigItem::MailPort(val)),
            ConfigKey::MailPw => to_string(raw).map(|val| ConfigItem::MailPw(val)),
            ConfigKey::MailServer => to_string(raw).map(|val| ConfigItem::MailServer(val)),
            ConfigKey::MailUser => to_string(raw).map(|val| ConfigItem::MailUser(val)),
            ConfigKey::MdnsEnabled => to_string(raw).map(|val| ConfigItem::MdnsEnabled(val)),
            ConfigKey::MvboxMove => to_string(raw).map(|val| ConfigItem::MvboxMove(val)),
            ConfigKey::MvboxWatch => to_string(raw).map(|val| ConfigItem::MvboxWatch(val)),
            ConfigKey::SaveMimeHeaders => {
                to_string(raw).map(|val| ConfigItem::SaveMimeHeaders(val))
            }
            ConfigKey::Selfavatar => to_string(raw).map(|val| ConfigItem::Selfavatar(val)),
            ConfigKey::Selfstatus => to_string(raw).map(|val| ConfigItem::Selfstatus(val)),
            ConfigKey::SendPort => to_string(raw).map(|val| ConfigItem::SendPort(val)),
            ConfigKey::SendPw => to_string(raw).map(|val| ConfigItem::SendPw(val)),
            ConfigKey::SendServer => to_string(raw).map(|val| ConfigItem::SendServer(val)),
            ConfigKey::SendUser => to_string(raw).map(|val| ConfigItem::SendUser(val)),
            ConfigKey::SentboxWatch => to_string(raw).map(|val| ConfigItem::SentboxWatch(val)),
            ConfigKey::ServerFlags => to_string(raw).map(|val| ConfigItem::ServerFlags(val)),
            ConfigKey::ShowEmails => to_string(raw).map(|val| ConfigItem::ShowEmails(val)),
            ConfigKey::SmtpCertificateChecks => {
                to_string(raw).map(|val| ConfigItem::SmtpCertificateChecks(val))
            }
            ConfigKey::SysConfigKeys => to_string(raw).map(|val| ConfigItem::SysConfigKeys(val)),
            ConfigKey::SysMsgsizeMaxRecommended => {
                to_string(raw).map(|val| ConfigItem::SysMsgsizeMaxRecommended(val))
            }
            ConfigKey::SysVersion => to_string(raw).map(|val| ConfigItem::SysVersion(val)),
        }
    }

    /// Stores a configuration item in the database.
    ///
    /// # Errors
    ///
    /// You can not store any of the [ConfigItem::SysVersion],
    /// [ConfigItem::SysMsgsizemaxrecommended] or
    /// [ConfigItem::SysConfigkeys] variants.
    pub fn set_config_item(&self, item: ConfigItem) -> Result<ConfigItem, Error> {
        match item {
            ConfigItem::SysConfigKeys(_)
            | ConfigItem::SysMsgsizeMaxRecommended(_)
            | ConfigItem::SysVersion(_) => bail!("Can not set config item {}", item),
            _ => (),
        }
        // Would prefer to use INSERT OR REPLACE but this needs a
        // uniqueness constraint on the keyname column which does not
        // yet exist.
        if self.sql.exists(
            "SELECT value FROM config WHERE keyname=?;",
            params![item.to_string()],
        )? {
            self.sql.execute(
                "UPDATE config SET value=? WHERE keyname=?",
                params![item, item.to_string()],
            )?;
        } else {
            self.sql.execute(
                "INSERT INTO config (keyname, value) VALUES (?, ?)",
                params![item.to_string(), item],
            )?;
        }
        match item {
            ConfigItem::InboxWatch(_) => interrupt_inbox_idle(self, true),
            ConfigItem::SentboxWatch(_) => interrupt_sentbox_idle(self),
            ConfigItem::MvboxWatch(_) => interrupt_mvbox_idle(self),
            _ => (),
        };
        Ok(item)
    }

    /// Deletes a configuration option.
    ///
    /// Returns `true` if the option was deleted, `false` if it wasn't
    /// present in the first place.
    pub fn del_config_item(&self, key: ConfigKey) -> Result<bool, Error> {
        match self.sql.execute(
            "DELETE FROM config WHERE keyname=?;",
            params![key.to_string()],
        ) {
            Ok(0) => Ok(false),
            Ok(_) => Ok(true),
            Err(err) => Err(err),
        }
    }

    /// Transitional: migrate to get_config_item.
    ///
    /// This will migrate to the FFI once nothing in the core uses
    /// this anymore.
    pub fn get_config(&self, key: Config) -> Option<String> {
        if let Some(item) = self.get_config_item(key) {
            let value = match item {
                // Bool values.
                ConfigItem::InboxWatch(value) => format!("{}", value as u32),
                // String values.
                ConfigItem::Addr(value)
                | ConfigItem::BccSelf(value)
                | ConfigItem::Configured(value)
                | ConfigItem::ConfiguredAddr(value)
                | ConfigItem::ConfiguredE2EEEnabled(value)
                | ConfigItem::ConfiguredImapCertificateChecks(value)
                | ConfigItem::ConfiguredMailPort(value)
                | ConfigItem::ConfiguredMailPw(value)
                | ConfigItem::ConfiguredMailSecurity(value)
                | ConfigItem::ConfiguredMailServer(value)
                | ConfigItem::ConfiguredMailUser(value)
                | ConfigItem::ConfiguredSendPort(value)
                | ConfigItem::ConfiguredSendPw(value)
                | ConfigItem::ConfiguredSendSecurity(value)
                | ConfigItem::ConfiguredSendServer(value)
                | ConfigItem::ConfiguredSendUser(value)
                | ConfigItem::ConfiguredServerFlags(value)
                | ConfigItem::ConfiguredSmtpCertificateChecks(value)
                | ConfigItem::Displayname(value)
                | ConfigItem::E2eeEnabled(value)
                | ConfigItem::ImapCertificateChecks(value)
                | ConfigItem::ImapFolder(value)
                | ConfigItem::MailPort(value)
                | ConfigItem::MailPw(value)
                | ConfigItem::MailServer(value)
                | ConfigItem::MailUser(value)
                | ConfigItem::MdnsEnabled(value)
                | ConfigItem::MvboxMove(value)
                | ConfigItem::MvboxWatch(value)
                | ConfigItem::SaveMimeHeaders(value)
                | ConfigItem::Selfavatar(value)
                | ConfigItem::Selfstatus(value)
                | ConfigItem::SendPort(value)
                | ConfigItem::SendPw(value)
                | ConfigItem::SendServer(value)
                | ConfigItem::SendUser(value)
                | ConfigItem::SentboxWatch(value)
                | ConfigItem::ServerFlags(value)
                | ConfigItem::ShowEmails(value)
                | ConfigItem::SmtpCertificateChecks(value)
                | ConfigItem::SysConfigKeys(value)
                | ConfigItem::SysMsgsizeMaxRecommended(value)
                | ConfigItem::SysVersion(value) => value,
            };
            Some(value)
        } else {
            None
        }
    }

    /// Transitional: migrate to get_config_item.
    pub fn get_config_int(&self, key: Config) -> i32 {
        self.get_config(key)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default()
    }

    /// Transitional: migrate to get_config_item.
    pub fn get_config_bool(&self, key: Config) -> bool {
        self.get_config_int(key) != 0
    }

    /// Set the given config key.
    ///
    /// Transitional: migrate to set_config_item/del_config_item.
    ///
    /// If `None` is passed as a value the value is cleared and set to
    /// the default if there is one.
    pub fn set_config(&self, key: Config, value: Option<&str>) -> Result<(), Error> {
        let maybe_val = match key {
            Config::Selfstatus => {
                let def = self.stock_str(StockMessage::StatusLine);
                if value.is_none() || value.unwrap() == def {
                    None
                } else {
                    value
                }
            }
            _ => value,
        };
        if let Some(val) = maybe_val {
            let v = val.to_string();
            let item = match key {
                ConfigKey::Addr => ConfigItem::Addr(v),
                ConfigKey::BccSelf => ConfigItem::BccSelf(v),
                ConfigKey::Configured => ConfigItem::Configured(v),
                ConfigKey::ConfiguredAddr => ConfigItem::ConfiguredAddr(v),
                ConfigKey::ConfiguredE2EEEnabled => ConfigItem::ConfiguredE2EEEnabled(v),
                ConfigKey::ConfiguredImapCertificateChecks => {
                    ConfigItem::ConfiguredImapCertificateChecks(v)
                }
                ConfigKey::ConfiguredMailPort => ConfigItem::ConfiguredMailPort(v),
                ConfigKey::ConfiguredMailPw => ConfigItem::ConfiguredMailPw(v),
                ConfigKey::ConfiguredMailSecurity => ConfigItem::ConfiguredMailSecurity(v),
                ConfigKey::ConfiguredMailServer => ConfigItem::ConfiguredMailServer(v),
                ConfigKey::ConfiguredMailUser => ConfigItem::ConfiguredMailUser(v),
                ConfigKey::ConfiguredSendPort => ConfigItem::ConfiguredSendPort(v),
                ConfigKey::ConfiguredSendPw => ConfigItem::ConfiguredSendPw(v),
                ConfigKey::ConfiguredSendSecurity => ConfigItem::ConfiguredSendSecurity(v),
                ConfigKey::ConfiguredSendServer => ConfigItem::ConfiguredSendServer(v),
                ConfigKey::ConfiguredSendUser => ConfigItem::ConfiguredSendUser(v),
                ConfigKey::ConfiguredServerFlags => ConfigItem::ConfiguredServerFlags(v),
                ConfigKey::ConfiguredSmtpCertificateChecks => {
                    ConfigItem::ConfiguredSmtpCertificateChecks(v)
                }
                ConfigKey::Displayname => ConfigItem::Displayname(v),
                ConfigKey::E2eeEnabled => ConfigItem::E2eeEnabled(v),
                ConfigKey::ImapCertificateChecks => ConfigItem::ImapCertificateChecks(v),
                ConfigKey::ImapFolder => ConfigItem::ImapFolder(v),
                ConfigKey::InboxWatch => {
                    let val = match v.parse::<u32>() {
                        Ok(0) => false,
                        Ok(1) => true,
                        _ => bail!("set_config for {}: not a bool: {}", key, v),
                    };
                    ConfigItem::InboxWatch(val)
                }
                ConfigKey::MailPort => ConfigItem::MailPort(v),
                ConfigKey::MailPw => ConfigItem::MailPw(v),
                ConfigKey::MailServer => ConfigItem::MailServer(v),
                ConfigKey::MailUser => ConfigItem::MailUser(v),
                ConfigKey::MdnsEnabled => ConfigItem::MdnsEnabled(v),
                ConfigKey::MvboxMove => ConfigItem::MvboxMove(v),
                ConfigKey::MvboxWatch => ConfigItem::MvboxWatch(v),
                ConfigKey::SaveMimeHeaders => ConfigItem::SaveMimeHeaders(v),
                ConfigKey::Selfavatar => ConfigItem::Selfavatar(v),
                ConfigKey::Selfstatus => ConfigItem::Selfstatus(v),
                ConfigKey::SendPort => ConfigItem::SendPort(v),
                ConfigKey::SendPw => ConfigItem::SendPw(v),
                ConfigKey::SendServer => ConfigItem::SendServer(v),
                ConfigKey::SendUser => ConfigItem::SendUser(v),
                ConfigKey::SentboxWatch => ConfigItem::SentboxWatch(v),
                ConfigKey::ServerFlags => ConfigItem::ServerFlags(v),
                ConfigKey::ShowEmails => ConfigItem::ShowEmails(v),
                ConfigKey::SmtpCertificateChecks => ConfigItem::SmtpCertificateChecks(v),
                ConfigKey::SysConfigKeys => ConfigItem::SysConfigKeys(v),
                ConfigKey::SysMsgsizeMaxRecommended => ConfigItem::SysMsgsizeMaxRecommended(v),
                ConfigKey::SysVersion => ConfigItem::SysVersion(v),
            };
            self.set_config_item(item).map(|_| ())
        } else {
            self.del_config_item(key).map(|_| ())
        }
    }
}

/// Returns all available configuration keys concated together.
fn get_config_keys_string() -> String {
    let keys = ConfigItem::iter().fold(String::new(), |mut acc, key| {
        acc += key.as_ref();
        acc += " ";
        acc
    });
    format!(" {} ", keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;
    use std::string::ToString;

    use crate::test_utils::*;

    impl ConfigKey {
        fn to_key_string(&self) -> String {
            let s = self.to_string();
            if s.starts_with("sys_") {
                s.replacen("sys_", "sys.", 1)
            } else {
                s
            }
        }
    }

    #[test]
    fn test_to_string() {
        assert_eq!(
            ConfigItem::MailServer(Default::default()).to_string(),
            "mail_server"
        );
        assert_eq!(
            ConfigItem::from_str("mail_server"),
            Ok(ConfigItem::MailServer(Default::default()))
        );

        assert_eq!(
            ConfigItem::SysConfigKeys(Default::default()).to_string(),
            "sys.config_keys"
        );
        assert_eq!(
            ConfigItem::from_str("sys.config_keys"),
            Ok(ConfigItem::SysConfigKeys(Default::default()))
        );

        // The ConfigItem.to_string() is used as key in the SQL table
        // on set operations and ConfigKey.to_string() is the key used
        // on get operations.  Therefore we want to make sure they are
        // the same keys.
        for item in ConfigItem::iter() {
            let name = item.to_string();
            let key = ConfigKey::from_key_str(&name).unwrap();
            assert_eq!(name, key.to_key_string());
        }
    }

    #[test]
    fn test_config_item() {
        let t = test_context(Some(Box::new(logging_cb)));

        // An item which has a default.
        let opt = t.ctx.get_config_item(ConfigKey::ImapFolder);
        assert_eq!(opt, Some(ConfigItem::ImapFolder("INBOX".into())));

        // Set a different value.
        t.ctx
            .set_config_item(ConfigItem::ImapFolder("DeltaChat".into()))
            .unwrap();
        let opt = t.ctx.get_config_item(ConfigKey::ImapFolder);
        assert_eq!(opt, Some(ConfigItem::ImapFolder("DeltaChat".into())));

        // Set another value, testing update.
        t.ctx
            .set_config_item(ConfigItem::ImapFolder("Chat".into()))
            .unwrap();
        let opt = t.ctx.get_config_item(ConfigKey::ImapFolder);
        assert_eq!(opt, Some(ConfigItem::ImapFolder("Chat".into())));

        // Reset to the default.
        t.ctx.del_config_item(ConfigKey::ImapFolder).unwrap();
        let opt = t.ctx.get_config_item(ConfigKey::ImapFolder);
        assert_eq!(opt, Some(ConfigItem::ImapFolder("INBOX".into())));

        // An item without default.
        let opt = t.ctx.get_config_item(ConfigKey::Addr);
        assert!(opt.is_none());

        // Set the item.
        t.ctx
            .set_config_item(ConfigItem::Addr("me@example.com".into()))
            .unwrap();
        let opt = t.ctx.get_config_item(ConfigKey::Addr);
        assert_eq!(opt, Some(ConfigItem::Addr("me@example.com".into())));

        // Delete the item.
        t.ctx.del_config_item(ConfigKey::Addr).unwrap();
        let opt = t.ctx.get_config_item(ConfigKey::Addr);
        assert!(opt.is_none());
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
    fn test_config_item_bool() {
        let t = test_context(Some(Box::new(logging_cb)));

        // Backwards compatible value.
        t.ctx
            .sql
            .execute(
                "INSERT INTO config (keyname, value) VALUES (?, ?)",
                params![ConfigKey::InboxWatch.to_string(), "0"],
            )
            .unwrap();
        let item = t.ctx.get_config_item(ConfigKey::InboxWatch).unwrap();
        assert_eq!(item, ConfigItem::InboxWatch(false));
        t.ctx
            .sql
            .execute(
                "UPDATE config SET value=? WHERE keyname=?",
                params!["1", ConfigKey::InboxWatch.to_string()],
            )
            .unwrap();
        let item = t.ctx.get_config_item(ConfigKey::InboxWatch).unwrap();
        assert_eq!(item, ConfigItem::InboxWatch(true));
        t.ctx
            .sql
            .execute(
                "UPDATE config SET value=? WHERE keyname=?",
                params!["bad", ConfigKey::InboxWatch.to_string()],
            )
            .unwrap();
        let item = t.ctx.get_config_item(ConfigKey::InboxWatch);
        assert!(item.is_none());

        // Normal value.
        t.ctx.set_config_item(ConfigItem::InboxWatch(true)).unwrap();
        let item = t.ctx.get_config_item(ConfigKey::InboxWatch).unwrap();
        assert_eq!(item, ConfigItem::InboxWatch(true));
        t.ctx
            .set_config_item(ConfigItem::InboxWatch(false))
            .unwrap();
        let item = t.ctx.get_config_item(ConfigKey::InboxWatch).unwrap();
        assert_eq!(item, ConfigItem::InboxWatch(false));
    }
}

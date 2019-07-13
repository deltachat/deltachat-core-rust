use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumIter, EnumProperty, EnumString};

use crate::constants::DC_VERSION_STR;
use crate::context::Context;
use crate::dc_job::*;
use crate::dc_tools::*;
use crate::error::Error;
use crate::stock::StockMessage;
use crate::x::*;

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
    SendServer,
    SendUser,
    SendPw,
    SendPort,
    ServerFlags,
    #[strum(props(default = "INBOX"))]
    ImapFolder,
    Displayname,
    Selfstatus,
    Selfavatar,
    #[strum(props(default = "1"))]
    E2eeEnabled,
    #[strum(props(default = "1"))]
    MdnsEnabled,
    InboxWatch,
    #[strum(props(default = "1"))]
    SentboxWatch,
    #[strum(props(default = "1"))]
    MvboxWatch,
    #[strum(props(default = "1"))]
    MvboxMove,
    #[strum(props(default = "0"))]
    ShowEmails,
    SaveMimeHeaders,
    ConfiguredAddr,
    ConfiguredMailServer,
    ConfiguredMailUser,
    ConfiguredMailPw,
    ConfiguredMailPort,
    ConfiguredSendServer,
    ConfiguredSendUser,
    ConfiguredSendPw,
    ConfiguredSendPort,
    ConfiguredServerFlags,
    Configured,
    // Deprecated
    #[strum(serialize = "sys.version")]
    SysVersion,
    #[strum(serialize = "sys.msgsize_max_recommended")]
    SysMsgsizeMaxRecommended,
    #[strum(serialize = "sys.config_keys")]
    SysConfigKeys,
}

impl Context {
    /// Get a configuration key. Returns `None` if no value is set, and no default value found.
    pub fn get_config(&self, key: Config) -> Option<String> {
        let value = match key {
            Config::Selfavatar => {
                let rel_path = self.sql.get_config(self, key);
                rel_path.map(|p| {
                    let v = unsafe {
                        let n = to_cstring(p);
                        let res = dc_get_abs_path(self, n);
                        free(n as *mut libc::c_void);
                        res
                    };
                    let r = to_string(v);
                    unsafe { free(v as *mut _) };
                    r
                })
            }
            Config::SysVersion => Some(std::str::from_utf8(DC_VERSION_STR).unwrap().into()),
            Config::SysMsgsizeMaxRecommended => Some(format!("{}", 24 * 1024 * 1024 / 4 * 3)),
            Config::SysConfigKeys => Some(get_config_keys_string()),
            _ => self.sql.get_config(self, key),
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

    /// Set the given config key.
    /// If `None` is passed as a value the value is cleared and set to the default if there is one.
    pub fn set_config(&self, key: Config, value: Option<&str>) -> Result<(), Error> {
        match key {
            Config::Selfavatar if value.is_some() => {
                let rel_path = std::fs::canonicalize(value.unwrap())?;
                self.sql
                    .set_config(self, key, Some(&rel_path.to_string_lossy()))
            }
            Config::InboxWatch => {
                let ret = self.sql.set_config(self, key, value);
                unsafe { dc_interrupt_imap_idle(self) };
                ret
            }
            Config::SentboxWatch => {
                let ret = self.sql.set_config(self, key, value);
                unsafe { dc_interrupt_sentbox_idle(self) };
                ret
            }
            Config::MvboxWatch => {
                let ret = self.sql.set_config(self, key, value);
                unsafe { dc_interrupt_mvbox_idle(self) };
                ret
            }
            Config::Selfstatus => {
                let def = self.stock_str(StockMessage::StatusLine);
                let val = if value.is_none() || value.unwrap() == def {
                    None
                } else {
                    value
                };

                let ret = self.sql.set_config(self, key, val);
                ret
            }
            _ => self.sql.set_config(self, key, value),
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

    use std::str::FromStr;
    use std::string::ToString;

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
}

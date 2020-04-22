//! # Key-value configuration management

use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumIter, EnumProperty, EnumString};

use crate::blob::BlobObject;
use crate::chat::ChatId;
use crate::constants::DC_VERSION_STR;
use crate::context::Context;
use crate::dc_tools::*;
use crate::events::Event;
use crate::job::*;
use crate::message::MsgId;
use crate::mimefactory::RECOMMENDED_FILE_SIZE;
use crate::stock::StockMessage;
use rusqlite::NO_PARAMS;

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

    #[strum(props(default = "0"))]
    KeyGenType,

    /// Timer in seconds after which the message is deleted from the
    /// server.
    ///
    /// Equals to 0 by default, which means the message is never
    /// deleted.
    ///
    /// Value 1 is treated as "delete at once": messages are deleted
    /// immediately, without moving to DeltaChat folder.
    #[strum(props(default = "0"))]
    DeleteServerAfter,

    /// Timer in seconds after which the message is deleted from the
    /// device.
    ///
    /// Equals to 0 by default, which means the message is never
    /// deleted.
    #[strum(props(default = "0"))]
    DeleteDeviceAfter,

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

impl Context {
    /// Get a configuration key. Returns `None` if no value is set, and no default value found.
    pub fn get_config(&self, key: Config) -> Option<String> {
        let value = match key {
            Config::Selfavatar => {
                let rel_path = self.sql.get_raw_config(self, key);
                rel_path.map(|p| dc_get_abs_path(self, &p).to_string_lossy().into_owned())
            }
            Config::SysVersion => Some((&*DC_VERSION_STR).clone()),
            Config::SysMsgsizeMaxRecommended => Some(format!("{}", RECOMMENDED_FILE_SIZE)),
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

    /// Gets configured "delete_server_after" value.
    ///
    /// `None` means never delete the message, `Some(0)` means delete
    /// at once, `Some(x)` means delete after `x` seconds.
    pub fn get_config_delete_server_after(&self) -> Option<i64> {
        match self.get_config_int(Config::DeleteServerAfter) {
            0 => None,
            1 => Some(0),
            x => Some(x as i64),
        }
    }

    /// Gets configured "delete_device_after" value.
    ///
    /// `None` means never delete the message, `Some(x)` means delete
    /// after `x` seconds.
    pub fn get_config_delete_device_after(&self) -> Option<i64> {
        match self.get_config_int(Config::DeleteDeviceAfter) {
            0 => None,
            x => Some(x as i64),
        }
    }

    /// Set the given config key.
    /// If `None` is passed as a value the value is cleared and set to the default if there is one.
    pub fn set_config(&self, key: Config, value: Option<&str>) -> crate::sql::Result<()> {
        match key {
            Config::Selfavatar => {
                self.sql
                    .execute("UPDATE contacts SET selfavatar_sent=0;", NO_PARAMS)?;
                self.sql
                    .set_raw_config_bool(self, "attach_selfavatar", true)?;
                match value {
                    Some(value) => {
                        let blob = BlobObject::new_from_path(&self, value)?;
                        blob.recode_to_avatar_size(self)?;
                        self.sql.set_raw_config(self, key, Some(blob.as_name()))
                    }
                    None => self.sql.set_raw_config(self, key, None),
                }
            }
            Config::InboxWatch => {
                let ret = self.sql.set_raw_config(self, key, value);
                interrupt_inbox_idle(self);
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
            Config::DeleteDeviceAfter => {
                let ret = self.sql.set_raw_config(self, key, value);
                // Force chatlist reload to delete old messages immediately.
                self.call_cb(Event::MsgsChanged {
                    msg_id: MsgId::new(0),
                    chat_id: ChatId::new(0),
                });
                ret
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

    use std::str::FromStr;
    use std::string::ToString;

    use crate::constants::AVATAR_SIZE;
    use crate::test_utils::*;
    use image::GenericImageView;
    use std::fs::File;
    use std::io::Write;

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
    fn test_selfavatar_outside_blobdir() {
        let t = dummy_context();
        let avatar_src = t.dir.path().join("avatar.jpg");
        let avatar_bytes = include_bytes!("../test-data/image/avatar1000x1000.jpg");
        File::create(&avatar_src)
            .unwrap()
            .write_all(avatar_bytes)
            .unwrap();
        let avatar_blob = t.ctx.get_blobdir().join("avatar.jpg");
        assert!(!avatar_blob.exists());
        t.ctx
            .set_config(Config::Selfavatar, Some(&avatar_src.to_str().unwrap()))
            .unwrap();
        assert!(avatar_blob.exists());
        assert!(std::fs::metadata(&avatar_blob).unwrap().len() < avatar_bytes.len() as u64);
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar);
        assert_eq!(avatar_cfg, avatar_blob.to_str().map(|s| s.to_string()));

        let img = image::open(avatar_src).unwrap();
        assert_eq!(img.width(), 1000);
        assert_eq!(img.height(), 1000);

        let img = image::open(avatar_blob).unwrap();
        assert_eq!(img.width(), AVATAR_SIZE);
        assert_eq!(img.height(), AVATAR_SIZE);
    }

    #[test]
    fn test_selfavatar_in_blobdir() {
        let t = dummy_context();
        let avatar_src = t.ctx.get_blobdir().join("avatar.png");
        let avatar_bytes = include_bytes!("../test-data/image/avatar900x900.png");
        File::create(&avatar_src)
            .unwrap()
            .write_all(avatar_bytes)
            .unwrap();

        let img = image::open(&avatar_src).unwrap();
        assert_eq!(img.width(), 900);
        assert_eq!(img.height(), 900);

        t.ctx
            .set_config(Config::Selfavatar, Some(&avatar_src.to_str().unwrap()))
            .unwrap();
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar);
        assert_eq!(avatar_cfg, avatar_src.to_str().map(|s| s.to_string()));

        let img = image::open(avatar_src).unwrap();
        assert_eq!(img.width(), AVATAR_SIZE);
        assert_eq!(img.height(), AVATAR_SIZE);
    }

    #[test]
    fn test_selfavatar_copy_without_recode() {
        let t = dummy_context();
        let avatar_src = t.dir.path().join("avatar.png");
        let avatar_bytes = include_bytes!("../test-data/image/avatar64x64.png");
        File::create(&avatar_src)
            .unwrap()
            .write_all(avatar_bytes)
            .unwrap();
        let avatar_blob = t.ctx.get_blobdir().join("avatar.png");
        assert!(!avatar_blob.exists());
        t.ctx
            .set_config(Config::Selfavatar, Some(&avatar_src.to_str().unwrap()))
            .unwrap();
        assert!(avatar_blob.exists());
        assert_eq!(
            std::fs::metadata(&avatar_blob).unwrap().len(),
            avatar_bytes.len() as u64
        );
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar);
        assert_eq!(avatar_cfg, avatar_blob.to_str().map(|s| s.to_string()));
    }
}

//! # Key-value configuration management

use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumIter, EnumProperty, EnumString};

use crate::blob::BlobObject;
use crate::chat::ChatId;
use crate::constants::DC_VERSION_STR;
use crate::context::Context;
use crate::dc_tools::*;
use crate::events::EventType;
use crate::job;
use crate::message::MsgId;
use crate::mimefactory::RECOMMENDED_FILE_SIZE;
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
    MailSecurity,
    ImapCertificateChecks,
    SendServer,
    SendUser,
    SendPw,
    SendPort,
    SendSecurity,
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

    #[strum(props(default = "0"))] // also change MediaQuality.default() on changes
    MediaQuality,

    #[strum(props(default = "50"))]
    MaxSmtpRcptTo,

    /// If set to "1", on the first time `start_io()` is called after configuring,
    /// the newest existing messages are fetched.
    /// Existing recipients are added to the contact database regardless of this setting.
    #[strum(props(default = "0"))]
    // disabled for now, we'll set this back to "1" at some point
    FetchExistingMsgs,

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
    ConfiguredInboxFolder,
    ConfiguredMvboxFolder,
    ConfiguredSentboxFolder,
    Configured,

    #[strum(serialize = "sys.version")]
    SysVersion,

    #[strum(serialize = "sys.msgsize_max_recommended")]
    SysMsgsizeMaxRecommended,

    #[strum(serialize = "sys.config_keys")]
    SysConfigKeys,

    Bot,

    /// Whether we send a warning if the password is wrong (set to false when we send a warning
    /// because we do not want to send a second warning)
    #[strum(props(default = "0"))]
    NotifyAboutWrongPw,

    /// address to webrtc instance to use for videochats
    WebrtcInstance,
}

impl Context {
    pub async fn config_exists(&self, key: Config) -> bool {
        self.sql.get_raw_config(self, key).await.is_some()
    }

    /// Get a configuration key. Returns `None` if no value is set, and no default value found.
    pub async fn get_config(&self, key: Config) -> Option<String> {
        let value = match key {
            Config::Selfavatar => {
                let rel_path = self.sql.get_raw_config(self, key).await;
                rel_path.map(|p| dc_get_abs_path(self, &p).to_string_lossy().into_owned())
            }
            Config::SysVersion => Some((&*DC_VERSION_STR).clone()),
            Config::SysMsgsizeMaxRecommended => Some(format!("{}", RECOMMENDED_FILE_SIZE)),
            Config::SysConfigKeys => Some(get_config_keys_string()),
            _ => self.sql.get_raw_config(self, key).await,
        };

        if value.is_some() {
            return value;
        }

        // Default values
        match key {
            Config::Selfstatus => Some(self.stock_str(StockMessage::StatusLine).await.into_owned()),
            Config::ConfiguredInboxFolder => Some("INBOX".to_owned()),
            _ => key.get_str("default").map(|s| s.to_string()),
        }
    }

    pub async fn get_config_int(&self, key: Config) -> i32 {
        self.get_config(key)
            .await
            .and_then(|s| s.parse().ok())
            .unwrap_or_default()
    }

    pub async fn get_config_bool(&self, key: Config) -> bool {
        self.get_config_int(key).await != 0
    }

    /// Gets configured "delete_server_after" value.
    ///
    /// `None` means never delete the message, `Some(0)` means delete
    /// at once, `Some(x)` means delete after `x` seconds.
    pub async fn get_config_delete_server_after(&self) -> Option<i64> {
        match self.get_config_int(Config::DeleteServerAfter).await {
            0 => None,
            1 => Some(0),
            x => Some(x as i64),
        }
    }

    /// Gets configured "delete_device_after" value.
    ///
    /// `None` means never delete the message, `Some(x)` means delete
    /// after `x` seconds.
    pub async fn get_config_delete_device_after(&self) -> Option<i64> {
        match self.get_config_int(Config::DeleteDeviceAfter).await {
            0 => None,
            x => Some(x as i64),
        }
    }

    /// Set the given config key.
    /// If `None` is passed as a value the value is cleared and set to the default if there is one.
    pub async fn set_config(&self, key: Config, value: Option<&str>) -> crate::sql::Result<()> {
        match key {
            Config::Selfavatar => {
                self.sql
                    .execute("UPDATE contacts SET selfavatar_sent=0;", paramsv![])
                    .await?;
                self.sql
                    .set_raw_config_bool(self, "attach_selfavatar", true)
                    .await?;
                match value {
                    Some(value) => {
                        let blob = BlobObject::new_from_path(&self, value).await?;
                        blob.recode_to_avatar_size(self)?;
                        self.sql
                            .set_raw_config(self, key, Some(blob.as_name()))
                            .await
                    }
                    None => self.sql.set_raw_config(self, key, None).await,
                }
            }
            Config::Selfstatus => {
                let def = self.stock_str(StockMessage::StatusLine).await;
                let val = if value.is_none() || value.unwrap() == def {
                    None
                } else {
                    value
                };

                self.sql.set_raw_config(self, key, val).await
            }
            Config::DeleteDeviceAfter => {
                let ret = self.sql.set_raw_config(self, key, value).await;
                // Force chatlist reload to delete old messages immediately.
                self.emit_event(EventType::MsgsChanged {
                    msg_id: MsgId::new(0),
                    chat_id: ChatId::new(0),
                });
                ret
            }
            Config::Displayname => {
                let value = value.map(improve_single_line_input);
                self.sql.set_raw_config(self, key, value.as_deref()).await
            }
            Config::DeleteServerAfter => {
                let ret = self.sql.set_raw_config(self, key, value).await;
                job::schedule_resync(self).await;
                ret
            }
            _ => self.sql.set_raw_config(self, key, value).await,
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

    use crate::constants;
    use crate::constants::AVATAR_SIZE;
    use crate::test_utils::*;
    use image::GenericImageView;
    use num_traits::FromPrimitive;
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

    #[async_std::test]
    async fn test_selfavatar_outside_blobdir() {
        let t = TestContext::new().await;
        let avatar_src = t.dir.path().join("avatar.jpg");
        let avatar_bytes = include_bytes!("../test-data/image/avatar1000x1000.jpg");
        File::create(&avatar_src)
            .unwrap()
            .write_all(avatar_bytes)
            .unwrap();
        let avatar_blob = t.ctx.get_blobdir().join("avatar.jpg");
        assert!(!avatar_blob.exists().await);
        t.ctx
            .set_config(Config::Selfavatar, Some(&avatar_src.to_str().unwrap()))
            .await
            .unwrap();
        assert!(avatar_blob.exists().await);
        assert!(std::fs::metadata(&avatar_blob).unwrap().len() < avatar_bytes.len() as u64);
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar).await;
        assert_eq!(avatar_cfg, avatar_blob.to_str().map(|s| s.to_string()));

        let img = image::open(avatar_src).unwrap();
        assert_eq!(img.width(), 1000);
        assert_eq!(img.height(), 1000);

        let img = image::open(avatar_blob).unwrap();
        assert_eq!(img.width(), AVATAR_SIZE);
        assert_eq!(img.height(), AVATAR_SIZE);
    }

    #[async_std::test]
    async fn test_selfavatar_in_blobdir() {
        let t = TestContext::new().await;
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
            .await
            .unwrap();
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar).await;
        assert_eq!(avatar_cfg, avatar_src.to_str().map(|s| s.to_string()));

        let img = image::open(avatar_src).unwrap();
        assert_eq!(img.width(), AVATAR_SIZE);
        assert_eq!(img.height(), AVATAR_SIZE);
    }

    #[async_std::test]
    async fn test_selfavatar_copy_without_recode() {
        let t = TestContext::new().await;
        let avatar_src = t.dir.path().join("avatar.png");
        let avatar_bytes = include_bytes!("../test-data/image/avatar64x64.png");
        File::create(&avatar_src)
            .unwrap()
            .write_all(avatar_bytes)
            .unwrap();
        let avatar_blob = t.ctx.get_blobdir().join("avatar.png");
        assert!(!avatar_blob.exists().await);
        t.ctx
            .set_config(Config::Selfavatar, Some(&avatar_src.to_str().unwrap()))
            .await
            .unwrap();
        assert!(avatar_blob.exists().await);
        assert_eq!(
            std::fs::metadata(&avatar_blob).unwrap().len(),
            avatar_bytes.len() as u64
        );
        let avatar_cfg = t.ctx.get_config(Config::Selfavatar).await;
        assert_eq!(avatar_cfg, avatar_blob.to_str().map(|s| s.to_string()));
    }

    #[async_std::test]
    async fn test_media_quality_config_option() {
        let t = TestContext::new().await;
        let media_quality = t.ctx.get_config_int(Config::MediaQuality).await;
        assert_eq!(media_quality, 0);
        let media_quality = constants::MediaQuality::from_i32(media_quality).unwrap_or_default();
        assert_eq!(media_quality, constants::MediaQuality::Balanced);

        t.ctx
            .set_config(Config::MediaQuality, Some("1"))
            .await
            .unwrap();

        let media_quality = t.ctx.get_config_int(Config::MediaQuality).await;
        assert_eq!(media_quality, 1);
        assert_eq!(constants::MediaQuality::Worse as i32, 1);
        let media_quality = constants::MediaQuality::from_i32(media_quality).unwrap_or_default();
        assert_eq!(media_quality, constants::MediaQuality::Worse);
    }
}

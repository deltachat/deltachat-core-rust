//! # Key-value configuration management.

use std::env;
use std::path::Path;
use std::str::FromStr;

use anyhow::{ensure, Context as _, Result};
use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumIter, EnumProperty, EnumString};

use crate::blob::BlobObject;
use crate::constants::DC_VERSION_STR;
use crate::contact::addr_cmp;
use crate::context::Context;
use crate::events::EventType;
use crate::mimefactory::RECOMMENDED_FILE_SIZE;
use crate::provider::{get_provider_by_id, Provider};
use crate::tools::{get_abs_path, improve_single_line_input, EmailAddress};

/// The available configuration keys.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Display,
    EnumString,
    AsRefStr,
    EnumIter,
    EnumProperty,
    PartialOrd,
    Ord,
)]
#[strum(serialize_all = "snake_case")]
pub enum Config {
    /// Email address, used in the `From:` field.
    Addr,

    /// IMAP server hostname.
    MailServer,

    /// IMAP server username.
    MailUser,

    /// IMAP server password.
    MailPw,

    /// IMAP server port.
    MailPort,

    /// IMAP server security (e.g. TLS, STARTTLS).
    MailSecurity,

    /// How to check IMAP server TLS certificates.
    ImapCertificateChecks,

    /// SMTP server hostname.
    SendServer,

    /// SMTP server username.
    SendUser,

    /// SMTP server password.
    SendPw,

    /// SMTP server port.
    SendPort,

    /// SMTP server security (e.g. TLS, STARTTLS).
    SendSecurity,

    /// How to check SMTP server TLS certificates.
    SmtpCertificateChecks,

    /// Whether to use OAuth 2.
    ///
    /// Historically contained other bitflags, which are now deprecated.
    /// Should not be extended in the future, create new config keys instead.
    ServerFlags,

    /// True if SOCKS5 is enabled.
    ///
    /// Can be used to disable SOCKS5 without erasing SOCKS5 configuration.
    Socks5Enabled,

    /// SOCKS5 proxy server hostname or address.
    Socks5Host,

    /// SOCKS5 proxy server port.
    Socks5Port,

    /// SOCKS5 proxy server username.
    Socks5User,

    /// SOCKS5 proxy server password.
    Socks5Password,

    /// Own name to use in the `From:` field when sending messages.
    Displayname,

    /// Own status to display, sent in message footer.
    Selfstatus,

    /// Own avatar filename.
    Selfavatar,

    /// Send BCC copy to self.
    ///
    /// Should be enabled for multidevice setups.
    #[strum(props(default = "1"))]
    BccSelf,

    /// True if encryption is preferred according to Autocrypt standard.
    #[strum(props(default = "1"))]
    E2eeEnabled,

    /// True if Message Delivery Notifications (read receipts) should
    /// be sent and requested.
    #[strum(props(default = "1"))]
    MdnsEnabled,

    /// True if "Sent" folder should be watched for changes.
    #[strum(props(default = "0"))]
    SentboxWatch,

    /// True if chat messages should be moved to a separate folder.
    #[strum(props(default = "1"))]
    MvboxMove,

    /// Watch for new messages in the "Mvbox" (aka DeltaChat folder) only.
    ///
    /// This will not entirely disable other folders, e.g. the spam folder will also still
    /// be watched for new messages.
    #[strum(props(default = "0"))]
    OnlyFetchMvbox,

    /// Whether to show classic emails or only chat messages.
    #[strum(props(default = "2"))] // also change ShowEmails.default() on changes
    ShowEmails,

    /// Quality of the media files to send.
    #[strum(props(default = "0"))] // also change MediaQuality.default() on changes
    MediaQuality,

    /// If set to "1", on the first time `start_io()` is called after configuring,
    /// the newest existing messages are fetched.
    /// Existing recipients are added to the contact database regardless of this setting.
    #[strum(props(default = "0"))]
    FetchExistingMsgs,

    /// If set to "1", then existing messages are considered to be already fetched.
    /// This flag is reset after successful configuration.
    #[strum(props(default = "1"))]
    FetchedExistingMsgs,

    /// Type of the OpenPGP key to generate.
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

    /// Move messages to the Trash folder instead of marking them "\Deleted". Overrides
    /// `ProviderOptions::delete_to_trash`.
    DeleteToTrash,

    /// Save raw MIME messages with headers in the database if true.
    SaveMimeHeaders,

    /// The primary email address. Also see `SecondaryAddrs`.
    ConfiguredAddr,

    /// Configured IMAP server hostname.
    ConfiguredMailServer,

    /// Configured IMAP server username.
    ConfiguredMailUser,

    /// Configured IMAP server password.
    ConfiguredMailPw,

    /// Configured IMAP server port.
    ConfiguredMailPort,

    /// Configured IMAP server security (e.g. TLS, STARTTLS).
    ConfiguredMailSecurity,

    /// How to check IMAP server TLS certificates.
    ConfiguredImapCertificateChecks,

    /// Configured SMTP server hostname.
    ConfiguredSendServer,

    /// Configured SMTP server username.
    ConfiguredSendUser,

    /// Configured SMTP server password.
    ConfiguredSendPw,

    /// Configured SMTP server port.
    ConfiguredSendPort,

    /// How to check SMTP server TLS certificates.
    ConfiguredSmtpCertificateChecks,

    /// Whether OAuth 2 is used with configured provider.
    ConfiguredServerFlags,

    /// Configured SMTP server security (e.g. TLS, STARTTLS).
    ConfiguredSendSecurity,

    /// Configured folder for incoming messages.
    ConfiguredInboxFolder,

    /// Configured folder for chat messages.
    ConfiguredMvboxFolder,

    /// Configured "Sent" folder.
    ConfiguredSentboxFolder,

    /// Configured "Trash" folder.
    ConfiguredTrashFolder,

    /// Unix timestamp of the last successful configuration.
    ConfiguredTimestamp,

    /// ID of the configured provider from the provider database.
    ConfiguredProvider,

    /// True if account is configured.
    Configured,

    /// All secondary self addresses separated by spaces
    /// (`addr1@example.org addr2@example.org addr3@example.org`)
    SecondaryAddrs,

    /// Read-only core version string.
    #[strum(serialize = "sys.version")]
    SysVersion,

    /// Maximal recommended attachment size in bytes.
    #[strum(serialize = "sys.msgsize_max_recommended")]
    SysMsgsizeMaxRecommended,

    /// Space separated list of all config keys available.
    #[strum(serialize = "sys.config_keys")]
    SysConfigKeys,

    /// True if it is a bot account.
    Bot,

    /// True when to skip initial start messages in groups.
    #[strum(props(default = "0"))]
    SkipStartMessages,

    /// Whether we send a warning if the password is wrong (set to false when we send a warning
    /// because we do not want to send a second warning)
    #[strum(props(default = "0"))]
    NotifyAboutWrongPw,

    /// If a warning about exceeding quota was shown recently,
    /// this is the percentage of quota at the time the warning was given.
    /// Unset, when quota falls below minimal warning threshold again.
    QuotaExceeding,

    /// address to webrtc instance to use for videochats
    WebrtcInstance,

    /// Timestamp of the last time housekeeping was run
    LastHousekeeping,

    /// To how many seconds to debounce scan_all_folders. Used mainly in tests, to disable debouncing completely.
    #[strum(props(default = "60"))]
    ScanAllFoldersDebounceSecs,

    /// Whether to avoid using IMAP IDLE even if the server supports it.
    ///
    /// This is a developer option for testing "fake idle".
    #[strum(props(default = "0"))]
    DisableIdle,

    /// Defines the max. size (in bytes) of messages downloaded automatically.
    /// 0 = no limit.
    #[strum(props(default = "0"))]
    DownloadLimit,

    /// Enable sending and executing (applying) sync messages. Sending requires `BccSelf` to be set.
    #[strum(props(default = "1"))]
    SyncMsgs,

    /// Space-separated list of all the authserv-ids which we believe
    /// may be the one of our email server.
    ///
    /// See `crate::authres::update_authservid_candidates`.
    AuthservIdCandidates,

    /// Make all outgoing messages with Autocrypt header "multipart/signed".
    SignUnencrypted,

    /// Let the core save all events to the database.
    /// This value is used internally to remember the MsgId of the logging xdc
    #[strum(props(default = "0"))]
    DebugLogging,

    /// Last message processed by the bot.
    LastMsgId,

    /// How often to gossip Autocrypt keys in chats with multiple recipients, in seconds. 2 days by
    /// default.
    ///
    /// This is not supposed to be changed by UIs and only used for testing.
    #[strum(props(default = "172800"))]
    GossipPeriod,

    /// Feature flag for verified 1:1 chats; the UI should set it
    /// to 1 if it supports verified 1:1 chats.
    /// Regardless of this setting, `chat.is_protected()` returns true while the key is verified,
    /// and when the key changes, an info message is posted into the chat.
    /// 0=Nothing else happens when the key changes.
    /// 1=After the key changed, `can_send()` returns false and `is_protection_broken()` returns true
    /// until `chat_id.accept()` is called.
    #[strum(props(default = "0"))]
    VerifiedOneOnOneChats,
}

impl Context {
    /// Returns true if configuration value is set for the given key.
    pub async fn config_exists(&self, key: Config) -> Result<bool> {
        Ok(self.sql.get_raw_config(key.as_ref()).await?.is_some())
    }

    /// Get a configuration key. Returns `None` if no value is set, and no default value found.
    pub async fn get_config(&self, key: Config) -> Result<Option<String>> {
        let env_key = format!("DELTACHAT_{}", key.as_ref().to_uppercase());
        if let Ok(value) = env::var(env_key) {
            return Ok(Some(value));
        }

        let value = match key {
            Config::Selfavatar => {
                let rel_path = self.sql.get_raw_config(key.as_ref()).await?;
                rel_path.map(|p| {
                    get_abs_path(self, Path::new(&p))
                        .to_string_lossy()
                        .into_owned()
                })
            }
            Config::SysVersion => Some((*DC_VERSION_STR).clone()),
            Config::SysMsgsizeMaxRecommended => Some(format!("{RECOMMENDED_FILE_SIZE}")),
            Config::SysConfigKeys => Some(get_config_keys_string()),
            _ => self.sql.get_raw_config(key.as_ref()).await?,
        };

        if value.is_some() {
            return Ok(value);
        }

        // Default values
        match key {
            Config::ConfiguredInboxFolder => Ok(Some("INBOX".to_owned())),
            _ => Ok(key.get_str("default").map(|s| s.to_string())),
        }
    }

    /// Returns Some(T) if a value for the given key exists and was successfully parsed.
    /// Returns None if could not parse.
    pub async fn get_config_parsed<T: FromStr>(&self, key: Config) -> Result<Option<T>> {
        self.get_config(key)
            .await
            .map(|s: Option<String>| s.and_then(|s| s.parse().ok()))
    }

    /// Returns 32-bit signed integer configuration value for the given key.
    pub async fn get_config_int(&self, key: Config) -> Result<i32> {
        Ok(self.get_config_parsed(key).await?.unwrap_or_default())
    }

    /// Returns 32-bit unsigned integer configuration value for the given key.
    pub async fn get_config_u32(&self, key: Config) -> Result<u32> {
        Ok(self.get_config_parsed(key).await?.unwrap_or_default())
    }

    /// Returns 64-bit signed integer configuration value for the given key.
    pub async fn get_config_i64(&self, key: Config) -> Result<i64> {
        Ok(self.get_config_parsed(key).await?.unwrap_or_default())
    }

    /// Returns 64-bit unsigned integer configuration value for the given key.
    pub async fn get_config_u64(&self, key: Config) -> Result<u64> {
        Ok(self.get_config_parsed(key).await?.unwrap_or_default())
    }

    /// Returns boolean configuration value (if any) for the given key.
    pub async fn get_config_bool_opt(&self, key: Config) -> Result<Option<bool>> {
        Ok(self.get_config_parsed::<i32>(key).await?.map(|x| x != 0))
    }

    /// Returns boolean configuration value for the given key.
    pub async fn get_config_bool(&self, key: Config) -> Result<bool> {
        Ok(self.get_config_bool_opt(key).await?.unwrap_or_default())
    }

    /// Returns true if movebox ("DeltaChat" folder) should be watched.
    pub(crate) async fn should_watch_mvbox(&self) -> Result<bool> {
        Ok(self.get_config_bool(Config::MvboxMove).await?
            || self.get_config_bool(Config::OnlyFetchMvbox).await?)
    }

    /// Gets configured "delete_server_after" value.
    ///
    /// `None` means never delete the message, `Some(0)` means delete
    /// at once, `Some(x)` means delete after `x` seconds.
    pub async fn get_config_delete_server_after(&self) -> Result<Option<i64>> {
        match self.get_config_int(Config::DeleteServerAfter).await? {
            0 => Ok(None),
            1 => Ok(Some(0)),
            x => Ok(Some(i64::from(x))),
        }
    }

    /// Gets the configured provider, as saved in the `configured_provider` value.
    ///
    /// The provider is determined by `get_provider_info()` during configuration and then saved
    /// to the db in `param.save_to_database()`, together with all the other `configured_*` values.
    pub async fn get_configured_provider(&self) -> Result<Option<&'static Provider>> {
        if let Some(cfg) = self.get_config(Config::ConfiguredProvider).await? {
            return Ok(get_provider_by_id(&cfg));
        }
        Ok(None)
    }

    /// Gets configured "delete_device_after" value.
    ///
    /// `None` means never delete the message, `Some(x)` means delete
    /// after `x` seconds.
    pub async fn get_config_delete_device_after(&self) -> Result<Option<i64>> {
        match self.get_config_int(Config::DeleteDeviceAfter).await? {
            0 => Ok(None),
            x => Ok(Some(i64::from(x))),
        }
    }

    /// Set the given config key.
    /// If `None` is passed as a value the value is cleared and set to the default if there is one.
    pub async fn set_config(&self, key: Config, value: Option<&str>) -> Result<()> {
        match key {
            Config::Selfavatar => {
                self.sql
                    .execute("UPDATE contacts SET selfavatar_sent=0;", ())
                    .await?;
                match value {
                    Some(value) => {
                        let mut blob = BlobObject::new_from_path(self, value.as_ref()).await?;
                        blob.recode_to_avatar_size(self).await?;
                        self.sql
                            .set_raw_config(key.as_ref(), Some(blob.as_name()))
                            .await?;
                    }
                    None => {
                        self.sql.set_raw_config(key.as_ref(), None).await?;
                    }
                }
                self.emit_event(EventType::SelfavatarChanged);
            }
            Config::DeleteDeviceAfter => {
                let ret = self.sql.set_raw_config(key.as_ref(), value).await;
                // Interrupt ephemeral loop to delete old messages immediately.
                self.scheduler.interrupt_ephemeral_task().await;
                ret?
            }
            Config::Displayname => {
                let value = value.map(improve_single_line_input);
                self.sql
                    .set_raw_config(key.as_ref(), value.as_deref())
                    .await?;
            }
            Config::Socks5Enabled
            | Config::BccSelf
            | Config::E2eeEnabled
            | Config::MdnsEnabled
            | Config::SentboxWatch
            | Config::MvboxMove
            | Config::OnlyFetchMvbox
            | Config::FetchExistingMsgs
            | Config::DeleteToTrash
            | Config::SaveMimeHeaders
            | Config::Configured
            | Config::Bot
            | Config::NotifyAboutWrongPw
            | Config::SyncMsgs
            | Config::SignUnencrypted
            | Config::DisableIdle => {
                ensure!(
                    matches!(value, None | Some("0") | Some("1")),
                    "Boolean value must be either 0 or 1"
                );
                self.sql.set_raw_config(key.as_ref(), value).await?;
            }
            _ => {
                self.sql.set_raw_config(key.as_ref(), value).await?;
            }
        }
        Ok(())
    }

    /// Set the given config to an unsigned 32-bit integer value.
    pub async fn set_config_u32(&self, key: Config, value: u32) -> Result<()> {
        self.set_config(key, Some(&value.to_string())).await?;
        Ok(())
    }

    /// Set the given config to a boolean value.
    pub async fn set_config_bool(&self, key: Config, value: bool) -> Result<()> {
        self.set_config(key, if value { Some("1") } else { Some("0") })
            .await?;
        Ok(())
    }

    /// Sets an ui-specific key-value pair.
    /// Keys must be prefixed by `ui.`
    /// and should be followed by the name of the system and maybe subsystem,
    /// eg. `ui.desktop.linux.foo`, `ui.desktop.macos.bar`, `ui.ios.foobar`.
    pub async fn set_ui_config(&self, key: &str, value: Option<&str>) -> Result<()> {
        ensure!(key.starts_with("ui."), "set_ui_config(): prefix missing.");
        self.sql.set_raw_config(key, value).await
    }

    /// Gets an ui-specific value set by set_ui_config().
    pub async fn get_ui_config(&self, key: &str) -> Result<Option<String>> {
        ensure!(key.starts_with("ui."), "get_ui_config(): prefix missing.");
        self.sql.get_raw_config(key).await
    }
}

// Separate impl block for self address handling
impl Context {
    /// Determine whether the specified addr maps to the/a self addr.
    /// Returns `false` if no addresses are configured.
    pub(crate) async fn is_self_addr(&self, addr: &str) -> Result<bool> {
        Ok(self
            .get_config(Config::ConfiguredAddr)
            .await?
            .iter()
            .any(|a| addr_cmp(addr, a))
            || self
                .get_secondary_self_addrs()
                .await?
                .iter()
                .any(|a| addr_cmp(addr, a)))
    }

    /// Sets `primary_new` as the new primary self address and saves the old
    /// primary address (if exists) as a secondary address.
    ///
    /// This should only be used by test code and during configure.
    pub(crate) async fn set_primary_self_addr(&self, primary_new: &str) -> Result<()> {
        let old_addr = self.get_config(Config::ConfiguredAddr).await?;

        // add old primary address (if exists) to secondary addresses
        let mut secondary_addrs = self.get_all_self_addrs().await?;
        // never store a primary address also as a secondary
        secondary_addrs.retain(|a| !addr_cmp(a, primary_new));
        self.set_config(
            Config::SecondaryAddrs,
            Some(secondary_addrs.join(" ").as_str()),
        )
        .await?;

        self.set_config(Config::ConfiguredAddr, Some(primary_new))
            .await?;

        if let Some(old_addr) = old_addr {
            let old_addr = EmailAddress::new(&old_addr)?;
            let old_keypair = crate::key::load_keypair(self, &old_addr).await?;

            if let Some(mut old_keypair) = old_keypair {
                old_keypair.addr = EmailAddress::new(primary_new)?;
                crate::key::store_self_keypair(self, &old_keypair, crate::key::KeyPairUse::Default)
                    .await?;
            }
        }

        Ok(())
    }

    /// Returns all primary and secondary self addresses.
    pub(crate) async fn get_all_self_addrs(&self) -> Result<Vec<String>> {
        let primary_addrs = self.get_config(Config::ConfiguredAddr).await?.into_iter();
        let secondary_addrs = self.get_secondary_self_addrs().await?.into_iter();

        Ok(primary_addrs.chain(secondary_addrs).collect())
    }

    /// Returns all secondary self addresses.
    pub(crate) async fn get_secondary_self_addrs(&self) -> Result<Vec<String>> {
        let secondary_addrs = self
            .get_config(Config::SecondaryAddrs)
            .await?
            .unwrap_or_default();
        Ok(secondary_addrs
            .split_ascii_whitespace()
            .map(|s| s.to_string())
            .collect())
    }

    /// Returns the primary self address.
    /// Returns an error if no self addr is configured.
    pub async fn get_primary_self_addr(&self) -> Result<String> {
        self.get_config(Config::ConfiguredAddr)
            .await?
            .context("No self addr configured")
    }
}

/// Returns all available configuration keys concated together.
fn get_config_keys_string() -> String {
    let keys = Config::iter().fold(String::new(), |mut acc, key| {
        acc += key.as_ref();
        acc += " ";
        acc
    });

    format!(" {keys} ")
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use num_traits::FromPrimitive;

    use super::*;
    use crate::constants;
    use crate::test_utils::TestContext;

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

    /// Tests that "bot" config can only be set to "0" or "1".
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_config_bot() {
        let t = TestContext::new().await;

        assert!(t.set_config(Config::Bot, None).await.is_ok());
        assert!(t.set_config(Config::Bot, Some("0")).await.is_ok());
        assert!(t.set_config(Config::Bot, Some("1")).await.is_ok());
        assert!(t.set_config(Config::Bot, Some("2")).await.is_err());
        assert!(t.set_config(Config::Bot, Some("Foobar")).await.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_media_quality_config_option() {
        let t = TestContext::new().await;
        let media_quality = t.get_config_int(Config::MediaQuality).await.unwrap();
        assert_eq!(media_quality, 0);
        let media_quality = constants::MediaQuality::from_i32(media_quality).unwrap_or_default();
        assert_eq!(media_quality, constants::MediaQuality::Balanced);

        t.set_config(Config::MediaQuality, Some("1")).await.unwrap();

        let media_quality = t.get_config_int(Config::MediaQuality).await.unwrap();
        assert_eq!(media_quality, 1);
        assert_eq!(constants::MediaQuality::Worse as i32, 1);
        let media_quality = constants::MediaQuality::from_i32(media_quality).unwrap_or_default();
        assert_eq!(media_quality, constants::MediaQuality::Worse);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ui_config() -> Result<()> {
        let t = TestContext::new().await;

        assert_eq!(t.get_ui_config("ui.desktop.linux.systray").await?, None);

        t.set_ui_config("ui.android.screen_security", Some("safe"))
            .await?;
        assert_eq!(
            t.get_ui_config("ui.android.screen_security").await?,
            Some("safe".to_string())
        );

        t.set_ui_config("ui.android.screen_security", None).await?;
        assert_eq!(t.get_ui_config("ui.android.screen_security").await?, None);

        assert!(t.set_ui_config("configured", Some("bar")).await.is_err());

        Ok(())
    }

    /// Regression test for https://github.com/deltachat/deltachat-core-rust/issues/3012
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_config_bool() -> Result<()> {
        let t = TestContext::new().await;

        // We need some config that defaults to true
        let c = Config::E2eeEnabled;
        assert_eq!(t.get_config_bool(c).await?, true);
        t.set_config_bool(c, false).await?;
        assert_eq!(t.get_config_bool(c).await?, false);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_self_addrs() -> Result<()> {
        let alice = TestContext::new_alice().await;

        assert!(alice.is_self_addr("alice@example.org").await?);
        assert_eq!(alice.get_all_self_addrs().await?, vec!["alice@example.org"]);
        assert!(!alice.is_self_addr("alice@alice.com").await?);

        // Test adding the same primary address
        alice.set_primary_self_addr("alice@example.org").await?;
        alice.set_primary_self_addr("Alice@Example.Org").await?;
        assert_eq!(alice.get_all_self_addrs().await?, vec!["Alice@Example.Org"]);

        // Test adding a new (primary) self address
        // The address is trimmed during configure by `LoginParam::from_database()`,
        // so `set_primary_self_addr()` doesn't have to trim it.
        alice.set_primary_self_addr("Alice@alice.com").await?;
        assert!(alice.is_self_addr("aliCe@example.org").await?);
        assert!(alice.is_self_addr("alice@alice.com").await?);
        assert_eq!(
            alice.get_all_self_addrs().await?,
            vec!["Alice@alice.com", "Alice@Example.Org"]
        );

        // Check that the entry is not duplicated
        alice.set_primary_self_addr("alice@alice.com").await?;
        alice.set_primary_self_addr("alice@alice.com").await?;
        assert_eq!(
            alice.get_all_self_addrs().await?,
            vec!["alice@alice.com", "Alice@Example.Org"]
        );

        // Test switching back
        alice.set_primary_self_addr("alice@example.org").await?;
        assert_eq!(
            alice.get_all_self_addrs().await?,
            vec!["alice@example.org", "alice@alice.com"]
        );

        // Test setting a new primary self address, the previous self address
        // should be kept as a secondary self address
        alice.set_primary_self_addr("alice@alice.xyz").await?;
        assert_eq!(
            alice.get_all_self_addrs().await?,
            vec!["alice@alice.xyz", "alice@example.org", "alice@alice.com"]
        );
        assert!(alice.is_self_addr("alice@example.org").await?);
        assert!(alice.is_self_addr("alice@alice.com").await?);
        assert!(alice.is_self_addr("Alice@alice.xyz").await?);

        Ok(())
    }
}

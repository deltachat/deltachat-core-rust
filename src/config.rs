//! # Key-value configuration management.

use anyhow::{ensure, Context as _, Result};
use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumIter, EnumProperty, EnumString};

use crate::blob::BlobObject;
use crate::constants::DC_VERSION_STR;
use crate::contact::addr_cmp;
use crate::context::Context;
use crate::dc_tools::{dc_get_abs_path, improve_single_line_input, EmailAddress};
use crate::events::EventType;
use crate::mimefactory::RECOMMENDED_FILE_SIZE;
use crate::provider::{get_provider_by_id, Provider};

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

    Socks5Enabled,
    Socks5Host,
    Socks5Port,
    Socks5User,
    Socks5Password,

    Displayname,
    Selfstatus,
    Selfavatar,

    #[strum(props(default = "0"))]
    BccSelf,

    #[strum(props(default = "1"))]
    E2eeEnabled,

    #[strum(props(default = "1"))]
    MdnsEnabled,

    #[strum(props(default = "0"))]
    SentboxWatch,

    #[strum(props(default = "1"))]
    MvboxMove,

    /// Watch for new messages in the "Mvbox" (aka DeltaChat folder) only.
    ///
    /// This will not entirely disable other folders, e.g. the spam folder will also still
    /// be watched for new messages.
    #[strum(props(default = "0"))]
    OnlyFetchMvbox,

    #[strum(props(default = "0"))] // also change ShowEmails.default() on changes
    ShowEmails,

    #[strum(props(default = "0"))] // also change MediaQuality.default() on changes
    MediaQuality,

    /// If set to "1", on the first time `start_io()` is called after configuring,
    /// the newest existing messages are fetched.
    /// Existing recipients are added to the contact database regardless of this setting.
    #[strum(props(default = "1"))]
    FetchExistingMsgs,

    /// If set to "1", then existing messages are considered to be already fetched.
    /// This flag is reset after successful configuration.
    #[strum(props(default = "1"))]
    FetchedExistingMsgs,

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
    /// The primary email address. Also see `SecondaryAddrs`.
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
    ConfiguredTimestamp,
    ConfiguredProvider,
    Configured,

    /// All secondary self addresses separated by spaces
    /// (`addr1@example.org addr2@exapmle.org addr3@example.org`)
    SecondaryAddrs,

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

    /// Defines the max. size (in bytes) of messages downloaded automatically.
    /// 0 = no limit.
    #[strum(props(default = "0"))]
    DownloadLimit,

    /// Send sync messages, requires `BccSelf` to be set as well.
    /// In a future versions, this switch may be removed.
    #[strum(props(default = "0"))]
    SendSyncMsgs,
}

impl Context {
    pub async fn config_exists(&self, key: Config) -> Result<bool> {
        Ok(self.sql.get_raw_config(key).await?.is_some())
    }

    /// Get a configuration key. Returns `None` if no value is set, and no default value found.
    pub async fn get_config(&self, key: Config) -> Result<Option<String>> {
        let value = match key {
            Config::Selfavatar => {
                let rel_path = self.sql.get_raw_config(key).await?;
                rel_path.map(|p| dc_get_abs_path(self, &p).to_string_lossy().into_owned())
            }
            Config::SysVersion => Some((&*DC_VERSION_STR).clone()),
            Config::SysMsgsizeMaxRecommended => Some(format!("{}", RECOMMENDED_FILE_SIZE)),
            Config::SysConfigKeys => Some(get_config_keys_string()),
            _ => self.sql.get_raw_config(key).await?,
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

    pub async fn get_config_int(&self, key: Config) -> Result<i32> {
        self.get_config(key)
            .await
            .map(|s: Option<String>| s.and_then(|s| s.parse().ok()).unwrap_or_default())
    }

    pub async fn get_config_i64(&self, key: Config) -> Result<i64> {
        self.get_config(key)
            .await
            .map(|s: Option<String>| s.and_then(|s| s.parse().ok()).unwrap_or_default())
    }

    pub async fn get_config_u64(&self, key: Config) -> Result<u64> {
        self.get_config(key)
            .await
            .map(|s: Option<String>| s.and_then(|s| s.parse().ok()).unwrap_or_default())
    }

    pub async fn get_config_bool(&self, key: Config) -> Result<bool> {
        Ok(self.get_config_int(key).await? != 0)
    }

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
                    .execute("UPDATE contacts SET selfavatar_sent=0;", paramsv![])
                    .await?;
                self.sql
                    .set_raw_config_bool("attach_selfavatar", true)
                    .await?;
                match value {
                    Some(value) => {
                        let mut blob = BlobObject::new_from_path(self, value.as_ref()).await?;
                        blob.recode_to_avatar_size(self).await?;
                        self.sql.set_raw_config(key, Some(blob.as_name())).await?;
                    }
                    None => {
                        self.sql.set_raw_config(key, None).await?;
                    }
                }
                self.emit_event(EventType::SelfavatarChanged);
            }
            Config::DeleteDeviceAfter => {
                let ret = self.sql.set_raw_config(key, value).await;
                // Interrupt ephemeral loop to delete old messages immediately.
                self.interrupt_ephemeral_task().await;
                ret?
            }
            Config::Displayname => {
                let value = value.map(improve_single_line_input);
                self.sql.set_raw_config(key, value.as_deref()).await?;
            }
            _ => {
                self.sql.set_raw_config(key, value).await?;
            }
        }
        Ok(())
    }

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

    format!(" {} ", keys)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;

    use std::str::FromStr;
    use std::string::ToString;

    use crate::chat;
    use crate::chat::ChatId;
    use crate::constants;
    use crate::contact;
    use crate::contact::Contact;
    use crate::contact::ContactId;
    use crate::dc_receive_imf::dc_receive_imf;
    use crate::message::Message;
    use crate::peerstate;
    use crate::peerstate::Peerstate;
    use crate::stock_str;
    use crate::test_utils::TestContext;
    use crate::test_utils::TestContextManager;
    use num_traits::FromPrimitive;

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_change_primary_self_addr() -> Result<()> {
        let mut tcm = TestContextManager::new().await;
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        tcm.send_recv_accept(&alice, &bob, "Hi").await;
        let bob_alice_chat = bob.create_chat(&alice).await;

        tcm.change_addr(&alice, "alice@someotherdomain.xyz").await;

        tcm.section("Bob sends a message to Alice, encrypting to her previous key");
        let sent = bob.send_text(bob_alice_chat.id, "hi back").await;

        // Alice set up message forwarding so that she still receives
        // the message with her new address
        let alice_msg = alice.recv_msg(&sent).await;
        assert_eq!(alice_msg.text, Some("hi back".to_string()));
        assert_eq!(alice_msg.get_showpadlock(), true);
        let alice_bob_chat = alice.create_chat(&bob).await;
        assert_eq!(alice_msg.chat_id, alice_bob_chat.id);

        tcm.section("Bob sends a message to Alice without In-Reply-To");
        // Even if Bob sends a message to Alice without In-Reply-To,
        // it's still assigned to the 1:1 chat with Bob and not to
        // a group (without secondary addresses, an ad-hoc group
        // would be created)
        dc_receive_imf(
            &alice,
            b"From: bob@example.net
To: alice@example.org
Chat-Version: 1.0
Message-ID: <456@example.com>

Message w/out In-Reply-To
",
            false,
        )
        .await?;

        let alice_msg = alice.get_last_msg().await;

        assert_eq!(
            alice_msg.text,
            Some("Message w/out In-Reply-To".to_string())
        );
        assert_eq!(alice_msg.get_showpadlock(), false);
        assert_eq!(alice_msg.chat_id, alice_bob_chat.id);

        Ok(())
    }

    // TODO refactoring: Should this really be placed here? But I wouldn't know a better place, either
    #[async_std::test]
    async fn test_aeap_transition_0() {
        check_aeap_transition(0, false, false).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_1() {
        check_aeap_transition(1, false, false).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_0_verified() {
        check_aeap_transition(0, true, false).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_1_verified() {
        check_aeap_transition(1, true, false).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_2_verified() {
        check_aeap_transition(2, true, false).await;
    }

    #[async_std::test]
    async fn test_aeap_transition_0_bob_knew_new_addr() {
        check_aeap_transition(0, false, true).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_1_bob_knew_new_addr() {
        check_aeap_transition(1, false, true).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_0_verified_bob_knew_new_addr() {
        check_aeap_transition(0, true, true).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_1_verified_bob_knew_new_addr() {
        check_aeap_transition(1, true, true).await;
    }
    #[async_std::test]
    async fn test_aeap_transition_2_verified_bob_knew_new_addr() {
        check_aeap_transition(2, true, true).await;
    }

    /// Happy path test for AEAP in various configurations.
    async fn check_aeap_transition(round: u32, verified: bool, bob_knew_new_addr: bool) {
        // Alice's new address is "fiona@example.net" so that we can test
        // the case where Bob already had contact with Alice's new address
        const ALICE_NEW_ADDR: &str = "fiona@example.net";

        let mut tcm = TestContextManager::new().await;
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        if bob_knew_new_addr {
            let fiona = tcm.fiona().await;

            tcm.send_recv_accept(&fiona, &bob, "Hi").await;
            tcm.send_recv_accept(&bob, &fiona, "Hi back").await;
        }

        tcm.send_recv_accept(&alice, &bob, "Hi").await;
        tcm.send_recv_accept(&bob, &alice, "Hi back").await;

        if verified {
            mark_as_verified(&alice, &bob).await;
            mark_as_verified(&bob, &alice).await;
        }

        let mut groups = vec![
            chat::create_group_chat(&bob, chat::ProtectionStatus::Unprotected, "Group 0")
                .await
                .unwrap(),
            chat::create_group_chat(&bob, chat::ProtectionStatus::Unprotected, "Group 1")
                .await
                .unwrap(),
        ];
        if verified {
            groups.push(
                chat::create_group_chat(&bob, chat::ProtectionStatus::Protected, "Group 2")
                    .await
                    .unwrap(),
            );
            groups.push(
                chat::create_group_chat(&bob, chat::ProtectionStatus::Protected, "Group 3")
                    .await
                    .unwrap(),
            );
        }

        let old_contact = Contact::create(&bob, "Alice", "alice@example.org")
            .await
            .unwrap();
        for group in &groups {
            chat::add_contact_to_chat(&bob, *group, old_contact)
                .await
                .unwrap();
        }

        // groups 0 and 2 stay unpromoted (i.e. local
        // on Bob's device, Alice doesn't know about them)
        tcm.section("Promoting group 1");
        let sent = bob.send_text(groups[1], "group created").await;
        let group1_alice = alice.recv_msg(&sent).await.chat_id;

        let mut group3_alice = None;
        if verified {
            tcm.section("Promoting group 3");
            let sent = bob.send_text(groups[3], "group created").await;
            group3_alice = Some(alice.recv_msg(&sent).await.chat_id);
        }

        tcm.change_addr(&alice, ALICE_NEW_ADDR).await;

        tcm.section("Alice sends another message to Bob, this time from her new addr");
        // No matter to which chat Alice send, the transition should be done in all groups
        let chat_to_send = match round {
            0 => alice.create_chat(&bob).await.id,
            1 => group1_alice,
            2 => group3_alice.expect("There is no round 2 for verified=false"),
            _ => panic!("There is no round {}", round),
        };
        let sent = alice
            .send_text(chat_to_send, "Hello from my new addr!")
            .await;
        let recvd = bob.recv_msg(&sent).await;
        let sent_timestamp = recvd.timestamp_sent;
        assert_eq!(recvd.text.unwrap(), "Hello from my new addr!");

        tcm.section("Check that the AEAP transition worked");
        check_that_transition_worked(
            &groups,
            &alice,
            "alice@example.org",
            ALICE_NEW_ADDR,
            "Alice",
            &bob,
        )
        .await;

        // Assert that the autocrypt header is also applied to the peerstate
        // if the address changed
        let bob_alice_peerstate = Peerstate::from_addr(&bob, ALICE_NEW_ADDR)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bob_alice_peerstate.last_seen, sent_timestamp);
        assert_eq!(bob_alice_peerstate.last_seen_autocrypt, sent_timestamp);

        tcm.section("Test switching back");
        tcm.change_addr(&alice, "alice@example.org").await;
        let sent = alice
            .send_text(chat_to_send, "Hello from my old addr!")
            .await;
        let recvd = bob.recv_msg(&sent).await;
        assert_eq!(recvd.text.unwrap(), "Hello from my old addr!");

        check_that_transition_worked(
            &groups,
            &alice,
            // Note that "alice@example.org" and ALICE_NEW_ADDR are switched now:
            ALICE_NEW_ADDR,
            "alice@example.org",
            // Alice's display name will be her address at this point, since Bob didn't set anything:
            ALICE_NEW_ADDR,
            &bob,
        )
        .await;
    }

    async fn check_that_transition_worked(
        groups: &[ChatId],
        alice: &TestContext,
        old_alice_addr: &str,
        new_alice_addr: &str,
        name: &str,
        bob: &TestContext,
    ) {
        let old_contact = Contact::lookup_id_by_addr(bob, old_alice_addr, contact::Origin::Unknown)
            .await
            .unwrap()
            .unwrap();
        let new_contact = Contact::lookup_id_by_addr(bob, new_alice_addr, contact::Origin::Unknown)
            .await
            .unwrap()
            .unwrap();

        for group in groups {
            assert!(!chat::is_contact_in_chat(bob, *group, old_contact)
                .await
                .unwrap());
            assert!(chat::is_contact_in_chat(bob, *group, new_contact)
                .await
                .unwrap());

            let info_msg = get_last_info_msg(bob, *group).await;
            let expected_text =
                stock_str::aeap_addr_changed(bob, name, old_alice_addr, new_alice_addr).await;
            assert_eq!(info_msg.text.unwrap(), expected_text);
            assert_eq!(info_msg.from_id, ContactId::INFO);

            let msg = format!("Sending to group {}", group);
            let sent = bob.send_text(*group, &msg).await;
            let recvd = alice.recv_msg(&sent).await;
            assert_eq!(recvd.text.unwrap(), msg);
        }
    }

    async fn mark_as_verified(this: &TestContext, other: &TestContext) {
        let other_addr = other.get_primary_self_addr().await.unwrap();
        let mut peerstate = peerstate::Peerstate::from_addr(this, &other_addr)
            .await
            .unwrap()
            .unwrap();

        peerstate.verified_key = peerstate.public_key.clone();
        peerstate.verified_key_fingerprint = peerstate.public_key_fingerprint.clone();
        peerstate.to_save = Some(peerstate::ToSave::All);

        peerstate.save_to_db(&this.sql, false).await.unwrap();
    }

    async fn get_last_info_msg(t: &TestContext, chat_id: ChatId) -> Message {
        let msgs = chat::get_chat_msgs(&t.ctx, chat_id, constants::DC_GCM_INFO_ONLY)
            .await
            .unwrap();
        let msg_id = if let chat::ChatItem::Message { msg_id } = msgs.last().unwrap() {
            msg_id
        } else {
            panic!("Wrong item type");
        };
        Message::load_from_db(&t.ctx, *msg_id).await.unwrap()
    }

    /// Test that an attacker - here Fiona - can't replay a message sent by Alice
    /// to make Bob think that there was a transition to Fiona's address.
    #[async_std::test]
    async fn test_aeap_replay_attack() -> Result<()> {
        let mut tcm = TestContextManager::new().await;
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        tcm.send_recv_accept(&alice, &bob, "Hi").await;
        tcm.send_recv_accept(&bob, &alice, "Hi back").await;

        let group =
            chat::create_group_chat(&bob, chat::ProtectionStatus::Unprotected, "Group 0").await?;

        let bob_alice_contact = Contact::create(&bob, "Alice", "alice@example.org").await?;
        chat::add_contact_to_chat(&bob, group, bob_alice_contact).await?;

        // Alice sends a message which Bob doesn't receive or something
        // A real attack would rather re-use a message that was sent to a group
        // and replace the Message-Id or so.
        let chat = alice.create_chat(&bob).await;
        let sent = alice.send_text(chat.id, "whoop whoop").await;

        // Fiona gets the message, replaces the From addr...
        let sent = sent
            .payload()
            .replace("From: <alice@example.org>", "From: <fiona@example.net>")
            .replace("addr=alice@example.org;", "addr=fiona@example.net;");
        sent.find("From: <fiona@example.net>").unwrap(); // Assert that it worked
        sent.find("addr=fiona@example.net;").unwrap(); // Assert that it worked

        tcm.section("Fiona replaced the From addr and forwards the message to Bob");
        dc_receive_imf(&bob, sent.as_bytes(), false).await?.unwrap();

        // Check that no transition was done
        assert!(chat::is_contact_in_chat(&bob, group, bob_alice_contact).await?);
        let bob_fiona_contact = Contact::create(&bob, "", "fiona@example.net").await?;
        assert!(!chat::is_contact_in_chat(&bob, group, bob_fiona_contact).await?);

        Ok(())
    }
}

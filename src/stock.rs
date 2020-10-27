//! Module to work with translatable stock strings

use std::borrow::Cow;

use strum::EnumProperty;
use strum_macros::EnumProperty;

use crate::chat;
use crate::chat::ProtectionStatus;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
use crate::contact::*;
use crate::context::Context;
use crate::error::{bail, Error};
use crate::message::Message;
use crate::param::Param;
use crate::stock::StockMessage::{DeviceMessagesHint, WelcomeMessage};
use crate::{blob::BlobObject, config::Config};

/// Stock strings
///
/// These identify the string to return in [Context.stock_str].  The
/// numbers must stay in sync with `deltachat.h` `DC_STR_*` constants.
///
/// See the `stock_*` methods on [Context] to use these.
///
/// [Context]: crate::context::Context
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, EnumProperty)]
#[repr(u32)]
pub enum StockMessage {
    #[strum(props(fallback = "No messages."))]
    NoMessages = 1,

    #[strum(props(fallback = "Me"))]
    SelfMsg = 2,

    #[strum(props(fallback = "Draft"))]
    Draft = 3,

    #[strum(props(fallback = "Voice message"))]
    VoiceMessage = 7,

    #[strum(props(fallback = "Contact requests"))]
    DeadDrop = 8,

    #[strum(props(fallback = "Image"))]
    Image = 9,

    #[strum(props(fallback = "Video"))]
    Video = 10,

    #[strum(props(fallback = "Audio"))]
    Audio = 11,

    #[strum(props(fallback = "File"))]
    File = 12,

    #[strum(props(fallback = "Sent with my Delta Chat Messenger: https://delta.chat"))]
    StatusLine = 13,

    #[strum(props(fallback = "Hello, I\'ve just created the group \"%1$s\" for us."))]
    NewGroupDraft = 14,

    #[strum(props(fallback = "Group name changed from \"%1$s\" to \"%2$s\"."))]
    MsgGrpName = 15,

    #[strum(props(fallback = "Group image changed."))]
    MsgGrpImgChanged = 16,

    #[strum(props(fallback = "Member %1$s added."))]
    MsgAddMember = 17,

    #[strum(props(fallback = "Member %1$s removed."))]
    MsgDelMember = 18,

    #[strum(props(fallback = "Group left."))]
    MsgGroupLeft = 19,

    #[strum(props(fallback = "GIF"))]
    Gif = 23,

    #[strum(props(fallback = "Encrypted message"))]
    EncryptedMsg = 24,

    #[strum(props(fallback = "End-to-end encryption available."))]
    E2eAvailable = 25,

    #[strum(props(fallback = "Transport-encryption."))]
    EncrTransp = 27,

    #[strum(props(fallback = "No encryption."))]
    EncrNone = 28,

    #[strum(props(fallback = "This message was encrypted for another setup."))]
    CantDecryptMsgBody = 29,

    #[strum(props(fallback = "Fingerprints"))]
    FingerPrints = 30,

    #[strum(props(fallback = "Return receipt"))]
    ReadRcpt = 31,

    #[strum(props(fallback = "This is a return receipt for the message \"%1$s\"."))]
    ReadRcptMailBody = 32,

    #[strum(props(fallback = "Group image deleted."))]
    MsgGrpImgDeleted = 33,

    #[strum(props(fallback = "End-to-end encryption preferred."))]
    E2ePreferred = 34,

    #[strum(props(fallback = "%1$s verified."))]
    ContactVerified = 35,

    #[strum(props(fallback = "Cannot verify %1$s"))]
    ContactNotVerified = 36,

    #[strum(props(fallback = "Changed setup for %1$s"))]
    ContactSetupChanged = 37,

    #[strum(props(fallback = "Archived chats"))]
    ArchivedChats = 40,

    #[strum(props(fallback = "Autocrypt Setup Message"))]
    AcSetupMsgSubject = 42,

    #[strum(props(
        fallback = "This is the Autocrypt Setup Message used to transfer your key between clients.\n\nTo decrypt and use your key, open the message in an Autocrypt-compliant client and enter the setup code presented on the generating device."
    ))]
    AcSetupMsgBody = 43,

    #[strum(props(
        fallback = "Cannot login as \"%1$s\". Please check if the email address and the password are correct."
    ))]
    CannotLogin = 60,

    #[strum(props(fallback = "Could not connect to %1$s: %2$s"))]
    ServerResponse = 61,

    #[strum(props(fallback = "%1$s by %2$s."))]
    MsgActionByUser = 62,

    #[strum(props(fallback = "%1$s by me."))]
    MsgActionByMe = 63,

    #[strum(props(fallback = "Location streaming enabled."))]
    MsgLocationEnabled = 64,

    #[strum(props(fallback = "Location streaming disabled."))]
    MsgLocationDisabled = 65,

    #[strum(props(fallback = "Location"))]
    Location = 66,

    #[strum(props(fallback = "Sticker"))]
    Sticker = 67,

    #[strum(props(fallback = "Device messages"))]
    DeviceMessages = 68,

    #[strum(props(fallback = "Saved messages"))]
    SavedMessages = 69,

    #[strum(props(
        fallback = "Messages in this chat are generated locally by your Delta Chat app. \
                    Its makers use it to inform about app updates and problems during usage."
    ))]
    DeviceMessagesHint = 70,

    #[strum(props(fallback = "Welcome to Delta Chat! – \
                    Delta Chat looks and feels like other popular messenger apps, \
                    but does not involve centralized control, \
                    tracking or selling you, friends, colleagues or family out to large organizations.\n\n\
                    Technically, Delta Chat is an email application with a modern chat interface. \
                    Email in a new dress if you will 👻\n\n\
                    Use Delta Chat with anyone out of billions of people: just use their e-mail address. \
                    Recipients don't need to install Delta Chat, visit websites or sign up anywhere - \
                    however, of course, if they like, you may point them to 👉 https://get.delta.chat"))]
    WelcomeMessage = 71,

    #[strum(props(fallback = "Unknown sender for this chat. See 'info' for more details."))]
    UnknownSenderForChat = 72,

    #[strum(props(fallback = "Message from %1$s"))]
    SubjectForNewContact = 73,

    #[strum(props(fallback = "Failed to send message to %1$s."))]
    FailedSendingTo = 74,

    #[strum(props(fallback = "Message deletion timer is disabled."))]
    MsgEphemeralTimerDisabled = 75,

    // A fallback message for unknown timer values.
    // "s" stands for "second" SI unit here.
    #[strum(props(fallback = "Message deletion timer is set to %1$s s."))]
    MsgEphemeralTimerEnabled = 76,

    #[strum(props(fallback = "Message deletion timer is set to 1 minute."))]
    MsgEphemeralTimerMinute = 77,

    #[strum(props(fallback = "Message deletion timer is set to 1 hour."))]
    MsgEphemeralTimerHour = 78,

    #[strum(props(fallback = "Message deletion timer is set to 1 day."))]
    MsgEphemeralTimerDay = 79,

    #[strum(props(fallback = "Message deletion timer is set to 1 week."))]
    MsgEphemeralTimerWeek = 80,

    #[strum(props(fallback = "Message deletion timer is set to 4 weeks."))]
    MsgEphemeralTimerFourWeeks = 81,

    #[strum(props(fallback = "Video chat invitation"))]
    VideochatInvitation = 82,

    #[strum(props(fallback = "You are invited to a video chat, click %1$s to join."))]
    VideochatInviteMsgBody = 83,

    #[strum(props(fallback = "Error:\n\n“%1$s”"))]
    ConfigurationFailed = 84,

    #[strum(props(
        fallback = "⚠️ Date or time of your device seem to be inaccurate (%1$s).\n\n\
                    Adjust your clock ⏰🔧 to ensure your messages are received correctly."
    ))]
    BadTimeMsgBody = 85,

    #[strum(props(fallback = "⚠️ Your Delta Chat version might be outdated.\n\n\
                    This may cause problems because your chat partners use newer versions - \
                    and you are missing the latest features 😳\n\
                    Please check https://get.delta.chat or your app store for updates."))]
    UpdateReminderMsgBody = 86,

    #[strum(props(
        fallback = "Could not find your mail server.\n\nPlease check your internet connection."
    ))]
    ErrorNoNetwork = 87,

    #[strum(props(fallback = "Chat protection enabled."))]
    ProtectionEnabled = 88,

    #[strum(props(fallback = "Chat protection disabled."))]
    ProtectionDisabled = 89,

    // used in summaries, a noun, not a verb (not: "to reply")
    #[strum(props(fallback = "Reply"))]
    ReplyNoun = 90,

    #[strum(props(fallback = "You deleted the \"Saved messages\" chat.\n\n\
                    To use the \"Saved messages\" feature again, create a new chat with yourself."))]
    SelfDeletedMsgBody = 91,
}

/*
"
*/

impl StockMessage {
    /// Default untranslated strings for stock messages.
    ///
    /// These could be used in logging calls, so no logging here.
    fn fallback(self) -> &'static str {
        self.get_str("fallback").unwrap_or_default()
    }
}

impl Context {
    /// Set the stock string for the [StockMessage].
    ///
    pub async fn set_stock_translation(
        &self,
        id: StockMessage,
        stockstring: String,
    ) -> Result<(), Error> {
        if stockstring.contains("%1") && !id.fallback().contains("%1") {
            bail!(
                "translation {} contains invalid %1 placeholder, default is {}",
                stockstring,
                id.fallback()
            );
        }
        if stockstring.contains("%2") && !id.fallback().contains("%2") {
            bail!(
                "translation {} contains invalid %2 placeholder, default is {}",
                stockstring,
                id.fallback()
            );
        }
        self.translated_stockstrings
            .write()
            .await
            .insert(id as usize, stockstring);
        Ok(())
    }

    /// Return the stock string for the [StockMessage].
    ///
    /// Return a translation (if it was set with set_stock_translation before)
    /// or a default (English) string.
    pub async fn stock_str(&self, id: StockMessage) -> Cow<'_, str> {
        match self
            .translated_stockstrings
            .read()
            .await
            .get(&(id as usize))
        {
            Some(ref x) => Cow::Owned((*x).to_string()),
            None => Cow::Borrowed(id.fallback()),
        }
    }

    /// Return stock string, replacing placeholders with provided string.
    ///
    /// This replaces both the *first* `%1$s`, `%1$d` and `%1$@`
    /// placeholders with the provided string.
    /// (the `%1$@` variant is used on iOS, the other are used on Android and Desktop)
    pub async fn stock_string_repl_str(&self, id: StockMessage, insert: impl AsRef<str>) -> String {
        self.stock_str(id)
            .await
            .replacen("%1$s", insert.as_ref(), 1)
            .replacen("%1$d", insert.as_ref(), 1)
            .replacen("%1$@", insert.as_ref(), 1)
    }

    /// Return stock string, replacing placeholders with provided int.
    ///
    /// Like [Context::stock_string_repl_str] but substitute the placeholders
    /// with an integer.
    pub async fn stock_string_repl_int(&self, id: StockMessage, insert: i32) -> String {
        self.stock_string_repl_str(id, format!("{}", insert).as_str())
            .await
    }

    /// Return stock string, replacing 2 placeholders with provided string.
    ///
    /// This replaces both the *first* `%1$s`, `%1$d` and `%1$@`
    /// placeholders with the string in `insert` and does the same for
    /// `%2$s`, `%2$d` and `%2$@` for `insert2`.
    /// (the `%1$@` variant is used on iOS, the other are used on Android and Desktop)
    pub async fn stock_string_repl_str2(
        &self,
        id: StockMessage,
        insert: impl AsRef<str>,
        insert2: impl AsRef<str>,
    ) -> String {
        self.stock_str(id)
            .await
            .replacen("%1$s", insert.as_ref(), 1)
            .replacen("%1$d", insert.as_ref(), 1)
            .replacen("%1$@", insert.as_ref(), 1)
            .replacen("%2$s", insert2.as_ref(), 1)
            .replacen("%2$d", insert2.as_ref(), 1)
            .replacen("%2$@", insert2.as_ref(), 1)
    }

    /// Return some kind of stock message
    ///
    /// If the `id` is [StockMessage::MsgAddMember] or
    /// [StockMessage::MsgDelMember] then `param1` is considered to be the
    /// contact address and will be replaced by that contact's display
    /// name.
    ///
    /// If `from_id` is not `0`, any trailing dot is removed from the
    /// first stock string created so far.  If the `from_id` contact is
    /// the user itself, i.e. `DC_CONTACT_ID_SELF` the string is used
    /// itself as param to the [StockMessage::MsgActionByMe] stock string
    /// resulting in a string like "Member Alice added by me." (for
    /// [StockMessage::MsgAddMember] as `id`).  If the `from_id` contact
    /// is any other user than the contact's display name is looked up and
    /// used as the second parameter to [StockMessage::MsgActionByUser] with
    /// again the original stock string being used as the first parameter,
    /// resulting in a string like "Member Alice added by Bob.".
    pub async fn stock_system_msg(
        &self,
        id: StockMessage,
        param1: impl AsRef<str>,
        param2: impl AsRef<str>,
        from_id: u32,
    ) -> String {
        let insert1 = if id == StockMessage::MsgAddMember || id == StockMessage::MsgDelMember {
            let contact_id =
                Contact::lookup_id_by_addr(self, param1.as_ref(), Origin::Unknown).await;
            if contact_id != 0 {
                Contact::get_by_id(self, contact_id)
                    .await
                    .map(|contact| contact.get_name_n_addr())
                    .unwrap_or_default()
            } else {
                param1.as_ref().to_string()
            }
        } else {
            param1.as_ref().to_string()
        };

        let action = self
            .stock_string_repl_str2(id, insert1, param2.as_ref().to_string())
            .await;
        let action1 = action.trim_end_matches('.');
        match from_id {
            0 => action,
            DC_CONTACT_ID_SELF => {
                self.stock_string_repl_str(StockMessage::MsgActionByMe, action1)
                    .await
            }
            _ => {
                let displayname = Contact::get_by_id(self, from_id)
                    .await
                    .map(|contact| contact.get_name_n_addr())
                    .unwrap_or_default();

                self.stock_string_repl_str2(StockMessage::MsgActionByUser, action1, &displayname)
                    .await
            }
        }
    }

    /// Returns a stock message saying that protection status has changed.
    pub async fn stock_protection_msg(&self, protect: ProtectionStatus, from_id: u32) -> String {
        self.stock_system_msg(
            match protect {
                ProtectionStatus::Protected => StockMessage::ProtectionEnabled,
                ProtectionStatus::Unprotected => StockMessage::ProtectionDisabled,
            },
            "",
            "",
            from_id,
        )
        .await
    }

    pub(crate) async fn update_device_chats(&self) -> Result<(), Error> {
        if self.get_config_bool(Config::Bot).await {
            return Ok(());
        }

        // create saved-messages chat; we do this only once, if the user has deleted the chat,
        // he can recreate it manually (make sure we do not re-add it when configure() was called a second time)
        if !self.sql.get_raw_config_bool(&self, "self-chat-added").await {
            self.sql
                .set_raw_config_bool(&self, "self-chat-added", true)
                .await?;
            chat::create_by_contact_id(&self, DC_CONTACT_ID_SELF).await?;
        }

        // add welcome-messages. by the label, this is done only once,
        // if the user has deleted the message or the chat, it is not added again.
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(self.stock_str(DeviceMessagesHint).await.to_string());
        chat::add_device_msg(&self, Some("core-about-device-chat"), Some(&mut msg)).await?;

        let image = include_bytes!("../assets/welcome-image.jpg");
        let blob = BlobObject::create(&self, "welcome-image.jpg".to_string(), image).await?;
        let mut msg = Message::new(Viewtype::Image);
        msg.param.set(Param::File, blob.as_name());
        chat::add_device_msg(&self, Some("core-welcome-image"), Some(&mut msg)).await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(self.stock_str(WelcomeMessage).await.to_string());
        chat::add_device_msg(&self, Some("core-welcome"), Some(&mut msg)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    use crate::constants::DC_CONTACT_ID_SELF;

    use crate::chatlist::Chatlist;
    use num_traits::ToPrimitive;

    #[test]
    fn test_enum_mapping() {
        assert_eq!(StockMessage::NoMessages.to_usize().unwrap(), 1);
        assert_eq!(StockMessage::SelfMsg.to_usize().unwrap(), 2);
    }

    #[test]
    fn test_fallback() {
        assert_eq!(StockMessage::NoMessages.fallback(), "No messages.");
    }

    #[async_std::test]
    async fn test_set_stock_translation() {
        let t = TestContext::new().await;
        t.ctx
            .set_stock_translation(StockMessage::NoMessages, "xyz".to_string())
            .await
            .unwrap();
        assert_eq!(t.ctx.stock_str(StockMessage::NoMessages).await, "xyz")
    }

    #[async_std::test]
    async fn test_set_stock_translation_wrong_replacements() {
        let t = TestContext::new().await;
        assert!(t
            .ctx
            .set_stock_translation(StockMessage::NoMessages, "xyz %1$s ".to_string())
            .await
            .is_err());
        assert!(t
            .ctx
            .set_stock_translation(StockMessage::NoMessages, "xyz %2$s ".to_string())
            .await
            .is_err());
    }

    #[async_std::test]
    async fn test_stock_str() {
        let t = TestContext::new().await;
        assert_eq!(
            t.ctx.stock_str(StockMessage::NoMessages).await,
            "No messages."
        );
    }

    #[async_std::test]
    async fn test_stock_string_repl_str() {
        let t = TestContext::new().await;
        // uses %1$s substitution
        assert_eq!(
            t.ctx
                .stock_string_repl_str(StockMessage::MsgAddMember, "Foo")
                .await,
            "Member Foo added."
        );
        // We have no string using %1$d to test...
    }

    #[async_std::test]
    async fn test_stock_string_repl_int() {
        let t = TestContext::new().await;
        assert_eq!(
            t.ctx
                .stock_string_repl_int(StockMessage::MsgAddMember, 42)
                .await,
            "Member 42 added."
        );
    }

    #[async_std::test]
    async fn test_stock_string_repl_str2() {
        let t = TestContext::new().await;
        assert_eq!(
            t.ctx
                .stock_string_repl_str2(StockMessage::ServerResponse, "foo", "bar")
                .await,
            "Could not connect to foo: bar"
        );
    }

    #[async_std::test]
    async fn test_stock_system_msg_simple() {
        let t = TestContext::new().await;
        assert_eq!(
            t.ctx
                .stock_system_msg(StockMessage::MsgLocationEnabled, "", "", 0)
                .await,
            "Location streaming enabled."
        )
    }

    #[async_std::test]
    async fn test_stock_system_msg_add_member_by_me() {
        let t = TestContext::new().await;
        assert_eq!(
            t.ctx
                .stock_system_msg(
                    StockMessage::MsgAddMember,
                    "alice@example.com",
                    "",
                    DC_CONTACT_ID_SELF
                )
                .await,
            "Member alice@example.com added by me."
        )
    }

    #[async_std::test]
    async fn test_stock_system_msg_add_member_by_me_with_displayname() {
        let t = TestContext::new().await;
        Contact::create(&t.ctx, "Alice", "alice@example.com")
            .await
            .expect("failed to create contact");
        assert_eq!(
            t.ctx
                .stock_system_msg(
                    StockMessage::MsgAddMember,
                    "alice@example.com",
                    "",
                    DC_CONTACT_ID_SELF
                )
                .await,
            "Member Alice (alice@example.com) added by me."
        );
    }

    #[async_std::test]
    async fn test_stock_system_msg_add_member_by_other_with_displayname() {
        let t = TestContext::new().await;
        let contact_id = {
            Contact::create(&t.ctx, "Alice", "alice@example.com")
                .await
                .expect("Failed to create contact Alice");
            Contact::create(&t.ctx, "Bob", "bob@example.com")
                .await
                .expect("failed to create bob")
        };
        assert_eq!(
            t.ctx
                .stock_system_msg(
                    StockMessage::MsgAddMember,
                    "alice@example.com",
                    "",
                    contact_id,
                )
                .await,
            "Member Alice (alice@example.com) added by Bob (bob@example.com)."
        );
    }

    #[async_std::test]
    async fn test_stock_system_msg_grp_name() {
        let t = TestContext::new().await;
        assert_eq!(
            t.ctx
                .stock_system_msg(
                    StockMessage::MsgGrpName,
                    "Some chat",
                    "Other chat",
                    DC_CONTACT_ID_SELF
                )
                .await,
            "Group name changed from \"Some chat\" to \"Other chat\" by me."
        )
    }

    #[async_std::test]
    async fn test_stock_system_msg_grp_name_other() {
        let t = TestContext::new().await;
        let id = Contact::create(&t.ctx, "Alice", "alice@example.com")
            .await
            .expect("failed to create contact");

        assert_eq!(
            t.ctx
                .stock_system_msg(StockMessage::MsgGrpName, "Some chat", "Other chat", id)
                .await,
            "Group name changed from \"Some chat\" to \"Other chat\" by Alice (alice@example.com)."
        )
    }

    #[async_std::test]
    async fn test_update_device_chats() {
        let t = TestContext::new().await;
        t.ctx.update_device_chats().await.ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 2);

        chats.get_chat_id(0).delete(&t.ctx).await.ok();
        chats.get_chat_id(1).delete(&t.ctx).await.ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);

        // a subsequent call to update_device_chats() must not re-add manally deleted messages or chats
        t.ctx.update_device_chats().await.ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);
    }
}

//! Module to work with translatable stock strings

use std::borrow::Cow;

use strum::EnumProperty;
use strum_macros::EnumProperty;

use crate::blob::BlobObject;
use crate::chat;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
use crate::contact::*;
use crate::context::Context;
use crate::error::Error;
use crate::message::Message;
use crate::param::Param;
use crate::stock::StockMessage::{DeviceMessagesHint, WelcomeMessage};

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

    #[strum(props(fallback = "%1$s member(s)"))]
    Member = 4,

    #[strum(props(fallback = "%1$s contact(s)"))]
    Contact = 6,

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

    #[strum(props(fallback = "Starred messages"))]
    StarredMsgs = 41,

    #[strum(props(fallback = "Autocrypt Setup Message"))]
    AcSetupMsgSubject = 42,

    #[strum(props(
        fallback = "This is the Autocrypt Setup Message used to transfer your key between clients.\n\nTo decrypt and use your key, open the message in an Autocrypt-compliant client and enter the setup code presented on the generating device."
    ))]
    AcSetupMsgBody = 43,

    #[strum(props(fallback = "Messages I sent to myself"))]
    SelfTalkSubTitle = 50,

    #[strum(props(fallback = "Cannot login as %1$s."))]
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

    #[strum(props(fallback = "Unknown Sender for this chat. See 'info' for more details."))]
    UnknownSenderForChat = 72,
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
    pub fn set_stock_translation(
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
            .unwrap()
            .insert(id as usize, stockstring);
        Ok(())
    }

    /// Return the stock string for the [StockMessage].
    ///
    /// Return a translation (if it was set with set_stock_translation before)
    /// or a default (English) string.
    pub fn stock_str(&self, id: StockMessage) -> Cow<str> {
        match self
            .translated_stockstrings
            .read()
            .unwrap()
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
    pub fn stock_string_repl_str(&self, id: StockMessage, insert: impl AsRef<str>) -> String {
        self.stock_str(id)
            .replacen("%1$s", insert.as_ref(), 1)
            .replacen("%1$d", insert.as_ref(), 1)
            .replacen("%1$@", insert.as_ref(), 1)
    }

    /// Return stock string, replacing placeholders with provided int.
    ///
    /// Like [Context::stock_string_repl_str] but substitute the placeholders
    /// with an integer.
    pub fn stock_string_repl_int(&self, id: StockMessage, insert: i32) -> String {
        self.stock_string_repl_str(id, format!("{}", insert).as_str())
    }

    /// Return stock string, replacing 2 placeholders with provided string.
    ///
    /// This replaces both the *first* `%1$s`, `%1$d` and `%1$@`
    /// placeholders with the string in `insert` and does the same for
    /// `%2$s`, `%2$d` and `%2$@` for `insert2`.
    /// (the `%1$@` variant is used on iOS, the other are used on Android and Desktop)
    pub fn stock_string_repl_str2(
        &self,
        id: StockMessage,
        insert: impl AsRef<str>,
        insert2: impl AsRef<str>,
    ) -> String {
        self.stock_str(id)
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
    pub fn stock_system_msg(
        &self,
        id: StockMessage,
        param1: impl AsRef<str>,
        param2: impl AsRef<str>,
        from_id: u32,
    ) -> String {
        let insert1 = if id == StockMessage::MsgAddMember || id == StockMessage::MsgDelMember {
            let contact_id = Contact::lookup_id_by_addr(self, param1.as_ref());
            if contact_id != 0 {
                Contact::get_by_id(self, contact_id)
                    .map(|contact| contact.get_name_n_addr())
                    .unwrap_or_default()
            } else {
                param1.as_ref().to_string()
            }
        } else {
            param1.as_ref().to_string()
        };

        let action = self.stock_string_repl_str2(id, insert1, param2.as_ref().to_string());
        let action1 = action.trim_end_matches('.');
        match from_id {
            0 => action,
            1 => self.stock_string_repl_str(StockMessage::MsgActionByMe, action1), // DC_CONTACT_ID_SELF
            _ => {
                let displayname = Contact::get_by_id(self, from_id)
                    .map(|contact| contact.get_name_n_addr())
                    .unwrap_or_default();

                self.stock_string_repl_str2(StockMessage::MsgActionByUser, action1, &displayname)
            }
        }
    }

    pub fn update_device_chats(&self) -> Result<(), Error> {
        // check for the LAST added device message - if it is present, we can skip message creation.
        // this is worthwhile as this function is typically called
        // by the ui on every probram start or even on every opening of the chatlist.
        if chat::was_device_msg_ever_added(&self, "core-welcome")? {
            return Ok(());
        }

        // create saved-messages chat;
        // we do this only once, if the user has deleted the chat, he can recreate it manually.
        if !self.sql.get_raw_config_bool(&self, "self-chat-added") {
            self.sql
                .set_raw_config_bool(&self, "self-chat-added", true)?;
            chat::create_by_contact_id(&self, DC_CONTACT_ID_SELF)?;
        }

        // add welcome-messages. by the label, this is done only once,
        // if the user has deleted the message or the chat, it is not added again.
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(self.stock_str(DeviceMessagesHint).to_string());
        chat::add_device_msg(&self, Some("core-about-device-chat"), Some(&mut msg))?;

        let image = include_bytes!("../assets/welcome-image.jpg");
        let blob = BlobObject::create(&self, "welcome-image.jpg".to_string(), image)?;
        let mut msg = Message::new(Viewtype::Image);
        msg.param.set(Param::File, blob.as_name());
        chat::add_device_msg(&self, Some("core-welcome-image"), Some(&mut msg))?;

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(self.stock_str(WelcomeMessage).to_string());
        chat::add_device_msg(&self, Some("core-welcome"), Some(&mut msg))?;
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

    #[test]
    fn test_set_stock_translation() {
        let t = dummy_context();
        t.ctx
            .set_stock_translation(StockMessage::NoMessages, "xyz".to_string())
            .unwrap();
        assert_eq!(t.ctx.stock_str(StockMessage::NoMessages), "xyz")
    }

    #[test]
    fn test_set_stock_translation_wrong_replacements() {
        let t = dummy_context();
        assert!(t
            .ctx
            .set_stock_translation(StockMessage::NoMessages, "xyz %1$s ".to_string())
            .is_err());
        assert!(t
            .ctx
            .set_stock_translation(StockMessage::NoMessages, "xyz %2$s ".to_string())
            .is_err());
    }

    #[test]
    fn test_stock_str() {
        let t = dummy_context();
        assert_eq!(t.ctx.stock_str(StockMessage::NoMessages), "No messages.");
    }

    #[test]
    fn test_stock_string_repl_str() {
        let t = dummy_context();
        // uses %1$s substitution
        assert_eq!(
            t.ctx.stock_string_repl_str(StockMessage::Member, "42"),
            "42 member(s)"
        );
        // We have no string using %1$d to test...
    }

    #[test]
    fn test_stock_string_repl_int() {
        let t = dummy_context();
        assert_eq!(
            t.ctx.stock_string_repl_int(StockMessage::Member, 42),
            "42 member(s)"
        );
    }

    #[test]
    fn test_stock_string_repl_str2() {
        let t = dummy_context();
        assert_eq!(
            t.ctx
                .stock_string_repl_str2(StockMessage::ServerResponse, "foo", "bar"),
            "Could not connect to foo: bar"
        );
    }

    #[test]
    fn test_stock_system_msg_simple() {
        let t = dummy_context();
        assert_eq!(
            t.ctx
                .stock_system_msg(StockMessage::MsgLocationEnabled, "", "", 0),
            "Location streaming enabled."
        )
    }

    #[test]
    fn test_stock_system_msg_add_member_by_me() {
        let t = dummy_context();
        assert_eq!(
            t.ctx.stock_system_msg(
                StockMessage::MsgAddMember,
                "alice@example.com",
                "",
                DC_CONTACT_ID_SELF
            ),
            "Member alice@example.com added by me."
        )
    }

    #[test]
    fn test_stock_system_msg_add_member_by_me_with_displayname() {
        let t = dummy_context();
        Contact::create(&t.ctx, "Alice", "alice@example.com").expect("failed to create contact");
        assert_eq!(
            t.ctx.stock_system_msg(
                StockMessage::MsgAddMember,
                "alice@example.com",
                "",
                DC_CONTACT_ID_SELF
            ),
            "Member Alice (alice@example.com) added by me."
        );
    }

    #[test]
    fn test_stock_system_msg_add_member_by_other_with_displayname() {
        let t = dummy_context();
        let contact_id = {
            Contact::create(&t.ctx, "Alice", "alice@example.com")
                .expect("Failed to create contact Alice");
            Contact::create(&t.ctx, "Bob", "bob@example.com").expect("failed to create bob")
        };
        assert_eq!(
            t.ctx.stock_system_msg(
                StockMessage::MsgAddMember,
                "alice@example.com",
                "",
                contact_id,
            ),
            "Member Alice (alice@example.com) added by Bob (bob@example.com)."
        );
    }

    #[test]
    fn test_stock_system_msg_grp_name() {
        let t = dummy_context();
        assert_eq!(
            t.ctx.stock_system_msg(
                StockMessage::MsgGrpName,
                "Some chat",
                "Other chat",
                DC_CONTACT_ID_SELF
            ),
            "Group name changed from \"Some chat\" to \"Other chat\" by me."
        )
    }

    #[test]
    fn test_stock_system_msg_grp_name_other() {
        let t = dummy_context();
        let id = Contact::create(&t.ctx, "Alice", "alice@example.com")
            .expect("failed to create contact");

        assert_eq!(
            t.ctx
                .stock_system_msg(StockMessage::MsgGrpName, "Some chat", "Other chat", id,),
            "Group name changed from \"Some chat\" to \"Other chat\" by Alice (alice@example.com)."
        )
    }

    #[test]
    fn test_update_device_chats() {
        let t = dummy_context();
        t.ctx.update_device_chats().ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 2);

        chats.get_chat_id(0).delete(&t.ctx).ok();
        chats.get_chat_id(1).delete(&t.ctx).ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 0);

        // a subsequent call to update_device_chats() must not re-add manally deleted messages or chats
        t.ctx.update_device_chats().ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 0);
    }
}

//! Module to work with translatable stock strings

use std::borrow::Cow;

use anyhow::{bail, Error};
use strum::EnumProperty;
use strum_macros::EnumProperty;

use crate::blob::BlobObject;
use crate::chat;
use crate::chat::ProtectionStatus;
use crate::config::Config;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
use crate::contact::{Contact, Origin};
use crate::context::Context;
use crate::message::Message;
use crate::param::Param;

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

    #[strum(props(
        fallback = "⚠️ The \"Delete messages from server\" feature now also deletes messages in folders other than Inbox, DeltaChat and Sent.\n\n\
                    ℹ️ To avoid accidentally deleting messages, we turned it off for you. Please turn it on again at \
                    Settings → \"Chats and Media\" → \"Delete messages from server\" to continue using it."
    ))]
    DeleteServerTurnedOff = 92,

    #[strum(props(fallback = "Message deletion timer is set to %1$s minutes."))]
    MsgEphemeralTimerMinutes = 93,

    #[strum(props(fallback = "Message deletion timer is set to %1$s hours."))]
    MsgEphemeralTimerHours = 94,

    #[strum(props(fallback = "Message deletion timer is set to %1$s days."))]
    MsgEphemeralTimerDays = 95,

    #[strum(props(fallback = "Message deletion timer is set to %1$s weeks."))]
    MsgEphemeralTimerWeeks = 96,
}

impl StockMessage {
    /// Default untranslated strings for stock messages.
    ///
    /// These could be used in logging calls, so no logging here.
    fn fallback(self) -> &'static str {
        self.get_str("fallback").unwrap_or_default()
    }
}

/// Builder for a stock string.
///
/// See [`NoMessages`] or any other stock string in this module for an example of how to use
/// this.
struct StockString<'a> {
    context: &'a Context,
}

impl<'a> StockString<'a> {
    /// Creates a new [`StockString`] builder.
    fn new(context: &'a Context) -> Self {
        Self { context }
    }

    /// Looks up a translation and returns a further builder.
    ///
    /// This will look up the translation in the [`Context`] if one is registered.  It
    /// returns a further builder type which can be used to substitute replacement strings
    /// or build the final message.
    async fn id(self, id: StockMessage) -> TranslatedStockString<'a> {
        TranslatedStockString {
            context: self.context,
            message: self
                .context
                .translated_stockstrings
                .read()
                .await
                .get(&(id as usize))
                .map(|s| Cow::Owned(s.to_owned()))
                .unwrap_or_else(|| Cow::Borrowed(id.fallback())),
        }
    }
}

/// Stock string builder which allows retrieval of the message.
///
/// This builder allows retrieval of the message using [`TranslatedStockString::msg`], if it
/// needs substitutions first however it provides further builder methods.
struct TranslatedStockString<'a> {
    context: &'a Context,
    message: Cow<'static, str>,
}

impl<'a> TranslatedStockString<'a> {
    /// Retrieves the built message.
    fn msg(self) -> Cow<'static, str> {
        self.message
    }

    /// Substitutes the first replacement value if one is present.
    fn replace1(self, replacement: impl AsRef<str>) -> Self {
        let msg = self
            .message
            .as_ref()
            .replacen("%1$s", replacement.as_ref(), 1)
            .replacen("%1$d", replacement.as_ref(), 1)
            .replacen("%1$@", replacement.as_ref(), 1);
        Self {
            context: self.context,
            message: Cow::Owned(msg),
        }
    }

    /// Substitutes the second replacement value if one is present.
    ///
    /// Be aware you probably should have also called [`TranslatedStockString::replace1`] if
    /// you are calling this.
    fn replace2(self, replacement: impl AsRef<str>) -> Self {
        let msg = self
            .message
            .as_ref()
            .replacen("%2$s", replacement.as_ref(), 1)
            .replacen("%2$d", replacement.as_ref(), 1)
            .replacen("%2$@", replacement.as_ref(), 1);
        Self {
            context: self.context,
            message: Cow::Owned(msg),
        }
    }

    /// Augments the message by saying it was performed by a user.
    ///
    /// This looks up the display name of `contact` and uses the [`MsgActionByMe`] and
    /// [`MsgActionByUser`] stock strings to turn the stock string in one that says the
    /// action was performed by this user.
    ///
    /// E.g. this turns `Group image changed.` into `Group image changed by me.` or `Group
    /// image changed by Alice`.
    ///
    /// Note that the original message should end in a `.`.
    async fn action_by_contact(self, contact: u32) -> TranslatedStockString<'a> {
        let message = self.message.as_ref().trim_end_matches('.');
        let message = match contact {
            DC_CONTACT_ID_SELF => MsgActionByMe::stock_str(self.context, message).await,
            _ => {
                let displayname = Contact::get_by_id(self.context, contact)
                    .await
                    .map(|contact| contact.get_name_n_addr())
                    .unwrap_or_else(|_| format!("{}", contact));
                MsgActionByUser::stock_str(self.context, message, displayname).await
            }
        };
        TranslatedStockString {
            context: self.context,
            message,
        }
    }
}

#[derive(Debug)]
pub(crate) enum NoMessages {}

impl NoMessages {
    /// Stock string: `No messages.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::NoMessages)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum SelfMsg {}

impl SelfMsg {
    /// Stock string: `Me`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::SelfMsg)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum Draft {}

impl Draft {
    /// Stock string: `Draft`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::Draft)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum VoiceMessage {}

impl VoiceMessage {
    /// Stock string: `Voice message`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::VoiceMessage)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum DeadDrop {}

impl DeadDrop {
    /// Stock string: `Contact requests`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::DeadDrop)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum Image {}

impl Image {
    /// Stock string: `Image`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::Image)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum Video {}

impl Video {
    /// Stock string: `Video`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::Video)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum Audio {}

impl Audio {
    /// Stock string: `Audio`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::Audio)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum File {}

impl File {
    /// Stock string: `File`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context).id(StockMessage::File).await.msg()
    }
}

#[derive(Debug)]
pub(crate) enum StatusLine {}

impl StatusLine {
    /// Stock string: `Sent with my Delta Chat Messenger: https://delta.chat`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::StatusLine)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum NewGroupDraft {}

impl NewGroupDraft {
    /// Stock string: `Hello, I've just created the group "%1$s" for us.`.
    pub async fn stock_str(context: &Context, group_name: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::NewGroupDraft)
            .await
            .replace1(group_name)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgGrpName {}

impl MsgGrpName {
    /// Stock string: `Group name changed from "%1$s" to "%2$s".`.
    pub async fn stock_str(
        context: &Context,
        from_group: impl AsRef<str>,
        to_group: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgGrpName)
            .await
            .replace1(from_group)
            .replace2(to_group)
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgGrpImgChanged {}

impl MsgGrpImgChanged {
    /// Stock string: `Group image changed.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgGrpImgChanged)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgAddMember {}

impl MsgAddMember {
    /// Stock string: `Member %1$s added.`.
    ///
    /// The `added_member_addr` parameter should be an email address and is looked up in the
    /// contacts to combine with the display name.
    pub async fn stock_str(
        context: &Context,
        added_member_addr: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        let addr = added_member_addr.as_ref();
        let who = match Contact::lookup_id_by_addr(context, addr, Origin::Unknown).await {
            Ok(Some(contact_id)) => Contact::get_by_id(context, contact_id)
                .await
                .map(|contact| contact.get_name_n_addr())
                .unwrap_or_else(|_| addr.to_string()),
            _ => addr.to_string(),
        };
        StockString::new(context)
            .id(StockMessage::MsgAddMember)
            .await
            .replace1(who)
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgDelMember {}

impl MsgDelMember {
    /// Stock string: `Member %1$s removed.`.
    ///
    /// The `removed_member_addr` parameter should be an email address and is looked up in
    /// the contacts to combine with the display name.
    pub async fn stock_str(
        context: &Context,
        removed_member_addr: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        let addr = removed_member_addr.as_ref();
        let who = match Contact::lookup_id_by_addr(context, addr, Origin::Unknown).await {
            Ok(Some(contact_id)) => Contact::get_by_id(context, contact_id)
                .await
                .map(|contact| contact.get_name_n_addr())
                .unwrap_or_else(|_| addr.to_string()),
            _ => addr.to_string(),
        };
        StockString::new(context)
            .id(StockMessage::MsgDelMember)
            .await
            .replace1(who)
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgGroupLeft {}

impl MsgGroupLeft {
    /// Stock string: `Group left.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgGroupLeft)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum Gif {}

impl Gif {
    /// Stock string: `GIF`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context).id(StockMessage::Gif).await.msg()
    }
}

#[derive(Debug)]
pub(crate) enum EncryptedMsg {}

impl EncryptedMsg {
    /// Stock string: `Encrypted message`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::EncryptedMsg)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum E2eAvailable {}

impl E2eAvailable {
    /// Stock string: `End-to-end encryption available.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::E2eAvailable)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum EncrNone {}

impl EncrNone {
    /// Stock string: `No encryption.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::EncrNone)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum CantDecryptMsgBody {}

impl CantDecryptMsgBody {
    /// Stock string: `This message was encrypted for another setup.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::CantDecryptMsgBody)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum FingerPrints {}

impl FingerPrints {
    /// Stock string: `Fingerprints`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::FingerPrints)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ReadRcpt {}

impl ReadRcpt {
    /// Stock string: `Return receipt`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ReadRcpt)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ReadRcptMailBody {}

impl ReadRcptMailBody {
    /// Stock string: `This is a return receipt for the message "%1$s".`.
    pub async fn stock_str(context: &Context, message: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ReadRcptMailBody)
            .await
            .replace1(message)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgGrpImgDeleted {}

impl MsgGrpImgDeleted {
    /// Stock string: `Group image deleted.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgGrpImgDeleted)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum E2ePreferred {}

impl E2ePreferred {
    /// Stock string: `End-to-end encryption preferred.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::E2ePreferred)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ContactVerified {}

impl ContactVerified {
    /// Stock string: `%1$s verified.`.
    pub async fn stock_str(context: &Context, contact_addr: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ContactVerified)
            .await
            .replace1(contact_addr)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ContactNotVerified {}

impl ContactNotVerified {
    /// Stock string: `Cannot verify %1$s`.
    pub async fn stock_str(context: &Context, contact_addr: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ContactNotVerified)
            .await
            .replace1(contact_addr)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ContactSetupChanged {}

impl ContactSetupChanged {
    /// Stock string: `Changed setup for %1$s`.
    pub async fn stock_str(context: &Context, contact_addr: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ContactSetupChanged)
            .await
            .replace1(contact_addr)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ArchivedChats {}

impl ArchivedChats {
    /// Stock string: `Archived chats`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ArchivedChats)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum AcSetupMsgSubject {}

impl AcSetupMsgSubject {
    /// Stock string: `Autocrypt Setup Message`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::AcSetupMsgSubject)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum AcSetupMsgBody {}

impl AcSetupMsgBody {
    /// Stock string: `This is the Autocrypt Setup Message used to transfer...`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::AcSetupMsgBody)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum CannotLogin {}

impl CannotLogin {
    /// Stock string: `Cannot login as \"%1$s\". Please check...`.
    pub async fn stock_str(context: &Context, user: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::CannotLogin)
            .await
            .replace1(user)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ServerResponse {}

impl ServerResponse {
    /// Stock string: `Could not connect to %1$s: %2$s`.
    pub async fn stock_str(
        context: &Context,
        server: impl AsRef<str>,
        details: impl AsRef<str>,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ServerResponse)
            .await
            .replace1(server)
            .replace2(details)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgActionByUser {}

impl MsgActionByUser {
    /// Stock string: `%1$s by %2$s.`.
    pub async fn stock_str(
        context: &Context,
        action: impl AsRef<str>,
        user: impl AsRef<str>,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgActionByUser)
            .await
            .replace1(action)
            .replace2(user)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgActionByMe {}

impl MsgActionByMe {
    /// Stock string: `%1$s by me.`.
    pub async fn stock_str(context: &Context, action: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgActionByMe)
            .await
            .replace1(action)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgLocationEnabled {}

impl MsgLocationEnabled {
    /// Stock string: `Location streaming enabled.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgLocationEnabled)
            .await
            .msg()
    }

    /// Stock string: `Location streaming enabled.`.
    pub async fn stock_str_by(context: &Context, contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgLocationEnabled)
            .await
            .action_by_contact(contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgLocationDisabled {}

impl MsgLocationDisabled {
    /// Stock string: `Location streaming disabled.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgLocationDisabled)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum Location {}

impl Location {
    /// Stock string: `Location`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::Location)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum Sticker {}

impl Sticker {
    /// Stock string: `Sticker`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::Sticker)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum DeviceMessages {}

impl DeviceMessages {
    /// Stock string: `Device messages`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::DeviceMessages)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum SavedMessages {}

impl SavedMessages {
    /// Stock string: `Saved messages`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::SavedMessages)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum DeviceMessagesHint {}

impl DeviceMessagesHint {
    /// Stock string: `Messages in this chat are generated locally by...`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::DeviceMessagesHint)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum WelcomeMessage {}

impl WelcomeMessage {
    /// Stock string: `Welcome to Delta Chat! – ...`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::WelcomeMessage)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum UnknownSenderForChat {}

impl UnknownSenderForChat {
    /// Stock string: `Unknown sender for this chat. See 'info' for more details.`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::UnknownSenderForChat)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum SubjectForNewContact {}

impl SubjectForNewContact {
    /// Stock string: `Message from %1$s`.
    // TODO: This can compute `self_name` itself instead of asking the caller to do this.
    pub async fn stock_str(context: &Context, self_name: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::SubjectForNewContact)
            .await
            .replace1(self_name)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum FailedSendingTo {}

impl FailedSendingTo {
    /// Stock string: `Failed to send message to %1$s.`.
    pub async fn stock_str(context: &Context, name: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::FailedSendingTo)
            .await
            .replace1(name)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerDisabled {}

impl MsgEphemeralTimerDisabled {
    /// Stock string: `Message deletion timer is disabled.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerDisabled)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerEnabled {}

impl MsgEphemeralTimerEnabled {
    /// Stock string: `Message deletion timer is set to %1$s s.`.
    pub async fn stock_str(
        context: &Context,
        timer: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerEnabled)
            .await
            .replace1(timer)
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerMinute {}

impl MsgEphemeralTimerMinute {
    /// Stock string: `Message deletion timer is set to 1 minute.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerMinute)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerHour {}

impl MsgEphemeralTimerHour {
    /// Stock string: `Message deletion timer is set to 1 hour.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerHour)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerDay {}

impl MsgEphemeralTimerDay {
    /// Stock string: `Message deletion timer is set to 1 day.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerDay)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerWeek {}

impl MsgEphemeralTimerWeek {
    /// Stock string: `Message deletion timer is set to 1 week.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerWeek)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum VideochatInvitation {}

impl VideochatInvitation {
    /// Stock string: `Video chat invitation`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::VideochatInvitation)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum VideochatInviteMsgBody {}

impl VideochatInviteMsgBody {
    /// Stock string: `You are invited to a video chat, click %1$s to join.`.
    pub async fn stock_str(context: &Context, url: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::VideochatInviteMsgBody)
            .await
            .replace1(url)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ConfigurationFailed {}

impl ConfigurationFailed {
    /// Stock string: `Error:\n\n“%1$s”`.
    pub async fn stock_str(context: &Context, details: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ConfigurationFailed)
            .await
            .replace1(details)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum BadTimeMsgBody {}

impl BadTimeMsgBody {
    /// Stock string: `⚠️ Date or time of your device seem to be inaccurate (%1$s)...`.
    // TODO: This could compute now itself.
    pub async fn stock_str(context: &Context, now: impl AsRef<str>) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::BadTimeMsgBody)
            .await
            .replace1(now)
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum UpdateReminderMsgBody {}

impl UpdateReminderMsgBody {
    /// Stock string: `⚠️ Your Delta Chat version might be outdated...`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::UpdateReminderMsgBody)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ErrorNoNetwork {}

impl ErrorNoNetwork {
    /// Stock string: `Could not find your mail server...`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ErrorNoNetwork)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ProtectionEnabled {}

impl ProtectionEnabled {
    /// Stock string: `Chat protection enabled.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ProtectionEnabled)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ProtectionDisabled {}

impl ProtectionDisabled {
    /// Stock string: `Chat protection disabled.`.
    pub async fn stock_str(context: &Context, by_contact: u32) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ProtectionDisabled)
            .await
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum ReplyNoun {}

impl ReplyNoun {
    /// Stock string: `Reply`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::ReplyNoun)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum SelfDeletedMsgBody {}

impl SelfDeletedMsgBody {
    /// Stock string: `You deleted the \"Saved messages\" chat...`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::SelfDeletedMsgBody)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum DeleteServerTurnedOff {}

impl DeleteServerTurnedOff {
    /// Stock string: `⚠️ The "Delete messages from server" feature now also...`.
    pub async fn stock_str(context: &Context) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::DeleteServerTurnedOff)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerMinutes {}

impl MsgEphemeralTimerMinutes {
    /// Stock string: `Message deletion timer is set to %1$s minutes.`.
    pub async fn stock_str(
        context: &Context,
        minutes: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerMinutes)
            .await
            .replace1(minutes)
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerHours {}

impl MsgEphemeralTimerHours {
    /// Stock string: `Message deletion timer is set to %1$s hours.`.
    pub async fn stock_str(
        context: &Context,
        hours: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerHours)
            .await
            .replace1(hours)
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerDays {}

impl MsgEphemeralTimerDays {
    /// Stock string: `Message deletion timer is set to %1$s days.`.
    pub async fn stock_str(
        context: &Context,
        days: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerDays)
            .await
            .replace1(days)
            .action_by_contact(by_contact)
            .await
            .msg()
    }
}

#[derive(Debug)]
pub(crate) enum MsgEphemeralTimerWeeks {}

impl MsgEphemeralTimerWeeks {
    /// Stock string: `Message deletion timer is set to %1$s weeks.`.
    pub async fn stock_str(
        context: &Context,
        weeks: impl AsRef<str>,
        by_contact: u32,
    ) -> Cow<'static, str> {
        StockString::new(context)
            .id(StockMessage::MsgEphemeralTimerWeeks)
            .await
            .replace1(weeks)
            .action_by_contact(by_contact)
            .await
            .msg()
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

    /// Returns a stock message saying that protection status has changed.
    pub(crate) async fn stock_protection_msg(
        &self,
        protect: ProtectionStatus,
        from_id: u32,
    ) -> String {
        match protect {
            ProtectionStatus::Unprotected => ProtectionEnabled::stock_str(self, from_id).await,
            ProtectionStatus::Protected => ProtectionDisabled::stock_str(self, from_id).await,
        }
        .to_string()
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
        msg.text = Some(DeviceMessagesHint::stock_str(self).await.to_string());
        chat::add_device_msg(&self, Some("core-about-device-chat"), Some(&mut msg)).await?;

        let image = include_bytes!("../assets/welcome-image.jpg");
        let blob = BlobObject::create(&self, "welcome-image.jpg".to_string(), image).await?;
        let mut msg = Message::new(Viewtype::Image);
        msg.param.set(Param::File, blob.as_name());
        chat::add_device_msg(&self, Some("core-welcome-image"), Some(&mut msg)).await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(WelcomeMessage::stock_str(self).await.to_string());
        chat::add_device_msg(&self, Some("core-welcome"), Some(&mut msg)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;

    use crate::constants::DC_CONTACT_ID_SELF;

    use crate::chat::Chat;
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
        t.set_stock_translation(StockMessage::NoMessages, "xyz".to_string())
            .await
            .unwrap();
        assert_eq!(NoMessages::stock_str(&t).await, "xyz")
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
        assert_eq!(NoMessages::stock_str(&t).await, "No messages.");
    }

    #[async_std::test]
    async fn test_stock_string_repl_str() {
        let t = TestContext::new().await;
        // uses %1$s substitution
        assert_eq!(ContactVerified::stock_str(&t, "Foo").await, "Foo verified.");
        // We have no string using %1$d to test...
    }

    #[async_std::test]
    async fn test_stock_string_repl_str2() {
        let t = TestContext::new().await;
        assert_eq!(
            ServerResponse::stock_str(&t, "foo", "bar").await,
            "Could not connect to foo: bar"
        );
    }

    #[async_std::test]
    async fn test_stock_system_msg_simple() {
        let t = TestContext::new().await;
        assert_eq!(
            MsgLocationEnabled::stock_str(&t).await,
            "Location streaming enabled."
        )
    }

    #[async_std::test]
    async fn test_stock_system_msg_add_member_by_me() {
        let t = TestContext::new().await;
        assert_eq!(
            MsgAddMember::stock_str(&t, "alice@example.com", DC_CONTACT_ID_SELF).await,
            "Member alice@example.com added by me."
        )
    }

    #[async_std::test]
    async fn test_stock_system_msg_add_member_by_me_with_displayname() {
        let t = TestContext::new().await;
        Contact::create(&t, "Alice", "alice@example.com")
            .await
            .expect("failed to create contact");
        assert_eq!(
            MsgAddMember::stock_str(&t, "alice@example.com", DC_CONTACT_ID_SELF).await,
            "Member Alice (alice@example.com) added by me."
        );
    }

    #[async_std::test]
    async fn test_stock_system_msg_add_member_by_other_with_displayname() {
        let t = TestContext::new().await;
        let contact_id = {
            Contact::create(&t, "Alice", "alice@example.com")
                .await
                .expect("Failed to create contact Alice");
            Contact::create(&t, "Bob", "bob@example.com")
                .await
                .expect("failed to create bob")
        };
        assert_eq!(
            MsgAddMember::stock_str(&t, "alice@example.com", contact_id,).await,
            "Member Alice (alice@example.com) added by Bob (bob@example.com)."
        );
    }

    #[async_std::test]
    async fn test_update_device_chats() {
        let t = TestContext::new().await;
        t.update_device_chats().await.ok();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 2);

        let chat0 = Chat::load_from_db(&t, chats.get_chat_id(0)).await.unwrap();
        let (self_talk_id, device_chat_id) = if chat0.is_self_talk() {
            (chats.get_chat_id(0), chats.get_chat_id(1))
        } else {
            (chats.get_chat_id(1), chats.get_chat_id(0))
        };

        // delete self-talk first; this adds a message to device-chat about how self-talk can be restored
        let device_chat_msgs_before = chat::get_chat_msgs(&t, device_chat_id, 0, None).await.len();
        self_talk_id.delete(&t).await.ok();
        assert_eq!(
            chat::get_chat_msgs(&t, device_chat_id, 0, None).await.len(),
            device_chat_msgs_before + 1
        );

        // delete device chat
        device_chat_id.delete(&t).await.ok();

        // check, that the chatlist is empty
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);

        // a subsequent call to update_device_chats() must not re-add manally deleted messages or chats
        t.update_device_chats().await.ok();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);
    }
}

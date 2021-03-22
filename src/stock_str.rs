//! Module to work with translatable stock strings

use std::future::Future;
use std::pin::Pin;

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

    #[strum(props(fallback = "Forwarded"))]
    Forwarded = 97,
}

impl StockMessage {
    /// Default untranslated strings for stock messages.
    ///
    /// These could be used in logging calls, so no logging here.
    fn fallback(self) -> &'static str {
        self.get_str("fallback").unwrap_or_default()
    }
}

async fn translated(context: &Context, id: StockMessage) -> String {
    context
        .translated_stockstrings
        .read()
        .await
        .get(&(id as usize))
        .map(AsRef::as_ref)
        .unwrap_or_else(|| id.fallback())
        .to_string()
}

/// Helper trait only meant to be implemented for [`String`].
trait StockStringMods: AsRef<str> + Sized {
    /// Substitutes the first replacement value if one is present.
    fn replace1(&self, replacement: impl AsRef<str>) -> String {
        self.as_ref()
            .replacen("%1$s", replacement.as_ref(), 1)
            .replacen("%1$d", replacement.as_ref(), 1)
            .replacen("%1$@", replacement.as_ref(), 1)
    }

    /// Substitutes the second replacement value if one is present.
    ///
    /// Be aware you probably should have also called [`StockStringMods::replace1`] if
    /// you are calling this.
    fn replace2(&self, replacement: impl AsRef<str>) -> String {
        self.as_ref()
            .replacen("%2$s", replacement.as_ref(), 1)
            .replacen("%2$d", replacement.as_ref(), 1)
            .replacen("%2$@", replacement.as_ref(), 1)
    }

    /// Augments the message by saying it was performed by a user.
    ///
    /// This looks up the display name of `contact` and uses the [`msg_action_by_me`] and
    /// [`msg_action_by_user`] stock strings to turn the stock string in one that says the
    /// action was performed by this user.
    ///
    /// E.g. this turns `Group image changed.` into `Group image changed by me.` or `Group
    /// image changed by Alice.`.
    ///
    /// Note that the original message should end in a `.`.
    fn action_by_contact<'a>(
        self,
        context: &'a Context,
        contact_id: u32,
    ) -> Pin<Box<dyn Future<Output = String> + Send + 'a>>
    where
        Self: Send + 'a,
    {
        Box::pin(async move {
            let message = self.as_ref().trim_end_matches('.');
            match contact_id {
                DC_CONTACT_ID_SELF => msg_action_by_me(context, message).await,
                _ => {
                    let displayname = Contact::get_by_id(context, contact_id)
                        .await
                        .map(|contact| contact.get_name_n_addr())
                        .unwrap_or_else(|_| contact_id.to_string());
                    msg_action_by_user(context, message, displayname).await
                }
            }
        })
    }
}

impl StockStringMods for String {}

/// Stock string: `No messages.`.
pub(crate) async fn no_messages(context: &Context) -> String {
    translated(context, StockMessage::NoMessages).await
}

/// Stock string: `Me`.
pub(crate) async fn self_msg(context: &Context) -> String {
    translated(context, StockMessage::SelfMsg).await
}

/// Stock string: `Draft`.
pub(crate) async fn draft(context: &Context) -> String {
    translated(context, StockMessage::Draft).await
}

/// Stock string: `Voice message`.
pub(crate) async fn voice_message(context: &Context) -> String {
    translated(context, StockMessage::VoiceMessage).await
}

/// Stock string: `Contact requests`.
pub(crate) async fn dead_drop(context: &Context) -> String {
    translated(context, StockMessage::DeadDrop).await
}

/// Stock string: `Image`.
pub(crate) async fn image(context: &Context) -> String {
    translated(context, StockMessage::Image).await
}

/// Stock string: `Video`.
pub(crate) async fn video(context: &Context) -> String {
    translated(context, StockMessage::Video).await
}

/// Stock string: `Audio`.
pub(crate) async fn audio(context: &Context) -> String {
    translated(context, StockMessage::Audio).await
}

/// Stock string: `File`.
pub(crate) async fn file(context: &Context) -> String {
    translated(context, StockMessage::File).await
}

/// Stock string: `Sent with my Delta Chat Messenger: https://delta.chat`.
pub(crate) async fn status_line(context: &Context) -> String {
    translated(context, StockMessage::StatusLine).await
}

/// Stock string: `Hello, I've just created the group "%1$s" for us.`.
pub(crate) async fn new_group_draft(context: &Context, group_name: impl AsRef<str>) -> String {
    translated(context, StockMessage::NewGroupDraft)
        .await
        .replace1(group_name)
}

/// Stock string: `Group name changed from "%1$s" to "%2$s".`.
pub(crate) async fn msg_grp_name(
    context: &Context,
    from_group: impl AsRef<str>,
    to_group: impl AsRef<str>,
    by_contact: u32,
) -> String {
    translated(context, StockMessage::MsgGrpName)
        .await
        .replace1(from_group)
        .replace2(to_group)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Group image changed.`.
pub(crate) async fn msg_grp_img_changed(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgGrpImgChanged)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Member %1$s added.`.
///
/// The `added_member_addr` parameter should be an email address and is looked up in the
/// contacts to combine with the display name.
pub(crate) async fn msg_add_member(
    context: &Context,
    added_member_addr: impl AsRef<str>,
    by_contact: u32,
) -> String {
    let addr = added_member_addr.as_ref();
    let who = match Contact::lookup_id_by_addr(context, addr, Origin::Unknown).await {
        Ok(Some(contact_id)) => Contact::get_by_id(context, contact_id)
            .await
            .map(|contact| contact.get_name_n_addr())
            .unwrap_or_else(|_| addr.to_string()),
        _ => addr.to_string(),
    };
    translated(context, StockMessage::MsgAddMember)
        .await
        .replace1(who)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Member %1$s removed.`.
///
/// The `removed_member_addr` parameter should be an email address and is looked up in
/// the contacts to combine with the display name.
pub(crate) async fn msg_del_member(
    context: &Context,
    removed_member_addr: impl AsRef<str>,
    by_contact: u32,
) -> String {
    let addr = removed_member_addr.as_ref();
    let who = match Contact::lookup_id_by_addr(context, addr, Origin::Unknown).await {
        Ok(Some(contact_id)) => Contact::get_by_id(context, contact_id)
            .await
            .map(|contact| contact.get_name_n_addr())
            .unwrap_or_else(|_| addr.to_string()),
        _ => addr.to_string(),
    };
    translated(context, StockMessage::MsgDelMember)
        .await
        .replace1(who)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Group left.`.
pub(crate) async fn msg_group_left(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgGroupLeft)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `GIF`.
pub(crate) async fn gif(context: &Context) -> String {
    translated(context, StockMessage::Gif).await
}

/// Stock string: `Encrypted message`.
pub(crate) async fn encrypted_msg(context: &Context) -> String {
    translated(context, StockMessage::EncryptedMsg).await
}

/// Stock string: `End-to-end encryption available.`.
pub(crate) async fn e2e_available(context: &Context) -> String {
    translated(context, StockMessage::E2eAvailable).await
}

/// Stock string: `No encryption.`.
pub(crate) async fn encr_none(context: &Context) -> String {
    translated(context, StockMessage::EncrNone).await
}

/// Stock string: `This message was encrypted for another setup.`.
pub(crate) async fn cant_decrypt_msg_body(context: &Context) -> String {
    translated(context, StockMessage::CantDecryptMsgBody).await
}

/// Stock string: `Fingerprints`.
pub(crate) async fn finger_prints(context: &Context) -> String {
    translated(context, StockMessage::FingerPrints).await
}

/// Stock string: `Return receipt`.
pub(crate) async fn read_rcpt(context: &Context) -> String {
    translated(context, StockMessage::ReadRcpt).await
}

/// Stock string: `This is a return receipt for the message "%1$s".`.
pub(crate) async fn read_rcpt_mail_body(context: &Context, message: impl AsRef<str>) -> String {
    translated(context, StockMessage::ReadRcptMailBody)
        .await
        .replace1(message)
}

/// Stock string: `Group image deleted.`.
pub(crate) async fn msg_grp_img_deleted(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgGrpImgDeleted)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `End-to-end encryption preferred.`.
pub(crate) async fn e2e_preferred(context: &Context) -> String {
    translated(context, StockMessage::E2ePreferred).await
}

/// Stock string: `%1$s verified.`.
pub(crate) async fn contact_verified(context: &Context, contact_addr: impl AsRef<str>) -> String {
    translated(context, StockMessage::ContactVerified)
        .await
        .replace1(contact_addr)
}

/// Stock string: `Cannot verify %1$s`.
pub(crate) async fn contact_not_verified(
    context: &Context,
    contact_addr: impl AsRef<str>,
) -> String {
    translated(context, StockMessage::ContactNotVerified)
        .await
        .replace1(contact_addr)
}

/// Stock string: `Changed setup for %1$s`.
pub(crate) async fn contact_setup_changed(
    context: &Context,
    contact_addr: impl AsRef<str>,
) -> String {
    translated(context, StockMessage::ContactSetupChanged)
        .await
        .replace1(contact_addr)
}

/// Stock string: `Archived chats`.
pub(crate) async fn archived_chats(context: &Context) -> String {
    translated(context, StockMessage::ArchivedChats).await
}

/// Stock string: `Autocrypt Setup Message`.
pub(crate) async fn ac_setup_msg_subject(context: &Context) -> String {
    translated(context, StockMessage::AcSetupMsgSubject).await
}

/// Stock string: `This is the Autocrypt Setup Message used to transfer...`.
pub(crate) async fn ac_setup_msg_body(context: &Context) -> String {
    translated(context, StockMessage::AcSetupMsgBody).await
}

/// Stock string: `Cannot login as \"%1$s\". Please check...`.
pub(crate) async fn cannot_login(context: &Context, user: impl AsRef<str>) -> String {
    translated(context, StockMessage::CannotLogin)
        .await
        .replace1(user)
}

/// Stock string: `Could not connect to %1$s: %2$s`.
pub(crate) async fn server_response(
    context: &Context,
    server: impl AsRef<str>,
    details: impl AsRef<str>,
) -> String {
    translated(context, StockMessage::ServerResponse)
        .await
        .replace1(server)
        .replace2(details)
}

/// Stock string: `%1$s by %2$s.`.
pub(crate) async fn msg_action_by_user(
    context: &Context,
    action: impl AsRef<str>,
    user: impl AsRef<str>,
) -> String {
    translated(context, StockMessage::MsgActionByUser)
        .await
        .replace1(action)
        .replace2(user)
}

/// Stock string: `%1$s by me.`.
pub(crate) async fn msg_action_by_me(context: &Context, action: impl AsRef<str>) -> String {
    translated(context, StockMessage::MsgActionByMe)
        .await
        .replace1(action)
}

/// Stock string: `Location streaming enabled.`.
pub(crate) async fn msg_location_enabled(context: &Context) -> String {
    translated(context, StockMessage::MsgLocationEnabled).await
}

/// Stock string: `Location streaming enabled by ...`.
pub(crate) async fn msg_location_enabled_by(context: &Context, contact: u32) -> String {
    translated(context, StockMessage::MsgLocationEnabled)
        .await
        .action_by_contact(context, contact)
        .await
}

/// Stock string: `Location streaming disabled.`.
pub(crate) async fn msg_location_disabled(context: &Context) -> String {
    translated(context, StockMessage::MsgLocationDisabled).await
}

/// Stock string: `Location`.
pub(crate) async fn location(context: &Context) -> String {
    translated(context, StockMessage::Location).await
}

/// Stock string: `Sticker`.
pub(crate) async fn sticker(context: &Context) -> String {
    translated(context, StockMessage::Sticker).await
}

/// Stock string: `Device messages`.
pub(crate) async fn device_messages(context: &Context) -> String {
    translated(context, StockMessage::DeviceMessages).await
}

/// Stock string: `Saved messages`.
pub(crate) async fn saved_messages(context: &Context) -> String {
    translated(context, StockMessage::SavedMessages).await
}

/// Stock string: `Messages in this chat are generated locally by...`.
pub(crate) async fn device_messages_hint(context: &Context) -> String {
    translated(context, StockMessage::DeviceMessagesHint).await
}

/// Stock string: `Welcome to Delta Chat! – ...`.
pub(crate) async fn welcome_message(context: &Context) -> String {
    translated(context, StockMessage::WelcomeMessage).await
}

/// Stock string: `Unknown sender for this chat. See 'info' for more details.`.
pub(crate) async fn unknown_sender_for_chat(context: &Context) -> String {
    translated(context, StockMessage::UnknownSenderForChat).await
}

/// Stock string: `Message from %1$s`.
// TODO: This can compute `self_name` itself instead of asking the caller to do this.
pub(crate) async fn subject_for_new_contact(
    context: &Context,
    self_name: impl AsRef<str>,
) -> String {
    translated(context, StockMessage::SubjectForNewContact)
        .await
        .replace1(self_name)
}

/// Stock string: `Failed to send message to %1$s.`.
pub(crate) async fn failed_sending_to(context: &Context, name: impl AsRef<str>) -> String {
    translated(context, StockMessage::FailedSendingTo)
        .await
        .replace1(name)
}

/// Stock string: `Message deletion timer is disabled.`.
pub(crate) async fn msg_ephemeral_timer_disabled(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgEphemeralTimerDisabled)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to %1$s s.`.
pub(crate) async fn msg_ephemeral_timer_enabled(
    context: &Context,
    timer: impl AsRef<str>,
    by_contact: u32,
) -> String {
    translated(context, StockMessage::MsgEphemeralTimerEnabled)
        .await
        .replace1(timer)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to 1 minute.`.
pub(crate) async fn msg_ephemeral_timer_minute(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgEphemeralTimerMinute)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to 1 hour.`.
pub(crate) async fn msg_ephemeral_timer_hour(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgEphemeralTimerHour)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to 1 day.`.
pub(crate) async fn msg_ephemeral_timer_day(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgEphemeralTimerDay)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to 1 week.`.
pub(crate) async fn msg_ephemeral_timer_week(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::MsgEphemeralTimerWeek)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Video chat invitation`.
pub(crate) async fn videochat_invitation(context: &Context) -> String {
    translated(context, StockMessage::VideochatInvitation).await
}

/// Stock string: `You are invited to a video chat, click %1$s to join.`.
pub(crate) async fn videochat_invite_msg_body(context: &Context, url: impl AsRef<str>) -> String {
    translated(context, StockMessage::VideochatInviteMsgBody)
        .await
        .replace1(url)
}

/// Stock string: `Error:\n\n“%1$s”`.
pub(crate) async fn configuration_failed(context: &Context, details: impl AsRef<str>) -> String {
    translated(context, StockMessage::ConfigurationFailed)
        .await
        .replace1(details)
}

/// Stock string: `⚠️ Date or time of your device seem to be inaccurate (%1$s)...`.
// TODO: This could compute now itself.
pub(crate) async fn bad_time_msg_body(context: &Context, now: impl AsRef<str>) -> String {
    translated(context, StockMessage::BadTimeMsgBody)
        .await
        .replace1(now)
}

/// Stock string: `⚠️ Your Delta Chat version might be outdated...`.
pub(crate) async fn update_reminder_msg_body(context: &Context) -> String {
    translated(context, StockMessage::UpdateReminderMsgBody).await
}

/// Stock string: `Could not find your mail server...`.
pub(crate) async fn error_no_network(context: &Context) -> String {
    translated(context, StockMessage::ErrorNoNetwork).await
}

/// Stock string: `Chat protection enabled.`.
pub(crate) async fn protection_enabled(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::ProtectionEnabled)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Chat protection disabled.`.
pub(crate) async fn protection_disabled(context: &Context, by_contact: u32) -> String {
    translated(context, StockMessage::ProtectionDisabled)
        .await
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Reply`.
pub(crate) async fn reply_noun(context: &Context) -> String {
    translated(context, StockMessage::ReplyNoun).await
}

/// Stock string: `You deleted the \"Saved messages\" chat...`.
pub(crate) async fn self_deleted_msg_body(context: &Context) -> String {
    translated(context, StockMessage::SelfDeletedMsgBody).await
}

/// Stock string: `⚠️ The "Delete messages from server" feature now also...`.
pub(crate) async fn delete_server_turned_off(context: &Context) -> String {
    translated(context, StockMessage::DeleteServerTurnedOff).await
}

/// Stock string: `Message deletion timer is set to %1$s minutes.`.
pub(crate) async fn msg_ephemeral_timer_minutes(
    context: &Context,
    minutes: impl AsRef<str>,
    by_contact: u32,
) -> String {
    translated(context, StockMessage::MsgEphemeralTimerMinutes)
        .await
        .replace1(minutes)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to %1$s hours.`.
pub(crate) async fn msg_ephemeral_timer_hours(
    context: &Context,
    hours: impl AsRef<str>,
    by_contact: u32,
) -> String {
    translated(context, StockMessage::MsgEphemeralTimerHours)
        .await
        .replace1(hours)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to %1$s days.`.
pub(crate) async fn msg_ephemeral_timer_days(
    context: &Context,
    days: impl AsRef<str>,
    by_contact: u32,
) -> String {
    translated(context, StockMessage::MsgEphemeralTimerDays)
        .await
        .replace1(days)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Message deletion timer is set to %1$s weeks.`.
pub(crate) async fn msg_ephemeral_timer_weeks(
    context: &Context,
    weeks: impl AsRef<str>,
    by_contact: u32,
) -> String {
    translated(context, StockMessage::MsgEphemeralTimerWeeks)
        .await
        .replace1(weeks)
        .action_by_contact(context, by_contact)
        .await
}

/// Stock string: `Forwarded`.
pub(crate) async fn forwarded(context: &Context) -> String {
    translated(context, StockMessage::Forwarded).await
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
            ProtectionStatus::Unprotected => protection_enabled(self, from_id).await,
            ProtectionStatus::Protected => protection_disabled(self, from_id).await,
        }
    }

    pub(crate) async fn update_device_chats(&self) -> Result<(), Error> {
        if self.get_config_bool(Config::Bot).await? {
            return Ok(());
        }

        // create saved-messages chat; we do this only once, if the user has deleted the chat,
        // he can recreate it manually (make sure we do not re-add it when configure() was called a second time)
        if !self.sql.get_raw_config_bool("self-chat-added").await? {
            self.sql
                .set_raw_config_bool("self-chat-added", true)
                .await?;
            chat::create_by_contact_id(self, DC_CONTACT_ID_SELF).await?;
        }

        // add welcome-messages. by the label, this is done only once,
        // if the user has deleted the message or the chat, it is not added again.
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(device_messages_hint(self).await);
        chat::add_device_msg(self, Some("core-about-device-chat"), Some(&mut msg)).await?;

        let image = include_bytes!("../assets/welcome-image.jpg");
        let blob = BlobObject::create(self, "welcome-image.jpg".to_string(), image).await?;
        let mut msg = Message::new(Viewtype::Image);
        msg.param.set(Param::File, blob.as_name());
        chat::add_device_msg(self, Some("core-welcome-image"), Some(&mut msg)).await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(welcome_message(self).await);
        chat::add_device_msg(self, Some("core-welcome"), Some(&mut msg)).await?;
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
        assert_eq!(no_messages(&t).await, "xyz")
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
        assert_eq!(no_messages(&t).await, "No messages.");
    }

    #[async_std::test]
    async fn test_stock_string_repl_str() {
        let t = TestContext::new().await;
        // uses %1$s substitution
        assert_eq!(contact_verified(&t, "Foo").await, "Foo verified.");
        // We have no string using %1$d to test...
    }

    #[async_std::test]
    async fn test_stock_string_repl_str2() {
        let t = TestContext::new().await;
        assert_eq!(
            server_response(&t, "foo", "bar").await,
            "Could not connect to foo: bar"
        );
    }

    #[async_std::test]
    async fn test_stock_system_msg_simple() {
        let t = TestContext::new().await;
        assert_eq!(
            msg_location_enabled(&t).await,
            "Location streaming enabled."
        )
    }

    #[async_std::test]
    async fn test_stock_system_msg_add_member_by_me() {
        let t = TestContext::new().await;
        assert_eq!(
            msg_add_member(&t, "alice@example.com", DC_CONTACT_ID_SELF).await,
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
            msg_add_member(&t, "alice@example.com", DC_CONTACT_ID_SELF).await,
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
            msg_add_member(&t, "alice@example.com", contact_id,).await,
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
        let device_chat_msgs_before = chat::get_chat_msgs(&t, device_chat_id, 0, None)
            .await
            .unwrap()
            .len();
        self_talk_id.delete(&t).await.ok();
        assert_eq!(
            chat::get_chat_msgs(&t, device_chat_id, 0, None)
                .await
                .unwrap()
                .len(),
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

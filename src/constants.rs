//! # Constants
use deltachat_derive::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static DC_VERSION_STR: Lazy<String> = Lazy::new(|| env!("CARGO_PKG_VERSION").to_string());

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum Blocked {
    Not = 0,
    Manually = 1,
    Deaddrop = 2,
}

impl Default for Blocked {
    fn default() -> Self {
        Blocked::Not
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(u8)]
pub enum ShowEmails {
    Off = 0,
    AcceptedContacts = 1,
    All = 2,
}

impl Default for ShowEmails {
    fn default() -> Self {
        ShowEmails::Off // also change Config.ShowEmails props(default) on changes
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(u8)]
pub enum MediaQuality {
    Balanced = 0,
    Worse = 1,
}

impl Default for MediaQuality {
    fn default() -> Self {
        MediaQuality::Balanced // also change Config.MediaQuality props(default) on changes
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(u8)]
pub enum KeyGenType {
    Default = 0,
    Rsa2048 = 1,
    Ed25519 = 2,
}

impl Default for KeyGenType {
    fn default() -> Self {
        KeyGenType::Default
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(i8)]
pub enum VideochatType {
    Unknown = 0,
    BasicWebrtc = 1,
    Jitsi = 2,
}

impl Default for VideochatType {
    fn default() -> Self {
        VideochatType::Unknown
    }
}

pub const DC_HANDSHAKE_CONTINUE_NORMAL_PROCESSING: i32 = 0x01;
pub const DC_HANDSHAKE_STOP_NORMAL_PROCESSING: i32 = 0x02;
pub const DC_HANDSHAKE_ADD_DELETE_JOB: i32 = 0x04;

pub(crate) const DC_FROM_HANDSHAKE: i32 = 0x01;

pub const DC_GCL_ARCHIVED_ONLY: usize = 0x01;
pub const DC_GCL_NO_SPECIALS: usize = 0x02;
pub const DC_GCL_ADD_ALLDONE_HINT: usize = 0x04;
pub const DC_GCL_FOR_FORWARDING: usize = 0x08;

pub const DC_GCM_ADDDAYMARKER: u32 = 0x01;

pub const DC_GCL_VERIFIED_ONLY: usize = 0x01;
pub const DC_GCL_ADD_SELF: usize = 0x02;

// unchanged user avatars are resent to the recipients every some days
pub const DC_RESEND_USER_AVATAR_DAYS: i64 = 14;

// warn about an outdated app after a given number of days.
// as we use the "provider-db generation date" as reference (that might not be updated very often)
// and as not all system get speedy updates,
// do not use too small value that will annoy users checking for nonexistant updates.
pub const DC_OUTDATED_WARNING_DAYS: i64 = 365;

/// virtual chat showing all messages belonging to chats flagged with chats.blocked=2
pub const DC_CHAT_ID_DEADDROP: u32 = 1;
/// messages that should be deleted get this chat_id; the messages are deleted from the working thread later then. This is also needed as rfc724_mid should be preset as long as the message is not deleted on the server (otherwise it is downloaded again)
pub const DC_CHAT_ID_TRASH: u32 = 3;
/// only an indicator in a chatlist
pub const DC_CHAT_ID_ARCHIVED_LINK: u32 = 6;
/// only an indicator in a chatlist
pub const DC_CHAT_ID_ALLDONE_HINT: u32 = 7;
/// larger chat IDs are "real" chats, their messages are "real" messages.
pub const DC_CHAT_ID_LAST_SPECIAL: u32 = 9;

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
    IntoStaticStr,
    Serialize,
    Deserialize,
)]
#[repr(u32)]
pub enum Chattype {
    Undefined = 0,
    Single = 100,
    Group = 120,
}

impl Default for Chattype {
    fn default() -> Self {
        Chattype::Undefined
    }
}

pub const DC_MSG_ID_MARKER1: u32 = 1;
pub const DC_MSG_ID_DAYMARKER: u32 = 9;
pub const DC_MSG_ID_LAST_SPECIAL: u32 = 9;

/// approx. max. length returned by dc_msg_get_text()
pub const DC_MAX_GET_TEXT_LEN: usize = 30000;
/// approx. max. length returned by dc_get_msg_info()
pub const DC_MAX_GET_INFO_LEN: usize = 100_000;

pub const DC_CONTACT_ID_UNDEFINED: u32 = 0;
pub const DC_CONTACT_ID_SELF: u32 = 1;
pub const DC_CONTACT_ID_INFO: u32 = 2;
pub const DC_CONTACT_ID_DEVICE: u32 = 5;
pub const DC_CONTACT_ID_LAST_SPECIAL: u32 = 9;

// decorative address that is used for DC_CONTACT_ID_DEVICE
// when an api that returns an email is called.
pub const DC_CONTACT_ID_DEVICE_ADDR: &str = "device@localhost";

// Flags for empty server job

pub const DC_EMPTY_MVBOX: u32 = 0x01;
pub const DC_EMPTY_INBOX: u32 = 0x02;

// Flags for configuring IMAP and SMTP servers.
// These flags are optional
// and may be set together with the username, password etc.
// via dc_set_config() using the key "server_flags".

/// Force OAuth2 authorization. This flag does not skip automatic configuration.
/// Before calling configure() with DC_LP_AUTH_OAUTH2 set,
/// the user has to confirm access at the URL returned by dc_get_oauth2_url().
pub const DC_LP_AUTH_OAUTH2: i32 = 0x2;

/// Force NORMAL authorization, this is the default.
/// If this flag is set, automatic configuration is skipped.
pub const DC_LP_AUTH_NORMAL: i32 = 0x4;

/// if none of these flags are set, the default is chosen
pub const DC_LP_AUTH_FLAGS: i32 = DC_LP_AUTH_OAUTH2 | DC_LP_AUTH_NORMAL;

/// How many existing messages shall be fetched after configuration.
pub const DC_FETCH_EXISTING_MSGS_COUNT: i64 = 100;

// max. width/height of an avatar
pub const AVATAR_SIZE: u32 = 192;

// max. width/height of images
pub const BALANCED_IMAGE_SIZE: u32 = 1280;
pub const WORSE_IMAGE_SIZE: u32 = 640;

// this value can be increased if the folder configuration is changed and must be redone on next program start
pub const DC_FOLDERS_CONFIGURED_VERSION: i32 = 3;

// if more recipients are needed in SMTP's `RCPT TO:` header, recipient-list is splitted to chunks.
// this does not affect MIME'e `To:` header.
// can be overwritten by the setting `max_smtp_rcpt_to` in provider-db.
pub const DEFAULT_MAX_SMTP_RCPT_TO: usize = 50;

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
    Serialize,
    Deserialize,
)]
#[repr(i32)]
pub enum Viewtype {
    Unknown = 0,

    /// Text message.
    /// The text of the message is set using dc_msg_set_text()
    /// and retrieved with dc_msg_get_text().
    Text = 10,

    /// Image message.
    /// If the image is an animated GIF, the type DC_MSG_GIF should be used.
    /// File, width and height are set via dc_msg_set_file(), dc_msg_set_dimension
    /// and retrieved via dc_msg_set_file(), dc_msg_set_dimension().
    Image = 20,

    /// Animated GIF message.
    /// File, width and height are set via dc_msg_set_file(), dc_msg_set_dimension()
    /// and retrieved via dc_msg_get_file(), dc_msg_get_width(), dc_msg_get_height().
    Gif = 21,

    /// Message containing a sticker, similar to image.
    /// If possible, the ui should display the image without borders in a transparent way.
    /// A click on a sticker will offer to install the sticker set in some future.
    Sticker = 23,

    /// Message containing an Audio file.
    /// File and duration are set via dc_msg_set_file(), dc_msg_set_duration()
    /// and retrieved via dc_msg_get_file(), dc_msg_get_duration().
    Audio = 40,

    /// A voice message that was directly recorded by the user.
    /// For all other audio messages, the type #DC_MSG_AUDIO should be used.
    /// File and duration are set via dc_msg_set_file(), dc_msg_set_duration()
    /// and retrieved via dc_msg_get_file(), dc_msg_get_duration()
    Voice = 41,

    /// Video messages.
    /// File, width, height and durarion
    /// are set via dc_msg_set_file(), dc_msg_set_dimension(), dc_msg_set_duration()
    /// and retrieved via
    /// dc_msg_get_file(), dc_msg_get_width(),
    /// dc_msg_get_height(), dc_msg_get_duration().
    Video = 50,

    /// Message containing any file, eg. a PDF.
    /// The file is set via dc_msg_set_file()
    /// and retrieved via dc_msg_get_file().
    File = 60,

    /// Message is an invitation to a videochat.
    VideochatInvitation = 70,
}

impl Default for Viewtype {
    fn default() -> Self {
        Viewtype::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_display_works_as_expected() {
        assert_eq!(format!("{}", Viewtype::Audio), "Audio");
    }
}

pub const DC_JOB_DELETE_MSG_ON_IMAP: i32 = 110;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum KeyType {
    Public = 0,
    Private = 1,
}

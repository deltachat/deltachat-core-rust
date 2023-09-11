//! # Constants.

#![allow(missing_docs)]

use deltachat_derive::{FromSql, ToSql};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::chat::ChatId;

pub static DC_VERSION_STR: Lazy<String> = Lazy::new(|| env!("CARGO_PKG_VERSION").to_string());

#[derive(
    Debug,
    Default,
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
#[repr(i8)]
pub enum Blocked {
    #[default]
    Not = 0,
    Yes = 1,
    Request = 2,
}

#[derive(
    Debug, Default, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql,
)]
#[repr(u8)]
pub enum ShowEmails {
    Off = 0,
    AcceptedContacts = 1,
    #[default] // also change Config.ShowEmails props(default) on changes
    All = 2,
}

#[derive(
    Debug, Default, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql,
)]
#[repr(u8)]
pub enum MediaQuality {
    #[default] // also change Config.MediaQuality props(default) on changes
    Balanced = 0,
    Worse = 1,
}

/// Type of the key to generate.
#[derive(
    Debug, Default, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql,
)]
#[repr(u8)]
pub enum KeyGenType {
    #[default]
    Default = 0,

    /// 2048-bit RSA.
    Rsa2048 = 1,

    /// [Ed25519](https://ed25519.cr.yp.to/) signature and X25519 encryption.
    Ed25519 = 2,

    /// 4096-bit RSA.
    Rsa4096 = 3,
}

/// Video chat URL type.
#[derive(
    Debug, Default, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql,
)]
#[repr(i8)]
pub enum VideochatType {
    /// Unknown type.
    #[default]
    Unknown = 0,

    /// [basicWebRTC](https://github.com/cracker0dks/basicwebrtc) instance.
    BasicWebrtc = 1,

    /// [Jitsi Meet](https://jitsi.org/jitsi-meet/) instance.
    Jitsi = 2,
}

pub const DC_HANDSHAKE_CONTINUE_NORMAL_PROCESSING: i32 = 0x01;
pub const DC_HANDSHAKE_STOP_NORMAL_PROCESSING: i32 = 0x02;
pub const DC_HANDSHAKE_ADD_DELETE_JOB: i32 = 0x04;

pub(crate) const DC_FROM_HANDSHAKE: i32 = 0x01;

pub const DC_GCL_ARCHIVED_ONLY: usize = 0x01;
pub const DC_GCL_NO_SPECIALS: usize = 0x02;
pub const DC_GCL_ADD_ALLDONE_HINT: usize = 0x04;
pub const DC_GCL_FOR_FORWARDING: usize = 0x08;

pub const DC_GCL_VERIFIED_ONLY: u32 = 0x01;
pub const DC_GCL_ADD_SELF: u32 = 0x02;

// unchanged user avatars are resent to the recipients every some days
pub(crate) const DC_RESEND_USER_AVATAR_DAYS: i64 = 14;

// warn about an outdated app after a given number of days.
// as we use the "provider-db generation date" as reference (that might not be updated very often)
// and as not all system get speedy updates,
// do not use too small value that will annoy users checking for nonexistent updates.
pub(crate) const DC_OUTDATED_WARNING_DAYS: i64 = 365;

/// messages that should be deleted get this chat_id; the messages are deleted from the working thread later then. This is also needed as rfc724_mid should be preset as long as the message is not deleted on the server (otherwise it is downloaded again)
pub const DC_CHAT_ID_TRASH: ChatId = ChatId::new(3);
/// only an indicator in a chatlist
pub const DC_CHAT_ID_ARCHIVED_LINK: ChatId = ChatId::new(6);
/// only an indicator in a chatlist
pub const DC_CHAT_ID_ALLDONE_HINT: ChatId = ChatId::new(7);
/// larger chat IDs are "real" chats, their messages are "real" messages.
pub const DC_CHAT_ID_LAST_SPECIAL: ChatId = ChatId::new(9);

/// Chat type.
#[derive(
    Debug,
    Default,
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
    /// Undefined chat type.
    #[default]
    Undefined = 0,

    /// 1:1 chat.
    Single = 100,

    /// Group chat.
    Group = 120,

    /// Mailing list.
    Mailinglist = 140,

    /// Broadcast list.
    Broadcast = 160,
}

pub const DC_MSG_ID_DAYMARKER: u32 = 9;
pub const DC_MSG_ID_LAST_SPECIAL: u32 = 9;

/// String that indicates that something is left out or truncated.
pub(crate) const DC_ELLIPSIS: &str = "[...]";
// how many lines desktop can display when fullscreen (fullscreen at zoomlevel 1x)
// (taken from "subjective" testing what looks ok)
pub const DC_DESIRED_TEXT_LINES: usize = 38;
// how many chars desktop can display per line (from "subjective" testing)
pub const DC_DESIRED_TEXT_LINE_LEN: usize = 100;

/// Message length limit.
///
/// To keep bubbles and chat flow usable and to avoid problems with controls using very long texts,
/// we limit the text length to `DC_DESIRED_TEXT_LEN`.  If the text is longer, the full text can be
/// retrieved using has_html()/get_html().
///
/// Note that for simplicity maximum length is defined as the number of Unicode Scalar Values (Rust
/// `char`s), not Unicode Grapheme Clusters.
pub const DC_DESIRED_TEXT_LEN: usize = DC_DESIRED_TEXT_LINE_LEN * DC_DESIRED_TEXT_LINES;

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
pub(crate) const DC_FETCH_EXISTING_MSGS_COUNT: i64 = 100;

// max. weight of images to send w/o recoding
pub const BALANCED_IMAGE_BYTES: usize = 500_000;
pub const WORSE_IMAGE_BYTES: usize = 130_000;

// max. width/height of an avatar
pub(crate) const BALANCED_AVATAR_SIZE: u32 = 256;
pub(crate) const WORSE_AVATAR_SIZE: u32 = 128;

// max. width/height of images scaled down because of being too huge
pub const BALANCED_IMAGE_SIZE: u32 = 1280;
pub const WORSE_IMAGE_SIZE: u32 = 640;

// this value can be increased if the folder configuration is changed and must be redone on next program start
pub(crate) const DC_FOLDERS_CONFIGURED_VERSION: i32 = 4;

#[cfg(test)]
mod tests {
    use num_traits::FromPrimitive;

    use super::*;

    #[test]
    fn test_chattype_values() {
        // values may be written to disk and must not change
        assert_eq!(Chattype::Undefined, Chattype::default());
        assert_eq!(Chattype::Undefined, Chattype::from_i32(0).unwrap());
        assert_eq!(Chattype::Single, Chattype::from_i32(100).unwrap());
        assert_eq!(Chattype::Group, Chattype::from_i32(120).unwrap());
        assert_eq!(Chattype::Mailinglist, Chattype::from_i32(140).unwrap());
        assert_eq!(Chattype::Broadcast, Chattype::from_i32(160).unwrap());
    }

    #[test]
    fn test_keygentype_values() {
        // values may be written to disk and must not change
        assert_eq!(KeyGenType::Default, KeyGenType::default());
        assert_eq!(KeyGenType::Default, KeyGenType::from_i32(0).unwrap());
        assert_eq!(KeyGenType::Rsa2048, KeyGenType::from_i32(1).unwrap());
        assert_eq!(KeyGenType::Ed25519, KeyGenType::from_i32(2).unwrap());
        assert_eq!(KeyGenType::Rsa4096, KeyGenType::from_i32(3).unwrap());
    }

    #[test]
    fn test_showemails_values() {
        // values may be written to disk and must not change
        assert_eq!(ShowEmails::All, ShowEmails::default());
        assert_eq!(ShowEmails::Off, ShowEmails::from_i32(0).unwrap());
        assert_eq!(
            ShowEmails::AcceptedContacts,
            ShowEmails::from_i32(1).unwrap()
        );
        assert_eq!(ShowEmails::All, ShowEmails::from_i32(2).unwrap());
    }

    #[test]
    fn test_blocked_values() {
        // values may be written to disk and must not change
        assert_eq!(Blocked::Not, Blocked::default());
        assert_eq!(Blocked::Not, Blocked::from_i32(0).unwrap());
        assert_eq!(Blocked::Yes, Blocked::from_i32(1).unwrap());
        assert_eq!(Blocked::Request, Blocked::from_i32(2).unwrap());
    }

    #[test]
    fn test_mediaquality_values() {
        // values may be written to disk and must not change
        assert_eq!(MediaQuality::Balanced, MediaQuality::default());
        assert_eq!(MediaQuality::Balanced, MediaQuality::from_i32(0).unwrap());
        assert_eq!(MediaQuality::Worse, MediaQuality::from_i32(1).unwrap());
    }

    #[test]
    fn test_videochattype_values() {
        // values may be written to disk and must not change
        assert_eq!(VideochatType::Unknown, VideochatType::default());
        assert_eq!(VideochatType::Unknown, VideochatType::from_i32(0).unwrap());
        assert_eq!(
            VideochatType::BasicWebrtc,
            VideochatType::from_i32(1).unwrap()
        );
        assert_eq!(VideochatType::Jitsi, VideochatType::from_i32(2).unwrap());
    }
}

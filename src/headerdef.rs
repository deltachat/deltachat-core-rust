//! # List of email headers.

use mailparse::{MailHeader, MailHeaderMap};

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumVariantNames, IntoStaticStr)]
#[strum(serialize_all = "kebab_case")]
pub enum HeaderDef {
    MessageId,
    Subject,
    Date,
    From_,
    To,
    Cc,
    Disposition,

    /// Used in the "Body Part Header" of MDNs as of RFC 8098.
    /// Indicates the Message-ID of the message for which the MDN is being issued.
    OriginalMessageId,

    /// Delta Chat extension for message IDs in combined MDNs
    AdditionalMessageIds,

    /// Outlook-SMTP-server replace the `Message-ID:`-header
    /// and write the original ID to `X-Microsoft-Original-Message-ID`.
    /// To sort things correctly and to not show outgoing messages twice,
    /// we need to check that header as well.
    XMicrosoftOriginalMessageId,

    /// Thunderbird header used to store Draft information.
    ///
    /// Thunderbird 78.11.0 does not set \Draft flag on messages saved as "Template", but sets this
    /// header, so it can be used to ignore such messages.
    XMozillaDraftInfo,

    ListId,
    References,
    InReplyTo,
    Precedence,
    ContentType,
    ContentId,
    ChatVersion,
    ChatGroupId,
    ChatGroupName,
    ChatGroupNameChanged,
    ChatVerified,
    ChatGroupAvatar,
    ChatUserAvatar,
    ChatVoiceMessage,
    ChatGroupMemberRemoved,
    ChatGroupMemberAdded,
    ChatContent,
    ChatDuration,
    ChatDispositionNotificationTo,
    ChatWebrtcRoom,
    Autocrypt,
    AutocryptSetupMessage,
    SecureJoin,
    SecureJoinGroup,
    SecureJoinFingerprint,
    SecureJoinInvitenumber,
    SecureJoinAuth,
    Sender,
    EphemeralTimer,
    Received,
    _TestHeader,
}

impl HeaderDef {
    /// Returns the corresponding header string.
    pub fn get_headername(&self) -> &'static str {
        self.into()
    }
}

pub trait HeaderDefMap {
    fn get_header_value(&self, headerdef: HeaderDef) -> Option<String>;
    fn get_header(&self, headerdef: HeaderDef) -> Option<&MailHeader>;
}

impl HeaderDefMap for [MailHeader<'_>] {
    fn get_header_value(&self, headerdef: HeaderDef) -> Option<String> {
        self.get_first_value(headerdef.get_headername())
    }
    fn get_header(&self, headerdef: HeaderDef) -> Option<&MailHeader> {
        self.get_first_header(headerdef.get_headername())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that kebab_case serialization works as expected
    fn kebab_test() {
        assert_eq!(HeaderDef::From_.get_headername(), "from");

        assert_eq!(HeaderDef::_TestHeader.get_headername(), "test-header");
    }

    #[test]
    /// Test that headers are parsed case-insensitively
    fn test_get_header_value_case() {
        let (headers, _) =
            mailparse::parse_headers(b"fRoM: Bob\naUtoCryPt-SeTup-MessAge: v99").unwrap();
        assert_eq!(
            headers.get_header_value(HeaderDef::AutocryptSetupMessage),
            Some("v99".to_string())
        );
        assert_eq!(
            headers.get_header_value(HeaderDef::From_),
            Some("Bob".to_string())
        );
        assert_eq!(headers.get_header_value(HeaderDef::Autocrypt), None);
    }
}

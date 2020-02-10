use crate::strum::AsStaticRef;
use mailparse::{MailHeader, MailHeaderMap, MailParseError};

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumVariantNames, AsStaticStr)]
#[strum(serialize_all = "kebab_case")]
#[allow(dead_code)]
pub enum HeaderDef {
    MessageId,
    Subject,
    Date,
    From_,
    To,
    Cc,
    Disposition,
    OriginalMessageId,

    /// Delta Chat extension for message IDs in combined MDNs
    AdditionalMessageIds,

    ListId,
    References,
    InReplyTo,
    Precedence,
    ChatVersion,
    ChatGroupId,
    ChatGroupName,
    ChatGroupNameChanged,
    ChatVerified,
    ChatGroupImage, // deprecated
    ChatGroupAvatar,
    ChatUserAvatar,
    ChatVoiceMessage,
    ChatGroupMemberRemoved,
    ChatGroupMemberAdded,
    ChatContent,
    ChatDuration,
    ChatDispositionNotificationTo,
    Autocrypt,
    AutocryptSetupMessage,
    SecureJoin,
    SecureJoinGroup,
    SecureJoinFingerprint,
    SecureJoinInvitenumber,
    SecureJoinAuth,
    _TestHeader,
}

impl HeaderDef {
    /// Returns the corresponding Event id.
    pub fn get_headername(&self) -> &'static str {
        self.as_static()
    }
}

pub trait HeaderDefMap {
    fn get_headerdef(&self, headerdef: HeaderDef) -> Result<Option<String>, MailParseError>;
}

impl HeaderDefMap for [MailHeader<'_>] {
    fn get_headerdef(&self, headerdef: HeaderDef) -> Result<Option<String>, MailParseError> {
        self.get_first_value(headerdef.get_headername())
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
    fn headerdef_case() {
        let (headers, _) =
            mailparse::parse_headers(b"fRoM: Bob\naUtoCryPt-SeTup-MessAge: v99").unwrap();
        assert_eq!(
            headers
                .get_headerdef(HeaderDef::AutocryptSetupMessage)
                .unwrap(),
            Some("v99".to_string())
        );
        assert_eq!(
            headers.get_headerdef(HeaderDef::From_).unwrap(),
            Some("Bob".to_string())
        );
        assert_eq!(headers.get_headerdef(HeaderDef::Autocrypt).unwrap(), None);
    }
}

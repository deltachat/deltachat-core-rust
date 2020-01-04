use crate::strum::AsStaticRef;
use mailparse::{MailHeader, MailHeaderMap};

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
    ContentType,
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
    Autocrypt,
    AutocryptSetupMessage,
    SecureJoin,
    SecureJoinGroup,
    SecureJoinFingerprint,
    SecureJoinInvitenumber,
    SecureJoinAuth,
    AutodeleteTimer,
    _TestHeader,
}

impl HeaderDef {
    /// Returns the corresponding Event id.
    pub fn get_headername(&self) -> &'static str {
        self.as_static()
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

use crate::strum::AsStaticRef;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that kebab_case serialization works as expected
    fn kebab_test() {
        assert_eq!(HeaderDef::From_.get_headername(), "from");

        assert_eq!(HeaderDef::_TestHeader.get_headername(), "test-header");
    }
}

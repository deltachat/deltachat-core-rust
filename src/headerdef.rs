#[derive(Debug, Display, Clone, PartialEq, Eq, EnumVariantNames)]
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
    pub fn get_headername(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that kebab_case serialization works as expected
    fn kebab_test() {
        assert_eq!(HeaderDef::From_.to_string(), "from");

        assert_eq!(HeaderDef::_TestHeader.to_string(), "test-header");
    }
}

use crate::strum::EnumProperty;

#[derive(Debug, Clone, PartialEq, Eq, EnumProperty)]
pub enum HeaderDef {
    #[strum(props(header = "message-id"))]
    MessageId,

    #[strum(props(header = "subject"))]
    Subject,

    #[strum(props(header = "date"))]
    Date,

    #[strum(props(header = "from"))]
    From_,

    #[strum(props(header = "to"))]
    To,

    #[strum(props(header = "cc"))]
    Cc,

    #[strum(props(header = "disposition"))]
    Disposition,

    #[strum(props(header = "original-message-id"))]
    OriginalMessageId,

    #[strum(props(header = "list-id"))]
    ListId,

    #[strum(props(header = "references"))]
    References,

    #[strum(props(header = "in-reply-to"))]
    InReplyTo,

    #[strum(props(header = "precedence"))]
    Precedence,

    #[strum(props(header = "chat-version"))]
    ChatVersion,

    #[strum(props(header = "chat-group-id"))]
    ChatGroupId,

    #[strum(props(header = "chat-group-name"))]
    ChatGroupName,

    #[strum(props(header = "chat-group-name-changed"))]
    ChatGroupNameChanged,

    #[strum(props(header = "chat-verified"))]
    ChatVerified,

    #[strum(props(header = "chat-group-image"))]
    ChatGroupImage,

    #[strum(props(header = "chat-voice-message"))]
    ChatVoiceMessage,

    #[strum(props(header = "chat-group-member-removed"))]
    ChatGroupMemberRemoved,

    #[strum(props(header = "chat-group-member-added"))]
    ChatGroupMemberAdded,

    #[strum(props(header = "chat-content"))]
    ChatContent,

    #[strum(props(header = "chat-duration"))]
    ChatDuration,

    #[strum(props(header = "chat-disposition-notification-to"))]
    ChatDispositionNotificationTo,

    #[strum(props(header = "autocrypt-setup-message"))]
    AutocryptSetupMessage,

    #[strum(props(header = "secure-join"))]
    SecureJoin,

    #[strum(props(header = "secure-join-group"))]
    SecureJoinGroup,

    #[strum(props(header = "secure-join-fingerprint"))]
    SecureJoinFingerprint,

    #[strum(props(header = "secure-join-invitenumber"))]
    SecureJoinInvitenumber,

    #[strum(props(header = "secure-join-group"))]
    SecureJoinAuth,

    #[strum(props(header = "test-header"))]
    _TestHeader,
}

impl HeaderDef {
    /// Returns the corresponding Event id.
    pub fn get_headername(&self) -> &str {
        self.get_str("header").expect("missing header definition")
    }
}

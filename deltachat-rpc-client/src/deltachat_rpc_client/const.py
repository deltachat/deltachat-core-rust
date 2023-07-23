from enum import Enum, IntEnum

COMMAND_PREFIX = "/"


class ContactFlag(IntEnum):
    VERIFIED_ONLY = 0x01
    ADD_SELF = 0x02


class ChatlistFlag(IntEnum):
    ARCHIVED_ONLY = 0x01
    NO_SPECIALS = 0x02
    ADD_ALLDONE_HINT = 0x04
    FOR_FORWARDING = 0x08


class SpecialContactId(IntEnum):
    SELF = 1
    INFO = 2  # centered messages as "member added", used in all chats
    DEVICE = 5  #  messages "update info" in the device-chat
    LAST_SPECIAL = 9


class EventType(str, Enum):
    """Core event types"""

    INFO = "Info"
    SMTP_CONNECTED = "SmtpConnected"
    IMAP_CONNECTED = "ImapConnected"
    SMTP_MESSAGE_SENT = "SmtpMessageSent"
    IMAP_MESSAGE_DELETED = "ImapMessageDeleted"
    IMAP_MESSAGE_MOVED = "ImapMessageMoved"
    IMAP_INBOX_IDLE = "ImapInboxIdle"
    NEW_BLOB_FILE = "NewBlobFile"
    DELETED_BLOB_FILE = "DeletedBlobFile"
    WARNING = "Warning"
    ERROR = "Error"
    ERROR_SELF_NOT_IN_GROUP = "ErrorSelfNotInGroup"
    MSGS_CHANGED = "MsgsChanged"
    REACTIONS_CHANGED = "ReactionsChanged"
    INCOMING_MSG = "IncomingMsg"
    INCOMING_MSG_BUNCH = "IncomingMsgBunch"
    MSGS_NOTICED = "MsgsNoticed"
    MSG_DELIVERED = "MsgDelivered"
    MSG_FAILED = "MsgFailed"
    MSG_READ = "MsgRead"
    MSG_DELETED = "MsgDeleted"
    CHAT_MODIFIED = "ChatModified"
    CHAT_EPHEMERAL_TIMER_MODIFIED = "ChatEphemeralTimerModified"
    CONTACTS_CHANGED = "ContactsChanged"
    LOCATION_CHANGED = "LocationChanged"
    CONFIGURE_PROGRESS = "ConfigureProgress"
    IMEX_PROGRESS = "ImexProgress"
    IMEX_FILE_WRITTEN = "ImexFileWritten"
    SECUREJOIN_INVITER_PROGRESS = "SecurejoinInviterProgress"
    SECUREJOIN_JOINER_PROGRESS = "SecurejoinJoinerProgress"
    CONNECTIVITY_CHANGED = "ConnectivityChanged"
    SELFAVATAR_CHANGED = "SelfavatarChanged"
    WEBXDC_STATUS_UPDATE = "WebxdcStatusUpdate"
    WEBXDC_INSTANCE_DELETED = "WebxdcInstanceDeleted"


class ChatType(IntEnum):
    """Chat types"""

    UNDEFINED = 0
    SINGLE = 100
    GROUP = 120
    MAILINGLIST = 140
    BROADCAST = 160


class ChatVisibility(str, Enum):
    """Chat visibility types"""

    NORMAL = "Normal"
    ARCHIVED = "Archived"
    PINNED = "Pinned"


class DownloadState(str, Enum):
    """Message download state"""

    DONE = "Done"
    AVAILABLE = "Available"
    FAILURE = "Failure"
    IN_PROGRESS = "InProgress"


class ViewType(str, Enum):
    """Message view type."""

    UNKNOWN = "Unknown"
    TEXT = "Text"
    IMAGE = "Image"
    GIF = "Gif"
    STICKER = "Sticker"
    AUDIO = "Audio"
    VOICE = "Voice"
    VIDEO = "Video"
    FILE = "File"
    VIDEOCHAT_INVITATION = "VideochatInvitation"
    WEBXDC = "Webxdc"


class SystemMessageType(str, Enum):
    """System message type."""

    UNKNOWN = "Unknown"
    GROUP_NAME_CHANGED = "GroupNameChanged"
    GROUP_IMAGE_CHANGED = "GroupImageChanged"
    MEMBER_ADDED_TO_GROUP = "MemberAddedToGroup"
    MEMBER_REMOVED_FROM_GROUP = "MemberRemovedFromGroup"
    AUTOCRYPT_SETUP_MESSAGE = "AutocryptSetupMessage"
    SECUREJOIN_MESSAGE = "SecurejoinMessage"
    LOCATION_STREAMING_ENABLED = "LocationStreamingEnabled"
    LOCATION_ONLY = "LocationOnly"
    CHAT_PROTECTION_ENABLED = "ChatProtectionEnabled"
    CHAT_PROTECTION_DISABLED = "ChatProtectionDisabled"
    WEBXDC_STATUS_UPDATE = "WebxdcStatusUpdate"
    EPHEMERAL_TIMER_CHANGED = "EphemeralTimerChanged"
    MULTI_DEVICE_SYNC = "MultiDeviceSync"
    WEBXDC_INFO_MESSAGE = "WebxdcInfoMessage"

from dataclasses import dataclass
from enum import Enum
from typing import TypeAlias, Union, Optional, Tuple, Any


class AccountEnum:
    @dataclass(kw_only=True)
    class Configured:
        kind: str = "Configured"
        addr: Optional[str]
        color: str
        display_name: Optional[str]
        id: int
        profile_image: Optional[str]

    @dataclass(kw_only=True)
    class Unconfigured:
        kind: str = "Unconfigured"
        id: int


Account: TypeAlias = AccountEnum.Configured | AccountEnum.Unconfigured


@dataclass(kw_only=True)
class BasicChat:
    archived: bool
    chat_type: int
    color: str
    id: int
    is_contact_request: bool
    is_device_chat: bool
    is_muted: bool
    is_protected: bool
    is_self_talk: bool
    is_unpromoted: bool
    name: str
    profile_image: str


class ChatListItemFetchResultEnum:
    @dataclass(kw_only=True)
    class ChatListItem:
        kind: str = "ChatListItem"
        avatar_path: Optional[str]
        color: str
        dm_chat_contact: Optional[int]
        fresh_message_counter: int
        id: int
        is_archived: bool
        is_broadcast: bool
        is_contact_request: bool
        is_device_talk: bool
        is_group: bool
        is_muted: bool
        is_pinned: bool
        is_protected: bool
        is_self_in_group: bool
        is_self_talk: bool
        is_sending_location: bool
        last_message_id: Optional[int]
        last_message_type: Optional["Viewtype"]
        last_updated: Optional[int]
        name: str
        summary_preview_image: Optional[str]
        summary_status: int
        summary_text1: str
        summary_text2: str
        was_seen_recently: bool

    @dataclass(kw_only=True)
    class ArchiveLink:
        kind: str = "ArchiveLink"
        fresh_message_counter: int

    @dataclass(kw_only=True)
    class Error:
        kind: str = "Error"
        error: str
        id: int


ChatListItemFetchResult: TypeAlias = (
    ChatListItemFetchResultEnum.ChatListItem
    | ChatListItemFetchResultEnum.ArchiveLink
    | ChatListItemFetchResultEnum.Error
)


class ChatVisibility(Enum):
    NORMAL = "Normal"
    ARCHIVED = "Archived"
    PINNED = "Pinned"


@dataclass(kw_only=True)
class Contact:
    address: str
    auth_name: str
    color: str
    display_name: str
    id: int
    is_blocked: bool
    is_verified: bool
    last_seen: int
    name: str
    name_and_addr: str
    profile_image: str
    status: str
    verifier_addr: str
    verifier_id: int
    was_seen_recently: bool


class DownloadState(Enum):
    DONE = "Done"
    AVAILABLE = "Available"
    FAILURE = "Failure"
    IN_PROGRESS = "InProgress"


@dataclass(kw_only=True)
class Event:
    context_id: int
    event: "EventType"


class EventTypeEnum:
    @dataclass(kw_only=True)
    class Info:
        kind: str = "Info"
        msg: str

    @dataclass(kw_only=True)
    class SmtpConnected:
        kind: str = "SmtpConnected"
        msg: str

    @dataclass(kw_only=True)
    class ImapConnected:
        kind: str = "ImapConnected"
        msg: str

    @dataclass(kw_only=True)
    class SmtpMessageSent:
        kind: str = "SmtpMessageSent"
        msg: str

    @dataclass(kw_only=True)
    class ImapMessageDeleted:
        kind: str = "ImapMessageDeleted"
        msg: str

    @dataclass(kw_only=True)
    class ImapMessageMoved:
        kind: str = "ImapMessageMoved"
        msg: str

    @dataclass(kw_only=True)
    class ImapInboxIdle:
        kind: str = "ImapInboxIdle"

    @dataclass(kw_only=True)
    class NewBlobFile:
        kind: str = "NewBlobFile"
        file: str

    @dataclass(kw_only=True)
    class DeletedBlobFile:
        kind: str = "DeletedBlobFile"
        file: str

    @dataclass(kw_only=True)
    class Warning:
        kind: str = "Warning"
        msg: str

    @dataclass(kw_only=True)
    class Error:
        kind: str = "Error"
        msg: str

    @dataclass(kw_only=True)
    class ErrorSelfNotInGroup:
        kind: str = "ErrorSelfNotInGroup"
        msg: str

    @dataclass(kw_only=True)
    class MsgsChanged:
        kind: str = "MsgsChanged"
        chat_id: int
        msg_id: int

    @dataclass(kw_only=True)
    class ReactionsChanged:
        kind: str = "ReactionsChanged"
        chat_id: int
        contact_id: int
        msg_id: int

    @dataclass(kw_only=True)
    class IncomingMsg:
        kind: str = "IncomingMsg"
        chat_id: int
        msg_id: int

    @dataclass(kw_only=True)
    class IncomingMsgBunch:
        kind: str = "IncomingMsgBunch"
        msg_ids: list[int]

    @dataclass(kw_only=True)
    class MsgsNoticed:
        kind: str = "MsgsNoticed"
        chat_id: int

    @dataclass(kw_only=True)
    class MsgDelivered:
        kind: str = "MsgDelivered"
        chat_id: int
        msg_id: int

    @dataclass(kw_only=True)
    class MsgFailed:
        kind: str = "MsgFailed"
        chat_id: int
        msg_id: int

    @dataclass(kw_only=True)
    class MsgRead:
        kind: str = "MsgRead"
        chat_id: int
        msg_id: int

    @dataclass(kw_only=True)
    class MsgDeleted:
        kind: str = "MsgDeleted"
        chat_id: int
        msg_id: int

    @dataclass(kw_only=True)
    class ChatModified:
        kind: str = "ChatModified"
        chat_id: int

    @dataclass(kw_only=True)
    class ChatEphemeralTimerModified:
        kind: str = "ChatEphemeralTimerModified"
        chat_id: int
        timer: int

    @dataclass(kw_only=True)
    class ContactsChanged:
        kind: str = "ContactsChanged"
        contact_id: Optional[int]

    @dataclass(kw_only=True)
    class LocationChanged:
        kind: str = "LocationChanged"
        contact_id: Optional[int]

    @dataclass(kw_only=True)
    class ConfigureProgress:
        kind: str = "ConfigureProgress"
        comment: Optional[str]
        progress: int

    @dataclass(kw_only=True)
    class ImexProgress:
        kind: str = "ImexProgress"
        progress: int

    @dataclass(kw_only=True)
    class ImexFileWritten:
        kind: str = "ImexFileWritten"
        path: str

    @dataclass(kw_only=True)
    class SecurejoinInviterProgress:
        kind: str = "SecurejoinInviterProgress"
        contact_id: int
        progress: int

    @dataclass(kw_only=True)
    class SecurejoinJoinerProgress:
        kind: str = "SecurejoinJoinerProgress"
        contact_id: int
        progress: int

    @dataclass(kw_only=True)
    class ConnectivityChanged:
        kind: str = "ConnectivityChanged"

    @dataclass(kw_only=True)
    class SelfavatarChanged:
        kind: str = "SelfavatarChanged"

    @dataclass(kw_only=True)
    class WebxdcStatusUpdate:
        kind: str = "WebxdcStatusUpdate"
        msg_id: int
        status_update_serial: int

    @dataclass(kw_only=True)
    class WebxdcInstanceDeleted:
        kind: str = "WebxdcInstanceDeleted"
        msg_id: int


EventType: TypeAlias = (
    EventTypeEnum.Info
    | EventTypeEnum.SmtpConnected
    | EventTypeEnum.ImapConnected
    | EventTypeEnum.SmtpMessageSent
    | EventTypeEnum.ImapMessageDeleted
    | EventTypeEnum.ImapMessageMoved
    | EventTypeEnum.ImapInboxIdle
    | EventTypeEnum.NewBlobFile
    | EventTypeEnum.DeletedBlobFile
    | EventTypeEnum.Warning
    | EventTypeEnum.Error
    | EventTypeEnum.ErrorSelfNotInGroup
    | EventTypeEnum.MsgsChanged
    | EventTypeEnum.ReactionsChanged
    | EventTypeEnum.IncomingMsg
    | EventTypeEnum.IncomingMsgBunch
    | EventTypeEnum.MsgsNoticed
    | EventTypeEnum.MsgDelivered
    | EventTypeEnum.MsgFailed
    | EventTypeEnum.MsgRead
    | EventTypeEnum.MsgDeleted
    | EventTypeEnum.ChatModified
    | EventTypeEnum.ChatEphemeralTimerModified
    | EventTypeEnum.ContactsChanged
    | EventTypeEnum.LocationChanged
    | EventTypeEnum.ConfigureProgress
    | EventTypeEnum.ImexProgress
    | EventTypeEnum.ImexFileWritten
    | EventTypeEnum.SecurejoinInviterProgress
    | EventTypeEnum.SecurejoinJoinerProgress
    | EventTypeEnum.ConnectivityChanged
    | EventTypeEnum.SelfavatarChanged
    | EventTypeEnum.WebxdcStatusUpdate
    | EventTypeEnum.WebxdcInstanceDeleted
)


@dataclass(kw_only=True)
class FullChat:
    archived: bool
    can_send: bool
    chat_type: int
    color: str
    contact_ids: list[int]
    contacts: list["Contact"]
    ephemeral_timer: int
    fresh_message_counter: int
    id: int
    is_contact_request: bool
    is_device_chat: bool
    is_muted: bool
    is_protected: bool
    is_self_talk: bool
    is_unpromoted: bool
    mailing_list_address: str
    name: str
    profile_image: str
    self_in_group: bool
    was_seen_recently: bool


@dataclass(kw_only=True)
class HttpResponse:
    blob: str
    encoding: str
    mimetype: str


@dataclass(kw_only=True)
class Location:
    accuracy: float
    chat_id: int
    contact_id: int
    is_independent: bool
    latitude: float
    location_id: int
    longitude: float
    marker: str
    msg_id: int
    timestamp: int


@dataclass(kw_only=True)
class Message:
    chat_id: int
    dimensions_height: int
    dimensions_width: int
    download_state: "DownloadState"
    duration: int
    error: str
    file: str
    file_bytes: int
    file_mime: str
    file_name: str
    from_id: int
    has_deviating_timestamp: bool
    has_html: bool
    has_location: bool
    id: int
    is_bot: bool
    is_forwarded: bool
    is_info: bool
    is_setupmessage: bool
    override_sender_name: str
    parent_id: int
    quote: Optional["MessageQuote"]
    reactions: Optional["Reactions"]
    received_timestamp: int
    sender: "Contact"
    setup_code_begin: str
    show_padlock: bool
    sort_timestamp: int
    state: int
    subject: str
    system_message_type: "SystemMessageType"
    text: str
    timestamp: int
    videochat_type: int
    videochat_url: str
    view_type: "Viewtype"
    webxdc_info: Optional["WebxdcMessageInfo"]


@dataclass(kw_only=True)
class MessageData:
    file: str
    html: str
    location: Tuple[float, float]
    override_sender_name: str
    quoted_message_id: int
    text: str
    viewtype: Optional["Viewtype"]


class MessageListItemEnum:
    @dataclass(kw_only=True)
    class Message:
        kind: str = "Message"
        msg_id: int

    @dataclass(kw_only=True)
    class DayMarker:
        kind: str = "DayMarker"
        timestamp: int


MessageListItem: TypeAlias = MessageListItemEnum.Message | MessageListItemEnum.DayMarker


class MessageLoadResultEnum:
    @dataclass(kw_only=True)
    class Message:
        kind: str = "Message"
        chat_id: int
        dimensions_height: int
        dimensions_width: int
        download_state: "DownloadState"
        duration: int
        error: Optional[str]
        file: Optional[str]
        file_bytes: int
        file_mime: Optional[str]
        file_name: Optional[str]
        from_id: int
        has_deviating_timestamp: bool
        has_html: bool
        has_location: bool
        id: int
        is_bot: bool
        is_forwarded: bool
        is_info: bool
        is_setupmessage: bool
        override_sender_name: Optional[str]
        parent_id: Optional[int]
        quote: Optional["MessageQuote"]
        reactions: Optional["Reactions"]
        received_timestamp: int
        sender: "Contact"
        setup_code_begin: Optional[str]
        show_padlock: bool
        sort_timestamp: int
        state: int
        subject: str
        system_message_type: "SystemMessageType"
        text: str
        timestamp: int
        videochat_type: Optional[int]
        videochat_url: Optional[str]
        view_type: "Viewtype"
        webxdc_info: Optional["WebxdcMessageInfo"]

    @dataclass(kw_only=True)
    class LoadingError:
        kind: str = "LoadingError"
        error: str


MessageLoadResult: TypeAlias = MessageLoadResultEnum.Message | MessageLoadResultEnum.LoadingError


@dataclass(kw_only=True)
class MessageNotificationInfo:
    account_id: int
    chat_id: int
    chat_name: str
    chat_profile_image: str
    id: int
    image: str
    image_mime_type: str
    summary_prefix: str
    summary_text: str


class MessageQuoteEnum:
    @dataclass(kw_only=True)
    class JustText:
        kind: str = "JustText"
        text: str

    @dataclass(kw_only=True)
    class WithMessage:
        kind: str = "WithMessage"
        author_display_color: str
        author_display_name: str
        image: Optional[str]
        is_forwarded: bool
        message_id: int
        override_sender_name: Optional[str]
        text: str
        view_type: "Viewtype"


MessageQuote: TypeAlias = MessageQuoteEnum.JustText | MessageQuoteEnum.WithMessage


@dataclass(kw_only=True)
class MessageReadReceipt:
    contact_id: int
    timestamp: int


@dataclass(kw_only=True)
class MessageSearchResult:
    author_color: str
    author_id: int
    author_name: str
    author_profile_image: str
    chat_color: str
    chat_name: str
    chat_profile_image: str
    chat_type: int
    id: int
    is_chat_archived: bool
    is_chat_contact_request: bool
    is_chat_protected: bool
    message: str
    timestamp: int


class MuteDurationEnum:
    @dataclass(kw_only=True)
    class NotMuted:
        kind: str = "NotMuted"

    @dataclass(kw_only=True)
    class Forever:
        kind: str = "Forever"

    @dataclass(kw_only=True)
    class Until:
        kind: str = "Until"
        timestamp: int


MuteDuration: TypeAlias = MuteDurationEnum.NotMuted | MuteDurationEnum.Forever | MuteDurationEnum.Until


@dataclass(kw_only=True)
class ProviderInfo:
    before_login_hint: str
    overview_page: str
    status: int


class QrEnum:
    @dataclass(kw_only=True)
    class AskVerifyContact:
        kind: str = "AskVerifyContact"
        authcode: str
        contact_id: int
        fingerprint: str
        invitenumber: str

    @dataclass(kw_only=True)
    class AskVerifyGroup:
        kind: str = "AskVerifyGroup"
        authcode: str
        contact_id: int
        fingerprint: str
        grpid: str
        grpname: str
        invitenumber: str

    @dataclass(kw_only=True)
    class FprOk:
        kind: str = "FprOk"
        contact_id: int

    @dataclass(kw_only=True)
    class FprMismatch:
        kind: str = "FprMismatch"
        contact_id: Optional[int]

    @dataclass(kw_only=True)
    class FprWithoutAddr:
        kind: str = "FprWithoutAddr"
        fingerprint: str

    @dataclass(kw_only=True)
    class Account:
        kind: str = "Account"
        domain: str

    @dataclass(kw_only=True)
    class Backup:
        kind: str = "Backup"
        ticket: str

    @dataclass(kw_only=True)
    class WebrtcInstance:
        kind: str = "WebrtcInstance"
        domain: str
        instance_pattern: str

    @dataclass(kw_only=True)
    class Addr:
        kind: str = "Addr"
        contact_id: int
        draft: Optional[str]

    @dataclass(kw_only=True)
    class Url:
        kind: str = "Url"
        url: str

    @dataclass(kw_only=True)
    class Text:
        kind: str = "Text"
        text: str

    @dataclass(kw_only=True)
    class WithdrawVerifyContact:
        kind: str = "WithdrawVerifyContact"
        authcode: str
        contact_id: int
        fingerprint: str
        invitenumber: str

    @dataclass(kw_only=True)
    class WithdrawVerifyGroup:
        kind: str = "WithdrawVerifyGroup"
        authcode: str
        contact_id: int
        fingerprint: str
        grpid: str
        grpname: str
        invitenumber: str

    @dataclass(kw_only=True)
    class ReviveVerifyContact:
        kind: str = "ReviveVerifyContact"
        authcode: str
        contact_id: int
        fingerprint: str
        invitenumber: str

    @dataclass(kw_only=True)
    class ReviveVerifyGroup:
        kind: str = "ReviveVerifyGroup"
        authcode: str
        contact_id: int
        fingerprint: str
        grpid: str
        grpname: str
        invitenumber: str

    @dataclass(kw_only=True)
    class Login:
        kind: str = "Login"
        address: str


Qr: TypeAlias = (
    QrEnum.AskVerifyContact
    | QrEnum.AskVerifyGroup
    | QrEnum.FprOk
    | QrEnum.FprMismatch
    | QrEnum.FprWithoutAddr
    | QrEnum.Account
    | QrEnum.Backup
    | QrEnum.WebrtcInstance
    | QrEnum.Addr
    | QrEnum.Url
    | QrEnum.Text
    | QrEnum.WithdrawVerifyContact
    | QrEnum.WithdrawVerifyGroup
    | QrEnum.ReviveVerifyContact
    | QrEnum.ReviveVerifyGroup
    | QrEnum.Login
)


@dataclass(kw_only=True)
class Reaction:
    count: int
    emoji: str
    is_from_self: bool


@dataclass(kw_only=True)
class Reactions:
    reactions: list["Reaction"]
    reactions_by_contact: dict[Any, list[str]]


class SystemMessageType(Enum):
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


class Viewtype(Enum):
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


@dataclass(kw_only=True)
class WebxdcMessageInfo:
    document: str
    icon: str
    internet_access: bool
    name: str
    source_code_url: str
    summary: str

use anyhow::{anyhow, Result};
use deltachat::chat::Chat;
use deltachat::chat::ChatItem;
use deltachat::constants::Chattype;
use deltachat::contact::Contact;
use deltachat::context::Context;
use deltachat::download;
use deltachat::message::Message;
use deltachat::message::MsgId;
use deltachat::message::Viewtype;
use deltachat::reaction::get_msg_reactions;
use num_traits::cast::ToPrimitive;
use serde::Deserialize;
use serde::Serialize;
use typescript_type_def::TypeDef;

use super::color_int_to_hex_string;
use super::contact::ContactObject;
use super::reactions::JSONRPCReactions;
use super::webxdc::WebxdcMessageInfo;

#[derive(Serialize, TypeDef)]
#[serde(rename_all = "camelCase", tag = "variant")]
pub enum MessageLoadResult {
    Message(MessageObject),
    LoadingError { error: String },
}

#[derive(Serialize, TypeDef)]
#[serde(rename = "Message", rename_all = "camelCase")]
pub struct MessageObject {
    id: u32,
    chat_id: u32,
    from_id: u32,
    quote: Option<MessageQuote>,
    parent_id: Option<u32>,

    text: Option<String>,
    has_location: bool,
    has_html: bool,
    view_type: MessageViewtype,
    state: u32,

    /// An error text, if there is one.
    error: Option<String>,

    timestamp: i64,
    sort_timestamp: i64,
    received_timestamp: i64,
    has_deviating_timestamp: bool,

    // summary - use/create another function if you need it
    subject: String,
    show_padlock: bool,
    is_setupmessage: bool,
    is_info: bool,
    is_forwarded: bool,

    /// True if the message was sent by a bot.
    is_bot: bool,

    /// when is_info is true this describes what type of system message it is
    system_message_type: SystemMessageType,

    duration: i32,
    dimensions_height: i32,
    dimensions_width: i32,

    videochat_type: Option<u32>,
    videochat_url: Option<String>,

    override_sender_name: Option<String>,
    sender: ContactObject,

    setup_code_begin: Option<String>,

    file: Option<String>,
    file_mime: Option<String>,
    file_bytes: u64,
    file_name: Option<String>,

    webxdc_info: Option<WebxdcMessageInfo>,

    download_state: DownloadState,

    reactions: Option<JSONRPCReactions>,
}

#[derive(Serialize, TypeDef)]
#[serde(tag = "kind")]
enum MessageQuote {
    JustText {
        text: String,
    },
    #[serde(rename_all = "camelCase")]
    WithMessage {
        text: String,
        message_id: u32,
        author_display_name: String,
        author_display_color: String,
        override_sender_name: Option<String>,
        image: Option<String>,
        is_forwarded: bool,
        view_type: MessageViewtype,
    },
}

impl MessageObject {
    pub async fn from_message_id(context: &Context, message_id: u32) -> Result<Self> {
        let msg_id = MsgId::new(message_id);
        Self::from_msg_id(context, msg_id).await
    }

    pub async fn from_msg_id(context: &Context, msg_id: MsgId) -> Result<Self> {
        let message = Message::load_from_db(context, msg_id).await?;

        let sender_contact = Contact::load_from_db(context, message.get_from_id()).await?;
        let sender = ContactObject::try_from_dc_contact(context, sender_contact).await?;
        let file_bytes = message.get_filebytes(context).await?.unwrap_or_default();
        let override_sender_name = message.get_override_sender_name();

        let webxdc_info = if message.get_viewtype() == Viewtype::Webxdc {
            Some(WebxdcMessageInfo::get_for_message(context, msg_id).await?)
        } else {
            None
        };

        let parent_id = message.parent(context).await?.map(|m| m.get_id().to_u32());

        let download_state = message.download_state().into();

        let quote = if let Some(quoted_text) = message.quoted_text() {
            match message.quoted_message(context).await? {
                Some(quote) => {
                    let quote_author = Contact::load_from_db(context, quote.get_from_id()).await?;
                    Some(MessageQuote::WithMessage {
                        text: quoted_text,
                        message_id: quote.get_id().to_u32(),
                        author_display_name: quote_author.get_display_name().to_owned(),
                        author_display_color: color_int_to_hex_string(quote_author.get_color()),
                        override_sender_name: quote.get_override_sender_name(),
                        image: if quote.get_viewtype() == Viewtype::Image
                            || quote.get_viewtype() == Viewtype::Gif
                            || quote.get_viewtype() == Viewtype::Sticker
                        {
                            match quote.get_file(context) {
                                Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
                                None => None,
                            }
                        } else {
                            None
                        },
                        is_forwarded: quote.is_forwarded(),
                        view_type: quote.get_viewtype().into(),
                    })
                }
                None => Some(MessageQuote::JustText { text: quoted_text }),
            }
        } else {
            None
        };

        let reactions = get_msg_reactions(context, msg_id).await?;
        let reactions = if reactions.is_empty() {
            None
        } else {
            Some(reactions.into())
        };

        Ok(MessageObject {
            id: msg_id.to_u32(),
            chat_id: message.get_chat_id().to_u32(),
            from_id: message.get_from_id().to_u32(),
            quote,
            parent_id,
            text: message.get_text(),
            has_location: message.has_location(),
            has_html: message.has_html(),
            view_type: message.get_viewtype().into(),
            state: message
                .get_state()
                .to_u32()
                .ok_or_else(|| anyhow!("state conversion to number failed"))?,
            error: message.error(),

            timestamp: message.get_timestamp(),
            sort_timestamp: message.get_sort_timestamp(),
            received_timestamp: message.get_received_timestamp(),
            has_deviating_timestamp: message.has_deviating_timestamp(),

            subject: message.get_subject().to_owned(),
            show_padlock: message.get_showpadlock(),
            is_setupmessage: message.is_setupmessage(),
            is_info: message.is_info(),
            is_forwarded: message.is_forwarded(),
            is_bot: message.is_bot(),
            system_message_type: message.get_info_type().into(),

            duration: message.get_duration(),
            dimensions_height: message.get_height(),
            dimensions_width: message.get_width(),

            videochat_type: match message.get_videochat_type() {
                Some(vct) => Some(
                    vct.to_u32()
                        .ok_or_else(|| anyhow!("state conversion to number failed"))?,
                ),
                None => None,
            },
            videochat_url: message.get_videochat_url(),

            override_sender_name,
            sender,

            setup_code_begin: message.get_setupcodebegin(context).await,

            file: match message.get_file(context) {
                Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
                None => None,
            }, //BLOBS
            file_mime: message.get_filemime(),
            file_bytes,
            file_name: message.get_filename(),
            webxdc_info,

            download_state,

            reactions,
        })
    }
}

#[derive(Serialize, Deserialize, TypeDef)]
#[serde(rename = "Viewtype")]
pub enum MessageViewtype {
    Unknown,

    /// Text message.
    Text,

    /// Image message.
    /// If the image is an animated GIF, the type `Viewtype.Gif` should be used.
    Image,

    /// Animated GIF message.
    Gif,

    /// Message containing a sticker, similar to image.
    /// If possible, the ui should display the image without borders in a transparent way.
    /// A click on a sticker will offer to install the sticker set in some future.
    Sticker,

    /// Message containing an Audio file.
    Audio,

    /// A voice message that was directly recorded by the user.
    /// For all other audio messages, the type `Viewtype.Audio` should be used.
    Voice,

    /// Video messages.
    Video,

    /// Message containing any file, eg. a PDF.
    File,

    /// Message is an invitation to a videochat.
    VideochatInvitation,

    /// Message is an webxdc instance.
    Webxdc,
}

impl From<Viewtype> for MessageViewtype {
    fn from(viewtype: Viewtype) -> Self {
        match viewtype {
            Viewtype::Unknown => MessageViewtype::Unknown,
            Viewtype::Text => MessageViewtype::Text,
            Viewtype::Image => MessageViewtype::Image,
            Viewtype::Gif => MessageViewtype::Gif,
            Viewtype::Sticker => MessageViewtype::Sticker,
            Viewtype::Audio => MessageViewtype::Audio,
            Viewtype::Voice => MessageViewtype::Voice,
            Viewtype::Video => MessageViewtype::Video,
            Viewtype::File => MessageViewtype::File,
            Viewtype::VideochatInvitation => MessageViewtype::VideochatInvitation,
            Viewtype::Webxdc => MessageViewtype::Webxdc,
        }
    }
}

impl From<MessageViewtype> for Viewtype {
    fn from(viewtype: MessageViewtype) -> Self {
        match viewtype {
            MessageViewtype::Unknown => Viewtype::Unknown,
            MessageViewtype::Text => Viewtype::Text,
            MessageViewtype::Image => Viewtype::Image,
            MessageViewtype::Gif => Viewtype::Gif,
            MessageViewtype::Sticker => Viewtype::Sticker,
            MessageViewtype::Audio => Viewtype::Audio,
            MessageViewtype::Voice => Viewtype::Voice,
            MessageViewtype::Video => Viewtype::Video,
            MessageViewtype::File => Viewtype::File,
            MessageViewtype::VideochatInvitation => Viewtype::VideochatInvitation,
            MessageViewtype::Webxdc => Viewtype::Webxdc,
        }
    }
}

#[derive(Serialize, TypeDef)]
pub enum DownloadState {
    Done,
    Available,
    Failure,
    InProgress,
}

impl From<download::DownloadState> for DownloadState {
    fn from(state: download::DownloadState) -> Self {
        match state {
            download::DownloadState::Done => DownloadState::Done,
            download::DownloadState::Available => DownloadState::Available,
            download::DownloadState::Failure => DownloadState::Failure,
            download::DownloadState::InProgress => DownloadState::InProgress,
        }
    }
}

#[derive(Serialize, TypeDef)]
pub enum SystemMessageType {
    Unknown,
    GroupNameChanged,
    GroupImageChanged,
    MemberAddedToGroup,
    MemberRemovedFromGroup,
    AutocryptSetupMessage,
    SecurejoinMessage,
    LocationStreamingEnabled,
    LocationOnly,

    /// Chat ephemeral message timer is changed.
    EphemeralTimerChanged,

    // Chat protection state changed
    ChatProtectionEnabled,
    ChatProtectionDisabled,

    /// Self-sent-message that contains only json used for multi-device-sync;
    /// if possible, we attach that to other messages as for locations.
    MultiDeviceSync,

    // Sync message that contains a json payload
    // sent to the other webxdc instances
    // These messages are not shown in the chat.
    WebxdcStatusUpdate,

    /// Webxdc info added with `info` set in `send_webxdc_status_update()`.
    WebxdcInfoMessage,
}

impl From<deltachat::mimeparser::SystemMessage> for SystemMessageType {
    fn from(system_message_type: deltachat::mimeparser::SystemMessage) -> Self {
        use deltachat::mimeparser::SystemMessage;
        match system_message_type {
            SystemMessage::Unknown => SystemMessageType::Unknown,
            SystemMessage::GroupNameChanged => SystemMessageType::GroupNameChanged,
            SystemMessage::GroupImageChanged => SystemMessageType::GroupImageChanged,
            SystemMessage::MemberAddedToGroup => SystemMessageType::MemberAddedToGroup,
            SystemMessage::MemberRemovedFromGroup => SystemMessageType::MemberRemovedFromGroup,
            SystemMessage::AutocryptSetupMessage => SystemMessageType::AutocryptSetupMessage,
            SystemMessage::SecurejoinMessage => SystemMessageType::SecurejoinMessage,
            SystemMessage::LocationStreamingEnabled => SystemMessageType::LocationStreamingEnabled,
            SystemMessage::LocationOnly => SystemMessageType::LocationOnly,
            SystemMessage::EphemeralTimerChanged => SystemMessageType::EphemeralTimerChanged,
            SystemMessage::ChatProtectionEnabled => SystemMessageType::ChatProtectionEnabled,
            SystemMessage::ChatProtectionDisabled => SystemMessageType::ChatProtectionDisabled,
            SystemMessage::MultiDeviceSync => SystemMessageType::MultiDeviceSync,
            SystemMessage::WebxdcStatusUpdate => SystemMessageType::WebxdcStatusUpdate,
            SystemMessage::WebxdcInfoMessage => SystemMessageType::WebxdcInfoMessage,
        }
    }
}

#[derive(Serialize, TypeDef)]
#[serde(rename_all = "camelCase")]
pub struct MessageNotificationInfo {
    id: u32,
    chat_id: u32,
    account_id: u32,

    image: Option<String>,
    image_mime_type: Option<String>,

    chat_name: String,
    chat_profile_image: Option<String>,

    /// also known as summary_text1
    summary_prefix: Option<String>,
    /// also known as summary_text2
    summary_text: String,
}

impl MessageNotificationInfo {
    pub async fn from_msg_id(context: &Context, msg_id: MsgId) -> Result<Self> {
        let message = Message::load_from_db(context, msg_id).await?;
        let chat = Chat::load_from_db(context, message.get_chat_id()).await?;

        let image = if matches!(
            message.get_viewtype(),
            Viewtype::Image | Viewtype::Gif | Viewtype::Sticker
        ) {
            message
                .get_file(context)
                .map(|path_buf| path_buf.to_str().map(|s| s.to_owned()))
                .unwrap_or_default()
        } else {
            None
        };

        let chat_profile_image = chat
            .get_profile_image(context)
            .await?
            .map(|path_buf| path_buf.to_str().map(|s| s.to_owned()))
            .unwrap_or_default();

        let summary = message.get_summary(context, Some(&chat)).await?;

        Ok(MessageNotificationInfo {
            id: msg_id.to_u32(),
            chat_id: message.get_chat_id().to_u32(),
            account_id: context.get_id(),
            image,
            image_mime_type: message.get_filemime(),
            chat_name: chat.name,
            chat_profile_image,
            summary_prefix: summary.prefix.map(|s| s.to_string()),
            summary_text: summary.text,
        })
    }
}

#[derive(Serialize, TypeDef)]
#[serde(rename_all = "camelCase")]
pub struct MessageSearchResult {
    id: u32,
    author_profile_image: Option<String>,
    author_name: String,
    author_color: String,
    chat_name: Option<String>,
    message: String,
    timestamp: i64,
}

impl MessageSearchResult {
    pub async fn from_msg_id(context: &Context, msg_id: MsgId) -> Result<Self> {
        let message = Message::load_from_db(context, msg_id).await?;
        let chat = Chat::load_from_db(context, message.get_chat_id()).await?;
        let sender = Contact::load_from_db(context, message.get_from_id()).await?;

        let profile_image = match sender.get_profile_image(context).await? {
            Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
            None => None,
        };

        Ok(Self {
            id: msg_id.to_u32(),
            author_profile_image: profile_image,
            author_name: sender.get_display_name().to_owned(),
            author_color: color_int_to_hex_string(sender.get_color()),
            chat_name: if chat.get_type() == Chattype::Single {
                Some(chat.get_name().to_owned())
            } else {
                None
            },
            message: message.get_text().unwrap_or_default(),
            timestamp: message.get_timestamp(),
        })
    }
}

#[derive(Serialize, TypeDef)]
#[serde(rename_all = "camelCase", rename = "MessageListItem", tag = "kind")]
pub enum JSONRPCMessageListItem {
    Message {
        msg_id: u32,
    },

    /// Day marker, separating messages that correspond to different
    /// days according to local time.
    DayMarker {
        /// Marker timestamp, for day markers, in unix milliseconds
        timestamp: i64,
    },
}

impl From<ChatItem> for JSONRPCMessageListItem {
    fn from(item: ChatItem) -> Self {
        match item {
            ChatItem::Message { msg_id } => JSONRPCMessageListItem::Message {
                msg_id: msg_id.to_u32(),
            },
            ChatItem::DayMarker { timestamp } => JSONRPCMessageListItem::DayMarker { timestamp },
        }
    }
}

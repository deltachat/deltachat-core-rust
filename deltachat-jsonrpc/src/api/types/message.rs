use anyhow::{anyhow, Result};
use deltachat::contact::Contact;
use deltachat::context::Context;
use deltachat::message::Message;
use deltachat::message::MsgId;
use deltachat::message::Viewtype;
use num_traits::cast::ToPrimitive;
use serde::Deserialize;
use serde::Serialize;
use typescript_type_def::TypeDef;

use super::contact::ContactObject;
use super::webxdc::WebxdcMessageInfo;

#[derive(Serialize, TypeDef)]
#[serde(rename = "Message", rename_all = "camelCase")]
pub struct MessageObject {
    id: u32,
    chat_id: u32,
    from_id: u32,
    quoted_text: Option<String>,
    quoted_message_id: Option<u32>,
    text: Option<String>,
    has_location: bool,
    has_html: bool,
    view_type: MessageViewtype,
    state: u32,

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
}

impl MessageObject {
    pub async fn from_message_id(context: &Context, message_id: u32) -> Result<Self> {
        let msg_id = MsgId::new(message_id);
        let message = Message::load_from_db(context, msg_id).await?;

        let quoted_message_id = message
            .quoted_message(context)
            .await?
            .map(|m| m.get_id().to_u32());

        let sender_contact = Contact::load_from_db(context, message.get_from_id()).await?;
        let sender = ContactObject::try_from_dc_contact(context, sender_contact).await?;
        let file_bytes = message.get_filebytes(context).await;
        let override_sender_name = message.get_override_sender_name();

        let webxdc_info = if message.get_viewtype() == Viewtype::Webxdc {
            Some(WebxdcMessageInfo::get_for_message(context, msg_id).await?)
        } else {
            None
        };

        Ok(MessageObject {
            id: message_id,
            chat_id: message.get_chat_id().to_u32(),
            from_id: message.get_from_id().to_u32(),
            quoted_text: message.quoted_text(),
            quoted_message_id,
            text: message.get_text(),
            has_location: message.has_location(),
            has_html: message.has_html(),
            view_type: message.get_viewtype().into(),
            state: message
                .get_state()
                .to_u32()
                .ok_or_else(|| anyhow!("state conversion to number failed"))?,

            timestamp: message.get_timestamp(),
            sort_timestamp: message.get_sort_timestamp(),
            received_timestamp: message.get_received_timestamp(),
            has_deviating_timestamp: message.has_deviating_timestamp(),

            subject: message.get_subject().to_owned(),
            show_padlock: message.get_showpadlock(),
            is_setupmessage: message.is_setupmessage(),
            is_info: message.is_info(),
            is_forwarded: message.is_forwarded(),

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

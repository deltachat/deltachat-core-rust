//! # Message summary for chatlist.

use crate::chat::Chat;
use crate::constants::{Chattype, Viewtype, DC_CONTACT_ID_SELF};
use crate::contact::Contact;
use crate::context::Context;
use crate::dc_tools::dc_truncate;
use crate::message::{Message, MessageState};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::stock_str;
use std::borrow::Cow;
use std::fmt;

/// Prefix displayed before message and separated by ":" in the chatlist.
#[derive(Debug)]
pub enum SummaryPrefix {
    /// Username.
    Username(String),

    /// Stock string saying "Draft".
    Draft(String),

    /// Stock string saying "Me".
    Me(String),
}

impl fmt::Display for SummaryPrefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SummaryPrefix::Username(username) => write!(f, "{}", username),
            SummaryPrefix::Draft(text) => write!(f, "{}", text),
            SummaryPrefix::Me(text) => write!(f, "{}", text),
        }
    }
}

/// Message summary.
#[derive(Debug, Default)]
pub struct Summary {
    /// Part displayed before ":", such as an username or a string "Draft".
    pub prefix: Option<SummaryPrefix>,

    /// Summary text, always present.
    pub text: String,

    /// Message timestamp.
    pub timestamp: i64,

    /// Message state.
    pub state: MessageState,
}

impl Summary {
    pub async fn new(
        context: &Context,
        msg: &Message,
        chat: &Chat,
        contact: Option<&Contact>,
    ) -> Self {
        let prefix = if msg.state == MessageState::OutDraft {
            Some(SummaryPrefix::Draft(stock_str::draft(context).await))
        } else if msg.from_id == DC_CONTACT_ID_SELF {
            if msg.is_info() || chat.is_self_talk() {
                None
            } else {
                Some(SummaryPrefix::Me(stock_str::self_msg(context).await))
            }
        } else {
            match chat.typ {
                Chattype::Group | Chattype::Broadcast | Chattype::Mailinglist => {
                    if msg.is_info() || contact.is_none() {
                        None
                    } else {
                        msg.get_override_sender_name()
                            .or_else(|| contact.map(|contact| msg.get_sender_name(contact)))
                            .map(SummaryPrefix::Username)
                    }
                }
                Chattype::Single | Chattype::Undefined => None,
            }
        };

        let mut text = msg.get_summary_text(context).await;

        if text.is_empty() && msg.quoted_text().is_some() {
            text = stock_str::reply_noun(context).await
        }

        Self {
            prefix,
            text,
            timestamp: msg.get_timestamp(),
            state: msg.state,
        }
    }

    /// Returns the [`Summary::text`] attribute truncated to an approximate length.
    pub fn truncated_text(&self, approx_chars: usize) -> Cow<str> {
        dc_truncate(&self.text, approx_chars)
    }
}

impl Message {
    /// Returns a summary text.
    async fn get_summary_text(&self, context: &Context) -> String {
        let mut append_text = true;
        let prefix = match self.viewtype {
            Viewtype::Image => stock_str::image(context).await,
            Viewtype::Gif => stock_str::gif(context).await,
            Viewtype::Sticker => stock_str::sticker(context).await,
            Viewtype::Video => stock_str::video(context).await,
            Viewtype::Voice => stock_str::voice_message(context).await,
            Viewtype::Audio | Viewtype::File => {
                if self.param.get_cmd() == SystemMessage::AutocryptSetupMessage {
                    append_text = false;
                    stock_str::ac_setup_msg_subject(context).await
                } else {
                    let file_name: String = self
                        .param
                        .get_path(Param::File, context)
                        .unwrap_or(None)
                        .and_then(|path| {
                            path.file_name()
                                .map(|fname| fname.to_string_lossy().into_owned())
                        })
                        .unwrap_or_else(|| String::from("ErrFileName"));
                    let label = if self.viewtype == Viewtype::Audio {
                        stock_str::audio(context).await
                    } else {
                        stock_str::file(context).await
                    };
                    format!("{} – {}", label, file_name)
                }
            }
            Viewtype::VideochatInvitation => {
                append_text = false;
                stock_str::videochat_invitation(context).await
            }
            Viewtype::Webxdc => {
                append_text = true;
                self.get_webxdc_info(context)
                    .await
                    .map(|info| info.name)
                    .unwrap_or_else(|_| "ErrWebxdcName".to_string())
            }
            Viewtype::Text | Viewtype::Unknown => {
                if self.param.get_cmd() != SystemMessage::LocationOnly {
                    "".to_string()
                } else {
                    append_text = false;
                    stock_str::location(context).await
                }
            }
        };

        if !append_text {
            return prefix;
        }

        let summary_content = if let Some(text) = &self.text {
            if text.is_empty() {
                prefix
            } else if prefix.is_empty() {
                text.to_string()
            } else {
                format!("{} – {}", prefix, text)
            }
        } else {
            prefix
        };

        let summary = if self.is_forwarded() {
            format!(
                "{}: {}",
                stock_str::forwarded(context).await,
                summary_content
            )
        } else {
            summary_content
        };

        summary.split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils as test;

    #[async_std::test]
    async fn test_get_summary_text() {
        let d = test::TestContext::new().await;
        let ctx = &d.ctx;

        let some_text = Some(" bla \t\n\tbla\n\t".to_string());
        let empty_text = Some("".to_string());
        let no_text: Option<String> = None;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(some_text.clone());
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "bla bla" // for simple text, the type is not added to the summary
        );

        let mut msg = Message::new(Viewtype::Image);
        msg.set_text(no_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Image" // file names are not added for images
        );

        let mut msg = Message::new(Viewtype::Video);
        msg.set_text(no_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Video" // file names are not added for videos
        );

        let mut msg = Message::new(Viewtype::Gif);
        msg.set_text(no_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "GIF" // file names are not added for GIFs
        );

        let mut msg = Message::new(Viewtype::Sticker);
        msg.set_text(no_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Sticker" // file names are not added for stickers
        );

        let mut msg = Message::new(Viewtype::Voice);
        msg.set_text(empty_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Voice message" // file names are not added for voice messages, empty text is skipped
        );

        let mut msg = Message::new(Viewtype::Voice);
        msg.set_text(no_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Voice message" // file names are not added for voice messages
        );

        let mut msg = Message::new(Viewtype::Voice);
        msg.set_text(some_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Voice message \u{2013} bla bla" // `\u{2013}` explicitly checks for "EN DASH"
        );

        let mut msg = Message::new(Viewtype::Audio);
        msg.set_text(no_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Audio \u{2013} foo.bar" // file name is added for audio
        );

        let mut msg = Message::new(Viewtype::Audio);
        msg.set_text(empty_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Audio \u{2013} foo.bar" // file name is added for audio, empty text is not added
        );

        let mut msg = Message::new(Viewtype::Audio);
        msg.set_text(some_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Audio \u{2013} foo.bar \u{2013} bla bla" // file name and text added for audio
        );

        let mut msg = Message::new(Viewtype::File);
        msg.set_text(some_text.clone());
        msg.set_file("foo.bar", None);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "File \u{2013} foo.bar \u{2013} bla bla" // file name is added for files
        );

        // Forwarded
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(some_text.clone());
        msg.param.set_int(Param::Forwarded, 1);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Forwarded: bla bla" // for simple text, the type is not added to the summary
        );

        let mut msg = Message::new(Viewtype::File);
        msg.set_text(some_text.clone());
        msg.set_file("foo.bar", None);
        msg.param.set_int(Param::Forwarded, 1);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Forwarded: File \u{2013} foo.bar \u{2013} bla bla"
        );

        let mut msg = Message::new(Viewtype::File);
        msg.set_text(no_text.clone());
        msg.param.set(Param::File, "foo.bar");
        msg.param.set_cmd(SystemMessage::AutocryptSetupMessage);
        assert_eq!(
            msg.get_summary_text(ctx).await,
            "Autocrypt Setup Message" // file name is not added for autocrypt setup messages
        );
    }
}

//! # Message summary for chatlist.

use crate::chat::Chat;
use crate::constants::{Chattype, Viewtype, DC_CONTACT_ID_SELF};
use crate::contact::Contact;
use crate::context::Context;
use crate::dc_tools::dc_truncate;
use crate::message::{Message, MessageState};
use crate::mimeparser::SystemMessage;
use crate::param::{Param, Params};
use crate::stock_str;
use itertools::Itertools;
use std::fmt;

// In practice, the user additionally cuts the string themselves
// pixel-accurate.
const SUMMARY_CHARACTERS: usize = 160;

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

        let mut text = get_summarytext_by_raw(
            msg.viewtype,
            msg.text.as_ref(),
            msg.is_forwarded(),
            &msg.param,
            SUMMARY_CHARACTERS,
            context,
        )
        .await;

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
}

/// Returns a summary text.
pub async fn get_summarytext_by_raw(
    viewtype: Viewtype,
    text: Option<impl AsRef<str>>,
    was_forwarded: bool,
    param: &Params,
    approx_characters: usize,
    context: &Context,
) -> String {
    let mut append_text = true;
    let prefix = match viewtype {
        Viewtype::Image => stock_str::image(context).await,
        Viewtype::Gif => stock_str::gif(context).await,
        Viewtype::Sticker => stock_str::sticker(context).await,
        Viewtype::Video => stock_str::video(context).await,
        Viewtype::Voice => stock_str::voice_message(context).await,
        Viewtype::Audio | Viewtype::File => {
            if param.get_cmd() == SystemMessage::AutocryptSetupMessage {
                append_text = false;
                stock_str::ac_setup_msg_subject(context).await
            } else {
                let file_name: String = param
                    .get_path(Param::File, context)
                    .unwrap_or(None)
                    .and_then(|path| {
                        path.file_name()
                            .map(|fname| fname.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| String::from("ErrFileName"));
                let label = if viewtype == Viewtype::Audio {
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
        _ => {
            if param.get_cmd() != SystemMessage::LocationOnly {
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

    let summary_content = if let Some(text) = text {
        if text.as_ref().is_empty() {
            prefix
        } else if prefix.is_empty() {
            dc_truncate(text.as_ref(), approx_characters).to_string()
        } else {
            let tmp = format!("{} – {}", prefix, text.as_ref());
            dc_truncate(&tmp, approx_characters).to_string()
        }
    } else {
        prefix
    };

    let summary = if was_forwarded {
        let tmp = format!(
            "{}: {}",
            stock_str::forwarded(context).await,
            summary_content
        );
        dc_truncate(&tmp, approx_characters).to_string()
    } else {
        summary_content
    };

    summary.split_whitespace().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils as test;

    #[async_std::test]
    async fn test_get_summarytext_by_raw() {
        let d = test::TestContext::new().await;
        let ctx = &d.ctx;

        let some_text = Some(" bla \t\n\tbla\n\t".to_string());
        let empty_text = Some("".to_string());
        let no_text: Option<String> = None;

        let mut some_file = Params::new();
        some_file.set(Param::File, "foo.bar");

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Text,
                some_text.as_ref(),
                false,
                &Params::new(),
                50,
                ctx
            )
            .await,
            "bla bla" // for simple text, the type is not added to the summary
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Image,
                no_text.as_ref(),
                false,
                &some_file,
                50,
                ctx
            )
            .await,
            "Image" // file names are not added for images
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Video,
                no_text.as_ref(),
                false,
                &some_file,
                50,
                ctx
            )
            .await,
            "Video" // file names are not added for videos
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Gif, no_text.as_ref(), false, &some_file, 50, ctx,)
                .await,
            "GIF" // file names are not added for GIFs
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Sticker,
                no_text.as_ref(),
                false,
                &some_file,
                50,
                ctx,
            )
            .await,
            "Sticker" // file names are not added for stickers
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Voice,
                empty_text.as_ref(),
                false,
                &some_file,
                50,
                ctx,
            )
            .await,
            "Voice message" // file names are not added for voice messages, empty text is skipped
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Voice,
                no_text.as_ref(),
                false,
                &some_file,
                50,
                ctx
            )
            .await,
            "Voice message" // file names are not added for voice messages
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Voice,
                some_text.as_ref(),
                false,
                &some_file,
                50,
                ctx
            )
            .await,
            "Voice message \u{2013} bla bla" // `\u{2013}` explicitly checks for "EN DASH"
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Audio,
                no_text.as_ref(),
                false,
                &some_file,
                50,
                ctx
            )
            .await,
            "Audio \u{2013} foo.bar" // file name is added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Audio,
                empty_text.as_ref(),
                false,
                &some_file,
                50,
                ctx,
            )
            .await,
            "Audio \u{2013} foo.bar" // file name is added for audio, empty text is not added
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Audio,
                some_text.as_ref(),
                false,
                &some_file,
                50,
                ctx
            )
            .await,
            "Audio \u{2013} foo.bar \u{2013} bla bla" // file name and text added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::File,
                some_text.as_ref(),
                false,
                &some_file,
                50,
                ctx
            )
            .await,
            "File \u{2013} foo.bar \u{2013} bla bla" // file name is added for files
        );

        // Forwarded
        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::Text,
                some_text.as_ref(),
                true,
                &Params::new(),
                50,
                ctx
            )
            .await,
            "Forwarded: bla bla" // for simple text, the type is not added to the summary
        );
        assert_eq!(
            get_summarytext_by_raw(
                Viewtype::File,
                some_text.as_ref(),
                true,
                &some_file,
                50,
                ctx
            )
            .await,
            "Forwarded: File \u{2013} foo.bar \u{2013} bla bla"
        );

        let mut asm_file = Params::new();
        asm_file.set(Param::File, "foo.bar");
        asm_file.set_cmd(SystemMessage::AutocryptSetupMessage);
        assert_eq!(
            get_summarytext_by_raw(Viewtype::File, no_text.as_ref(), false, &asm_file, 50, ctx)
                .await,
            "Autocrypt Setup Message" // file name is not added for autocrypt setup messages
        );
    }
}

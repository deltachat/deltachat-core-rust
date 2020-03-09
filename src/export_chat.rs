// use crate::dc_tools::*;
use crate::chat::*;
use crate::constants::DC_CONTACT_ID_SELF;
use crate::contact::*;
use crate::context::Context;
use crate::error::Error;
use crate::message::*;
use std::collections::HashMap;

pub struct ExportChatResult {
    html: String,
    referenced_blobs: Vec<String>,
}

struct ContactInfo<'t> {
    name: &'t str,
    initial: &'t str,
    color: &'t str,
    profile_img: Option<&'t str>,
}

// pub fn packExportedChat(artifact:ExportChatResult) -> ? {}

pub fn export_chat(context: &Context, chat_id: ChatId) -> ExportChatResult {
    let mut blobs = Vec::new();
    let mut chat_author_ids = Vec::new();
    // get all messages
    let messages: Vec<std::result::Result<Message, Error>> =
        get_chat_msgs(context, chat_id, 0, None)
            .into_iter()
            .map(|msg_id| Message::load_from_db(context, msg_id))
            .collect();
    // push all referenced blobs and populate contactid list
    for message in messages {
        if let Ok(msg) = message {
            let filename = msg.get_filename();
            if let Some(file) = filename {
                blobs.push(file);
            }
            chat_author_ids.push(msg.from_id);
        }
    }
    // deduplicate contact list and load the contacts
    chat_author_ids.dedup();
    let mut chat_authors: HashMap<u32, ContactInfo> = HashMap::new();
    for author_id in chat_author_ids {
        let contact = Contact::get_by_id(context, author_id);
        if let Ok(c) = contact {
            let profile_img_path: String;
            if let Some(path) = c.get_profile_image(context) {
                profile_img_path = path.to_str().unwrap_or_else(|| "").to_owned();
                blobs.push(profile_img_path.clone());
            } else {
                profile_img_path = "".to_owned();
            }
            chat_authors.insert(
                author_id,
                ContactInfo {
                    name: c.get_display_name(),
                    initial: "#",
                    profile_img: match profile_img_path != "" {
                        true => Some(&profile_img_path),
                        false => None,
                    },
                    color: "rgb(18, 126, 208)",
                },
            );
        }
    }

    // author props
    // name, id, image, color

    // push all referenced blobs (avatars)

    // run message_to_html for each message and generate the html that way

    ExportChatResult {
        html: "".to_owned(),
        referenced_blobs: blobs,
    }
}

fn message_to_html(ctx: &Context, author_cache: HashMap<u32, ContactInfo>, id: MsgId) -> String {
    let message = Message::load_from_db(ctx, id).unwrap();

    let author: &ContactInfo = {
        if let Some(c) = author_cache.get(&message.get_from_id()) {
            c
        } else {
            &ContactInfo {
                name: "Err: Contact not found",
                initial: "#",
                profile_img: None,
                color: "grey",
            }
        }
    };

    let avatar: String = {
        if let Some(profile_img) = author.profile_img {
            format!(
                r#"<div class="author-avatar">
                    <img
                        alt="{author_name}"
                        src="{author_avatar_src}"
                    />
                </div>"#,
                author_name = author.name,
                author_avatar_src = profile_img
            )
        } else {
            format!(
                r#"<div class="author-avatar default" alt="{name}">
                <div class="label" style="background-color: {color}">
                    {initial}
                </div>
            </div>"#,
                name = author.name,
                initial = author.initial,
                color = author.color
            )
        }
    };

    // save and refernce message source code somehow?

    //todo support images / voice message / attachments

    format!(
        r#"<li><div class='message {direction}'>
    {avatar}
    <div class="msg-container">
          <span class="author" style="color: {author_color};">{author_name}</span>
          <div class="msg-body">
            <div dir="auto" class="text">
              {content}
            </div>
            <div class="metadata">
              {encryption}
              <span
                class="date date--{direction}"
                title="{full_time}"
                >{relative_time}</span
              ><span class="spacer"></span>
            </div>
          </div>
        </div>
    <div></li>"#,
        direction = match message.from_id == DC_CONTACT_ID_SELF {
            true => "outgoing",
            false => "incomming",
        },
        avatar = avatar,
        author_name = author.name,
        author_color = author.color,
        content = message.get_text().unwrap_or_else(|| "".to_owned()),
        encryption = match message.get_showpadlock() {
            true => r#"<div aria-label="Encryption padlock" class="padlock-icon"></div>"#,
            false => "",
        },
        full_time = "Tue, Feb 25, 2020 3:49 PM", // message.get_timestamp() ?
        relative_time = "Tue 3:49 PM"
    )

    // todo link to raw message data / link to message info
}

//TODO tests

// use crate::dc_tools::*;
use crate::chat::*;
use crate::constants::Viewtype;
use crate::contact::*;
use crate::context::Context;
// use crate::error::Error;
use crate::message::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use zip::write::FileOptions;

use crate::location::Location;
use futures::future::join_all;
use serde::Serialize;

#[derive(Debug)]
pub struct ExportChatResult {
    chat_json: String,
    // locations_geo_json: String,
    message_info: Vec<(u32, String, Option<String>)>,
    referenced_blobs: Vec<String>,
}

pub fn pack_exported_chat(
    context: &Context,
    artifact: ExportChatResult,
    filename: &str,
) -> zip::result::ZipResult<()> {
    let path = std::path::Path::new(filename);
    let file = std::fs::File::create(&path).unwrap();

    let mut zip = zip::ZipWriter::new(file);

    zip.start_file("index.json", Default::default())?;
    zip.write_all(artifact.chat_json.as_bytes())?;

    zip.add_directory("blobs/", Default::default())?;

    let options = FileOptions::default();
    for blob_name in artifact.referenced_blobs {
        let path = context.get_blobdir().join(&blob_name);

        // println!("adding file {:?} as {:?} ...", path, &blob_name);
        zip.start_file_from_path(Path::new(&format!("blobs/{}", &blob_name)), options)?;
        let mut f = File::open(path)?;

        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        zip.write_all(&*buffer)?;
        buffer.clear();
    }

    zip.add_directory("msg_info/", Default::default())?;
    for msg_info in artifact.message_info {
        zip.start_file_from_path(Path::new(&format!("msg_info/{}.txt", msg_info.0)), options)?;
        zip.write_all((msg_info.1).as_bytes())?;
        if let Some(mime_headers) = msg_info.2 {
            zip.start_file_from_path(Path::new(&format!("msg_info/{}.eml", msg_info.0)), options)?;
            zip.write_all((mime_headers).as_bytes())?;
        }
    }

    zip.finish()?;
    Ok(())
}

#[derive(Serialize)]
struct ChatJSON {
    chat_json_version: u8,
    name: String,
    color: String,
    profile_img: Option<String>,
    contacts: HashMap<u32, ContactJSON>,
    messages: Vec<MessageJSON>,
}

#[derive(Serialize)]
struct ContactJSON {
    name: String,
    email: String,
    color: String,
    profile_img: Option<String>,
}

#[derive(Serialize)]
struct FileReference {
    name: String,
    filesize: u64,
    mime: String,
    path: String,
}

#[derive(Serialize)]
struct MessageJSON {
    id: u32,
    author_id: u32, // from_id
    view_type: Viewtype,
    timestamp_sort: i64,
    timestamp_sent: i64,
    timestamp_rcvd: i64,
    text: Option<String>,
    attachment: Option<FileReference>,
    location: Option<Location>,
    is_info_message: bool,
    show_padlock: bool,
}

impl MessageJSON {
    pub async fn from_message(message: &Message, context: &Context) -> MessageJSON {
        let msg_id = message.get_id();
        MessageJSON {
            id: msg_id.to_u32(),
            author_id: message.get_from_id(), // from_id
            view_type: message.get_viewtype(),
            timestamp_sort: message.timestamp_sort,
            timestamp_sent: message.timestamp_sent,
            timestamp_rcvd: message.timestamp_rcvd,
            text: message.get_text(),
            attachment: match message.get_file(context) {
                Some(file) => Some(FileReference {
                    name: message.get_filename().unwrap_or_else(|| "".to_owned()),
                    filesize: message.get_filebytes(context).await,
                    mime: message.get_filemime().unwrap_or_else(|| "".to_owned()),
                    path: format!(
                        "blobs/{}",
                        file.file_name()
                            .unwrap_or_else(|| std::ffi::OsStr::new(""))
                            .to_str()
                            .unwrap()
                    ),
                }),
                None => None,
            },
            location: match message.has_location() {
                true => None, // todo, location needs a function to get a single location from the db by id first
                false => None,
            },
            is_info_message: message.is_info(),
            show_padlock: message.get_showpadlock(),
        }
    }
}

pub async fn export_chat(context: &Context, chat_id: ChatId) -> ExportChatResult {
    let mut blobs = Vec::new();
    let mut chat_author_ids = Vec::new();
    // get all messages

    let message_futures = get_chat_msgs(context, chat_id, 0, None)
        .await
        .into_iter()
        .map(|msg_id| Message::load_from_db(context, msg_id))
        .collect::<Vec<_>>();
    let messages: Vec<std::result::Result<Message, anyhow::Error>> =
        join_all(message_futures).await;
    // push all referenced blobs and populate contactid list
    for message in &messages {
        if let Ok(msg) = &message {
            let filename = msg.get_filename();
            if let Some(file) = filename {
                // push referenced blobs (attachments)
                blobs.push(file);
            }
            chat_author_ids.push(msg.from_id);
        }
    }
    // deduplicate contact list and load the contacts
    chat_author_ids.dedup();
    // load information about the authors
    let mut chat_authors: HashMap<u32, ContactJSON> = HashMap::new();
    chat_authors.insert(
        0,
        ContactJSON {
            name: "Err: Contact not found".to_owned(),
            email: "error@localhost".to_owned(),
            profile_img: None,
            color: "grey".to_owned(),
        },
    );
    for author_id in chat_author_ids {
        let contact = Contact::get_by_id(context, author_id).await;
        if let Ok(c) = contact {
            let profile_img_path: String;
            if let Some(path) = c.get_profile_image(context).await {
                profile_img_path = path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new(""))
                    .to_str()
                    .unwrap()
                    .to_owned();
                // push referenced blobs (avatars)
                blobs.push(profile_img_path.clone());
            } else {
                profile_img_path = "".to_owned();
            }
            chat_authors.insert(
                author_id,
                ContactJSON {
                    name: c.get_display_name().to_owned(),
                    email: c.get_addr().to_owned(),
                    profile_img: match profile_img_path != "" {
                        true => Some(profile_img_path),
                        false => None,
                    },
                    color: format!("{:#}", c.get_color()), // TODO
                },
            );
        }
    }

    // Load information about the chat
    let chat: Chat = Chat::load_from_db(context, chat_id).await.unwrap();
    let chat_avatar = match chat.get_profile_image(context).await {
        Some(img) => {
            let path = img
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(""))
                .to_str()
                .unwrap()
                .to_owned();
            blobs.push(path.clone());
            Some(format!("blobs/{}", path))
        }
        None => None,
    };

    let mut message_info: Vec<(
        u32,
        String,         /* message info */
        Option<String>, /* mime headers */
    )> = Vec::new();
    let mut message_json: Vec<MessageJSON> = Vec::new();

    for message in &messages {
        if let Ok(msg) = &message {
            let msg_json: MessageJSON = MessageJSON::from_message(msg, &context).await;
            message_json.push(msg_json);
            message_info.push((
                msg.id.to_u32(),
                get_msg_info(&context, msg.id).await,
                get_mime_headers(&context, msg.id).await,
            ));
        } else {
            // todo
        }
    }

    let chat_json = ChatJSON {
        chat_json_version: 1,
        name: chat.get_name().to_owned(),
        color: format!("{:#}", chat.get_color(&context).await),
        profile_img: chat_avatar,
        contacts: chat_authors,
        messages: message_json,
    };

    blobs.sort();
    blobs.dedup();
    ExportChatResult {
        chat_json: serde_json::to_string(&chat_json).unwrap(),
        message_info: message_info,
        referenced_blobs: blobs,
    }
}

// use crate::dc_tools::*;
use crate::chat::*;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
use crate::contact::*;
use crate::context::Context;
use crate::error::Error;
use crate::message::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use zip::write::FileOptions;

use serde::Serialize;

#[derive(Debug)]
pub struct ExportChatResult {
    chat_json: String,
    // locations_geo_json: String,
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

    zip.finish()?;
    Ok(())
}

#[derive(Serialize)]
struct ChatJSON {
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
    filesize: String, /* todo human readable file size*/
    extension: String,
}

#[derive(Serialize)]
enum MessageJSON {
    Message {
        id: u32,
        author_id: u32, // from_id
        viewType: Viewtype,
        timestamp_sort: i64,
        timestamp_sent: i64,
        timestamp_rcvd: i64,
        text: Option<String>,
        attachment: Option<FileReference>,
        // location
    }, // Info Message?
}

impl MessageJSON {
    pub fn from_message(message: Message, context: &Context) -> MessageJSON {}
}

pub async fn export_chat(context: &Context, chat_id: ChatId) -> ExportChatResult {
    let mut blobs = Vec::new();
    let mut chat_author_ids = Vec::new();
    // get all messages
    let messages: Vec<std::result::Result<Message, Error>> =
        get_chat_msgs(context, chat_id, 0, None)
            .await
            .into_iter()
            .map(async move |msg_id| Message::load_from_db(context, msg_id).await)
            .collect();
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

    let chat_json = ChatJSON {
        name: chat.get_name(),
        color: format!("{:#}", chat.get_color()),
        profile_img: chat_avatar,
        contacts: chat_authors,
        messages: vec![], //todo
    };

    blobs.dedup();
    ExportChatResult {
        chat_json: serde_json::to_string(&chat_json).unwrap(),
        referenced_blobs: blobs,
    }
}

// fn message_to_html(
//     author_cache: &HashMap<u32, ContactInfo>,
//     message: Message,
//     context: &Context,
// ) -> String {
//     let author: &ContactInfo = {
//         if let Some(c) = author_cache.get(&message.get_from_id()) {
//             c
//         } else {
//             author_cache.get(&0).unwrap()
//         }
//     };

//     let avatar: String = {
//         if let Some(profile_img) = &author.profile_img {
//             format!(
//                 "<div class=\"author-avatar\">\
//                  <img \
//                  alt=\"{author_name}\"\
//                  src=\"blobs/{author_avatar_src}\"\
//                  />\
//                  </div>",
//                 author_name = author.name,
//                 author_avatar_src = profile_img
//             )
//         } else {
//             format!(
//                 "<div class=\"author-avatar default\" alt=\"{name}\">\
//                  <div class=\"label\" style=\"background-color: {color}\">\
//                  {initial}\
//                  </div>\
//                  </div>",
//                 name = author.name,
//                 initial = author.initial,
//                 color = author.color
//             )
//         }
//     };

//     // save and refernce message source code somehow?

//     let has_text = message.get_text().is_some() && !message.get_text().unwrap().is_empty();

//     let attachment = match message.get_file(context) {
//         None => "".to_owned(),
//         Some(file) => {
//             let modifier_class = if has_text { "content-below" } else { "" };
//             let filename = file
//                 .file_name()
//                 .unwrap_or_else(|| std::ffi::OsStr::new(""))
//                 .to_str()
//                 .unwrap()
//                 .to_owned();
//             match message.get_viewtype() {
//                 Viewtype::Audio => {
//                     format!("<audio \
//                     controls \
//                     class=\"message-attachment-audio {}\"> \
//                     <source src=\"blobs/{}\" /> \
//                   </audio>", modifier_class ,filename)
//                 },
//                 Viewtype::Gif | Viewtype::Image | Viewtype::Sticker => {
//                     format!("<a \
//                         href=\"blobs/{filename}\" \
//                         role=\"button\" \
//                         class=\"message-attachment-media {modifier_class}\"> \
//                         <img className='attachment-content' src=\"blobs/{filename}\" /> \
//                     </a>", modifier_class=modifier_class, filename=filename)
//                 },
//                 Viewtype::Video => {
//                     format!("<a \
//                     href=\"blobs/{filename}\" \
//                     role=\"button\" \
//                     class=\"message-attachment-media {modifier_class}\"> \
//                     <video className='attachment-content' src=\"blobs/{filename}\" controls=\"true\" /> \
//                 </a>", modifier_class=modifier_class, filename=filename)
//                 },
//                 _ => {
//                     format!("<div class=\"message-attachment-generic {modifier_class}\">\
//                         <div class=\"file-icon\">\
//                             <div class=\"file-extension\">\
//                             {extension} \
//                             </div>\
//                         </div>\
//                         <div className=\"text-part\">\
//                         <a href=\"blobs/{filename}\" className=\"name\">{filename}</a>\
//                         <div className=\"size\">{filesize}</div>\
//                         </div>\
//                     </div>",
//                     modifier_class=modifier_class,
//                     filename=filename,
//                     filesize=message.get_filebytes(&context) /* todo human readable file size*/,
//                     extension=file.extension().unwrap_or_else(|| std::ffi::OsStr::new("")).to_str().unwrap().to_owned())
//                 }
//             }
//         }
//     };

//     format!(
//         "<li>\
//          <div class=\"message {direction}\">\
//          {avatar}\
//          <div class=\"msg-container\">\
//          <span class=\"author\" style=\"color: {author_color};\">{author_name}</span>\
//          <div class=\"msg-body\">\
//          {attachment}
//          <div dir=\"auto\" class=\"text\">\
//          {content}\
//          </div>\
//          <div class=\"metadata {with_image_no_caption}\">\
//          {encryption}\
//          <span class=\"date date--{direction}\" title=\"{full_time}\">{relative_time}</span>\
//          <span class=\"spacer\"></span>\
//          </div>\
//          </div>\
//          </div>\
//          <div>\
//          </li>",
//         direction = match message.from_id == DC_CONTACT_ID_SELF {
//             true => "outgoing",
//             false => "incoming",
//         },
//         avatar = avatar,
//         author_name = author.name,
//         author_color = author.color,
//         attachment = attachment,
//         content = message.get_text().unwrap_or_else(|| "".to_owned()),
//         with_image_no_caption = if !has_text && message.get_viewtype() == Viewtype::Image {
//             "with-image-no-caption"
//         } else {
//             ""
//         },
//         encryption = match message.get_showpadlock() {
//             true => r#"<div aria-label="Encryption padlock" class="padlock-icon"></div>"#,
//             false => "",
//         },
//         full_time = "Tue, Feb 25, 2020 3:49 PM", // message.get_timestamp() ? // todo
//         relative_time = "Tue 3:49 PM"            // todo
//     )

//     // todo link to raw message data
//     // todo link to message info
// }

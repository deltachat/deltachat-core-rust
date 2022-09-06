use anyhow::{anyhow, Result};
use deltachat::chat::get_chat_contacts;
use deltachat::chat::{Chat, ChatId};
use deltachat::constants::Chattype;
use deltachat::contact::{Contact, ContactId};
use deltachat::context::Context;
use num_traits::cast::ToPrimitive;
use serde::Serialize;
use typescript_type_def::TypeDef;

use super::color_int_to_hex_string;
use super::contact::ContactObject;

#[derive(Serialize, TypeDef)]
#[serde(rename_all = "camelCase")]
pub struct FullChat {
    id: u32,
    name: String,
    is_protected: bool,
    profile_image: Option<String>, //BLOBS ?
    archived: bool,
    // subtitle  - will be moved to frontend because it uses translation functions
    chat_type: u32,
    is_unpromoted: bool,
    is_self_talk: bool,
    contacts: Vec<ContactObject>,
    contact_ids: Vec<u32>,
    color: String,
    fresh_message_counter: usize,
    // is_group - please check over chat.type in frontend instead
    is_contact_request: bool,
    is_device_chat: bool,
    self_in_group: bool,
    is_muted: bool,
    ephemeral_timer: u32, //TODO look if there are more important properties in newer core versions
    can_send: bool,
    was_seen_recently: bool,
}

impl FullChat {
    pub async fn try_from_dc_chat_id(context: &Context, chat_id: u32) -> Result<Self> {
        let rust_chat_id = ChatId::new(chat_id);
        let chat = Chat::load_from_db(context, rust_chat_id).await?;

        let contact_ids = get_chat_contacts(context, rust_chat_id).await?;

        let mut contacts = Vec::with_capacity(contact_ids.len());

        for contact_id in &contact_ids {
            contacts.push(
                ContactObject::try_from_dc_contact(
                    context,
                    Contact::load_from_db(context, *contact_id).await?,
                )
                .await?,
            )
        }

        let profile_image = match chat.get_profile_image(context).await? {
            Some(path_buf) => path_buf.to_str().map(|s| s.to_owned()),
            None => None,
        };

        let color = color_int_to_hex_string(chat.get_color(context).await?);
        let fresh_message_counter = rust_chat_id.get_fresh_msg_cnt(context).await?;
        let ephemeral_timer = rust_chat_id.get_ephemeral_timer(context).await?.to_u32();

        let can_send = chat.can_send(context).await?;

        let was_seen_recently = if chat.get_type() == Chattype::Single {
            match contact_ids.get(0) {
                Some(contact) => Contact::load_from_db(context, *contact)
                    .await?
                    .was_seen_recently(),
                None => false,
            }
        } else {
            false
        };

        Ok(FullChat {
            id: chat_id,
            name: chat.name.clone(),
            is_protected: chat.is_protected(),
            profile_image, //BLOBS ?
            archived: chat.get_visibility() == deltachat::chat::ChatVisibility::Archived,
            chat_type: chat
                .get_type()
                .to_u32()
                .ok_or_else(|| anyhow!("unknown chat type id"))?, // TODO get rid of this unwrap?
            is_unpromoted: chat.is_unpromoted(),
            is_self_talk: chat.is_self_talk(),
            contacts,
            contact_ids: contact_ids.iter().map(|id| id.to_u32()).collect(),
            color,
            fresh_message_counter,
            is_contact_request: chat.is_contact_request(),
            is_device_chat: chat.is_device_talk(),
            self_in_group: contact_ids.contains(&ContactId::SELF),
            is_muted: chat.is_muted(),
            ephemeral_timer,
            can_send,
            was_seen_recently,
        })
    }
}

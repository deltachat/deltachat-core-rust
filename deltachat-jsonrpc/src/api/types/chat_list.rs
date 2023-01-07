use anyhow::{Context, Result};
use deltachat::chatlist::get_chatlistitem_for_chat;
use deltachat::constants::*;
use deltachat::contact::{Contact, ContactId};
use deltachat::{
    chat::{get_chat_contacts, ChatVisibility},
    chatlist::Chatlist,
};
use deltachat::{
    chat::{Chat, ChatId},
};
use num_traits::cast::ToPrimitive;
use serde::{Serialize};
use typescript_type_def::TypeDef;

use super::color_int_to_hex_string;

#[derive(Serialize, TypeDef)]
#[serde(tag = "type")]
pub enum ChatListItemFetchResult {
    #[serde(rename_all = "camelCase")]
    ChatListItem {
        id: u32,
        name: String,
        avatar_path: Option<String>,
        color: String,
        last_updated: Option<i64>,
        summary_text1: String,
        summary_text2: String,
        summary_status: u32,
        is_protected: bool,
        is_group: bool,
        fresh_message_counter: usize,
        is_self_talk: bool,
        is_device_talk: bool,
        is_sending_location: bool,
        is_self_in_group: bool,
        is_archived: bool,
        is_pinned: bool,
        is_muted: bool,
        is_contact_request: bool,
        /// true when chat is a broadcastlist
        is_broadcast: bool,
        /// contact id if this is a dm chat (for view profile entry in context menu)
        dm_chat_contact: Option<u32>,
        was_seen_recently: bool,
    },
    #[serde(rename_all = "camelCase")]
    ArchiveLink { fresh_message_counter: usize },
    #[serde(rename_all = "camelCase")]
    Error { id: u32, error: String },
}

pub(crate) async fn get_chat_list_item_by_id(
    ctx: &deltachat::context::Context,
    entry: u32,
) -> Result<ChatListItemFetchResult> {
    let chat_id = ChatId::new(entry);

    let (_, last_msgid) = get_chatlistitem_for_chat(&ctx, chat_id).await?;

    let fresh_message_counter = chat_id.get_fresh_msg_cnt(ctx).await?;

    if chat_id.is_archived_link() {
        return Ok(ChatListItemFetchResult::ArchiveLink {
            fresh_message_counter,
        });
    }

    let chat = Chat::load_from_db(ctx, chat_id).await.context("chat:")?;
    let summary = Chatlist::get_summary2(ctx, chat_id, last_msgid, Some(&chat))
        .await
        .context("summary:")?;

    let summary_text1 = summary.prefix.map_or_else(String::new, |s| s.to_string());
    let summary_text2 = summary.text.to_owned();

    let visibility = chat.get_visibility();

    let avatar_path = chat
        .get_profile_image(ctx)
        .await?
        .map(|path| path.to_str().unwrap_or("invalid/path").to_owned());

    let last_updated = match last_msgid {
        Some(id) => {
            let last_message = deltachat::message::Message::load_from_db(ctx, id).await?;
            Some(last_message.get_timestamp() * 1000)
        }
        None => None,
    };

    let chat_contacts = get_chat_contacts(ctx, chat_id).await?;

    let self_in_group = chat_contacts.contains(&ContactId::SELF);

    let (dm_chat_contact, was_seen_recently) = if chat.get_type() == Chattype::Single {
        let contact = chat_contacts.get(0);
        let was_seen_recently = match contact {
            Some(contact) => Contact::load_from_db(ctx, *contact)
                .await
                .context("contact:")?
                .was_seen_recently(),
            None => false,
        };
        (
            contact.map(|contact_id| contact_id.to_u32()),
            was_seen_recently,
        )
    } else {
        (None, false)
    };

    let color = color_int_to_hex_string(chat.get_color(ctx).await?);

    Ok(ChatListItemFetchResult::ChatListItem {
        id: chat_id.to_u32(),
        name: chat.get_name().to_owned(),
        avatar_path,
        color,
        last_updated,
        summary_text1,
        summary_text2,
        summary_status: summary.state.to_u32().expect("impossible"), // idea and a function to transform the constant to strings? or return string enum
        is_protected: chat.is_protected(),
        is_group: chat.get_type() == Chattype::Group,
        fresh_message_counter,
        is_self_talk: chat.is_self_talk(),
        is_device_talk: chat.is_device_talk(),
        is_self_in_group: self_in_group,
        is_sending_location: chat.is_sending_locations(),
        is_archived: visibility == ChatVisibility::Archived,
        is_pinned: visibility == ChatVisibility::Pinned,
        is_muted: chat.is_muted(),
        is_contact_request: chat.is_contact_request(),
        is_broadcast: chat.get_type() == Chattype::Broadcast,
        dm_chat_contact,
        was_seen_recently,
    })
}

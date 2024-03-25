use std::fmt::format;

use anyhow::Result;
use futures::executor::block_on;

use crate::{chat::ChatId, contact::ContactId, context::Context, EventType};

/// order or content of chatlist changes (chat ids, not the actual chatlist item)
pub(crate) fn emit_chatlist_changed(context: &Context) {
    context.emit_event(EventType::ChatlistChanged);
}

/// Chatlist item of a specific chat changed
pub(crate) fn emit_chatlist_item_changed(context: &Context, chat_id: ChatId) {
    context.emit_event(EventType::ChatlistItemChanged {
        chat_id: Some(chat_id),
    });
}

/// Used when you don't know which chatlist items changed, this reloads all cached chatlist items in the UI
///
/// Avoid calling this when you can find out the affected chat ids easialy (without extra expensive db queries).
///
/// This method is not public, so you have to define and document your new case here in this file.
fn emit_unknown_chatlist_items_changed(context: &Context) {
    context.emit_event(EventType::ChatlistItemChanged { chat_id: None });
}

/// update event for the 1:1 chat with the contact
/// used when recently seen changes and when profile image changes
pub(crate) async fn emit_chatlist_item_changed_for_contact_chat(
    context: &Context,
    contact_id: ContactId,
) {
    match ChatId::lookup_by_contact(context, contact_id).await {
        Ok(Some(chat_id)) => self::emit_chatlist_item_changed(context, chat_id),
        Ok(None) => {}
        Err(error) => context.emit_event(EventType::Error(format!(
            "failed to find chat id for contact for chatlist event: {error:?}"
        ))),
    }
}

/// update items for chats that have the contact
/// used when contact changes their name or did AEAP for example
///
/// The most common case is that the contact changed their name
/// and their name should be updated in the chatlistitems for the chats
/// where they sent the last message as there their name is shown in the summary on those
pub(crate) fn emit_chatlist_items_changed_for_contact(context: &Context, _contact_id: ContactId) {
    // note:(treefit): it is too expensive to find the right chats
    // so we'll just tell ui to reload every loaded item
    emit_unknown_chatlist_items_changed(context)
    // note:(treefit): in the future we could instead emit an extra event for this and also store contact id in the chatlistitems
    // (contact id for dm chats and contact id of contact that wrote the message in the summary)
    // the ui could then look for this info in the cache and only reload the needed chats.
}

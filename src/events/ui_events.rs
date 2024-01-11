use crate::{
    chat::{ChatId, ChatIdBlocked},
    contact::{Contact, ContactId},
    context::Context,
    EventType,
};
use async_channel::{self as channel, Receiver, Sender};
use futures::executor::block_on;
use tokio::time::Duration;
use tokio::{
    task,
    time::{sleep_until, Instant},
};

/// order or content of chatlist changes (chat ids, not the actual chatlist item)
pub(crate) fn emit_chatlist_changed(context: &Context) {
    context.emit_event(EventType::UIChatListChanged);
}

/// Chatlist item of a specific chat changed
pub(crate) fn emit_chatlist_item_changed(context: &Context, chat_id: ChatId) {
    context.emit_event(EventType::UIChatListItemChanged {
        chat_id: Some(chat_id),
    });
}

#[allow(unused)]
/// Used when you don't know which chatlist items changed, this reloads all cached chatlist items in the UI
/// note(treefit): This is not used right now, but I know there will be a point where someone wants it
pub(crate) fn emit_unknown_chatlist_items_changed(context: &Context) {
    context.emit_event(EventType::UIChatListItemChanged { chat_id: None });
}

/// update event for dm chat of contact
/// used when recently seen changes and when profile image changes
pub(crate) fn emit_chatlist_item_changed_for_contacts_dm_chat(
    context: &Context,
    contact_id: ContactId,
) {
    block_on(async {
        if let Ok(Some(chat_id)) = ChatId::lookup_by_contact(context, contact_id).await {
            self::emit_chatlist_item_changed(context, chat_id);
        }
    });
}

/// update dm for chats that have the contact
/// used when contact changes their name or did AEAP for example
pub(crate) fn emit_chatlist_items_changed_for_contact(context: &Context, contact_id: ContactId) {
    // note:(treefit): could make sense to only update chats where the last message is from the contact, but the db query for that is more expensive
    block_on(async {
        if let Ok(chat_ids) = Contact::get_chats_with_contact(context, &contact_id).await {
            for chat_id in chat_ids {
                self::emit_chatlist_item_changed(context, chat_id);
            }
        }
    });
}

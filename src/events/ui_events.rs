use std::{sync::Arc, task::Poll};

use crate::{
    chat::ChatId,
    contact::{Contact, ContactId},
    context::Context,
    EventType,
};
use async_channel::{self as channel, Receiver, Sender};
use channel::{bounded, TrySendError};
use futures::{executor::block_on, Future};
use tokio::sync::RwLock;

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

/// how many items can be pending before all visiable items are updated
const THRESHOLD_UNTIL_REFRESH_ALL: usize = 20;

struct UIEventsStateInner {
    list_changed: bool,
    all_items_changed: bool,
    updated_items_queu_sender: Sender<ChatId>,
    updated_items_queu_receiver: Receiver<ChatId>,
    // this variable makes polling simpler & faster, but makes coding harder because it can be forgotten
    has_updates: bool,
}

struct UIEventsState {
    inner: Arc<RwLock<UIEventsStateInner>>,
}

impl Future for UIEventsState {
    type Output = EventType;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.inner.blocking_read().has_updates {
            if let Some(event) = { self.inner.blocking_write().get_next() } {
                Poll::Ready(event)
            } else {
                Poll::Pending
            }
        } else {
            Poll::Pending
        }
    }
}

impl UIEventsStateInner {
    pub(crate) fn new() -> Self {
        let (server, receiver) = bounded(THRESHOLD_UNTIL_REFRESH_ALL);

        UIEventsStateInner {
            list_changed: false,
            all_items_changed: false,
            updated_items_queu_sender: server,
            updated_items_queu_receiver: receiver,
            has_updates: false,
        }
    }

    fn get_next(&mut self) -> Option<EventType> {
        if self.list_changed {
            self.list_changed = false;
            Some(EventType::UIChatListChanged)
        } else if self.all_items_changed {
            self.all_items_changed = false;
            Some(EventType::UIChatListItemChanged { chat_id: None })
        } else if let Ok(id) = self.updated_items_queu_receiver.try_recv() {
            Some(EventType::UIChatListItemChanged { chat_id: Some(id) })
        } else {
            self.has_updates = false;
            None
        }
    }

    pub(crate) fn chatlist_changed(&mut self) {
        self.list_changed = true;
        self.has_updates = true;
    }

    fn empty_updated_items_queu(&mut self) {
        if !self.updated_items_queu_receiver.is_empty() {
            while self.updated_items_queu_receiver.try_recv().is_ok() {}
            // TODO: is it more efficient to recreate the channel? or some faster way to drain
        }
    }

    pub(crate) fn unknown_chatlist_items_changed(&mut self) {
        self.empty_updated_items_queu();
        self.all_items_changed = true;
        self.has_updates = true;
    }

    pub(crate) fn unknown_chatlist_item_changed(&mut self, chat_id: ChatId) {
        if let Err(TrySendError::Full(_)) = self.updated_items_queu_sender.try_send(chat_id) {
            self.empty_updated_items_queu();
            self.all_items_changed = true;
        }
        self.has_updates = true;
    }
}

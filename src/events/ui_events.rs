use crate::{
    chat::{ChatId, ChatIdBlocked},
    constants::UI_EVENTS_TICK_RATE,
    contact::{Contact, ContactId},
    context::Context,
    EventType,
};
use async_channel::{self as channel, Receiver, Sender};
use tokio::{
    task,
    time::{sleep_until, Instant},
};

/// order or content of chatlist changes (chat ids, not the actual chatlist item)
pub(crate) fn emit_chatlist_changed(context: &Context) {
    context
        .ui_events
        .blocking_lock()
        .send_chat_list_event(context, InternalUIEvent::ChatListChanged)
}

/// Chatlist item of a specific chat changed
pub(crate) fn emit_chatlist_item_changed(context: &Context, chat_id: ChatId) {
    context
        .ui_events
        .blocking_lock()
        .send_chat_list_event(context, InternalUIEvent::ChatListItemChanged(chat_id))
}

#[allow(unused)]
/// Used when you don't know which chatlist items changed, this reloads all cached chatlist items in the UI
/// note(treefit): This is not used right now, but I know there will be a point where someone wants it
pub(crate) fn emit_unknown_chatlist_items_changed(context: &Context) {
    context
        .ui_events
        .blocking_lock()
        .send_chat_list_event(context, InternalUIEvent::UnknownChatListItemsChanged)
}

/// update event for dm chat of contact
/// used when recently seen changes and when profile image changes
pub(crate) fn emit_chatlist_item_changed_for_contacts_dm_chat(
    context: &Context,
    contact_id: ContactId,
) {
    context
        .ui_events
        .blocking_lock()
        .send_chat_list_event(context, InternalUIEvent::ContactDMChatChanged(contact_id))
}

/// update dm for chats that have the contact
/// used when contact changes their name or did AEAP for example
pub(crate) fn emit_chatlist_items_changed_for_contact(context: &Context, contact_id: ContactId) {
    context
        .ui_events
        .blocking_lock()
        .send_chat_list_event(context, InternalUIEvent::ContactChatsChanged(contact_id));
}

#[derive(Debug)]
pub(crate) enum InternalUIEvent {
    ChatListChanged,
    ChatListItemChanged(ChatId),
    UnknownChatListItemsChanged,
    ContactDMChatChanged(ContactId),
    ContactChatsChanged(ContactId),
}

struct EventLoopTickState {
    chat_list_changed: bool,
    has_unknown_items: bool,
    chat_ids: Vec<ChatId>,
    contact_ids_dm: Vec<ContactId>,
    contact_ids_chats: Vec<ContactId>,
}

impl EventLoopTickState {
    fn new(capacity: usize) -> Self {
        Self {
            chat_list_changed: false,
            has_unknown_items: false,
            chat_ids: Vec::with_capacity(capacity),
            contact_ids_dm: Vec::with_capacity(capacity),
            contact_ids_chats: Vec::with_capacity(capacity),
        }
    }

    fn apply_internal_ui_event(&mut self, event: InternalUIEvent) {
        match event {
            InternalUIEvent::ChatListChanged => {
                self.chat_list_changed = true;
            }
            InternalUIEvent::ChatListItemChanged(chat_id) => {
                self.chat_ids.push(chat_id);
            }
            InternalUIEvent::UnknownChatListItemsChanged => {
                self.has_unknown_items = true;
            }
            InternalUIEvent::ContactDMChatChanged(contact_id) => {
                self.contact_ids_dm.push(contact_id);
            }
            InternalUIEvent::ContactChatsChanged(contact_id) => {
                self.contact_ids_chats.push(contact_id);
            }
        }
    }

    async fn emit_chatlist_ui_events(&mut self, context: &Context) {
        if self.chat_list_changed {
            context.emit_event(EventType::UIChatListChanged);
        }
        if self.has_unknown_items {
            context.emit_event(EventType::UIChatListItemChanged { chat_id: None });
            return; // since this refreshes everything no further events are needed
        }

        for contact_id in self
            .contact_ids_dm
            .iter()
            .filter(|contact| !self.contact_ids_chats.contains(contact))
            .collect::<Vec<&ContactId>>()
        {
            if let Ok(Some(chat_id)) = ChatIdBlocked::lookup_by_contact(context, *contact_id).await
            {
                self.chat_ids.push(chat_id.id)
            }
        }

        // note:(treefit): could make sense to only update chats where the last message is from the contact, but the db query for that is more expensive
        for contact_id in &self.contact_ids_chats {
            match Contact::get_chats_with_contact(context, contact_id).await {
                Ok(contacts_chat_ids) => {
                    self.chat_ids.extend(contacts_chat_ids);
                }
                Err(err) => {
                    warn!(
                        context,
                        "Error while getting chats for contact {} in chatlist events loop: {}",
                        contact_id,
                        err
                    );
                }
            }
        }

        self.chat_ids.sort();
        self.chat_ids.dedup();

        // TODO change event so it accepts a list of chat ids to get rid of this loop? wouldn't work with cffi unless we give it out as json
        for chat_id in &self.chat_ids {
            context.emit_event(EventType::UIChatListItemChanged {
                chat_id: Some(*chat_id),
            })
        }
    }
}

/// Debounces UI events
#[derive(Debug)]
pub(crate) struct UIEvents {
    task_handle: Option<task::JoinHandle<()>>,
    chatlist_event_queue: Sender<InternalUIEvent>,
}

impl UIEvents {
    pub(crate) fn new() -> (Self, Receiver<InternalUIEvent>) {
        let (chatlist_event_queue, chatlist_event_queue_recv) = channel::unbounded();
        (
            Self {
                task_handle: None,
                chatlist_event_queue,
            },
            chatlist_event_queue_recv,
        )
    }

    pub(crate) fn start(
        &mut self,
        context: &Context,
        chatlist_event_queue_recv: Receiver<InternalUIEvent>,
    ) {
        if let Some(handle) = self.task_handle {
            handle.abort()
        }
        self.task_handle = Some(task::spawn(Self::run_task(
            context,
            chatlist_event_queue_recv,
        )))
    }

    async fn run_task(context: &Context, chatlist_event_queue: Receiver<InternalUIEvent>) {
        loop {
            match chatlist_event_queue.recv().await {
                Ok(chatlist_event) => {
                    let backlog_len = chatlist_event_queue.len();
                    let mut tick_state = EventLoopTickState::new(backlog_len);

                    tick_state.apply_internal_ui_event(chatlist_event);
                    // get all events from the queue
                    while let Ok(event) = chatlist_event_queue.try_recv() {
                        tick_state.apply_internal_ui_event(event);
                    }

                    tick_state.emit_chatlist_ui_events(context).await;

                    // cooldown
                    sleep_until(Instant::now() + UI_EVENTS_TICK_RATE).await;
                }
                Err(err) => {
                    warn!(
                        context,
                        "Error receiving an interruption in ui chatlist events loop: {}", err
                    );
                    // Maybe the sender side is closed, so terminate the loop to avoid looping indefinitely.
                    return;
                }
            }
        }
    }

    pub(crate) fn send_chat_list_event(&self, context: &Context, event: InternalUIEvent) {
        // todo check if ui events are enabled?
        if let Err(error) = self.chatlist_event_queue.try_send(event) {
            warn!(
                context,
                "Error receiving an interruption in ui chatlist events loop: {}", error
            );
        }
    }
}

impl Drop for UIEvents {
    fn drop(&mut self) {
        if let Some(handle) = &self.task_handle {
            handle.abort()
        }
    }
}

#[cfg(test)]
mod test {

    // todo tests:

    // send ui events though the UIEventsLoop

    // check that UIEventsLoop really ratelimits the events

    // check that has_unknown_items does not send out any ids before or afterwards

    // if we should make it possible to disable via config then test that as well
}

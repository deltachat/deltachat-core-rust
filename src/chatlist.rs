//! # Chat list module

use crate::chat;
use crate::chat::*;
use crate::constants::*;
use crate::contact::*;
use crate::context::*;
use crate::error::{bail, ensure, Result};
use crate::lot::Lot;
use crate::message::{Message, MessageState, MsgId};
use crate::stock::StockMessage;

/// An object representing a single chatlist in memory.
///
/// Chatlist objects contain chat IDs and, if possible, message IDs belonging to them.
/// The chatlist object is not updated; if you want an update, you have to recreate the object.
///
/// For a **typical chat overview**, the idea is to get the list of all chats via dc_get_chatlist()
/// without any listflags (see below) and to implement a "virtual list" or so
/// (the count of chats is known by chatlist.len()).
///
/// Only for the items that are in view (the list may have several hundreds chats),
/// the UI should call chatlist.get_summary() then.
/// chatlist.get_summary() provides all elements needed for painting the item.
///
/// On a click of such an item, the UI should change to the chat view
/// and get all messages from this view via dc_get_chat_msgs().
/// Again, a "virtual list" is created (the count of messages is known)
/// and for each messages that is scrolled into view, dc_get_msg() is called then.
///
/// Why no listflags?
/// Without listflags, dc_get_chatlist() adds the deaddrop and the archive "link" automatically as needed.
/// The UI can just render these items differently then. Although the deaddrop link is currently always the
/// first entry and only present on new messages, there is the rough idea that it can be optionally always
/// present and sorted into the list by date. Rendering the deaddrop in the described way
/// would not add extra work in the UI then.
#[derive(Debug)]
pub struct Chatlist {
    /// Stores pairs of `chat_id, message_id`
    ids: Vec<(ChatId, MsgId)>,
}

impl Chatlist {
    /// Get a list of chats.
    /// The list can be filtered by query parameters.
    ///
    /// The list is already sorted and starts with the most recent chat in use.
    /// The sorting takes care of invalid sending dates, drafts and chats without messages.
    /// Clients should not try to re-sort the list as this would be an expensive action
    /// and would result in inconsistencies between clients.
    ///
    /// To get information about each entry, use eg. chatlist.get_summary().
    ///
    /// By default, the function adds some special entries to the list.
    /// These special entries can be identified by the ID returned by chatlist.get_chat_id():
    /// - DC_CHAT_ID_DEADDROP (1) - this special chat is present if there are
    ///   messages from addresses that have no relationship to the configured account.
    ///   The last of these messages is represented by DC_CHAT_ID_DEADDROP and you can retrieve details
    ///   about it with chatlist.get_msg_id(). Typically, the UI asks the user "Do you want to chat with NAME?"
    ///   and offers the options "Yes" (call dc_create_chat_by_msg_id()), "Never" (call dc_block_contact())
    ///   or "Not now".
    ///   The UI can also offer a "Close" button that calls dc_marknoticed_contact() then.
    /// - DC_CHAT_ID_ARCHIVED_LINK (6) - this special chat is present if the user has
    ///   archived *any* chat using dc_set_chat_visibility(). The UI should show a link as
    ///   "Show archived chats", if the user clicks this item, the UI should show a
    ///   list of all archived chats that can be created by this function hen using
    ///   the DC_GCL_ARCHIVED_ONLY flag.
    /// - DC_CHAT_ID_ALLDONE_HINT (7) - this special chat is present
    ///   if DC_GCL_ADD_ALLDONE_HINT is added to listflags
    ///   and if there are only archived chats.
    ///
    /// The `listflags` is a combination of flags:
    /// - if the flag DC_GCL_ARCHIVED_ONLY is set, only archived chats are returned.
    ///   if DC_GCL_ARCHIVED_ONLY is not set, only unarchived chats are returned and
    ///   the pseudo-chat DC_CHAT_ID_ARCHIVED_LINK is added if there are *any* archived
    ///   chats
    /// - the flag DC_GCL_FOR_FORWARDING sorts "Saved messages" to the top of the chatlist
    ///   and hides the device-chat,
    //    typically used on forwarding, may be combined with DC_GCL_NO_SPECIALS
    /// - if the flag DC_GCL_NO_SPECIALS is set, deaddrop and archive link are not added
    ///   to the list (may be used eg. for selecting chats on forwarding, the flag is
    ///   not needed when DC_GCL_ARCHIVED_ONLY is already set)
    /// - if the flag DC_GCL_ADD_ALLDONE_HINT is set, DC_CHAT_ID_ALLDONE_HINT
    ///   is added as needed.
    /// `query`: An optional query for filtering the list. Only chats matching this query
    ///     are returned.
    /// `query_contact_id`: An optional contact ID for filtering the list. Only chats including this contact ID
    ///     are returned.
    pub fn try_load(
        context: &Context,
        listflags: usize,
        query: Option<&str>,
        query_contact_id: Option<u32>,
    ) -> Result<Self> {
        // Note that we do not emit DC_EVENT_MSGS_MODIFIED here even if some
        // messages get deleted to avoid reloading the same chatlist.
        if let Err(err) = delete_device_expired_messages(context) {
            warn!(context, "Failed to hide expired messages: {}", err);
        }

        let mut add_archived_link_item = false;

        let process_row = |row: &rusqlite::Row| {
            let chat_id: ChatId = row.get(0)?;
            let msg_id: MsgId = row.get(1).unwrap_or_default();
            Ok((chat_id, msg_id))
        };

        let process_rows = |rows: rusqlite::MappedRows<_>| {
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        };

        let skip_id = if 0 != listflags & DC_GCL_FOR_FORWARDING {
            chat::lookup_by_contact_id(context, DC_CONTACT_ID_DEVICE)
                .unwrap_or_default()
                .0
        } else {
            ChatId::new(0)
        };

        // select with left join and minimum:
        //
        // - the inner select must use `hidden` and _not_ `m.hidden`
        //   which would refer the outer select and take a lot of time
        // - `GROUP BY` is needed several messages may have the same
        //   timestamp
        // - the list starts with the newest chats
        //
        // nb: the query currently shows messages from blocked
        // contacts in groups.  however, for normal-groups, this is
        // okay as the message is also returned by dc_get_chat_msgs()
        // (otherwise it would be hard to follow conversations, wa and
        // tg do the same) for the deaddrop, however, they should
        // really be hidden, however, _currently_ the deaddrop is not
        // shown at all permanent in the chatlist.
        let mut ids = if let Some(query_contact_id) = query_contact_id {
            // show chats shared with a given contact
            context.sql.query_map(
                "SELECT c.id, m.id
                 FROM chats c
                 LEFT JOIN msgs m
                        ON c.id=m.chat_id
                       AND m.timestamp=(
                               SELECT MAX(timestamp)
                                 FROM msgs
                                WHERE chat_id=c.id
                                  AND (hidden=0 OR state=?1))
                 WHERE c.id>9
                   AND c.blocked=0
                   AND c.id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?2)
                 GROUP BY c.id
                 ORDER BY c.archived=?3 DESC, IFNULL(m.timestamp,c.created_timestamp) DESC, m.id DESC;",
                params![MessageState::OutDraft, query_contact_id as i32, ChatVisibility::Pinned],
                process_row,
                process_rows,
            )?
        } else if 0 != listflags & DC_GCL_ARCHIVED_ONLY {
            // show archived chats
            // (this includes the archived device-chat; we could skip it,
            // however, then the number of archived chats do not match, which might be even more irritating.
            // and adapting the number requires larger refactorings and seems not to be worth the effort)
            context.sql.query_map(
                "SELECT c.id, m.id
                 FROM chats c
                 LEFT JOIN msgs m
                        ON c.id=m.chat_id
                       AND m.timestamp=(
                               SELECT MAX(timestamp)
                                 FROM msgs
                                WHERE chat_id=c.id
                                  AND (hidden=0 OR state=?))
                 WHERE c.id>9
                   AND c.blocked=0
                   AND c.archived=1
                 GROUP BY c.id
                 ORDER BY IFNULL(m.timestamp,c.created_timestamp) DESC, m.id DESC;",
                params![MessageState::OutDraft],
                process_row,
                process_rows,
            )?
        } else if let Some(query) = query {
            let query = query.trim().to_string();
            ensure!(!query.is_empty(), "missing query");

            // allow searching over special names that may change at any time
            // when the ui calls set_stock_translation()
            if let Err(err) = update_special_chat_names(context) {
                warn!(context, "cannot update special chat names: {:?}", err)
            }

            let str_like_cmd = format!("%{}%", query);
            context.sql.query_map(
                "SELECT c.id, m.id
                 FROM chats c
                 LEFT JOIN msgs m
                        ON c.id=m.chat_id
                       AND m.timestamp=(
                               SELECT MAX(timestamp)
                                 FROM msgs
                                WHERE chat_id=c.id
                                  AND (hidden=0 OR state=?1))
                 WHERE c.id>9 AND c.id!=?2
                   AND c.blocked=0
                   AND c.name LIKE ?3
                 GROUP BY c.id
                 ORDER BY IFNULL(m.timestamp,c.created_timestamp) DESC, m.id DESC;",
                params![MessageState::OutDraft, skip_id, str_like_cmd],
                process_row,
                process_rows,
            )?
        } else {
            //  show normal chatlist
            let sort_id_up = if 0 != listflags & DC_GCL_FOR_FORWARDING {
                chat::lookup_by_contact_id(context, DC_CONTACT_ID_SELF)
                    .unwrap_or_default()
                    .0
            } else {
                ChatId::new(0)
            };
            let mut ids = context.sql.query_map(
                "SELECT c.id, m.id
                 FROM chats c
                 LEFT JOIN msgs m
                        ON c.id=m.chat_id
                       AND m.timestamp=(
                               SELECT MAX(timestamp)
                                 FROM msgs
                                WHERE chat_id=c.id
                                  AND (hidden=0 OR state=?1))
                 WHERE c.id>9 AND c.id!=?2
                   AND c.blocked=0
                   AND NOT c.archived=?3
                 GROUP BY c.id
                 ORDER BY c.id=?4 DESC, c.archived=?5 DESC, IFNULL(m.timestamp,c.created_timestamp) DESC, m.id DESC;",
                params![MessageState::OutDraft, skip_id, ChatVisibility::Archived, sort_id_up, ChatVisibility::Pinned],
                process_row,
                process_rows,
            )?;
            if 0 == listflags & DC_GCL_NO_SPECIALS {
                if let Some(last_deaddrop_fresh_msg_id) = get_last_deaddrop_fresh_msg(context) {
                    if 0 == listflags & DC_GCL_FOR_FORWARDING {
                        ids.insert(
                            0,
                            (ChatId::new(DC_CHAT_ID_DEADDROP), last_deaddrop_fresh_msg_id),
                        );
                    }
                }
                add_archived_link_item = true;
            }
            ids
        };

        if add_archived_link_item && dc_get_archived_cnt(context) > 0 {
            if ids.is_empty() && 0 != listflags & DC_GCL_ADD_ALLDONE_HINT {
                ids.push((ChatId::new(DC_CHAT_ID_ALLDONE_HINT), MsgId::new(0)));
            }
            ids.push((ChatId::new(DC_CHAT_ID_ARCHIVED_LINK), MsgId::new(0)));
        }

        Ok(Chatlist { ids })
    }

    /// Find out the number of chats.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns true if chatlist is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Get a single chat ID of a chatlist.
    ///
    /// To get the message object from the message ID, use dc_get_chat().
    pub fn get_chat_id(&self, index: usize) -> ChatId {
        match self.ids.get(index) {
            Some((chat_id, _msg_id)) => *chat_id,
            None => ChatId::new(0),
        }
    }

    /// Get a single message ID of a chatlist.
    ///
    /// To get the message object from the message ID, use dc_get_msg().
    pub fn get_msg_id(&self, index: usize) -> Result<MsgId> {
        match self.ids.get(index) {
            Some((_chat_id, msg_id)) => Ok(*msg_id),
            None => bail!("Chatlist index out of range"),
        }
    }

    /// Get a summary for a chatlist index.
    ///
    /// The summary is returned by a dc_lot_t object with the following fields:
    ///
    /// - dc_lot_t::text1: contains the username or the strings "Me", "Draft" and so on.
    ///   The string may be colored by having a look at text1_meaning.
    ///   If there is no such name or it should not be displayed, the element is NULL.
    /// - dc_lot_t::text1_meaning: one of DC_TEXT1_USERNAME, DC_TEXT1_SELF or DC_TEXT1_DRAFT.
    ///   Typically used to show dc_lot_t::text1 with different colors. 0 if not applicable.
    /// - dc_lot_t::text2: contains an excerpt of the message text or strings as
    ///   "No messages".  May be NULL of there is no such text (eg. for the archive link)
    /// - dc_lot_t::timestamp: the timestamp of the message.  0 if not applicable.
    /// - dc_lot_t::state: The state of the message as one of the DC_STATE_* constants (see #dc_msg_get_state()).
    //    0 if not applicable.
    pub fn get_summary(&self, context: &Context, index: usize, chat: Option<&Chat>) -> Lot {
        // The summary is created by the chat, not by the last message.
        // This is because we may want to display drafts here or stuff as
        // "is typing".
        // Also, sth. as "No messages" would not work if the summary comes from a message.
        let mut ret = Lot::new();

        let (chat_id, lastmsg_id) = match self.ids.get(index) {
            Some(ids) => ids,
            None => {
                ret.text2 = Some("ErrBadChatlistIndex".to_string());
                return ret;
            }
        };

        let chat_loaded: Chat;
        let chat = if let Some(chat) = chat {
            chat
        } else if let Ok(chat) = Chat::load_from_db(context, *chat_id) {
            chat_loaded = chat;
            &chat_loaded
        } else {
            return ret;
        };

        let mut lastcontact = None;

        let lastmsg = if let Ok(lastmsg) = Message::load_from_db(context, *lastmsg_id) {
            if lastmsg.from_id != DC_CONTACT_ID_SELF
                && (chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup)
            {
                lastcontact = Contact::load_from_db(context, lastmsg.from_id).ok();
            }

            Some(lastmsg)
        } else {
            None
        };

        if chat.id.is_archived_link() {
            ret.text2 = None;
        } else if lastmsg.is_none() || lastmsg.as_ref().unwrap().from_id == DC_CONTACT_ID_UNDEFINED
        {
            ret.text2 = Some(context.stock_str(StockMessage::NoMessages).to_string());
        } else {
            ret.fill(&mut lastmsg.unwrap(), chat, lastcontact.as_ref(), context);
        }

        ret
    }

    pub fn get_index_for_id(&self, id: ChatId) -> Option<usize> {
        self.ids.iter().position(|(chat_id, _)| chat_id == &id)
    }
}

/// Returns the number of archived chats
pub fn dc_get_archived_cnt(context: &Context) -> u32 {
    context
        .sql
        .query_get_value(
            context,
            "SELECT COUNT(*) FROM chats WHERE blocked=0 AND archived=1;",
            params![],
        )
        .unwrap_or_default()
}

fn get_last_deaddrop_fresh_msg(context: &Context) -> Option<MsgId> {
    // We have an index over the state-column, this should be
    // sufficient as there are typically only few fresh messages.
    context.sql.query_get_value(
        context,
        concat!(
            "SELECT m.id",
            " FROM msgs m",
            " LEFT JOIN chats c",
            "        ON c.id=m.chat_id",
            " WHERE m.state=10",
            "   AND m.hidden=0",
            "   AND c.blocked=2",
            " ORDER BY m.timestamp DESC, m.id DESC;"
        ),
        params![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    #[test]
    fn test_try_load() {
        let t = dummy_context();
        let chat_id1 = create_group_chat(&t.ctx, VerifiedStatus::Unverified, "a chat").unwrap();
        let chat_id2 = create_group_chat(&t.ctx, VerifiedStatus::Unverified, "b chat").unwrap();
        let chat_id3 = create_group_chat(&t.ctx, VerifiedStatus::Unverified, "c chat").unwrap();

        // check that the chatlist starts with the most recent message
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 3);
        assert_eq!(chats.get_chat_id(0), chat_id3);
        assert_eq!(chats.get_chat_id(1), chat_id2);
        assert_eq!(chats.get_chat_id(2), chat_id1);

        // drafts are sorted to the top
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hello".to_string()));
        chat_id2.set_draft(&t.ctx, Some(&mut msg));
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.get_chat_id(0), chat_id2);

        // check chatlist query and archive functionality
        let chats = Chatlist::try_load(&t.ctx, 0, Some("b"), None).unwrap();
        assert_eq!(chats.len(), 1);

        let chats = Chatlist::try_load(&t.ctx, DC_GCL_ARCHIVED_ONLY, None, None).unwrap();
        assert_eq!(chats.len(), 0);

        chat_id1
            .set_visibility(&t.ctx, ChatVisibility::Archived)
            .ok();
        let chats = Chatlist::try_load(&t.ctx, DC_GCL_ARCHIVED_ONLY, None, None).unwrap();
        assert_eq!(chats.len(), 1);
    }

    #[test]
    fn test_sort_self_talk_up_on_forward() {
        let t = dummy_context();
        t.ctx.update_device_chats().unwrap();
        create_group_chat(&t.ctx, VerifiedStatus::Unverified, "a chat").unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert!(chats.len() == 3);
        assert!(!Chat::load_from_db(&t.ctx, chats.get_chat_id(0))
            .unwrap()
            .is_self_talk());

        let chats = Chatlist::try_load(&t.ctx, DC_GCL_FOR_FORWARDING, None, None).unwrap();
        assert!(chats.len() == 2); // device chat cannot be written and is skipped on forwarding
        assert!(Chat::load_from_db(&t.ctx, chats.get_chat_id(0))
            .unwrap()
            .is_self_talk());
    }

    #[test]
    fn test_search_special_chat_names() {
        let t = dummy_context();
        t.ctx.update_device_chats().unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, Some("t-1234-s"), None).unwrap();
        assert_eq!(chats.len(), 0);
        let chats = Chatlist::try_load(&t.ctx, 0, Some("t-5678-b"), None).unwrap();
        assert_eq!(chats.len(), 0);

        t.ctx
            .set_stock_translation(StockMessage::SavedMessages, "test-1234-save".to_string())
            .unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, Some("t-1234-s"), None).unwrap();
        assert_eq!(chats.len(), 1);

        t.ctx
            .set_stock_translation(StockMessage::DeviceMessages, "test-5678-babbel".to_string())
            .unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, Some("t-5678-b"), None).unwrap();
        assert_eq!(chats.len(), 1);
    }

    #[test]
    fn test_get_summary_unwrap() {
        let t = dummy_context();
        let chat_id1 = create_group_chat(&t.ctx, VerifiedStatus::Unverified, "a chat").unwrap();

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("foo:\nbar \r\n test".to_string()));
        chat_id1.set_draft(&t.ctx, Some(&mut msg));

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        let summary = chats.get_summary(&t.ctx, 0, None);
        assert_eq!(summary.get_text2().unwrap(), "foo: bar test"); // the linebreak should be removed from summary
    }
}

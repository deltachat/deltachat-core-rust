use crate::chat::*;
use crate::constants::*;
use crate::contact::*;
use crate::context::*;
use crate::error::Result;
use crate::lot::Lot;
use crate::message::Message;
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
    ids: Vec<(u32, u32)>,
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
    ///   archived _any_ chat using dc_archive_chat(). The UI should show a link as
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
    ///   the pseudo-chat DC_CHAT_ID_ARCHIVED_LINK is added if there are _any_ archived
    ///   chats
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
        let mut add_archived_link_item = 0;

        // select with left join and minimum:
        // - the inner select must use `hidden` and _not_ `m.hidden`
        //   which would refer the outer select and take a lot of time
        // - `GROUP BY` is needed several messages may have the same timestamp
        // - the list starts with the newest chats
        // nb: the query currently shows messages from blocked contacts in groups.
        // however, for normal-groups, this is okay as the message is also returned by dc_get_chat_msgs()
        // (otherwise it would be hard to follow conversations, wa and tg do the same)
        // for the deaddrop, however, they should really be hidden, however, _currently_ the deaddrop is not
        // shown at all permanent in the chatlist.

        let process_row = |row: &rusqlite::Row| {
            let chat_id: i32 = row.get(0)?;
            // TODO: verify that it is okay for this to be Null
            let msg_id: i32 = row.get(1).unwrap_or_default();

            Ok((chat_id as u32, msg_id as u32))
        };

        let process_rows = |rows: rusqlite::MappedRows<_>| {
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        };

        // nb: the query currently shows messages from blocked contacts in groups.
        // however, for normal-groups, this is okay as the message is also returned by dc_get_chat_msgs()
        // (otherwise it would be hard to follow conversations, wa and tg do the same)
        // for the deaddrop, however, they should really be hidden, however, _currently_ the deaddrop is not
        // shown at all permanent in the chatlist.

        let mut ids = if let Some(query_contact_id) = query_contact_id {
            // show chats shared with a given contact
            context.sql.query_map(
            "SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         \
             ON c.id=m.chat_id        \
             AND m.timestamp=( SELECT MAX(timestamp)   \
             FROM msgs  WHERE chat_id=c.id    \
             AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   \
             AND c.blocked=0 AND c.id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?)  \
             GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;",
            params![query_contact_id as i32],
            process_row,
            process_rows,
        )?
        } else if 0 != listflags & DC_GCL_ARCHIVED_ONLY {
            // show archived chats
            context.sql.query_map(
                "SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         \
                 ON c.id=m.chat_id        \
                 AND m.timestamp=( SELECT MAX(timestamp)   \
                 FROM msgs  WHERE chat_id=c.id    \
                 AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   \
                 AND c.blocked=0 AND c.archived=1  GROUP BY c.id  \
                 ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;",
                params![],
                process_row,
                process_rows,
            )?
        } else if let Some(query) = query {
            let query = query.trim().to_string();
            ensure!(!query.is_empty(), "missing query");

            let str_like_cmd = format!("%{}%", query);
            context.sql.query_map(
                "SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         \
                 ON c.id=m.chat_id        \
                 AND m.timestamp=( SELECT MAX(timestamp)   \
                 FROM msgs  WHERE chat_id=c.id    \
                 AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   \
                 AND c.blocked=0 AND c.name LIKE ?  \
                 GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;",
                params![str_like_cmd],
                process_row,
                process_rows,
            )?
        } else {
            //  show normal chatlist
            let mut ids = context.sql.query_map(
                "SELECT c.id, m.id FROM chats c  \
                 LEFT JOIN msgs m         \
                 ON c.id=m.chat_id        \
                 AND m.timestamp=( SELECT MAX(timestamp)   \
                 FROM msgs  WHERE chat_id=c.id    \
                 AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   \
                 AND c.blocked=0 AND c.archived=0  \
                 GROUP BY c.id  \
                 ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;",
                params![],
                process_row,
                process_rows,
            )?;
            if 0 == listflags & DC_GCL_NO_SPECIALS {
                let last_deaddrop_fresh_msg_id = get_last_deaddrop_fresh_msg(context);
                if last_deaddrop_fresh_msg_id > 0 {
                    ids.insert(0, (DC_CHAT_ID_DEADDROP, last_deaddrop_fresh_msg_id));
                }
                add_archived_link_item = 1;
            }
            ids
        };

        if 0 != add_archived_link_item && dc_get_archived_cnt(context) > 0 {
            if ids.is_empty() && 0 != listflags & DC_GCL_ADD_ALLDONE_HINT {
                ids.push((DC_CHAT_ID_ALLDONE_HINT, 0));
            }
            ids.push((DC_CHAT_ID_ARCHIVED_LINK, 0));
        }

        Ok(Chatlist { ids })
    }

    /// Find out the number of chats.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Get a single chat ID of a chatlist.
    ///
    /// To get the message object from the message ID, use dc_get_chat().
    pub fn get_chat_id(&self, index: usize) -> u32 {
        if index >= self.ids.len() {
            return 0;
        }
        self.ids[index].0
    }

    /// Get a single message ID of a chatlist.
    ///
    /// To get the message object from the message ID, use dc_get_msg().
    pub fn get_msg_id(&self, index: usize) -> u32 {
        if index >= self.ids.len() {
            return 0;
        }

        self.ids[index].1
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
        if index >= self.ids.len() {
            ret.text2 = Some("ErrBadChatlistIndex".to_string());
            return ret;
        }

        let chat_loaded: Chat;
        let chat = if let Some(chat) = chat {
            chat
        } else if let Ok(chat) = Chat::load_from_db(context, self.ids[index].0) {
            chat_loaded = chat;
            &chat_loaded
        } else {
            return ret;
        };

        let lastmsg_id = self.ids[index].1;
        let mut lastcontact = None;

        let lastmsg = if 0 != lastmsg_id {
            if let Ok(lastmsg) = Message::load_from_db(context, lastmsg_id) {
                if lastmsg.from_id != 1 as libc::c_uint
                    && (chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup)
                {
                    lastcontact = Contact::load_from_db(context, lastmsg.from_id).ok();
                }

                Some(lastmsg)
            } else {
                None
            }
        } else {
            None
        };

        if chat.id == DC_CHAT_ID_ARCHIVED_LINK {
            ret.text2 = None;
        } else if lastmsg.is_none() || lastmsg.as_ref().unwrap().from_id == DC_CONTACT_ID_UNDEFINED
        {
            ret.text2 = Some(context.stock_str(StockMessage::NoMessages).to_string());
        } else {
            ret.fill(&mut lastmsg.unwrap(), chat, lastcontact.as_ref(), context);
        }

        ret
    }
}

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

fn get_last_deaddrop_fresh_msg(context: &Context) -> u32 {
    // We have an index over the state-column, this should be sufficient as there are typically
    // only few fresh messages.
    context
        .sql
        .query_get_value(
            context,
            "SELECT m.id  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  \
             WHERE m.state=10   \
             AND m.hidden=0    \
             AND c.blocked=2 \
             ORDER BY m.timestamp DESC, m.id DESC;",
            params![],
        )
        .unwrap_or_default()
}

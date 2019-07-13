use crate::context::*;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_lot::*;
use crate::dc_msg::*;
use crate::dc_tools::*;
use crate::stock::StockMessage;
use crate::types::*;
use crate::x::*;

/* * the structure behind dc_chatlist_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_chatlist_t<'a> {
    pub magic: uint32_t,
    pub context: &'a Context,
    pub cnt: size_t,
    pub chatNlastmsg_ids: *mut dc_array_t,
}

// handle chatlists
pub unsafe fn dc_get_chatlist<'a>(
    context: &'a Context,
    listflags: libc::c_int,
    query_str: *const libc::c_char,
    query_id: uint32_t,
) -> *mut dc_chatlist_t<'a> {
    let obj = dc_chatlist_new(context);

    if 0 != dc_chatlist_load_from_db(obj, listflags, query_str, query_id) {
        return obj;
    }

    dc_chatlist_unref(obj);
    return 0 as *mut dc_chatlist_t;
}

/**
 * @class dc_chatlist_t
 *
 * An object representing a single chatlist in memory.
 * Chatlist objects contain chat IDs
 * and, if possible, message IDs belonging to them.
 * The chatlist object is not updated;
 * if you want an update, you have to recreate the object.
 *
 * For a **typical chat overview**,
 * the idea is to get the list of all chats via dc_get_chatlist()
 * without any listflags (see below)
 * and to implement a "virtual list" or so
 * (the count of chats is known by dc_chatlist_get_cnt()).
 *
 * Only for the items that are in view
 * (the list may have several hundreds chats),
 * the UI should call dc_chatlist_get_summary() then.
 * dc_chatlist_get_summary() provides all elements needed for painting the item.
 *
 * On a click of such an item,
 * the UI should change to the chat view
 * and get all messages from this view via dc_get_chat_msgs().
 * Again, a "virtual list" is created
 * (the count of messages is known)
 * and for each messages that is scrolled into view, dc_get_msg() is called then.
 *
 * Why no listflags?
 * Without listflags, dc_get_chatlist() adds the deaddrop
 * and the archive "link" automatically as needed.
 * The UI can just render these items differently then.
 * Although the deaddrop link is currently always the first entry
 * and only present on new messages,
 * there is the rough idea that it can be optionally always present
 * and sorted into the list by date.
 * Rendering the deaddrop in the described way
 * would not add extra work in the UI then.
 */
pub unsafe fn dc_chatlist_new(context: &Context) -> *mut dc_chatlist_t {
    let mut chatlist: *mut dc_chatlist_t;
    chatlist = calloc(1, ::std::mem::size_of::<dc_chatlist_t>()) as *mut dc_chatlist_t;
    assert!(!chatlist.is_null());

    (*chatlist).magic = 0xc4a71157u32;
    (*chatlist).context = context;
    (*chatlist).chatNlastmsg_ids = dc_array_new(128i32 as size_t);
    assert!(!(*chatlist).chatNlastmsg_ids.is_null());
    chatlist
}

pub unsafe fn dc_chatlist_unref(mut chatlist: *mut dc_chatlist_t) {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return;
    }
    dc_chatlist_empty(chatlist);
    dc_array_unref((*chatlist).chatNlastmsg_ids);
    (*chatlist).magic = 0i32 as uint32_t;
    free(chatlist as *mut libc::c_void);
}

pub unsafe fn dc_chatlist_empty(mut chatlist: *mut dc_chatlist_t) {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return;
    }
    (*chatlist).cnt = 0i32 as size_t;
    dc_array_empty((*chatlist).chatNlastmsg_ids);
}

/**
 * Load a chatlist from the database to the chatlist object.
 *
 * @private @memberof dc_chatlist_t
 */
// TODO should return bool /rtn
unsafe fn dc_chatlist_load_from_db(
    mut chatlist: *mut dc_chatlist_t,
    listflags: libc::c_int,
    query__: *const libc::c_char,
    query_contact_id: u32,
) -> libc::c_int {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return 0;
    }
    dc_chatlist_empty(chatlist);

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

        Ok((chat_id, msg_id))
    };

    let process_rows = |rows: rusqlite::MappedRows<_>| {
        for row in rows {
            let (id1, id2) = row?;

            dc_array_add_id((*chatlist).chatNlastmsg_ids, id1 as u32);
            dc_array_add_id((*chatlist).chatNlastmsg_ids, id2 as u32);
        }
        Ok(())
    };

    // nb: the query currently shows messages from blocked contacts in groups.
    // however, for normal-groups, this is okay as the message is also returned by dc_get_chat_msgs()
    // (otherwise it would be hard to follow conversations, wa and tg do the same)
    // for the deaddrop, however, they should really be hidden, however, _currently_ the deaddrop is not
    // shown at all permanent in the chatlist.

    let success = if query_contact_id != 0 {
        // show chats shared with a given contact
        (*chatlist).context.sql.query_map(
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
        )
    } else if 0 != listflags & 0x1 {
        // show archived chats
        (*chatlist).context.sql.query_map(
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
        )
    } else if query__.is_null() {
        //  show normal chatlist
        if 0 == listflags & 0x2 {
            let last_deaddrop_fresh_msg_id = get_last_deaddrop_fresh_msg((*chatlist).context);
            if last_deaddrop_fresh_msg_id > 0 {
                dc_array_add_id((*chatlist).chatNlastmsg_ids, 1);
                dc_array_add_id((*chatlist).chatNlastmsg_ids, last_deaddrop_fresh_msg_id);
            }
            add_archived_link_item = 1;
        }
        (*chatlist).context.sql.query_map(
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
        )
    } else {
        let query = to_string(query__).trim().to_string();
        if query.is_empty() {
            return 1;
        } else {
            let strLikeCmd = format!("%{}%", query);
            (*chatlist).context.sql.query_map(
                "SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         \
                 ON c.id=m.chat_id        \
                 AND m.timestamp=( SELECT MAX(timestamp)   \
                 FROM msgs  WHERE chat_id=c.id    \
                 AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   \
                 AND c.blocked=0 AND c.name LIKE ?  \
                 GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;",
                params![strLikeCmd],
                process_row,
                process_rows,
            )
        }
    };

    if 0 != add_archived_link_item && dc_get_archived_cnt((*chatlist).context) > 0 {
        if dc_array_get_cnt((*chatlist).chatNlastmsg_ids) == 0 && 0 != listflags & 0x4 {
            dc_array_add_id((*chatlist).chatNlastmsg_ids, 7);
            dc_array_add_id((*chatlist).chatNlastmsg_ids, 0);
        }
        dc_array_add_id((*chatlist).chatNlastmsg_ids, 6);
        dc_array_add_id((*chatlist).chatNlastmsg_ids, 0);
    }
    (*chatlist).cnt = dc_array_get_cnt((*chatlist).chatNlastmsg_ids) / 2;

    match success {
        Ok(_) => 1,
        Err(err) => {
            error!(
                (*chatlist).context,
                0, "chatlist: failed to load from database: {:?}", err
            );
            0
        }
    }
}

// Context functions to work with chatlist
pub fn dc_get_archived_cnt(context: &Context) -> libc::c_int {
    context
        .sql
        .query_row_col(
            context,
            "SELECT COUNT(*) FROM chats WHERE blocked=0 AND archived=1;",
            params![],
            0,
        )
        .unwrap_or_default()
}

fn get_last_deaddrop_fresh_msg(context: &Context) -> u32 {
    // we have an index over the state-column, this should be sufficient as there are typically only few fresh messages
    context
        .sql
        .query_row_col(
            context,
            "SELECT m.id  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  \
             WHERE m.state=10   \
             AND m.hidden=0    \
             AND c.blocked=2 \
             ORDER BY m.timestamp DESC, m.id DESC;",
            params![],
            0,
        )
        .unwrap_or_default()
}

pub unsafe fn dc_chatlist_get_cnt(chatlist: *const dc_chatlist_t) -> size_t {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return 0i32 as size_t;
    }
    (*chatlist).cnt
}

pub unsafe fn dc_chatlist_get_chat_id(chatlist: *const dc_chatlist_t, index: size_t) -> uint32_t {
    if chatlist.is_null()
        || (*chatlist).magic != 0xc4a71157u32
        || (*chatlist).chatNlastmsg_ids.is_null()
        || index >= (*chatlist).cnt
    {
        return 0i32 as uint32_t;
    }
    dc_array_get_id((*chatlist).chatNlastmsg_ids, index.wrapping_mul(2))
}

pub unsafe fn dc_chatlist_get_msg_id(chatlist: *const dc_chatlist_t, index: size_t) -> uint32_t {
    if chatlist.is_null()
        || (*chatlist).magic != 0xc4a71157u32
        || (*chatlist).chatNlastmsg_ids.is_null()
        || index >= (*chatlist).cnt
    {
        return 0i32 as uint32_t;
    }
    dc_array_get_id(
        (*chatlist).chatNlastmsg_ids,
        index.wrapping_mul(2).wrapping_add(1),
    )
}

pub unsafe fn dc_chatlist_get_summary<'a>(
    chatlist: *const dc_chatlist_t<'a>,
    index: size_t,
    mut chat: *mut Chat<'a>,
) -> *mut dc_lot_t {
    let current_block: u64;
    /* The summary is created by the chat, not by the last message.
    This is because we may want to display drafts here or stuff as
    "is typing".
    Also, sth. as "No messages" would not work if the summary comes from a
    message. */
    /* the function never returns NULL */
    let mut ret: *mut dc_lot_t = dc_lot_new();
    let lastmsg_id: uint32_t;
    let mut lastmsg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut lastcontact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut chat_to_delete: *mut Chat = 0 as *mut Chat;
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 || index >= (*chatlist).cnt {
        (*ret).text2 = dc_strdup(b"ErrBadChatlistIndex\x00" as *const u8 as *const libc::c_char)
    } else {
        lastmsg_id = dc_array_get_id(
            (*chatlist).chatNlastmsg_ids,
            index.wrapping_mul(2).wrapping_add(1),
        );
        if chat.is_null() {
            chat = dc_chat_new((*chatlist).context);
            chat_to_delete = chat;
            if !dc_chat_load_from_db(
                chat,
                dc_array_get_id((*chatlist).chatNlastmsg_ids, index.wrapping_mul(2)),
            ) {
                (*ret).text2 =
                    dc_strdup(b"ErrCannotReadChat\x00" as *const u8 as *const libc::c_char);
                current_block = 3777403817673069519;
            } else {
                current_block = 7651349459974463963;
            }
        } else {
            current_block = 7651349459974463963;
        }
        match current_block {
            3777403817673069519 => {}
            _ => {
                if 0 != lastmsg_id {
                    lastmsg = dc_msg_new_untyped((*chatlist).context);
                    dc_msg_load_from_db(lastmsg, (*chatlist).context, lastmsg_id);
                    if (*lastmsg).from_id != 1i32 as libc::c_uint
                        && ((*chat).type_0 == 120i32 || (*chat).type_0 == 130i32)
                    {
                        lastcontact = dc_contact_new((*chatlist).context);
                        dc_contact_load_from_db(
                            lastcontact,
                            &(*chatlist).context.sql,
                            (*lastmsg).from_id,
                        );
                    }
                }
                if (*chat).id == 6i32 as libc::c_uint {
                    (*ret).text2 = dc_strdup(0 as *const libc::c_char)
                } else if lastmsg.is_null() || (*lastmsg).from_id == 0i32 as libc::c_uint {
                    (*ret).text2 =
                        to_cstring((*chatlist).context.stock_str(StockMessage::NoMessages));
                } else {
                    dc_lot_fill(ret, lastmsg, chat, lastcontact, (*chatlist).context);
                }
            }
        }
    }
    dc_msg_unref(lastmsg);
    dc_contact_unref(lastcontact);
    dc_chat_unref(chat_to_delete);
    ret
}

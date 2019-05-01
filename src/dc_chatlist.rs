use libc;

use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_context::*;
use crate::dc_lot::*;
use crate::dc_msg::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/* * the structure behind dc_chatlist_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_chatlist_t {
    pub magic: uint32_t,
    pub context: &dc_context_t,
    pub cnt: size_t,
    pub chatNlastmsg_ids: *mut dc_array_t,
}

// handle chatlists
pub unsafe fn dc_get_chatlist(
    mut context: &dc_context_t,
    mut listflags: libc::c_int,
    mut query_str: *const libc::c_char,
    mut query_id: uint32_t,
) -> *mut dc_chatlist_t {
    let mut success: libc::c_int = 0i32;
    let mut obj: *mut dc_chatlist_t = dc_chatlist_new(context);
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        if !(0 == dc_chatlist_load_from_db(obj, listflags, query_str, query_id)) {
            success = 1i32
        }
    }
    if 0 != success {
        return obj;
    } else {
        dc_chatlist_unref(obj);
        return 0 as *mut dc_chatlist_t;
    };
}
/* *
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
pub unsafe fn dc_chatlist_new(mut context: &dc_context_t) -> *mut dc_chatlist_t {
    let mut chatlist: *mut dc_chatlist_t = 0 as *mut dc_chatlist_t;
    chatlist = calloc(1, ::std::mem::size_of::<dc_chatlist_t>()) as *mut dc_chatlist_t;
    if chatlist.is_null() {
        exit(20i32);
    }
    (*chatlist).magic = 0xc4a71157u32;
    (*chatlist).context = context;
    (*chatlist).chatNlastmsg_ids = dc_array_new(context, 128i32 as size_t);
    if (*chatlist).chatNlastmsg_ids.is_null() {
        exit(32i32);
    }
    return chatlist;
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
/* *
 * Load a chatlist from the database to the chatlist object.
 *
 * @private @memberof dc_chatlist_t
 */
unsafe fn dc_chatlist_load_from_db(
    mut chatlist: *mut dc_chatlist_t,
    mut listflags: libc::c_int,
    mut query__: *const libc::c_char,
    mut query_contact_id: uint32_t,
) -> libc::c_int {
    let mut current_block: u64;
    //clock_t       start = clock();
    let mut success: libc::c_int = 0i32;
    let mut add_archived_link_item: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut strLikeCmd: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut query: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 || (*chatlist).context.is_null())
    {
        dc_chatlist_empty(chatlist);
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
        if 0 != query_contact_id {
            stmt =
                dc_sqlite3_prepare((*(*chatlist).context).sql,
                                   b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.id IN(SELECT chat_id FROM chats_contacts WHERE contact_id=?)  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, query_contact_id as libc::c_int);
            current_block = 3437258052017859086;
        } else if 0 != listflags & 0x1i32 {
            stmt =
                dc_sqlite3_prepare((*(*chatlist).context).sql,
                                   b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.archived=1  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                       as *const u8 as *const libc::c_char);
            current_block = 3437258052017859086;
        } else if query__.is_null() {
            if 0 == listflags & 0x2i32 {
                let mut last_deaddrop_fresh_msg_id: uint32_t =
                    get_last_deaddrop_fresh_msg((*chatlist).context);
                if last_deaddrop_fresh_msg_id > 0i32 as libc::c_uint {
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, 1i32 as uint32_t);
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, last_deaddrop_fresh_msg_id);
                }
                add_archived_link_item = 1i32
            }
            stmt =
                dc_sqlite3_prepare((*(*chatlist).context).sql,
                                   b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.archived=0  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                       as *const u8 as *const libc::c_char);
            current_block = 3437258052017859086;
        } else {
            query = dc_strdup(query__);
            dc_trim(query);
            if *query.offset(0isize) as libc::c_int == 0i32 {
                success = 1i32;
                current_block = 15179736777190528364;
            } else {
                strLikeCmd = dc_mprintf(b"%%%s%%\x00" as *const u8 as *const libc::c_char, query);
                stmt =
                    dc_sqlite3_prepare((*(*chatlist).context).sql,
                                       b"SELECT c.id, m.id FROM chats c  LEFT JOIN msgs m         ON c.id=m.chat_id        AND m.timestamp=( SELECT MAX(timestamp)   FROM msgs  WHERE chat_id=c.id    AND (hidden=0 OR (hidden=1 AND state=19))) WHERE c.id>9   AND c.blocked=0 AND c.name LIKE ?  GROUP BY c.id  ORDER BY IFNULL(m.timestamp,0) DESC, m.id DESC;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_text(stmt, 1i32, strLikeCmd, -1i32, None);
                current_block = 3437258052017859086;
            }
        }
        match current_block {
            15179736777190528364 => {}
            _ => {
                while sqlite3_step(stmt) == 100i32 {
                    dc_array_add_id(
                        (*chatlist).chatNlastmsg_ids,
                        sqlite3_column_int(stmt, 0i32) as uint32_t,
                    );
                    dc_array_add_id(
                        (*chatlist).chatNlastmsg_ids,
                        sqlite3_column_int(stmt, 1i32) as uint32_t,
                    );
                }
                if 0 != add_archived_link_item && dc_get_archived_cnt((*chatlist).context) > 0i32 {
                    if dc_array_get_cnt((*chatlist).chatNlastmsg_ids) == 0
                        && 0 != listflags & 0x4i32
                    {
                        dc_array_add_id((*chatlist).chatNlastmsg_ids, 7i32 as uint32_t);
                        dc_array_add_id((*chatlist).chatNlastmsg_ids, 0i32 as uint32_t);
                    }
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, 6i32 as uint32_t);
                    dc_array_add_id((*chatlist).chatNlastmsg_ids, 0i32 as uint32_t);
                }
                (*chatlist).cnt = dc_array_get_cnt((*chatlist).chatNlastmsg_ids).wrapping_div(2);
                success = 1i32
            }
        }
    }
    sqlite3_finalize(stmt);
    free(query as *mut libc::c_void);
    free(strLikeCmd as *mut libc::c_void);
    return success;
}
// Context functions to work with chatlist
pub unsafe fn dc_get_archived_cnt(mut context: &dc_context_t) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT COUNT(*) FROM chats WHERE blocked=0 AND archived=1;\x00" as *const u8
            as *const libc::c_char,
    );
    if sqlite3_step(stmt) == 100i32 {
        ret = sqlite3_column_int(stmt, 0i32)
    }
    sqlite3_finalize(stmt);
    return ret;
}
unsafe fn get_last_deaddrop_fresh_msg(mut context: &dc_context_t) -> uint32_t {
    let mut ret: uint32_t = 0i32 as uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    stmt =
        dc_sqlite3_prepare((*context).sql,
                           b"SELECT m.id  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  WHERE m.state=10   AND m.hidden=0    AND c.blocked=2 ORDER BY m.timestamp DESC, m.id DESC;\x00"
                               as *const u8 as *const libc::c_char);
    /* we have an index over the state-column, this should be sufficient as there are typically only few fresh messages */
    if !(sqlite3_step(stmt) != 100i32) {
        ret = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    return ret;
}
pub unsafe fn dc_chatlist_get_cnt(mut chatlist: *const dc_chatlist_t) -> size_t {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return 0i32 as size_t;
    }
    return (*chatlist).cnt;
}
pub unsafe fn dc_chatlist_get_chat_id(
    mut chatlist: *const dc_chatlist_t,
    mut index: size_t,
) -> uint32_t {
    if chatlist.is_null()
        || (*chatlist).magic != 0xc4a71157u32
        || (*chatlist).chatNlastmsg_ids.is_null()
        || index >= (*chatlist).cnt
    {
        return 0i32 as uint32_t;
    }
    return dc_array_get_id((*chatlist).chatNlastmsg_ids, index.wrapping_mul(2));
}
pub unsafe fn dc_chatlist_get_msg_id(
    mut chatlist: *const dc_chatlist_t,
    mut index: size_t,
) -> uint32_t {
    if chatlist.is_null()
        || (*chatlist).magic != 0xc4a71157u32
        || (*chatlist).chatNlastmsg_ids.is_null()
        || index >= (*chatlist).cnt
    {
        return 0i32 as uint32_t;
    }
    return dc_array_get_id(
        (*chatlist).chatNlastmsg_ids,
        index.wrapping_mul(2).wrapping_add(1),
    );
}
pub unsafe fn dc_chatlist_get_summary(
    mut chatlist: *const dc_chatlist_t,
    mut index: size_t,
    mut chat: *mut dc_chat_t,
) -> *mut dc_lot_t {
    let mut current_block: u64;
    /* The summary is created by the chat, not by the last message.
    This is because we may want to display drafts here or stuff as
    "is typing".
    Also, sth. as "No messages" would not work if the summary comes from a
    message. */
    /* the function never returns NULL */
    let mut ret: *mut dc_lot_t = dc_lot_new();
    let mut lastmsg_id: uint32_t = 0i32 as uint32_t;
    let mut lastmsg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut lastcontact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut chat_to_delete: *mut dc_chat_t = 0 as *mut dc_chat_t;
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
            if 0 == dc_chat_load_from_db(
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
                            (*(*chatlist).context).sql,
                            (*lastmsg).from_id,
                        );
                    }
                }
                if (*chat).id == 6i32 as libc::c_uint {
                    (*ret).text2 = dc_strdup(0 as *const libc::c_char)
                } else if lastmsg.is_null() || (*lastmsg).from_id == 0i32 as libc::c_uint {
                    (*ret).text2 = dc_stock_str((*chatlist).context, 1i32)
                } else {
                    dc_lot_fill(ret, lastmsg, chat, lastcontact, (*chatlist).context);
                }
            }
        }
    }
    dc_msg_unref(lastmsg);
    dc_contact_unref(lastcontact);
    dc_chat_unref(chat_to_delete);
    return ret;
}
pub unsafe fn dc_chatlist_get_context(mut chatlist: *mut dc_chatlist_t) -> &dc_context_t {
    if chatlist.is_null() || (*chatlist).magic != 0xc4a71157u32 {
        return 0 as *mut dc_context_t;
    }
    return (*chatlist).context;
}

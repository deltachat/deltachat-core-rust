use std::ffi::CString;

use crate::chatlist::*;
use crate::constants::*;
use crate::context::Context;
use crate::dc_array::*;
use crate::dc_contact::*;
use crate::dc_job::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_tools::*;
use crate::sql::{self, Sql};
use crate::stock::StockMessage;
use crate::types::*;
use crate::x::*;

/**
 * @class dc_chat_t
 *
 * An object representing a single chat in memory.
 * Chat objects are created using eg. dc_get_chat()
 * and are not updated on database changes;
 * if you want an update, you have to recreate the object.
 */
#[derive(Clone)]
pub struct Chat<'a> {
    magic: uint32_t,
    pub id: uint32_t,
    pub type_0: libc::c_int,
    pub name: *mut libc::c_char,
    archived: libc::c_int,
    pub context: &'a Context,
    pub grpid: *mut libc::c_char,
    blocked: libc::c_int,
    pub param: dc_param_t,
    pub gossiped_timestamp: i64,
    is_sending_locations: libc::c_int,
}

// handle chats
pub unsafe fn dc_create_chat_by_msg_id(context: &Context, msg_id: uint32_t) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut send_event: libc::c_int = 0i32;
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let chat: *mut Chat = dc_chat_new(context);
    if dc_msg_load_from_db(msg, context, msg_id)
        && dc_chat_load_from_db(chat, (*msg).chat_id)
        && (*chat).id > 9i32 as libc::c_uint
    {
        chat_id = (*chat).id;
        if 0 != (*chat).blocked {
            dc_unblock_chat(context, (*chat).id);
            send_event = 1i32
        }
        dc_scaleup_contact_origin(context, (*msg).from_id, 0x800i32);
    }

    dc_msg_unref(msg);
    dc_chat_unref(chat);
    if 0 != send_event {
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
    }
    chat_id
}

pub unsafe fn dc_chat_new<'a>(context: &'a Context) -> *mut Chat<'a> {
    let mut chat: *mut Chat;
    chat = calloc(1, ::std::mem::size_of::<Chat>()) as *mut Chat;
    (*chat).magic = 0xc4a7c4a7u32;
    (*chat).context = context;
    (*chat).type_0 = 0i32;
    (*chat).param = Default::default();
    chat
}

pub unsafe fn dc_chat_unref(mut chat: *mut Chat) {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return;
    }
    dc_chat_empty(chat);
    (*chat).magic = 0i32 as uint32_t;
    free(chat as *mut libc::c_void);
}

pub unsafe fn dc_chat_empty(mut chat: *mut Chat) {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return;
    }
    free((*chat).name as *mut libc::c_void);
    (*chat).name = 0 as *mut libc::c_char;
    (*chat).type_0 = 0i32;
    (*chat).id = 0i32 as uint32_t;
    free((*chat).grpid as *mut libc::c_void);
    (*chat).grpid = 0 as *mut libc::c_char;
    (*chat).blocked = 0i32;
    (*chat).gossiped_timestamp = 0;
    (*chat).param = Default::default();
}

pub unsafe fn dc_unblock_chat(context: &Context, chat_id: uint32_t) {
    dc_block_chat(context, chat_id, 0i32);
}

pub fn dc_block_chat(context: &Context, chat_id: u32, new_blocking: libc::c_int) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET blocked=? WHERE id=?;",
        params![new_blocking, chat_id as i32],
    )
    .is_ok()
}

pub fn dc_chat_load_from_db(chat: *mut Chat, chat_id: u32) -> bool {
    if chat.is_null() || unsafe { (*chat).magic != 0xc4a7c4a7u32 } {
        return false;
    }
    unsafe { dc_chat_empty(chat) };

    let context = unsafe { (*chat).context };

    let res = context.sql.query_row(
        "SELECT c.id,c.type,c.name, c.grpid,c.param,c.archived, \
         c.blocked, c.gossiped_timestamp, c.locations_send_until  \
         FROM chats c WHERE c.id=?;",
        params![chat_id as i32],
        |row| {
            let c = unsafe { &mut *chat };

            c.id = row.get(0)?;
            c.type_0 = row.get(1)?;
            c.name = {
                let raw: String = row.get(2)?;
                unsafe { to_cstring(raw) }
            };
            c.grpid = {
                let raw: String = row.get(3)?;
                unsafe { to_cstring(raw) }
            };

            let packed: String = row.get(4)?;
            c.param = packed.parse()?;

            c.archived = row.get(5)?;
            c.blocked = row.get::<_, Option<i32>>(6)?.unwrap_or_default();
            c.gossiped_timestamp = row.get(7)?;
            c.is_sending_locations = row.get(8)?;
            Ok(())
        },
    );

    match res {
        Err(crate::error::Error::Sql(rusqlite::Error::QueryReturnedNoRows)) => false,
        Err(err) => match err {
            _ => {
                error!(
                    context,
                    0, "chat: failed to load from db {}: {:?}", chat_id, err
                );
                false
            }
        },
        Ok(_) => {
            let c = unsafe { &mut *chat };
            match c.id {
                1 => unsafe {
                    free((*chat).name as *mut libc::c_void);
                    (*chat).name = to_cstring((*chat).context.stock_str(StockMessage::DeadDrop));
                },
                6 => unsafe {
                    free((*chat).name as *mut libc::c_void);
                    let tempname = (*chat).context.stock_str(StockMessage::ArchivedChats);
                    let cnt = dc_get_archived_cnt((*chat).context);
                    (*chat).name = to_cstring(format!("{} ({})", tempname, cnt));
                },
                5 => unsafe {
                    free((*chat).name as *mut libc::c_void);
                    (*chat).name = to_cstring((*chat).context.stock_str(StockMessage::StarredMsgs));
                },
                _ => {
                    if dc_param_exists((*chat).param, Param::Selftalk) {
                        unsafe {
                            free((*chat).name as *mut libc::c_void);
                            (*chat).name =
                                to_cstring((*chat).context.stock_str(StockMessage::SelfMsg));
                        }
                    }
                }
            }
            true
        }
    }
}

pub unsafe fn dc_create_chat_by_contact_id(context: &Context, contact_id: uint32_t) -> uint32_t {
    let mut chat_id = 0;
    let mut chat_blocked = 0;
    let mut send_event = 0;
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_blocked);
    if 0 != chat_id {
        if 0 != chat_blocked {
            dc_unblock_chat(context, chat_id);
            send_event = 1i32
        }
    } else if !dc_real_contact_exists(context, contact_id) && contact_id != 1i32 as libc::c_uint {
        warn!(
            context,
            0, "Cannot create chat, contact {} does not exist.", contact_id as libc::c_int,
        );
    } else {
        dc_create_or_lookup_nchat_by_contact_id(
            context,
            contact_id,
            0i32,
            &mut chat_id,
            0 as *mut libc::c_int,
        );
        if 0 != chat_id {
            send_event = 1;
        }
        dc_scaleup_contact_origin(context, contact_id, 0x800i32);
    }
    if 0 != send_event {
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
    }
    chat_id
}

pub unsafe fn dc_create_or_lookup_nchat_by_contact_id(
    context: &Context,
    contact_id: uint32_t,
    create_blocked: libc::c_int,
    ret_chat_id: *mut uint32_t,
    ret_chat_blocked: *mut libc::c_int,
) {
    let mut chat_id = 0;
    let mut chat_blocked = 0;
    let contact: *mut dc_contact_t;
    let chat_name: *mut libc::c_char;

    if !ret_chat_id.is_null() {
        *ret_chat_id = 0;
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = 0;
    }
    if !context.sql.is_open() {
        return;
    }
    if contact_id == 0 as libc::c_uint {
        return;
    }
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_blocked);
    if chat_id != 0 {
        if !ret_chat_id.is_null() {
            *ret_chat_id = chat_id
        }
        if !ret_chat_blocked.is_null() {
            *ret_chat_blocked = chat_blocked
        }
        return;
    }
    contact = dc_contact_new(context);
    if dc_contact_load_from_db(contact, &context.sql, contact_id) {
        chat_name =
            if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
                (*contact).name
            } else {
                (*contact).addr
            };

        if sql::execute(
            context,
            &context.sql,
            format!(
                "INSERT INTO chats (type, name, param, blocked, grpid) VALUES({}, '{}', '{}', {}, '{}')",
                100,
                as_str(chat_name),
                if contact_id == 1 { "K=1" } else { "" },
                create_blocked,
                as_str((*contact).addr),
            ),
            params![],
        ).is_ok() {
            chat_id = sql::get_rowid(
                context,
                &context.sql,
                "chats",
                "grpid",
                as_str((*contact).addr),
            );

            sql::execute(
                context,
                &context.sql,
                format!("INSERT INTO chats_contacts (chat_id, contact_id) VALUES({}, {})", chat_id, contact_id),
                params![],
            ).ok();
        }
    }

    dc_contact_unref(contact);
    if !ret_chat_id.is_null() {
        *ret_chat_id = chat_id
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = create_blocked
    };
}

pub fn dc_lookup_real_nchat_by_contact_id(
    context: &Context,
    contact_id: uint32_t,
    ret_chat_id: *mut uint32_t,
    ret_chat_blocked: *mut libc::c_int,
) {
    /* checks for "real" chats or self-chat */
    if !ret_chat_id.is_null() {
        unsafe { *ret_chat_id = 0 };
    }
    if !ret_chat_blocked.is_null() {
        unsafe { *ret_chat_blocked = 0 };
    }
    if !context.sql.is_open() {
        return;
    }

    if let Ok((id, blocked)) = context.sql.query_row(
        "SELECT c.id, c.blocked FROM chats c INNER JOIN chats_contacts j ON c.id=j.chat_id WHERE c.type=100 AND c.id>9 AND j.contact_id=?;",
        params![contact_id as i32],
        |row| Ok((row.get(0)?, row.get::<_, Option<i32>>(1)?.unwrap_or_default())),
    ) {
        unsafe { *ret_chat_id = id };
        unsafe { *ret_chat_blocked = blocked };
    }
}

pub unsafe fn dc_get_chat_id_by_contact_id(context: &Context, contact_id: uint32_t) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0i32;
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_id_blocked);
    return if 0 != chat_id_blocked {
        0i32 as libc::c_uint
    } else {
        chat_id
    };
}

pub unsafe fn dc_prepare_msg<'a>(
    context: &'a Context,
    chat_id: uint32_t,
    mut msg: *mut dc_msg_t<'a>,
) -> uint32_t {
    if msg.is_null() || chat_id <= 9i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    (*msg).state = DC_STATE_OUT_PREPARING;
    let msg_id: uint32_t = prepare_msg_common(context, chat_id, msg);
    context.call_cb(
        Event::MSGS_CHANGED,
        (*msg).chat_id as uintptr_t,
        (*msg).id as uintptr_t,
    );
    return msg_id;
}

pub fn msgtype_has_file(msgtype: i32) -> bool {
    match msgtype {
        DC_MSG_IMAGE => true,
        DC_MSG_GIF => true,
        DC_MSG_AUDIO => true,
        DC_MSG_VOICE => true,
        DC_MSG_VIDEO => true,
        DC_MSG_FILE => true,
        _ => false,
    }
}

unsafe fn prepare_msg_common<'a>(
    context: &'a Context,
    chat_id: uint32_t,
    mut msg: *mut dc_msg_t<'a>,
) -> uint32_t {
    let mut OK_TO_CONTINUE = true;
    (*msg).id = 0i32 as uint32_t;
    (*msg).context = context;
    if (*msg).type_0 == DC_MSG_TEXT {
        /* the caller should check if the message text is empty */
    } else if msgtype_has_file((*msg).type_0) {
        let pathNfilename = dc_param_get((*msg).param, Param::File);
        if pathNfilename.is_none() {
            error!(
                context,
                0,
                "Attachment missing for message of type #{}.",
                (*msg).type_0,
            );
            OK_TO_CONTINUE = false;
        } else if (*msg).state == DC_STATE_OUT_PREPARING
            && !dc_is_blobdir_path_r(context, pathNfilename.as_ref().unwrap())
        {
            error!(context, 0, "Files must be created in the blob-directory.",);
            OK_TO_CONTINUE = false;
        } else {
            let mut pathNfilename = to_cstring(pathNfilename.unwrap());
            if !dc_make_rel_and_copy(context, &mut pathNfilename) {
                OK_TO_CONTINUE = false;
            } else {
                dc_param_set(&mut (*msg).param, Param::File, as_str(pathNfilename));
                if (*msg).type_0 == DC_MSG_FILE || (*msg).type_0 == DC_MSG_IMAGE {
                    /* Correct the type, take care not to correct already very special formats as GIF or VOICE.
                    Typical conversions:
                    - from FILE to AUDIO/VIDEO/IMAGE
                    - from FILE/IMAGE to GIF */
                    let mut better_type = 0;
                    let mut better_mime = std::ptr::null_mut();

                    dc_msg_guess_msgtype_from_suffix(
                        pathNfilename,
                        &mut better_type,
                        &mut better_mime,
                    );
                    if 0 != better_type && !better_mime.is_null() {
                        (*msg).type_0 = better_type;
                        dc_param_set(&mut (*msg).param, Param::MimeType, as_str(better_mime));
                    }
                    free(better_mime as *mut libc::c_void);
                } else if !dc_param_exists(&(*msg).param, Param::MimeType) {
                    let mut better_mime = std::ptr::null_mut();

                    dc_msg_guess_msgtype_from_suffix(
                        pathNfilename,
                        0 as *mut libc::c_int,
                        &mut better_mime,
                    );

                    if !better_mime.is_null() {
                        dc_param_set(&mut (*msg).param, Param::MimeType, as_str(better_mime));
                    }
                    free(better_mime as *mut _);
                }
                info!(
                    context,
                    0,
                    "Attaching \"{}\" for message type #{}.",
                    as_str(pathNfilename),
                    (*msg).type_0
                );

                free(pathNfilename as *mut _);
            }
        }
    } else {
        error!(
            context,
            0,
            "Cannot send messages of type #{}.",
            (*msg).type_0
        );
        OK_TO_CONTINUE = false;
    }
    if OK_TO_CONTINUE {
        dc_unarchive_chat(context, chat_id);
        let chat = dc_chat_new(context);
        if dc_chat_load_from_db(chat, chat_id) {
            if (*msg).state != DC_STATE_OUT_PREPARING {
                (*msg).state = DC_STATE_OUT_PENDING
            }
            (*msg).id = prepare_msg_raw(context, chat, msg, dc_create_smeared_timestamp(context));
            (*msg).chat_id = chat_id
        }
        dc_chat_unref(chat);
    }

    (*msg).id
}

unsafe fn prepare_msg_raw(
    context: &Context,
    chat: *mut Chat,
    msg: *const dc_msg_t,
    timestamp: i64,
) -> uint32_t {
    let mut do_guarantee_e2ee: libc::c_int;
    let e2ee_enabled: libc::c_int;
    let mut OK_TO_CONTINUE = true;
    let mut parent_rfc724_mid = 0 as *mut libc::c_char;
    let mut parent_references = 0 as *mut libc::c_char;
    let mut parent_in_reply_to = 0 as *mut libc::c_char;
    let mut new_rfc724_mid = 0 as *mut libc::c_char;
    let mut new_references = 0 as *mut libc::c_char;
    let mut new_in_reply_to = 0 as *mut libc::c_char;
    let mut msg_id = 0;
    let mut to_id = 0;
    let mut location_id = 0;

    if !((*chat).type_0 == 100 || (*chat).type_0 == 120 || (*chat).type_0 == 130) {
        error!(context, 0, "Cannot send to chat type #{}.", (*chat).type_0,);
    } else if ((*chat).type_0 == 120 || (*chat).type_0 == 130)
        && 0 == dc_is_contact_in_chat(context, (*chat).id, 1 as uint32_t)
    {
        log_event!(
            context,
            Event::ERROR_SELF_NOT_IN_GROUP,
            0,
            "Cannot send message; self not in group.",
        );
    } else {
        let from = context.sql.get_config(context, "configured_addr");
        if from.is_none() {
            error!(context, 0, "Cannot send message, not configured.",);
        } else {
            let from_c = to_cstring(from.unwrap());
            new_rfc724_mid = dc_create_outgoing_rfc724_mid(
                if (*chat).type_0 == 120 || (*chat).type_0 == 130 {
                    (*chat).grpid
                } else {
                    0 as *mut libc::c_char
                },
                from_c,
            );
            free(from_c as *mut _);

            if (*chat).type_0 == DC_CHAT_TYPE_SINGLE {
                if let Some(id) = context.sql.query_row_col(
                    context,
                    "SELECT contact_id FROM chats_contacts WHERE chat_id=?;",
                    params![(*chat).id as i32],
                    0,
                ) {
                    to_id = id;
                } else {
                    error!(
                        context,
                        0,
                        "Cannot send message, contact for chat #{} not found.",
                        (*chat).id,
                    );
                    OK_TO_CONTINUE = false;
                }
            } else {
                if (*chat).type_0 == DC_CHAT_TYPE_GROUP
                    || (*chat).type_0 == DC_CHAT_TYPE_VERIFIED_GROUP
                {
                    if dc_param_get_int(&(*chat).param, Param::Unpromoted).unwrap_or_default() == 1
                    {
                        dc_param_remove(&mut (*chat).param, Param::Unpromoted);
                        dc_chat_update_param(chat);
                    }
                }
            }
            if OK_TO_CONTINUE {
                /* check if we can guarantee E2EE for this message.
                if we guarantee E2EE, and circumstances change
                so that E2EE is no longer available at a later point (reset, changed settings),
                we do not send the message out at all */
                do_guarantee_e2ee = 0;
                e2ee_enabled = context
                    .sql
                    .get_config_int(context, "e2ee_enabled")
                    .unwrap_or_else(|| 1);
                if 0 != e2ee_enabled
                    && dc_param_get_int(&(*msg).param, Param::ForcePlaintext).unwrap_or_default()
                        == 0
                {
                    let mut can_encrypt = 1;
                    let mut all_mutual = 1;

                    let res = context.sql.query_row(
                        "SELECT ps.prefer_encrypted, c.addr \
                         FROM chats_contacts cc  \
                         LEFT JOIN contacts c ON cc.contact_id=c.id  \
                         LEFT JOIN acpeerstates ps ON c.addr=ps.addr  \
                         WHERE cc.chat_id=?  AND cc.contact_id>9;",
                        params![(*chat).id],
                        |row| {
                            let state: String = row.get(1)?;

                            if let Some(prefer_encrypted) = row.get::<_, Option<i32>>(0)? {
                                if prefer_encrypted != 1 {
                                    info!(
                                        context,
                                        0,
                                        "[autocrypt] peerstate for {} is {}",
                                        state,
                                        if prefer_encrypted == 0 {
                                            "NOPREFERENCE"
                                        } else {
                                            "RESET"
                                        },
                                    );
                                    all_mutual = 0;
                                }
                            } else {
                                info!(context, 0, "[autocrypt] no peerstate for {}", state,);
                                can_encrypt = 0;
                                all_mutual = 0;
                            }
                            Ok(())
                        },
                    );
                    match res {
                        Ok(_) => {}
                        Err(err) => {
                            warn!(context, 0, "chat: failed to load peerstates: {:?}", err);
                        }
                    }

                    if 0 != can_encrypt {
                        if 0 != all_mutual {
                            do_guarantee_e2ee = 1;
                        } else if 0 != last_msg_in_chat_encrypted(context, &context.sql, (*chat).id)
                        {
                            do_guarantee_e2ee = 1;
                        }
                    }
                }
                if 0 != do_guarantee_e2ee {
                    dc_param_set_int(&mut (*msg).param, Param::GuranteeE2ee, 1);
                }
                dc_param_remove(&mut (*msg).param, Param::ErroneousE2ee);
                if 0 == dc_chat_is_self_talk(chat)
                    && 0 != get_parent_mime_headers(
                        chat,
                        &mut parent_rfc724_mid,
                        &mut parent_in_reply_to,
                        &mut parent_references,
                    )
                {
                    if !parent_rfc724_mid.is_null()
                        && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                    {
                        new_in_reply_to = dc_strdup(parent_rfc724_mid)
                    }
                    if !parent_references.is_null() {
                        let space: *mut libc::c_char;
                        space = strchr(parent_references, ' ' as i32);
                        if !space.is_null() {
                            *space = 0 as libc::c_char
                        }
                    }
                    if !parent_references.is_null()
                        && 0 != *parent_references.offset(0isize) as libc::c_int
                        && !parent_rfc724_mid.is_null()
                        && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                    {
                        new_references = dc_mprintf(
                            b"%s %s\x00" as *const u8 as *const libc::c_char,
                            parent_references,
                            parent_rfc724_mid,
                        )
                    } else if !parent_references.is_null()
                        && 0 != *parent_references.offset(0isize) as libc::c_int
                    {
                        new_references = dc_strdup(parent_references)
                    } else if !parent_in_reply_to.is_null()
                        && 0 != *parent_in_reply_to.offset(0isize) as libc::c_int
                        && !parent_rfc724_mid.is_null()
                        && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                    {
                        new_references = dc_mprintf(
                            b"%s %s\x00" as *const u8 as *const libc::c_char,
                            parent_in_reply_to,
                            parent_rfc724_mid,
                        )
                    } else if !parent_in_reply_to.is_null()
                        && 0 != *parent_in_reply_to.offset(0isize) as libc::c_int
                    {
                        new_references = dc_strdup(parent_in_reply_to)
                    }
                }

                // add independent location to database

                if 0 != dc_param_exists(&(*msg).param, Param::SetLatitude) {
                    if sql::execute(
                        context,
                        &context.sql,
                        "INSERT INTO locations \
                         (timestamp,from_id,chat_id, latitude,longitude,independent)\
                         VALUES (?,?,?, ?,?,1);",
                        params![
                            timestamp,
                            DC_CONTACT_ID_SELF as i32,
                            (*chat).id as i32,
                            dc_param_get_float(&(*msg).param, Param::SetLatitude)
                                .unwrap_or_defalut(),
                            dc_param_get_float(&(*msg).param, Param::SetLongitude)
                                .unwrap_or_default(),
                        ],
                    )
                    .is_ok()
                    {
                        location_id = sql::get_rowid2(
                            context,
                            &context.sql,
                            "locations",
                            "timestamp",
                            timestamp,
                            "from_id",
                            DC_CONTACT_ID_SELF as i32,
                        );
                    }
                }

                // add message to the database

                if sql::execute(
                        context,
                        &context.sql,
                        "INSERT INTO msgs (rfc724_mid, chat_id, from_id, to_id, timestamp, type, state, txt, param, hidden, mime_in_reply_to, mime_references, location_id) VALUES (?,?,?,?,?, ?,?,?,?,?, ?,?,?);",
                        params![
                            as_str(new_rfc724_mid),
                            (*chat).id as i32,
                            1i32,
                            to_id as i32,
                            timestamp,
                            (*msg).type_0,
                            (*msg).state,
                            if !(*msg).text.is_null() { Some(as_str((*msg).text)) } else { None },
                            if (*(*msg).param).packed.is_null() { None } else { Some(as_str((*(*msg).param).packed)) },
                            (*msg).hidden,
                            to_string(new_in_reply_to),
                            to_string(new_references),
                            location_id as i32,
                        ]
                    ).is_ok() {
                        msg_id = sql::get_rowid(
                            context,
                            &context.sql,
                            "msgs",
                            "rfc724_mid",
                            as_str(new_rfc724_mid),
                        );
                    } else {
                        error!(
                            context,
                            0,
                            "Cannot send message, cannot insert to database (chat #{}).",
                            (*chat).id,
                        );
                    }
            }
        }
    }

    free(parent_rfc724_mid as *mut libc::c_void);
    free(parent_in_reply_to as *mut libc::c_void);
    free(parent_references as *mut libc::c_void);
    free(new_rfc724_mid as *mut libc::c_void);
    free(new_in_reply_to as *mut libc::c_void);
    free(new_references as *mut libc::c_void);

    msg_id
}

// TODO should return bool /rtn
unsafe fn get_parent_mime_headers(
    chat: *const Chat,
    parent_rfc724_mid: *mut *mut libc::c_char,
    parent_in_reply_to: *mut *mut libc::c_char,
    parent_references: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut success = 0;

    if !(chat.is_null()
        || parent_rfc724_mid.is_null()
        || parent_in_reply_to.is_null()
        || parent_references.is_null())
    {
        success = (*chat)
            .context
            .sql
            .query_row(
                "SELECT rfc724_mid, mime_in_reply_to, mime_references \
                 FROM msgs WHERE timestamp=(SELECT max(timestamp) \
                 FROM msgs WHERE chat_id=? AND from_id!=?);",
                params![(*chat).id as i32, 1],
                |row| {
                    *parent_rfc724_mid = to_cstring(row.get::<_, String>(0)?);
                    *parent_in_reply_to = to_cstring(row.get::<_, String>(1)?);
                    *parent_references = to_cstring(row.get::<_, String>(2)?);
                    Ok(())
                },
            )
            .is_ok() as libc::c_int;

        if 0 == success {
            success = (*chat)
                .context
                .sql
                .query_row(
                    "SELECT rfc724_mid, mime_in_reply_to, mime_references \
                     FROM msgs WHERE timestamp=(SELECT min(timestamp) \
                     FROM msgs WHERE chat_id=? AND from_id==?);",
                    params![(*chat).id as i32, 1],
                    |row| {
                        *parent_rfc724_mid = to_cstring(row.get::<_, String>(0)?);
                        *parent_in_reply_to = to_cstring(row.get::<_, String>(1)?);
                        *parent_references = to_cstring(row.get::<_, String>(2)?);
                        Ok(())
                    },
                )
                .is_ok() as libc::c_int;
        }
    }
    success
}

pub unsafe fn dc_chat_is_self_talk(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0;
    }
    dc_param_exists(&(*chat).param, Param::Selftalk) as libc::c_int
}

/*******************************************************************************
 * Sending messages
 ******************************************************************************/
// TODO should return bool /rtn
unsafe fn last_msg_in_chat_encrypted(
    context: &Context,
    sql: &Sql,
    chat_id: uint32_t,
) -> libc::c_int {
    let packed: Option<String> = sql.query_row_col(
        context,
        "SELECT param  \
         FROM msgs  WHERE timestamp=(SELECT MAX(timestamp) FROM msgs WHERE chat_id=?)  \
         ORDER BY id DESC;",
        params![chat_id as i32],
        0,
    );

    if let Some(ref packed) = packed {
        match packed.parse() {
            Ok(param) => dc_param_exists(&param, Param::GuranteeE2ee) as libc::c_int,
            Err(err) => {
                error!(context, 0, "invalid params stored: '{}', {:?}", packed, err);
                0
            }
        }
    } else {
        0
    }
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_update_param(chat: *mut Chat) -> libc::c_int {
    sql::execute(
        (*chat).context,
        &(*chat).context.sql,
        "UPDATE chats SET param=? WHERE id=?",
        params![to_string((*(*chat).param).packed), (*chat).id as i32],
    )
    .is_ok() as libc::c_int
}

pub unsafe fn dc_is_contact_in_chat(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
) -> libc::c_int {
    /* this function works for group and for normal chats, however, it is more useful for group chats.
    DC_CONTACT_ID_SELF may be used to check, if the user itself is in a group chat (DC_CONTACT_ID_SELF is not added to normal chats) */

    context
        .sql
        .exists(
            "SELECT contact_id FROM chats_contacts WHERE chat_id=? AND contact_id=?;",
            params![chat_id as i32, contact_id as i32],
        )
        .unwrap_or_default() as libc::c_int
}

// Should return Result
pub fn dc_unarchive_chat(context: &Context, chat_id: u32) {
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET archived=0 WHERE id=?",
        params![chat_id as i32],
    )
    .ok();
}

pub unsafe fn dc_send_msg<'a>(
    context: &'a Context,
    chat_id: uint32_t,
    msg: *mut dc_msg_t<'a>,
) -> uint32_t {
    if msg.is_null() {
        return 0;
    }
    if (*msg).state != DC_STATE_OUT_PREPARING {
        if 0 == prepare_msg_common(context, chat_id, msg) {
            return 0;
        }
    } else {
        if chat_id != 0 && chat_id != (*msg).chat_id {
            return 0;
        }
        dc_update_msg_state(context, (*msg).id, DC_STATE_OUT_PENDING);
    }
    if 0 == dc_job_send_msg(context, (*msg).id) {
        return 0;
    }
    context.call_cb(
        Event::MSGS_CHANGED,
        (*msg).chat_id as uintptr_t,
        (*msg).id as uintptr_t,
    );

    if dc_param_exists(&(*msg).param, Param::SetLatitude) {
        context.call_cb(Event::LOCATION_CHANGED, DC_CONTACT_ID_SELF, 0);
    }

    if 0 == chat_id {
        let forwards = dc_param_get(&(*msg).param, Param::PrepForwards);
        if let Some(forwards) = forwards {
            let mut p = to_cstring(forwards);
            while 0 != *p {
                let id = strtol(p, &mut p, 10) as int32_t;
                if 0 == id {
                    // avoid hanging if user tampers with db
                    break;
                } else {
                    let copy = dc_get_msg(context, id as uint32_t);
                    if !copy.is_null() {
                        dc_send_msg(context, 0 as uint32_t, copy);
                    }
                    dc_msg_unref(copy);
                }
            }
            dc_param_remove(&mut (*msg).param, Param::PrepForwards);
            dc_msg_save_param_to_disk(msg);
            free(p as *mut _);
        }
    }

    (*msg).id
}

pub unsafe fn dc_send_text_msg(
    context: &Context,
    chat_id: uint32_t,
    text_to_send: *const libc::c_char,
) -> uint32_t {
    if chat_id <= 9 {
        warn!(
            context,
            0, "dc_send_text_msg: bad chat_id = {} <= 9", chat_id
        );
        return 0;
    }

    if text_to_send.is_null() {
        warn!(context, 0, "dc_send_text_msg: text_to_send is emtpy");
        return 0;
    }

    if let Err(err) = as_str_safe(text_to_send) {
        warn!(context, 0, "{}", err);
        return 0;
    }

    let mut msg = dc_msg_new(context, 10);
    (*msg).text = dc_strdup(text_to_send);
    let ret = dc_send_msg(context, chat_id, msg);
    dc_msg_unref(msg);
    ret
}

pub unsafe fn dc_set_draft(context: &Context, chat_id: uint32_t, msg: *mut dc_msg_t) {
    if chat_id <= 9i32 as libc::c_uint {
        return;
    }
    if 0 != set_draft_raw(context, chat_id, msg) {
        context.call_cb(Event::MSGS_CHANGED, chat_id as uintptr_t, 0i32 as uintptr_t);
    };
}

// TODO should return bool /rtn
unsafe fn set_draft_raw(context: &Context, chat_id: uint32_t, msg: *mut dc_msg_t) -> libc::c_int {
    let mut OK_TO_CONTINUE = true;
    // similar to as dc_set_draft() but does not emit an event
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let prev_draft_msg_id: uint32_t;
    let mut sth_changed: libc::c_int = 0i32;
    prev_draft_msg_id = get_draft_msg_id(context, chat_id);
    if 0 != prev_draft_msg_id {
        dc_delete_msg_from_db(context, prev_draft_msg_id);
        sth_changed = 1i32
    }
    // save new draft
    if !msg.is_null() {
        if (*msg).type_0 == DC_MSG_TEXT {
            if (*msg).text.is_null() || *(*msg).text.offset(0isize) as libc::c_int == 0i32 {
                OK_TO_CONTINUE = false;
            }
        } else if msgtype_has_file((*msg).type_0) {
            pathNfilename = dc_param_get(&(*msg).param, Param::File);
            if pathNfilename.is_none() {
                OK_TO_CONTINUE = false;
            } else if 0 != dc_msg_is_increation(msg)
                && !dc_is_blobdir_path_r(context, pathNfilename.as_ref().unwrap())
            {
                OK_TO_CONTINUE = false;
            } else {
                let mut pathNfilename = to_cstring(pathNfilename.unwrap());
                if !dc_make_rel_and_copy(context, &mut pathNfilename) {
                    OK_TO_CONTINUE = false;
                } else {
                    dc_param_set(&mut (*msg).param, Param::File, as_str(pathNfilename));
                }
                free(pathNfilename as *mut _);
            }
        } else {
            OK_TO_CONTINUE = false;
        }
        if OK_TO_CONTINUE {
            if sql::execute(
                context,
                &context.sql,
                "INSERT INTO msgs (chat_id, from_id, timestamp, type, state, txt, param, hidden) \
                 VALUES (?,?,?, ?,?,?,?,?);",
                params![
                    chat_id as i32,
                    1,
                    time(),
                    (*msg).type_0,
                    DC_STATE_OUT_DRAFT,
                    if !(*msg).text.is_null() {
                        as_str((*msg).text)
                    } else {
                        ""
                    },
                    to_string((*(*msg).param).packed),
                    1,
                ],
            )
            .is_ok()
            {
                sth_changed = 1;
            }
        }
    }
    free(pathNfilename as *mut libc::c_void);
    sth_changed
}

fn get_draft_msg_id(context: &Context, chat_id: u32) -> u32 {
    let draft_msg_id: i32 = context
        .sql
        .query_row_col(
            context,
            "SELECT id FROM msgs WHERE chat_id=? AND state=?;",
            params![chat_id as i32, DC_STATE_OUT_DRAFT],
            0,
        )
        .unwrap_or_default();

    draft_msg_id as u32
}

pub unsafe fn dc_get_draft(context: &Context, chat_id: uint32_t) -> *mut dc_msg_t {
    let draft_msg_id: uint32_t;
    let draft_msg: *mut dc_msg_t;
    if chat_id <= 9i32 as libc::c_uint {
        return 0 as *mut dc_msg_t;
    }
    draft_msg_id = get_draft_msg_id(context, chat_id);
    if draft_msg_id == 0i32 as libc::c_uint {
        return 0 as *mut dc_msg_t;
    }
    draft_msg = dc_msg_new_untyped(context);
    if !dc_msg_load_from_db(draft_msg, context, draft_msg_id) {
        dc_msg_unref(draft_msg);
        return 0 as *mut dc_msg_t;
    }

    draft_msg
}

pub fn dc_get_chat_msgs(
    context: &Context,
    chat_id: uint32_t,
    flags: uint32_t,
    marker1before: uint32_t,
) -> *mut dc_array_t {
    let mut ret = dc_array_t::new(512);

    let mut last_day = 0;
    let cnv_to_local = dc_gm2local_offset();

    let process_row = |row: &rusqlite::Row| Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?));
    let process_rows = |rows: rusqlite::MappedRows<_>| {
        for row in rows {
            let (curr_id, ts) = row?;
            if curr_id as u32 == marker1before {
                ret.add_id(1);
            }
            if 0 != flags & 0x1 {
                let curr_local_timestamp = ts + cnv_to_local;
                let curr_day = (curr_local_timestamp / 86400) as libc::c_int;
                if curr_day != last_day {
                    ret.add_id(9);
                    last_day = curr_day;
                }
            }
            ret.add_id(curr_id as u32);
        }
        Ok(())
    };

    let success = if chat_id == 1 {
        let show_emails = context
            .sql
            .get_config_int(context, "show_emails")
            .unwrap_or_default();
        context.sql.query_map(
            "SELECT m.id, m.timestamp FROM msgs m \
             LEFT JOIN chats ON m.chat_id=chats.id \
             LEFT JOIN contacts ON m.from_id=contacts.id WHERE m.from_id!=1   \
             AND m.from_id!=2   \
             AND m.hidden=0    \
             AND chats.blocked=2   \
             AND contacts.blocked=0   \
             AND m.msgrmsg>=?  \
             ORDER BY m.timestamp,m.id;",
            params![if show_emails == 2 { 0 } else { 1 }],
            process_row,
            process_rows,
        )
    } else if chat_id == 5 {
        context.sql.query_map(
            "SELECT m.id, m.timestamp FROM msgs m \
             LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.starred=1    \
             AND m.hidden=0    \
             AND ct.blocked=0 \
             ORDER BY m.timestamp,m.id;",
            params![],
            process_row,
            process_rows,
        )
    } else {
        context.sql.query_map(
            "SELECT m.id, m.timestamp FROM msgs m \
             WHERE m.chat_id=?    \
             AND m.hidden=0  \
             ORDER BY m.timestamp,m.id;",
            params![chat_id as i32],
            process_row,
            process_rows,
        )
    };

    if success.is_ok() {
        ret.as_ptr()
    } else {
        0 as *mut dc_array_t
    }
}

pub fn dc_get_msg_cnt(context: &Context, chat_id: u32) -> libc::c_int {
    context
        .sql
        .query_row_col(
            context,
            "SELECT COUNT(*) FROM msgs WHERE chat_id=?;",
            params![chat_id as i32],
            0,
        )
        .unwrap_or_default()
}

pub fn dc_get_fresh_msg_cnt(context: &Context, chat_id: u32) -> libc::c_int {
    context
        .sql
        .query_row_col(
            context,
            "SELECT COUNT(*) FROM msgs  \
             WHERE state=10   \
             AND hidden=0    \
             AND chat_id=?;",
            params![chat_id as i32],
            0,
        )
        .unwrap_or_default()
}

pub fn dc_marknoticed_chat(context: &Context, chat_id: u32) -> bool {
    if !context
        .sql
        .exists(
            "SELECT id FROM msgs  WHERE chat_id=? AND state=?;",
            params![chat_id as i32, DC_STATE_IN_FRESH],
        )
        .unwrap_or_default()
    {
        return false;
    }
    if sql::execute(
        context,
        &context.sql,
        "UPDATE msgs    \
         SET state=13 WHERE chat_id=? AND state=10;",
        params![chat_id as i32],
    )
    .is_err()
    {
        return false;
    }
    context.call_cb(Event::MSGS_CHANGED, 0 as uintptr_t, 0 as uintptr_t);
    true
}

pub fn dc_marknoticed_all_chats(context: &Context) -> bool {
    if !context
        .sql
        .exists(
            "SELECT id FROM msgs  \
             WHERE state=10;",
            params![],
        )
        .unwrap_or_default()
    {
        return false;
    }

    if sql::execute(
        context,
        &context.sql,
        "UPDATE msgs    \
         SET state=13 WHERE state=10;",
        params![],
    )
    .is_err()
    {
        return false;
    }

    context.call_cb(Event::MSGS_CHANGED, 0 as uintptr_t, 0 as uintptr_t);

    true
}

pub fn dc_get_chat_media(
    context: &Context,
    chat_id: uint32_t,
    msg_type: libc::c_int,
    msg_type2: libc::c_int,
    msg_type3: libc::c_int,
) -> *mut dc_array_t {
    context.sql.query_map(
        "SELECT id FROM msgs WHERE chat_id=? AND (type=? OR type=? OR type=?) ORDER BY timestamp, id;",
        params![
            chat_id as i32,
            msg_type,
            if msg_type2 > 0 {
                msg_type2
            } else {
                msg_type
            }, if msg_type3 > 0 {
                msg_type3
            } else {
                msg_type
            },
        ],
        |row| row.get::<_, i32>(0),
        |ids| {
            let mut ret = dc_array_t::new(100);
            for id in ids {
                ret.add_id(id? as u32);
            }
            Ok(ret.as_ptr())
        }
    ).unwrap_or_else(|_| std::ptr::null_mut())
}

pub unsafe fn dc_get_next_media(
    context: &Context,
    curr_msg_id: uint32_t,
    dir: libc::c_int,
    msg_type: libc::c_int,
    msg_type2: libc::c_int,
    msg_type3: libc::c_int,
) -> uint32_t {
    let mut ret_msg_id: uint32_t = 0i32 as uint32_t;
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut list: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut i: libc::c_int;
    let cnt: libc::c_int;

    if dc_msg_load_from_db(msg, context, curr_msg_id) {
        list = dc_get_chat_media(
            context,
            (*msg).chat_id,
            if msg_type > 0i32 {
                msg_type
            } else {
                (*msg).type_0
            },
            msg_type2,
            msg_type3,
        );
        if !list.is_null() {
            cnt = dc_array_get_cnt(list) as libc::c_int;
            i = 0i32;
            while i < cnt {
                if curr_msg_id == dc_array_get_id(list, i as size_t) {
                    if dir > 0i32 {
                        if i + 1i32 < cnt {
                            ret_msg_id = dc_array_get_id(list, (i + 1i32) as size_t)
                        }
                    } else if dir < 0i32 {
                        if i - 1i32 >= 0i32 {
                            ret_msg_id = dc_array_get_id(list, (i - 1i32) as size_t)
                        }
                    }
                    break;
                } else {
                    i += 1
                }
            }
        }
    }

    dc_array_unref(list);
    dc_msg_unref(msg);
    ret_msg_id
}

pub fn dc_archive_chat(context: &Context, chat_id: u32, archive: libc::c_int) -> bool {
    if chat_id <= 9 || archive != 0 && archive != 1 {
        return true;
    }
    if 0 != archive {
        if sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET state=? WHERE chat_id=? AND state=?;",
            params![DC_STATE_IN_NOTICED, chat_id as i32, DC_STATE_IN_FRESH],
        )
        .is_err()
        {
            return false;
        }
    }
    if sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET archived=? WHERE id=?;",
        params![archive, chat_id as i32],
    )
    .is_err()
    {
        return false;
    }
    context.call_cb(Event::MSGS_CHANGED, 0 as uintptr_t, 0 as uintptr_t);

    true
}

pub fn dc_delete_chat(context: &Context, chat_id: u32) -> bool {
    /* Up to 2017-11-02 deleting a group also implied leaving it, see above why we have changed this. */
    if chat_id <= 9 {
        return false;
    }
    let obj = unsafe { dc_chat_new(context) };
    if !dc_chat_load_from_db(obj, chat_id) {
        return false;
    }
    unsafe { dc_chat_unref(obj) };

    if sql::execute(
        context,
        &context.sql,
        "DELETE FROM msgs_mdns WHERE msg_id IN (SELECT id FROM msgs WHERE chat_id=?);",
        params![chat_id as i32],
    )
    .is_err()
    {
        return false;
    }
    if sql::execute(
        context,
        &context.sql,
        "DELETE FROM msgs WHERE chat_id=?;",
        params![chat_id as i32],
    )
    .is_err()
    {
        return false;
    }
    if sql::execute(
        context,
        &context.sql,
        "DELETE FROM chats_contacts WHERE chat_id=?;",
        params![chat_id as i32],
    )
    .is_err()
    {
        return false;
    }
    if sql::execute(
        context,
        &context.sql,
        "DELETE FROM chats WHERE id=?;",
        params![chat_id as i32],
    )
    .is_err()
    {
        return false;
    }

    context.call_cb(Event::MSGS_CHANGED, 0 as uintptr_t, 0 as uintptr_t);

    dc_job_kill_action(context, 105);
    unsafe { dc_job_add(context, 105, 0, 0 as *const libc::c_char, 10) };

    true
}

pub fn dc_get_chat_contacts(context: &Context, chat_id: u32) -> *mut dc_array_t {
    /* Normal chats do not include SELF.  Group chats do (as it may happen that one is deleted from a
    groupchat but the chats stays visible, moreover, this makes displaying lists easier) */

    if chat_id == 1 {
        return std::ptr::null_mut();
    }

    // we could also create a list for all contacts in the deaddrop by searching contacts belonging to chats with
    // chats.blocked=2, however, currently this is not needed

    context
        .sql
        .query_map(
            "SELECT cc.contact_id FROM chats_contacts cc \
             LEFT JOIN contacts c ON c.id=cc.contact_id WHERE cc.chat_id=? \
             ORDER BY c.id=1, LOWER(c.name||c.addr), c.id;",
            params![chat_id as i32],
            |row| row.get::<_, i32>(0),
            |ids| {
                let mut ret = dc_array_t::new(100);

                for id in ids {
                    ret.add_id(id? as u32);
                }

                Ok(ret.as_ptr())
            },
        )
        .unwrap_or_else(|_| std::ptr::null_mut())
}

pub unsafe fn dc_get_chat(context: &Context, chat_id: uint32_t) -> *mut Chat {
    let mut success: libc::c_int = 0i32;
    let obj: *mut Chat = dc_chat_new(context);

    if dc_chat_load_from_db(obj, chat_id) {
        success = 1i32
    }

    if 0 != success {
        return obj;
    } else {
        dc_chat_unref(obj);
        return 0 as *mut Chat;
    };
}

// handle group chats
pub unsafe fn dc_create_group_chat(
    context: &Context,
    verified: libc::c_int,
    chat_name: *const libc::c_char,
) -> u32 {
    let mut chat_id = 0;

    if chat_name.is_null() || *chat_name.offset(0) as libc::c_int == 0 {
        return 0;
    }
    let draft_txt =
        CString::new(context.stock_string_repl_str(StockMessage::NewGroupDraft, as_str(chat_name)))
            .unwrap();
    let grpid = as_str(dc_create_id());
    if sql::execute(
        context,
        &context.sql,
        "INSERT INTO chats (type, name, grpid, param) VALUES(?, ?, ?, \'U=1\');",
        params![
            if verified != 0 { 130 } else { 120 },
            as_str(chat_name),
            grpid
        ],
    )
    .is_ok()
    {
        chat_id = sql::get_rowid(context, &context.sql, "chats", "grpid", grpid);
        if chat_id != 0 {
            if 0 != dc_add_to_chat_contacts_table(context, chat_id, 1) {
                let draft_msg = dc_msg_new(context, 10);
                dc_msg_set_text(draft_msg, draft_txt.as_ptr());
                set_draft_raw(context, chat_id, draft_msg);
                dc_msg_unref(draft_msg);
            }
        }
    }
    if 0 != chat_id {
        context.call_cb(Event::MSGS_CHANGED, 0 as uintptr_t, 0 as uintptr_t);
    }

    chat_id
}

/* you MUST NOT modify this or the following strings */
// Context functions to work with chats
// TODO should return bool /rtn
pub fn dc_add_to_chat_contacts_table(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
) -> libc::c_int {
    // add a contact to a chat; the function does not check the type or if any of the record exist or are already
    // added to the chat!
    sql::execute(
        context,
        &context.sql,
        "INSERT INTO chats_contacts (chat_id, contact_id) VALUES(?, ?)",
        params![chat_id as i32, contact_id as i32],
    )
    .is_ok() as libc::c_int
}

pub unsafe fn dc_add_contact_to_chat(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
) -> libc::c_int {
    dc_add_contact_to_chat_ex(context, chat_id, contact_id, 0)
}

// TODO should return bool /rtn
pub unsafe fn dc_add_contact_to_chat_ex(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
    flags: libc::c_int,
) -> libc::c_int {
    let mut OK_TO_CONTINUE = true;
    let mut success: libc::c_int = 0;
    let contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);

    if !(contact.is_null() || chat_id <= 9 as libc::c_uint) {
        dc_reset_gossiped_timestamp(context, chat_id);
        /*this also makes sure, not contacts are added to special or normal chats*/
        if !(0 == real_group_exists(context, chat_id)
            || !dc_real_contact_exists(context, contact_id) && contact_id != 1 as libc::c_uint
            || !dc_chat_load_from_db(chat, chat_id))
        {
            if !(dc_is_contact_in_chat(context, chat_id, 1 as uint32_t) == 1) {
                log_event!(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0,
                    "Cannot add contact to group; self not in group.",
                );
            } else {
                /* we should respect this - whatever we send to the group, it gets discarded anyway! */
                if 0 != flags & 0x1
                    && dc_param_get_int(&(*chat).param, Param::Unpromoted).unwrap_or_default() == 1
                {
                    dc_param_remove(&mut (*chat).param, Param::Unpromoted);
                    dc_chat_update_param(chat);
                }
                let self_addr = context
                    .sql
                    .get_config(context, "configured_addr")
                    .unwrap_or_default();
                if as_str((*contact).addr) != &self_addr {
                    // ourself is added using DC_CONTACT_ID_SELF, do not add it explicitly.
                    // if SELF is not in the group, members cannot be added at all.

                    if 0 != dc_is_contact_in_chat(context, chat_id, contact_id) {
                        if 0 == flags & 0x1 {
                            success = 1;
                            OK_TO_CONTINUE = false;
                        }
                    } else {
                        // else continue and send status mail
                        if (*chat).type_0 == 130 {
                            if dc_contact_is_verified(contact) != 2 {
                                error!(
                                    context, 0,
                                    "Only bidirectional verified contacts can be added to verified groups."
                                );
                                OK_TO_CONTINUE = false;
                            }
                        }
                        if OK_TO_CONTINUE {
                            if 0 == dc_add_to_chat_contacts_table(context, chat_id, contact_id) {
                                OK_TO_CONTINUE = false;
                            }
                        }
                    }
                    if OK_TO_CONTINUE {
                        if dc_param_get_int(&(*chat).param, Param::Unpromoted).unwrap_or_default()
                            == 0
                        {
                            (*msg).type_0 = DC_MSG_TEXT;
                            (*msg).text = to_cstring(context.stock_system_msg(
                                StockMessage::MsgAddMember,
                                as_str((*contact).addr),
                                "",
                                DC_CONTACT_ID_SELF as uint32_t,
                            ));
                            dc_param_set_int(&mut (*msg).param, Param::Cmd, 4);
                            if !(*contact).addr.is_null() {
                                dc_param_set(
                                    &mut (*msg).param,
                                    Param::Arg,
                                    as_str((*contact).addr),
                                );
                            }
                            dc_param_set_int(&mut (*msg).param, Param::Arg2, flags);
                            (*msg).id = dc_send_msg(context, chat_id, msg);
                            context.call_cb(
                                Event::MSGS_CHANGED,
                                chat_id as uintptr_t,
                                (*msg).id as uintptr_t,
                            );
                        }
                        context.call_cb(Event::MSGS_CHANGED, chat_id as uintptr_t, 0 as uintptr_t);
                        success = 1;
                    }
                }
            }
        }
    }
    dc_chat_unref(chat);
    dc_contact_unref(contact);
    dc_msg_unref(msg);

    success
}

// TODO should return bool /rtn
fn real_group_exists(context: &Context, chat_id: u32) -> libc::c_int {
    // check if a group or a verified group exists under the given ID
    if !context.sql.is_open() || chat_id <= 9 {
        return 02;
    }

    context
        .sql
        .exists(
            "SELECT id FROM chats  WHERE id=?    AND (type=120 OR type=130);",
            params![chat_id as i32],
        )
        .unwrap_or_default() as libc::c_int
}

pub fn dc_reset_gossiped_timestamp(context: &Context, chat_id: u32) {
    dc_set_gossiped_timestamp(context, chat_id, 0);
}

// Should return Result
pub fn dc_set_gossiped_timestamp(context: &Context, chat_id: u32, timestamp: i64) {
    if 0 != chat_id {
        info!(
            context,
            0, "set gossiped_timestamp for chat #{} to {}.", chat_id, timestamp,
        );

        sql::execute(
            context,
            &context.sql,
            "UPDATE chats SET gossiped_timestamp=? WHERE id=?;",
            params![timestamp, chat_id as i32],
        )
        .ok();
    } else {
        info!(
            context,
            0, "set gossiped_timestamp for all chats to {}.", timestamp,
        );
        sql::execute(
            context,
            &context.sql,
            "UPDATE chats SET gossiped_timestamp=?;",
            params![timestamp],
        )
        .ok();
    }
}

// TODO should return bool /rtn
pub unsafe fn dc_remove_contact_from_chat(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
) -> libc::c_int {
    let mut success: libc::c_int = 0;
    let contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);

    if !(chat_id <= 9 as libc::c_uint
        || contact_id <= 9 as libc::c_uint && contact_id != 1 as libc::c_uint)
    {
        /* we do not check if "contact_id" exists but just delete all records with the id from chats_contacts */
        /* this allows to delete pending references to deleted contacts.  Of course, this should _not_ happen. */
        if !(0 == real_group_exists(context, chat_id) || !dc_chat_load_from_db(chat, chat_id)) {
            if !(dc_is_contact_in_chat(context, chat_id, 1 as uint32_t) == 1) {
                log_event!(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0,
                    "Cannot remove contact from chat; self not in group.",
                );
            } else {
                /* we should respect this - whatever we send to the group, it gets discarded anyway! */
                if !contact.is_null() {
                    if dc_param_get_int(&(*chat).param, Param::Unpromoted).unwrap_or_default() == 0
                    {
                        (*msg).type_0 = DC_MSG_TEXT;
                        if (*contact).id == 1 as libc::c_uint {
                            dc_set_group_explicitly_left(context, (*chat).grpid);
                            (*msg).text = to_cstring(context.stock_system_msg(
                                StockMessage::MsgGroupLeft,
                                "",
                                "",
                                DC_CONTACT_ID_SELF as u32,
                            ));
                        } else {
                            (*msg).text = to_cstring(context.stock_system_msg(
                                StockMessage::MsgDelMember,
                                as_str((*contact).addr),
                                "",
                                DC_CONTACT_ID_SELF as u32,
                            ));
                        }
                        dc_param_set_int(&mut (*msg).param, Param::Cmd, 5);
                        if !(*contact).addr.is_null() {
                            dc_param_set(&mut (*msg).param, Param::Arg, as_str((*contact).addr));
                        }
                        (*msg).id = dc_send_msg(context, chat_id, msg);
                        context.call_cb(
                            Event::MSGS_CHANGED,
                            chat_id as uintptr_t,
                            (*msg).id as uintptr_t,
                        );
                    }
                }
                if sql::execute(
                    context,
                    &context.sql,
                    "DELETE FROM chats_contacts WHERE chat_id=? AND contact_id=?;",
                    params![chat_id as i32, contact_id as i32],
                )
                .is_ok()
                {
                    context.call_cb(Event::CHAT_MODIFIED, chat_id as uintptr_t, 0 as uintptr_t);
                    success = 1;
                }
            }
        }
    }

    dc_chat_unref(chat);
    dc_contact_unref(contact);
    dc_msg_unref(msg);

    success
}

// Should return Result
pub fn dc_set_group_explicitly_left(context: &Context, grpid: *const libc::c_char) {
    if 0 == dc_is_group_explicitly_left(context, grpid) {
        sql::execute(
            context,
            &context.sql,
            "INSERT INTO leftgrps (grpid) VALUES(?);",
            params![as_str(grpid)],
        )
        .ok();
    }
}

// TODO should return bool /rtn
pub fn dc_is_group_explicitly_left(context: &Context, grpid: *const libc::c_char) -> libc::c_int {
    context
        .sql
        .exists(
            "SELECT id FROM leftgrps WHERE grpid=?;",
            params![as_str(grpid)],
        )
        .unwrap_or_default() as libc::c_int
}

// TODO should return bool /rtn
pub unsafe fn dc_set_chat_name(
    context: &Context,
    chat_id: uint32_t,
    new_name: *const libc::c_char,
) -> libc::c_int {
    /* the function only sets the names of group chats; normal chats get their names from the contacts */
    let mut success: libc::c_int = 0i32;
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);

    if !(new_name.is_null()
        || *new_name.offset(0isize) as libc::c_int == 0i32
        || chat_id <= 9i32 as libc::c_uint)
    {
        if !(0i32 == real_group_exists(context, chat_id) || !dc_chat_load_from_db(chat, chat_id)) {
            if strcmp((*chat).name, new_name) == 0i32 {
                success = 1i32
            } else if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                log_event!(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0,
                    "Cannot set chat name; self not in group",
                );
            } else {
                /* we should respect this - whatever we send to the group, it gets discarded anyway! */
                if sql::execute(
                    context,
                    &context.sql,
                    format!(
                        "UPDATE chats SET name='{}' WHERE id={};",
                        as_str(new_name),
                        chat_id as i32
                    ),
                    params![],
                )
                .is_ok()
                {
                    if dc_param_get_int(&(*chat).param, Param::Unpromoted).unwrap_or_default() == 0
                    {
                        (*msg).type_0 = DC_MSG_TEXT;
                        (*msg).text = to_cstring(context.stock_system_msg(
                            StockMessage::MsgGrpName,
                            as_str((*chat).name),
                            as_str(new_name),
                            DC_CONTACT_ID_SELF as u32,
                        ));
                        dc_param_set_int(&mut (*msg).param, Param::Cmd, 2);
                        if !(*chat).name.is_null() {
                            dc_param_set(&mut (*msg).param, Param::Arg, (*chat).name);
                        }
                        (*msg).id = dc_send_msg(context, chat_id, msg);
                        context.call_cb(
                            Event::MSGS_CHANGED,
                            chat_id as uintptr_t,
                            (*msg).id as uintptr_t,
                        );
                    }
                    context.call_cb(
                        Event::CHAT_MODIFIED,
                        chat_id as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    success = 1i32
                }
            }
        }
    }

    dc_chat_unref(chat);
    dc_msg_unref(msg);

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_set_chat_profile_image(
    context: &Context,
    chat_id: uint32_t,
    new_image: *const libc::c_char,
) -> libc::c_int {
    let mut OK_TO_CONTINUE = true;
    let mut success: libc::c_int = 0i32;
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut new_image_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(chat_id <= 9i32 as libc::c_uint) {
        if !(0i32 == real_group_exists(context, chat_id) || !dc_chat_load_from_db(chat, chat_id)) {
            if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                log_event!(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0,
                    "Cannot set chat profile image; self not in group.",
                );
            } else {
                /* we should respect this - whatever we send to the group, it gets discarded anyway! */
                if !new_image.is_null() {
                    new_image_rel = dc_strdup(new_image);
                    if !dc_make_rel_and_copy(context, &mut new_image_rel) {
                        OK_TO_CONTINUE = false;
                    }
                }
                if OK_TO_CONTINUE {
                    dc_param_set(
                        &mut (*chat).param,
                        Param::ProfileImage,
                        as_str(new_image_rel),
                    );
                    if !(0 == dc_chat_update_param(chat)) {
                        if dc_param_get_int(&(*chat).param, Param::Unpromoted).unwrap_or_default()
                            == 0
                        {
                            dc_param_set_int(&mut (*msg).param, Param::Cmd, 3);
                            dc_param_set(&mut (*msg).param, Param::Arg, new_image_rel);
                            (*msg).type_0 = DC_MSG_TEXT;
                            (*msg).text = to_cstring(context.stock_system_msg(
                                if !new_image_rel.is_null() {
                                    StockMessage::MsgGrpImgChanged
                                } else {
                                    StockMessage::MsgGrpImgDeleted
                                },
                                "",
                                "",
                                DC_CONTACT_ID_SELF as uint32_t,
                            ));
                            (*msg).id = dc_send_msg(context, chat_id, msg);
                            context.call_cb(
                                Event::MSGS_CHANGED,
                                chat_id as uintptr_t,
                                (*msg).id as uintptr_t,
                            );
                        }
                        context.call_cb(
                            Event::CHAT_MODIFIED,
                            chat_id as uintptr_t,
                            0i32 as uintptr_t,
                        );
                        success = 1i32
                    }
                }
            }
        }
    }

    dc_chat_unref(chat);
    dc_msg_unref(msg);
    free(new_image_rel as *mut libc::c_void);

    success
}

pub unsafe fn dc_forward_msgs(
    context: &Context,
    msg_ids: *const u32,
    msg_cnt: libc::c_int,
    chat_id: u32,
) {
    if msg_ids.is_null() || msg_cnt <= 0 || chat_id <= 9 {
        return;
    }

    let msg = dc_msg_new_untyped(context);
    let chat = dc_chat_new(context);
    let contact = dc_contact_new(context);
    let created_db_entries = carray_new(16);
    let mut curr_timestamp: i64;

    let mut original_param = dc_param_new();
    dc_unarchive_chat(context, chat_id);
    if dc_chat_load_from_db(chat, chat_id) {
        curr_timestamp = dc_create_smeared_timestamps(context, msg_cnt);
        let idsstr = std::slice::from_raw_parts(msg_ids, msg_cnt as usize)
            .iter()
            .enumerate()
            .fold(
                String::with_capacity(2 * msg_cnt as usize),
                |acc, (i, n)| (if i == 0 { acc } else { acc + "," }) + &n.to_string(),
            );

        let ids = context
            .sql
            .query_map(
                format!(
                    "SELECT id FROM msgs WHERE id IN({}) ORDER BY timestamp,id",
                    idsstr
                ),
                params![],
                |row| row.get::<_, i32>(0),
                |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .unwrap(); // TODO: better error handling

        for id in ids {
            let src_msg_id = id;
            if !dc_msg_load_from_db(msg, context, src_msg_id as u32) {
                break;
            }
            dc_param_set_packed(original_param, as_str((*(*msg).param).packed));
            if (*msg).from_id != 1 {
                dc_param_set_int(&mut (*msg).param, Param::Forwarded, 1);
            }
            dc_param_remove(&mut (*msg).param, Param::GuranteeE2ee);
            dc_param_remove(&mut (*msg).param, Param::ForcePlaintext);
            dc_param_remove(&mut (*msg).param, Param::Cmd);

            let new_msg_id: uint32_t;
            if (*msg).state == DC_STATE_OUT_PREPARING {
                let fresh9 = curr_timestamp;
                curr_timestamp = curr_timestamp + 1;
                new_msg_id = prepare_msg_raw(context, chat, msg, fresh9);
                let save_param = (*msg).param.clone();
                (*msg).param = original_param;
                (*msg).id = src_msg_id as uint32_t;

                if let Some(old_fwd) = dc_param_get(&(*msg).param, Param::PrepForwards) {
                    let new_fwd = format!("{} {}", old_fwd, new_msg_id);
                    dc_param_set(&mut (*msg).param, Param::PrepForwards, new_fwd);
                } else {
                    dc_param_set(
                        &mut (*msg).param,
                        Param::PrepForwards,
                        new_msg_id.to_string(),
                    );
                }

                dc_msg_save_param_to_disk(msg);
                (*msg).param = save_param;
            } else {
                (*msg).state = DC_STATE_OUT_PENDING;
                let fresh10 = curr_timestamp;
                curr_timestamp = curr_timestamp + 1;
                new_msg_id = prepare_msg_raw(context, chat, msg, fresh10);
                dc_job_send_msg(context, new_msg_id);
            }
            carray_add(
                created_db_entries,
                chat_id as uintptr_t as *mut libc::c_void,
                0 as *mut libc::c_uint,
            );
            carray_add(
                created_db_entries,
                new_msg_id as uintptr_t as *mut libc::c_void,
                0 as *mut libc::c_uint,
            );
        }
    }

    if !created_db_entries.is_null() {
        let mut i = 0u32;
        let icnt = carray_count(created_db_entries);
        while i < icnt {
            context.call_cb(
                Event::MSGS_CHANGED,
                carray_get(created_db_entries, i) as uintptr_t,
                carray_get(created_db_entries, i.wrapping_add(1)) as uintptr_t,
            );
            i = i.wrapping_add(2);
        }
        carray_free(created_db_entries);
    }
    dc_contact_unref(contact);
    dc_msg_unref(msg);
    dc_chat_unref(chat);
}

pub unsafe fn dc_chat_get_id(chat: *const Chat) -> uint32_t {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32 as uint32_t;
    }
    (*chat).id
}

pub unsafe fn dc_chat_get_type(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    (*chat).type_0
}

pub unsafe fn dc_chat_get_name(chat: *const Chat) -> *mut libc::c_char {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char);
    }
    dc_strdup((*chat).name)
}

pub unsafe fn dc_chat_get_subtitle(chat: *const Chat) -> *mut libc::c_char {
    /* returns either the address or the number of chat members */
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char);
    }

    let mut ret: *mut libc::c_char = std::ptr::null_mut();
    if (*chat).type_0 == 100 && 0 != dc_param_exists(&(*chat).param, Param::Selftalk) {
        ret = to_cstring((*chat).context.stock_str(StockMessage::SelfTalkSubTitle));
    } else if (*chat).type_0 == 100 {
        let ret_raw: String = (*chat)
            .context
            .sql
            .query_row_col(
                (*chat).context,
                "SELECT c.addr FROM chats_contacts cc  \
                 LEFT JOIN contacts c ON c.id=cc.contact_id  \
                 WHERE cc.chat_id=?;",
                params![(*chat).id as i32],
                0,
            )
            .unwrap_or_else(|| "Err".into());
        ret = to_cstring(ret_raw);
    } else if (*chat).type_0 == 120 || (*chat).type_0 == 130 {
        if (*chat).id == 1 {
            ret = to_cstring((*chat).context.stock_str(StockMessage::DeadDrop));
        } else {
            let cnt = dc_get_chat_contact_cnt((*chat).context, (*chat).id);
            ret = to_cstring(
                (*chat)
                    .context
                    .stock_string_repl_int(StockMessage::Member, cnt),
            );
        }
    }
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char)
    };
}

pub fn dc_get_chat_contact_cnt(context: &Context, chat_id: u32) -> libc::c_int {
    context
        .sql
        .query_row_col(
            context,
            "SELECT COUNT(*) FROM chats_contacts WHERE chat_id=?;",
            params![chat_id as i32],
            0,
        )
        .unwrap_or_default()
}

pub unsafe fn dc_chat_get_profile_image(chat: *const Chat) -> *mut libc::c_char {
    let mut image_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut image_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contacts: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        image_rel =
            to_cstring(dc_param_get(&(*chat).param, Param::ProfileImage).unwrap_or_default());
        if !image_rel.is_null() && 0 != *image_rel.offset(0isize) as libc::c_int {
            image_abs = dc_get_abs_path((*chat).context, image_rel)
        } else if (*chat).type_0 == 100i32 {
            contacts = dc_get_chat_contacts((*chat).context, (*chat).id);
            if !(*contacts).is_empty() {
                contact = dc_get_contact((*chat).context, (*contacts).get_id(0));
                image_abs = dc_contact_get_profile_image(contact)
            }
        }
    }

    free(image_rel as *mut libc::c_void);
    dc_array_unref(contacts);
    dc_contact_unref(contact);

    image_abs
}

pub unsafe fn dc_chat_get_color(chat: *const Chat) -> uint32_t {
    let mut color: uint32_t = 0i32 as uint32_t;
    let mut contacts: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        if (*chat).type_0 == 100i32 {
            contacts = dc_get_chat_contacts((*chat).context, (*chat).id);
            if !(*contacts).is_empty() {
                contact = dc_get_contact((*chat).context, (*contacts).get_id(0));
                color = dc_str_to_color((*contact).addr) as uint32_t
            }
        } else {
            color = dc_str_to_color((*chat).name) as uint32_t
        }
    }

    dc_array_unref(contacts);
    dc_contact_unref(contact);

    color
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_get_archived(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    (*chat).archived
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_is_unpromoted(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0;
    }
    dc_param_get_int(&(*chat).param, Param::Unpromoted).unwrap_or_default() as libc::c_int
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_is_verified(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    ((*chat).type_0 == 130i32) as libc::c_int
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_is_sending_locations(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    (*chat).is_sending_locations
}

pub fn dc_get_chat_cnt(context: &Context) -> usize {
    if context.sql.is_open() {
        /* no database, no chats - this is no error (needed eg. for information) */
        context
            .sql
            .query_row_col::<_, isize>(
                context,
                "SELECT COUNT(*) FROM chats WHERE id>9 AND blocked=0;",
                params![],
                0,
            )
            .unwrap_or_default() as usize
    } else {
        0
    }
}

pub unsafe fn dc_get_chat_id_by_grpid(
    context: &Context,
    grpid: *const libc::c_char,
    ret_blocked: *mut libc::c_int,
    ret_verified: *mut libc::c_int,
) -> u32 {
    if !ret_blocked.is_null() {
        *ret_blocked = 0;
    }
    if !ret_verified.is_null() {
        *ret_verified = 0;
    }

    context
        .sql
        .query_row(
            "SELECT id, blocked, type FROM chats WHERE grpid=?;",
            params![as_str(grpid)],
            |row| {
                let chat_id = row.get(0)?;
                if !ret_blocked.is_null() {
                    *ret_blocked = row.get(1)?;
                }
                if !ret_verified.is_null() {
                    let v: i32 = row.get(2)?;
                    *ret_verified = (v == 130) as libc::c_int;
                }
                Ok(chat_id)
            },
        )
        .unwrap_or_default()
}

pub fn dc_add_device_msg(context: &Context, chat_id: uint32_t, text: *const libc::c_char) {
    if text.is_null() {
        return;
    }
    let rfc724_mid = unsafe {
        dc_create_outgoing_rfc724_mid(
            0 as *const libc::c_char,
            b"@device\x00" as *const u8 as *const libc::c_char,
        )
    };

    if context.sql.execute(
        "INSERT INTO msgs (chat_id,from_id,to_id, timestamp,type,state, txt,rfc724_mid) VALUES (?,?,?, ?,?,?, ?,?);",
        params![
            chat_id as i32,
            2,
            2,
            unsafe {dc_create_smeared_timestamp(context)},
            DC_MSG_TEXT,
            DC_STATE_IN_NOTICED,
            as_str(text),
            as_str(rfc724_mid),
        ]
    ).is_err() {
        unsafe { free(rfc724_mid as *mut libc::c_void) };
        return;
    }

    let msg_id = sql::get_rowid(
        context,
        &context.sql,
        "msgs",
        "rfc724_mid",
        as_str(rfc724_mid),
    );
    unsafe { free(rfc724_mid as *mut libc::c_void) };
    context.call_cb(
        Event::MSGS_CHANGED,
        chat_id as uintptr_t,
        msg_id as uintptr_t,
    );
}

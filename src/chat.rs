use std::path::{Path, PathBuf};

use crate::chatlist::*;
use crate::config::*;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_mimeparser::SystemMessage;
use crate::dc_tools::*;
use crate::error::Error;
use crate::events::Event;
use crate::job::*;
use crate::message::{self, Message, MessageState};
use crate::param::*;
use crate::sql::{self, Sql};
use crate::stock::StockMessage;

/// An object representing a single chat in memory.
/// Chat objects are created using eg. `Chat::load_from_db`
/// and are not updated on database changes;
/// if you want an update, you have to recreate the object.
#[derive(Debug, Clone)]
pub struct Chat {
    pub id: u32,
    pub typ: Chattype,
    pub name: String,
    archived: bool,
    pub grpid: String,
    blocked: Blocked,
    pub param: Params,
    pub gossiped_timestamp: i64,
    is_sending_locations: bool,
}

impl Chat {
    pub fn load_from_db(context: &Context, chat_id: u32) -> Result<Self, Error> {
        let res = context.sql.query_row(
            "SELECT c.id,c.type,c.name, c.grpid,c.param,c.archived, \
             c.blocked, c.gossiped_timestamp, c.locations_send_until  \
             FROM chats c WHERE c.id=?;",
            params![chat_id as i32],
            |row| {
                let c = Chat {
                    id: row.get(0)?,
                    typ: row.get(1)?,
                    name: row.get::<_, String>(2)?,
                    grpid: row.get::<_, String>(3)?,
                    param: row.get::<_, String>(4)?.parse().unwrap_or_default(),
                    archived: row.get(5)?,
                    blocked: row.get::<_, Option<_>>(6)?.unwrap_or_default(),
                    gossiped_timestamp: row.get(7)?,
                    is_sending_locations: row.get(8)?,
                };

                Ok(c)
            },
        );

        match res {
            Err(err @ crate::error::Error::Sql(rusqlite::Error::QueryReturnedNoRows)) => Err(err),
            Err(err) => {
                error!(
                    context,
                    "chat: failed to load from db {}: {:?}", chat_id, err
                );
                Err(err)
            }
            Ok(mut chat) => {
                match chat.id {
                    DC_CHAT_ID_DEADDROP => {
                        chat.name = context.stock_str(StockMessage::DeadDrop).into();
                    }
                    DC_CHAT_ID_ARCHIVED_LINK => {
                        let tempname = context.stock_str(StockMessage::ArchivedChats);
                        let cnt = dc_get_archived_cnt(context);
                        chat.name = format!("{} ({})", tempname, cnt);
                    }
                    DC_CHAT_ID_STARRED => {
                        chat.name = context.stock_str(StockMessage::StarredMsgs).into();
                    }
                    _ => {
                        if chat.typ == Chattype::Single {
                            let contacts = get_chat_contacts(context, chat.id);
                            let mut chat_name = "Err [Name not found]".to_owned();

                            if !(*contacts).is_empty() {
                                if let Ok(contact) = Contact::get_by_id(context, contacts[0]) {
                                    chat_name = contact.get_display_name().to_owned();
                                }
                            }

                            chat.name = chat_name;
                        }

                        if chat.param.exists(Param::Selftalk) {
                            chat.name = context.stock_str(StockMessage::SelfMsg).into();
                        }
                    }
                }
                Ok(chat)
            }
        }
    }

    pub fn is_self_talk(&self) -> bool {
        self.param.exists(Param::Selftalk)
    }

    pub fn update_param(&mut self, context: &Context) -> Result<(), Error> {
        sql::execute(
            context,
            &context.sql,
            "UPDATE chats SET param=? WHERE id=?",
            params![self.param.to_string(), self.id as i32],
        )
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_type(&self) -> Chattype {
        self.typ
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_subtitle(&self, context: &Context) -> String {
        // returns either the address or the number of chat members

        if self.typ == Chattype::Single && self.param.exists(Param::Selftalk) {
            return context.stock_str(StockMessage::SelfTalkSubTitle).into();
        }

        if self.typ == Chattype::Single {
            return context
                .sql
                .query_get_value(
                    context,
                    "SELECT c.addr FROM chats_contacts cc  \
                     LEFT JOIN contacts c ON c.id=cc.contact_id  \
                     WHERE cc.chat_id=?;",
                    params![self.id as i32],
                )
                .unwrap_or_else(|| "Err".into());
        }

        if self.typ == Chattype::Group || self.typ == Chattype::VerifiedGroup {
            if self.id == 1 {
                return context.stock_str(StockMessage::DeadDrop).into();
            }
            let cnt = get_chat_contact_cnt(context, self.id);
            return context
                .stock_string_repl_int(StockMessage::Member, cnt as i32)
                .into();
        }

        return "Err".into();
    }

    pub fn get_parent_mime_headers(&self, context: &Context) -> Option<(String, String, String)> {
        let collect = |row: &rusqlite::Row| Ok((row.get(0)?, row.get(1)?, row.get(2)?));
        let params = params![self.id as i32, DC_CONTACT_ID_SELF as i32];
        let sql = &context.sql;
        let main_query = "SELECT rfc724_mid, mime_in_reply_to, mime_references \
                          FROM msgs WHERE chat_id=?1 AND timestamp=(SELECT max(timestamp) \
                          FROM msgs WHERE chat_id=?1 AND from_id!=?2);";
        let fallback_query = "SELECT rfc724_mid, mime_in_reply_to, mime_references \
                              FROM msgs WHERE chat_id=?1 AND timestamp=(SELECT min(timestamp) \
                              FROM msgs WHERE chat_id=?1 AND from_id==?2);";

        sql.query_row(main_query, params, collect)
            .or_else(|_| sql.query_row(fallback_query, params, collect))
            .ok()
    }

    pub fn get_profile_image(&self, context: &Context) -> Option<PathBuf> {
        if let Some(image_rel) = self.param.get(Param::ProfileImage) {
            if !image_rel.is_empty() {
                return Some(dc_get_abs_path(context, image_rel));
            }
        } else if self.typ == Chattype::Single {
            let contacts = get_chat_contacts(context, self.id);
            if !contacts.is_empty() {
                if let Ok(contact) = Contact::get_by_id(context, contacts[0]) {
                    return contact.get_profile_image(context);
                }
            }
        }

        None
    }

    pub fn get_color(&self, context: &Context) -> u32 {
        let mut color = 0;

        if self.typ == Chattype::Single {
            let contacts = get_chat_contacts(context, self.id);
            if !contacts.is_empty() {
                if let Ok(contact) = Contact::get_by_id(context, contacts[0]) {
                    color = contact.get_color();
                }
            }
        } else {
            color = dc_str_to_color(&self.name);
        }

        color
    }

    pub fn is_archived(&self) -> bool {
        self.archived
    }

    pub fn is_unpromoted(&self) -> bool {
        self.param.get_int(Param::Unpromoted).unwrap_or_default() == 1
    }

    pub fn is_promoted(&self) -> bool {
        !self.is_unpromoted()
    }

    pub fn is_verified(&self) -> bool {
        (self.typ == Chattype::VerifiedGroup)
    }

    pub fn is_sending_locations(&self) -> bool {
        self.is_sending_locations
    }

    fn prepare_msg_raw(
        &mut self,
        context: &Context,
        msg: &mut Message,
        timestamp: i64,
    ) -> Result<u32, Error> {
        let mut do_guarantee_e2ee: bool;
        let e2ee_enabled: bool;
        let mut new_references = "".into();
        let mut new_in_reply_to = "".into();
        let mut msg_id = 0;
        let mut to_id = 0;
        let mut location_id = 0;

        if !(self.typ == Chattype::Single
            || self.typ == Chattype::Group
            || self.typ == Chattype::VerifiedGroup)
        {
            error!(context, "Cannot send to chat type #{}.", self.typ,);
            return Ok(0);
        }

        if (self.typ == Chattype::Group || self.typ == Chattype::VerifiedGroup)
            && !is_contact_in_chat(context, self.id, 1 as u32)
        {
            emit_event!(
                context,
                Event::ErrorSelfNotInGroup("Cannot send message; self not in group.".into())
            );
            return Ok(0);
        }

        if let Some(from) = context.get_config(Config::ConfiguredAddr) {
            let new_rfc724_mid = {
                let grpid = match self.typ {
                    Chattype::Group | Chattype::VerifiedGroup => Some(self.grpid.as_str()),
                    _ => None,
                };
                dc_create_outgoing_rfc724_mid(grpid, &from)
            };

            if self.typ == Chattype::Single {
                if let Some(id) = context.sql.query_get_value(
                    context,
                    "SELECT contact_id FROM chats_contacts WHERE chat_id=?;",
                    params![self.id as i32],
                ) {
                    to_id = id;
                } else {
                    error!(
                        context,
                        "Cannot send message, contact for chat #{} not found.", self.id,
                    );
                    return Ok(0);
                }
            } else {
                if self.typ == Chattype::Group || self.typ == Chattype::VerifiedGroup {
                    if self.param.get_int(Param::Unpromoted).unwrap_or_default() == 1 {
                        self.param.remove(Param::Unpromoted);
                        self.update_param(context).unwrap();
                    }
                }
            }

            /* check if we can guarantee E2EE for this message.
            if we guarantee E2EE, and circumstances change
            so that E2EE is no longer available at a later point (reset, changed settings),
            we do not send the message out at all */
            do_guarantee_e2ee = false;
            e2ee_enabled = context.get_config_bool(Config::E2eeEnabled);
            if e2ee_enabled && msg.param.get_int(Param::ForcePlaintext).unwrap_or_default() == 0 {
                let mut can_encrypt = 1;
                let mut all_mutual = 1;

                let res = context.sql.query_row(
                    "SELECT ps.prefer_encrypted, c.addr \
                     FROM chats_contacts cc  \
                     LEFT JOIN contacts c ON cc.contact_id=c.id  \
                     LEFT JOIN acpeerstates ps ON c.addr=ps.addr  \
                     WHERE cc.chat_id=?  AND cc.contact_id>9;",
                    params![self.id],
                    |row| {
                        let state: String = row.get(1)?;

                        if let Some(prefer_encrypted) = row.get::<_, Option<i32>>(0)? {
                            if prefer_encrypted != 1 {
                                info!(
                                    context,
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
                            info!(context, "[autocrypt] no peerstate for {}", state,);
                            can_encrypt = 0;
                            all_mutual = 0;
                        }
                        Ok(())
                    },
                );
                match res {
                    Ok(_) => {}
                    Err(err) => {
                        warn!(context, "chat: failed to load peerstates: {:?}", err);
                    }
                }

                if 0 != can_encrypt {
                    if 0 != all_mutual {
                        do_guarantee_e2ee = true;
                    } else if last_msg_in_chat_encrypted(context, &context.sql, self.id) {
                        do_guarantee_e2ee = true;
                    }
                }
            }
            if do_guarantee_e2ee {
                msg.param.set_int(Param::GuranteeE2ee, 1);
            }
            msg.param.remove(Param::ErroneousE2ee);
            if !self.is_self_talk() {
                if let Some((parent_rfc724_mid, parent_in_reply_to, parent_references)) =
                    self.get_parent_mime_headers(context)
                {
                    if !parent_rfc724_mid.is_empty() {
                        new_in_reply_to = parent_rfc724_mid.clone();
                    }
                    let parent_references = if let Some(n) = parent_references.find(' ') {
                        &parent_references[0..n]
                    } else {
                        &parent_references
                    };

                    if !parent_references.is_empty() && !parent_rfc724_mid.is_empty() {
                        new_references = format!("{} {}", parent_references, parent_rfc724_mid);
                    } else if !parent_references.is_empty() {
                        new_references = parent_references.to_string();
                    } else if !parent_in_reply_to.is_empty() && !parent_rfc724_mid.is_empty() {
                        new_references = format!("{} {}", parent_in_reply_to, parent_rfc724_mid);
                    } else if !parent_in_reply_to.is_empty() {
                        new_references = parent_in_reply_to.clone();
                    }
                }
            }

            // add independent location to database

            if msg.param.exists(Param::SetLatitude) {
                if sql::execute(
                    context,
                    &context.sql,
                    "INSERT INTO locations \
                     (timestamp,from_id,chat_id, latitude,longitude,independent)\
                     VALUES (?,?,?, ?,?,1);",
                    params![
                        timestamp,
                        DC_CONTACT_ID_SELF,
                        self.id as i32,
                        msg.param.get_float(Param::SetLatitude).unwrap_or_default(),
                        msg.param.get_float(Param::SetLongitude).unwrap_or_default(),
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
                            new_rfc724_mid,
                            self.id as i32,
                            1i32,
                            to_id as i32,
                            timestamp,
                            msg.type_0,
                            msg.state,
                            msg.text.as_ref().map_or("", String::as_str),
                            msg.param.to_string(),
                            msg.hidden,
                            new_in_reply_to,
                            new_references,
                            location_id as i32,
                        ]
                    ).is_ok() {
                        msg_id = sql::get_rowid(
                            context,
                            &context.sql,
                            "msgs",
                            "rfc724_mid",
                            new_rfc724_mid,
                        );
                    } else {
                        error!(
                            context,
                            "Cannot send message, cannot insert to database (chat #{}).",
                            self.id,
                        );
                    }
        } else {
            error!(context, "Cannot send message, not configured.",);
        }

        Ok(msg_id)
    }
}

/// Create a normal chat or a group chat by a messages ID that comes typically
/// from the deaddrop, DC_CHAT_ID_DEADDROP (1).
///
/// If the given message ID already belongs to a normal chat or to a group chat,
/// the chat ID of this chat is returned and no new chat is created.
/// If a new chat is created, the given message ID is moved to this chat, however,
/// there may be more messages moved to the chat from the deaddrop. To get the
/// chat messages, use dc_get_chat_msgs().
///
/// If the user is asked before creation, he should be
/// asked whether he wants to chat with the _contact_ belonging to the message;
/// the group names may be really weird when taken from the subject of implicit
/// groups and this may look confusing.
///
/// Moreover, this function also scales up the origin of the contact belonging
/// to the message and, depending on the contacts origin, messages from the
/// same group may be shown or not - so, all in all, it is fine to show the
/// contact name only.
pub fn create_by_msg_id(context: &Context, msg_id: u32) -> Result<u32, Error> {
    let mut chat_id = 0;
    let mut send_event = false;

    if let Ok(msg) = Message::load_from_db(context, msg_id) {
        if let Ok(chat) = Chat::load_from_db(context, msg.chat_id) {
            if chat.id > DC_CHAT_ID_LAST_SPECIAL {
                chat_id = chat.id;
                if chat.blocked != Blocked::Not {
                    unblock(context, chat.id);
                    send_event = true;
                }
                Contact::scaleup_origin_by_id(context, msg.from_id, Origin::CreateChat);
            }
        }
    }

    if send_event {
        context.call_cb(Event::MsgsChanged {
            chat_id: 0,
            msg_id: 0,
        });
    }

    ensure!(chat_id > 0, "failed to load create chat");

    Ok(chat_id)
}

/// Create a normal chat with a single user.  To create group chats,
/// see dc_create_group_chat().
///
/// If a chat already exists, this ID is returned, otherwise a new chat is created;
/// this new chat may already contain messages, eg. from the deaddrop, to get the
/// chat messages, use dc_get_chat_msgs().
pub fn create_by_contact_id(context: &Context, contact_id: u32) -> Result<u32, Error> {
    let chat_id = match lookup_by_contact_id(context, contact_id) {
        Ok((chat_id, chat_blocked)) => {
            if chat_blocked != Blocked::Not {
                // unblock chat (typically move it from the deaddrop to view
                unblock(context, chat_id);
            }
            chat_id
        }
        Err(err) => {
            if !Contact::real_exists_by_id(context, contact_id) && contact_id != DC_CONTACT_ID_SELF
            {
                warn!(
                    context,
                    "Cannot create chat, contact {} does not exist.", contact_id,
                );
                return Err(err);
            } else {
                let (chat_id, _) =
                    create_or_lookup_by_contact_id(context, contact_id, Blocked::Not)?;
                Contact::scaleup_origin_by_id(context, contact_id, Origin::CreateChat);
                chat_id
            }
        }
    };

    context.call_cb(Event::MsgsChanged {
        chat_id: 0,
        msg_id: 0,
    });

    Ok(chat_id)
}

pub fn unblock(context: &Context, chat_id: u32) {
    set_blocking(context, chat_id, Blocked::Not);
}

pub fn set_blocking(context: &Context, chat_id: u32, new_blocking: Blocked) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET blocked=? WHERE id=?;",
        params![new_blocking, chat_id as i32],
    )
    .is_ok()
}

pub fn create_or_lookup_by_contact_id(
    context: &Context,
    contact_id: u32,
    create_blocked: Blocked,
) -> Result<(u32, Blocked), Error> {
    ensure!(context.sql.is_open(), "Database not available");
    ensure!(contact_id > 0, "Invalid contact id requested");

    if let Ok((chat_id, chat_blocked)) = lookup_by_contact_id(context, contact_id) {
        // Already exists, no need to create.
        return Ok((chat_id, chat_blocked));
    }

    let contact = Contact::load_from_db(context, contact_id)?;
    let chat_name = contact.get_display_name();

    sql::execute(
        context,
        &context.sql,
        format!(
            "INSERT INTO chats (type, name, param, blocked, grpid) VALUES({}, '{}', '{}', {}, '{}')",
            100,
            chat_name,
            if contact_id == DC_CONTACT_ID_SELF as u32 { "K=1" } else { "" },
            create_blocked as u8,
            contact.get_addr(),
        ),
        params![],
    )?;

    let chat_id = sql::get_rowid(context, &context.sql, "chats", "grpid", contact.get_addr());

    sql::execute(
        context,
        &context.sql,
        format!(
            "INSERT INTO chats_contacts (chat_id, contact_id) VALUES({}, {})",
            chat_id, contact_id
        ),
        params![],
    )?;

    Ok((chat_id, create_blocked))
}

pub fn lookup_by_contact_id(context: &Context, contact_id: u32) -> Result<(u32, Blocked), Error> {
    ensure!(context.sql.is_open(), "Database not available");

    context.sql.query_row(
        "SELECT c.id, c.blocked FROM chats c INNER JOIN chats_contacts j ON c.id=j.chat_id WHERE c.type=100 AND c.id>9 AND j.contact_id=?;",
        params![contact_id as i32],
        |row| Ok((row.get(0)?, row.get::<_, Option<_>>(1)?.unwrap_or_default())),
    )
}

pub fn get_by_contact_id(context: &Context, contact_id: u32) -> Result<u32, Error> {
    let (chat_id, blocked) = lookup_by_contact_id(context, contact_id)?;
    ensure_eq!(blocked, Blocked::Not, "Requested contact is blocked");

    Ok(chat_id)
}

pub fn prepare_msg<'a>(
    context: &'a Context,
    chat_id: u32,
    msg: &mut Message,
) -> Result<u32, Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "Cannot prepare message for special chat"
    );

    msg.state = MessageState::OutPreparing;
    let msg_id = prepare_msg_common(context, chat_id, msg)?;
    context.call_cb(Event::MsgsChanged {
        chat_id: msg.chat_id,
        msg_id: msg.id,
    });

    Ok(msg_id)
}

pub fn msgtype_has_file(msgtype: Viewtype) -> bool {
    match msgtype {
        Viewtype::Image => true,
        Viewtype::Gif => true,
        Viewtype::Sticker => true,
        Viewtype::Audio => true,
        Viewtype::Voice => true,
        Viewtype::Video => true,
        Viewtype::File => true,
        _ => false,
    }
}

fn prepare_msg_common(context: &Context, chat_id: u32, msg: &mut Message) -> Result<u32, Error> {
    msg.id = 0;

    if msg.type_0 == Viewtype::Text {
        // the caller should check if the message text is empty
    } else if msgtype_has_file(msg.type_0) {
        let path_filename = msg.param.get(Param::File);

        ensure!(
            path_filename.is_some(),
            "Attachment missing for message of type #{}.",
            msg.type_0
        );

        let mut path_filename = path_filename.unwrap().to_string();

        if msg.state == MessageState::OutPreparing && !dc_is_blobdir_path(context, &path_filename) {
            bail!("Files must be created in the blob-directory.");
        }

        ensure!(
            dc_make_rel_and_copy(context, &mut path_filename),
            "Failed to copy"
        );

        msg.param.set(Param::File, &path_filename);
        if msg.type_0 == Viewtype::File || msg.type_0 == Viewtype::Image {
            // Correct the type, take care not to correct already very special
            // formats as GIF or VOICE.
            //
            // Typical conversions:
            // - from FILE to AUDIO/VIDEO/IMAGE
            // - from FILE/IMAGE to GIF */
            if let Some((better_type, better_mime)) =
                message::guess_msgtype_from_suffix(Path::new(&path_filename))
            {
                msg.type_0 = better_type;
                msg.param.set(Param::MimeType, better_mime);
            }
        } else if !msg.param.exists(Param::MimeType) {
            if let Some((_, mime)) = message::guess_msgtype_from_suffix(Path::new(&path_filename)) {
                msg.param.set(Param::MimeType, mime);
            }
        }
        info!(
            context,
            "Attaching \"{}\" for message type #{}.", &path_filename, msg.type_0
        );
    } else {
        bail!("Cannot send messages of type #{}.", msg.type_0);
    }

    unarchive(context, chat_id)?;

    let mut chat = Chat::load_from_db(context, chat_id)?;
    if msg.state != MessageState::OutPreparing {
        msg.state = MessageState::OutPending;
    }

    msg.id = chat.prepare_msg_raw(context, msg, dc_create_smeared_timestamp(context))?;
    msg.chat_id = chat_id;

    Ok(msg.id)
}

fn last_msg_in_chat_encrypted(context: &Context, sql: &Sql, chat_id: u32) -> bool {
    let packed: Option<String> = sql.query_get_value(
        context,
        "SELECT param  \
         FROM msgs  WHERE timestamp=(SELECT MAX(timestamp) FROM msgs WHERE chat_id=?)  \
         ORDER BY id DESC;",
        params![chat_id as i32],
    );

    if let Some(ref packed) = packed {
        match packed.parse::<Params>() {
            Ok(param) => param.exists(Param::GuranteeE2ee),
            Err(err) => {
                error!(context, "invalid params stored: '{}', {:?}", packed, err);
                false
            }
        }
    } else {
        false
    }
}

pub fn is_contact_in_chat(context: &Context, chat_id: u32, contact_id: u32) -> bool {
    /* this function works for group and for normal chats, however, it is more useful for group chats.
    DC_CONTACT_ID_SELF may be used to check, if the user itself is in a group chat (DC_CONTACT_ID_SELF is not added to normal chats) */

    context
        .sql
        .exists(
            "SELECT contact_id FROM chats_contacts WHERE chat_id=? AND contact_id=?;",
            params![chat_id as i32, contact_id as i32],
        )
        .unwrap_or_default()
}

// Should return Result
pub fn unarchive(context: &Context, chat_id: u32) -> Result<(), Error> {
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET archived=0 WHERE id=?",
        params![chat_id as i32],
    )
}

/// Send a message defined by a dc_msg_t object to a chat.
///
/// Sends the event #DC_EVENT_MSGS_CHANGED on succcess.
/// However, this does not imply, the message really reached the recipient -
/// sending may be delayed eg. due to network problems. However, from your
/// view, you're done with the message. Sooner or later it will find its way.
pub fn send_msg(context: &Context, chat_id: u32, msg: &mut Message) -> Result<u32, Error> {
    if msg.state != MessageState::OutPreparing {
        // automatically prepare normal messages
        prepare_msg_common(context, chat_id, msg)?;
    } else {
        // update message state of separately prepared messages
        ensure!(
            chat_id == 0 || chat_id == msg.chat_id,
            "Inconsistent chat ID"
        );
        message::update_msg_state(context, msg.id, MessageState::OutPending);
    }

    job_send_msg(context, msg.id)?;

    context.call_cb(Event::MsgsChanged {
        chat_id: msg.chat_id,
        msg_id: msg.id,
    });

    if msg.param.exists(Param::SetLatitude) {
        context.call_cb(Event::LocationChanged(Some(DC_CONTACT_ID_SELF)));
    }

    if 0 == chat_id {
        let forwards = msg.param.get(Param::PrepForwards);
        if let Some(forwards) = forwards {
            for forward in forwards.split(' ') {
                let id: i32 = forward.parse().unwrap_or_default();
                if 0 == id {
                    // avoid hanging if user tampers with db
                    break;
                } else {
                    if let Ok(mut copy) = Message::load_from_db(context, id as u32) {
                        send_msg(context, 0, &mut copy)?;
                    }
                }
            }
            msg.param.remove(Param::PrepForwards);
            msg.save_param_to_disk(context);
        }
    }

    Ok(msg.id)
}

pub fn send_text_msg(context: &Context, chat_id: u32, text_to_send: String) -> Result<u32, Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= 9",
        chat_id
    );

    let mut msg = Message::new(Viewtype::Text);
    msg.text = Some(text_to_send);
    send_msg(context, chat_id, &mut msg)
}

// passing `None` as message jsut deletes the draft
pub fn set_draft(context: &Context, chat_id: u32, msg: Option<&mut Message>) {
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL {
        return;
    }

    let changed = match msg {
        None => maybe_delete_draft(context, chat_id),
        Some(msg) => set_draft_raw(context, chat_id, msg),
    };

    if changed {
        context.call_cb(Event::MsgsChanged { chat_id, msg_id: 0 });
    }
}

/// Delete draft message in specified chat, if there is one.
///
/// Return {true}, if message was deleted, {false} otherwise.
fn maybe_delete_draft(context: &Context, chat_id: u32) -> bool {
    let draft = get_draft_msg_id(context, chat_id);
    if draft != 0 {
        Message::delete_from_db(context, draft);
        return true;
    }
    false
}

/// Set provided message as draft message for specified chat.
///
/// Return true on success, false on database error.
fn do_set_draft(context: &Context, chat_id: u32, msg: &mut Message) -> bool {
    match msg.type_0 {
        Viewtype::Unknown => return false,
        Viewtype::Text => {
            if msg.text.as_ref().map_or(false, |s| s.is_empty()) {
                return false;
            }
        }
        _ => {
            if let Some(path_filename) = msg.param.get(Param::File) {
                let mut path_filename = path_filename.to_string();
                if msg.is_increation() && !dc_is_blobdir_path(context, &path_filename) {
                    return false;
                }
                if !dc_make_rel_and_copy(context, &mut path_filename) {
                    return false;
                }
                msg.param.set(Param::File, path_filename);
            }
        }
    }

    sql::execute(
        context,
        &context.sql,
        "INSERT INTO msgs (chat_id, from_id, timestamp, type, state, txt, param, hidden) \
         VALUES (?,?,?, ?,?,?,?,?);",
        params![
            chat_id as i32,
            1,
            time(),
            msg.type_0,
            MessageState::OutDraft,
            msg.text.as_ref().map(String::as_str).unwrap_or(""),
            msg.param.to_string(),
            1,
        ],
    )
    .is_ok()
}

// similar to as dc_set_draft() but does not emit an event
fn set_draft_raw(context: &Context, chat_id: u32, msg: &mut Message) -> bool {
    let deleted = maybe_delete_draft(context, chat_id);
    let set = do_set_draft(context, chat_id, msg);

    // Can't inline. Both functions above must be called, no shortcut!
    deleted || set
}

fn get_draft_msg_id(context: &Context, chat_id: u32) -> u32 {
    context
        .sql
        .query_get_value::<_, i32>(
            context,
            "SELECT id FROM msgs WHERE chat_id=? AND state=?;",
            params![chat_id as i32, MessageState::OutDraft],
        )
        .unwrap_or_default() as u32
}

pub fn get_draft(context: &Context, chat_id: u32) -> Result<Option<Message>, Error> {
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL {
        return Ok(None);
    }
    let draft_msg_id = get_draft_msg_id(context, chat_id);
    if draft_msg_id == 0 {
        return Ok(None);
    }
    Ok(Some(Message::load_from_db(context, draft_msg_id)?))
}

pub fn get_chat_msgs(context: &Context, chat_id: u32, flags: u32, marker1before: u32) -> Vec<u32> {
    let mut ret = Vec::new();

    let mut last_day = 0;
    let cnv_to_local = dc_gm2local_offset();

    let process_row = |row: &rusqlite::Row| Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?));
    let process_rows = |rows: rusqlite::MappedRows<_>| {
        for row in rows {
            let (curr_id, ts) = row?;
            if curr_id as u32 == marker1before {
                ret.push(DC_MSG_ID_MARKER1);
            }
            if 0 != flags & 0x1 {
                let curr_local_timestamp = ts + cnv_to_local;
                let curr_day = curr_local_timestamp / 86400;
                if curr_day != last_day {
                    ret.push(DC_MSG_ID_LAST_SPECIAL);
                    last_day = curr_day;
                }
            }
            ret.push(curr_id as u32);
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
        ret
    } else {
        Vec::new()
    }
}

pub fn get_msg_cnt(context: &Context, chat_id: u32) -> usize {
    context
        .sql
        .query_get_value::<_, i32>(
            context,
            "SELECT COUNT(*) FROM msgs WHERE chat_id=?;",
            params![chat_id as i32],
        )
        .unwrap_or_default() as usize
}

pub fn get_fresh_msg_cnt(context: &Context, chat_id: u32) -> usize {
    context
        .sql
        .query_get_value::<_, i32>(
            context,
            "SELECT COUNT(*) FROM msgs  \
             WHERE state=10   \
             AND hidden=0    \
             AND chat_id=?;",
            params![chat_id as i32],
        )
        .unwrap_or_default() as usize
}

pub fn marknoticed_chat(context: &Context, chat_id: u32) -> Result<(), Error> {
    if !context.sql.exists(
        "SELECT id FROM msgs  WHERE chat_id=? AND state=?;",
        params![chat_id as i32, MessageState::InFresh],
    )? {
        return Ok(());
    }

    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs    \
         SET state=13 WHERE chat_id=? AND state=10;",
        params![chat_id as i32],
    )?;

    context.call_cb(Event::MsgsChanged {
        chat_id: 0,
        msg_id: 0,
    });

    Ok(())
}

pub fn marknoticed_all_chats(context: &Context) -> Result<(), Error> {
    if !context.sql.exists(
        "SELECT id FROM msgs  \
         WHERE state=10;",
        params![],
    )? {
        return Ok(());
    }

    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs    \
         SET state=13 WHERE state=10;",
        params![],
    )?;

    context.call_cb(Event::MsgsChanged {
        msg_id: 0,
        chat_id: 0,
    });

    Ok(())
}

pub fn get_chat_media(
    context: &Context,
    chat_id: u32,
    msg_type: Viewtype,
    msg_type2: Viewtype,
    msg_type3: Viewtype,
) -> Vec<u32> {
    context.sql.query_map(
        "SELECT id FROM msgs WHERE chat_id=? AND (type=? OR type=? OR type=?) ORDER BY timestamp, id;",
        params![
            chat_id as i32,
            msg_type,
            if msg_type2 != Viewtype::Unknown {
                msg_type2
            } else {
                msg_type
            }, if msg_type3 != Viewtype::Unknown {
                msg_type3
            } else {
                msg_type
            },
        ],
        |row| row.get::<_, i32>(0),
        |ids| {
            let mut ret = Vec::new();
            for id in ids {
                ret.push(id? as u32);
            }
            Ok(ret)
        }
    ).unwrap_or_default()
}

/// Indicates the direction over which to iterate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum Direction {
    Forward = 1,
    Backward = -1,
}

pub fn get_next_media(
    context: &Context,
    curr_msg_id: u32,
    direction: Direction,
    msg_type: Viewtype,
    msg_type2: Viewtype,
    msg_type3: Viewtype,
) -> u32 {
    let mut ret = 0;

    if let Ok(msg) = Message::load_from_db(context, curr_msg_id) {
        let list = get_chat_media(
            context,
            msg.chat_id,
            if msg_type != Viewtype::Unknown {
                msg_type
            } else {
                msg.type_0
            },
            msg_type2,
            msg_type3,
        );
        for i in 0..list.len() {
            if curr_msg_id == list[i] {
                match direction {
                    Direction::Forward => {
                        if i + 1 < list.len() {
                            ret = list[i + 1]
                        }
                    }
                    Direction::Backward => {
                        if i >= 1 {
                            ret = list[i - 1];
                        }
                    }
                }
                break;
            }
        }
    }
    ret
}

pub fn archive(context: &Context, chat_id: u32, archive: bool) -> Result<(), Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= 9",
        chat_id
    );

    if archive {
        sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET state=? WHERE chat_id=? AND state=?;",
            params![
                MessageState::InNoticed,
                chat_id as i32,
                MessageState::InFresh
            ],
        )?;
    }

    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET archived=? WHERE id=?;",
        params![archive, chat_id as i32],
    )?;

    context.call_cb(Event::MsgsChanged {
        msg_id: 0,
        chat_id: 0,
    });

    Ok(())
}

pub fn delete(context: &Context, chat_id: u32) -> Result<(), Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= 9",
        chat_id
    );
    /* Up to 2017-11-02 deleting a group also implied leaving it, see above why we have changed this. */

    let _chat = Chat::load_from_db(context, chat_id)?;
    sql::execute(
        context,
        &context.sql,
        "DELETE FROM msgs_mdns WHERE msg_id IN (SELECT id FROM msgs WHERE chat_id=?);",
        params![chat_id as i32],
    )?;

    sql::execute(
        context,
        &context.sql,
        "DELETE FROM msgs WHERE chat_id=?;",
        params![chat_id as i32],
    )?;

    sql::execute(
        context,
        &context.sql,
        "DELETE FROM chats_contacts WHERE chat_id=?;",
        params![chat_id as i32],
    )?;

    sql::execute(
        context,
        &context.sql,
        "DELETE FROM chats WHERE id=?;",
        params![chat_id as i32],
    )?;

    context.call_cb(Event::MsgsChanged {
        msg_id: 0,
        chat_id: 0,
    });

    job_kill_action(context, Action::Housekeeping);
    job_add(context, Action::Housekeeping, 0, Params::new(), 10);

    Ok(())
}

pub fn get_chat_contacts(context: &Context, chat_id: u32) -> Vec<u32> {
    /* Normal chats do not include SELF.  Group chats do (as it may happen that one is deleted from a
    groupchat but the chats stays visible, moreover, this makes displaying lists easier) */

    if chat_id == 1 {
        return Vec::new();
    }

    // we could also create a list for all contacts in the deaddrop by searching contacts belonging to chats with
    // chats.blocked=2, however, currently this is not needed

    context
        .sql
        .query_map(
            "SELECT cc.contact_id FROM chats_contacts cc \
             LEFT JOIN contacts c ON c.id=cc.contact_id WHERE cc.chat_id=? \
             ORDER BY c.id=1, LOWER(c.name||c.addr), c.id;",
            params![chat_id],
            |row| row.get::<_, u32>(0),
            |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into),
        )
        .unwrap_or_default()
}

pub fn create_group_chat(
    context: &Context,
    verified: VerifiedStatus,
    chat_name: impl AsRef<str>,
) -> Result<u32, Error> {
    ensure!(!chat_name.as_ref().is_empty(), "Invalid chat name");

    let draft_txt = context.stock_string_repl_str(StockMessage::NewGroupDraft, &chat_name);
    let grpid = dc_create_id();

    sql::execute(
        context,
        &context.sql,
        "INSERT INTO chats (type, name, grpid, param) VALUES(?, ?, ?, \'U=1\');",
        params![
            if verified != VerifiedStatus::Unverified {
                Chattype::VerifiedGroup
            } else {
                Chattype::Group
            },
            chat_name.as_ref(),
            grpid
        ],
    )?;

    let chat_id = sql::get_rowid(context, &context.sql, "chats", "grpid", grpid);

    if chat_id != 0 {
        if add_to_chat_contacts_table(context, chat_id, 1) {
            let mut draft_msg = Message::new(Viewtype::Text);
            draft_msg.set_text(Some(draft_txt));
            set_draft_raw(context, chat_id, &mut draft_msg);
        }

        context.call_cb(Event::MsgsChanged {
            msg_id: 0,
            chat_id: 0,
        });
    }

    Ok(chat_id)
}

/* you MUST NOT modify this or the following strings */
// Context functions to work with chats
pub fn add_to_chat_contacts_table(context: &Context, chat_id: u32, contact_id: u32) -> bool {
    // add a contact to a chat; the function does not check the type or if any of the record exist or are already
    // added to the chat!
    sql::execute(
        context,
        &context.sql,
        "INSERT INTO chats_contacts (chat_id, contact_id) VALUES(?, ?)",
        params![chat_id as i32, contact_id as i32],
    )
    .is_ok()
}

pub fn add_contact_to_chat(context: &Context, chat_id: u32, contact_id: u32) -> bool {
    match add_contact_to_chat_ex(context, chat_id, contact_id, false) {
        Ok(res) => res,
        Err(err) => {
            error!(context, "failed to add contact: {}", err);
            false
        }
    }
}

#[allow(non_snake_case)]
pub(crate) fn add_contact_to_chat_ex(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
    from_handshake: bool,
) -> Result<bool, Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "can not add member to special chats"
    );
    let contact = Contact::get_by_id(context, contact_id)?;
    let mut msg = Message::default();

    reset_gossiped_timestamp(context, chat_id);

    /*this also makes sure, not contacts are added to special or normal chats*/
    let mut chat = Chat::load_from_db(context, chat_id)?;
    ensure!(
        real_group_exists(context, chat_id),
        "chat_id {} is not a group where one can add members",
        chat_id
    );
    ensure!(
        Contact::real_exists_by_id(context, contact_id) && contact_id != DC_CONTACT_ID_SELF,
        "invalid contact_id {} for removal in group",
        contact_id
    );

    if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF as u32) {
        /* we should respect this - whatever we send to the group, it gets discarded anyway! */
        emit_event!(
            context,
            Event::ErrorSelfNotInGroup("Cannot add contact to group; self not in group.".into())
        );
        bail!("can not add contact because our account is not part of it");
    }
    if from_handshake && chat.param.get_int(Param::Unpromoted).unwrap_or_default() == 1 {
        chat.param.remove(Param::Unpromoted);
        chat.update_param(context).unwrap();
    }
    let self_addr = context
        .sql
        .get_config(context, "configured_addr")
        .unwrap_or_default();
    if contact.get_addr() == &self_addr {
        bail!("invalid attempt to add self e-mail address to group");
    }
    // ourself is added using DC_CONTACT_ID_SELF, do not add it explicitly.
    // if SELF is not in the group, members cannot be added at all.

    if is_contact_in_chat(context, chat_id, contact_id) {
        if !from_handshake {
            return Ok(true);
        }
    } else {
        // else continue and send status mail
        if chat.typ == Chattype::VerifiedGroup {
            if contact.is_verified(context) != VerifiedStatus::BidirectVerified {
                error!(
                    context,
                    "Only bidirectional verified contacts can be added to verified groups."
                );
                return Ok(false);
            }
        }
        if !add_to_chat_contacts_table(context, chat_id, contact_id) {
            return Ok(false);
        }
    }
    if chat.param.get_int(Param::Unpromoted).unwrap_or_default() == 0 {
        msg.type_0 = Viewtype::Text;
        msg.text = Some(context.stock_system_msg(
            StockMessage::MsgAddMember,
            contact.get_addr(),
            "",
            DC_CONTACT_ID_SELF as u32,
        ));
        msg.param.set_int(Param::Cmd, 4);
        msg.param.set(Param::Arg, contact.get_addr());
        msg.param.set_int(Param::Arg2, from_handshake.into());
        msg.id = send_msg(context, chat_id, &mut msg)?;
        context.call_cb(Event::MsgsChanged {
            chat_id,
            msg_id: msg.id,
        });
    }
    context.call_cb(Event::MsgsChanged { chat_id, msg_id: 0 });
    return Ok(true);
}

fn real_group_exists(context: &Context, chat_id: u32) -> bool {
    // check if a group or a verified group exists under the given ID
    if !context.sql.is_open() || chat_id <= DC_CHAT_ID_LAST_SPECIAL {
        return false;
    }

    context
        .sql
        .exists(
            "SELECT id FROM chats  WHERE id=?    AND (type=120 OR type=130);",
            params![chat_id as i32],
        )
        .unwrap_or_default()
}

pub fn reset_gossiped_timestamp(context: &Context, chat_id: u32) {
    set_gossiped_timestamp(context, chat_id, 0);
}

// Should return Result
pub fn set_gossiped_timestamp(context: &Context, chat_id: u32, timestamp: i64) {
    if 0 != chat_id {
        info!(
            context,
            "set gossiped_timestamp for chat #{} to {}.", chat_id, timestamp,
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
            "set gossiped_timestamp for all chats to {}.", timestamp,
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

pub fn remove_contact_from_chat(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
) -> Result<(), Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= 9",
        chat_id
    );
    ensure!(
        contact_id > DC_CONTACT_ID_LAST_SPECIAL || contact_id == DC_CONTACT_ID_SELF,
        "Cannot remove special contact"
    );

    let mut msg = Message::default();
    let mut success = false;

    /* we do not check if "contact_id" exists but just delete all records with the id from chats_contacts */
    /* this allows to delete pending references to deleted contacts.  Of course, this should _not_ happen. */
    if let Ok(chat) = Chat::load_from_db(context, chat_id) {
        if real_group_exists(context, chat_id) {
            if !is_contact_in_chat(context, chat_id, 1 as u32) {
                emit_event!(
                    context,
                    Event::ErrorSelfNotInGroup(
                        "Cannot remove contact from chat; self not in group.".into()
                    )
                );
            } else {
                /* we should respect this - whatever we send to the group, it gets discarded anyway! */
                if let Ok(contact) = Contact::get_by_id(context, contact_id) {
                    if chat.is_promoted() {
                        msg.type_0 = Viewtype::Text;
                        if contact.id == DC_CONTACT_ID_SELF {
                            set_group_explicitly_left(context, chat.grpid).unwrap();
                            msg.text = Some(context.stock_system_msg(
                                StockMessage::MsgGroupLeft,
                                "",
                                "",
                                DC_CONTACT_ID_SELF,
                            ));
                        } else {
                            msg.text = Some(context.stock_system_msg(
                                StockMessage::MsgDelMember,
                                contact.get_addr(),
                                "",
                                DC_CONTACT_ID_SELF,
                            ));
                        }
                        msg.param.set_int(Param::Cmd, 5);
                        msg.param.set(Param::Arg, contact.get_addr());
                        msg.id = send_msg(context, chat_id, &mut msg)?;
                        context.call_cb(Event::MsgsChanged {
                            chat_id,
                            msg_id: msg.id,
                        });
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
                    context.call_cb(Event::ChatModified(chat_id));
                    success = true;
                }
            }
        }
    }

    if !success {
        bail!("Failed to remove contact");
    }

    Ok(())
}

fn set_group_explicitly_left(context: &Context, grpid: impl AsRef<str>) -> Result<(), Error> {
    if !is_group_explicitly_left(context, grpid.as_ref())? {
        sql::execute(
            context,
            &context.sql,
            "INSERT INTO leftgrps (grpid) VALUES(?);",
            params![grpid.as_ref()],
        )?;
    }

    Ok(())
}

pub fn is_group_explicitly_left(context: &Context, grpid: impl AsRef<str>) -> Result<bool, Error> {
    context.sql.exists(
        "SELECT id FROM leftgrps WHERE grpid=?;",
        params![grpid.as_ref()],
    )
}

pub fn set_chat_name(
    context: &Context,
    chat_id: u32,
    new_name: impl AsRef<str>,
) -> Result<(), Error> {
    /* the function only sets the names of group chats; normal chats get their names from the contacts */
    let mut success = false;

    ensure!(!new_name.as_ref().is_empty(), "Invalid name");
    ensure!(chat_id > DC_CHAT_ID_LAST_SPECIAL, "Invalid chat ID");

    let chat = Chat::load_from_db(context, chat_id)?;
    let mut msg = Message::default();

    if real_group_exists(context, chat_id) {
        if &chat.name == new_name.as_ref() {
            success = true;
        } else if !is_contact_in_chat(context, chat_id, 1) {
            emit_event!(
                context,
                Event::ErrorSelfNotInGroup("Cannot set chat name; self not in group".into())
            );
        } else {
            /* we should respect this - whatever we send to the group, it gets discarded anyway! */
            if sql::execute(
                context,
                &context.sql,
                format!(
                    "UPDATE chats SET name='{}' WHERE id={};",
                    new_name.as_ref(),
                    chat_id as i32
                ),
                params![],
            )
            .is_ok()
            {
                if chat.is_promoted() {
                    msg.type_0 = Viewtype::Text;
                    msg.text = Some(context.stock_system_msg(
                        StockMessage::MsgGrpName,
                        &chat.name,
                        new_name.as_ref(),
                        DC_CONTACT_ID_SELF,
                    ));
                    msg.param.set_int(Param::Cmd, 2);
                    if !chat.name.is_empty() {
                        msg.param.set(Param::Arg, &chat.name);
                    }
                    msg.id = send_msg(context, chat_id, &mut msg)?;
                    context.call_cb(Event::MsgsChanged {
                        chat_id,
                        msg_id: msg.id,
                    });
                }
                context.call_cb(Event::ChatModified(chat_id));
                success = true;
            }
        }
    }

    if !success {
        bail!("Failed to set name");
    }

    Ok(())
}

#[allow(non_snake_case)]
pub fn set_chat_profile_image(
    context: &Context,
    chat_id: u32,
    new_image: impl AsRef<str>, // XXX use PathBuf
) -> Result<(), Error> {
    ensure!(chat_id > DC_CHAT_ID_LAST_SPECIAL, "Invalid chat ID");

    let mut chat = Chat::load_from_db(context, chat_id)?;

    if real_group_exists(context, chat_id) {
        /* we should respect this - whatever we send to the group, it gets discarded anyway! */
        if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF) {
            emit_event!(
                context,
                Event::ErrorSelfNotInGroup(
                    "Cannot set chat profile image; self not in group.".into()
                )
            );
            bail!("Failed to set profile image");
        }
        let mut new_image_rel: String;
        if !new_image.as_ref().is_empty() {
            new_image_rel = new_image.as_ref().to_string();
            if !dc_make_rel_and_copy(context, &mut new_image_rel) {
                bail!("Failed to get relative path for profile image");
            }
        } else {
            new_image_rel = "".to_string();
        }

        chat.param.set(Param::ProfileImage, &new_image_rel);
        if chat.update_param(context).is_ok() {
            if chat.is_promoted() {
                let mut msg = Message::default();
                msg.param
                    .set_int(Param::Cmd, SystemMessage::GroupImageChanged as i32);
                msg.type_0 = Viewtype::Text;
                msg.text = Some(context.stock_system_msg(
                    if new_image_rel == "" {
                        msg.param.remove(Param::Arg);
                        StockMessage::MsgGrpImgDeleted
                    } else {
                        msg.param.set(Param::Arg, &new_image_rel);
                        StockMessage::MsgGrpImgChanged
                    },
                    "",
                    "",
                    DC_CONTACT_ID_SELF,
                ));
                msg.id = send_msg(context, chat_id, &mut msg)?;
                emit_event!(
                    context,
                    Event::MsgsChanged {
                        chat_id,
                        msg_id: msg.id
                    }
                );
            }
            emit_event!(context, Event::ChatModified(chat_id));
            return Ok(());
        }
    }
    bail!("Failed to set profile image");
}

pub fn forward_msgs(context: &Context, msg_ids: &[u32], chat_id: u32) -> Result<(), Error> {
    ensure!(!msg_ids.is_empty(), "empty msgs_ids: no one to forward to");
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "can not forward to special chat"
    );

    let mut created_db_entries = Vec::new();
    let mut curr_timestamp: i64;

    unarchive(context, chat_id).unwrap();
    if let Ok(mut chat) = Chat::load_from_db(context, chat_id) {
        curr_timestamp = dc_create_smeared_timestamps(context, msg_ids.len());
        let idsstr = msg_ids
            .into_iter()
            .enumerate()
            .fold(String::with_capacity(2 * msg_ids.len()), |acc, (i, n)| {
                (if i == 0 { acc } else { acc + "," }) + &n.to_string()
            });

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
            let msg = Message::load_from_db(context, src_msg_id as u32);
            if msg.is_err() {
                break;
            }
            let mut msg = msg.unwrap();
            let original_param = msg.param.clone();
            if msg.from_id != DC_CONTACT_ID_SELF {
                msg.param.set_int(Param::Forwarded, 1);
            }
            msg.param.remove(Param::GuranteeE2ee);
            msg.param.remove(Param::ForcePlaintext);
            msg.param.remove(Param::Cmd);

            let new_msg_id: u32;
            if msg.state == MessageState::OutPreparing {
                let fresh9 = curr_timestamp;
                curr_timestamp = curr_timestamp + 1;
                new_msg_id = chat
                    .prepare_msg_raw(context, &mut msg, fresh9)
                    .unwrap_or_default();
                let save_param = msg.param.clone();
                msg.param = original_param;
                msg.id = src_msg_id as u32;

                if let Some(old_fwd) = msg.param.get(Param::PrepForwards) {
                    let new_fwd = format!("{} {}", old_fwd, new_msg_id);
                    msg.param.set(Param::PrepForwards, new_fwd);
                } else {
                    msg.param.set(Param::PrepForwards, new_msg_id.to_string());
                }

                msg.save_param_to_disk(context);
                msg.param = save_param;
            } else {
                msg.state = MessageState::OutPending;
                let fresh10 = curr_timestamp;
                curr_timestamp = curr_timestamp + 1;
                new_msg_id = chat
                    .prepare_msg_raw(context, &mut msg, fresh10)
                    .unwrap_or_default();
                job_send_msg(context, new_msg_id)?;
            }
            created_db_entries.push(chat_id);
            created_db_entries.push(new_msg_id);
        }
    }

    for i in (0..created_db_entries.len()).step_by(2) {
        context.call_cb(Event::MsgsChanged {
            chat_id: created_db_entries[i],
            msg_id: created_db_entries[i + 1],
        });
    }

    Ok(())
}

pub fn get_chat_contact_cnt(context: &Context, chat_id: u32) -> usize {
    context
        .sql
        .query_get_value::<_, isize>(
            context,
            "SELECT COUNT(*) FROM chats_contacts WHERE chat_id=?;",
            params![chat_id as i32],
        )
        .unwrap_or_default() as usize
}

pub fn get_chat_cnt(context: &Context) -> usize {
    if context.sql.is_open() {
        /* no database, no chats - this is no error (needed eg. for information) */
        context
            .sql
            .query_get_value::<_, isize>(
                context,
                "SELECT COUNT(*) FROM chats WHERE id>9 AND blocked=0;",
                params![],
            )
            .unwrap_or_default() as usize
    } else {
        0
    }
}

pub fn get_chat_id_by_grpid(context: &Context, grpid: impl AsRef<str>) -> (u32, bool, Blocked) {
    context
        .sql
        .query_row(
            "SELECT id, blocked, type FROM chats WHERE grpid=?;",
            params![grpid.as_ref()],
            |row| {
                let chat_id = row.get(0)?;

                let b = row.get::<_, Option<Blocked>>(1)?.unwrap_or_default();
                let v = row.get::<_, Option<Chattype>>(2)?.unwrap_or_default();
                Ok((chat_id, v == Chattype::VerifiedGroup, b))
            },
        )
        .unwrap_or((0, false, Blocked::Not))
}

pub fn add_device_msg(context: &Context, chat_id: u32, text: impl AsRef<str>) {
    let rfc724_mid = dc_create_outgoing_rfc724_mid(None, "@device");

    if context.sql.execute(
        "INSERT INTO msgs (chat_id,from_id,to_id, timestamp,type,state, txt,rfc724_mid) VALUES (?,?,?, ?,?,?, ?,?);",
        params![
            chat_id as i32,
            2,
            2,
            dc_create_smeared_timestamp(context),
            Viewtype::Text,
            MessageState::InNoticed,
            text.as_ref(),
            rfc724_mid,
        ]
    ).is_err() {
        return;
    }

    let msg_id = sql::get_rowid(context, &context.sql, "msgs", "rfc724_mid", &rfc724_mid);
    context.call_cb(Event::MsgsChanged { chat_id, msg_id });
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    #[test]
    fn test_get_draft_no_draft() {
        let t = dummy_context();
        let chat_id = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF).unwrap();
        let draft = get_draft(&t.ctx, chat_id).unwrap();
        assert!(draft.is_none());
    }

    #[test]
    fn test_get_draft_special_chat_id() {
        let t = dummy_context();
        let draft = get_draft(&t.ctx, DC_CHAT_ID_LAST_SPECIAL).unwrap();
        assert!(draft.is_none());
    }

    #[test]
    fn test_get_draft_no_chat() {
        // This is a weird case, maybe this should be an error but we
        // do not get this info from the database currently.
        let t = dummy_context();
        let draft = get_draft(&t.ctx, 42).unwrap();
        assert!(draft.is_none());
    }

    #[test]
    fn test_get_draft() {
        let t = dummy_context();
        let chat_id = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF).unwrap();
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hello".to_string()));
        set_draft(&t.ctx, chat_id, Some(&mut msg));
        let draft = get_draft(&t.ctx, chat_id).unwrap().unwrap();
        let msg_text = msg.get_text();
        let draft_text = draft.get_text();
        assert_eq!(msg_text, draft_text);
    }
}

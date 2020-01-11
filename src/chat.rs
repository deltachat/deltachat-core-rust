//! # Chat module

use std::path::{Path, PathBuf};

use itertools::Itertools;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};

use crate::blob::{BlobError, BlobObject};
use crate::chatlist::*;
use crate::config::*;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::Error;
use crate::events::Event;
use crate::job::*;
use crate::message::{self, InvalidMsgId, Message, MessageState, MsgId};
use crate::mimeparser::SystemMessage;
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
    is_sending_locations: bool,
}

impl Chat {
    /// Loads chat from the database by its ID.
    pub fn load_from_db(context: &Context, chat_id: u32) -> Result<Self, Error> {
        let res = context.sql.query_row(
            "SELECT c.id,c.type,c.name, c.grpid,c.param,c.archived, \
             c.blocked, c.locations_send_until  \
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
                    is_sending_locations: row.get(7)?,
                };

                Ok(c)
            },
        );

        match res {
            Err(err @ crate::sql::Error::Sql(rusqlite::Error::QueryReturnedNoRows)) => {
                Err(err.into())
            }
            Err(err) => {
                error!(
                    context,
                    "chat: failed to load from db {}: {:?}", chat_id, err
                );
                Err(err.into())
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

                            if let Some(contact_id) = contacts.first() {
                                if let Ok(contact) = Contact::get_by_id(context, *contact_id) {
                                    chat_name = contact.get_display_name().to_owned();
                                }
                            }

                            chat.name = chat_name;
                        }

                        if chat.param.exists(Param::Selftalk) {
                            chat.name = context.stock_str(StockMessage::SavedMessages).into();
                        } else if chat.param.exists(Param::Devicetalk) {
                            chat.name = context.stock_str(StockMessage::DeviceMessages).into();
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

    /// Returns true if chat is a device chat.
    pub fn is_device_talk(&self) -> bool {
        self.param.exists(Param::Devicetalk)
    }

    /// Returns true if user can send messages to this chat.
    pub fn can_send(&self) -> bool {
        self.id > DC_CHAT_ID_LAST_SPECIAL && !self.is_device_talk()
    }

    pub fn update_param(&mut self, context: &Context) -> Result<(), Error> {
        sql::execute(
            context,
            &context.sql,
            "UPDATE chats SET param=? WHERE id=?",
            params![self.param.to_string(), self.id as i32],
        )?;
        Ok(())
    }

    /// Returns chat ID.
    pub fn get_id(&self) -> u32 {
        self.id
    }

    /// Returns chat type.
    pub fn get_type(&self) -> Chattype {
        self.typ
    }

    /// Returns chat name.
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
            if self.id == DC_CHAT_ID_DEADDROP {
                return context.stock_str(StockMessage::DeadDrop).into();
            }
            let cnt = get_chat_contact_cnt(context, self.id);
            return context.stock_string_repl_int(StockMessage::Member, cnt as i32);
        }

        "Err".to_string()
    }

    pub fn get_parent_mime_headers(&self, context: &Context) -> Option<(String, String, String)> {
        let collect = |row: &rusqlite::Row| Ok((row.get(0)?, row.get(1)?, row.get(2)?));
        let params = params![self.id as i32, DC_CONTACT_ID_SELF as i32];
        let sql = &context.sql;

        // use the last messsage of another user in the group as the parent
        let main_query = "SELECT rfc724_mid, mime_in_reply_to, mime_references \
                          FROM msgs WHERE chat_id=?1 AND timestamp=(SELECT max(timestamp) \
                          FROM msgs WHERE chat_id=?1 AND from_id!=?2);";

        // there are no messages of other users - use the first message if SELF as parent
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
            if let Some(contact_id) = contacts.first() {
                if let Ok(contact) = Contact::get_by_id(context, *contact_id) {
                    return contact.get_profile_image(context);
                }
            }
        }

        None
    }

    pub fn get_gossiped_timestamp(&self, context: &Context) -> i64 {
        get_gossiped_timestamp(context, self.id)
    }

    pub fn get_color(&self, context: &Context) -> u32 {
        let mut color = 0;

        if self.typ == Chattype::Single {
            let contacts = get_chat_contacts(context, self.id);
            if let Some(contact_id) = contacts.first() {
                if let Ok(contact) = Contact::get_by_id(context, *contact_id) {
                    color = contact.get_color();
                }
            }
        } else {
            color = dc_str_to_color(&self.name);
        }

        color
    }

    /// Returns a struct describing the current state of the chat.
    ///
    /// This is somewhat experimental, even more so than the rest of
    /// deltachat, and the data returned is still subject to change.
    pub fn get_info(&self, context: &Context) -> Result<ChatInfo, Error> {
        let draft = match get_draft(context, self.id)? {
            Some(message) => message.text.unwrap_or_else(String::new),
            _ => String::new(),
        };
        Ok(ChatInfo {
            id: self.id,
            type_: self.typ as u32,
            name: self.name.clone(),
            archived: self.archived,
            param: self.param.to_string(),
            gossiped_timestamp: self.get_gossiped_timestamp(context),
            is_sending_locations: self.is_sending_locations,
            color: self.get_color(context),
            profile_image: self.get_profile_image(context).unwrap_or_else(PathBuf::new),
            subtitle: self.get_subtitle(context),
            draft,
        })
    }

    /// Returns true if the chat is archived.
    pub fn is_archived(&self) -> bool {
        self.archived
    }

    pub fn is_unpromoted(&self) -> bool {
        self.param.get_int(Param::Unpromoted).unwrap_or_default() == 1
    }

    pub fn is_promoted(&self) -> bool {
        !self.is_unpromoted()
    }

    /// Returns true if chat is a verified group chat.
    pub fn is_verified(&self) -> bool {
        self.typ == Chattype::VerifiedGroup
    }

    /// Returns true if location streaming is enabled in the chat.
    pub fn is_sending_locations(&self) -> bool {
        self.is_sending_locations
    }

    fn prepare_msg_raw(
        &mut self,
        context: &Context,
        msg: &mut Message,
        timestamp: i64,
    ) -> Result<MsgId, Error> {
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
            bail!("Cannot set to chat type #{}", self.typ);
        }

        if (self.typ == Chattype::Group || self.typ == Chattype::VerifiedGroup)
            && !is_contact_in_chat(context, self.id, DC_CONTACT_ID_SELF)
        {
            emit_event!(
                context,
                Event::ErrorSelfNotInGroup("Cannot send message; self not in group.".into())
            );
            bail!("Cannot set message; self not in group.");
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
                    bail!(
                        "Cannot set message, contact for chat #{} not found.",
                        self.id
                    );
                }
            } else if (self.typ == Chattype::Group || self.typ == Chattype::VerifiedGroup)
                && self.param.get_int(Param::Unpromoted).unwrap_or_default() == 1
            {
                msg.param.set_int(Param::AttachGroupImage, 1);
                self.param.remove(Param::Unpromoted);
                self.update_param(context)?;
            }

            /* check if we want to encrypt this message.  If yes and circumstances change
            so that E2EE is no longer available at a later point (reset, changed settings),
            we might not send the message out at all */
            if msg.param.get_int(Param::ForcePlaintext).unwrap_or_default() == 0 {
                let mut can_encrypt = true;
                let mut all_mutual = context.get_config_bool(Config::E2eeEnabled);

                // take care that this statement returns NULL rows
                // if there is no peerstates for a chat member!
                // for DC_PARAM_SELFTALK this statement does not return any row
                let res = context.sql.query_map(
                    "SELECT ps.prefer_encrypted, c.addr \
                     FROM chats_contacts cc  \
                     LEFT JOIN contacts c ON cc.contact_id=c.id  \
                     LEFT JOIN acpeerstates ps ON c.addr=ps.addr  \
                     WHERE cc.chat_id=?  AND cc.contact_id>9;",
                    params![self.id],
                    |row| {
                        let addr: String = row.get(1)?;

                        if let Some(prefer_encrypted) = row.get::<_, Option<i32>>(0)? {
                            // the peerstate exist, so we have either public_key or gossip_key
                            // and can encrypt potentially
                            if prefer_encrypted != 1 {
                                info!(
                                    context,
                                    "[autocrypt] peerstate for {} is {}",
                                    addr,
                                    if prefer_encrypted == 0 {
                                        "NOPREFERENCE"
                                    } else {
                                        "RESET"
                                    },
                                );
                                all_mutual = false;
                            }
                        } else {
                            info!(context, "[autocrypt] no peerstate for {}", addr,);
                            can_encrypt = false;
                            all_mutual = false;
                        }
                        Ok(())
                    },
                    |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
                );
                match res {
                    Ok(_) => {}
                    Err(err) => {
                        warn!(context, "chat: failed to load peerstates: {:?}", err);
                    }
                }

                if can_encrypt
                    && (all_mutual || last_msg_in_chat_encrypted(context, &context.sql, self.id))
                {
                    msg.param.set_int(Param::GuaranteeE2ee, 1);
                }
            }
            // reset encrypt error state eg. for forwarding
            msg.param.remove(Param::ErroneousE2ee);

            // set "In-Reply-To:" to identify the message to which the composed message is a reply;
            // set "References:" to identify the "thread" of the conversation;
            // both according to RFC 5322 3.6.4, page 25
            //
            // as self-talks are mainly used to transfer data between devices,
            // we do not set In-Reply-To/References in this case.
            if !self.is_self_talk() {
                if let Some((parent_rfc724_mid, parent_in_reply_to, parent_references)) =
                    self.get_parent_mime_headers(context)
                {
                    if !parent_rfc724_mid.is_empty() {
                        new_in_reply_to = parent_rfc724_mid.clone();
                    }

                    // the whole list of messages referenced may be huge;
                    // only use the oldest and and the parent message
                    let parent_references = if let Some(n) = parent_references.find(' ') {
                        &parent_references[0..n]
                    } else {
                        &parent_references
                    };

                    if !parent_references.is_empty() && !parent_rfc724_mid.is_empty() {
                        // angle brackets are added by the mimefactory later
                        new_references = format!("{} {}", parent_references, parent_rfc724_mid);
                    } else if !parent_references.is_empty() {
                        new_references = parent_references.to_string();
                    } else if !parent_in_reply_to.is_empty() && !parent_rfc724_mid.is_empty() {
                        new_references = format!("{} {}", parent_in_reply_to, parent_rfc724_mid);
                    } else if !parent_in_reply_to.is_empty() {
                        new_references = parent_in_reply_to;
                    }
                }
            }

            // add independent location to database

            if msg.param.exists(Param::SetLatitude)
                && sql::execute(
                    context,
                    &context.sql,
                    "INSERT INTO locations \
                     (timestamp,from_id,chat_id, latitude,longitude,independent)\
                     VALUES (?,?,?, ?,?,1);", // 1=DC_CONTACT_ID_SELF
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

            // add message to the database

            if sql::execute(
                        context,
                        &context.sql,
                        "INSERT INTO msgs (rfc724_mid, chat_id, from_id, to_id, timestamp, type, state, txt, param, hidden, mime_in_reply_to, mime_references, location_id) VALUES (?,?,?,?,?, ?,?,?,?,?, ?,?,?);",
                        params![
                            new_rfc724_mid,
                            self.id as i32,
                            DC_CONTACT_ID_SELF,
                            to_id as i32,
                            timestamp,
                            msg.viewtype,
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

        Ok(MsgId::new(msg_id))
    }
}

/// The current state of a chat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatInfo {
    /// The chat ID.
    pub id: u32,

    /// The type of chat as a `u32` representation of [Chattype].
    ///
    /// On the C API this number is one of the
    /// `DC_CHAT_TYPE_UNDEFINED`, `DC_CHAT_TYPE_SINGLE`,
    /// `DC_CHAT_TYPE_GROUP` or `DC_CHAT_TYPE_VERIFIED_GROUP`
    /// constants.
    #[serde(rename = "type")]
    pub type_: u32,

    /// The name of the chat.
    pub name: String,

    /// Whether the chat is archived.
    pub archived: bool,

    /// The "params" of the chat.
    ///
    /// This is the string-serialised version of [Params] currently.
    pub param: String,

    /// Last time this client sent autocrypt gossip headers to this chat.
    pub gossiped_timestamp: i64,

    /// Whether this chat is currently sending location-stream messages.
    pub is_sending_locations: bool,

    /// Colour this chat should be represented in by the UI.
    ///
    /// Yes, spelling colour is hard.
    pub color: u32,

    /// The path to the profile image.
    ///
    /// If there is no profile image set this will be an empty string
    /// currently.
    pub profile_image: PathBuf,

    /// Subtitle for the chat.
    pub subtitle: String,

    /// The draft message text.
    ///
    /// If the chat has not draft this is an empty string.
    ///
    /// TODO: This doesn't seem rich enough, it can not handle drafts
    ///       which contain non-text parts.  Perhaps it should be a
    ///       simple `has_draft` bool instead.
    pub draft: String,
    // ToDo:
    // - [ ] deaddrop,
    // - [ ] summary,
    // - [ ] lastUpdated,
    // - [ ] freshMessageCounter,
    // - [ ] email
}

/// Create a chat from a message ID.
///
/// Typically you'd do this for a message ID found in the
/// [DC_CHAT_ID_DEADDROP] which turns the chat the message belongs to
/// into a normal chat.  The chat can be a 1:1 chat or a group chat
/// and all messages belonging to the chat will be moved from the
/// deaddrop to the normal chat.
///
/// In reality the messages already belong to this chat as receive_imf
/// always creates chat IDs appropriately, so this function really
/// only unblocks the chat and "scales up" the origin of the contact
/// the message is from.
///
/// If prompting the user before calling this function, they should be
/// asked whether they want to chat with the **contact** the message
/// is from and **not** the group name since this can be really weird
/// and confusing when taken from subject of implicit groups.
///
/// # Returns
///
/// The "created" chat ID is returned.
pub fn create_by_msg_id(context: &Context, msg_id: MsgId) -> Result<u32, Error> {
    let msg = Message::load_from_db(context, msg_id)?;
    let chat = Chat::load_from_db(context, msg.chat_id)?;
    ensure!(
        chat.id > DC_CHAT_ID_LAST_SPECIAL,
        "Message can not belong to a special chat"
    );
    if chat.blocked != Blocked::Not {
        unblock(context, chat.id);

        // Sending with 0s as data since multiple messages may have changed.
        context.call_cb(Event::MsgsChanged {
            chat_id: 0,
            msg_id: MsgId::new(0),
        });
    }
    Contact::scaleup_origin_by_id(context, msg.from_id, Origin::CreateChat);
    Ok(chat.id)
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
        msg_id: MsgId::new(0),
    });

    Ok(chat_id)
}

pub fn unblock(context: &Context, chat_id: u32) {
    set_blocking(context, chat_id, Blocked::Not);
}

pub fn set_blocking(context: &Context, chat_id: u32, new_blocking: Blocked) -> bool {
    if chat_id == 0 {
        warn!(context, "ignoring setting of Block-status for chat_id=0");
        return false;
    }
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET blocked=? WHERE id=?;",
        params![new_blocking, chat_id as i32],
    )
    .is_ok()
}

pub fn update_saved_messages_icon(context: &Context) -> Result<(), Error> {
    // if there is no saved-messages chat, there is nothing to update. this is no error.
    if let Ok((chat_id, _)) = lookup_by_contact_id(context, DC_CONTACT_ID_SELF) {
        let icon = include_bytes!("../assets/icon-saved-messages.png");
        let blob = BlobObject::create(context, "icon-saved-messages.png".to_string(), icon)?;
        let icon = blob.as_name().to_string();

        let mut chat = Chat::load_from_db(context, chat_id)?;
        chat.param.set(Param::ProfileImage, icon);
        chat.update_param(context)?;
    }
    Ok(())
}

pub fn update_device_icon(context: &Context) -> Result<(), Error> {
    // if there is no device-chat, there is nothing to update. this is no error.
    if let Ok((chat_id, _)) = lookup_by_contact_id(context, DC_CONTACT_ID_DEVICE) {
        let icon = include_bytes!("../assets/icon-device.png");
        let blob = BlobObject::create(context, "icon-device.png".to_string(), icon)?;
        let icon = blob.as_name().to_string();

        let mut chat = Chat::load_from_db(context, chat_id)?;
        chat.param.set(Param::ProfileImage, &icon);
        chat.update_param(context)?;

        let mut contact = Contact::load_from_db(context, DC_CONTACT_ID_DEVICE)?;
        contact.param.set(Param::ProfileImage, icon);
        contact.update_param(context)?;
    }
    Ok(())
}

fn update_special_chat_name(
    context: &Context,
    contact_id: u32,
    stock_id: StockMessage,
) -> Result<(), Error> {
    if let Ok((chat_id, _)) = lookup_by_contact_id(context, contact_id) {
        let name: String = context.stock_str(stock_id).into();
        // the `!= name` condition avoids unneeded writes
        context.sql.execute(
            "UPDATE chats SET name=? WHERE id=? AND name!=?;",
            params![name, chat_id, name],
        )?;
    }
    Ok(())
}

pub fn update_special_chat_names(context: &Context) -> Result<(), Error> {
    update_special_chat_name(context, DC_CONTACT_ID_DEVICE, StockMessage::DeviceMessages)?;
    update_special_chat_name(context, DC_CONTACT_ID_SELF, StockMessage::SavedMessages)?;
    Ok(())
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
        "INSERT INTO chats (type, name, param, blocked, grpid, created_timestamp) VALUES(?, ?, ?, ?, ?, ?)",
        params![
            100,
            chat_name,
            match contact_id {
                DC_CONTACT_ID_SELF => "K=1".to_string(), // K = Param::Selftalk
                DC_CONTACT_ID_DEVICE => "D=1".to_string(), // D = Param::Devicetalk
                _ => "".to_string()
            },
            create_blocked as u8,
            contact.get_addr(),
            time(),
        ]
    )?;

    let chat_id = sql::get_rowid(context, &context.sql, "chats", "grpid", contact.get_addr());

    sql::execute(
        context,
        &context.sql,
        "INSERT INTO chats_contacts (chat_id, contact_id) VALUES(?, ?)",
        params![chat_id, contact_id],
    )?;

    if contact_id == DC_CONTACT_ID_SELF {
        update_saved_messages_icon(context)?;
    } else if contact_id == DC_CONTACT_ID_DEVICE {
        update_device_icon(context)?;
    }

    Ok((chat_id, create_blocked))
}

pub fn lookup_by_contact_id(context: &Context, contact_id: u32) -> Result<(u32, Blocked), Error> {
    ensure!(context.sql.is_open(), "Database not available");

    context.sql.query_row(
        "SELECT c.id, c.blocked FROM chats c INNER JOIN chats_contacts j ON c.id=j.chat_id WHERE c.type=100 AND c.id>9 AND j.contact_id=?;",
        params![contact_id as i32],
        |row| Ok((row.get(0)?, row.get::<_, Option<_>>(1)?.unwrap_or_default())),
    ).map_err(Into::into)
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
) -> Result<MsgId, Error> {
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
        Viewtype::Unknown => false,
        Viewtype::Text => false,
        Viewtype::Image => true,
        Viewtype::Gif => true,
        Viewtype::Sticker => true,
        Viewtype::Audio => true,
        Viewtype::Voice => true,
        Viewtype::Video => true,
        Viewtype::File => true,
    }
}

fn prepare_msg_blob(context: &Context, msg: &mut Message) -> Result<(), Error> {
    if msg.viewtype == Viewtype::Text {
        // the caller should check if the message text is empty
    } else if msgtype_has_file(msg.viewtype) {
        let blob = msg
            .param
            .get_blob(Param::File, context, !msg.is_increation())?
            .ok_or_else(|| {
                format_err!("Attachment missing for message of type #{}", msg.viewtype)
            })?;
        msg.param.set(Param::File, blob.as_name());

        if msg.viewtype == Viewtype::File || msg.viewtype == Viewtype::Image {
            // Correct the type, take care not to correct already very special
            // formats as GIF or VOICE.
            //
            // Typical conversions:
            // - from FILE to AUDIO/VIDEO/IMAGE
            // - from FILE/IMAGE to GIF */
            if let Some((better_type, better_mime)) =
                message::guess_msgtype_from_suffix(&blob.to_abs_path())
            {
                msg.viewtype = better_type;
                msg.param.set(Param::MimeType, better_mime);
            }
        } else if !msg.param.exists(Param::MimeType) {
            if let Some((_, mime)) = message::guess_msgtype_from_suffix(&blob.to_abs_path()) {
                msg.param.set(Param::MimeType, mime);
            }
        }
        info!(
            context,
            "Attaching \"{}\" for message type #{}.",
            blob.to_abs_path().display(),
            msg.viewtype
        );
    } else {
        bail!("Cannot send messages of type #{}.", msg.viewtype);
    }
    Ok(())
}

fn prepare_msg_common(context: &Context, chat_id: u32, msg: &mut Message) -> Result<MsgId, Error> {
    msg.id = MsgId::new_unset();
    prepare_msg_blob(context, msg)?;
    unarchive(context, chat_id)?;

    let mut chat = Chat::load_from_db(context, chat_id)?;
    ensure!(chat.can_send(), "cannot send to chat #{}", chat_id);

    // The OutPreparing state is set by dc_prepare_msg() before it
    // calls this function and the message is left in the OutPreparing
    // state.  Otherwise we got called by send_msg() and we change the
    // state to OutPending.
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
            Ok(param) => param.exists(Param::GuaranteeE2ee),
            Err(err) => {
                error!(context, "invalid params stored: '{}', {:?}", packed, err);
                false
            }
        }
    } else {
        false
    }
}

/// Returns whether a contact is in a chat or not.
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

// note that unarchive() is not the same as archive(false) -
// eg. unarchive() does not send events as done for archive(false).
pub fn unarchive(context: &Context, chat_id: u32) -> Result<(), Error> {
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET archived=0 WHERE id=?",
        params![chat_id as i32],
    )?;
    Ok(())
}

/// Send a message defined by a dc_msg_t object to a chat.
///
/// Sends the event #DC_EVENT_MSGS_CHANGED on succcess.
/// However, this does not imply, the message really reached the recipient -
/// sending may be delayed eg. due to network problems. However, from your
/// view, you're done with the message. Sooner or later it will find its way.
pub fn send_msg(context: &Context, chat_id: u32, msg: &mut Message) -> Result<MsgId, Error> {
    // dc_prepare_msg() leaves the message state to OutPreparing, we
    // only have to change the state to OutPending in this case.
    // Otherwise we still have to prepare the message, which will set
    // the state to OutPending.
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
                if let Ok(msg_id) = forward
                    .parse::<u32>()
                    .map_err(|_| InvalidMsgId)
                    .map(MsgId::new)
                {
                    if let Ok(mut msg) = Message::load_from_db(context, msg_id) {
                        send_msg(context, 0, &mut msg)?;
                    };
                }
            }
            msg.param.remove(Param::PrepForwards);
            msg.save_param_to_disk(context);
        }
    }

    Ok(msg.id)
}

pub fn send_text_msg(
    context: &Context,
    chat_id: u32,
    text_to_send: String,
) -> Result<MsgId, Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= DC_CHAT_ID_LAST_SPECIAL",
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
        context.call_cb(Event::MsgsChanged {
            chat_id,
            msg_id: MsgId::new(0),
        });
    }
}

/// Delete draft message in specified chat, if there is one.
///
/// Return {true}, if message was deleted, {false} otherwise.
fn maybe_delete_draft(context: &Context, chat_id: u32) -> bool {
    match get_draft_msg_id(context, chat_id) {
        Some(msg_id) => {
            Message::delete_from_db(context, msg_id);
            true
        }
        None => false,
    }
}

/// Set provided message as draft message for specified chat.
///
/// Return true on success, false on database error.
fn do_set_draft(context: &Context, chat_id: u32, msg: &mut Message) -> Result<(), Error> {
    match msg.viewtype {
        Viewtype::Unknown => bail!("Can not set draft of unknown type."),
        Viewtype::Text => match msg.text.as_ref() {
            Some(text) => {
                if text.is_empty() {
                    bail!("No text in draft");
                }
            }
            None => bail!("No text in draft"),
        },
        _ => {
            let blob = msg
                .param
                .get_blob(Param::File, context, !msg.is_increation())?
                .ok_or_else(|| format_err!("No file stored in params"))?;
            msg.param.set(Param::File, blob.as_name());
        }
    }
    sql::execute(
        context,
        &context.sql,
        "INSERT INTO msgs (chat_id, from_id, timestamp, type, state, txt, param, hidden) \
         VALUES (?,?,?, ?,?,?,?,?);",
        params![
            chat_id as i32,
            DC_CONTACT_ID_SELF,
            time(),
            msg.viewtype,
            MessageState::OutDraft,
            msg.text.as_ref().map(String::as_str).unwrap_or(""),
            msg.param.to_string(),
            1,
        ],
    )?;
    Ok(())
}

// similar to as dc_set_draft() but does not emit an event
fn set_draft_raw(context: &Context, chat_id: u32, msg: &mut Message) -> bool {
    let deleted = maybe_delete_draft(context, chat_id);
    let set = do_set_draft(context, chat_id, msg).is_ok();

    // Can't inline. Both functions above must be called, no shortcut!
    deleted || set
}

fn get_draft_msg_id(context: &Context, chat_id: u32) -> Option<MsgId> {
    context.sql.query_get_value::<_, MsgId>(
        context,
        "SELECT id FROM msgs WHERE chat_id=? AND state=?;",
        params![chat_id as i32, MessageState::OutDraft],
    )
}

pub fn get_draft(context: &Context, chat_id: u32) -> Result<Option<Message>, Error> {
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL {
        return Ok(None);
    }
    match get_draft_msg_id(context, chat_id) {
        Some(draft_msg_id) => Ok(Some(Message::load_from_db(context, draft_msg_id)?)),
        None => Ok(None),
    }
}

pub fn get_chat_msgs(
    context: &Context,
    chat_id: u32,
    flags: u32,
    marker1before: Option<MsgId>,
) -> Vec<MsgId> {
    let process_row =
        |row: &rusqlite::Row| Ok((row.get::<_, MsgId>("id")?, row.get::<_, i64>("timestamp")?));
    let process_rows = |rows: rusqlite::MappedRows<_>| {
        let mut ret = Vec::new();
        let mut last_day = 0;
        let cnv_to_local = dc_gm2local_offset();
        for row in rows {
            let (curr_id, ts) = row?;
            if let Some(marker_id) = marker1before {
                if curr_id == marker_id {
                    ret.push(MsgId::new(DC_MSG_ID_MARKER1));
                }
            }
            if (flags & DC_GCM_ADDDAYMARKER) != 0 {
                let curr_local_timestamp = ts + cnv_to_local;
                let curr_day = curr_local_timestamp / 86400;
                if curr_day != last_day {
                    ret.push(MsgId::new(DC_MSG_ID_DAYMARKER));
                    last_day = curr_day;
                }
            }
            ret.push(curr_id);
        }
        Ok(ret)
    };
    let success = if chat_id == DC_CHAT_ID_DEADDROP {
        let show_emails =
            ShowEmails::from_i32(context.get_config_int(Config::ShowEmails)).unwrap_or_default();
        context.sql.query_map(
            concat!(
                "SELECT m.id AS id, m.timestamp AS timestamp",
                " FROM msgs m",
                " LEFT JOIN chats",
                "        ON m.chat_id=chats.id",
                " LEFT JOIN contacts",
                "        ON m.from_id=contacts.id",
                " WHERE m.from_id!=1", // 1=DC_CONTACT_ID_SELF
                "   AND m.from_id!=2", // 2=DC_CONTACT_ID_INFO
                "   AND m.hidden=0",
                "   AND chats.blocked=2",
                "   AND contacts.blocked=0",
                "   AND m.msgrmsg>=?",
                " ORDER BY m.timestamp,m.id;"
            ),
            params![if show_emails == ShowEmails::All { 0 } else { 1 }],
            process_row,
            process_rows,
        )
    } else if chat_id == DC_CHAT_ID_STARRED {
        context.sql.query_map(
            concat!(
                "SELECT m.id AS id, m.timestamp AS timestamp",
                " FROM msgs m",
                " LEFT JOIN contacts ct",
                "        ON m.from_id=ct.id",
                " WHERE m.starred=1",
                "   AND m.hidden=0",
                "   AND ct.blocked=0",
                " ORDER BY m.timestamp,m.id;"
            ),
            params![],
            process_row,
            process_rows,
        )
    } else {
        context.sql.query_map(
            concat!(
                "SELECT m.id AS id, m.timestamp AS timestamp",
                " FROM msgs m",
                " WHERE m.chat_id=?",
                "   AND m.hidden=0",
                " ORDER BY m.timestamp, m.id;"
            ),
            params![chat_id as i32],
            process_row,
            process_rows,
        )
    };
    match success {
        Ok(ret) => ret,
        Err(e) => {
            error!(context, "Failed to get chat messages: {}", e);
            Vec::new()
        }
    }
}

/// Returns number of messages in a chat.
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
        msg_id: MsgId::new(0),
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
        msg_id: MsgId::new(0),
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
) -> Vec<MsgId> {
    context
        .sql
        .query_map(
            concat!(
                "SELECT",
                "    id",
                " FROM msgs",
                " WHERE chat_id=? AND (type=? OR type=? OR type=?)",
                " ORDER BY timestamp, id;"
            ),
            params![
                chat_id as i32,
                msg_type,
                if msg_type2 != Viewtype::Unknown {
                    msg_type2
                } else {
                    msg_type
                },
                if msg_type3 != Viewtype::Unknown {
                    msg_type3
                } else {
                    msg_type
                },
            ],
            |row| row.get::<_, MsgId>(0),
            |ids| {
                let mut ret = Vec::new();
                for id in ids {
                    if let Ok(msg_id) = id {
                        ret.push(msg_id)
                    }
                }
                Ok(ret)
            },
        )
        .unwrap_or_default()
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
    curr_msg_id: MsgId,
    direction: Direction,
    msg_type: Viewtype,
    msg_type2: Viewtype,
    msg_type3: Viewtype,
) -> Option<MsgId> {
    let mut ret: Option<MsgId> = None;

    if let Ok(msg) = Message::load_from_db(context, curr_msg_id) {
        let list: Vec<MsgId> = get_chat_media(
            context,
            msg.chat_id,
            if msg_type != Viewtype::Unknown {
                msg_type
            } else {
                msg.viewtype
            },
            msg_type2,
            msg_type3,
        );
        for i in 0..list.len() {
            if curr_msg_id == list[i] {
                match direction {
                    Direction::Forward => {
                        if i + 1 < list.len() {
                            ret = Some(list[i + 1]);
                        }
                    }
                    Direction::Backward => {
                        if i >= 1 {
                            ret = Some(list[i - 1]);
                        }
                    }
                }
                break;
            }
        }
    }
    ret
}

/// Archives or unarchives a chat.
pub fn archive(context: &Context, chat_id: u32, archive: bool) -> Result<(), Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= DC_CHAT_ID_LAST_SPECIAL",
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
        msg_id: MsgId::new(0),
        chat_id: 0,
    });

    Ok(())
}

/// Deletes a chat.
pub fn delete(context: &Context, chat_id: u32) -> Result<(), Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= DC_CHAT_ID_LAST_SPECIAL",
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
        msg_id: MsgId::new(0),
        chat_id: 0,
    });

    job_kill_action(context, Action::Housekeeping);
    job_add(context, Action::Housekeeping, 0, Params::new(), 10);

    Ok(())
}

pub fn get_chat_contacts(context: &Context, chat_id: u32) -> Vec<u32> {
    /* Normal chats do not include SELF.  Group chats do (as it may happen that one is deleted from a
    groupchat but the chats stays visible, moreover, this makes displaying lists easier) */

    if chat_id == DC_CHAT_ID_DEADDROP {
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
        "INSERT INTO chats (type, name, grpid, param, created_timestamp) VALUES(?, ?, ?, \'U=1\', ?);",
        params![
            if verified != VerifiedStatus::Unverified {
                Chattype::VerifiedGroup
            } else {
                Chattype::Group
            },
            chat_name.as_ref(),
            grpid,
            time(),
        ],
    )?;

    let chat_id = sql::get_rowid(context, &context.sql, "chats", "grpid", grpid);

    if chat_id != 0 {
        if add_to_chat_contacts_table(context, chat_id, DC_CONTACT_ID_SELF) {
            let mut draft_msg = Message::new(Viewtype::Text);
            draft_msg.set_text(Some(draft_txt));
            set_draft_raw(context, chat_id, &mut draft_msg);
        }

        context.call_cb(Event::MsgsChanged {
            msg_id: MsgId::new(0),
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

/// Adds a contact to the chat.
pub fn add_contact_to_chat(context: &Context, chat_id: u32, contact_id: u32) -> bool {
    match add_contact_to_chat_ex(context, chat_id, contact_id, false) {
        Ok(res) => res,
        Err(err) => {
            error!(context, "failed to add contact: {}", err);
            false
        }
    }
}

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

    reset_gossiped_timestamp(context, chat_id)?;

    /*this also makes sure, not contacts are added to special or normal chats*/
    let mut chat = Chat::load_from_db(context, chat_id)?;
    ensure!(
        real_group_exists(context, chat_id),
        "chat_id {} is not a group where one can add members",
        chat_id
    );
    ensure!(
        Contact::real_exists_by_id(context, contact_id) || contact_id == DC_CONTACT_ID_SELF,
        "invalid contact_id {} for adding to group",
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
        chat.update_param(context)?;
    }
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_default();
    if addr_cmp(contact.get_addr(), &self_addr) {
        // ourself is added using DC_CONTACT_ID_SELF, do not add this address explicitly.
        // if SELF is not in the group, members cannot be added at all.
        warn!(
            context,
            "invalid attempt to add self e-mail address to group"
        );
        return Ok(false);
    }

    if is_contact_in_chat(context, chat_id, contact_id) {
        if !from_handshake {
            return Ok(true);
        }
    } else {
        // else continue and send status mail
        if chat.typ == Chattype::VerifiedGroup
            && contact.is_verified(context) != VerifiedStatus::BidirectVerified
        {
            error!(
                context,
                "Only bidirectional verified contacts can be added to verified groups."
            );
            return Ok(false);
        }
        if !add_to_chat_contacts_table(context, chat_id, contact_id) {
            return Ok(false);
        }
    }
    if chat.param.get_int(Param::Unpromoted).unwrap_or_default() == 0 {
        msg.viewtype = Viewtype::Text;
        msg.text = Some(context.stock_system_msg(
            StockMessage::MsgAddMember,
            contact.get_addr(),
            "",
            DC_CONTACT_ID_SELF as u32,
        ));
        msg.param.set_cmd(SystemMessage::MemberAddedToGroup);
        msg.param.set(Param::Arg, contact.get_addr());
        msg.param.set_int(Param::Arg2, from_handshake.into());
        msg.id = send_msg(context, chat_id, &mut msg)?;
        context.call_cb(Event::MsgsChanged {
            chat_id,
            msg_id: msg.id,
        });
    }
    context.call_cb(Event::MsgsChanged {
        chat_id,
        msg_id: MsgId::new(0),
    });
    context.call_cb(Event::ChatModified(chat_id));
    Ok(true)
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

pub fn reset_gossiped_timestamp(context: &Context, chat_id: u32) -> Result<(), Error> {
    set_gossiped_timestamp(context, chat_id, 0)
}

/// Get timestamp of the last gossip sent in the chat.
/// Zero return value means that gossip was never sent.
pub fn get_gossiped_timestamp(context: &Context, chat_id: u32) -> i64 {
    context
        .sql
        .query_get_value::<_, i64>(
            context,
            "SELECT gossiped_timestamp FROM chats WHERE id=?;",
            params![chat_id as i32],
        )
        .unwrap_or_default()
}

pub fn set_gossiped_timestamp(
    context: &Context,
    chat_id: u32,
    timestamp: i64,
) -> Result<(), Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "can not add member to special chats"
    );
    info!(
        context,
        "set gossiped_timestamp for chat #{} to {}.", chat_id, timestamp,
    );

    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET gossiped_timestamp=? WHERE id=?;",
        params![timestamp, chat_id as i32],
    )?;
    Ok(())
}

pub fn shall_attach_selfavatar(context: &Context, chat_id: u32) -> Result<bool, Error> {
    // versions before 12/2019 already allowed to set selfavatar, however, it was never sent to others.
    // to avoid sending out previously set selfavatars unexpectedly we added this additional check.
    // it can be removed after some time.
    if !context
        .sql
        .get_raw_config_bool(context, "attach_selfavatar")
    {
        return Ok(false);
    }

    let timestamp_some_days_ago = time() - DC_RESEND_USER_AVATAR_DAYS * 24 * 60 * 60;
    let needs_attach = context.sql.query_map(
        "SELECT c.selfavatar_sent
           FROM chats_contacts cc
           LEFT JOIN contacts c ON c.id=cc.contact_id
          WHERE cc.chat_id=? AND cc.contact_id!=?;",
        params![chat_id, DC_CONTACT_ID_SELF],
        |row| Ok(row.get::<_, i64>(0)),
        |rows| {
            let mut needs_attach = false;
            for row in rows {
                if let Ok(selfavatar_sent) = row {
                    let selfavatar_sent = selfavatar_sent?;
                    if selfavatar_sent < timestamp_some_days_ago {
                        needs_attach = true;
                    }
                }
            }
            Ok(needs_attach)
        },
    )?;
    Ok(needs_attach)
}

pub fn set_selfavatar_timestamp(
    context: &Context,
    chat_id: u32,
    timestamp: i64,
) -> Result<(), Error> {
    context.sql.execute(
        "UPDATE contacts
            SET selfavatar_sent=?
          WHERE id IN(SELECT contact_id FROM chats_contacts WHERE chat_id=?);",
        params![timestamp, chat_id],
    )?;
    Ok(())
}

pub fn remove_contact_from_chat(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
) -> Result<(), Error> {
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "bad chat_id = {} <= DC_CHAT_ID_LAST_SPECIAL",
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
            if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF) {
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
                        msg.viewtype = Viewtype::Text;
                        if contact.id == DC_CONTACT_ID_SELF {
                            set_group_explicitly_left(context, chat.grpid)?;
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
                        msg.param.set_cmd(SystemMessage::MemberRemovedFromGroup);
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
    context
        .sql
        .exists(
            "SELECT id FROM leftgrps WHERE grpid=?;",
            params![grpid.as_ref()],
        )
        .map_err(Into::into)
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
        if chat.name == new_name.as_ref() {
            success = true;
        } else if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF) {
            emit_event!(
                context,
                Event::ErrorSelfNotInGroup("Cannot set chat name; self not in group".into())
            );
        } else {
            /* we should respect this - whatever we send to the group, it gets discarded anyway! */
            if sql::execute(
                context,
                &context.sql,
                "UPDATE chats SET name=? WHERE id=?;",
                params![new_name.as_ref(), chat_id as i32],
            )
            .is_ok()
            {
                if chat.is_promoted() {
                    msg.viewtype = Viewtype::Text;
                    msg.text = Some(context.stock_system_msg(
                        StockMessage::MsgGrpName,
                        &chat.name,
                        new_name.as_ref(),
                        DC_CONTACT_ID_SELF,
                    ));
                    msg.param.set_cmd(SystemMessage::GroupNameChanged);
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

/// Set a new profile image for the chat.
///
/// The profile image can only be set when you are a member of the
/// chat.  To remove the profile image pass an empty string for the
/// `new_image` parameter.
pub fn set_chat_profile_image(
    context: &Context,
    chat_id: u32,
    new_image: impl AsRef<str>, // XXX use PathBuf
) -> Result<(), Error> {
    ensure!(chat_id > DC_CHAT_ID_LAST_SPECIAL, "Invalid chat ID");
    let mut chat = Chat::load_from_db(context, chat_id)?;
    ensure!(
        real_group_exists(context, chat_id),
        "Failed to set profile image; group does not exist"
    );
    /* we should respect this - whatever we send to the group, it gets discarded anyway! */
    if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF) {
        emit_event!(
            context,
            Event::ErrorSelfNotInGroup("Cannot set chat profile image; self not in group.".into())
        );
        bail!("Failed to set profile image");
    }
    let mut msg = Message::new(Viewtype::Text);
    msg.param
        .set_int(Param::Cmd, SystemMessage::GroupImageChanged as i32);
    if new_image.as_ref().is_empty() {
        chat.param.remove(Param::ProfileImage);
        msg.param.remove(Param::Arg);
        msg.text = Some(context.stock_system_msg(
            StockMessage::MsgGrpImgDeleted,
            "",
            "",
            DC_CONTACT_ID_SELF,
        ));
    } else {
        let image_blob = BlobObject::from_path(context, Path::new(new_image.as_ref())).or_else(
            |err| match err {
                BlobError::WrongBlobdir { .. } => {
                    BlobObject::create_and_copy(context, Path::new(new_image.as_ref()))
                }
                _ => Err(err),
            },
        )?;
        image_blob.recode_to_avatar_size(context)?;
        chat.param.set(Param::ProfileImage, image_blob.as_name());
        msg.param.set(Param::Arg, image_blob.as_name());
        msg.text = Some(context.stock_system_msg(
            StockMessage::MsgGrpImgChanged,
            "",
            "",
            DC_CONTACT_ID_SELF,
        ));
    }
    chat.update_param(context)?;
    if chat.is_promoted() {
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
    Ok(())
}

pub fn forward_msgs(context: &Context, msg_ids: &[MsgId], chat_id: u32) -> Result<(), Error> {
    ensure!(!msg_ids.is_empty(), "empty msgs_ids: nothing to forward");
    ensure!(
        chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "can not forward to special chat"
    );

    let mut created_chats: Vec<u32> = Vec::new();
    let mut created_msgs: Vec<MsgId> = Vec::new();
    let mut curr_timestamp: i64;

    unarchive(context, chat_id)?;
    if let Ok(mut chat) = Chat::load_from_db(context, chat_id) {
        ensure!(chat.can_send(), "cannot send to chat #{}", chat_id);
        curr_timestamp = dc_create_smeared_timestamps(context, msg_ids.len());
        let ids = context.sql.query_map(
            format!(
                "SELECT id FROM msgs WHERE id IN({}) ORDER BY timestamp,id",
                msg_ids.iter().map(|_| "?").join(",")
            ),
            msg_ids,
            |row| row.get::<_, MsgId>(0),
            |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into),
        )?;

        for id in ids {
            let src_msg_id: MsgId = id;
            let msg = Message::load_from_db(context, src_msg_id);
            if msg.is_err() {
                break;
            }
            let mut msg = msg.unwrap();
            let original_param = msg.param.clone();

            // we tested a sort of broadcast
            // by not marking own forwarded messages as such,
            // however, this turned out to be to confusing and unclear.
            msg.param.set_int(Param::Forwarded, 1);

            msg.param.remove(Param::GuaranteeE2ee);
            msg.param.remove(Param::ForcePlaintext);
            msg.param.remove(Param::Cmd);

            let new_msg_id: MsgId;
            if msg.state == MessageState::OutPreparing {
                let fresh9 = curr_timestamp;
                curr_timestamp += 1;
                new_msg_id = chat.prepare_msg_raw(context, &mut msg, fresh9)?;
                let save_param = msg.param.clone();
                msg.param = original_param;
                msg.id = src_msg_id;

                if let Some(old_fwd) = msg.param.get(Param::PrepForwards) {
                    let new_fwd = format!("{} {}", old_fwd, new_msg_id.to_u32());
                    msg.param.set(Param::PrepForwards, new_fwd);
                } else {
                    msg.param
                        .set(Param::PrepForwards, new_msg_id.to_u32().to_string());
                }

                msg.save_param_to_disk(context);
                msg.param = save_param;
            } else {
                msg.state = MessageState::OutPending;
                let fresh10 = curr_timestamp;
                curr_timestamp += 1;
                new_msg_id = chat.prepare_msg_raw(context, &mut msg, fresh10)?;
                job_send_msg(context, new_msg_id)?;
            }
            created_chats.push(chat_id);
            created_msgs.push(new_msg_id);
        }
    }
    for (chat_id, msg_id) in created_chats.iter().zip(created_msgs.iter()) {
        context.call_cb(Event::MsgsChanged {
            chat_id: *chat_id,
            msg_id: *msg_id,
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

/// Adds a message to device chat.
///
/// Optional `label` can be provided to ensure that message is added only once.
pub fn add_device_msg(
    context: &Context,
    label: Option<&str>,
    msg: Option<&mut Message>,
) -> Result<MsgId, Error> {
    ensure!(
        label.is_some() || msg.is_some(),
        "device-messages need label, msg or both"
    );
    let mut chat_id = 0;
    let mut msg_id = MsgId::new_unset();

    if let Some(label) = label {
        if was_device_msg_ever_added(context, label)? {
            info!(context, "device-message {} already added", label);
            return Ok(msg_id);
        }
    }

    if let Some(msg) = msg {
        chat_id = create_or_lookup_by_contact_id(context, DC_CONTACT_ID_DEVICE, Blocked::Not)?.0;

        let rfc724_mid = dc_create_outgoing_rfc724_mid(None, "@device");
        msg.try_calc_and_set_dimensions(context).ok();
        prepare_msg_blob(context, msg)?;
        unarchive(context, chat_id)?;

        context.sql.execute(
            "INSERT INTO msgs (chat_id,from_id,to_id, timestamp,type,state, txt,param,rfc724_mid) \
             VALUES (?,?,?, ?,?,?, ?,?,?);",
            params![
                chat_id,
                DC_CONTACT_ID_DEVICE,
                DC_CONTACT_ID_SELF,
                dc_create_smeared_timestamp(context),
                msg.viewtype,
                MessageState::InFresh,
                msg.text.as_ref().map_or("", String::as_str),
                msg.param.to_string(),
                rfc724_mid,
            ],
        )?;

        let row_id = sql::get_rowid(context, &context.sql, "msgs", "rfc724_mid", &rfc724_mid);
        msg_id = MsgId::new(row_id);
    }

    if let Some(label) = label {
        context.sql.execute(
            "INSERT INTO devmsglabels (label) VALUES (?);",
            params![label],
        )?;
    }

    if !msg_id.is_unset() {
        context.call_cb(Event::IncomingMsg { chat_id, msg_id });
    }

    Ok(msg_id)
}

pub fn was_device_msg_ever_added(context: &Context, label: &str) -> Result<bool, Error> {
    ensure!(!label.is_empty(), "empty label");
    if let Ok(()) = context.sql.query_row(
        "SELECT label FROM devmsglabels WHERE label=?",
        params![label],
        |_| Ok(()),
    ) {
        return Ok(true);
    }

    Ok(false)
}

// needed on device-switches during export/import;
// - deletion in `msgs` with `DC_CONTACT_ID_DEVICE` makes sure,
//   no wrong information are shown in the device chat
// - deletion in `devmsglabels` makes sure,
//   deleted messages are resetted and useful messages can be added again
pub fn delete_and_reset_all_device_msgs(context: &Context) -> Result<(), Error> {
    context.sql.execute(
        "DELETE FROM msgs WHERE from_id=?;",
        params![DC_CONTACT_ID_DEVICE],
    )?;
    context
        .sql
        .execute("DELETE FROM devmsglabels;", params![])?;
    Ok(())
}

/// Adds an informational message to chat.
///
/// For example, it can be a message showing that a member was added to a group.
pub fn add_info_msg(context: &Context, chat_id: u32, text: impl AsRef<str>) {
    let rfc724_mid = dc_create_outgoing_rfc724_mid(None, "@device");

    if context.sql.execute(
        "INSERT INTO msgs (chat_id,from_id,to_id, timestamp,type,state, txt,rfc724_mid) VALUES (?,?,?, ?,?,?, ?,?);",
        params![
            chat_id as i32,
            DC_CONTACT_ID_INFO,
            DC_CONTACT_ID_INFO,
            dc_create_smeared_timestamp(context),
            Viewtype::Text,
            MessageState::InNoticed,
            text.as_ref(),
            rfc724_mid,
        ]
    ).is_err() {
        return;
    }

    let row_id = sql::get_rowid(context, &context.sql, "msgs", "rfc724_mid", &rfc724_mid);
    context.call_cb(Event::MsgsChanged {
        chat_id,
        msg_id: MsgId::new(row_id),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contact::Contact;
    use crate::test_utils::*;

    #[test]
    fn test_chat_info() {
        let t = dummy_context();
        let bob = Contact::create(&t.ctx, "bob", "bob@example.com").unwrap();
        let chat_id = create_by_contact_id(&t.ctx, bob).unwrap();
        let chat = Chat::load_from_db(&t.ctx, chat_id).unwrap();
        let info = chat.get_info(&t.ctx).unwrap();

        // Ensure we can serialise this.
        println!("{}", serde_json::to_string_pretty(&info).unwrap());

        let expected = r#"
            {
                "id": 10,
                "type": 100,
                "name": "bob",
                "archived": false,
                "param": "",
                "gossiped_timestamp": 0,
                "is_sending_locations": false,
                "color": 15895624,
                "profile_image": "",
                "subtitle": "bob@example.com",
                "draft": ""
            }
        "#;

        // Ensure we can deserialise this.
        let loaded: ChatInfo = serde_json::from_str(expected).unwrap();
        assert_eq!(info, loaded);
    }

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

    #[test]
    fn test_add_contact_to_chat_ex_add_self() {
        // Adding self to a contact should succeed, even though it's pointless.
        let t = test_context(Some(Box::new(logging_cb)));
        let chat_id = create_group_chat(&t.ctx, VerifiedStatus::Unverified, "foo").unwrap();
        let added = add_contact_to_chat_ex(&t.ctx, chat_id, DC_CONTACT_ID_SELF, false).unwrap();
        assert_eq!(added, false);
    }

    #[test]
    fn test_self_talk() {
        let t = dummy_context();
        let chat_id = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF).unwrap();
        assert_eq!(DC_CONTACT_ID_SELF, 1);
        assert!(chat_id > DC_CHAT_ID_LAST_SPECIAL);
        let chat = Chat::load_from_db(&t.ctx, chat_id).unwrap();
        assert_eq!(chat.id, chat_id);
        assert!(chat.is_self_talk());
        assert!(!chat.archived);
        assert!(!chat.is_device_talk());
        assert!(chat.can_send());
        assert_eq!(chat.name, t.ctx.stock_str(StockMessage::SavedMessages));
        assert!(chat.get_profile_image(&t.ctx).is_some());
    }

    #[test]
    fn test_deaddrop_chat() {
        let t = dummy_context();
        let chat = Chat::load_from_db(&t.ctx, DC_CHAT_ID_DEADDROP).unwrap();
        assert_eq!(DC_CHAT_ID_DEADDROP, 1);
        assert_eq!(chat.id, DC_CHAT_ID_DEADDROP);
        assert!(!chat.is_self_talk());
        assert!(!chat.archived);
        assert!(!chat.is_device_talk());
        assert!(!chat.can_send());
        assert_eq!(chat.name, t.ctx.stock_str(StockMessage::DeadDrop));
    }

    #[test]
    fn test_add_device_msg_unlabelled() {
        let t = test_context(Some(Box::new(logging_cb)));

        // add two device-messages
        let mut msg1 = Message::new(Viewtype::Text);
        msg1.text = Some("first message".to_string());
        let msg1_id = add_device_msg(&t.ctx, None, Some(&mut msg1));
        assert!(msg1_id.is_ok());

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.text = Some("second message".to_string());
        let msg2_id = add_device_msg(&t.ctx, None, Some(&mut msg2));
        assert!(msg2_id.is_ok());
        assert_ne!(msg1_id.as_ref().unwrap(), msg2_id.as_ref().unwrap());

        // check added messages
        let msg1 = message::Message::load_from_db(&t.ctx, msg1_id.unwrap());
        assert!(msg1.is_ok());
        let msg1 = msg1.unwrap();
        assert_eq!(msg1.text.as_ref().unwrap(), "first message");
        assert_eq!(msg1.from_id, DC_CONTACT_ID_DEVICE);
        assert_eq!(msg1.to_id, DC_CONTACT_ID_SELF);
        assert!(!msg1.is_info());
        assert!(!msg1.is_setupmessage());

        let msg2 = message::Message::load_from_db(&t.ctx, msg2_id.unwrap());
        assert!(msg2.is_ok());
        let msg2 = msg2.unwrap();
        assert_eq!(msg2.text.as_ref().unwrap(), "second message");

        // check device chat
        assert_eq!(get_msg_cnt(&t.ctx, msg2.chat_id), 2);
    }

    #[test]
    fn test_add_device_msg_labelled() {
        let t = test_context(Some(Box::new(logging_cb)));

        // add two device-messages with the same label (second attempt is not added)
        let mut msg1 = Message::new(Viewtype::Text);
        msg1.text = Some("first message".to_string());
        let msg1_id = add_device_msg(&t.ctx, Some("any-label"), Some(&mut msg1));
        assert!(msg1_id.is_ok());
        assert!(!msg1_id.as_ref().unwrap().is_unset());

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.text = Some("second message".to_string());
        let msg2_id = add_device_msg(&t.ctx, Some("any-label"), Some(&mut msg2));
        assert!(msg2_id.is_ok());
        assert!(msg2_id.as_ref().unwrap().is_unset());

        // check added message
        let msg1 = message::Message::load_from_db(&t.ctx, *msg1_id.as_ref().unwrap());
        assert!(msg1.is_ok());
        let msg1 = msg1.unwrap();
        assert_eq!(msg1_id.as_ref().unwrap(), &msg1.id);
        assert_eq!(msg1.text.as_ref().unwrap(), "first message");
        assert_eq!(msg1.from_id, DC_CONTACT_ID_DEVICE);
        assert_eq!(msg1.to_id, DC_CONTACT_ID_SELF);
        assert!(!msg1.is_info());
        assert!(!msg1.is_setupmessage());

        // check device chat
        let chat_id = msg1.chat_id;
        assert_eq!(get_msg_cnt(&t.ctx, chat_id), 1);
        assert!(chat_id > DC_CHAT_ID_LAST_SPECIAL);
        let chat = Chat::load_from_db(&t.ctx, chat_id);
        assert!(chat.is_ok());
        let chat = chat.unwrap();
        assert_eq!(chat.get_type(), Chattype::Single);
        assert!(chat.is_device_talk());
        assert!(!chat.is_self_talk());
        assert!(!chat.can_send());
        assert_eq!(chat.name, t.ctx.stock_str(StockMessage::DeviceMessages));
        assert!(chat.get_profile_image(&t.ctx).is_some());

        // delete device message, make sure it is not added again
        message::delete_msgs(&t.ctx, &[*msg1_id.as_ref().unwrap()]);
        let msg1 = message::Message::load_from_db(&t.ctx, *msg1_id.as_ref().unwrap());
        assert!(msg1.is_err() || msg1.unwrap().chat_id == DC_CHAT_ID_TRASH);
        let msg3_id = add_device_msg(&t.ctx, Some("any-label"), Some(&mut msg2));
        assert!(msg3_id.is_ok());
        assert!(msg2_id.as_ref().unwrap().is_unset());
    }

    #[test]
    fn test_add_device_msg_label_only() {
        let t = test_context(Some(Box::new(logging_cb)));
        let res = add_device_msg(&t.ctx, Some(""), None);
        assert!(res.is_err());
        let res = add_device_msg(&t.ctx, Some("some-label"), None);
        assert!(res.is_ok());

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());

        let msg_id = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg));
        assert!(msg_id.is_ok());
        assert!(msg_id.as_ref().unwrap().is_unset());

        let msg_id = add_device_msg(&t.ctx, Some("unused-label"), Some(&mut msg));
        assert!(msg_id.is_ok());
        assert!(!msg_id.as_ref().unwrap().is_unset());
    }

    #[test]
    fn test_was_device_msg_ever_added() {
        let t = test_context(Some(Box::new(logging_cb)));
        add_device_msg(&t.ctx, Some("some-label"), None).ok();
        assert!(was_device_msg_ever_added(&t.ctx, "some-label").unwrap());

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        add_device_msg(&t.ctx, Some("another-label"), Some(&mut msg)).ok();
        assert!(was_device_msg_ever_added(&t.ctx, "another-label").unwrap());

        assert!(!was_device_msg_ever_added(&t.ctx, "unused-label").unwrap());

        assert!(was_device_msg_ever_added(&t.ctx, "").is_err());
    }

    #[test]
    fn test_delete_device_chat() {
        let t = test_context(Some(Box::new(logging_cb)));

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg)).ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 1);

        // after the device-chat and all messages are deleted, a re-adding should do nothing
        delete(&t.ctx, chats.get_chat_id(0)).ok();
        add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg)).ok();
        assert_eq!(chatlist_len(&t.ctx, 0), 0)
    }

    #[test]
    fn test_device_chat_cannot_sent() {
        let t = test_context(Some(Box::new(logging_cb)));
        t.ctx.update_device_chats().unwrap();
        let (device_chat_id, _) =
            create_or_lookup_by_contact_id(&t.ctx, DC_CONTACT_ID_DEVICE, Blocked::Not).unwrap();

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        assert!(send_msg(&t.ctx, device_chat_id, &mut msg).is_err());
        assert!(prepare_msg(&t.ctx, device_chat_id, &mut msg).is_err());

        let msg_id = add_device_msg(&t.ctx, None, Some(&mut msg)).unwrap();
        assert!(forward_msgs(&t.ctx, &[msg_id], device_chat_id).is_err());
    }

    #[test]
    fn test_delete_and_reset_all_device_msgs() {
        let t = test_context(Some(Box::new(logging_cb)));
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        let msg_id1 = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg)).unwrap();

        // adding a device message with the same label won't be executed again ...
        assert!(was_device_msg_ever_added(&t.ctx, "some-label").unwrap());
        let msg_id2 = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg)).unwrap();
        assert!(msg_id2.is_unset());

        // ... unless everything is deleted and resetted - as needed eg. on device switch
        delete_and_reset_all_device_msgs(&t.ctx).unwrap();
        assert!(!was_device_msg_ever_added(&t.ctx, "some-label").unwrap());
        let msg_id3 = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg)).unwrap();
        assert_ne!(msg_id1, msg_id3);
    }

    fn chatlist_len(ctx: &Context, listflags: usize) -> usize {
        Chatlist::try_load(ctx, listflags, None, None)
            .unwrap()
            .len()
    }

    #[test]
    fn test_archive() {
        // create two chats
        let t = dummy_context();
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("foo".to_string());
        let msg_id = add_device_msg(&t.ctx, None, Some(&mut msg)).unwrap();
        let chat_id1 = message::Message::load_from_db(&t.ctx, msg_id)
            .unwrap()
            .chat_id;
        let chat_id2 = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF).unwrap();
        assert!(chat_id1 > DC_CHAT_ID_LAST_SPECIAL);
        assert!(chat_id2 > DC_CHAT_ID_LAST_SPECIAL);
        assert_eq!(get_chat_cnt(&t.ctx), 2);
        assert_eq!(chatlist_len(&t.ctx, 0), 2);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS), 2);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY), 0);
        assert_eq!(DC_GCL_ARCHIVED_ONLY, 0x01);
        assert_eq!(DC_GCL_NO_SPECIALS, 0x02);

        // archive first chat
        assert!(archive(&t.ctx, chat_id1, true).is_ok());
        assert!(Chat::load_from_db(&t.ctx, chat_id1).unwrap().is_archived());
        assert!(!Chat::load_from_db(&t.ctx, chat_id2).unwrap().is_archived());
        assert_eq!(get_chat_cnt(&t.ctx), 2);
        assert_eq!(chatlist_len(&t.ctx, 0), 2); // including DC_CHAT_ID_ARCHIVED_LINK now
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS), 1);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY), 1);

        // archive second chat
        assert!(archive(&t.ctx, chat_id2, true).is_ok());
        assert!(Chat::load_from_db(&t.ctx, chat_id1).unwrap().is_archived());
        assert!(Chat::load_from_db(&t.ctx, chat_id2).unwrap().is_archived());
        assert_eq!(get_chat_cnt(&t.ctx), 2);
        assert_eq!(chatlist_len(&t.ctx, 0), 1); // only DC_CHAT_ID_ARCHIVED_LINK now
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS), 0);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY), 2);

        // archive already archived first chat, unarchive second chat two times
        assert!(archive(&t.ctx, chat_id1, true).is_ok());
        assert!(archive(&t.ctx, chat_id2, false).is_ok());
        assert!(archive(&t.ctx, chat_id2, false).is_ok());
        assert!(Chat::load_from_db(&t.ctx, chat_id1).unwrap().is_archived());
        assert!(!Chat::load_from_db(&t.ctx, chat_id2).unwrap().is_archived());
        assert_eq!(get_chat_cnt(&t.ctx), 2);
        assert_eq!(chatlist_len(&t.ctx, 0), 2);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS), 1);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY), 1);
    }

    #[test]
    fn test_set_chat_name() {
        let t = dummy_context();
        let chat_id = create_group_chat(&t.ctx, VerifiedStatus::Unverified, "foo").unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id).unwrap().get_name(),
            "foo"
        );

        set_chat_name(&t.ctx, chat_id, "bar").unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id).unwrap().get_name(),
            "bar"
        );
    }

    #[test]
    fn test_create_same_chat_twice() {
        let context = dummy_context();
        let contact1 = Contact::create(&context.ctx, "bob", "bob@mail.de").unwrap();
        assert_ne!(contact1, 0);

        let chat_id = create_by_contact_id(&context.ctx, contact1).unwrap();
        assert!(
            chat_id > DC_CHAT_ID_LAST_SPECIAL,
            "chat_id too small {}",
            chat_id
        );
        let chat = Chat::load_from_db(&context.ctx, chat_id).unwrap();

        let chat2_id = create_by_contact_id(&context.ctx, contact1).unwrap();
        assert_eq!(chat2_id, chat_id);
        let chat2 = Chat::load_from_db(&context.ctx, chat2_id).unwrap();

        assert_eq!(chat2.name, chat.name);
    }

    #[test]
    fn test_shall_attach_selfavatar() {
        let t = dummy_context();
        let chat_id = create_group_chat(&t.ctx, VerifiedStatus::Unverified, "foo").unwrap();
        assert!(!shall_attach_selfavatar(&t.ctx, chat_id).unwrap());

        let (contact_id, _) =
            Contact::add_or_lookup(&t.ctx, "", "foo@bar.org", Origin::IncomingUnknownTo).unwrap();
        add_contact_to_chat(&t.ctx, chat_id, contact_id);
        assert!(!shall_attach_selfavatar(&t.ctx, chat_id).unwrap());
        t.ctx.set_config(Config::Selfavatar, None).unwrap(); // setting to None also forces re-sending
        assert!(shall_attach_selfavatar(&t.ctx, chat_id).unwrap());

        assert!(set_selfavatar_timestamp(&t.ctx, chat_id, time()).is_ok());
        assert!(!shall_attach_selfavatar(&t.ctx, chat_id).unwrap());
    }
}

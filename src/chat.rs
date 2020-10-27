//! # Chat module

use deltachat_derive::{FromSql, ToSql};
use std::convert::TryFrom;
use std::time::{Duration, SystemTime};

use anyhow::Context as _;
use async_std::path::{Path, PathBuf};
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
use crate::ephemeral::{delete_expired_messages, schedule_ephemeral_task, Timer as EphemeralTimer};
use crate::error::{bail, ensure, format_err, Error};
use crate::events::EventType;
use crate::job::{self, Action};
use crate::message::{self, InvalidMsgId, Message, MessageState, MsgId};
use crate::mimeparser::SystemMessage;
use crate::param::*;
use crate::sql;
use crate::stock::StockMessage;

/// An chat item, such as a message or a marker.
#[derive(Debug, Copy, Clone)]
pub enum ChatItem {
    Message {
        msg_id: MsgId,
    },

    /// A marker without inherent meaning. It is inserted before user
    /// supplied MsgId.
    Marker1,

    /// Day marker, separating messages that correspond to different
    /// days according to local time.
    DayMarker {
        /// Marker timestamp, for day markers
        timestamp: i64,
    },
}

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
    IntoStaticStr,
    Serialize,
    Deserialize,
)]
#[repr(u32)]
pub enum ProtectionStatus {
    Unprotected = 0,
    Protected = 1,
}

impl Default for ProtectionStatus {
    fn default() -> Self {
        ProtectionStatus::Unprotected
    }
}

/// Chat ID, including reserved IDs.
///
/// Some chat IDs are reserved to identify special chat types.  This
/// type can represent both the special as well as normal chats.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord,
)]
pub struct ChatId(u32);

impl ChatId {
    /// Create a new [ChatId].
    pub fn new(id: u32) -> ChatId {
        ChatId(id)
    }

    /// An unset ChatId
    ///
    /// This is transitional and should not be used in new code.
    pub fn is_unset(self) -> bool {
        self.0 == 0
    }

    /// Whether the chat ID signifies a special chat.
    ///
    /// This kind of chat ID can not be used for real chats.
    pub fn is_special(self) -> bool {
        matches!(self.0, 0..=DC_CHAT_ID_LAST_SPECIAL)
    }

    /// Chat ID which represents the deaddrop chat.
    ///
    /// This is a virtual chat showing all messages belonging to chats
    /// flagged with [Blocked::Deaddrop].  Usually the UI will show
    /// these messages as contact requests.
    pub fn is_deaddrop(self) -> bool {
        self.0 == DC_CHAT_ID_DEADDROP
    }

    /// Chat ID for messages which need to be deleted.
    ///
    /// Messages which should be deleted get this chat ID and are
    /// deleted later.  Deleted messages need to stay around as long
    /// as they are not deleted on the server so that their rfc724_mid
    /// remains known and downloading them again can be avoided.
    pub fn is_trash(self) -> bool {
        self.0 == DC_CHAT_ID_TRASH
    }

    /// Chat ID signifying there are **any** number of archived chats.
    ///
    /// This chat ID can be returned in a [Chatlist] and signals to
    /// the UI to include a link to the archived chats.
    pub fn is_archived_link(self) -> bool {
        self.0 == DC_CHAT_ID_ARCHIVED_LINK
    }

    /// Virtual chat ID signalling there are **only** archived chats.
    ///
    /// This can be included in the chatlist if the
    /// [DC_GCL_ADD_ALLDONE_HINT] flag is used to build the
    /// [Chatlist].
    pub fn is_alldone_hint(self) -> bool {
        self.0 == DC_CHAT_ID_ALLDONE_HINT
    }

    pub async fn set_selfavatar_timestamp(
        self,
        context: &Context,
        timestamp: i64,
    ) -> Result<(), Error> {
        context
            .sql
            .execute(
                "UPDATE contacts
                SET selfavatar_sent=?
              WHERE id IN(SELECT contact_id FROM chats_contacts WHERE chat_id=?);",
                paramsv![timestamp, self],
            )
            .await?;
        Ok(())
    }

    pub async fn set_blocked(self, context: &Context, new_blocked: Blocked) -> bool {
        if self.is_special() {
            warn!(context, "ignoring setting of Block-status for {}", self);
            return false;
        }
        context
            .sql
            .execute(
                "UPDATE chats SET blocked=? WHERE id=?;",
                paramsv![new_blocked, self],
            )
            .await
            .is_ok()
    }

    pub async fn unblock(self, context: &Context) {
        self.set_blocked(context, Blocked::Not).await;
    }

    /// Sets protection without sending a message.
    ///
    /// Used when a message arrives indicating that someone else has
    /// changed the protection value for a chat.
    pub(crate) async fn inner_set_protection(
        self,
        context: &Context,
        protect: ProtectionStatus,
    ) -> Result<(), Error> {
        ensure!(!self.is_special(), "Invalid chat-id.");

        let chat = Chat::load_from_db(context, self).await?;

        if protect == chat.protected {
            info!(context, "Protection status unchanged for {}.", self);
            return Ok(());
        }

        match protect {
            ProtectionStatus::Protected => match chat.typ {
                Chattype::Single | Chattype::Group => {
                    let contact_ids = get_chat_contacts(context, self).await;
                    for contact_id in contact_ids.into_iter() {
                        let contact = Contact::get_by_id(context, contact_id).await?;
                        if contact.is_verified(context).await != VerifiedStatus::BidirectVerified {
                            bail!("{} is not verified.", contact.get_display_name());
                        }
                    }
                }
                Chattype::Undefined => bail!("Undefined group type"),
            },
            ProtectionStatus::Unprotected => {}
        };

        context
            .sql
            .execute(
                "UPDATE chats SET protected=? WHERE id=?;",
                paramsv![protect, self],
            )
            .await?;

        context.emit_event(EventType::ChatModified(self));

        // make sure, the receivers will get all keys
        reset_gossiped_timestamp(context, self).await?;

        Ok(())
    }

    /// Send protected status message to the chat.
    ///
    /// This sends the message with the protected status change to the chat,
    /// notifying the user on this device as well as the other users in the chat.
    ///
    /// If `promote` is false this means, the message must not be sent out
    /// and only a local info message should be added to the chat.
    /// This is used when protection is enabled implicitly or when a chat is not yet promoted.
    pub(crate) async fn add_protection_msg(
        self,
        context: &Context,
        protect: ProtectionStatus,
        promote: bool,
        from_id: u32,
    ) -> Result<(), Error> {
        let msg_text = context.stock_protection_msg(protect, from_id).await;
        let cmd = match protect {
            ProtectionStatus::Protected => SystemMessage::ChatProtectionEnabled,
            ProtectionStatus::Unprotected => SystemMessage::ChatProtectionDisabled,
        };

        if promote {
            let mut msg = Message::default();
            msg.viewtype = Viewtype::Text;
            msg.text = Some(msg_text);
            msg.param.set_cmd(cmd);
            send_msg(context, self, &mut msg).await?;
        } else {
            add_info_msg_with_cmd(context, self, msg_text, cmd).await?;
        }

        Ok(())
    }

    /// Sets protection and sends or adds a message.
    pub async fn set_protection(
        self,
        context: &Context,
        protect: ProtectionStatus,
    ) -> Result<(), Error> {
        ensure!(!self.is_special(), "set protection: invalid chat-id.");

        let chat = Chat::load_from_db(context, self).await?;

        if let Err(e) = self.inner_set_protection(context, protect).await {
            error!(context, "Cannot set protection: {}", e); // make error user-visible
            return Err(e);
        }

        self.add_protection_msg(context, protect, chat.is_promoted(), DC_CONTACT_ID_SELF)
            .await
    }

    /// Archives or unarchives a chat.
    pub async fn set_visibility(
        self,
        context: &Context,
        visibility: ChatVisibility,
    ) -> Result<(), Error> {
        ensure!(
            !self.is_special(),
            "bad chat_id, can not be special chat: {}",
            self
        );

        if visibility == ChatVisibility::Archived {
            context
                .sql
                .execute(
                    "UPDATE msgs SET state=? WHERE chat_id=? AND state=?;",
                    paramsv![MessageState::InNoticed, self, MessageState::InFresh],
                )
                .await?;
        }

        context
            .sql
            .execute(
                "UPDATE chats SET archived=? WHERE id=?;",
                paramsv![visibility, self],
            )
            .await?;

        context.emit_event(EventType::MsgsChanged {
            msg_id: MsgId::new(0),
            chat_id: ChatId::new(0),
        });

        Ok(())
    }

    // note that unarchive() is not the same as set_visibility(Normal) -
    // eg. unarchive() does not modify pinned chats and does not send events.
    pub async fn unarchive(self, context: &Context) -> Result<(), Error> {
        context
            .sql
            .execute(
                "UPDATE chats SET archived=0 WHERE id=? and archived=1",
                paramsv![self],
            )
            .await?;
        Ok(())
    }

    /// Deletes a chat.
    pub async fn delete(self, context: &Context) -> Result<(), Error> {
        ensure!(
            !self.is_special(),
            "bad chat_id, can not be a special chat: {}",
            self
        );
        /* Up to 2017-11-02 deleting a group also implied leaving it, see above why we have changed this. */

        let chat = Chat::load_from_db(context, self).await?;
        context
            .sql
            .execute(
                "DELETE FROM msgs_mdns WHERE msg_id IN (SELECT id FROM msgs WHERE chat_id=?);",
                paramsv![self],
            )
            .await?;

        context
            .sql
            .execute("DELETE FROM msgs WHERE chat_id=?;", paramsv![self])
            .await?;

        context
            .sql
            .execute(
                "DELETE FROM chats_contacts WHERE chat_id=?;",
                paramsv![self],
            )
            .await?;

        context
            .sql
            .execute("DELETE FROM chats WHERE id=?;", paramsv![self])
            .await?;

        context.emit_event(EventType::MsgsChanged {
            msg_id: MsgId::new(0),
            chat_id: ChatId::new(0),
        });

        job::kill_action(context, Action::Housekeeping).await;
        let j = job::Job::new(Action::Housekeeping, 0, Params::new(), 10);
        job::add(context, j).await;

        if chat.is_self_talk() {
            let mut msg = Message::new(Viewtype::Text);
            msg.text = Some(
                context
                    .stock_str(StockMessage::SelfDeletedMsgBody)
                    .await
                    .into(),
            );
            add_device_msg(&context, None, Some(&mut msg)).await?;
        }

        Ok(())
    }

    /// Sets draft message.
    ///
    /// Passing `None` as message just deletes the draft
    pub async fn set_draft(self, context: &Context, msg: Option<&mut Message>) {
        if self.is_special() {
            return;
        }

        let changed = match msg {
            None => self.maybe_delete_draft(context).await,
            Some(msg) => self.set_draft_raw(context, msg).await,
        };

        if changed {
            context.emit_event(EventType::MsgsChanged {
                chat_id: self,
                msg_id: MsgId::new(0),
            });
        }
    }

    // similar to as dc_set_draft() but does not emit an event
    async fn set_draft_raw(self, context: &Context, msg: &mut Message) -> bool {
        let deleted = self.maybe_delete_draft(context).await;
        let set = self.do_set_draft(context, msg).await.is_ok();

        // Can't inline. Both functions above must be called, no shortcut!
        deleted || set
    }

    async fn get_draft_msg_id(self, context: &Context) -> Option<MsgId> {
        context
            .sql
            .query_get_value::<MsgId>(
                context,
                "SELECT id FROM msgs WHERE chat_id=? AND state=?;",
                paramsv![self, MessageState::OutDraft],
            )
            .await
    }

    pub async fn get_draft(self, context: &Context) -> Result<Option<Message>, Error> {
        if self.is_special() {
            return Ok(None);
        }
        match self.get_draft_msg_id(context).await {
            Some(draft_msg_id) => {
                let msg = Message::load_from_db(context, draft_msg_id).await?;
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }

    /// Delete draft message in specified chat, if there is one.
    ///
    /// Returns `true`, if message was deleted, `false` otherwise.
    async fn maybe_delete_draft(self, context: &Context) -> bool {
        match self.get_draft_msg_id(context).await {
            Some(msg_id) => msg_id.delete_from_db(context).await.is_ok(),
            None => false,
        }
    }

    /// Set provided message as draft message for specified chat.
    ///
    /// Return true on success, false on database error.
    async fn do_set_draft(self, context: &Context, msg: &mut Message) -> Result<(), Error> {
        match msg.viewtype {
            Viewtype::Unknown => bail!("Can not set draft of unknown type."),
            Viewtype::Text => {
                if msg.text.is_none_or_empty() && msg.in_reply_to.is_none_or_empty() {
                    bail!("No text and no quote in draft");
                }
            }
            _ => {
                let blob = msg
                    .param
                    .get_blob(Param::File, context, !msg.is_increation())
                    .await?
                    .ok_or_else(|| format_err!("No file stored in params"))?;
                msg.param.set(Param::File, blob.as_name());
            }
        }

        let chat = Chat::load_from_db(context, self).await?;
        if !chat.can_send() {
            bail!("Can't set a draft: Can't send");
        }

        context
            .sql
            .execute(
                "INSERT INTO msgs (chat_id, from_id, timestamp, type, state, txt, param, hidden, mime_in_reply_to)
         VALUES (?,?,?, ?,?,?,?,?,?);",
                paramsv![
                    self,
                    DC_CONTACT_ID_SELF,
                    time(),
                    msg.viewtype,
                    MessageState::OutDraft,
                    msg.text.as_deref().unwrap_or(""),
                    msg.param.to_string(),
                    1,
                    msg.in_reply_to.as_deref().unwrap_or_default(),
                ],
            )
            .await?;
        Ok(())
    }

    /// Returns number of messages in a chat.
    pub async fn get_msg_cnt(self, context: &Context) -> usize {
        context
            .sql
            .query_get_value::<i32>(
                context,
                "SELECT COUNT(*) FROM msgs WHERE chat_id=?;",
                paramsv![self],
            )
            .await
            .unwrap_or_default() as usize
    }

    pub async fn get_fresh_msg_cnt(self, context: &Context) -> usize {
        // this function is typically used to show a badge counter beside _each_ chatlist item.
        // to make this as fast as possible, esp. on older devices, we added an combined index over the rows used for querying.
        // so if you alter the query here, you may want to alter the index over `(state, hidden, chat_id)` in `sql.rs`.
        //
        // the impact of the index is significant once the database grows:
        // - on an older android4 with 18k messages, query-time decreased from 110ms to 2ms
        // - on an mid-class moto-g or iphone7 with 50k messages, query-time decreased from 26ms or 6ms to 0-1ms
        // the times are average, no matter if there are fresh messages or not -
        // and have to be multiplied by the number of items shown at once on the chatlist,
        // so savings up to 2 seconds are possible on older devices - newer ones will feel "snappier" :)
        context
            .sql
            .query_get_value::<i32>(
                context,
                "SELECT COUNT(*)
                FROM msgs
                WHERE state=10
                AND hidden=0
                AND chat_id=?;",
                paramsv![self],
            )
            .await
            .unwrap_or_default() as usize
    }

    pub(crate) async fn get_param(self, context: &Context) -> Result<Params, Error> {
        let res: Option<String> = context
            .sql
            .query_get_value_result("SELECT param FROM chats WHERE id=?", paramsv![self])
            .await?;
        Ok(res
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default())
    }

    // Returns true if chat is a saved messages chat.
    pub async fn is_self_talk(self, context: &Context) -> Result<bool, Error> {
        Ok(self.get_param(context).await?.exists(Param::Selftalk))
    }

    /// Returns true if chat is a device chat.
    pub async fn is_device_talk(self, context: &Context) -> Result<bool, Error> {
        Ok(self.get_param(context).await?.exists(Param::Devicetalk))
    }

    async fn parent_query<T, F>(
        self,
        context: &Context,
        fields: &str,
        f: F,
    ) -> sql::Result<Option<T>>
    where
        F: FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let sql = &context.sql;
        let query = format!(
            "SELECT {} \
             FROM msgs WHERE chat_id=? AND state NOT IN (?, ?, ?, ?) AND NOT hidden \
             ORDER BY timestamp DESC, id DESC \
             LIMIT 1;",
            fields
        );
        sql.query_row_optional(
            query,
            paramsv![
                self,
                MessageState::OutPreparing,
                MessageState::OutDraft,
                MessageState::OutPending,
                MessageState::OutFailed
            ],
            f,
        )
        .await
    }

    async fn get_parent_mime_headers(self, context: &Context) -> Option<(String, String, String)> {
        let collect =
            |row: &rusqlite::Row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
        let (rfc724_mid, mime_in_reply_to, mime_references, error): (
            String,
            String,
            String,
            String,
        ) = self
            .parent_query(
                context,
                "rfc724_mid, mime_in_reply_to, mime_references, error",
                collect,
            )
            .await
            .ok()
            .flatten()?;

        if !error.is_empty() {
            // Do not reply to error messages.
            //
            // An error message could be a group chat message that we failed to decrypt and
            // assigned to 1:1 chat. A reply to it will show up as a reply to group message
            // on the other side. To avoid such situations, it is better not to reply to
            // error messages at all.
            None
        } else {
            Some((rfc724_mid, mime_in_reply_to, mime_references))
        }
    }

    /// Bad evil escape hatch.
    ///
    /// Avoid using this, eventually types should be cleaned up enough
    /// that it is no longer necessary.
    pub fn to_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ChatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_deaddrop() {
            write!(f, "Chat#Deadrop")
        } else if self.is_trash() {
            write!(f, "Chat#Trash")
        } else if self.is_archived_link() {
            write!(f, "Chat#ArchivedLink")
        } else if self.is_alldone_hint() {
            write!(f, "Chat#AlldoneHint")
        } else if self.is_special() {
            write!(f, "Chat#Special{}", self.0)
        } else {
            write!(f, "Chat#{}", self.0)
        }
    }
}

/// Allow converting [ChatId] to an SQLite type.
///
/// This allows you to directly store [ChatId] into the database as
/// well as query for a [ChatId].
impl rusqlite::types::ToSql for ChatId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let val = rusqlite::types::Value::Integer(self.0 as i64);
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

/// Allow converting an SQLite integer directly into [ChatId].
impl rusqlite::types::FromSql for ChatId {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).and_then(|val| {
            if 0 <= val && val <= std::u32::MAX as i64 {
                Ok(ChatId::new(val as u32))
            } else {
                Err(rusqlite::types::FromSqlError::OutOfRange(val))
            }
        })
    }
}

/// An object representing a single chat in memory.
/// Chat objects are created using eg. `Chat::load_from_db`
/// and are not updated on database changes;
/// if you want an update, you have to recreate the object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chat {
    pub id: ChatId,
    pub typ: Chattype,
    pub name: String,
    pub visibility: ChatVisibility,
    pub grpid: String,
    blocked: Blocked,
    pub param: Params,
    is_sending_locations: bool,
    pub mute_duration: MuteDuration,
    protected: ProtectionStatus,
}

impl Chat {
    /// Loads chat from the database by its ID.
    pub async fn load_from_db(context: &Context, chat_id: ChatId) -> Result<Self, Error> {
        let res = context
            .sql
            .query_row(
                "SELECT c.type, c.name, c.grpid, c.param, c.archived,
                    c.blocked, c.locations_send_until, c.muted_until, c.protected
             FROM chats c
             WHERE c.id=?;",
                paramsv![chat_id],
                |row| {
                    let c = Chat {
                        id: chat_id,
                        typ: row.get(0)?,
                        name: row.get::<_, String>(1)?,
                        grpid: row.get::<_, String>(2)?,
                        param: row.get::<_, String>(3)?.parse().unwrap_or_default(),
                        visibility: row.get(4)?,
                        blocked: row.get::<_, Option<_>>(5)?.unwrap_or_default(),
                        is_sending_locations: row.get(6)?,
                        mute_duration: row.get(7)?,
                        protected: row.get(8)?,
                    };
                    Ok(c)
                },
            )
            .await;

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
                if chat.id.is_deaddrop() {
                    chat.name = context.stock_str(StockMessage::DeadDrop).await.into();
                } else if chat.id.is_archived_link() {
                    let tempname = context.stock_str(StockMessage::ArchivedChats).await;
                    let cnt = dc_get_archived_cnt(context).await;
                    chat.name = format!("{} ({})", tempname, cnt);
                } else {
                    if chat.typ == Chattype::Single {
                        let contacts = get_chat_contacts(context, chat.id).await;
                        let mut chat_name = "Err [Name not found]".to_owned();
                        if let Some(contact_id) = contacts.first() {
                            if let Ok(contact) = Contact::get_by_id(context, *contact_id).await {
                                chat_name = contact.get_display_name().to_owned();
                            }
                        }
                        chat.name = chat_name;
                    }
                    if chat.param.exists(Param::Selftalk) {
                        chat.name = context.stock_str(StockMessage::SavedMessages).await.into();
                    } else if chat.param.exists(Param::Devicetalk) {
                        chat.name = context.stock_str(StockMessage::DeviceMessages).await.into();
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
        !self.id.is_special() && !self.is_device_talk()
    }

    pub async fn update_param(&mut self, context: &Context) -> Result<(), Error> {
        context
            .sql
            .execute(
                "UPDATE chats SET param=? WHERE id=?",
                paramsv![self.param.to_string(), self.id],
            )
            .await?;
        Ok(())
    }

    /// Returns chat ID.
    pub fn get_id(&self) -> ChatId {
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

    pub async fn get_profile_image(&self, context: &Context) -> Option<PathBuf> {
        if let Some(image_rel) = self.param.get(Param::ProfileImage) {
            if !image_rel.is_empty() {
                return Some(dc_get_abs_path(context, image_rel));
            }
        } else if self.typ == Chattype::Single {
            let contacts = get_chat_contacts(context, self.id).await;
            if let Some(contact_id) = contacts.first() {
                if let Ok(contact) = Contact::get_by_id(context, *contact_id).await {
                    return contact.get_profile_image(context).await;
                }
            }
        }

        None
    }

    pub async fn get_gossiped_timestamp(&self, context: &Context) -> i64 {
        get_gossiped_timestamp(context, self.id).await
    }

    pub async fn get_color(&self, context: &Context) -> u32 {
        let mut color = 0;

        if self.typ == Chattype::Single {
            let contacts = get_chat_contacts(context, self.id).await;
            if let Some(contact_id) = contacts.first() {
                if let Ok(contact) = Contact::get_by_id(context, *contact_id).await {
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
    pub async fn get_info(&self, context: &Context) -> Result<ChatInfo, Error> {
        let draft = match self.id.get_draft(context).await? {
            Some(message) => message.text.unwrap_or_else(String::new),
            _ => String::new(),
        };
        Ok(ChatInfo {
            id: self.id,
            type_: self.typ as u32,
            name: self.name.clone(),
            archived: self.visibility == ChatVisibility::Archived,
            param: self.param.to_string(),
            gossiped_timestamp: self.get_gossiped_timestamp(context).await,
            is_sending_locations: self.is_sending_locations,
            color: self.get_color(context).await,
            profile_image: self
                .get_profile_image(context)
                .await
                .map(Into::into)
                .unwrap_or_else(std::path::PathBuf::new),
            draft,
            is_muted: self.is_muted(),
            ephemeral_timer: self.id.get_ephemeral_timer(context).await?,
        })
    }

    pub fn get_visibility(&self) -> ChatVisibility {
        self.visibility
    }

    pub fn is_unpromoted(&self) -> bool {
        self.param.get_int(Param::Unpromoted).unwrap_or_default() == 1
    }

    pub fn is_promoted(&self) -> bool {
        !self.is_unpromoted()
    }

    /// Returns true if chat protection is enabled.
    pub fn is_protected(&self) -> bool {
        self.protected == ProtectionStatus::Protected
    }

    /// Returns true if location streaming is enabled in the chat.
    pub fn is_sending_locations(&self) -> bool {
        self.is_sending_locations
    }

    pub fn is_muted(&self) -> bool {
        match self.mute_duration {
            MuteDuration::NotMuted => false,
            MuteDuration::Forever => true,
            MuteDuration::Until(when) => when > SystemTime::now(),
        }
    }

    async fn prepare_msg_raw(
        &mut self,
        context: &Context,
        msg: &mut Message,
        timestamp: i64,
    ) -> Result<MsgId, Error> {
        let mut new_references = "".into();
        let mut msg_id = 0;
        let mut to_id = 0;
        let mut location_id = 0;

        if !(self.typ == Chattype::Single || self.typ == Chattype::Group) {
            error!(context, "Cannot send to chat type #{}.", self.typ,);
            bail!("Cannot set to chat type #{}", self.typ);
        }

        if self.typ == Chattype::Group
            && !is_contact_in_chat(context, self.id, DC_CONTACT_ID_SELF).await
        {
            emit_event!(
                context,
                EventType::ErrorSelfNotInGroup("Cannot send message; self not in group.".into())
            );
            bail!("Cannot set message; self not in group.");
        }

        let from = context
            .get_config(Config::ConfiguredAddr)
            .await
            .context("Cannot prepare message for sending, address is not configured.")?;

        let new_rfc724_mid = {
            let grpid = match self.typ {
                Chattype::Group => Some(self.grpid.as_str()),
                _ => None,
            };
            dc_create_outgoing_rfc724_mid(grpid, &from)
        };

        if self.typ == Chattype::Single {
            if let Some(id) = context
                .sql
                .query_get_value(
                    context,
                    "SELECT contact_id FROM chats_contacts WHERE chat_id=?;",
                    paramsv![self.id],
                )
                .await
            {
                to_id = id;
            } else {
                error!(
                    context,
                    "Cannot send message, contact for {} not found.", self.id,
                );
                bail!("Cannot set message, contact for {} not found.", self.id);
            }
        } else if self.typ == Chattype::Group
            && self.param.get_int(Param::Unpromoted).unwrap_or_default() == 1
        {
            msg.param.set_int(Param::AttachGroupImage, 1);
            self.param.remove(Param::Unpromoted);
            self.update_param(context).await?;
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
                self.id.get_parent_mime_headers(context).await
            {
                // "In-Reply-To:" is not changed if it is set manually.
                // This does not affect "References:" header, it will contain "default parent" (the
                // latest message in the thread) anyway.
                if msg.in_reply_to.is_none() && !parent_rfc724_mid.is_empty() {
                    msg.in_reply_to = Some(parent_rfc724_mid.clone());
                }

                // the whole list of messages referenced may be huge;
                // only use the oldest and and the parent message
                let parent_references = parent_references
                    .find(' ')
                    .and_then(|n| parent_references.get(..n))
                    .unwrap_or(&parent_references);

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
            && context
                .sql
                .execute(
                    "INSERT INTO locations \
                     (timestamp,from_id,chat_id, latitude,longitude,independent)\
                     VALUES (?,?,?, ?,?,1);", // 1=DC_CONTACT_ID_SELF
                    paramsv![
                        timestamp,
                        DC_CONTACT_ID_SELF,
                        self.id,
                        msg.param.get_float(Param::SetLatitude).unwrap_or_default(),
                        msg.param.get_float(Param::SetLongitude).unwrap_or_default(),
                    ],
                )
                .await
                .is_ok()
        {
            location_id = context
                .sql
                .get_rowid2(
                    context,
                    "locations",
                    "timestamp",
                    timestamp,
                    "from_id",
                    DC_CONTACT_ID_SELF as i32,
                )
                .await?;
        }

        let ephemeral_timer = if msg.param.get_cmd() == SystemMessage::EphemeralTimerChanged {
            EphemeralTimer::Disabled
        } else {
            self.id.get_ephemeral_timer(context).await?
        };
        let ephemeral_timestamp = match ephemeral_timer {
            EphemeralTimer::Disabled => 0,
            EphemeralTimer::Enabled { duration } => timestamp + i64::from(duration),
        };

        // add message to the database

        if context
            .sql
            .execute(
                "INSERT INTO msgs (
                        rfc724_mid,
                        chat_id,
                        from_id,
                        to_id,
                        timestamp,
                        type,
                        state,
                        txt,
                        param,
                        hidden,
                        mime_in_reply_to,
                        mime_references,
                        location_id,
                        ephemeral_timer,
                        ephemeral_timestamp)
                        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?);",
                paramsv![
                    new_rfc724_mid,
                    self.id,
                    DC_CONTACT_ID_SELF,
                    to_id as i32,
                    timestamp,
                    msg.viewtype,
                    msg.state,
                    msg.text.as_ref().cloned().unwrap_or_default(),
                    msg.param.to_string(),
                    msg.hidden,
                    msg.in_reply_to.as_deref().unwrap_or_default(),
                    new_references,
                    location_id as i32,
                    ephemeral_timer,
                    ephemeral_timestamp
                ],
            )
            .await
            .is_ok()
        {
            msg_id = context
                .sql
                .get_rowid(context, "msgs", "rfc724_mid", new_rfc724_mid)
                .await?;
        } else {
            error!(
                context,
                "Cannot send message, cannot insert to database ({}).", self.id,
            );
        }
        schedule_ephemeral_task(context).await;

        Ok(MsgId::new(msg_id))
    }
}

#[derive(Debug, Copy, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ChatVisibility {
    Normal,
    Archived,
    Pinned,
}

impl rusqlite::types::ToSql for ChatVisibility {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let visibility = match &self {
            ChatVisibility::Normal => 0,
            ChatVisibility::Archived => 1,
            ChatVisibility::Pinned => 2,
        };
        let val = rusqlite::types::Value::Integer(visibility);
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

impl rusqlite::types::FromSql for ChatVisibility {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).map(|val| {
            match val {
                2 => ChatVisibility::Pinned,
                1 => ChatVisibility::Archived,
                0 => ChatVisibility::Normal,
                // fallback to to Normal for unknown values, may happen eg. on imports created by a newer version.
                _ => ChatVisibility::Normal,
            }
        })
    }
}

/// The current state of a chat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatInfo {
    /// The chat ID.
    pub id: ChatId,

    /// The type of chat as a `u32` representation of [Chattype].
    ///
    /// On the C API this number is one of the
    /// `DC_CHAT_TYPE_UNDEFINED`, `DC_CHAT_TYPE_SINGLE`,
    /// or `DC_CHAT_TYPE_GROUP`
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
    pub profile_image: std::path::PathBuf,

    /// The draft message text.
    ///
    /// If the chat has not draft this is an empty string.
    ///
    /// TODO: This doesn't seem rich enough, it can not handle drafts
    ///       which contain non-text parts.  Perhaps it should be a
    ///       simple `has_draft` bool instead.
    pub draft: String,

    /// Whether the chat is muted
    ///
    /// The exact time its muted can be found out via the `chat.mute_duration` property
    pub is_muted: bool,

    /// Ephemeral message timer.
    pub ephemeral_timer: EphemeralTimer,
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
pub async fn create_by_msg_id(context: &Context, msg_id: MsgId) -> Result<ChatId, Error> {
    let msg = Message::load_from_db(context, msg_id).await?;
    let chat = Chat::load_from_db(context, msg.chat_id).await?;
    ensure!(
        !chat.id.is_special(),
        "Message can not belong to a special chat"
    );
    if chat.blocked != Blocked::Not {
        chat.id.unblock(context).await;

        // Sending with 0s as data since multiple messages may have changed.
        context.emit_event(EventType::MsgsChanged {
            chat_id: ChatId::new(0),
            msg_id: MsgId::new(0),
        });
    }
    Contact::scaleup_origin_by_id(context, msg.from_id, Origin::CreateChat).await;
    Ok(chat.id)
}

/// Create a normal chat with a single user.  To create group chats,
/// see [Chat::create_group_chat].
///
/// If a chat already exists, this ID is returned, otherwise a new chat is created;
/// this new chat may already contain messages, eg. from the deaddrop, to get the
/// chat messages, use dc_get_chat_msgs().
pub async fn create_by_contact_id(context: &Context, contact_id: u32) -> Result<ChatId, Error> {
    let chat_id = match lookup_by_contact_id(context, contact_id).await {
        Ok((chat_id, chat_blocked)) => {
            if chat_blocked != Blocked::Not {
                // unblock chat (typically move it from the deaddrop to view
                chat_id.unblock(context).await;
            }
            chat_id
        }
        Err(err) => {
            if !Contact::real_exists_by_id(context, contact_id).await
                && contact_id != DC_CONTACT_ID_SELF
            {
                warn!(
                    context,
                    "Cannot create chat, contact {} does not exist.", contact_id,
                );
                return Err(err);
            } else {
                let (chat_id, _) =
                    create_or_lookup_by_contact_id(context, contact_id, Blocked::Not).await?;
                Contact::scaleup_origin_by_id(context, contact_id, Origin::CreateChat).await;
                chat_id
            }
        }
    };

    context.emit_event(EventType::MsgsChanged {
        chat_id: ChatId::new(0),
        msg_id: MsgId::new(0),
    });

    Ok(chat_id)
}

pub(crate) async fn update_saved_messages_icon(context: &Context) -> Result<(), Error> {
    // if there is no saved-messages chat, there is nothing to update. this is no error.
    if let Ok((chat_id, _)) = lookup_by_contact_id(context, DC_CONTACT_ID_SELF).await {
        let icon = include_bytes!("../assets/icon-saved-messages.png");
        let blob = BlobObject::create(context, "icon-saved-messages.png".to_string(), icon).await?;
        let icon = blob.as_name().to_string();

        let mut chat = Chat::load_from_db(context, chat_id).await?;
        chat.param.set(Param::ProfileImage, icon);
        chat.update_param(context).await?;
    }
    Ok(())
}

pub(crate) async fn update_device_icon(context: &Context) -> Result<(), Error> {
    // if there is no device-chat, there is nothing to update. this is no error.
    if let Ok((chat_id, _)) = lookup_by_contact_id(context, DC_CONTACT_ID_DEVICE).await {
        let icon = include_bytes!("../assets/icon-device.png");
        let blob = BlobObject::create(context, "icon-device.png".to_string(), icon).await?;
        let icon = blob.as_name().to_string();

        let mut chat = Chat::load_from_db(context, chat_id).await?;
        chat.param.set(Param::ProfileImage, &icon);
        chat.update_param(context).await?;

        let mut contact = Contact::load_from_db(context, DC_CONTACT_ID_DEVICE).await?;
        contact.param.set(Param::ProfileImage, icon);
        contact.update_param(context).await?;
    }
    Ok(())
}

async fn update_special_chat_name(
    context: &Context,
    contact_id: u32,
    stock_id: StockMessage,
) -> Result<(), Error> {
    if let Ok((chat_id, _)) = lookup_by_contact_id(context, contact_id).await {
        let name: String = context.stock_str(stock_id).await.into();
        // the `!= name` condition avoids unneeded writes
        context
            .sql
            .execute(
                "UPDATE chats SET name=? WHERE id=? AND name!=?;",
                paramsv![name, chat_id, name],
            )
            .await?;
    }
    Ok(())
}

pub(crate) async fn update_special_chat_names(context: &Context) -> Result<(), Error> {
    update_special_chat_name(context, DC_CONTACT_ID_DEVICE, StockMessage::DeviceMessages).await?;
    update_special_chat_name(context, DC_CONTACT_ID_SELF, StockMessage::SavedMessages).await?;
    Ok(())
}

pub(crate) async fn create_or_lookup_by_contact_id(
    context: &Context,
    contact_id: u32,
    create_blocked: Blocked,
) -> Result<(ChatId, Blocked), Error> {
    ensure!(context.sql.is_open().await, "Database not available");
    ensure!(contact_id > 0, "Invalid contact id requested");

    if let Ok((chat_id, chat_blocked)) = lookup_by_contact_id(context, contact_id).await {
        // Already exists, no need to create.
        return Ok((chat_id, chat_blocked));
    }

    let contact = Contact::load_from_db(context, contact_id).await?;
    let chat_name = contact.get_display_name().to_string();

    context
        .sql
        .with_conn(move |mut conn| {
            let conn2 = &mut conn;
            let tx = conn2.transaction()?;
            tx.execute(
                "INSERT INTO chats (type, name, param, blocked, created_timestamp) VALUES(?, ?, ?, ?, ?)",
                params![
                    Chattype::Single,
                    chat_name,
                    match contact_id {
                        DC_CONTACT_ID_SELF => "K=1".to_string(), // K = Param::Selftalk
                        DC_CONTACT_ID_DEVICE => "D=1".to_string(), // D = Param::Devicetalk
                        _ => "".to_string(),
                    },
                    create_blocked as u8,
                    time(),
                ],
            )?;

            tx.execute(
                "INSERT INTO chats_contacts (chat_id, contact_id) VALUES((SELECT last_insert_rowid()), ?)",
                params![contact_id],
            )?;

            tx.commit()?;
            Ok(())
        })
        .await?;

    if contact_id == DC_CONTACT_ID_SELF {
        update_saved_messages_icon(context).await?;
    } else if contact_id == DC_CONTACT_ID_DEVICE {
        update_device_icon(context).await?;
    }

    lookup_by_contact_id(context, contact_id).await
}

pub(crate) async fn lookup_by_contact_id(
    context: &Context,
    contact_id: u32,
) -> Result<(ChatId, Blocked), Error> {
    ensure!(context.sql.is_open().await, "Database not available");

    context
        .sql
        .query_row(
            "SELECT c.id, c.blocked
               FROM chats c
              INNER JOIN chats_contacts j
                      ON c.id=j.chat_id
              WHERE c.type=100
                AND c.id>9
                AND j.contact_id=?;",
            paramsv![contact_id as i32],
            |row| {
                Ok((
                    row.get::<_, ChatId>(0)?,
                    row.get::<_, Option<_>>(1)?.unwrap_or_default(),
                ))
            },
        )
        .await
        .map_err(Into::into)
}

pub async fn get_by_contact_id(context: &Context, contact_id: u32) -> Result<ChatId, Error> {
    let (chat_id, blocked) = lookup_by_contact_id(context, contact_id).await?;
    ensure_eq!(blocked, Blocked::Not, "Requested contact is blocked");

    Ok(chat_id)
}

pub async fn prepare_msg(
    context: &Context,
    chat_id: ChatId,
    msg: &mut Message,
) -> Result<MsgId, Error> {
    ensure!(
        !chat_id.is_special(),
        "Cannot prepare message for special chat"
    );

    msg.state = MessageState::OutPreparing;
    let msg_id = prepare_msg_common(context, chat_id, msg).await?;
    context.emit_event(EventType::MsgsChanged {
        chat_id: msg.chat_id,
        msg_id: msg.id,
    });

    Ok(msg_id)
}

pub(crate) fn msgtype_has_file(msgtype: Viewtype) -> bool {
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
        Viewtype::VideochatInvitation => false,
    }
}

async fn prepare_msg_blob(context: &Context, msg: &mut Message) -> Result<(), Error> {
    if msg.viewtype == Viewtype::Text || msg.viewtype == Viewtype::VideochatInvitation {
        // the caller should check if the message text is empty
    } else if msgtype_has_file(msg.viewtype) {
        let blob = msg
            .param
            .get_blob(Param::File, context, !msg.is_increation())
            .await?
            .ok_or_else(|| {
                format_err!("Attachment missing for message of type #{}", msg.viewtype)
            })?;

        if msg.viewtype == Viewtype::Image {
            if let Err(e) = blob.recode_to_image_size(context).await {
                warn!(context, "Cannot recode image, using original data: {:?}", e);
            }
        }
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
                if !msg.param.exists(Param::MimeType) {
                    msg.param.set(Param::MimeType, better_mime);
                }
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

async fn prepare_msg_common(
    context: &Context,
    chat_id: ChatId,
    msg: &mut Message,
) -> Result<MsgId, Error> {
    msg.id = MsgId::new_unset();
    prepare_msg_blob(context, msg).await?;
    chat_id.unarchive(context).await?;

    let mut chat = Chat::load_from_db(context, chat_id).await?;
    ensure!(chat.can_send(), "cannot send to {}", chat_id);

    // The OutPreparing state is set by dc_prepare_msg() before it
    // calls this function and the message is left in the OutPreparing
    // state.  Otherwise we got called by send_msg() and we change the
    // state to OutPending.
    if msg.state != MessageState::OutPreparing {
        msg.state = MessageState::OutPending;
    }

    msg.id = chat
        .prepare_msg_raw(context, msg, dc_create_smeared_timestamp(context).await)
        .await?;
    msg.chat_id = chat_id;

    Ok(msg.id)
}

/// Returns whether a contact is in a chat or not.
pub async fn is_contact_in_chat(context: &Context, chat_id: ChatId, contact_id: u32) -> bool {
    // this function works for group and for normal chats, however, it is more useful
    // for group chats.
    // DC_CONTACT_ID_SELF may be used to check, if the user itself is in a group
    // chat (DC_CONTACT_ID_SELF is not added to normal chats)

    context
        .sql
        .exists(
            "SELECT contact_id FROM chats_contacts WHERE chat_id=? AND contact_id=?;",
            paramsv![chat_id, contact_id as i32],
        )
        .await
        .unwrap_or_default()
}

/// Send a message defined by a dc_msg_t object to a chat.
///
/// Sends the event #DC_EVENT_MSGS_CHANGED on succcess.
/// However, this does not imply, the message really reached the recipient -
/// sending may be delayed eg. due to network problems. However, from your
/// view, you're done with the message. Sooner or later it will find its way.
// TODO: Do not allow ChatId to be 0, if prepare_msg had been called
//   the caller can get it from msg.chat_id.  Forwards would need to
//   be fixed for this somehow too.
pub async fn send_msg(
    context: &Context,
    chat_id: ChatId,
    msg: &mut Message,
) -> Result<MsgId, Error> {
    if chat_id.is_unset() {
        let forwards = msg.param.get(Param::PrepForwards);
        if let Some(forwards) = forwards {
            for forward in forwards.split(' ') {
                if let Ok(msg_id) = forward
                    .parse::<u32>()
                    .map_err(|_| InvalidMsgId)
                    .map(MsgId::new)
                {
                    if let Ok(mut msg) = Message::load_from_db(context, msg_id).await {
                        send_msg_inner(context, chat_id, &mut msg).await?;
                    };
                }
            }
            msg.param.remove(Param::PrepForwards);
            msg.update_param(context).await;
        }
        return send_msg_inner(context, chat_id, msg).await;
    }

    send_msg_inner(context, chat_id, msg).await
}

/// Tries to send a message synchronously.
///
/// Directly  opens an smtp
/// connection and sends the message, bypassing the job system. If this fails, it writes a send job to
/// the database.
pub async fn send_msg_sync(
    context: &Context,
    chat_id: ChatId,
    msg: &mut Message,
) -> Result<MsgId, Error> {
    if context.is_io_running().await {
        return send_msg(context, chat_id, msg).await;
    }

    if let Some(mut job) = prepare_send_msg(context, chat_id, msg).await? {
        let mut smtp = crate::smtp::Smtp::new();

        let status = job.send_msg_to_smtp(context, &mut smtp).await;

        match status {
            job::Status::Finished(Ok(_)) => {
                context.emit_event(EventType::MsgsChanged {
                    chat_id: msg.chat_id,
                    msg_id: msg.id,
                });

                Ok(msg.id)
            }
            _ => {
                job.save(context).await?;
                Err(format_err!(
                    "failed to send message, queued for later sending"
                ))
            }
        }
    } else {
        // Nothing to do
        Ok(msg.id)
    }
}

async fn send_msg_inner(
    context: &Context,
    chat_id: ChatId,
    msg: &mut Message,
) -> Result<MsgId, Error> {
    if let Some(send_job) = prepare_send_msg(context, chat_id, msg).await? {
        job::add(context, send_job).await;

        context.emit_event(EventType::MsgsChanged {
            chat_id: msg.chat_id,
            msg_id: msg.id,
        });

        if msg.param.exists(Param::SetLatitude) {
            context.emit_event(EventType::LocationChanged(Some(DC_CONTACT_ID_SELF)));
        }
    }

    Ok(msg.id)
}

async fn prepare_send_msg(
    context: &Context,
    chat_id: ChatId,
    msg: &mut Message,
) -> Result<Option<crate::job::Job>, Error> {
    // dc_prepare_msg() leaves the message state to OutPreparing, we
    // only have to change the state to OutPending in this case.
    // Otherwise we still have to prepare the message, which will set
    // the state to OutPending.
    if msg.state != MessageState::OutPreparing {
        // automatically prepare normal messages
        prepare_msg_common(context, chat_id, msg).await?;
    } else {
        // update message state of separately prepared messages
        ensure!(
            chat_id.is_unset() || chat_id == msg.chat_id,
            "Inconsistent chat ID"
        );
        message::update_msg_state(context, msg.id, MessageState::OutPending).await;
    }
    let job = job::send_msg_job(context, msg.id).await?;

    Ok(job)
}

pub async fn send_text_msg(
    context: &Context,
    chat_id: ChatId,
    text_to_send: String,
) -> Result<MsgId, Error> {
    ensure!(
        !chat_id.is_special(),
        "bad chat_id, can not be a special chat: {}",
        chat_id
    );

    let mut msg = Message::new(Viewtype::Text);
    msg.text = Some(text_to_send);
    send_msg(context, chat_id, &mut msg).await
}

pub async fn send_videochat_invitation(context: &Context, chat_id: ChatId) -> Result<MsgId, Error> {
    ensure!(
        !chat_id.is_special(),
        "video chat invitation cannot be sent to special chat: {}",
        chat_id
    );

    let instance = if let Some(instance) = context.get_config(Config::WebrtcInstance).await {
        if !instance.is_empty() {
            instance
        } else {
            bail!("webrtc_instance is empty");
        }
    } else {
        bail!("webrtc_instance not set");
    };

    let instance = Message::create_webrtc_instance(&instance, &dc_create_id());

    let mut msg = Message::new(Viewtype::VideochatInvitation);
    msg.param.set(Param::WebrtcRoom, &instance);
    msg.text = Some(
        context
            .stock_string_repl_str(
                StockMessage::VideochatInviteMsgBody,
                Message::parse_webrtc_instance(&instance).1,
            )
            .await,
    );
    send_msg(context, chat_id, &mut msg).await
}

pub async fn get_chat_msgs(
    context: &Context,
    chat_id: ChatId,
    flags: u32,
    marker1before: Option<MsgId>,
) -> Vec<ChatItem> {
    match delete_expired_messages(context).await {
        Err(err) => warn!(context, "Failed to delete expired messages: {}", err),
        Ok(messages_deleted) => {
            if messages_deleted {
                // Trigger reload of chatlist.
                //
                // On desktop chatlist is always shown on the side,
                // and it is important to update the last message shown
                // there.
                context.emit_event(EventType::MsgsChanged {
                    msg_id: MsgId::new(0),
                    chat_id: ChatId::new(0),
                })
            }
        }
    }

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
                    ret.push(ChatItem::Marker1);
                }
            }
            if (flags & DC_GCM_ADDDAYMARKER) != 0 {
                let curr_local_timestamp = ts + cnv_to_local;
                let curr_day = curr_local_timestamp / 86400;
                if curr_day != last_day {
                    ret.push(ChatItem::DayMarker {
                        timestamp: curr_day,
                    });
                    last_day = curr_day;
                }
            }
            ret.push(ChatItem::Message { msg_id: curr_id });
        }
        Ok(ret)
    };
    let success = if chat_id.is_deaddrop() {
        let show_emails = ShowEmails::from_i32(context.get_config_int(Config::ShowEmails).await)
            .unwrap_or_default();
        context
            .sql
            .query_map(
                "SELECT m.id AS id, m.timestamp AS timestamp
               FROM msgs m
               LEFT JOIN chats
                      ON m.chat_id=chats.id
               LEFT JOIN contacts
                      ON m.from_id=contacts.id
              WHERE m.from_id!=1  -- 1=DC_CONTACT_ID_SELF
                AND m.from_id!=2  -- 2=DC_CONTACT_ID_INFO
                AND m.hidden=0
                AND chats.blocked=2
                AND contacts.blocked=0
                AND m.msgrmsg>=?
              ORDER BY m.timestamp,m.id;",
                paramsv![if show_emails == ShowEmails::All { 0 } else { 1 }],
                process_row,
                process_rows,
            )
            .await
    } else {
        context
            .sql
            .query_map(
                "SELECT m.id AS id, m.timestamp AS timestamp
               FROM msgs m
              WHERE m.chat_id=?
                AND m.hidden=0
              ORDER BY m.timestamp, m.id;",
                paramsv![chat_id],
                process_row,
                process_rows,
            )
            .await
    };
    match success {
        Ok(ret) => ret,
        Err(e) => {
            error!(context, "Failed to get chat messages: {}", e);
            Vec::new()
        }
    }
}

pub(crate) async fn marknoticed_chat_if_older_than(
    context: &Context,
    chat_id: ChatId,
    timestamp: i64,
) -> Result<(), Error> {
    if let Some(chat_timestamp) = context
        .sql
        .query_get_value(
            context,
            "SELECT MAX(timestamp) FROM msgs WHERE chat_id=?",
            paramsv![chat_id],
        )
        .await
    {
        if timestamp > chat_timestamp {
            marknoticed_chat(context, chat_id).await?;
        }
    }
    Ok(())
}

pub async fn marknoticed_chat(context: &Context, chat_id: ChatId) -> Result<(), Error> {
    // "WHERE" below uses the index `(state, hidden, chat_id)`, see get_fresh_msg_cnt() for reasoning
    if !context
        .sql
        .exists(
            "SELECT id FROM msgs WHERE state=? AND hidden=0 AND chat_id=?;",
            paramsv![MessageState::InFresh, chat_id],
        )
        .await?
    {
        return Ok(());
    }

    context
        .sql
        .execute(
            "UPDATE msgs
            SET state=?
          WHERE state=?
            AND hidden=0
            AND chat_id=?;",
            paramsv![MessageState::InNoticed, MessageState::InFresh, chat_id],
        )
        .await?;

    context.emit_event(EventType::MsgsNoticed(chat_id));

    Ok(())
}

pub async fn get_chat_media(
    context: &Context,
    chat_id: ChatId,
    msg_type: Viewtype,
    msg_type2: Viewtype,
    msg_type3: Viewtype,
) -> Vec<MsgId> {
    // TODO This query could/should be converted to `AND type IN (?, ?, ?)`.
    context
        .sql
        .query_map(
            "SELECT id
               FROM msgs
              WHERE chat_id=?
                AND (type=? OR type=? OR type=?)
              ORDER BY timestamp, id;",
            paramsv![
                chat_id,
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
        .await
        .unwrap_or_default()
}

/// Indicates the direction over which to iterate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum Direction {
    Forward = 1,
    Backward = -1,
}

pub async fn get_next_media(
    context: &Context,
    curr_msg_id: MsgId,
    direction: Direction,
    msg_type: Viewtype,
    msg_type2: Viewtype,
    msg_type3: Viewtype,
) -> Option<MsgId> {
    let mut ret: Option<MsgId> = None;

    if let Ok(msg) = Message::load_from_db(context, curr_msg_id).await {
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
        )
        .await;
        for (i, msg_id) in list.iter().enumerate() {
            if curr_msg_id == *msg_id {
                match direction {
                    Direction::Forward => {
                        if i + 1 < list.len() {
                            ret = list.get(i + 1).copied();
                        }
                    }
                    Direction::Backward => {
                        if i >= 1 {
                            ret = list.get(i - 1).copied();
                        }
                    }
                }
                break;
            }
        }
    }
    ret
}

pub async fn get_chat_contacts(context: &Context, chat_id: ChatId) -> Vec<u32> {
    /* Normal chats do not include SELF.  Group chats do (as it may happen that one is deleted from a
    groupchat but the chats stays visible, moreover, this makes displaying lists easier) */

    if chat_id.is_deaddrop() {
        return Vec::new();
    }

    // we could also create a list for all contacts in the deaddrop by searching contacts belonging to chats with
    // chats.blocked=2, however, currently this is not needed

    context
        .sql
        .query_map(
            "SELECT cc.contact_id
               FROM chats_contacts cc
               LEFT JOIN contacts c
                      ON c.id=cc.contact_id
              WHERE cc.chat_id=?
              ORDER BY c.id=1, LOWER(c.name||c.addr), c.id;",
            paramsv![chat_id],
            |row| row.get::<_, u32>(0),
            |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into),
        )
        .await
        .unwrap_or_default()
}

pub async fn create_group_chat(
    context: &Context,
    protect: ProtectionStatus,
    chat_name: impl AsRef<str>,
) -> Result<ChatId, Error> {
    let chat_name = improve_single_line_input(chat_name);
    ensure!(!chat_name.is_empty(), "Invalid chat name");

    let draft_txt = context
        .stock_string_repl_str(StockMessage::NewGroupDraft, &chat_name)
        .await;
    let grpid = dc_create_id();

    context.sql.execute(
        "INSERT INTO chats (type, name, grpid, param, created_timestamp) VALUES(?, ?, ?, \'U=1\', ?);",
        paramsv![
            Chattype::Group,
            chat_name,
            grpid,
            time(),
        ],
    ).await?;

    let row_id = context
        .sql
        .get_rowid(context, "chats", "grpid", grpid)
        .await?;

    let chat_id = ChatId::new(row_id);
    if add_to_chat_contacts_table(context, chat_id, DC_CONTACT_ID_SELF).await {
        let mut draft_msg = Message::new(Viewtype::Text);
        draft_msg.set_text(Some(draft_txt));
        chat_id.set_draft_raw(context, &mut draft_msg).await;
    }

    context.emit_event(EventType::MsgsChanged {
        msg_id: MsgId::new(0),
        chat_id: ChatId::new(0),
    });

    if protect == ProtectionStatus::Protected {
        // this part is to stay compatible to verified groups,
        // in some future, we will drop the "protect"-flag from create_group_chat()
        chat_id.inner_set_protection(context, protect).await?;
    }

    Ok(chat_id)
}

/// add a contact to the chats_contact table
pub(crate) async fn add_to_chat_contacts_table(
    context: &Context,
    chat_id: ChatId,
    contact_id: u32,
) -> bool {
    match context
        .sql
        .execute(
            "INSERT INTO chats_contacts (chat_id, contact_id) VALUES(?, ?)",
            paramsv![chat_id, contact_id as i32],
        )
        .await
    {
        Ok(_) => true,
        Err(err) => {
            error!(
                context,
                "could not add {} to chat {} table: {}", contact_id, chat_id, err
            );

            false
        }
    }
}

/// remove a contact from the chats_contact table
pub(crate) async fn remove_from_chat_contacts_table(
    context: &Context,
    chat_id: ChatId,
    contact_id: u32,
) -> bool {
    match context
        .sql
        .execute(
            "DELETE FROM chats_contacts WHERE chat_id=? AND contact_id=?",
            paramsv![chat_id, contact_id as i32],
        )
        .await
    {
        Ok(_) => true,
        Err(_) => {
            warn!(
                context,
                "could not remove contact {:?} from chat {:?}", contact_id, chat_id
            );

            false
        }
    }
}

/// Adds a contact to the chat.
pub async fn add_contact_to_chat(context: &Context, chat_id: ChatId, contact_id: u32) -> bool {
    match add_contact_to_chat_ex(context, chat_id, contact_id, false).await {
        Ok(res) => res,
        Err(err) => {
            error!(context, "failed to add contact: {}", err);
            false
        }
    }
}

pub(crate) async fn add_contact_to_chat_ex(
    context: &Context,
    chat_id: ChatId,
    contact_id: u32,
    from_handshake: bool,
) -> Result<bool, Error> {
    ensure!(!chat_id.is_special(), "can not add member to special chats");
    let contact = Contact::get_by_id(context, contact_id).await?;
    let mut msg = Message::default();

    reset_gossiped_timestamp(context, chat_id).await?;

    /*this also makes sure, not contacts are added to special or normal chats*/
    let mut chat = Chat::load_from_db(context, chat_id).await?;
    ensure!(
        real_group_exists(context, chat_id).await,
        "{} is not a group where one can add members",
        chat_id
    );
    ensure!(
        Contact::real_exists_by_id(context, contact_id).await || contact_id == DC_CONTACT_ID_SELF,
        "invalid contact_id {} for adding to group",
        contact_id
    );

    if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF as u32).await {
        /* we should respect this - whatever we send to the group, it gets discarded anyway! */
        emit_event!(
            context,
            EventType::ErrorSelfNotInGroup(
                "Cannot add contact to group; self not in group.".into()
            )
        );
        bail!("can not add contact because our account is not part of it");
    }
    if from_handshake && chat.param.get_int(Param::Unpromoted).unwrap_or_default() == 1 {
        chat.param.remove(Param::Unpromoted);
        chat.update_param(context).await?;
    }
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .await
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

    if is_contact_in_chat(context, chat_id, contact_id).await {
        if !from_handshake {
            return Ok(true);
        }
    } else {
        // else continue and send status mail
        if chat.is_protected()
            && contact.is_verified(context).await != VerifiedStatus::BidirectVerified
        {
            error!(
                context,
                "Only bidirectional verified contacts can be added to protected chats."
            );
            return Ok(false);
        }
        if !add_to_chat_contacts_table(context, chat_id, contact_id).await {
            return Ok(false);
        }
    }
    if chat.param.get_int(Param::Unpromoted).unwrap_or_default() == 0 {
        msg.viewtype = Viewtype::Text;
        msg.text = Some(
            context
                .stock_system_msg(
                    StockMessage::MsgAddMember,
                    contact.get_addr(),
                    "",
                    DC_CONTACT_ID_SELF as u32,
                )
                .await,
        );
        msg.param.set_cmd(SystemMessage::MemberAddedToGroup);
        msg.param.set(Param::Arg, contact.get_addr());
        msg.param.set_int(Param::Arg2, from_handshake.into());
        msg.id = send_msg(context, chat_id, &mut msg).await?;
    }
    context.emit_event(EventType::ChatModified(chat_id));
    Ok(true)
}

async fn real_group_exists(context: &Context, chat_id: ChatId) -> bool {
    // check if a group or a verified group exists under the given ID
    if !context.sql.is_open().await || chat_id.is_special() {
        return false;
    }

    context
        .sql
        .exists(
            "SELECT id FROM chats WHERE id=? AND type=120;",
            paramsv![chat_id],
        )
        .await
        .unwrap_or_default()
}

pub(crate) async fn reset_gossiped_timestamp(
    context: &Context,
    chat_id: ChatId,
) -> Result<(), Error> {
    set_gossiped_timestamp(context, chat_id, 0).await
}

/// Get timestamp of the last gossip sent in the chat.
/// Zero return value means that gossip was never sent.
pub async fn get_gossiped_timestamp(context: &Context, chat_id: ChatId) -> i64 {
    context
        .sql
        .query_get_value::<i64>(
            context,
            "SELECT gossiped_timestamp FROM chats WHERE id=?;",
            paramsv![chat_id],
        )
        .await
        .unwrap_or_default()
}

pub(crate) async fn set_gossiped_timestamp(
    context: &Context,
    chat_id: ChatId,
    timestamp: i64,
) -> Result<(), Error> {
    ensure!(!chat_id.is_special(), "can not add member to special chats");
    info!(
        context,
        "set gossiped_timestamp for chat #{} to {}.", chat_id, timestamp,
    );

    context
        .sql
        .execute(
            "UPDATE chats SET gossiped_timestamp=? WHERE id=?;",
            paramsv![timestamp, chat_id],
        )
        .await?;

    Ok(())
}

pub(crate) async fn shall_attach_selfavatar(
    context: &Context,
    chat_id: ChatId,
) -> Result<bool, Error> {
    // versions before 12/2019 already allowed to set selfavatar, however, it was never sent to others.
    // to avoid sending out previously set selfavatars unexpectedly we added this additional check.
    // it can be removed after some time.
    if !context
        .sql
        .get_raw_config_bool(context, "attach_selfavatar")
        .await
    {
        return Ok(false);
    }

    let timestamp_some_days_ago = time() - DC_RESEND_USER_AVATAR_DAYS * 24 * 60 * 60;
    let needs_attach = context
        .sql
        .query_map(
            "SELECT c.selfavatar_sent
           FROM chats_contacts cc
           LEFT JOIN contacts c ON c.id=cc.contact_id
          WHERE cc.chat_id=? AND cc.contact_id!=?;",
            paramsv![chat_id, DC_CONTACT_ID_SELF],
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
        )
        .await?;
    Ok(needs_attach)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MuteDuration {
    NotMuted,
    Forever,
    Until(SystemTime),
}

impl rusqlite::types::ToSql for MuteDuration {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let duration: i64 = match &self {
            MuteDuration::NotMuted => 0,
            MuteDuration::Forever => -1,
            MuteDuration::Until(when) => {
                let duration = when
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
                i64::try_from(duration.as_secs())
                    .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?
            }
        };
        let val = rusqlite::types::Value::Integer(duration);
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

impl rusqlite::types::FromSql for MuteDuration {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        // Negative values other than -1 should not be in the
        // database.  If found they'll be NotMuted.
        match i64::column_result(value)? {
            0 => Ok(MuteDuration::NotMuted),
            -1 => Ok(MuteDuration::Forever),
            n if n > 0 => match SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(n as u64)) {
                Some(t) => Ok(MuteDuration::Until(t)),
                None => Err(rusqlite::types::FromSqlError::OutOfRange(n)),
            },
            _ => Ok(MuteDuration::NotMuted),
        }
    }
}

pub async fn set_muted(
    context: &Context,
    chat_id: ChatId,
    duration: MuteDuration,
) -> Result<(), Error> {
    ensure!(!chat_id.is_special(), "Invalid chat ID");
    if context
        .sql
        .execute(
            "UPDATE chats SET muted_until=? WHERE id=?;",
            paramsv![duration, chat_id],
        )
        .await
        .is_ok()
    {
        context.emit_event(EventType::ChatModified(chat_id));
    } else {
        bail!("Failed to set mute duration, chat might not exist -");
    }
    Ok(())
}

pub async fn remove_contact_from_chat(
    context: &Context,
    chat_id: ChatId,
    contact_id: u32,
) -> Result<(), Error> {
    ensure!(
        !chat_id.is_special(),
        "bad chat_id, can not be special chat: {}",
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
    if let Ok(chat) = Chat::load_from_db(context, chat_id).await {
        if real_group_exists(context, chat_id).await {
            if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF).await {
                emit_event!(
                    context,
                    EventType::ErrorSelfNotInGroup(
                        "Cannot remove contact from chat; self not in group.".into()
                    )
                );
            } else {
                if let Ok(contact) = Contact::get_by_id(context, contact_id).await {
                    if chat.is_promoted() {
                        msg.viewtype = Viewtype::Text;
                        if contact.id == DC_CONTACT_ID_SELF {
                            set_group_explicitly_left(context, chat.grpid).await?;
                            msg.text = Some(
                                context
                                    .stock_system_msg(
                                        StockMessage::MsgGroupLeft,
                                        "",
                                        "",
                                        DC_CONTACT_ID_SELF,
                                    )
                                    .await,
                            );
                        } else {
                            msg.text = Some(
                                context
                                    .stock_system_msg(
                                        StockMessage::MsgDelMember,
                                        contact.get_addr(),
                                        "",
                                        DC_CONTACT_ID_SELF,
                                    )
                                    .await,
                            );
                        }
                        msg.param.set_cmd(SystemMessage::MemberRemovedFromGroup);
                        msg.param.set(Param::Arg, contact.get_addr());
                        msg.id = send_msg(context, chat_id, &mut msg).await?;
                    }
                }
                // we remove the member from the chat after constructing the
                // to-be-send message. If between send_msg() and here the
                // process dies the user will have to re-do the action.  It's
                // better than the other way round: you removed
                // someone from DB but no peer or device gets to know about it and
                // group membership is thus different on different devices.
                // Note also that sending a message needs all recipients
                // in order to correctly determine encryption so if we
                // removed it first, it would complicate the
                // check/encryption logic.
                success = remove_from_chat_contacts_table(context, chat_id, contact_id).await;
                context.emit_event(EventType::ChatModified(chat_id));
            }
        }
    }

    if !success {
        bail!("Failed to remove contact");
    }

    Ok(())
}

async fn set_group_explicitly_left(context: &Context, grpid: impl AsRef<str>) -> Result<(), Error> {
    if !is_group_explicitly_left(context, grpid.as_ref()).await? {
        context
            .sql
            .execute(
                "INSERT INTO leftgrps (grpid) VALUES(?);",
                paramsv![grpid.as_ref().to_string()],
            )
            .await?;
    }

    Ok(())
}

pub(crate) async fn is_group_explicitly_left(
    context: &Context,
    grpid: impl AsRef<str>,
) -> Result<bool, Error> {
    context
        .sql
        .exists(
            "SELECT id FROM leftgrps WHERE grpid=?;",
            paramsv![grpid.as_ref()],
        )
        .await
        .map_err(Into::into)
}

pub async fn set_chat_name(
    context: &Context,
    chat_id: ChatId,
    new_name: impl AsRef<str>,
) -> Result<(), Error> {
    let new_name = improve_single_line_input(new_name);
    /* the function only sets the names of group chats; normal chats get their names from the contacts */
    let mut success = false;

    ensure!(!new_name.is_empty(), "Invalid name");
    ensure!(!chat_id.is_special(), "Invalid chat ID");

    let chat = Chat::load_from_db(context, chat_id).await?;
    let mut msg = Message::default();

    if real_group_exists(context, chat_id).await {
        if chat.name == new_name {
            success = true;
        } else if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF).await {
            emit_event!(
                context,
                EventType::ErrorSelfNotInGroup("Cannot set chat name; self not in group".into())
            );
        } else {
            /* we should respect this - whatever we send to the group, it gets discarded anyway! */
            if context
                .sql
                .execute(
                    "UPDATE chats SET name=? WHERE id=?;",
                    paramsv![new_name.to_string(), chat_id],
                )
                .await
                .is_ok()
            {
                if chat.is_promoted() {
                    msg.viewtype = Viewtype::Text;
                    msg.text = Some(
                        context
                            .stock_system_msg(
                                StockMessage::MsgGrpName,
                                &chat.name,
                                &new_name,
                                DC_CONTACT_ID_SELF,
                            )
                            .await,
                    );
                    msg.param.set_cmd(SystemMessage::GroupNameChanged);
                    if !chat.name.is_empty() {
                        msg.param.set(Param::Arg, &chat.name);
                    }
                    msg.id = send_msg(context, chat_id, &mut msg).await?;
                    context.emit_event(EventType::MsgsChanged {
                        chat_id,
                        msg_id: msg.id,
                    });
                }
                context.emit_event(EventType::ChatModified(chat_id));
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
pub async fn set_chat_profile_image(
    context: &Context,
    chat_id: ChatId,
    new_image: impl AsRef<str>, // XXX use PathBuf
) -> Result<(), Error> {
    ensure!(!chat_id.is_special(), "Invalid chat ID");
    let mut chat = Chat::load_from_db(context, chat_id).await?;
    ensure!(
        real_group_exists(context, chat_id).await,
        "Failed to set profile image; group does not exist"
    );
    /* we should respect this - whatever we send to the group, it gets discarded anyway! */
    if !is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF).await {
        emit_event!(
            context,
            EventType::ErrorSelfNotInGroup(
                "Cannot set chat profile image; self not in group.".into()
            )
        );
        bail!("Failed to set profile image");
    }
    let mut msg = Message::new(Viewtype::Text);
    msg.param
        .set_int(Param::Cmd, SystemMessage::GroupImageChanged as i32);
    if new_image.as_ref().is_empty() {
        chat.param.remove(Param::ProfileImage);
        msg.param.remove(Param::Arg);
        msg.text = Some(
            context
                .stock_system_msg(StockMessage::MsgGrpImgDeleted, "", "", DC_CONTACT_ID_SELF)
                .await,
        );
    } else {
        let image_blob = match BlobObject::from_path(context, Path::new(new_image.as_ref())) {
            Ok(blob) => Ok(blob),
            Err(err) => match err {
                BlobError::WrongBlobdir { .. } => {
                    BlobObject::create_and_copy(context, Path::new(new_image.as_ref())).await
                }
                _ => Err(err),
            },
        }?;
        image_blob.recode_to_avatar_size(context)?;
        chat.param.set(Param::ProfileImage, image_blob.as_name());
        msg.param.set(Param::Arg, image_blob.as_name());
        msg.text = Some(
            context
                .stock_system_msg(StockMessage::MsgGrpImgChanged, "", "", DC_CONTACT_ID_SELF)
                .await,
        );
    }
    chat.update_param(context).await?;
    if chat.is_promoted() {
        msg.id = send_msg(context, chat_id, &mut msg).await?;
        emit_event!(
            context,
            EventType::MsgsChanged {
                chat_id,
                msg_id: msg.id
            }
        );
    }
    emit_event!(context, EventType::ChatModified(chat_id));
    Ok(())
}

pub async fn forward_msgs(
    context: &Context,
    msg_ids: &[MsgId],
    chat_id: ChatId,
) -> Result<(), Error> {
    ensure!(!msg_ids.is_empty(), "empty msgs_ids: nothing to forward");
    ensure!(!chat_id.is_special(), "can not forward to special chat");

    let mut created_chats: Vec<ChatId> = Vec::new();
    let mut created_msgs: Vec<MsgId> = Vec::new();
    let mut curr_timestamp: i64;

    chat_id.unarchive(context).await?;
    if let Ok(mut chat) = Chat::load_from_db(context, chat_id).await {
        ensure!(chat.can_send(), "cannot send to {}", chat_id);
        curr_timestamp = dc_create_smeared_timestamps(context, msg_ids.len()).await;
        let ids = context
            .sql
            .query_map(
                format!(
                    "SELECT id FROM msgs WHERE id IN({}) ORDER BY timestamp,id",
                    msg_ids.iter().map(|_| "?").join(",")
                ),
                msg_ids.iter().map(|v| v as &dyn crate::ToSql).collect(),
                |row| row.get::<_, MsgId>(0),
                |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .await?;

        for id in ids {
            let src_msg_id: MsgId = id;
            let msg = Message::load_from_db(context, src_msg_id).await;
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
                new_msg_id = chat.prepare_msg_raw(context, &mut msg, fresh9).await?;
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

                msg.update_param(context).await;
                msg.param = save_param;
            } else {
                msg.state = MessageState::OutPending;
                let fresh10 = curr_timestamp;
                curr_timestamp += 1;
                new_msg_id = chat.prepare_msg_raw(context, &mut msg, fresh10).await?;
                if let Some(send_job) = job::send_msg_job(context, new_msg_id).await? {
                    job::add(context, send_job).await;
                }
            }
            created_chats.push(chat_id);
            created_msgs.push(new_msg_id);
        }
    }
    for (chat_id, msg_id) in created_chats.iter().zip(created_msgs.iter()) {
        context.emit_event(EventType::MsgsChanged {
            chat_id: *chat_id,
            msg_id: *msg_id,
        });
    }
    Ok(())
}

pub(crate) async fn get_chat_contact_cnt(context: &Context, chat_id: ChatId) -> usize {
    context
        .sql
        .query_get_value::<isize>(
            context,
            "SELECT COUNT(*) FROM chats_contacts WHERE chat_id=?;",
            paramsv![chat_id],
        )
        .await
        .unwrap_or_default() as usize
}

pub(crate) async fn get_chat_cnt(context: &Context) -> usize {
    if context.sql.is_open().await {
        /* no database, no chats - this is no error (needed eg. for information) */
        context
            .sql
            .query_get_value::<isize>(
                context,
                "SELECT COUNT(*) FROM chats WHERE id>9 AND blocked=0;",
                paramsv![],
            )
            .await
            .unwrap_or_default() as usize
    } else {
        0
    }
}

/// Returns a tuple of `(chatid, is_protected, blocked)`.
pub(crate) async fn get_chat_id_by_grpid(
    context: &Context,
    grpid: impl AsRef<str>,
) -> Result<(ChatId, bool, Blocked), sql::Error> {
    context
        .sql
        .query_row(
            "SELECT id, blocked, protected FROM chats WHERE grpid=?;",
            paramsv![grpid.as_ref()],
            |row| {
                let chat_id = row.get::<_, ChatId>(0)?;

                let b = row.get::<_, Option<Blocked>>(1)?.unwrap_or_default();
                let p = row
                    .get::<_, Option<ProtectionStatus>>(2)?
                    .unwrap_or_default();
                Ok((chat_id, p == ProtectionStatus::Protected, b))
            },
        )
        .await
}

/// Adds a message to device chat.
///
/// Optional `label` can be provided to ensure that message is added only once.
/// If `important` is true, a notification will be sent.
pub async fn add_device_msg_with_importance(
    context: &Context,
    label: Option<&str>,
    msg: Option<&mut Message>,
    important: bool,
) -> Result<MsgId, Error> {
    ensure!(
        label.is_some() || msg.is_some(),
        "device-messages need label, msg or both"
    );
    let mut chat_id = ChatId::new(0);
    let mut msg_id = MsgId::new_unset();

    if let Some(label) = label {
        if was_device_msg_ever_added(context, label).await? {
            info!(context, "device-message {} already added", label);
            return Ok(msg_id);
        }
    }

    if let Some(msg) = msg {
        chat_id = create_or_lookup_by_contact_id(context, DC_CONTACT_ID_DEVICE, Blocked::Not)
            .await?
            .0;

        let rfc724_mid = dc_create_outgoing_rfc724_mid(None, "@device");
        msg.try_calc_and_set_dimensions(context).await.ok();
        prepare_msg_blob(context, msg).await?;
        chat_id.unarchive(context).await?;

        let timestamp_sent = dc_create_smeared_timestamp(context).await;

        // makes sure, the added message is the last one,
        // even if the date is wrong (useful esp. when warning about bad dates)
        let mut timestamp_sort = timestamp_sent;
        if let Some(last_msg_time) = context
            .sql
            .query_get_value(
                context,
                "SELECT MAX(timestamp) FROM msgs WHERE chat_id=?",
                paramsv![chat_id],
            )
            .await
        {
            if timestamp_sort <= last_msg_time {
                timestamp_sort = last_msg_time + 1;
            }
        }

        context.sql.execute(
            "INSERT INTO msgs (chat_id,from_id,to_id, timestamp,timestamp_sent,timestamp_rcvd,type,state, txt,param,rfc724_mid) \
             VALUES (?,?,?, ?,?,?,?,?, ?,?,?);",
            paramsv![
                chat_id,
                DC_CONTACT_ID_DEVICE,
                DC_CONTACT_ID_SELF,
                timestamp_sort,
                timestamp_sent,
                timestamp_sent, // timestamp_sent equals timestamp_rcvd
                msg.viewtype,
                MessageState::InFresh,
                msg.text.as_ref().cloned().unwrap_or_default(),
                msg.param.to_string(),
                rfc724_mid,
            ],
        ).await?;

        let row_id = context
            .sql
            .get_rowid(context, "msgs", "rfc724_mid", &rfc724_mid)
            .await?;
        msg_id = MsgId::new(row_id);
    }

    if let Some(label) = label {
        context
            .sql
            .execute(
                "INSERT INTO devmsglabels (label) VALUES (?);",
                paramsv![label.to_string()],
            )
            .await?;
    }

    if !msg_id.is_unset() {
        if important {
            context.emit_event(EventType::IncomingMsg { chat_id, msg_id });
        } else {
            context.emit_event(EventType::MsgsChanged { chat_id, msg_id });
        }
    }

    Ok(msg_id)
}

pub async fn add_device_msg(
    context: &Context,
    label: Option<&str>,
    msg: Option<&mut Message>,
) -> Result<MsgId, Error> {
    add_device_msg_with_importance(context, label, msg, false).await
}

pub async fn was_device_msg_ever_added(context: &Context, label: &str) -> Result<bool, Error> {
    ensure!(!label.is_empty(), "empty label");
    if let Ok(()) = context
        .sql
        .query_row(
            "SELECT label FROM devmsglabels WHERE label=?",
            paramsv![label],
            |_| Ok(()),
        )
        .await
    {
        return Ok(true);
    }

    Ok(false)
}

// needed on device-switches during export/import;
// - deletion in `msgs` with `DC_CONTACT_ID_DEVICE` makes sure,
//   no wrong information are shown in the device chat
// - deletion in `devmsglabels` makes sure,
//   deleted messages are resetted and useful messages can be added again
pub(crate) async fn delete_and_reset_all_device_msgs(context: &Context) -> Result<(), Error> {
    context
        .sql
        .execute(
            "DELETE FROM msgs WHERE from_id=?;",
            paramsv![DC_CONTACT_ID_DEVICE],
        )
        .await?;
    context
        .sql
        .execute("DELETE FROM devmsglabels;", paramsv![])
        .await?;
    Ok(())
}

/// Adds an informational message to chat.
///
/// For example, it can be a message showing that a member was added to a group.
pub(crate) async fn add_info_msg_with_cmd(
    context: &Context,
    chat_id: ChatId,
    text: impl AsRef<str>,
    cmd: SystemMessage,
) -> Result<MsgId, Error> {
    let rfc724_mid = dc_create_outgoing_rfc724_mid(None, "@device");
    let ephemeral_timer = chat_id.get_ephemeral_timer(context).await?;

    let mut param = Params::new();
    if cmd != SystemMessage::Unknown {
        param.set_cmd(cmd)
    }

    context.sql.execute(
        "INSERT INTO msgs (chat_id,from_id,to_id, timestamp,type,state, txt,rfc724_mid,ephemeral_timer, param) VALUES (?,?,?, ?,?,?, ?,?,?, ?);",
        paramsv![
            chat_id,
            DC_CONTACT_ID_INFO,
            DC_CONTACT_ID_INFO,
            dc_create_smeared_timestamp(context).await,
            Viewtype::Text,
            MessageState::InNoticed,
            text.as_ref().to_string(),
            rfc724_mid,
            ephemeral_timer,
            param.to_string(),
        ]
    ).await?;

    let row_id = context
        .sql
        .get_rowid(context, "msgs", "rfc724_mid", &rfc724_mid)
        .await
        .unwrap_or_default();
    let msg_id = MsgId::new(row_id);
    context.emit_event(EventType::MsgsChanged { chat_id, msg_id });
    Ok(msg_id)
}

pub(crate) async fn add_info_msg(context: &Context, chat_id: ChatId, text: impl AsRef<str>) {
    if let Err(e) = add_info_msg_with_cmd(context, chat_id, text, SystemMessage::Unknown).await {
        warn!(context, "Could not add info msg: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contact::Contact;
    use crate::test_utils::*;

    #[async_std::test]
    async fn test_chat_info() {
        let t = TestContext::new().await;
        let bob = Contact::create(&t.ctx, "bob", "bob@example.com")
            .await
            .unwrap();
        let chat_id = create_by_contact_id(&t.ctx, bob).await.unwrap();
        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        let info = chat.get_info(&t.ctx).await.unwrap();

        // Ensure we can serialize this.
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
                "draft": "",
                "is_muted": false,
                "ephemeral_timer": "Disabled"
            }
        "#;

        // Ensure we can deserialize this.
        let loaded: ChatInfo = serde_json::from_str(expected).unwrap();
        assert_eq!(info, loaded);
    }

    #[async_std::test]
    async fn test_get_draft_no_draft() {
        let t = TestContext::new().await;
        let chat_id = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF)
            .await
            .unwrap();
        let draft = chat_id.get_draft(&t.ctx).await.unwrap();
        assert!(draft.is_none());
    }

    #[async_std::test]
    async fn test_get_draft_special_chat_id() {
        let t = TestContext::new().await;
        let draft = ChatId::new(DC_CHAT_ID_LAST_SPECIAL)
            .get_draft(&t.ctx)
            .await
            .unwrap();
        assert!(draft.is_none());
    }

    #[async_std::test]
    async fn test_get_draft_no_chat() {
        // This is a weird case, maybe this should be an error but we
        // do not get this info from the database currently.
        let t = TestContext::new().await;
        let draft = ChatId::new(42).get_draft(&t.ctx).await.unwrap();
        assert!(draft.is_none());
    }

    #[async_std::test]
    async fn test_get_draft() {
        let t = TestContext::new().await;
        let chat_id = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF)
            .await
            .unwrap();
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hello".to_string()));
        chat_id.set_draft(&t.ctx, Some(&mut msg)).await;
        let draft = chat_id.get_draft(&t.ctx).await.unwrap().unwrap();
        let msg_text = msg.get_text();
        let draft_text = draft.get_text();
        assert_eq!(msg_text, draft_text);
    }

    #[async_std::test]
    async fn test_add_contact_to_chat_ex_add_self() {
        // Adding self to a contact should succeed, even though it's pointless.
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        let added = add_contact_to_chat_ex(&t.ctx, chat_id, DC_CONTACT_ID_SELF, false)
            .await
            .unwrap();
        assert_eq!(added, false);
    }

    #[async_std::test]
    async fn test_self_talk() {
        let t = TestContext::new().await;
        let chat_id = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF)
            .await
            .unwrap();
        assert_eq!(DC_CONTACT_ID_SELF, 1);
        assert!(!chat_id.is_special());
        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        assert_eq!(chat.id, chat_id);
        assert!(chat.is_self_talk());
        assert!(chat.visibility == ChatVisibility::Normal);
        assert!(!chat.is_device_talk());
        assert!(chat.can_send());
        assert_eq!(
            chat.name,
            t.ctx.stock_str(StockMessage::SavedMessages).await
        );
        assert!(chat.get_profile_image(&t.ctx).await.is_some());
    }

    #[async_std::test]
    async fn test_deaddrop_chat() {
        let t = TestContext::new().await;
        let chat = Chat::load_from_db(&t.ctx, ChatId::new(DC_CHAT_ID_DEADDROP))
            .await
            .unwrap();
        assert_eq!(DC_CHAT_ID_DEADDROP, 1);
        assert!(chat.id.is_deaddrop());
        assert!(!chat.is_self_talk());
        assert!(chat.visibility == ChatVisibility::Normal);
        assert!(!chat.is_device_talk());
        assert!(!chat.can_send());
        assert_eq!(chat.name, t.ctx.stock_str(StockMessage::DeadDrop).await);
    }

    #[async_std::test]
    async fn test_add_device_msg_unlabelled() {
        let t = TestContext::new().await;

        // add two device-messages
        let mut msg1 = Message::new(Viewtype::Text);
        msg1.text = Some("first message".to_string());
        let msg1_id = add_device_msg(&t.ctx, None, Some(&mut msg1)).await;
        assert!(msg1_id.is_ok());

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.text = Some("second message".to_string());
        let msg2_id = add_device_msg(&t.ctx, None, Some(&mut msg2)).await;
        assert!(msg2_id.is_ok());
        assert_ne!(msg1_id.as_ref().unwrap(), msg2_id.as_ref().unwrap());

        // check added messages
        let msg1 = message::Message::load_from_db(&t.ctx, msg1_id.unwrap()).await;
        assert!(msg1.is_ok());
        let msg1 = msg1.unwrap();
        assert_eq!(msg1.text.as_ref().unwrap(), "first message");
        assert_eq!(msg1.from_id, DC_CONTACT_ID_DEVICE);
        assert_eq!(msg1.to_id, DC_CONTACT_ID_SELF);
        assert!(!msg1.is_info());
        assert!(!msg1.is_setupmessage());

        let msg2 = message::Message::load_from_db(&t.ctx, msg2_id.unwrap()).await;
        assert!(msg2.is_ok());
        let msg2 = msg2.unwrap();
        assert_eq!(msg2.text.as_ref().unwrap(), "second message");

        // check device chat
        assert_eq!(msg2.chat_id.get_msg_cnt(&t.ctx).await, 2);
    }

    #[async_std::test]
    async fn test_add_device_msg_labelled() {
        let t = TestContext::new().await;

        // add two device-messages with the same label (second attempt is not added)
        let mut msg1 = Message::new(Viewtype::Text);
        msg1.text = Some("first message".to_string());
        let msg1_id = add_device_msg(&t.ctx, Some("any-label"), Some(&mut msg1)).await;
        assert!(msg1_id.is_ok());
        assert!(!msg1_id.as_ref().unwrap().is_unset());

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.text = Some("second message".to_string());
        let msg2_id = add_device_msg(&t.ctx, Some("any-label"), Some(&mut msg2)).await;
        assert!(msg2_id.is_ok());
        assert!(msg2_id.as_ref().unwrap().is_unset());

        // check added message
        let msg1 = message::Message::load_from_db(&t.ctx, *msg1_id.as_ref().unwrap()).await;
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
        assert_eq!(chat_id.get_msg_cnt(&t.ctx).await, 1);
        assert!(!chat_id.is_special());
        let chat = Chat::load_from_db(&t.ctx, chat_id).await;
        assert!(chat.is_ok());
        let chat = chat.unwrap();
        assert_eq!(chat.get_type(), Chattype::Single);
        assert!(chat.is_device_talk());
        assert!(!chat.is_self_talk());
        assert!(!chat.can_send());
        assert_eq!(
            chat.name,
            t.ctx.stock_str(StockMessage::DeviceMessages).await
        );
        assert!(chat.get_profile_image(&t.ctx).await.is_some());

        // delete device message, make sure it is not added again
        message::delete_msgs(&t.ctx, &[*msg1_id.as_ref().unwrap()]).await;
        let msg1 = message::Message::load_from_db(&t.ctx, *msg1_id.as_ref().unwrap()).await;
        assert!(msg1.is_err() || msg1.unwrap().chat_id.is_trash());
        let msg3_id = add_device_msg(&t.ctx, Some("any-label"), Some(&mut msg2)).await;
        assert!(msg3_id.is_ok());
        assert!(msg2_id.as_ref().unwrap().is_unset());
    }

    #[async_std::test]
    async fn test_add_device_msg_label_only() {
        let t = TestContext::new().await;
        let res = add_device_msg(&t.ctx, Some(""), None).await;
        assert!(res.is_err());
        let res = add_device_msg(&t.ctx, Some("some-label"), None).await;
        assert!(res.is_ok());

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());

        let msg_id = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg)).await;
        assert!(msg_id.is_ok());
        assert!(msg_id.as_ref().unwrap().is_unset());

        let msg_id = add_device_msg(&t.ctx, Some("unused-label"), Some(&mut msg)).await;
        assert!(msg_id.is_ok());
        assert!(!msg_id.as_ref().unwrap().is_unset());
    }

    #[async_std::test]
    async fn test_was_device_msg_ever_added() {
        let t = TestContext::new().await;
        add_device_msg(&t.ctx, Some("some-label"), None).await.ok();
        assert!(was_device_msg_ever_added(&t.ctx, "some-label")
            .await
            .unwrap());

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        add_device_msg(&t.ctx, Some("another-label"), Some(&mut msg))
            .await
            .ok();
        assert!(was_device_msg_ever_added(&t.ctx, "another-label")
            .await
            .unwrap());

        assert!(!was_device_msg_ever_added(&t.ctx, "unused-label")
            .await
            .unwrap());

        assert!(was_device_msg_ever_added(&t.ctx, "").await.is_err());
    }

    #[async_std::test]
    async fn test_delete_device_chat() {
        let t = TestContext::new().await;

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg))
            .await
            .ok();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);

        // after the device-chat and all messages are deleted, a re-adding should do nothing
        chats.get_chat_id(0).delete(&t.ctx).await.ok();
        add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg))
            .await
            .ok();
        assert_eq!(chatlist_len(&t.ctx, 0).await, 0)
    }

    #[async_std::test]
    async fn test_device_chat_cannot_sent() {
        let t = TestContext::new().await;
        t.ctx.update_device_chats().await.unwrap();
        let (device_chat_id, _) =
            create_or_lookup_by_contact_id(&t.ctx, DC_CONTACT_ID_DEVICE, Blocked::Not)
                .await
                .unwrap();

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        assert!(send_msg(&t.ctx, device_chat_id, &mut msg).await.is_err());
        assert!(prepare_msg(&t.ctx, device_chat_id, &mut msg).await.is_err());

        let msg_id = add_device_msg(&t.ctx, None, Some(&mut msg)).await.unwrap();
        assert!(forward_msgs(&t.ctx, &[msg_id], device_chat_id)
            .await
            .is_err());
    }

    #[async_std::test]
    async fn test_delete_and_reset_all_device_msgs() {
        let t = TestContext::new().await;
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        let msg_id1 = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg))
            .await
            .unwrap();

        // adding a device message with the same label won't be executed again ...
        assert!(was_device_msg_ever_added(&t.ctx, "some-label")
            .await
            .unwrap());
        let msg_id2 = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg))
            .await
            .unwrap();
        assert!(msg_id2.is_unset());

        // ... unless everything is deleted and resetted - as needed eg. on device switch
        delete_and_reset_all_device_msgs(&t.ctx).await.unwrap();
        assert!(!was_device_msg_ever_added(&t.ctx, "some-label")
            .await
            .unwrap());
        let msg_id3 = add_device_msg(&t.ctx, Some("some-label"), Some(&mut msg))
            .await
            .unwrap();
        assert_ne!(msg_id1, msg_id3);
    }

    async fn chatlist_len(ctx: &Context, listflags: usize) -> usize {
        Chatlist::try_load(ctx, listflags, None, None)
            .await
            .unwrap()
            .len()
    }

    #[async_std::test]
    async fn test_archive() {
        // create two chats
        let t = TestContext::new().await;
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("foo".to_string());
        let msg_id = add_device_msg(&t.ctx, None, Some(&mut msg)).await.unwrap();
        let chat_id1 = message::Message::load_from_db(&t.ctx, msg_id)
            .await
            .unwrap()
            .chat_id;
        let chat_id2 = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF)
            .await
            .unwrap();
        assert!(!chat_id1.is_special());
        assert!(!chat_id2.is_special());
        assert_eq!(get_chat_cnt(&t.ctx).await, 2);
        assert_eq!(chatlist_len(&t.ctx, 0).await, 2);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS).await, 2);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY).await, 0);
        assert_eq!(DC_GCL_ARCHIVED_ONLY, 0x01);
        assert_eq!(DC_GCL_NO_SPECIALS, 0x02);

        // archive first chat
        assert!(chat_id1
            .set_visibility(&t.ctx, ChatVisibility::Archived)
            .await
            .is_ok());
        assert!(
            Chat::load_from_db(&t.ctx, chat_id1)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert!(
            Chat::load_from_db(&t.ctx, chat_id2)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Normal
        );
        assert_eq!(get_chat_cnt(&t.ctx).await, 2);
        assert_eq!(chatlist_len(&t.ctx, 0).await, 2); // including DC_CHAT_ID_ARCHIVED_LINK now
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS).await, 1);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY).await, 1);

        // archive second chat
        assert!(chat_id2
            .set_visibility(&t.ctx, ChatVisibility::Archived)
            .await
            .is_ok());
        assert!(
            Chat::load_from_db(&t.ctx, chat_id1)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert!(
            Chat::load_from_db(&t.ctx, chat_id2)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert_eq!(get_chat_cnt(&t.ctx).await, 2);
        assert_eq!(chatlist_len(&t.ctx, 0).await, 1); // only DC_CHAT_ID_ARCHIVED_LINK now
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS).await, 0);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY).await, 2);

        // archive already archived first chat, unarchive second chat two times
        assert!(chat_id1
            .set_visibility(&t.ctx, ChatVisibility::Archived)
            .await
            .is_ok());
        assert!(chat_id2
            .set_visibility(&t.ctx, ChatVisibility::Normal)
            .await
            .is_ok());
        assert!(chat_id2
            .set_visibility(&t.ctx, ChatVisibility::Normal)
            .await
            .is_ok());
        assert!(
            Chat::load_from_db(&t.ctx, chat_id1)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert!(
            Chat::load_from_db(&t.ctx, chat_id2)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Normal
        );
        assert_eq!(get_chat_cnt(&t.ctx).await, 2);
        assert_eq!(chatlist_len(&t.ctx, 0).await, 2);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_NO_SPECIALS).await, 1);
        assert_eq!(chatlist_len(&t.ctx, DC_GCL_ARCHIVED_ONLY).await, 1);
    }

    async fn get_chats_from_chat_list(ctx: &Context, listflags: usize) -> Vec<ChatId> {
        let chatlist = Chatlist::try_load(ctx, listflags, None, None)
            .await
            .unwrap();
        let mut result = Vec::new();
        for chatlist_index in 0..chatlist.len() {
            result.push(chatlist.get_chat_id(chatlist_index))
        }
        result
    }

    #[async_std::test]
    async fn test_pinned() {
        let t = TestContext::new().await;

        // create 3 chats, wait 1 second in between to get a reliable order (we order by time)
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("foo".to_string());
        let msg_id = add_device_msg(&t.ctx, None, Some(&mut msg)).await.unwrap();
        let chat_id1 = message::Message::load_from_db(&t.ctx, msg_id)
            .await
            .unwrap()
            .chat_id;
        async_std::task::sleep(std::time::Duration::from_millis(1000)).await;
        let chat_id2 = create_by_contact_id(&t.ctx, DC_CONTACT_ID_SELF)
            .await
            .unwrap();
        async_std::task::sleep(std::time::Duration::from_millis(1000)).await;
        let chat_id3 = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();

        let chatlist = get_chats_from_chat_list(&t.ctx, DC_GCL_NO_SPECIALS).await;
        assert_eq!(chatlist, vec![chat_id3, chat_id2, chat_id1]);

        // pin
        assert!(chat_id1
            .set_visibility(&t.ctx, ChatVisibility::Pinned)
            .await
            .is_ok());
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id1)
                .await
                .unwrap()
                .get_visibility(),
            ChatVisibility::Pinned
        );

        // check if chat order changed
        let chatlist = get_chats_from_chat_list(&t.ctx, DC_GCL_NO_SPECIALS).await;
        assert_eq!(chatlist, vec![chat_id1, chat_id3, chat_id2]);

        // unpin
        assert!(chat_id1
            .set_visibility(&t.ctx, ChatVisibility::Normal)
            .await
            .is_ok());
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id1)
                .await
                .unwrap()
                .get_visibility(),
            ChatVisibility::Normal
        );

        // check if chat order changed back
        let chatlist = get_chats_from_chat_list(&t.ctx, DC_GCL_NO_SPECIALS).await;
        assert_eq!(chatlist, vec![chat_id3, chat_id2, chat_id1]);
    }

    #[async_std::test]
    async fn test_set_chat_name() {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id)
                .await
                .unwrap()
                .get_name(),
            "foo"
        );

        set_chat_name(&t.ctx, chat_id, "bar").await.unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id)
                .await
                .unwrap()
                .get_name(),
            "bar"
        );
    }

    #[async_std::test]
    async fn test_create_same_chat_twice() {
        let context = TestContext::new().await;
        let contact1 = Contact::create(&context.ctx, "bob", "bob@mail.de")
            .await
            .unwrap();
        assert_ne!(contact1, 0);

        let chat_id = create_by_contact_id(&context.ctx, contact1).await.unwrap();
        assert!(!chat_id.is_special(), "chat_id too small {}", chat_id);
        let chat = Chat::load_from_db(&context.ctx, chat_id).await.unwrap();

        let chat2_id = create_by_contact_id(&context.ctx, contact1).await.unwrap();
        assert_eq!(chat2_id, chat_id);
        let chat2 = Chat::load_from_db(&context.ctx, chat2_id).await.unwrap();

        assert_eq!(chat2.name, chat.name);
    }

    #[async_std::test]
    async fn test_shall_attach_selfavatar() {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        assert!(!shall_attach_selfavatar(&t.ctx, chat_id).await.unwrap());

        let (contact_id, _) =
            Contact::add_or_lookup(&t.ctx, "", "foo@bar.org", Origin::IncomingUnknownTo)
                .await
                .unwrap();
        add_contact_to_chat(&t.ctx, chat_id, contact_id).await;
        assert!(!shall_attach_selfavatar(&t.ctx, chat_id).await.unwrap());
        t.ctx.set_config(Config::Selfavatar, None).await.unwrap(); // setting to None also forces re-sending
        assert!(shall_attach_selfavatar(&t.ctx, chat_id).await.unwrap());

        assert!(chat_id
            .set_selfavatar_timestamp(&t.ctx, time())
            .await
            .is_ok());
        assert!(!shall_attach_selfavatar(&t.ctx, chat_id).await.unwrap());
    }

    #[async_std::test]
    async fn test_set_mute_duration() {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        // Initial
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id)
                .await
                .unwrap()
                .is_muted(),
            false
        );
        // Forever
        set_muted(&t.ctx, chat_id, MuteDuration::Forever)
            .await
            .unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id)
                .await
                .unwrap()
                .is_muted(),
            true
        );
        // unMute
        set_muted(&t.ctx, chat_id, MuteDuration::NotMuted)
            .await
            .unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id)
                .await
                .unwrap()
                .is_muted(),
            false
        );
        // Timed in the future
        set_muted(
            &t.ctx,
            chat_id,
            MuteDuration::Until(SystemTime::now() + Duration::from_secs(3600)),
        )
        .await
        .unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id)
                .await
                .unwrap()
                .is_muted(),
            true
        );
        // Time in the past
        set_muted(
            &t.ctx,
            chat_id,
            MuteDuration::Until(SystemTime::now() - Duration::from_secs(3600)),
        )
        .await
        .unwrap();
        assert_eq!(
            Chat::load_from_db(&t.ctx, chat_id)
                .await
                .unwrap()
                .is_muted(),
            false
        );
    }

    #[async_std::test]
    async fn test_add_info_msg() {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        add_info_msg(&t.ctx, chat_id, "foo info").await;

        let msg = t.get_last_msg(chat_id).await;
        assert_eq!(msg.get_chat_id(), chat_id);
        assert_eq!(msg.get_viewtype(), Viewtype::Text);
        assert_eq!(msg.get_text().unwrap(), "foo info");
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::Unknown);
    }

    #[async_std::test]
    async fn test_add_info_msg_with_cmd() {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        let msg_id = add_info_msg_with_cmd(
            &t.ctx,
            chat_id,
            "foo bar info",
            SystemMessage::EphemeralTimerChanged,
        )
        .await
        .unwrap();

        let msg = Message::load_from_db(&t.ctx, msg_id).await.unwrap();
        assert_eq!(msg.get_chat_id(), chat_id);
        assert_eq!(msg.get_viewtype(), Viewtype::Text);
        assert_eq!(msg.get_text().unwrap(), "foo bar info");
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::EphemeralTimerChanged);

        let msg2 = t.get_last_msg(chat_id).await;
        assert_eq!(msg.get_id(), msg2.get_id());
    }

    #[async_std::test]
    async fn test_set_protection() {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t.ctx, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        assert!(!chat.is_protected());
        assert!(chat.is_unpromoted());

        // enable protection on unpromoted chat, the info-message is added via add_info_msg()
        chat_id
            .set_protection(&t.ctx, ProtectionStatus::Protected)
            .await
            .unwrap();

        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        assert!(chat.is_protected());
        assert!(chat.is_unpromoted());

        let msgs = get_chat_msgs(&t.ctx, chat_id, 0, None).await;
        assert_eq!(msgs.len(), 1);

        let msg = t.get_last_msg(chat_id).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::ChatProtectionEnabled);
        assert_eq!(msg.get_state(), MessageState::InNoticed);

        // disable protection again, still unpromoted
        chat_id
            .set_protection(&t.ctx, ProtectionStatus::Unprotected)
            .await
            .unwrap();

        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        assert!(!chat.is_protected());
        assert!(chat.is_unpromoted());

        let msg = t.get_last_msg(chat_id).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::ChatProtectionDisabled);
        assert_eq!(msg.get_state(), MessageState::InNoticed);

        // send a message, this switches to promoted state
        send_text_msg(&t.ctx, chat_id, "hi!".to_string())
            .await
            .unwrap();

        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        assert!(!chat.is_protected());
        assert!(!chat.is_unpromoted());

        let msgs = get_chat_msgs(&t.ctx, chat_id, 0, None).await;
        assert_eq!(msgs.len(), 3);

        // enable protection on promoted chat, the info-message is sent via send_msg() this time
        chat_id
            .set_protection(&t.ctx, ProtectionStatus::Protected)
            .await
            .unwrap();
        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        assert!(chat.is_protected());
        assert!(!chat.is_unpromoted());

        let msg = t.get_last_msg(chat_id).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::ChatProtectionEnabled);
        assert_eq!(msg.get_state(), MessageState::OutDelivered); // as bcc-self is disabled and there is nobody else in the chat
    }
}

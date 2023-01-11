//! # Chat module.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use anyhow::{bail, ensure, Context as _, Result};
use deltachat_derive::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

use crate::aheader::EncryptPreference;
use crate::blob::BlobObject;
use crate::color::str_to_color;
use crate::config::Config;
use crate::constants::{
    Blocked, Chattype, DC_CHAT_ID_ALLDONE_HINT, DC_CHAT_ID_ARCHIVED_LINK, DC_CHAT_ID_LAST_SPECIAL,
    DC_CHAT_ID_TRASH, DC_GCM_ADDDAYMARKER, DC_GCM_INFO_ONLY, DC_RESEND_USER_AVATAR_DAYS,
};
use crate::contact::{Contact, ContactId, Origin, VerifiedStatus};
use crate::context::Context;
use crate::ephemeral::Timer as EphemeralTimer;
use crate::events::EventType;
use crate::html::new_html_mimepart;
use crate::message::{self, Message, MessageState, MsgId, Viewtype};
use crate::mimefactory::MimeFactory;
use crate::mimeparser::SystemMessage;
use crate::param::{Param, Params};
use crate::peerstate::{Peerstate, PeerstateVerifiedStatus};
use crate::receive_imf::ReceivedMsg;
use crate::scheduler::InterruptInfo;
use crate::smtp::send_msg_to_smtp;
use crate::stock_str;
use crate::tools::{
    create_id, create_outgoing_rfc724_mid, create_smeared_timestamp, create_smeared_timestamps,
    get_abs_path, gm2local_offset, improve_single_line_input, time, IsNoneOrEmpty,
};
use crate::webxdc::WEBXDC_SUFFIX;
use crate::{location, sql};

/// An chat item, such as a message or a marker.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChatItem {
    Message {
        msg_id: MsgId,
    },

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

/// The reason why messages cannot be sent to the chat.
///
/// The reason is mainly for logging and displaying in debug REPL, thus not translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CantSendReason {
    /// Special chat.
    SpecialChat,

    /// The chat is a device chat.
    DeviceChat,

    /// The chat is a contact request, it needs to be accepted before sending a message.
    ContactRequest,

    /// Mailing list without known List-Post header.
    ReadOnlyMailingList,

    /// Not a member of the chat.
    NotAMember,
}

impl fmt::Display for CantSendReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpecialChat => write!(f, "the chat is a special chat"),
            Self::DeviceChat => write!(f, "the chat is a device chat"),
            Self::ContactRequest => write!(
                f,
                "contact request chat should be accepted before sending messages"
            ),
            Self::ReadOnlyMailingList => {
                write!(f, "mailing list does not have a know post address")
            }
            Self::NotAMember => write!(f, "not a member of the chat"),
        }
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
    pub const fn new(id: u32) -> ChatId {
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
        (0..=DC_CHAT_ID_LAST_SPECIAL.0).contains(&self.0)
    }

    /// Chat ID for messages which need to be deleted.
    ///
    /// Messages which should be deleted get this chat ID and are
    /// deleted later.  Deleted messages need to stay around as long
    /// as they are not deleted on the server so that their rfc724_mid
    /// remains known and downloading them again can be avoided.
    pub fn is_trash(self) -> bool {
        self == DC_CHAT_ID_TRASH
    }

    /// Chat ID signifying there are **any** number of archived chats.
    ///
    /// This chat ID can be returned in a [`Chatlist`] and signals to
    /// the UI to include a link to the archived chats.
    ///
    /// [`Chatlist`]: crate::chatlist::Chatlist
    pub fn is_archived_link(self) -> bool {
        self == DC_CHAT_ID_ARCHIVED_LINK
    }

    /// Virtual chat ID signalling there are **only** archived chats.
    ///
    /// This can be included in the chatlist if the
    /// [`DC_GCL_ADD_ALLDONE_HINT`] flag is used to build the
    /// [`Chatlist`].
    ///
    /// [`DC_GCL_ADD_ALLDONE_HINT`]: crate::constants::DC_GCL_ADD_ALLDONE_HINT
    /// [`Chatlist`]: crate::chatlist::Chatlist
    pub fn is_alldone_hint(self) -> bool {
        self == DC_CHAT_ID_ALLDONE_HINT
    }

    /// Returns the [`ChatId`] for the 1:1 chat with `contact_id` if it exists.
    ///
    /// If it does not exist, `None` is returned.
    pub async fn lookup_by_contact(
        context: &Context,
        contact_id: ContactId,
    ) -> Result<Option<Self>> {
        ChatIdBlocked::lookup_by_contact(context, contact_id)
            .await
            .map(|lookup| lookup.map(|chat| chat.id))
    }

    /// Returns the [`ChatId`] for the 1:1 chat with `contact_id`.
    ///
    /// If the chat does not yet exist an unblocked chat ([`Blocked::Not`]) is created.
    ///
    /// This is an internal API, if **a user action** needs to get a chat
    /// [`ChatId::create_for_contact`] should be used as this also scales up the
    /// [`Contact`]'s origin.
    pub async fn get_for_contact(context: &Context, contact_id: ContactId) -> Result<Self> {
        ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Not)
            .await
            .map(|chat| chat.id)
    }

    /// Returns the unblocked 1:1 chat with `contact_id`.
    ///
    /// This should be used when **a user action** creates a chat 1:1, it ensures the chat
    /// exists, is unblocked and scales the [`Contact`]'s origin.
    pub async fn create_for_contact(context: &Context, contact_id: ContactId) -> Result<Self> {
        ChatId::create_for_contact_with_blocked(context, contact_id, Blocked::Not).await
    }

    /// Same as `create_for_contact()` with an additional `create_blocked` parameter
    /// that is used in case the chat does not exist or to unblock existing chats.
    /// `create_blocked` won't block already unblocked chats again.
    pub(crate) async fn create_for_contact_with_blocked(
        context: &Context,
        contact_id: ContactId,
        create_blocked: Blocked,
    ) -> Result<Self> {
        let chat_id = match ChatIdBlocked::lookup_by_contact(context, contact_id).await? {
            Some(chat) => {
                if create_blocked == Blocked::Not && chat.blocked != Blocked::Not {
                    chat.id.unblock(context).await?;
                }
                chat.id
            }
            None => {
                if Contact::real_exists_by_id(context, contact_id).await?
                    || contact_id == ContactId::SELF
                {
                    let chat_id =
                        ChatIdBlocked::get_for_contact(context, contact_id, create_blocked)
                            .await
                            .map(|chat| chat.id)?;
                    Contact::scaleup_origin_by_id(context, contact_id, Origin::CreateChat).await?;
                    chat_id
                } else {
                    warn!(
                        context,
                        "Cannot create chat, contact {} does not exist.", contact_id,
                    );
                    bail!("Can not create chat for non-existing contact");
                }
            }
        };
        context.emit_msgs_changed_without_ids();
        Ok(chat_id)
    }

    /// Create a group or mailinglist raw database record with the given parameters.
    /// The function does not add SELF nor checks if the record already exists.
    pub(crate) async fn create_multiuser_record(
        context: &Context,
        chattype: Chattype,
        grpid: &str,
        grpname: &str,
        create_blocked: Blocked,
        create_protected: ProtectionStatus,
        param: Option<String>,
    ) -> Result<Self> {
        let row_id =
            context.sql.insert(
                "INSERT INTO chats (type, name, grpid, blocked, created_timestamp, protected, param) VALUES(?, ?, ?, ?, ?, ?, ?);",
                paramsv![
                    chattype,
                    grpname,
                    grpid,
                    create_blocked,
                    create_smeared_timestamp(context).await,
                    create_protected,
                    param.unwrap_or_default(),
                ],
            ).await?;

        let chat_id = ChatId::new(u32::try_from(row_id)?);
        info!(
            context,
            "Created group/mailinglist '{}' grpid={} as {}", grpname, grpid, chat_id
        );

        Ok(chat_id)
    }

    pub async fn set_selfavatar_timestamp(self, context: &Context, timestamp: i64) -> Result<()> {
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

    /// Updates chat blocked status.
    ///
    /// Returns true if the value was modified.
    pub(crate) async fn set_blocked(self, context: &Context, new_blocked: Blocked) -> Result<bool> {
        if self.is_special() {
            bail!("ignoring setting of Block-status for {}", self);
        }
        let count = context
            .sql
            .execute(
                "UPDATE chats SET blocked=?1 WHERE id=?2 AND blocked != ?1",
                paramsv![new_blocked, self],
            )
            .await?;
        Ok(count > 0)
    }

    /// Blocks the chat as a result of explicit user action.
    pub async fn block(self, context: &Context) -> Result<()> {
        let chat = Chat::load_from_db(context, self).await?;

        match chat.typ {
            Chattype::Undefined | Chattype::Broadcast => {
                bail!("Can't block chat of type {:?}", chat.typ)
            }
            Chattype::Single => {
                for contact_id in get_chat_contacts(context, self).await? {
                    if contact_id != ContactId::SELF {
                        info!(
                            context,
                            "Blocking the contact {} to block 1:1 chat", contact_id
                        );
                        Contact::block(context, contact_id).await?;
                    }
                }
            }
            Chattype::Group => {
                info!(context, "Can't block groups yet, deleting the chat");
                self.delete(context).await?;
            }
            Chattype::Mailinglist => {
                if self.set_blocked(context, Blocked::Yes).await? {
                    context.emit_event(EventType::ChatModified(self));
                }
            }
        }

        Ok(())
    }

    /// Unblocks the chat.
    pub async fn unblock(self, context: &Context) -> Result<()> {
        self.set_blocked(context, Blocked::Not).await?;
        Ok(())
    }

    /// Accept the contact request.
    ///
    /// Unblocks the chat and scales up origin of contacts.
    pub async fn accept(self, context: &Context) -> Result<()> {
        let chat = Chat::load_from_db(context, self).await?;

        match chat.typ {
            Chattype::Undefined => bail!("Can't accept chat of undefined chattype"),
            Chattype::Single | Chattype::Group | Chattype::Broadcast => {
                // User has "created a chat" with all these contacts.
                //
                // Previously accepting a chat literally created a chat because unaccepted chats
                // went to "contact requests" list rather than normal chatlist.
                for contact_id in get_chat_contacts(context, self).await? {
                    if contact_id != ContactId::SELF {
                        Contact::scaleup_origin_by_id(context, contact_id, Origin::CreateChat)
                            .await?;
                    }
                }
            }
            Chattype::Mailinglist => {
                // If the message is from a mailing list, the contacts are not counted as "known"
            }
        }

        if self.set_blocked(context, Blocked::Not).await? {
            context.emit_event(EventType::ChatModified(self));
        }

        Ok(())
    }

    /// Sets protection without sending a message.
    ///
    /// Used when a message arrives indicating that someone else has
    /// changed the protection value for a chat.
    pub(crate) async fn inner_set_protection(
        self,
        context: &Context,
        protect: ProtectionStatus,
    ) -> Result<()> {
        ensure!(!self.is_special(), "Invalid chat-id.");

        let chat = Chat::load_from_db(context, self).await?;

        if protect == chat.protected {
            info!(context, "Protection status unchanged for {}.", self);
            return Ok(());
        }

        match protect {
            ProtectionStatus::Protected => match chat.typ {
                Chattype::Single | Chattype::Group | Chattype::Broadcast => {
                    let contact_ids = get_chat_contacts(context, self).await?;
                    for contact_id in contact_ids.into_iter() {
                        let contact = Contact::get_by_id(context, contact_id).await?;
                        if contact.is_verified(context).await? != VerifiedStatus::BidirectVerified {
                            bail!("{} is not verified.", contact.get_display_name());
                        }
                    }
                }
                Chattype::Mailinglist => bail!("Cannot protect mailing lists"),
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
        self.reset_gossiped_timestamp(context).await?;

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
        from_id: ContactId,
    ) -> Result<()> {
        let msg_text = context.stock_protection_msg(protect, from_id).await;
        let cmd = match protect {
            ProtectionStatus::Protected => SystemMessage::ChatProtectionEnabled,
            ProtectionStatus::Unprotected => SystemMessage::ChatProtectionDisabled,
        };

        if promote {
            let mut msg = Message {
                viewtype: Viewtype::Text,
                text: Some(msg_text),
                ..Default::default()
            };
            msg.param.set_cmd(cmd);
            send_msg(context, self, &mut msg).await?;
        } else {
            add_info_msg_with_cmd(
                context,
                self,
                &msg_text,
                cmd,
                create_smeared_timestamp(context).await,
                None,
                None,
                None,
            )
            .await?;
        }

        Ok(())
    }

    /// Sets protection and sends or adds a message.
    pub async fn set_protection(self, context: &Context, protect: ProtectionStatus) -> Result<()> {
        ensure!(!self.is_special(), "set protection: invalid chat-id.");

        let chat = Chat::load_from_db(context, self).await?;

        if let Err(e) = self.inner_set_protection(context, protect).await {
            error!(context, "Cannot set protection: {}", e); // make error user-visible
            return Err(e);
        }

        self.add_protection_msg(context, protect, chat.is_promoted(), ContactId::SELF)
            .await
    }

    /// Archives or unarchives a chat.
    pub async fn set_visibility(self, context: &Context, visibility: ChatVisibility) -> Result<()> {
        ensure!(
            !self.is_special(),
            "bad chat_id, can not be special chat: {}",
            self
        );

        context
            .sql
            .transaction(move |transaction| {
                if visibility == ChatVisibility::Archived {
                    transaction.execute(
                        "UPDATE msgs SET state=? WHERE chat_id=? AND state=?;",
                        paramsv![MessageState::InNoticed, self, MessageState::InFresh],
                    )?;
                }
                transaction.execute(
                    "UPDATE chats SET archived=? WHERE id=?;",
                    paramsv![visibility, self],
                )?;
                Ok(())
            })
            .await?;

        context.emit_msgs_changed_without_ids();

        Ok(())
    }

    // Unarchives a chat that is archived and not muted.
    // Needed when a message is added to a chat so that the chat gets a normal visibility again.
    // Sending an appropriate event is up to the caller.
    pub async fn unarchive_if_not_muted(self, context: &Context) -> Result<()> {
        context
            .sql
            .execute(
                "UPDATE chats SET archived=0 WHERE id=? AND archived=1 AND NOT(muted_until=-1 OR muted_until>?)",
                paramsv![self, time()],
            )
            .await?;
        Ok(())
    }

    /// Deletes a chat.
    pub async fn delete(self, context: &Context) -> Result<()> {
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

        context.emit_msgs_changed_without_ids();

        context.set_config(Config::LastHousekeeping, None).await?;
        context.interrupt_inbox(InterruptInfo::new(false)).await;

        if chat.is_self_talk() {
            let mut msg = Message::new(Viewtype::Text);
            msg.text = Some(stock_str::self_deleted_msg_body(context).await);
            add_device_msg(context, None, Some(&mut msg)).await?;
        }

        Ok(())
    }

    /// Sets draft message.
    ///
    /// Passing `None` as message just deletes the draft
    pub async fn set_draft(self, context: &Context, mut msg: Option<&mut Message>) -> Result<()> {
        if self.is_special() {
            return Ok(());
        }

        let changed = match &mut msg {
            None => self.maybe_delete_draft(context).await?,
            Some(msg) => self.do_set_draft(context, msg).await?,
        };

        if changed {
            context.emit_msgs_changed(
                self,
                if msg.is_some() {
                    match self.get_draft_msg_id(context).await? {
                        Some(msg_id) => msg_id,
                        None => MsgId::new(0),
                    }
                } else {
                    MsgId::new(0)
                },
            );
        }

        Ok(())
    }

    async fn get_draft_msg_id(self, context: &Context) -> Result<Option<MsgId>> {
        let msg_id: Option<MsgId> = context
            .sql
            .query_get_value(
                "SELECT id FROM msgs WHERE chat_id=? AND state=?;",
                paramsv![self, MessageState::OutDraft],
            )
            .await?;
        Ok(msg_id)
    }

    pub async fn get_draft(self, context: &Context) -> Result<Option<Message>> {
        if self.is_special() {
            return Ok(None);
        }
        match self.get_draft_msg_id(context).await? {
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
    async fn maybe_delete_draft(self, context: &Context) -> Result<bool> {
        match self.get_draft_msg_id(context).await? {
            Some(msg_id) => {
                msg_id.delete_from_db(context).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Set provided message as draft message for specified chat.
    /// Returns true if the draft was added or updated in place.
    async fn do_set_draft(self, context: &Context, msg: &mut Message) -> Result<bool> {
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
                    .context("no file stored in params")?;
                msg.param.set(Param::File, blob.as_name());
                if blob.suffix() == Some(WEBXDC_SUFFIX) {
                    msg.viewtype = Viewtype::Webxdc;
                }
            }
        }

        let chat = Chat::load_from_db(context, self).await?;
        if let Some(cant_send_reason) = chat.why_cant_send(context).await? {
            bail!(
                "Can't set a draft because chat is not writeable: {}",
                cant_send_reason
            );
        }

        // set back draft information to allow identifying the draft later on -
        // no matter if message object is reused or reloaded from db
        msg.state = MessageState::OutDraft;
        msg.chat_id = self;

        // if possible, replace existing draft and keep id
        if !msg.id.is_special() {
            if let Some(old_draft) = self.get_draft(context).await? {
                if old_draft.id == msg.id
                    && old_draft.chat_id == self
                    && old_draft.state == MessageState::OutDraft
                {
                    context
                        .sql
                        .execute(
                            "UPDATE msgs
                            SET timestamp=?,type=?,txt=?, param=?,mime_in_reply_to=?
                            WHERE id=?;",
                            paramsv![
                                time(),
                                msg.viewtype,
                                msg.text.as_deref().unwrap_or(""),
                                msg.param.to_string(),
                                msg.in_reply_to.as_deref().unwrap_or_default(),
                                msg.id
                            ],
                        )
                        .await?;
                    return Ok(true);
                }
            }
        }

        // insert new draft
        self.maybe_delete_draft(context).await?;
        let row_id = context
            .sql
            .insert(
                "INSERT INTO msgs (
                 chat_id,
                 from_id,
                 timestamp,
                 type,
                 state,
                 txt,
                 param,
                 hidden,
                 mime_in_reply_to)
         VALUES (?,?,?, ?,?,?,?,?,?);",
                paramsv![
                    self,
                    ContactId::SELF,
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
        msg.id = MsgId::new(row_id.try_into()?);
        Ok(true)
    }

    /// Returns number of messages in a chat.
    pub async fn get_msg_cnt(self, context: &Context) -> Result<usize> {
        let count = context
            .sql
            .count(
                "SELECT COUNT(*) FROM msgs WHERE hidden=0 AND chat_id=?",
                paramsv![self],
            )
            .await?;
        Ok(count)
    }

    pub async fn get_fresh_msg_cnt(self, context: &Context) -> Result<usize> {
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
        let count = if self.is_archived_link() {
            context
                .sql
                .count(
                    "SELECT COUNT(DISTINCT(m.chat_id))
                    FROM msgs m
                    LEFT JOIN chats c ON m.chat_id=c.id
                    WHERE m.state=10
                    and m.hidden=0
                    AND m.chat_id>9
                    AND c.blocked=0
                    AND c.archived=1
                    ",
                    paramsv![],
                )
                .await?
        } else {
            context
                .sql
                .count(
                    "SELECT COUNT(*)
                FROM msgs
                WHERE state=?
                AND hidden=0
                AND chat_id=?;",
                    paramsv![MessageState::InFresh, self],
                )
                .await?
        };
        Ok(count)
    }

    pub(crate) async fn get_param(self, context: &Context) -> Result<Params> {
        let res: Option<String> = context
            .sql
            .query_get_value("SELECT param FROM chats WHERE id=?", paramsv![self])
            .await?;
        Ok(res
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default())
    }

    /// Returns true if the chat is not promoted.
    pub(crate) async fn is_unpromoted(self, context: &Context) -> Result<bool> {
        let param = self.get_param(context).await?;
        let unpromoted = param.get_bool(Param::Unpromoted).unwrap_or_default();
        Ok(unpromoted)
    }

    /// Returns true if the chat is promoted.
    pub(crate) async fn is_promoted(self, context: &Context) -> Result<bool> {
        let promoted = !self.is_unpromoted(context).await?;
        Ok(promoted)
    }

    // Returns true if chat is a saved messages chat.
    pub async fn is_self_talk(self, context: &Context) -> Result<bool> {
        Ok(self.get_param(context).await?.exists(Param::Selftalk))
    }

    /// Returns true if chat is a device chat.
    pub async fn is_device_talk(self, context: &Context) -> Result<bool> {
        Ok(self.get_param(context).await?.exists(Param::Devicetalk))
    }

    async fn parent_query<T, F>(self, context: &Context, fields: &str, f: F) -> Result<Option<T>>
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
        let row = sql
            .query_row_optional(
                &query,
                paramsv![
                    self,
                    MessageState::OutPreparing,
                    MessageState::OutDraft,
                    MessageState::OutPending,
                    MessageState::OutFailed
                ],
                f,
            )
            .await?;
        Ok(row)
    }

    async fn get_parent_mime_headers(
        self,
        context: &Context,
    ) -> Result<Option<(String, String, String)>> {
        if let Some((rfc724_mid, mime_in_reply_to, mime_references, error)) = self
            .parent_query(
                context,
                "rfc724_mid, mime_in_reply_to, mime_references, error",
                |row: &rusqlite::Row| {
                    let rfc724_mid: String = row.get(0)?;
                    let mime_in_reply_to: String = row.get(1)?;
                    let mime_references: String = row.get(2)?;
                    let error: String = row.get(3)?;
                    Ok((rfc724_mid, mime_in_reply_to, mime_references, error))
                },
            )
            .await?
        {
            if !error.is_empty() {
                // Do not reply to error messages.
                //
                // An error message could be a group chat message that we failed to decrypt and
                // assigned to 1:1 chat. A reply to it will show up as a reply to group message
                // on the other side. To avoid such situations, it is better not to reply to
                // error messages at all.
                Ok(None)
            } else {
                Ok(Some((rfc724_mid, mime_in_reply_to, mime_references)))
            }
        } else {
            Ok(None)
        }
    }

    /// Returns multi-line text summary of encryption preferences of all chat contacts.
    ///
    /// This can be used to find out if encryption is not available because
    /// keys for some users are missing or simply because the majority of the users in a group
    /// prefer plaintext emails.
    ///
    /// To get more verbose summary for a contact, including its key fingerprint, use [`Contact::get_encrinfo`].
    pub async fn get_encryption_info(self, context: &Context) -> Result<String> {
        let mut ret_mutual = String::new();
        let mut ret_nopreference = String::new();
        let mut ret_reset = String::new();

        for contact_id in get_chat_contacts(context, self)
            .await?
            .iter()
            .filter(|&contact_id| !contact_id.is_special())
        {
            let contact = Contact::load_from_db(context, *contact_id).await?;
            let addr = contact.get_addr();
            let peerstate = Peerstate::from_addr(context, addr).await?;

            match peerstate
                .filter(|peerstate| {
                    peerstate
                        .peek_key(PeerstateVerifiedStatus::Unverified)
                        .is_some()
                })
                .map(|peerstate| peerstate.prefer_encrypt)
            {
                Some(EncryptPreference::Mutual) => ret_mutual += &format!("{}\n", addr),
                Some(EncryptPreference::NoPreference) => ret_nopreference += &format!("{}\n", addr),
                Some(EncryptPreference::Reset) | None => ret_reset += &format!("{}\n", addr),
            };
        }

        let mut ret = String::new();
        if !ret_reset.is_empty() {
            ret += &stock_str::encr_none(context).await;
            ret.push(':');
            ret.push('\n');
            ret += &ret_reset;
        }
        if !ret_nopreference.is_empty() {
            if !ret.is_empty() {
                ret.push('\n');
            }
            ret += &stock_str::e2e_available(context).await;
            ret.push(':');
            ret.push('\n');
            ret += &ret_nopreference;
        }
        if !ret_mutual.is_empty() {
            if !ret.is_empty() {
                ret.push('\n');
            }
            ret += &stock_str::e2e_preferred(context).await;
            ret.push(':');
            ret.push('\n');
            ret += &ret_mutual;
        }

        Ok(ret.trim().to_string())
    }

    /// Bad evil escape hatch.
    ///
    /// Avoid using this, eventually types should be cleaned up enough
    /// that it is no longer necessary.
    pub fn to_u32(self) -> u32 {
        self.0
    }

    pub(crate) async fn reset_gossiped_timestamp(self, context: &Context) -> Result<()> {
        self.set_gossiped_timestamp(context, 0).await
    }

    /// Get timestamp of the last gossip sent in the chat.
    /// Zero return value means that gossip was never sent.
    pub async fn get_gossiped_timestamp(self, context: &Context) -> Result<i64> {
        let timestamp: Option<i64> = context
            .sql
            .query_get_value(
                "SELECT gossiped_timestamp FROM chats WHERE id=?;",
                paramsv![self],
            )
            .await?;
        Ok(timestamp.unwrap_or_default())
    }

    pub(crate) async fn set_gossiped_timestamp(
        self,
        context: &Context,
        timestamp: i64,
    ) -> Result<()> {
        ensure!(
            !self.is_special(),
            "can not set gossiped timestamp for special chats"
        );
        info!(
            context,
            "set gossiped_timestamp for chat {} to {}.", self, timestamp,
        );

        context
            .sql
            .execute(
                "UPDATE chats SET gossiped_timestamp=? WHERE id=?;",
                paramsv![timestamp, self],
            )
            .await?;

        Ok(())
    }
}

impl std::fmt::Display for ChatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_trash() {
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
        let val = rusqlite::types::Value::Integer(i64::from(self.0));
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

/// Allow converting an SQLite integer directly into [ChatId].
impl rusqlite::types::FromSql for ChatId {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).and_then(|val| {
            if 0 <= val && val <= i64::from(std::u32::MAX) {
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
    pub(crate) blocked: Blocked,
    pub param: Params,
    is_sending_locations: bool,
    pub mute_duration: MuteDuration,
    protected: ProtectionStatus,
}

impl Chat {
    /// Loads chat from the database by its ID.
    pub async fn load_from_db(context: &Context, chat_id: ChatId) -> Result<Self> {
        let mut chat = context
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
            .await
            .context(format!("Failed loading chat {} from database", chat_id))?;

        if chat.id.is_archived_link() {
            chat.name = stock_str::archived_chats(context).await;
        } else {
            if chat.typ == Chattype::Single && chat.name.is_empty() {
                // chat.name is set to contact.display_name on changes,
                // however, if things went wrong somehow, we do this here explicitly.
                let mut chat_name = "Err [Name not found]".to_owned();
                match get_chat_contacts(context, chat.id).await {
                    Ok(contacts) => {
                        if let Some(contact_id) = contacts.first() {
                            if let Ok(contact) = Contact::get_by_id(context, *contact_id).await {
                                chat_name = contact.get_display_name().to_owned();
                            }
                        }
                    }
                    Err(err) => {
                        error!(context, "faild to load contacts for {}: {:#}", chat.id, err);
                    }
                }
                chat.name = chat_name;
            }
            if chat.param.exists(Param::Selftalk) {
                chat.name = stock_str::saved_messages(context).await;
            } else if chat.param.exists(Param::Devicetalk) {
                chat.name = stock_str::device_messages(context).await;
            }
        }

        Ok(chat)
    }

    pub fn is_self_talk(&self) -> bool {
        self.param.exists(Param::Selftalk)
    }

    /// Returns true if chat is a device chat.
    pub fn is_device_talk(&self) -> bool {
        self.param.exists(Param::Devicetalk)
    }

    pub fn is_mailing_list(&self) -> bool {
        self.typ == Chattype::Mailinglist
    }

    /// Returns None if user can send messages to this chat.
    ///
    /// Otherwise returns a reason useful for logging.
    pub(crate) async fn why_cant_send(&self, context: &Context) -> Result<Option<CantSendReason>> {
        use CantSendReason::*;

        let reason = if self.id.is_special() {
            Some(SpecialChat)
        } else if self.is_device_talk() {
            Some(DeviceChat)
        } else if self.is_contact_request() {
            Some(ContactRequest)
        } else if self.is_mailing_list() && self.param.get(Param::ListPost).is_none_or_empty() {
            Some(ReadOnlyMailingList)
        } else if !self.is_self_in_chat(context).await? {
            Some(NotAMember)
        } else {
            None
        };
        Ok(reason)
    }

    /// Returns true if can send to the chat.
    ///
    /// This function can be used by the UI to decide whether to display the input box.
    pub async fn can_send(&self, context: &Context) -> Result<bool> {
        Ok(self.why_cant_send(context).await?.is_none())
    }

    /// Checks if the user is part of a chat
    /// and has basically the permissions to edit the chat therefore.
    /// The function does not check if the chat type allows editing of concrete elements.
    pub(crate) async fn is_self_in_chat(&self, context: &Context) -> Result<bool> {
        match self.typ {
            Chattype::Single | Chattype::Broadcast | Chattype::Mailinglist => Ok(true),
            Chattype::Group => is_contact_in_chat(context, self.id, ContactId::SELF).await,
            Chattype::Undefined => Ok(false),
        }
    }

    pub async fn update_param(&mut self, context: &Context) -> Result<()> {
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

    /// Returns mailing list address where messages are sent to.
    pub fn get_mailinglist_addr(&self) -> Option<&str> {
        self.param.get(Param::ListPost)
    }

    /// Returns profile image path for the chat.
    pub async fn get_profile_image(&self, context: &Context) -> Result<Option<PathBuf>> {
        if let Some(image_rel) = self.param.get(Param::ProfileImage) {
            if !image_rel.is_empty() {
                return Ok(Some(get_abs_path(context, image_rel)));
            }
        } else if self.id.is_archived_link() {
            if let Ok(image_rel) = get_archive_icon(context).await {
                return Ok(Some(get_abs_path(context, image_rel)));
            }
        } else if self.typ == Chattype::Single {
            let contacts = get_chat_contacts(context, self.id).await?;
            if let Some(contact_id) = contacts.first() {
                if let Ok(contact) = Contact::get_by_id(context, *contact_id).await {
                    return contact.get_profile_image(context).await;
                }
            }
        } else if self.typ == Chattype::Broadcast {
            if let Ok(image_rel) = get_broadcast_icon(context).await {
                return Ok(Some(get_abs_path(context, image_rel)));
            }
        }
        Ok(None)
    }

    pub async fn get_color(&self, context: &Context) -> Result<u32> {
        let mut color = 0;

        if self.typ == Chattype::Single {
            let contacts = get_chat_contacts(context, self.id).await?;
            if let Some(contact_id) = contacts.first() {
                if let Ok(contact) = Contact::get_by_id(context, *contact_id).await {
                    color = contact.get_color();
                }
            }
        } else {
            color = str_to_color(&self.name);
        }

        Ok(color)
    }

    /// Returns a struct describing the current state of the chat.
    ///
    /// This is somewhat experimental, even more so than the rest of
    /// deltachat, and the data returned is still subject to change.
    pub async fn get_info(&self, context: &Context) -> Result<ChatInfo> {
        let draft = match self.id.get_draft(context).await? {
            Some(message) => message.text.unwrap_or_default(),
            _ => String::new(),
        };
        Ok(ChatInfo {
            id: self.id,
            type_: self.typ as u32,
            name: self.name.clone(),
            archived: self.visibility == ChatVisibility::Archived,
            param: self.param.to_string(),
            gossiped_timestamp: self.id.get_gossiped_timestamp(context).await?,
            is_sending_locations: self.is_sending_locations,
            color: self.get_color(context).await?,
            profile_image: self
                .get_profile_image(context)
                .await?
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

    /// Returns true if chat is a contact request.
    ///
    /// Messages cannot be sent to such chat and read receipts are not
    /// sent until the chat is manually unblocked.
    pub fn is_contact_request(&self) -> bool {
        self.blocked == Blocked::Request
    }

    pub fn is_unpromoted(&self) -> bool {
        self.param.get_bool(Param::Unpromoted).unwrap_or_default()
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

    /// Adds missing values to the msg object,
    /// writes the record to the database and returns its msg_id.
    ///
    /// If `update_msg_id` is set, that record is reused;
    /// if `update_msg_id` is None, a new record is created.
    async fn prepare_msg_raw(
        &mut self,
        context: &Context,
        msg: &mut Message,
        update_msg_id: Option<MsgId>,
        timestamp: i64,
    ) -> Result<MsgId> {
        let mut new_references = "".into();
        let mut to_id = 0;
        let mut location_id = 0;

        if let Some(reason) = self.why_cant_send(context).await? {
            if self.typ == Chattype::Group && reason == CantSendReason::NotAMember {
                context.emit_event(EventType::ErrorSelfNotInGroup(
                    "Cannot send message; self not in group.".into(),
                ));
            }
            bail!("Cannot send message to {}: {}", self.id, reason);
        }

        let from = context.get_primary_self_addr().await?;
        let new_rfc724_mid = {
            let grpid = match self.typ {
                Chattype::Group => Some(self.grpid.as_str()),
                _ => None,
            };
            create_outgoing_rfc724_mid(grpid, &from)
        };

        if self.typ == Chattype::Single {
            if let Some(id) = context
                .sql
                .query_get_value(
                    "SELECT contact_id FROM chats_contacts WHERE chat_id=?;",
                    paramsv![self.id],
                )
                .await?
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
            // send_sync_msg() is called (usually) a moment later at send_msg_to_smtp()
            // when the group-creation message is actually sent though SMTP -
            // this makes sure, the other devices are aware of grpid that is used in the sync-message.
            context.sync_qr_code_tokens(Some(self.id)).await?;
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
                self.id.get_parent_mime_headers(context).await?
            {
                // "In-Reply-To:" is not changed if it is set manually.
                // This does not affect "References:" header, it will contain "default parent" (the
                // latest message in the thread) anyway.
                if msg.in_reply_to.is_none() && !parent_rfc724_mid.is_empty() {
                    msg.in_reply_to = Some(parent_rfc724_mid.clone());
                }

                // the whole list of messages referenced may be huge;
                // only use the oldest and the parent message
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
                } else {
                    // as a fallback, use our Message-ID, see reasoning below.
                    new_references = new_rfc724_mid.clone();
                }
            } else {
                // this is a top-level message, add our Message-ID as first reference.
                // as we always try to extract the grpid also from `References:`-header,
                // this allows group conversations also if smtp-server as outlook change `Message-ID:`-header
                // (MUAs usually keep the first Message-ID in `References:`-header unchanged).
                new_references = new_rfc724_mid.clone();
            }
        }

        // add independent location to database

        if msg.param.exists(Param::SetLatitude) {
            if let Ok(row_id) = context
                .sql
                .insert(
                    "INSERT INTO locations \
                     (timestamp,from_id,chat_id, latitude,longitude,independent)\
                     VALUES (?,?,?, ?,?,1);",
                    paramsv![
                        timestamp,
                        ContactId::SELF,
                        self.id,
                        msg.param.get_float(Param::SetLatitude).unwrap_or_default(),
                        msg.param.get_float(Param::SetLongitude).unwrap_or_default(),
                    ],
                )
                .await
            {
                location_id = row_id;
            }
        }

        let ephemeral_timer = if msg.param.get_cmd() == SystemMessage::EphemeralTimerChanged {
            EphemeralTimer::Disabled
        } else {
            self.id.get_ephemeral_timer(context).await?
        };
        let ephemeral_timestamp = match ephemeral_timer {
            EphemeralTimer::Disabled => 0,
            EphemeralTimer::Enabled { duration } => time().saturating_add(duration.into()),
        };

        let new_mime_headers = if msg.has_html() {
            let html = if msg.param.exists(Param::Forwarded) {
                msg.get_id().get_html(context).await?
            } else {
                msg.param.get(Param::SendHtml).map(|s| s.to_string())
            };
            html.map(|html| new_html_mimepart(html).build().as_string())
        } else {
            None
        };

        // add message to the database

        if let Some(update_msg_id) = update_msg_id {
            context
                .sql
                .execute(
                    "UPDATE msgs
                     SET rfc724_mid=?, chat_id=?, from_id=?, to_id=?, timestamp=?, type=?,
                         state=?, txt=?, subject=?, param=?,
                         hidden=?, mime_in_reply_to=?, mime_references=?, mime_modified=?,
                         mime_headers=?, location_id=?, ephemeral_timer=?, ephemeral_timestamp=?
                     WHERE id=?;",
                    paramsv![
                        new_rfc724_mid,
                        self.id,
                        ContactId::SELF,
                        to_id,
                        timestamp,
                        msg.viewtype,
                        msg.state,
                        msg.text.as_ref().cloned().unwrap_or_default(),
                        &msg.subject,
                        msg.param.to_string(),
                        msg.hidden,
                        msg.in_reply_to.as_deref().unwrap_or_default(),
                        new_references,
                        new_mime_headers.is_some(),
                        new_mime_headers.unwrap_or_default(),
                        location_id as i32,
                        ephemeral_timer,
                        ephemeral_timestamp,
                        update_msg_id
                    ],
                )
                .await?;
            msg.id = update_msg_id;
        } else {
            let raw_id = context
                .sql
                .insert(
                    "INSERT INTO msgs (
                        rfc724_mid,
                        chat_id,
                        from_id,
                        to_id,
                        timestamp,
                        type,
                        state,
                        txt,
                        subject,
                        param,
                        hidden,
                        mime_in_reply_to,
                        mime_references,
                        mime_modified,
                        mime_headers,
                        location_id,
                        ephemeral_timer,
                        ephemeral_timestamp)
                        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?);",
                    paramsv![
                        new_rfc724_mid,
                        self.id,
                        ContactId::SELF,
                        to_id,
                        timestamp,
                        msg.viewtype,
                        msg.state,
                        msg.text.as_ref().cloned().unwrap_or_default(),
                        &msg.subject,
                        msg.param.to_string(),
                        msg.hidden,
                        msg.in_reply_to.as_deref().unwrap_or_default(),
                        new_references,
                        new_mime_headers.is_some(),
                        new_mime_headers.unwrap_or_default(),
                        location_id as i32,
                        ephemeral_timer,
                        ephemeral_timestamp
                    ],
                )
                .await?;
            msg.id = MsgId::new(u32::try_from(raw_id)?);
        }
        context.interrupt_ephemeral_task().await;
        Ok(msg.id)
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
                // fallback to Normal for unknown values, may happen eg. on imports created by a newer version.
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
    /// This is the string-serialised version of `Params` currently.
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
    // - [ ] summary,
    // - [ ] lastUpdated,
    // - [ ] freshMessageCounter,
    // - [ ] email
}

pub(crate) async fn update_saved_messages_icon(context: &Context) -> Result<()> {
    // if there is no saved-messages chat, there is nothing to update. this is no error.
    if let Some(chat_id) = ChatId::lookup_by_contact(context, ContactId::SELF).await? {
        let icon = include_bytes!("../assets/icon-saved-messages.png");
        let blob = BlobObject::create(context, "icon-saved-messages.png", icon).await?;
        let icon = blob.as_name().to_string();

        let mut chat = Chat::load_from_db(context, chat_id).await?;
        chat.param.set(Param::ProfileImage, icon);
        chat.update_param(context).await?;
    }
    Ok(())
}

pub(crate) async fn update_device_icon(context: &Context) -> Result<()> {
    // if there is no device-chat, there is nothing to update. this is no error.
    if let Some(chat_id) = ChatId::lookup_by_contact(context, ContactId::DEVICE).await? {
        let icon = include_bytes!("../assets/icon-device.png");
        let blob = BlobObject::create(context, "icon-device.png", icon).await?;
        let icon = blob.as_name().to_string();

        let mut chat = Chat::load_from_db(context, chat_id).await?;
        chat.param.set(Param::ProfileImage, &icon);
        chat.update_param(context).await?;

        let mut contact = Contact::load_from_db(context, ContactId::DEVICE).await?;
        contact.param.set(Param::ProfileImage, icon);
        contact.update_param(context).await?;
    }
    Ok(())
}

pub(crate) async fn get_broadcast_icon(context: &Context) -> Result<String> {
    if let Some(icon) = context.sql.get_raw_config("icon-broadcast").await? {
        return Ok(icon);
    }

    let icon = include_bytes!("../assets/icon-broadcast.png");
    let blob = BlobObject::create(context, "icon-broadcast.png", icon).await?;
    let icon = blob.as_name().to_string();
    context
        .sql
        .set_raw_config("icon-broadcast", Some(&icon))
        .await?;
    Ok(icon)
}

pub(crate) async fn get_archive_icon(context: &Context) -> Result<String> {
    if let Some(icon) = context.sql.get_raw_config("icon-archive").await? {
        return Ok(icon);
    }

    let icon = include_bytes!("../assets/icon-archive.png");
    let blob = BlobObject::create(context, "icon-archive.png", icon).await?;
    let icon = blob.as_name().to_string();
    context
        .sql
        .set_raw_config("icon-archive", Some(&icon))
        .await?;
    Ok(icon)
}

async fn update_special_chat_name(
    context: &Context,
    contact_id: ContactId,
    name: String,
) -> Result<()> {
    if let Some(chat_id) = ChatId::lookup_by_contact(context, contact_id).await? {
        // the `!= name` condition avoids unneeded writes
        context
            .sql
            .execute(
                "UPDATE chats SET name=? WHERE id=? AND name!=?",
                paramsv![name, chat_id, name],
            )
            .await?;
    }
    Ok(())
}

pub(crate) async fn update_special_chat_names(context: &Context) -> Result<()> {
    update_special_chat_name(
        context,
        ContactId::DEVICE,
        stock_str::device_messages(context).await,
    )
    .await?;
    update_special_chat_name(
        context,
        ContactId::SELF,
        stock_str::saved_messages(context).await,
    )
    .await?;
    Ok(())
}

/// Handle a [`ChatId`] and its [`Blocked`] status at once.
///
/// This struct is an optimisation to read a [`ChatId`] and its [`Blocked`] status at once
/// from the database.  It [`Deref`]s to [`ChatId`] so it can be used as an extension to
/// [`ChatId`].
///
/// [`Deref`]: std::ops::Deref
#[derive(Debug)]
pub(crate) struct ChatIdBlocked {
    pub id: ChatId,
    pub blocked: Blocked,
}

impl ChatIdBlocked {
    /// Searches the database for the 1:1 chat with this contact.
    ///
    /// If no chat is found `None` is returned.
    pub async fn lookup_by_contact(
        context: &Context,
        contact_id: ContactId,
    ) -> Result<Option<Self>> {
        ensure!(context.sql.is_open().await, "Database not available");
        ensure!(
            contact_id != ContactId::UNDEFINED,
            "Invalid contact id requested"
        );

        context
            .sql
            .query_row_optional(
                "SELECT c.id, c.blocked
                   FROM chats c
                  INNER JOIN chats_contacts j
                          ON c.id=j.chat_id
                  WHERE c.type=100  -- 100 = Chattype::Single
                    AND c.id>9      -- 9 = DC_CHAT_ID_LAST_SPECIAL
                    AND j.contact_id=?;",
                paramsv![contact_id],
                |row| {
                    let id: ChatId = row.get(0)?;
                    let blocked: Blocked = row.get(1)?;
                    Ok(ChatIdBlocked { id, blocked })
                },
            )
            .await
            .map_err(Into::into)
    }

    /// Returns the chat for the 1:1 chat with this contact.
    ///
    /// I the chat does not yet exist a new one is created, using the provided [`Blocked`]
    /// state.
    pub async fn get_for_contact(
        context: &Context,
        contact_id: ContactId,
        create_blocked: Blocked,
    ) -> Result<Self> {
        ensure!(context.sql.is_open().await, "Database not available");
        ensure!(
            contact_id != ContactId::UNDEFINED,
            "Invalid contact id requested"
        );

        if let Some(res) = Self::lookup_by_contact(context, contact_id).await? {
            // Already exists, no need to create.
            return Ok(res);
        }

        let contact = Contact::load_from_db(context, contact_id).await?;
        let chat_name = contact.get_display_name().to_string();
        let mut params = Params::new();
        match contact_id {
            ContactId::SELF => {
                params.set_int(Param::Selftalk, 1);
            }
            ContactId::DEVICE => {
                params.set_int(Param::Devicetalk, 1);
            }
            _ => (),
        }

        let created_timestamp = create_smeared_timestamp(context).await;
        let chat_id = context
            .sql
            .transaction(move |transaction| {
                transaction.execute(
                    "INSERT INTO chats
                     (type, name, param, blocked, created_timestamp)
                     VALUES(?, ?, ?, ?, ?)",
                    params![
                        Chattype::Single,
                        chat_name,
                        params.to_string(),
                        create_blocked as u8,
                        created_timestamp,
                    ],
                )?;
                let chat_id = ChatId::new(
                    transaction
                        .last_insert_rowid()
                        .try_into()
                        .context("chat table rowid overflows u32")?,
                );

                transaction.execute(
                    "INSERT INTO chats_contacts
                 (chat_id, contact_id)
                 VALUES((SELECT last_insert_rowid()), ?)",
                    params![contact_id],
                )?;

                Ok(chat_id)
            })
            .await?;

        match contact_id {
            ContactId::SELF => update_saved_messages_icon(context).await?,
            ContactId::DEVICE => update_device_icon(context).await?,
            _ => (),
        }

        Ok(Self {
            id: chat_id,
            blocked: create_blocked,
        })
    }
}

pub async fn prepare_msg(context: &Context, chat_id: ChatId, msg: &mut Message) -> Result<MsgId> {
    ensure!(
        !chat_id.is_special(),
        "Cannot prepare message for special chat"
    );

    let msg_id = prepare_msg_common(context, chat_id, msg, MessageState::OutPreparing).await?;
    context.emit_msgs_changed(msg.chat_id, msg.id);

    Ok(msg_id)
}

async fn prepare_msg_blob(context: &Context, msg: &mut Message) -> Result<()> {
    if msg.viewtype == Viewtype::Text || msg.viewtype == Viewtype::VideochatInvitation {
        // the caller should check if the message text is empty
    } else if msg.viewtype.has_file() {
        let blob = msg
            .param
            .get_blob(Param::File, context, !msg.is_increation())
            .await?
            .with_context(|| format!("attachment missing for message of type #{}", msg.viewtype))?;

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
                if better_type != Viewtype::Webxdc
                    || context
                        .ensure_sendable_webxdc_file(&blob.to_abs_path())
                        .await
                        .is_ok()
                {
                    msg.viewtype = better_type;
                    if !msg.param.exists(Param::MimeType) {
                        msg.param.set(Param::MimeType, better_mime);
                    }
                }
            }
        } else if msg.viewtype == Viewtype::Webxdc {
            context
                .ensure_sendable_webxdc_file(&blob.to_abs_path())
                .await?;
        }

        if !msg.param.exists(Param::MimeType) {
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
    change_state_to: MessageState,
) -> Result<MsgId> {
    let mut chat = Chat::load_from_db(context, chat_id).await?;
    if let Some(reason) = chat.why_cant_send(context).await? {
        bail!("cannot send to {}: {}", chat_id, reason);
    }

    // check current MessageState for drafts (to keep msg_id) ...
    let update_msg_id = if msg.state == MessageState::OutDraft {
        msg.hidden = false;
        if !msg.id.is_special() && msg.chat_id == chat_id {
            Some(msg.id)
        } else {
            None
        }
    } else {
        None
    };

    // ... then change the MessageState in the message object
    msg.state = change_state_to;

    prepare_msg_blob(context, msg).await?;
    chat_id.unarchive_if_not_muted(context).await?;
    msg.id = chat
        .prepare_msg_raw(
            context,
            msg,
            update_msg_id,
            create_smeared_timestamp(context).await,
        )
        .await?;
    msg.chat_id = chat_id;

    Ok(msg.id)
}

/// Returns whether a contact is in a chat or not.
pub async fn is_contact_in_chat(
    context: &Context,
    chat_id: ChatId,
    contact_id: ContactId,
) -> Result<bool> {
    // this function works for group and for normal chats, however, it is more useful
    // for group chats.
    // ContactId::SELF may be used to check, if the user itself is in a group
    // chat (ContactId::SELF is not added to normal chats)

    let exists = context
        .sql
        .exists(
            "SELECT COUNT(*) FROM chats_contacts WHERE chat_id=? AND contact_id=?;",
            paramsv![chat_id, contact_id],
        )
        .await?;
    Ok(exists)
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
pub async fn send_msg(context: &Context, chat_id: ChatId, msg: &mut Message) -> Result<MsgId> {
    if chat_id.is_unset() {
        let forwards = msg.param.get(Param::PrepForwards);
        if let Some(forwards) = forwards {
            for forward in forwards.split(' ') {
                if let Ok(msg_id) = forward.parse::<u32>().map(MsgId::new) {
                    if let Ok(mut msg) = Message::load_from_db(context, msg_id).await {
                        send_msg_inner(context, chat_id, &mut msg).await?;
                    };
                }
            }
            msg.param.remove(Param::PrepForwards);
            msg.update_param(context).await?;
        }
        return send_msg_inner(context, chat_id, msg).await;
    }

    send_msg_inner(context, chat_id, msg).await
}

/// Tries to send a message synchronously.
///
/// Creates a new message in `smtp` table, then drectly opens an SMTP connection and sends the
/// message. If this fails, the message remains in the database to be sent later.
pub async fn send_msg_sync(context: &Context, chat_id: ChatId, msg: &mut Message) -> Result<MsgId> {
    if let Some(rowid) = prepare_send_msg(context, chat_id, msg).await? {
        let mut smtp = crate::smtp::Smtp::new();
        send_msg_to_smtp(context, &mut smtp, rowid)
            .await
            .context("failed to send message, queued for later sending")?;

        context.emit_msgs_changed(msg.chat_id, msg.id);
    }
    Ok(msg.id)
}

async fn send_msg_inner(context: &Context, chat_id: ChatId, msg: &mut Message) -> Result<MsgId> {
    if prepare_send_msg(context, chat_id, msg).await?.is_some() {
        context.emit_msgs_changed(msg.chat_id, msg.id);

        if msg.param.exists(Param::SetLatitude) {
            context.emit_event(EventType::LocationChanged(Some(ContactId::SELF)));
        }

        context.interrupt_smtp(InterruptInfo::new(false)).await;
    }

    Ok(msg.id)
}

/// Returns rowid from `smtp` table.
async fn prepare_send_msg(
    context: &Context,
    chat_id: ChatId,
    msg: &mut Message,
) -> Result<Option<i64>> {
    // prepare_msg() leaves the message state to OutPreparing, we
    // only have to change the state to OutPending in this case.
    // Otherwise we still have to prepare the message, which will set
    // the state to OutPending.
    if msg.state != MessageState::OutPreparing {
        // automatically prepare normal messages
        prepare_msg_common(context, chat_id, msg, MessageState::OutPending).await?;
    } else {
        // update message state of separately prepared messages
        ensure!(
            chat_id.is_unset() || chat_id == msg.chat_id,
            "Inconsistent chat ID"
        );
        message::update_msg_state(context, msg.id, MessageState::OutPending).await?;
    }
    let row_id = create_send_msg_job(context, msg.id).await?;
    Ok(row_id)
}

/// Constructs a job for sending a message and inserts into `smtp` table.
///
/// Returns rowid if job was created or `None` if SMTP job is not needed, e.g. when sending to a
/// group with only self and no BCC-to-self configured.
///
/// The caller has to interrupt SMTP loop or otherwise process a new row.
async fn create_send_msg_job(context: &Context, msg_id: MsgId) -> Result<Option<i64>> {
    let mut msg = Message::load_from_db(context, msg_id).await?;
    msg.try_calc_and_set_dimensions(context)
        .await
        .context("failed to calculate media dimensions")?;

    /* create message */
    let needs_encryption = msg.param.get_bool(Param::GuaranteeE2ee).unwrap_or_default();

    let attach_selfavatar = match shall_attach_selfavatar(context, msg.chat_id).await {
        Ok(attach_selfavatar) => attach_selfavatar,
        Err(err) => {
            warn!(context, "job: cannot get selfavatar-state: {:#}", err);
            false
        }
    };

    let mimefactory = MimeFactory::from_msg(context, &msg, attach_selfavatar).await?;

    let mut recipients = mimefactory.recipients();

    let from = context.get_primary_self_addr().await?;
    let lowercase_from = from.to_lowercase();

    // Send BCC to self if it is enabled and we are not going to
    // delete it immediately.
    if context.get_config_bool(Config::BccSelf).await?
        && context.get_config_delete_server_after().await? != Some(0)
        && !recipients
            .iter()
            .any(|x| x.to_lowercase() == lowercase_from)
    {
        recipients.push(from);
    }

    if recipients.is_empty() {
        // may happen eg. for groups with only SELF and bcc_self disabled
        info!(
            context,
            "message {} has no recipient, skipping smtp-send", msg_id
        );
        msg_id.set_delivered(context).await?;
        return Ok(None);
    }

    let rendered_msg = match mimefactory.render(context).await {
        Ok(res) => Ok(res),
        Err(err) => {
            message::set_msg_failed(context, msg_id, &err.to_string()).await;
            Err(err)
        }
    }?;

    if needs_encryption && !rendered_msg.is_encrypted {
        /* unrecoverable */
        message::set_msg_failed(
            context,
            msg_id,
            "End-to-end-encryption unavailable unexpectedly.",
        )
        .await;
        bail!(
            "e2e encryption unavailable {} - {:?}",
            msg_id,
            needs_encryption
        );
    }

    if rendered_msg.is_gossiped {
        msg.chat_id.set_gossiped_timestamp(context, time()).await?;
    }

    if 0 != rendered_msg.last_added_location_id {
        if let Err(err) = location::set_kml_sent_timestamp(context, msg.chat_id, time()).await {
            error!(context, "Failed to set kml sent_timestamp: {:#}", err);
        }
        if !msg.hidden {
            if let Err(err) =
                location::set_msg_location_id(context, msg.id, rendered_msg.last_added_location_id)
                    .await
            {
                error!(context, "Failed to set msg_location_id: {:#}", err);
            }
        }
    }

    if let Some(sync_ids) = rendered_msg.sync_ids_to_delete {
        if let Err(err) = context.delete_sync_ids(sync_ids).await {
            error!(context, "Failed to delete sync ids: {:#}", err);
        }
    }

    if attach_selfavatar {
        if let Err(err) = msg.chat_id.set_selfavatar_timestamp(context, time()).await {
            error!(context, "Failed to set selfavatar timestamp: {:#}", err);
        }
    }

    if rendered_msg.is_encrypted && !needs_encryption {
        msg.param.set_int(Param::GuaranteeE2ee, 1);
        msg.update_param(context).await?;
    }

    ensure!(!recipients.is_empty(), "no recipients for smtp job set");

    let recipients = recipients.join(" ");

    msg.subject = rendered_msg.subject.clone();
    msg.update_subject(context).await?;

    let row_id = context
        .sql
        .insert(
            "INSERT INTO smtp (rfc724_mid, recipients, mime, msg_id)
             VALUES           (?1,         ?2,         ?3,   ?4)",
            paramsv![
                &rendered_msg.rfc724_mid,
                recipients,
                &rendered_msg.message,
                msg_id
            ],
        )
        .await?;
    Ok(Some(row_id))
}

pub async fn send_text_msg(
    context: &Context,
    chat_id: ChatId,
    text_to_send: String,
) -> Result<MsgId> {
    ensure!(
        !chat_id.is_special(),
        "bad chat_id, can not be a special chat: {}",
        chat_id
    );

    let mut msg = Message::new(Viewtype::Text);
    msg.text = Some(text_to_send);
    send_msg(context, chat_id, &mut msg).await
}

pub async fn send_videochat_invitation(context: &Context, chat_id: ChatId) -> Result<MsgId> {
    ensure!(
        !chat_id.is_special(),
        "video chat invitation cannot be sent to special chat: {}",
        chat_id
    );

    let instance = if let Some(instance) = context.get_config(Config::WebrtcInstance).await? {
        if !instance.is_empty() {
            instance
        } else {
            bail!("webrtc_instance is empty");
        }
    } else {
        bail!("webrtc_instance not set");
    };

    let instance = Message::create_webrtc_instance(&instance, &create_id());

    let mut msg = Message::new(Viewtype::VideochatInvitation);
    msg.param.set(Param::WebrtcRoom, &instance);
    msg.text = Some(
        stock_str::videochat_invite_msg_body(context, &Message::parse_webrtc_instance(&instance).1)
            .await,
    );
    send_msg(context, chat_id, &mut msg).await
}

pub async fn get_chat_msgs(
    context: &Context,
    chat_id: ChatId,
    flags: u32,
) -> Result<Vec<ChatItem>> {
    let process_row = if (flags & DC_GCM_INFO_ONLY) != 0 {
        |row: &rusqlite::Row| {
            // is_info logic taken from Message.is_info()
            let params = row.get::<_, String>("param")?;
            let (from_id, to_id) = (
                row.get::<_, ContactId>("from_id")?,
                row.get::<_, ContactId>("to_id")?,
            );
            let is_info_msg: bool = from_id == ContactId::INFO
                || to_id == ContactId::INFO
                || match Params::from_str(&params) {
                    Ok(p) => {
                        let cmd = p.get_cmd();
                        cmd != SystemMessage::Unknown && cmd != SystemMessage::AutocryptSetupMessage
                    }
                    _ => false,
                };

            Ok((
                row.get::<_, i64>("timestamp")?,
                row.get::<_, MsgId>("id")?,
                !is_info_msg,
            ))
        }
    } else {
        |row: &rusqlite::Row| {
            Ok((
                row.get::<_, i64>("timestamp")?,
                row.get::<_, MsgId>("id")?,
                false,
            ))
        }
    };
    let process_rows = |rows: rusqlite::MappedRows<_>| {
        // It is faster to sort here rather than
        // let sqlite execute an ORDER BY clause.
        let mut sorted_rows = Vec::new();
        for row in rows {
            let (ts, curr_id, exclude_message): (i64, MsgId, bool) = row?;
            if !exclude_message {
                sorted_rows.push((ts, curr_id));
            }
        }
        sorted_rows.sort_unstable();

        let mut ret = Vec::new();
        let mut last_day = 0;
        let cnv_to_local = gm2local_offset();

        for (ts, curr_id) in sorted_rows {
            if (flags & DC_GCM_ADDDAYMARKER) != 0 {
                let curr_local_timestamp = ts + cnv_to_local;
                let curr_day = curr_local_timestamp / 86400;
                if curr_day != last_day {
                    ret.push(ChatItem::DayMarker {
                        timestamp: curr_day * 86400, // Convert day back to Unix timestamp
                    });
                    last_day = curr_day;
                }
            }
            ret.push(ChatItem::Message { msg_id: curr_id });
        }
        Ok(ret)
    };

    let items = if (flags & DC_GCM_INFO_ONLY) != 0 {
        context
            .sql
            .query_map(
        // GLOB is used here instead of LIKE becase it is case-sensitive
                "SELECT m.id AS id, m.timestamp AS timestamp, m.param AS param, m.from_id AS from_id, m.to_id AS to_id
               FROM msgs m
              WHERE m.chat_id=?
                AND m.hidden=0
                AND (
                    m.param GLOB \"*S=*\"
                    OR m.from_id == ?
                    OR m.to_id == ?
                );",
                paramsv![chat_id, ContactId::INFO, ContactId::INFO],
                process_row,
                process_rows,
            )
            .await?
    } else {
        context
            .sql
            .query_map(
                "SELECT m.id AS id, m.timestamp AS timestamp
               FROM msgs m
              WHERE m.chat_id=?
                AND m.hidden=0;",
                paramsv![chat_id],
                process_row,
                process_rows,
            )
            .await?
    };
    Ok(items)
}

pub(crate) async fn marknoticed_chat_if_older_than(
    context: &Context,
    chat_id: ChatId,
    timestamp: i64,
) -> Result<()> {
    if let Some(chat_timestamp) = context
        .sql
        .query_get_value(
            "SELECT MAX(timestamp) FROM msgs WHERE chat_id=?",
            paramsv![chat_id],
        )
        .await?
    {
        if timestamp > chat_timestamp {
            marknoticed_chat(context, chat_id).await?;
        }
    }
    Ok(())
}

/// Marks all messages in the chat as noticed.
/// If the given chat-id is the archive-link, marks all messages in all archived chats as noticed.
pub async fn marknoticed_chat(context: &Context, chat_id: ChatId) -> Result<()> {
    // "WHERE" below uses the index `(state, hidden, chat_id)`, see get_fresh_msg_cnt() for reasoning
    // the additional SELECT statement may speed up things as no write-blocking is needed.
    if chat_id.is_archived_link() {
        let chat_ids_in_archive = context
            .sql
            .query_map(
                "SELECT DISTINCT(m.chat_id) FROM msgs m
                    LEFT JOIN chats c ON m.chat_id=c.id
                    WHERE m.state=10 AND m.hidden=0 AND m.chat_id>9 AND c.blocked=0 AND c.archived=1",
                paramsv![],
                |row| row.get::<_, ChatId>(0),
                |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            )
            .await?;
        if chat_ids_in_archive.is_empty() {
            return Ok(());
        }

        context
            .sql
            .execute(
                &format!(
                    "UPDATE msgs SET state=13 WHERE state=10 AND hidden=0 AND chat_id IN ({});",
                    sql::repeat_vars(chat_ids_in_archive.len())
                ),
                rusqlite::params_from_iter(&chat_ids_in_archive),
            )
            .await?;
        for chat_id_in_archive in chat_ids_in_archive {
            context.emit_event(EventType::MsgsNoticed(chat_id_in_archive));
        }
    } else {
        let exists = context
            .sql
            .exists(
                "SELECT COUNT(*) FROM msgs WHERE state=? AND hidden=0 AND chat_id=?;",
                paramsv![MessageState::InFresh, chat_id],
            )
            .await?;
        if !exists {
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
    }

    context.emit_event(EventType::MsgsNoticed(chat_id));

    Ok(())
}

/// Marks messages preceding outgoing messages as noticed.
///
/// In a chat, if there is an outgoing message, it can be assumed that all previous
/// messages were noticed. So, this function takes a Vec of messages that were
/// just received, and for all the outgoing messages, it marks all
/// previous messages as noticed.
pub(crate) async fn mark_old_messages_as_noticed(
    context: &Context,
    mut msgs: Vec<ReceivedMsg>,
) -> Result<()> {
    msgs.retain(|m| m.state.is_outgoing());
    if msgs.is_empty() {
        return Ok(());
    }

    let mut msgs_by_chat: HashMap<ChatId, ReceivedMsg> = HashMap::new();
    for msg in msgs {
        let chat_id = msg.chat_id;
        if let Some(existing_msg) = msgs_by_chat.get(&chat_id) {
            if msg.sort_timestamp > existing_msg.sort_timestamp {
                msgs_by_chat.insert(chat_id, msg);
            }
        } else {
            msgs_by_chat.insert(chat_id, msg);
        }
    }

    let changed_chats = context
        .sql
        .transaction(|transaction| {
            let mut changed_chats = Vec::new();
            for (_, msg) in msgs_by_chat {
                let changed_rows = transaction.execute(
                    "UPDATE msgs
            SET state=?
          WHERE state=?
            AND hidden=0
            AND chat_id=?
            AND timestamp<=?;",
                    paramsv![
                        MessageState::InNoticed,
                        MessageState::InFresh,
                        msg.chat_id,
                        msg.sort_timestamp
                    ],
                )?;
                if changed_rows > 0 {
                    changed_chats.push(msg.chat_id);
                }
            }
            Ok(changed_chats)
        })
        .await?;

    if !changed_chats.is_empty() {
        info!(
            context,
            "Marking chats as noticed because there are newer outgoing messages: {:?}",
            changed_chats
        );
    }

    for c in changed_chats {
        context.emit_event(EventType::MsgsNoticed(c));
    }

    Ok(())
}

pub async fn get_chat_media(
    context: &Context,
    chat_id: Option<ChatId>,
    msg_type: Viewtype,
    msg_type2: Viewtype,
    msg_type3: Viewtype,
) -> Result<Vec<MsgId>> {
    // TODO This query could/should be converted to `AND type IN (?, ?, ?)`.
    let list = context
        .sql
        .query_map(
            "SELECT id
               FROM msgs
              WHERE (1=? OR chat_id=?)
                AND (type=? OR type=? OR type=?)
                AND hidden=0
              ORDER BY timestamp, id;",
            paramsv![
                chat_id.is_none(),
                chat_id.unwrap_or_else(|| ChatId::new(0)),
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
            |ids| Ok(ids.flatten().collect()),
        )
        .await?;
    Ok(list)
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
) -> Result<Option<MsgId>> {
    let mut ret: Option<MsgId> = None;

    if let Ok(msg) = Message::load_from_db(context, curr_msg_id).await {
        let list: Vec<MsgId> = get_chat_media(
            context,
            Some(msg.chat_id),
            if msg_type != Viewtype::Unknown {
                msg_type
            } else {
                msg.viewtype
            },
            msg_type2,
            msg_type3,
        )
        .await?;
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
    Ok(ret)
}

/// Returns a vector of contact IDs for given chat ID.
pub async fn get_chat_contacts(context: &Context, chat_id: ChatId) -> Result<Vec<ContactId>> {
    // Normal chats do not include SELF.  Group chats do (as it may happen that one is deleted from a
    // groupchat but the chats stays visible, moreover, this makes displaying lists easier)

    let list = context
        .sql
        .query_map(
            "SELECT cc.contact_id
               FROM chats_contacts cc
               LEFT JOIN contacts c
                      ON c.id=cc.contact_id
              WHERE cc.chat_id=?
              ORDER BY c.id=1, c.last_seen DESC, c.id DESC;",
            paramsv![chat_id],
            |row| row.get::<_, ContactId>(0),
            |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into),
        )
        .await?;

    Ok(list)
}

/// Creates a group chat with a given `name`.
pub async fn create_group_chat(
    context: &Context,
    protect: ProtectionStatus,
    chat_name: &str,
) -> Result<ChatId> {
    let chat_name = improve_single_line_input(chat_name);
    ensure!(!chat_name.is_empty(), "Invalid chat name");

    let grpid = create_id();

    let row_id = context
        .sql
        .insert(
            "INSERT INTO chats
        (type, name, grpid, param, created_timestamp)
        VALUES(?, ?, ?, \'U=1\', ?);",
            paramsv![
                Chattype::Group,
                chat_name,
                grpid,
                create_smeared_timestamp(context).await,
            ],
        )
        .await?;

    let chat_id = ChatId::new(u32::try_from(row_id)?);
    if !is_contact_in_chat(context, chat_id, ContactId::SELF).await? {
        add_to_chat_contacts_table(context, chat_id, &[ContactId::SELF]).await?;
    }

    context.emit_msgs_changed_without_ids();

    if protect == ProtectionStatus::Protected {
        // this part is to stay compatible to verified groups,
        // in some future, we will drop the "protect"-flag from create_group_chat()
        chat_id.inner_set_protection(context, protect).await?;
    }

    Ok(chat_id)
}

/// Finds an unused name for a new broadcast list.
async fn find_unused_broadcast_list_name(context: &Context) -> Result<String> {
    let base_name = stock_str::broadcast_list(context).await;
    for attempt in 1..1000 {
        let better_name = if attempt > 1 {
            format!("{} {}", base_name, attempt)
        } else {
            base_name.clone()
        };
        if !context
            .sql
            .exists(
                "SELECT COUNT(*) FROM chats WHERE type=? AND name=?;",
                paramsv![Chattype::Broadcast, better_name],
            )
            .await?
        {
            return Ok(better_name);
        }
    }
    Ok(base_name)
}

/// Creates a new broadcast list.
pub async fn create_broadcast_list(context: &Context) -> Result<ChatId> {
    let chat_name = find_unused_broadcast_list_name(context).await?;
    let grpid = create_id();
    let row_id = context
        .sql
        .insert(
            "INSERT INTO chats
        (type, name, grpid, param, created_timestamp)
        VALUES(?, ?, ?, \'U=1\', ?);",
            paramsv![
                Chattype::Broadcast,
                chat_name,
                grpid,
                create_smeared_timestamp(context).await,
            ],
        )
        .await?;
    let chat_id = ChatId::new(u32::try_from(row_id)?);

    context.emit_msgs_changed_without_ids();
    Ok(chat_id)
}

/// Adds contacts to the `chats_contacts` table.
pub(crate) async fn add_to_chat_contacts_table(
    context: &Context,
    chat_id: ChatId,
    contact_ids: &[ContactId],
) -> Result<()> {
    context
        .sql
        .transaction(move |transaction| {
            for contact_id in contact_ids {
                transaction.execute(
                    "INSERT OR IGNORE INTO chats_contacts (chat_id, contact_id) VALUES(?, ?)",
                    paramsv![chat_id, contact_id],
                )?;
            }
            Ok(())
        })
        .await?;

    Ok(())
}

/// remove a contact from the chats_contact table
pub(crate) async fn remove_from_chat_contacts_table(
    context: &Context,
    chat_id: ChatId,
    contact_id: ContactId,
) -> Result<()> {
    context
        .sql
        .execute(
            "DELETE FROM chats_contacts WHERE chat_id=? AND contact_id=?",
            paramsv![chat_id, contact_id],
        )
        .await?;
    Ok(())
}

/// Adds a contact to the chat.
pub async fn add_contact_to_chat(
    context: &Context,
    chat_id: ChatId,
    contact_id: ContactId,
) -> Result<()> {
    add_contact_to_chat_ex(context, chat_id, contact_id, false).await?;
    Ok(())
}

pub(crate) async fn add_contact_to_chat_ex(
    context: &Context,
    chat_id: ChatId,
    contact_id: ContactId,
    from_handshake: bool,
) -> Result<bool> {
    ensure!(!chat_id.is_special(), "can not add member to special chats");
    let contact = Contact::get_by_id(context, contact_id).await?;
    let mut msg = Message::default();

    chat_id.reset_gossiped_timestamp(context).await?;

    /*this also makes sure, not contacts are added to special or normal chats*/
    let mut chat = Chat::load_from_db(context, chat_id).await?;
    ensure!(
        chat.typ == Chattype::Group || chat.typ == Chattype::Broadcast,
        "{} is not a group/broadcast where one can add members",
        chat_id
    );
    ensure!(
        Contact::real_exists_by_id(context, contact_id).await? || contact_id == ContactId::SELF,
        "invalid contact_id {} for adding to group",
        contact_id
    );
    ensure!(!chat.is_mailing_list(), "Mailing lists can't be changed");
    ensure!(
        chat.typ != Chattype::Broadcast || contact_id != ContactId::SELF,
        "Cannot add SELF to broadcast."
    );

    if !chat.is_self_in_chat(context).await? {
        context.emit_event(EventType::ErrorSelfNotInGroup(
            "Cannot add contact to group; self not in group.".into(),
        ));
        bail!("can not add contact because our account is not part of it");
    }

    if from_handshake && chat.param.get_int(Param::Unpromoted).unwrap_or_default() == 1 {
        chat.param.remove(Param::Unpromoted);
        chat.update_param(context).await?;
        context.sync_qr_code_tokens(Some(chat_id)).await?;
        context.send_sync_msg().await?;
    }

    if context.is_self_addr(contact.get_addr()).await? {
        // ourself is added using ContactId::SELF, do not add this address explicitly.
        // if SELF is not in the group, members cannot be added at all.
        warn!(
            context,
            "invalid attempt to add self e-mail address to group"
        );
        return Ok(false);
    }

    if is_contact_in_chat(context, chat_id, contact_id).await? {
        if !from_handshake {
            return Ok(true);
        }
    } else {
        // else continue and send status mail
        if chat.is_protected()
            && contact.is_verified(context).await? != VerifiedStatus::BidirectVerified
        {
            error!(
                context,
                "Only bidirectional verified contacts can be added to protected chats."
            );
            return Ok(false);
        }
        if is_contact_in_chat(context, chat_id, contact_id).await? {
            return Ok(false);
        }
        add_to_chat_contacts_table(context, chat_id, &[contact_id]).await?;
    }
    if chat.typ == Chattype::Group && chat.is_promoted() {
        msg.viewtype = Viewtype::Text;

        msg.text =
            Some(stock_str::msg_add_member(context, contact.get_addr(), ContactId::SELF).await);
        msg.param.set_cmd(SystemMessage::MemberAddedToGroup);
        msg.param.set(Param::Arg, contact.get_addr());
        msg.param.set_int(Param::Arg2, from_handshake.into());
        msg.id = send_msg(context, chat_id, &mut msg).await?;
    }
    context.emit_event(EventType::ChatModified(chat_id));
    Ok(true)
}

pub(crate) async fn shall_attach_selfavatar(context: &Context, chat_id: ChatId) -> Result<bool> {
    // versions before 12/2019 already allowed to set selfavatar, however, it was never sent to others.
    // to avoid sending out previously set selfavatars unexpectedly we added this additional check.
    // it can be removed after some time.
    if !context.sql.get_raw_config_bool("attach_selfavatar").await? {
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
            paramsv![chat_id, ContactId::SELF],
            |row| Ok(row.get::<_, i64>(0)),
            |rows| {
                let mut needs_attach = false;
                for row in rows {
                    let row = row?;
                    let selfavatar_sent = row?;
                    if selfavatar_sent < timestamp_some_days_ago {
                        needs_attach = true;
                    }
                }
                Ok(needs_attach)
            },
        )
        .await?;
    Ok(needs_attach)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub async fn set_muted(context: &Context, chat_id: ChatId, duration: MuteDuration) -> Result<()> {
    ensure!(!chat_id.is_special(), "Invalid chat ID");
    context
        .sql
        .execute(
            "UPDATE chats SET muted_until=? WHERE id=?;",
            paramsv![duration, chat_id],
        )
        .await
        .context(format!("Failed to set mute duration for {}", chat_id))?;
    context.emit_event(EventType::ChatModified(chat_id));
    Ok(())
}

pub async fn remove_contact_from_chat(
    context: &Context,
    chat_id: ChatId,
    contact_id: ContactId,
) -> Result<()> {
    ensure!(
        !chat_id.is_special(),
        "bad chat_id, can not be special chat: {}",
        chat_id
    );
    ensure!(
        !contact_id.is_special() || contact_id == ContactId::SELF,
        "Cannot remove special contact"
    );

    let mut msg = Message::default();
    let mut success = false;

    /* we do not check if "contact_id" exists but just delete all records with the id from chats_contacts */
    /* this allows to delete pending references to deleted contacts. Of course, this should _not_ happen. */
    if let Ok(chat) = Chat::load_from_db(context, chat_id).await {
        if chat.typ == Chattype::Group || chat.typ == Chattype::Broadcast {
            if !chat.is_self_in_chat(context).await? {
                context.emit_event(EventType::ErrorSelfNotInGroup(
                    "Cannot remove contact from chat; self not in group.".into(),
                ));
            } else {
                if let Ok(contact) = Contact::get_by_id(context, contact_id).await {
                    if chat.typ == Chattype::Group && chat.is_promoted() {
                        msg.viewtype = Viewtype::Text;
                        if contact.id == ContactId::SELF {
                            set_group_explicitly_left(context, &chat.grpid).await?;
                            msg.text =
                                Some(stock_str::msg_group_left(context, ContactId::SELF).await);
                        } else {
                            msg.text = Some(
                                stock_str::msg_del_member(
                                    context,
                                    contact.get_addr(),
                                    ContactId::SELF,
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
                success = remove_from_chat_contacts_table(context, chat_id, contact_id)
                    .await
                    .is_ok();
                context.emit_event(EventType::ChatModified(chat_id));
            }
        }
    }

    if !success {
        bail!("Failed to remove contact");
    }

    Ok(())
}

async fn set_group_explicitly_left(context: &Context, grpid: &str) -> Result<()> {
    if !is_group_explicitly_left(context, grpid).await? {
        context
            .sql
            .execute(
                "INSERT INTO leftgrps (grpid) VALUES(?);",
                paramsv![grpid.to_string()],
            )
            .await?;
    }

    Ok(())
}

pub(crate) async fn is_group_explicitly_left(context: &Context, grpid: &str) -> Result<bool> {
    let exists = context
        .sql
        .exists(
            "SELECT COUNT(*) FROM leftgrps WHERE grpid=?;",
            paramsv![grpid],
        )
        .await?;
    Ok(exists)
}

/// Sets group or mailing list chat name.
pub async fn set_chat_name(context: &Context, chat_id: ChatId, new_name: &str) -> Result<()> {
    let new_name = improve_single_line_input(new_name);
    /* the function only sets the names of group chats; normal chats get their names from the contacts */
    let mut success = false;

    ensure!(!new_name.is_empty(), "Invalid name");
    ensure!(!chat_id.is_special(), "Invalid chat ID");

    let chat = Chat::load_from_db(context, chat_id).await?;
    let mut msg = Message::default();

    if chat.typ == Chattype::Group
        || chat.typ == Chattype::Mailinglist
        || chat.typ == Chattype::Broadcast
    {
        if chat.name == new_name {
            success = true;
        } else if !chat.is_self_in_chat(context).await? {
            context.emit_event(EventType::ErrorSelfNotInGroup(
                "Cannot set chat name; self not in group".into(),
            ));
        } else {
            context
                .sql
                .execute(
                    "UPDATE chats SET name=? WHERE id=?;",
                    paramsv![new_name.to_string(), chat_id],
                )
                .await?;
            if chat.is_promoted()
                && !chat.is_mailing_list()
                && chat.typ != Chattype::Broadcast
                && improve_single_line_input(&chat.name) != new_name
            {
                msg.viewtype = Viewtype::Text;
                msg.text = Some(
                    stock_str::msg_grp_name(context, &chat.name, &new_name, ContactId::SELF).await,
                );
                msg.param.set_cmd(SystemMessage::GroupNameChanged);
                if !chat.name.is_empty() {
                    msg.param.set(Param::Arg, &chat.name);
                }
                msg.id = send_msg(context, chat_id, &mut msg).await?;
                context.emit_msgs_changed(chat_id, msg.id);
            }
            context.emit_event(EventType::ChatModified(chat_id));
            success = true;
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
    new_image: &str, // XXX use PathBuf
) -> Result<()> {
    ensure!(!chat_id.is_special(), "Invalid chat ID");
    let mut chat = Chat::load_from_db(context, chat_id).await?;
    ensure!(
        chat.typ == Chattype::Group || chat.typ == Chattype::Mailinglist,
        "Failed to set profile image; group does not exist"
    );
    /* we should respect this - whatever we send to the group, it gets discarded anyway! */
    if !is_contact_in_chat(context, chat_id, ContactId::SELF).await? {
        context.emit_event(EventType::ErrorSelfNotInGroup(
            "Cannot set chat profile image; self not in group.".into(),
        ));
        bail!("Failed to set profile image");
    }
    let mut msg = Message::new(Viewtype::Text);
    msg.param
        .set_int(Param::Cmd, SystemMessage::GroupImageChanged as i32);
    if new_image.is_empty() {
        chat.param.remove(Param::ProfileImage);
        msg.param.remove(Param::Arg);
        msg.text = Some(stock_str::msg_grp_img_deleted(context, ContactId::SELF).await);
    } else {
        let mut image_blob = BlobObject::new_from_path(context, Path::new(new_image)).await?;
        image_blob.recode_to_avatar_size(context).await?;
        chat.param.set(Param::ProfileImage, image_blob.as_name());
        msg.param.set(Param::Arg, image_blob.as_name());
        msg.text = Some(stock_str::msg_grp_img_changed(context, ContactId::SELF).await);
    }
    chat.update_param(context).await?;
    if chat.is_promoted() && !chat.is_mailing_list() {
        msg.id = send_msg(context, chat_id, &mut msg).await?;
        context.emit_msgs_changed(chat_id, msg.id);
    }
    context.emit_event(EventType::ChatModified(chat_id));
    Ok(())
}

pub async fn forward_msgs(context: &Context, msg_ids: &[MsgId], chat_id: ChatId) -> Result<()> {
    ensure!(!msg_ids.is_empty(), "empty msgs_ids: nothing to forward");
    ensure!(!chat_id.is_special(), "can not forward to special chat");

    let mut created_chats: Vec<ChatId> = Vec::new();
    let mut created_msgs: Vec<MsgId> = Vec::new();
    let mut curr_timestamp: i64;

    chat_id.unarchive_if_not_muted(context).await?;
    if let Ok(mut chat) = Chat::load_from_db(context, chat_id).await {
        if let Some(reason) = chat.why_cant_send(context).await? {
            bail!("cannot send to {}: {}", chat_id, reason);
        }
        curr_timestamp = create_smeared_timestamps(context, msg_ids.len()).await;
        let ids = context
            .sql
            .query_map(
                &format!(
                    "SELECT id FROM msgs WHERE id IN({}) ORDER BY timestamp,id",
                    sql::repeat_vars(msg_ids.len())
                ),
                rusqlite::params_from_iter(msg_ids),
                |row| row.get::<_, MsgId>(0),
                |ids| ids.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .await?;

        for id in ids {
            let src_msg_id: MsgId = id;
            let mut msg = Message::load_from_db(context, src_msg_id).await?;
            if msg.state == MessageState::OutDraft {
                bail!("cannot forward drafts.");
            }

            let original_param = msg.param.clone();

            // we tested a sort of broadcast
            // by not marking own forwarded messages as such,
            // however, this turned out to be to confusing and unclear.

            if msg.get_viewtype() != Viewtype::Sticker {
                msg.param
                    .set_int(Param::Forwarded, src_msg_id.to_u32() as i32);
            }

            msg.param.remove(Param::GuaranteeE2ee);
            msg.param.remove(Param::ForcePlaintext);
            msg.param.remove(Param::Cmd);
            msg.param.remove(Param::OverrideSenderDisplayname);
            msg.param.remove(Param::WebxdcSummary);
            msg.param.remove(Param::WebxdcSummaryTimestamp);
            msg.in_reply_to = None;

            // do not leak data as group names; a default subject is generated by mimfactory
            msg.subject = "".to_string();

            let new_msg_id: MsgId;
            if msg.state == MessageState::OutPreparing {
                new_msg_id = chat
                    .prepare_msg_raw(context, &mut msg, None, curr_timestamp)
                    .await?;
                curr_timestamp += 1;
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

                msg.update_param(context).await?;
                msg.param = save_param;
            } else {
                msg.state = MessageState::OutPending;
                new_msg_id = chat
                    .prepare_msg_raw(context, &mut msg, None, curr_timestamp)
                    .await?;
                curr_timestamp += 1;
                if create_send_msg_job(context, new_msg_id).await?.is_some() {
                    context.interrupt_smtp(InterruptInfo::new(false)).await;
                }
            }
            created_chats.push(chat_id);
            created_msgs.push(new_msg_id);
        }
    }
    for (chat_id, msg_id) in created_chats.iter().zip(created_msgs.iter()) {
        context.emit_msgs_changed(*chat_id, *msg_id);
    }
    Ok(())
}

pub async fn resend_msgs(context: &Context, msg_ids: &[MsgId]) -> Result<()> {
    let mut chat_id = None;
    let mut msgs: Vec<Message> = Vec::new();
    for msg_id in msg_ids {
        let msg = Message::load_from_db(context, *msg_id).await?;
        if let Some(chat_id) = chat_id {
            ensure!(
                chat_id == msg.chat_id,
                "messages to resend needs to be in the same chat"
            );
        } else {
            chat_id = Some(msg.chat_id);
        }
        ensure!(
            msg.from_id == ContactId::SELF,
            "can resend only own messages"
        );
        ensure!(!msg.is_info(), "cannot resend info messages");
        msgs.push(msg)
    }

    if let Some(chat_id) = chat_id {
        let chat = Chat::load_from_db(context, chat_id).await?;
        for mut msg in msgs {
            if msg.get_showpadlock() && !chat.is_protected() {
                msg.param.remove(Param::GuaranteeE2ee);
                msg.update_param(context).await?;
            }
            match msg.get_state() {
                MessageState::OutFailed | MessageState::OutDelivered | MessageState::OutMdnRcvd => {
                    message::update_msg_state(context, msg.id, MessageState::OutPending).await?
                }
                _ => bail!("unexpected message state"),
            }
            context.emit_event(EventType::MsgsChanged {
                chat_id: msg.chat_id,
                msg_id: msg.id,
            });
            if create_send_msg_job(context, msg.id).await?.is_some() {
                context.interrupt_smtp(InterruptInfo::new(false)).await;
            }
        }
    }
    Ok(())
}

pub(crate) async fn get_chat_cnt(context: &Context) -> Result<usize> {
    if context.sql.is_open().await {
        // no database, no chats - this is no error (needed eg. for information)
        let count = context
            .sql
            .count(
                "SELECT COUNT(*) FROM chats WHERE id>9 AND blocked=0;",
                paramsv![],
            )
            .await?;
        Ok(count)
    } else {
        Ok(0)
    }
}

/// Returns a tuple of `(chatid, is_protected, blocked)`.
pub(crate) async fn get_chat_id_by_grpid(
    context: &Context,
    grpid: &str,
) -> Result<Option<(ChatId, bool, Blocked)>> {
    context
        .sql
        .query_row_optional(
            "SELECT id, blocked, protected FROM chats WHERE grpid=?;",
            paramsv![grpid],
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
) -> Result<MsgId> {
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
        chat_id = ChatId::get_for_contact(context, ContactId::DEVICE).await?;

        let rfc724_mid = create_outgoing_rfc724_mid(None, "@device");
        msg.try_calc_and_set_dimensions(context).await.ok();
        prepare_msg_blob(context, msg).await?;
        chat_id.unarchive_if_not_muted(context).await?;

        let timestamp_sent = create_smeared_timestamp(context).await;

        // makes sure, the added message is the last one,
        // even if the date is wrong (useful esp. when warning about bad dates)
        let mut timestamp_sort = timestamp_sent;
        if let Some(last_msg_time) = context
            .sql
            .query_get_value(
                "SELECT MAX(timestamp) FROM msgs WHERE chat_id=?",
                paramsv![chat_id],
            )
            .await?
        {
            if timestamp_sort <= last_msg_time {
                timestamp_sort = last_msg_time + 1;
            }
        }

        let row_id = context
            .sql
            .insert(
                "INSERT INTO msgs (
            chat_id,
            from_id,
            to_id,
            timestamp,
            timestamp_sent,
            timestamp_rcvd,
            type,state,
            txt,
            param,
            rfc724_mid)
            VALUES (?,?,?,?,?,?,?,?,?,?,?);",
                paramsv![
                    chat_id,
                    ContactId::DEVICE,
                    ContactId::SELF,
                    timestamp_sort,
                    timestamp_sent,
                    timestamp_sent, // timestamp_sent equals timestamp_rcvd
                    msg.viewtype,
                    MessageState::InFresh,
                    msg.text.as_ref().cloned().unwrap_or_default(),
                    msg.param.to_string(),
                    rfc724_mid,
                ],
            )
            .await?;

        msg_id = MsgId::new(u32::try_from(row_id)?);
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
            context.emit_incoming_msg(chat_id, msg_id);
        } else {
            context.emit_msgs_changed(chat_id, msg_id);
        }
    }

    Ok(msg_id)
}

/// Adds a message to device chat.
pub async fn add_device_msg(
    context: &Context,
    label: Option<&str>,
    msg: Option<&mut Message>,
) -> Result<MsgId> {
    add_device_msg_with_importance(context, label, msg, false).await
}

pub async fn was_device_msg_ever_added(context: &Context, label: &str) -> Result<bool> {
    ensure!(!label.is_empty(), "empty label");
    let exists = context
        .sql
        .exists(
            "SELECT COUNT(label) FROM devmsglabels WHERE label=?",
            paramsv![label],
        )
        .await?;

    Ok(exists)
}

// needed on device-switches during export/import;
// - deletion in `msgs` with `ContactId::DEVICE` makes sure,
//   no wrong information are shown in the device chat
// - deletion in `devmsglabels` makes sure,
//   deleted messages are resetted and useful messages can be added again
// - we reset the config-option `QuotaExceeding`
//   that is used as a helper to drive the corresponding device message.
pub(crate) async fn delete_and_reset_all_device_msgs(context: &Context) -> Result<()> {
    context
        .sql
        .execute(
            "DELETE FROM msgs WHERE from_id=?;",
            paramsv![ContactId::DEVICE],
        )
        .await?;
    context
        .sql
        .execute("DELETE FROM devmsglabels;", paramsv![])
        .await?;

    // Insert labels for welcome messages to avoid them being readded on reconfiguration.
    context
        .sql
        .execute(
            r#"INSERT INTO devmsglabels (label) VALUES ("core-welcome-image"), ("core-welcome")"#,
            paramsv![],
        )
        .await?;
    context.set_config(Config::QuotaExceeding, None).await?;
    Ok(())
}

/// Adds an informational message to chat.
///
/// For example, it can be a message showing that a member was added to a group.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn add_info_msg_with_cmd(
    context: &Context,
    chat_id: ChatId,
    text: &str,
    cmd: SystemMessage,
    timestamp_sort: i64,
    // Timestamp to show to the user (if this is None, `timestamp_sort` will be shown to the user)
    timestamp_sent_rcvd: Option<i64>,
    parent: Option<&Message>,
    from_id: Option<ContactId>,
) -> Result<MsgId> {
    let rfc724_mid = create_outgoing_rfc724_mid(None, "@device");
    let ephemeral_timer = chat_id.get_ephemeral_timer(context).await?;

    let mut param = Params::new();
    if cmd != SystemMessage::Unknown {
        param.set_cmd(cmd)
    }

    let row_id =
    context.sql.insert(
        "INSERT INTO msgs (chat_id,from_id,to_id,timestamp,timestamp_sent,timestamp_rcvd,type,state,txt,rfc724_mid,ephemeral_timer, param,mime_in_reply_to)
        VALUES (?,?,?, ?,?,?,?,?, ?,?,?, ?,?);",
        paramsv![
            chat_id,
            from_id.unwrap_or(ContactId::INFO),
            ContactId::INFO,
            timestamp_sort,
            timestamp_sent_rcvd.unwrap_or(0),
            timestamp_sent_rcvd.unwrap_or(0),
            Viewtype::Text,
            MessageState::InNoticed,
            text,
            rfc724_mid,
            ephemeral_timer,
            param.to_string(),
            parent.map(|msg|msg.rfc724_mid.clone()).unwrap_or_default()
        ]
    ).await?;

    let msg_id = MsgId::new(row_id.try_into()?);
    context.emit_msgs_changed(chat_id, msg_id);

    Ok(msg_id)
}

/// Adds info message with a given text and `timestamp` to the chat.
pub(crate) async fn add_info_msg(
    context: &Context,
    chat_id: ChatId,
    text: &str,
    timestamp: i64,
) -> Result<MsgId> {
    add_info_msg_with_cmd(
        context,
        chat_id,
        text,
        SystemMessage::Unknown,
        timestamp,
        None,
        None,
        None,
    )
    .await
}

pub(crate) async fn update_msg_text_and_timestamp(
    context: &Context,
    chat_id: ChatId,
    msg_id: MsgId,
    text: &str,
    timestamp: i64,
) -> Result<()> {
    context
        .sql
        .execute(
            "UPDATE msgs SET txt=?, timestamp=? WHERE id=?;",
            paramsv![text, timestamp, msg_id],
        )
        .await?;
    context.emit_msgs_changed(chat_id, msg_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::chatlist::{get_archived_cnt, Chatlist};
    use crate::constants::{DC_GCL_ARCHIVED_ONLY, DC_GCL_NO_SPECIALS};
    use crate::contact::{Contact, ContactAddress};
    use crate::receive_imf::receive_imf;

    use crate::test_utils::TestContext;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_chat_info() {
        let t = TestContext::new().await;
        let chat = t.create_chat_with_contact("bob", "bob@example.com").await;
        let info = chat.get_info(&t).await.unwrap();

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
                "color": 35391,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_draft_no_draft() {
        let t = TestContext::new().await;
        let chat = t.get_self_chat().await;
        let draft = chat.id.get_draft(&t).await.unwrap();
        assert!(draft.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_draft_special_chat_id() {
        let t = TestContext::new().await;
        let draft = DC_CHAT_ID_LAST_SPECIAL.get_draft(&t).await.unwrap();
        assert!(draft.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_draft_no_chat() {
        // This is a weird case, maybe this should be an error but we
        // do not get this info from the database currently.
        let t = TestContext::new().await;
        let draft = ChatId::new(42).get_draft(&t).await.unwrap();
        assert!(draft.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_draft() {
        let t = TestContext::new().await;
        let chat_id = &t.get_self_chat().await.id;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hello".to_string()));

        chat_id.set_draft(&t, Some(&mut msg)).await.unwrap();
        let draft = chat_id.get_draft(&t).await.unwrap().unwrap();
        let msg_text = msg.get_text();
        let draft_text = draft.get_text();
        assert_eq!(msg_text, draft_text);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_delete_draft() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "abc").await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hi!".to_string()));
        chat_id.set_draft(&t, Some(&mut msg)).await?;
        assert!(chat_id.get_draft(&t).await?.is_some());

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("another".to_string()));
        chat_id.set_draft(&t, Some(&mut msg)).await?;
        assert!(chat_id.get_draft(&t).await?.is_some());

        chat_id.set_draft(&t, None).await?;
        assert!(chat_id.get_draft(&t).await?.is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_forwarding_draft_failing() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = &t.get_self_chat().await.id;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hello".to_string()));
        chat_id.set_draft(&t, Some(&mut msg)).await?;
        assert_eq!(msg.id, chat_id.get_draft(&t).await?.unwrap().id);

        let chat_id2 = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        assert!(forward_msgs(&t, &[msg.id], chat_id2).await.is_err());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_draft_stable_ids() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = &t.get_self_chat().await.id;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("hello".to_string()));
        assert_eq!(msg.id, MsgId::new_unset());
        assert!(chat_id.get_draft_msg_id(&t).await?.is_none());

        chat_id.set_draft(&t, Some(&mut msg)).await?;
        let id_after_1st_set = msg.id;
        assert_ne!(id_after_1st_set, MsgId::new_unset());
        assert_eq!(
            id_after_1st_set,
            chat_id.get_draft_msg_id(&t).await?.unwrap()
        );
        assert_eq!(id_after_1st_set, chat_id.get_draft(&t).await?.unwrap().id);

        msg.set_text(Some("hello2".to_string()));
        chat_id.set_draft(&t, Some(&mut msg)).await?;
        let id_after_2nd_set = msg.id;

        assert_eq!(id_after_2nd_set, id_after_1st_set);
        assert_eq!(
            id_after_2nd_set,
            chat_id.get_draft_msg_id(&t).await?.unwrap()
        );
        let test = chat_id.get_draft(&t).await?.unwrap();
        assert_eq!(id_after_2nd_set, test.id);
        assert_eq!(id_after_2nd_set, msg.id);
        assert_eq!(test.text, Some("hello2".to_string()));
        assert_eq!(test.state, MessageState::OutDraft);

        let id_after_prepare = prepare_msg(&t, *chat_id, &mut msg).await?;
        assert_eq!(id_after_prepare, id_after_1st_set);
        let test = Message::load_from_db(&t, id_after_prepare).await?;
        assert_eq!(test.state, MessageState::OutPreparing);
        assert!(!test.hidden); // sent draft must no longer be hidden

        let id_after_send = send_msg(&t, *chat_id, &mut msg).await?;
        assert_eq!(id_after_send, id_after_1st_set);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_change_quotes_on_reused_message_object() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "chat").await?;
        let quote1 =
            Message::load_from_db(&t, send_text_msg(&t, chat_id, "quote1".to_string()).await?)
                .await?;
        let quote2 =
            Message::load_from_db(&t, send_text_msg(&t, chat_id, "quote2".to_string()).await?)
                .await?;

        // save a draft
        let mut draft = Message::new(Viewtype::Text);
        draft.set_text(Some("draft text".to_string()));
        chat_id.set_draft(&t, Some(&mut draft)).await?;

        let test = Message::load_from_db(&t, draft.id).await?;
        assert_eq!(test.text, Some("draft text".to_string()));
        assert!(test.quoted_text().is_none());
        assert!(test.quoted_message(&t).await?.is_none());

        // add quote to same message object
        draft.set_quote(&t, Some(&quote1)).await?;
        chat_id.set_draft(&t, Some(&mut draft)).await?;

        let test = Message::load_from_db(&t, draft.id).await?;
        assert_eq!(test.text, Some("draft text".to_string()));
        assert_eq!(test.quoted_text(), Some("quote1".to_string()));
        assert_eq!(test.quoted_message(&t).await?.unwrap().id, quote1.id);

        // change quote on same message object
        draft.set_text(Some("another draft text".to_string()));
        draft.set_quote(&t, Some(&quote2)).await?;
        chat_id.set_draft(&t, Some(&mut draft)).await?;

        let test = Message::load_from_db(&t, draft.id).await?;
        assert_eq!(test.text, Some("another draft text".to_string()));
        assert_eq!(test.quoted_text(), Some("quote2".to_string()));
        assert_eq!(test.quoted_message(&t).await?.unwrap().id, quote2.id);

        // remove quote on same message object
        draft.set_quote(&t, None).await?;
        chat_id.set_draft(&t, Some(&mut draft)).await?;

        let test = Message::load_from_db(&t, draft.id).await?;
        assert_eq!(test.text, Some("another draft text".to_string()));
        assert!(test.quoted_text().is_none());
        assert!(test.quoted_message(&t).await?.is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_contact_to_chat_ex_add_self() {
        // Adding self to a contact should succeed, even though it's pointless.
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        let added = add_contact_to_chat_ex(&t, chat_id, ContactId::SELF, false)
            .await
            .unwrap();
        assert_eq!(added, false);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_modify_chat_multi_device() -> Result<()> {
        let a1 = TestContext::new_alice().await;
        let a2 = TestContext::new_alice().await;
        a1.set_config_bool(Config::BccSelf, true).await?;

        // create group and sync it to the second device
        let a1_chat_id = create_group_chat(&a1, ProtectionStatus::Unprotected, "foo").await?;
        let sent = a1.send_text(a1_chat_id, "ho!").await;
        let a1_msg = a1.get_last_msg().await;
        let a1_chat = Chat::load_from_db(&a1, a1_chat_id).await?;

        let a2_msg = a2.recv_msg(&sent).await;
        let a2_chat_id = a2_msg.chat_id;
        let a2_chat = Chat::load_from_db(&a2, a2_chat_id).await?;

        assert!(!a1_msg.is_system_message());
        assert!(!a2_msg.is_system_message());
        assert_eq!(a1_chat.grpid, a2_chat.grpid);
        assert_eq!(a1_chat.name, "foo");
        assert_eq!(a2_chat.name, "foo");
        assert_eq!(a1_chat.get_profile_image(&a1).await?, None);
        assert_eq!(a2_chat.get_profile_image(&a2).await?, None);
        assert_eq!(get_chat_contacts(&a1, a1_chat_id).await?.len(), 1);
        assert_eq!(get_chat_contacts(&a2, a2_chat_id).await?.len(), 1);

        // add a member to the group
        let bob = Contact::create(&a1, "", "bob@example.org").await?;
        add_contact_to_chat(&a1, a1_chat_id, bob).await?;
        let a1_msg = a1.get_last_msg().await;

        let a2_msg = a2.recv_msg(&a1.pop_sent_msg().await).await;

        assert!(a1_msg.is_system_message());
        assert!(a2_msg.is_system_message());
        assert_eq!(a1_msg.get_info_type(), SystemMessage::MemberAddedToGroup);
        assert_eq!(a2_msg.get_info_type(), SystemMessage::MemberAddedToGroup);
        assert_eq!(get_chat_contacts(&a1, a1_chat_id).await?.len(), 2);
        assert_eq!(get_chat_contacts(&a2, a2_chat_id).await?.len(), 2);

        // rename the group
        set_chat_name(&a1, a1_chat_id, "bar").await?;
        let a1_msg = a1.get_last_msg().await;

        let a2_msg = a2.recv_msg(&a1.pop_sent_msg().await).await;

        assert!(a1_msg.is_system_message());
        assert!(a2_msg.is_system_message());
        assert_eq!(a1_msg.get_info_type(), SystemMessage::GroupNameChanged);
        assert_eq!(a2_msg.get_info_type(), SystemMessage::GroupNameChanged);
        assert_eq!(Chat::load_from_db(&a1, a1_chat_id).await?.name, "bar");
        assert_eq!(Chat::load_from_db(&a2, a2_chat_id).await?.name, "bar");

        // remove member from group
        remove_contact_from_chat(&a1, a1_chat_id, bob).await?;
        let a1_msg = a1.get_last_msg().await;

        let a2_msg = a2.recv_msg(&a1.pop_sent_msg().await).await;

        assert!(a1_msg.is_system_message());
        assert!(a2_msg.is_system_message());
        assert_eq!(
            a1_msg.get_info_type(),
            SystemMessage::MemberRemovedFromGroup
        );
        assert_eq!(
            a2_msg.get_info_type(),
            SystemMessage::MemberRemovedFromGroup
        );
        assert_eq!(get_chat_contacts(&a1, a1_chat_id).await?.len(), 1);
        assert_eq!(get_chat_contacts(&a2, a2_chat_id).await?.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_modify_chat_disordered() -> Result<()> {
        // Alice creates a group with Bob, Claire and Daisy and then removes Claire and Daisy
        // (sleep() is needed as otherwise smeared time from Alice looks to Bob like messages from the future which are all set to "now" then)
        let alice = TestContext::new_alice().await;

        let bob_id = Contact::create(&alice, "", "bob@example.net").await?;
        let claire_id = Contact::create(&alice, "", "claire@foo.de").await?;
        let daisy_id = Contact::create(&alice, "", "daisy@bar.de").await?;

        let alice_chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "foo").await?;
        send_text_msg(&alice, alice_chat_id, "populate".to_string()).await?;

        add_contact_to_chat(&alice, alice_chat_id, bob_id).await?;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        let add1 = alice.pop_sent_msg().await;

        add_contact_to_chat(&alice, alice_chat_id, claire_id).await?;
        let add2 = alice.pop_sent_msg().await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        add_contact_to_chat(&alice, alice_chat_id, daisy_id).await?;
        let add3 = alice.pop_sent_msg().await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        assert_eq!(get_chat_contacts(&alice, alice_chat_id).await?.len(), 4);

        remove_contact_from_chat(&alice, alice_chat_id, claire_id).await?;
        let remove1 = alice.pop_sent_msg().await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        remove_contact_from_chat(&alice, alice_chat_id, daisy_id).await?;
        let remove2 = alice.pop_sent_msg().await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        assert_eq!(get_chat_contacts(&alice, alice_chat_id).await?.len(), 2);

        // Bob receives the add and deletion messages out of order
        let bob = TestContext::new_bob().await;
        bob.recv_msg(&add1).await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        bob.recv_msg(&add3).await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        let bob_chat_id = bob.recv_msg(&add2).await.chat_id;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        assert_eq!(get_chat_contacts(&bob, bob_chat_id).await?.len(), 4);

        bob.recv_msg(&remove2).await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        bob.recv_msg(&remove1).await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        assert_eq!(get_chat_contacts(&bob, bob_chat_id).await?.len(), 2);

        Ok(())
    }

    /// Test that group updates are robust to lost messages and eventual out of order arrival.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_modify_chat_lost() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let bob_id = Contact::create(&alice, "", "bob@example.net").await?;
        let claire_id = Contact::create(&alice, "", "claire@foo.de").await?;
        let daisy_id = Contact::create(&alice, "", "daisy@bar.de").await?;

        let alice_chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "foo").await?;
        add_contact_to_chat(&alice, alice_chat_id, bob_id).await?;
        add_contact_to_chat(&alice, alice_chat_id, claire_id).await?;
        add_contact_to_chat(&alice, alice_chat_id, daisy_id).await?;

        send_text_msg(&alice, alice_chat_id, "populate".to_string()).await?;
        let add = alice.pop_sent_msg().await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        remove_contact_from_chat(&alice, alice_chat_id, claire_id).await?;
        let remove1 = alice.pop_sent_msg().await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        remove_contact_from_chat(&alice, alice_chat_id, daisy_id).await?;
        let remove2 = alice.pop_sent_msg().await;

        let bob = TestContext::new_bob().await;

        bob.recv_msg(&add).await;
        let bob_chat_id = bob.get_last_msg().await.chat_id;
        assert_eq!(get_chat_contacts(&bob, bob_chat_id).await?.len(), 4);

        // First removal message is lost.
        // Nevertheless, two members are removed.
        bob.recv_msg(&remove2).await;
        assert_eq!(get_chat_contacts(&bob, bob_chat_id).await?.len(), 2);

        // Eventually, first removal message arrives.
        // This has no effect.
        bob.recv_msg(&remove1).await;
        assert_eq!(get_chat_contacts(&bob, bob_chat_id).await?.len(), 2);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_leave_group() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Create group chat with Bob.
        let alice_chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "foo").await?;
        let bob_contact = Contact::create(&alice, "", "bob@example.net").await?;
        add_contact_to_chat(&alice, alice_chat_id, bob_contact).await?;

        // Alice sends first message to group.
        let sent_msg = alice.send_text(alice_chat_id, "Hello!").await;
        let bob_msg = bob.recv_msg(&sent_msg).await;

        assert_eq!(get_chat_contacts(&alice, alice_chat_id).await?.len(), 2);

        // Bob leaves the group.
        let bob_chat_id = bob_msg.chat_id;
        bob_chat_id.accept(&bob).await?;
        remove_contact_from_chat(&bob, bob_chat_id, ContactId::SELF).await?;

        let leave_msg = bob.pop_sent_msg().await;
        alice.recv_msg(&leave_msg).await;

        assert_eq!(get_chat_contacts(&alice, alice_chat_id).await?.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_remove_contact_for_single() {
        let ctx = TestContext::new_alice().await;
        let bob = Contact::create(&ctx, "", "bob@f.br").await.unwrap();
        let chat_id = ChatId::create_for_contact(&ctx, bob).await.unwrap();
        let chat = Chat::load_from_db(&ctx, chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Single);
        assert_eq!(get_chat_contacts(&ctx, chat.id).await.unwrap().len(), 1);

        // adding or removing contacts from one-to-one-chats result in an error
        let claire = Contact::create(&ctx, "", "claire@foo.de").await.unwrap();
        let added = add_contact_to_chat_ex(&ctx, chat.id, claire, false).await;
        assert!(added.is_err());
        assert_eq!(get_chat_contacts(&ctx, chat.id).await.unwrap().len(), 1);

        let removed = remove_contact_from_chat(&ctx, chat.id, claire).await;
        assert!(removed.is_err());
        assert_eq!(get_chat_contacts(&ctx, chat.id).await.unwrap().len(), 1);

        let removed = remove_contact_from_chat(&ctx, chat.id, ContactId::SELF).await;
        assert!(removed.is_err());
        assert_eq!(get_chat_contacts(&ctx, chat.id).await.unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_self_talk() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat = &t.get_self_chat().await;
        assert!(!chat.id.is_special());
        assert!(chat.is_self_talk());
        assert!(chat.visibility == ChatVisibility::Normal);
        assert!(!chat.is_device_talk());
        assert!(chat.can_send(&t).await?);
        assert_eq!(chat.name, stock_str::saved_messages(&t).await);
        assert!(chat.get_profile_image(&t).await?.is_some());

        let msg_id = send_text_msg(&t, chat.id, "foo self".to_string()).await?;
        let msg = Message::load_from_db(&t, msg_id).await?;
        assert_eq!(msg.from_id, ContactId::SELF);
        assert_eq!(msg.to_id, ContactId::SELF);
        assert!(msg.get_showpadlock());

        let sent_msg = t.pop_sent_msg().await;
        let t2 = TestContext::new_alice().await;
        t2.recv_msg(&sent_msg).await;
        let chat = &t2.get_self_chat().await;
        let msg = t2.get_last_msg_in(chat.id).await;
        assert_eq!(msg.text, Some("foo self".to_string()));
        assert_eq!(msg.from_id, ContactId::SELF);
        assert_eq!(msg.to_id, ContactId::SELF);
        assert!(msg.get_showpadlock());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_device_msg_unlabelled() {
        let t = TestContext::new().await;

        // add two device-messages
        let mut msg1 = Message::new(Viewtype::Text);
        msg1.text = Some("first message".to_string());
        let msg1_id = add_device_msg(&t, None, Some(&mut msg1)).await;
        assert!(msg1_id.is_ok());

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.text = Some("second message".to_string());
        let msg2_id = add_device_msg(&t, None, Some(&mut msg2)).await;
        assert!(msg2_id.is_ok());
        assert_ne!(msg1_id.as_ref().unwrap(), msg2_id.as_ref().unwrap());

        // check added messages
        let msg1 = message::Message::load_from_db(&t, msg1_id.unwrap()).await;
        assert!(msg1.is_ok());
        let msg1 = msg1.unwrap();
        assert_eq!(msg1.text.as_ref().unwrap(), "first message");
        assert_eq!(msg1.from_id, ContactId::DEVICE);
        assert_eq!(msg1.to_id, ContactId::SELF);
        assert!(!msg1.is_info());
        assert!(!msg1.is_setupmessage());

        let msg2 = message::Message::load_from_db(&t, msg2_id.unwrap()).await;
        assert!(msg2.is_ok());
        let msg2 = msg2.unwrap();
        assert_eq!(msg2.text.as_ref().unwrap(), "second message");

        // check device chat
        assert_eq!(msg2.chat_id.get_msg_cnt(&t).await.unwrap(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_device_msg_labelled() -> Result<()> {
        let t = TestContext::new().await;

        // add two device-messages with the same label (second attempt is not added)
        let mut msg1 = Message::new(Viewtype::Text);
        msg1.text = Some("first message".to_string());
        let msg1_id = add_device_msg(&t, Some("any-label"), Some(&mut msg1)).await;
        assert!(msg1_id.is_ok());
        assert!(!msg1_id.as_ref().unwrap().is_unset());

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.text = Some("second message".to_string());
        let msg2_id = add_device_msg(&t, Some("any-label"), Some(&mut msg2)).await;
        assert!(msg2_id.is_ok());
        assert!(msg2_id.as_ref().unwrap().is_unset());

        // check added message
        let msg1 = message::Message::load_from_db(&t, *msg1_id.as_ref().unwrap()).await?;
        assert_eq!(msg1_id.as_ref().unwrap(), &msg1.id);
        assert_eq!(msg1.text.as_ref().unwrap(), "first message");
        assert_eq!(msg1.from_id, ContactId::DEVICE);
        assert_eq!(msg1.to_id, ContactId::SELF);
        assert!(!msg1.is_info());
        assert!(!msg1.is_setupmessage());

        // check device chat
        let chat_id = msg1.chat_id;

        assert_eq!(chat_id.get_msg_cnt(&t).await?, 1);
        assert!(!chat_id.is_special());
        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert_eq!(chat.get_type(), Chattype::Single);
        assert!(chat.is_device_talk());
        assert!(!chat.is_self_talk());
        assert!(!chat.can_send(&t).await?);
        assert!(chat.why_cant_send(&t).await? == Some(CantSendReason::DeviceChat));

        assert_eq!(chat.name, stock_str::device_messages(&t).await);
        assert!(chat.get_profile_image(&t).await?.is_some());

        // delete device message, make sure it is not added again
        message::delete_msgs(&t, &[*msg1_id.as_ref().unwrap()]).await?;
        let msg1 = message::Message::load_from_db(&t, *msg1_id.as_ref().unwrap()).await;
        assert!(msg1.is_err() || msg1.unwrap().chat_id.is_trash());
        let msg3_id = add_device_msg(&t, Some("any-label"), Some(&mut msg2)).await;
        assert!(msg3_id.is_ok());
        assert!(msg2_id.as_ref().unwrap().is_unset());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_device_msg_label_only() {
        let t = TestContext::new().await;
        let res = add_device_msg(&t, Some(""), None).await;
        assert!(res.is_err());
        let res = add_device_msg(&t, Some("some-label"), None).await;
        assert!(res.is_ok());

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());

        let msg_id = add_device_msg(&t, Some("some-label"), Some(&mut msg)).await;
        assert!(msg_id.is_ok());
        assert!(msg_id.as_ref().unwrap().is_unset());

        let msg_id = add_device_msg(&t, Some("unused-label"), Some(&mut msg)).await;
        assert!(msg_id.is_ok());
        assert!(!msg_id.as_ref().unwrap().is_unset());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_was_device_msg_ever_added() {
        let t = TestContext::new().await;
        add_device_msg(&t, Some("some-label"), None).await.ok();
        assert!(was_device_msg_ever_added(&t, "some-label").await.unwrap());

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        add_device_msg(&t, Some("another-label"), Some(&mut msg))
            .await
            .ok();
        assert!(was_device_msg_ever_added(&t, "another-label")
            .await
            .unwrap());

        assert!(!was_device_msg_ever_added(&t, "unused-label").await.unwrap());

        assert!(was_device_msg_ever_added(&t, "").await.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_delete_device_chat() {
        let t = TestContext::new().await;

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        add_device_msg(&t, Some("some-label"), Some(&mut msg))
            .await
            .ok();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);

        // after the device-chat and all messages are deleted, a re-adding should do nothing
        chats.get_chat_id(0).unwrap().delete(&t).await.ok();
        add_device_msg(&t, Some("some-label"), Some(&mut msg))
            .await
            .ok();
        assert_eq!(chatlist_len(&t, 0).await, 0)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_device_chat_cannot_sent() {
        let t = TestContext::new().await;
        t.update_device_chats().await.unwrap();
        let device_chat_id = ChatId::get_for_contact(&t, ContactId::DEVICE)
            .await
            .unwrap();

        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        assert!(send_msg(&t, device_chat_id, &mut msg).await.is_err());
        assert!(prepare_msg(&t, device_chat_id, &mut msg).await.is_err());

        let msg_id = add_device_msg(&t, None, Some(&mut msg)).await.unwrap();
        assert!(forward_msgs(&t, &[msg_id], device_chat_id).await.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_delete_and_reset_all_device_msgs() {
        let t = TestContext::new().await;
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("message text".to_string());
        let msg_id1 = add_device_msg(&t, Some("some-label"), Some(&mut msg))
            .await
            .unwrap();

        // adding a device message with the same label won't be executed again ...
        assert!(was_device_msg_ever_added(&t, "some-label").await.unwrap());
        let msg_id2 = add_device_msg(&t, Some("some-label"), Some(&mut msg))
            .await
            .unwrap();
        assert!(msg_id2.is_unset());

        // ... unless everything is deleted and resetted - as needed eg. on device switch
        delete_and_reset_all_device_msgs(&t).await.unwrap();
        assert!(!was_device_msg_ever_added(&t, "some-label").await.unwrap());
        let msg_id3 = add_device_msg(&t, Some("some-label"), Some(&mut msg))
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_archive() {
        // create two chats
        let t = TestContext::new().await;
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("foo".to_string());
        let msg_id = add_device_msg(&t, None, Some(&mut msg)).await.unwrap();
        let chat_id1 = message::Message::load_from_db(&t, msg_id)
            .await
            .unwrap()
            .chat_id;
        let chat_id2 = t.get_self_chat().await.id;
        assert!(!chat_id1.is_special());
        assert!(!chat_id2.is_special());

        assert_eq!(get_chat_cnt(&t).await.unwrap(), 2);
        assert_eq!(chatlist_len(&t, 0).await, 2);
        assert_eq!(chatlist_len(&t, DC_GCL_NO_SPECIALS).await, 2);
        assert_eq!(chatlist_len(&t, DC_GCL_ARCHIVED_ONLY).await, 0);
        assert_eq!(DC_GCL_ARCHIVED_ONLY, 0x01);
        assert_eq!(DC_GCL_NO_SPECIALS, 0x02);

        // archive first chat
        assert!(chat_id1
            .set_visibility(&t, ChatVisibility::Archived)
            .await
            .is_ok());
        assert!(
            Chat::load_from_db(&t, chat_id1)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert!(
            Chat::load_from_db(&t, chat_id2)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Normal
        );
        assert_eq!(get_chat_cnt(&t).await.unwrap(), 2);
        assert_eq!(chatlist_len(&t, 0).await, 2); // including DC_CHAT_ID_ARCHIVED_LINK now
        assert_eq!(chatlist_len(&t, DC_GCL_NO_SPECIALS).await, 1);
        assert_eq!(chatlist_len(&t, DC_GCL_ARCHIVED_ONLY).await, 1);

        // archive second chat
        assert!(chat_id2
            .set_visibility(&t, ChatVisibility::Archived)
            .await
            .is_ok());
        assert!(
            Chat::load_from_db(&t, chat_id1)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert!(
            Chat::load_from_db(&t, chat_id2)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert_eq!(get_chat_cnt(&t).await.unwrap(), 2);
        assert_eq!(chatlist_len(&t, 0).await, 1); // only DC_CHAT_ID_ARCHIVED_LINK now
        assert_eq!(chatlist_len(&t, DC_GCL_NO_SPECIALS).await, 0);
        assert_eq!(chatlist_len(&t, DC_GCL_ARCHIVED_ONLY).await, 2);

        // archive already archived first chat, unarchive second chat two times
        assert!(chat_id1
            .set_visibility(&t, ChatVisibility::Archived)
            .await
            .is_ok());
        assert!(chat_id2
            .set_visibility(&t, ChatVisibility::Normal)
            .await
            .is_ok());
        assert!(chat_id2
            .set_visibility(&t, ChatVisibility::Normal)
            .await
            .is_ok());
        assert!(
            Chat::load_from_db(&t, chat_id1)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Archived
        );
        assert!(
            Chat::load_from_db(&t, chat_id2)
                .await
                .unwrap()
                .get_visibility()
                == ChatVisibility::Normal
        );
        assert_eq!(get_chat_cnt(&t).await.unwrap(), 2);
        assert_eq!(chatlist_len(&t, 0).await, 2);
        assert_eq!(chatlist_len(&t, DC_GCL_NO_SPECIALS).await, 1);
        assert_eq!(chatlist_len(&t, DC_GCL_ARCHIVED_ONLY).await, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_unarchive_if_muted() -> Result<()> {
        let t = TestContext::new_alice().await;

        async fn msg_from_bob(t: &TestContext, num: u32) -> Result<()> {
            receive_imf(
                t,
                format!(
                    "From: bob@example.net\n\
                     To: alice@example.org\n\
                     Message-ID: <{}@example.org>\n\
                     Chat-Version: 1.0\n\
                     Date: Sun, 22 Mar 2022 19:37:57 +0000\n\
                     \n\
                     hello\n",
                    num
                )
                .as_bytes(),
                false,
            )
            .await?;
            Ok(())
        }

        msg_from_bob(&t, 1).await?;
        let chat_id = t.get_last_msg().await.get_chat_id();
        chat_id.accept(&t).await?;
        chat_id.set_visibility(&t, ChatVisibility::Archived).await?;
        assert_eq!(get_archived_cnt(&t).await?, 1);

        // not muted chat is unarchived on receiving a message
        msg_from_bob(&t, 2).await?;
        assert_eq!(get_archived_cnt(&t).await?, 0);

        // forever muted chat is not unarchived on receiving a message
        chat_id.set_visibility(&t, ChatVisibility::Archived).await?;
        set_muted(&t, chat_id, MuteDuration::Forever).await?;
        msg_from_bob(&t, 3).await?;
        assert_eq!(get_archived_cnt(&t).await?, 1);

        // otherwise muted chat is not unarchived on receiving a message
        set_muted(
            &t,
            chat_id,
            MuteDuration::Until(
                SystemTime::now()
                    .checked_add(Duration::from_secs(1000))
                    .unwrap(),
            ),
        )
        .await?;
        msg_from_bob(&t, 4).await?;
        assert_eq!(get_archived_cnt(&t).await?, 1);

        // expired mute will unarchive the chat
        set_muted(
            &t,
            chat_id,
            MuteDuration::Until(
                SystemTime::now()
                    .checked_sub(Duration::from_secs(1000))
                    .unwrap(),
            ),
        )
        .await?;
        msg_from_bob(&t, 5).await?;
        assert_eq!(get_archived_cnt(&t).await?, 0);

        // no unarchiving on sending to muted chat or on adding info messages to muted chat
        chat_id.set_visibility(&t, ChatVisibility::Archived).await?;
        set_muted(&t, chat_id, MuteDuration::Forever).await?;
        send_text_msg(&t, chat_id, "out".to_string()).await?;
        add_info_msg(&t, chat_id, "info", time()).await?;
        assert_eq!(get_archived_cnt(&t).await?, 1);

        // finally, unarchive on sending to not muted chat
        set_muted(&t, chat_id, MuteDuration::NotMuted).await?;
        send_text_msg(&t, chat_id, "out2".to_string()).await?;
        assert_eq!(get_archived_cnt(&t).await?, 0);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_archive_fresh_msgs() -> Result<()> {
        let t = TestContext::new_alice().await;

        async fn msg_from(t: &TestContext, name: &str, num: u32) -> Result<()> {
            receive_imf(
                t,
                format!(
                    "From: {}@example.net\n\
                     To: alice@example.org\n\
                     Message-ID: <{}@example.org>\n\
                     Chat-Version: 1.0\n\
                     Date: Sun, 22 Mar 2022 19:37:57 +0000\n\
                     \n\
                     hello\n",
                    name, num
                )
                .as_bytes(),
                false,
            )
            .await?;
            Ok(())
        }

        // receive some messages in archived+muted chats
        msg_from(&t, "bob", 1).await?;
        let bob_chat_id = t.get_last_msg().await.get_chat_id();
        bob_chat_id.accept(&t).await?;
        set_muted(&t, bob_chat_id, MuteDuration::Forever).await?;
        bob_chat_id
            .set_visibility(&t, ChatVisibility::Archived)
            .await?;
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 0);

        msg_from(&t, "bob", 2).await?;
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 1);

        msg_from(&t, "bob", 3).await?;
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 1);

        msg_from(&t, "claire", 4).await?;
        let claire_chat_id = t.get_last_msg().await.get_chat_id();
        claire_chat_id.accept(&t).await?;
        set_muted(&t, claire_chat_id, MuteDuration::Forever).await?;
        claire_chat_id
            .set_visibility(&t, ChatVisibility::Archived)
            .await?;
        msg_from(&t, "claire", 5).await?;
        msg_from(&t, "claire", 6).await?;
        msg_from(&t, "claire", 7).await?;
        assert_eq!(bob_chat_id.get_fresh_msg_cnt(&t).await?, 2);
        assert_eq!(claire_chat_id.get_fresh_msg_cnt(&t).await?, 3);
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 2);

        // mark one of the archived+muted chats as noticed: check that the archive-link counter is changed as well
        marknoticed_chat(&t, claire_chat_id).await?;
        assert_eq!(bob_chat_id.get_fresh_msg_cnt(&t).await?, 2);
        assert_eq!(claire_chat_id.get_fresh_msg_cnt(&t).await?, 0);
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 1);

        // receive some more messages
        msg_from(&t, "claire", 8).await?;
        assert_eq!(bob_chat_id.get_fresh_msg_cnt(&t).await?, 2);
        assert_eq!(claire_chat_id.get_fresh_msg_cnt(&t).await?, 1);
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 2);
        assert_eq!(t.get_fresh_msgs().await?.len(), 0);

        msg_from(&t, "dave", 9).await?;
        let dave_chat_id = t.get_last_msg().await.get_chat_id();
        dave_chat_id.accept(&t).await?;
        assert_eq!(dave_chat_id.get_fresh_msg_cnt(&t).await?, 1);
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 2);
        assert_eq!(t.get_fresh_msgs().await?.len(), 1);

        // mark the archived-link as noticed: check that the real chats are noticed as well
        marknoticed_chat(&t, DC_CHAT_ID_ARCHIVED_LINK).await?;
        assert_eq!(bob_chat_id.get_fresh_msg_cnt(&t).await?, 0);
        assert_eq!(claire_chat_id.get_fresh_msg_cnt(&t).await?, 0);
        assert_eq!(dave_chat_id.get_fresh_msg_cnt(&t).await?, 1);
        assert_eq!(DC_CHAT_ID_ARCHIVED_LINK.get_fresh_msg_cnt(&t).await?, 0);
        assert_eq!(t.get_fresh_msgs().await?.len(), 1);

        Ok(())
    }

    async fn get_chats_from_chat_list(ctx: &Context, listflags: usize) -> Vec<ChatId> {
        let chatlist = Chatlist::try_load(ctx, listflags, None, None)
            .await
            .unwrap();
        let mut result = Vec::new();
        for chatlist_index in 0..chatlist.len() {
            result.push(chatlist.get_chat_id(chatlist_index).unwrap())
        }
        result
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pinned() {
        let t = TestContext::new().await;

        // create 3 chats, wait 1 second in between to get a reliable order (we order by time)
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("foo".to_string());
        let msg_id = add_device_msg(&t, None, Some(&mut msg)).await.unwrap();
        let chat_id1 = message::Message::load_from_db(&t, msg_id)
            .await
            .unwrap()
            .chat_id;
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        let chat_id2 = t.get_self_chat().await.id;
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        let chat_id3 = create_group_chat(&t, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();

        let chatlist = get_chats_from_chat_list(&t, DC_GCL_NO_SPECIALS).await;
        assert_eq!(chatlist, vec![chat_id3, chat_id2, chat_id1]);

        // pin
        assert!(chat_id1
            .set_visibility(&t, ChatVisibility::Pinned)
            .await
            .is_ok());
        assert_eq!(
            Chat::load_from_db(&t, chat_id1)
                .await
                .unwrap()
                .get_visibility(),
            ChatVisibility::Pinned
        );

        // check if chat order changed
        let chatlist = get_chats_from_chat_list(&t, DC_GCL_NO_SPECIALS).await;
        assert_eq!(chatlist, vec![chat_id1, chat_id3, chat_id2]);

        // unpin
        assert!(chat_id1
            .set_visibility(&t, ChatVisibility::Normal)
            .await
            .is_ok());
        assert_eq!(
            Chat::load_from_db(&t, chat_id1)
                .await
                .unwrap()
                .get_visibility(),
            ChatVisibility::Normal
        );

        // check if chat order changed back
        let chatlist = get_chats_from_chat_list(&t, DC_GCL_NO_SPECIALS).await;
        assert_eq!(chatlist, vec![chat_id3, chat_id2, chat_id1]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_chat_name() {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        assert_eq!(
            Chat::load_from_db(&t, chat_id).await.unwrap().get_name(),
            "foo"
        );

        set_chat_name(&t, chat_id, "bar").await.unwrap();
        assert_eq!(
            Chat::load_from_db(&t, chat_id).await.unwrap().get_name(),
            "bar"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_same_chat_twice() {
        let context = TestContext::new().await;
        let contact1 = Contact::create(&context.ctx, "bob", "bob@mail.de")
            .await
            .unwrap();
        assert_ne!(contact1, ContactId::UNDEFINED);

        let chat_id = ChatId::create_for_contact(&context.ctx, contact1)
            .await
            .unwrap();
        assert!(!chat_id.is_special(), "chat_id too small {}", chat_id);
        let chat = Chat::load_from_db(&context.ctx, chat_id).await.unwrap();

        let chat2_id = ChatId::create_for_contact(&context.ctx, contact1)
            .await
            .unwrap();
        assert_eq!(chat2_id, chat_id);
        let chat2 = Chat::load_from_db(&context.ctx, chat2_id).await.unwrap();

        assert_eq!(chat2.name, chat.name);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_shall_attach_selfavatar() -> Result<()> {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        assert!(!shall_attach_selfavatar(&t, chat_id).await?);

        let (contact_id, _) = Contact::add_or_lookup(
            &t,
            "",
            ContactAddress::new("foo@bar.org")?,
            Origin::IncomingUnknownTo,
        )
        .await?;
        add_contact_to_chat(&t, chat_id, contact_id).await?;
        assert!(!shall_attach_selfavatar(&t, chat_id).await?);
        t.set_config(Config::Selfavatar, None).await?; // setting to None also forces re-sending
        assert!(shall_attach_selfavatar(&t, chat_id).await?);

        chat_id.set_selfavatar_timestamp(&t, time()).await?;
        assert!(!shall_attach_selfavatar(&t, chat_id).await?);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_mute_duration() {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        // Initial
        assert_eq!(
            Chat::load_from_db(&t, chat_id).await.unwrap().is_muted(),
            false
        );
        // Forever
        set_muted(&t, chat_id, MuteDuration::Forever).await.unwrap();
        assert_eq!(
            Chat::load_from_db(&t, chat_id).await.unwrap().is_muted(),
            true
        );
        // unMute
        set_muted(&t, chat_id, MuteDuration::NotMuted)
            .await
            .unwrap();
        assert_eq!(
            Chat::load_from_db(&t, chat_id).await.unwrap().is_muted(),
            false
        );
        // Timed in the future
        set_muted(
            &t,
            chat_id,
            MuteDuration::Until(SystemTime::now() + Duration::from_secs(3600)),
        )
        .await
        .unwrap();
        assert_eq!(
            Chat::load_from_db(&t, chat_id).await.unwrap().is_muted(),
            true
        );
        // Time in the past
        set_muted(
            &t,
            chat_id,
            MuteDuration::Until(SystemTime::now() - Duration::from_secs(3600)),
        )
        .await
        .unwrap();
        assert_eq!(
            Chat::load_from_db(&t, chat_id).await.unwrap().is_muted(),
            false
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_info_msg() -> Result<()> {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        add_info_msg(&t, chat_id, "foo info", 200000).await?;

        let msg = t.get_last_msg_in(chat_id).await;
        assert_eq!(msg.get_chat_id(), chat_id);
        assert_eq!(msg.get_viewtype(), Viewtype::Text);
        assert_eq!(msg.get_text().unwrap(), "foo info");
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::Unknown);
        assert!(msg.parent(&t).await?.is_none());
        assert!(msg.quoted_message(&t).await?.is_none());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_info_msg_with_cmd() -> Result<()> {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let msg_id = add_info_msg_with_cmd(
            &t,
            chat_id,
            "foo bar info",
            SystemMessage::EphemeralTimerChanged,
            10000,
            None,
            None,
            None,
        )
        .await?;

        let msg = Message::load_from_db(&t, msg_id).await?;
        assert_eq!(msg.get_chat_id(), chat_id);
        assert_eq!(msg.get_viewtype(), Viewtype::Text);
        assert_eq!(msg.get_text().unwrap(), "foo bar info");
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::EphemeralTimerChanged);
        assert!(msg.parent(&t).await?.is_none());
        assert!(msg.quoted_message(&t).await?.is_none());

        let msg2 = t.get_last_msg_in(chat_id).await;
        assert_eq!(msg.get_id(), msg2.get_id());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_protection() -> Result<()> {
        let t = TestContext::new_alice().await;
        t.set_config_bool(Config::BccSelf, false).await?;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert!(!chat.is_protected());
        assert!(chat.is_unpromoted());

        // enable protection on unpromoted chat, the info-message is added via add_info_msg()
        chat_id
            .set_protection(&t, ProtectionStatus::Protected)
            .await?;

        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert!(chat.is_protected());
        assert!(chat.is_unpromoted());

        let msgs = get_chat_msgs(&t, chat_id, 0).await?;
        assert_eq!(msgs.len(), 1);

        let msg = t.get_last_msg_in(chat_id).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::ChatProtectionEnabled);
        assert_eq!(msg.get_state(), MessageState::InNoticed);

        // disable protection again, still unpromoted
        chat_id
            .set_protection(&t, ProtectionStatus::Unprotected)
            .await?;

        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert!(!chat.is_protected());
        assert!(chat.is_unpromoted());

        let msg = t.get_last_msg_in(chat_id).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::ChatProtectionDisabled);
        assert_eq!(msg.get_state(), MessageState::InNoticed);

        // send a message, this switches to promoted state
        send_text_msg(&t, chat_id, "hi!".to_string()).await?;

        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert!(!chat.is_protected());
        assert!(!chat.is_unpromoted());

        let msgs = get_chat_msgs(&t, chat_id, 0).await?;
        assert_eq!(msgs.len(), 3);

        // enable protection on promoted chat, the info-message is sent via send_msg() this time
        chat_id
            .set_protection(&t, ProtectionStatus::Protected)
            .await?;
        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert!(chat.is_protected());
        assert!(!chat.is_unpromoted());

        let msg = t.get_last_msg_in(chat_id).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_info_type(), SystemMessage::ChatProtectionEnabled);
        assert_eq!(msg.get_state(), MessageState::OutDelivered); // as bcc-self is disabled and there is nobody else in the chat

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lookup_by_contact_id() {
        let ctx = TestContext::new_alice().await;

        // create contact, then unblocked chat
        let contact_id = Contact::create(&ctx, "", "bob@foo.de").await.unwrap();
        assert_ne!(contact_id, ContactId::UNDEFINED);
        let found = ChatId::lookup_by_contact(&ctx, contact_id).await.unwrap();
        assert!(found.is_none());

        let chat_id = ChatId::create_for_contact(&ctx, contact_id).await.unwrap();
        let chat2 = ChatIdBlocked::lookup_by_contact(&ctx, contact_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(chat_id, chat2.id);
        assert_eq!(chat2.blocked, Blocked::Not);

        // create contact, then blocked chat
        let contact_id = Contact::create(&ctx, "", "claire@foo.de").await.unwrap();
        let chat_id = ChatIdBlocked::get_for_contact(&ctx, contact_id, Blocked::Yes)
            .await
            .unwrap()
            .id;
        let chat2 = ChatIdBlocked::lookup_by_contact(&ctx, contact_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(chat_id, chat2.id);
        assert_eq!(chat2.blocked, Blocked::Yes);

        // test nonexistent contact
        let found = ChatId::lookup_by_contact(&ctx, ContactId::new(1234))
            .await
            .unwrap();
        assert!(found.is_none());

        let found = ChatIdBlocked::lookup_by_contact(&ctx, ContactId::new(1234))
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lookup_self_by_contact_id() {
        let ctx = TestContext::new_alice().await;

        let chat = ChatId::lookup_by_contact(&ctx, ContactId::SELF)
            .await
            .unwrap();
        assert!(chat.is_none());

        ctx.update_device_chats().await.unwrap();
        let chat = ChatIdBlocked::lookup_by_contact(&ctx, ContactId::SELF)
            .await
            .unwrap()
            .unwrap();
        assert!(!chat.id.is_special());
        assert!(chat.id.is_self_talk(&ctx).await.unwrap());
        assert_eq!(chat.blocked, Blocked::Not);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_group_with_removed_message_id() -> Result<()> {
        // Alice creates a group with Bob, sends a message to bob
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        alice.set_config(Config::ShowEmails, Some("2")).await?;
        bob.set_config(Config::ShowEmails, Some("2")).await?;

        let alice_bob_contact = alice.add_or_lookup_contact(&bob).await;
        let contact_id = alice_bob_contact.id;
        let alice_chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "grp").await?;
        let alice_chat = Chat::load_from_db(&alice, alice_chat_id).await?;

        add_contact_to_chat(&alice, alice_chat_id, contact_id).await?;
        assert_eq!(get_chat_contacts(&alice, alice_chat_id).await?.len(), 2);
        send_text_msg(&alice, alice_chat_id, "hi!".to_string()).await?;
        assert_eq!(get_chat_msgs(&alice, alice_chat_id, 0).await?.len(), 1);

        // Alice has an SMTP-server replacing the `Message-ID:`-header (as done eg. by outlook.com).
        let sent_msg = alice.pop_sent_msg().await;
        let msg = sent_msg.payload();
        assert_eq!(msg.match_indices("Gr.").count(), 2);
        let msg = msg.replace("Message-ID: <Gr.", "Message-ID: <XXX");
        assert_eq!(msg.match_indices("Gr.").count(), 1);

        // Bob receives this message, he may detect group by `References:`- or `Chat-Group:`-header
        receive_imf(&bob, msg.as_bytes(), false).await.unwrap();
        let msg = bob.get_last_msg().await;

        let bob_chat = Chat::load_from_db(&bob, msg.chat_id).await?;
        assert_eq!(bob_chat.grpid, alice_chat.grpid);

        // Bob accepts contact request.
        bob_chat.id.unblock(&bob).await?;

        // Bob answers - simulate a normal MUA by not setting `Chat-*`-headers;
        // moreover, Bob's SMTP-server also replaces the `Message-ID:`-header
        send_text_msg(&bob, bob_chat.id, "ho!".to_string()).await?;
        let sent_msg = bob.pop_sent_msg().await;
        let msg = sent_msg.payload();
        let msg = msg.replace("Message-ID: <Gr.", "Message-ID: <XXX");
        let msg = msg.replace("Chat-", "XXXX-");
        assert_eq!(msg.match_indices("Chat-").count(), 0);

        // Alice receives this message - she can still detect the group by the `References:`-header
        receive_imf(&alice, msg.as_bytes(), false).await.unwrap();
        let msg = alice.get_last_msg().await;
        assert_eq!(msg.chat_id, alice_chat_id);
        assert_eq!(msg.text, Some("ho!".to_string()));
        assert_eq!(get_chat_msgs(&alice, alice_chat_id, 0).await?.len(), 2);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_marknoticed_chat() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat = t.create_chat_with_contact("bob", "bob@example.org").await;

        receive_imf(
            &t,
            b"From: bob@example.org\n\
                 To: alice@example.org\n\
                 Message-ID: <1@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Fri, 23 Apr 2021 10:00:57 +0000\n\
                 \n\
                 hello\n",
            false,
        )
        .await?;

        let chats = Chatlist::try_load(&t, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        assert_eq!(chats.get_chat_id(0)?, chat.id);
        assert_eq!(chat.id.get_fresh_msg_cnt(&t).await?, 1);
        assert_eq!(t.get_fresh_msgs().await?.len(), 1);

        let msgs = get_chat_msgs(&t, chat.id, 0).await?;
        assert_eq!(msgs.len(), 1);
        let msg_id = match msgs.first().unwrap() {
            ChatItem::Message { msg_id } => *msg_id,
            _ => MsgId::new_unset(),
        };
        let msg = message::Message::load_from_db(&t, msg_id).await?;
        assert_eq!(msg.state, MessageState::InFresh);

        marknoticed_chat(&t, chat.id).await?;

        let chats = Chatlist::try_load(&t, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        let msg = message::Message::load_from_db(&t, msg_id).await?;
        assert_eq!(msg.state, MessageState::InNoticed);
        assert_eq!(chat.id.get_fresh_msg_cnt(&t).await?, 0);
        assert_eq!(t.get_fresh_msgs().await?.len(), 0);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_contact_request_fresh_messages() -> Result<()> {
        let t = TestContext::new_alice().await;

        let chats = Chatlist::try_load(&t, 0, None, None).await?;
        assert_eq!(chats.len(), 0);

        receive_imf(
            &t,
            b"From: bob@example.org\n\
                 To: alice@example.org\n\
                 Message-ID: <1@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 22 Mar 2021 19:37:57 +0000\n\
                 \n\
                 hello\n",
            false,
        )
        .await?;

        let chats = Chatlist::try_load(&t, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        let chat_id = chats.get_chat_id(0).unwrap();
        assert!(Chat::load_from_db(&t, chat_id)
            .await
            .unwrap()
            .is_contact_request());
        assert_eq!(chat_id.get_msg_cnt(&t).await?, 1);
        assert_eq!(chat_id.get_fresh_msg_cnt(&t).await?, 1);
        let msgs = get_chat_msgs(&t, chat_id, 0).await?;
        assert_eq!(msgs.len(), 1);
        let msg_id = match msgs.first().unwrap() {
            ChatItem::Message { msg_id } => *msg_id,
            _ => MsgId::new_unset(),
        };
        let msg = message::Message::load_from_db(&t, msg_id).await?;
        assert_eq!(msg.state, MessageState::InFresh);

        // Contact requests are excluded from global badge.
        assert_eq!(t.get_fresh_msgs().await?.len(), 0);

        let chats = Chatlist::try_load(&t, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        let msg = message::Message::load_from_db(&t, msg_id).await?;
        assert_eq!(msg.state, MessageState::InFresh);
        assert_eq!(t.get_fresh_msgs().await?.len(), 0);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_contact_request_archive() -> Result<()> {
        let t = TestContext::new_alice().await;

        receive_imf(
            &t,
            b"From: bob@example.org\n\
                 To: alice@example.org\n\
                 Message-ID: <2@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 22 Mar 2021 19:37:57 +0000\n\
                 \n\
                 hello\n",
            false,
        )
        .await?;

        let chats = Chatlist::try_load(&t, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        let chat_id = chats.get_chat_id(0)?;
        assert!(Chat::load_from_db(&t, chat_id).await?.is_contact_request());
        assert_eq!(get_archived_cnt(&t).await?, 0);

        // archive request without accepting or blocking
        chat_id.set_visibility(&t, ChatVisibility::Archived).await?;

        let chats = Chatlist::try_load(&t, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        let chat_id = chats.get_chat_id(0)?;
        assert!(chat_id.is_archived_link());
        assert_eq!(get_archived_cnt(&t).await?, 1);

        let chats = Chatlist::try_load(&t, DC_GCL_ARCHIVED_ONLY, None, None).await?;
        assert_eq!(chats.len(), 1);
        let chat_id = chats.get_chat_id(0)?;
        assert!(Chat::load_from_db(&t, chat_id).await?.is_contact_request());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_classic_email_chat() -> Result<()> {
        let alice = TestContext::new_alice().await;

        // Alice enables receiving classic emails.
        alice
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();

        // Alice receives a classic (non-chat) message from Bob.
        receive_imf(
            &alice,
            b"From: bob@example.org\n\
                 To: alice@example.org\n\
                 Message-ID: <1@example.org>\n\
                 Date: Sun, 22 Mar 2021 19:37:57 +0000\n\
                 \n\
                 hello\n",
            false,
        )
        .await?;

        let msg = alice.get_last_msg().await;
        let chat_id = msg.chat_id;
        assert_eq!(chat_id.get_fresh_msg_cnt(&alice).await?, 1);

        let msgs = get_chat_msgs(&alice, chat_id, 0).await?;
        assert_eq!(msgs.len(), 1);

        // Alice disables receiving classic emails.
        alice
            .set_config(Config::ShowEmails, Some("0"))
            .await
            .unwrap();

        // Already received classic email should still be in the chat.
        assert_eq!(chat_id.get_fresh_msg_cnt(&alice).await?, 1);

        let msgs = get_chat_msgs(&alice, chat_id, 0).await?;
        assert_eq!(msgs.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_chat_get_color() -> Result<()> {
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "a chat").await?;
        let color1 = Chat::load_from_db(&t, chat_id).await?.get_color(&t).await?;
        assert_eq!(color1, 0x008772);

        // upper-/lowercase makes a difference for the colors, these are different groups
        // (in contrast to email addresses, where upper-/lowercase is ignored in practise)
        let t = TestContext::new().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "A CHAT").await?;
        let color2 = Chat::load_from_db(&t, chat_id).await?.get_color(&t).await?;
        assert_ne!(color2, color1);
        Ok(())
    }

    async fn test_sticker(filename: &str, bytes: &[u8], w: i32, h: i32) -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let alice_chat = alice.create_chat(&bob).await;
        let bob_chat = bob.create_chat(&alice).await;

        let file = alice.get_blobdir().join(filename);
        tokio::fs::write(&file, bytes).await?;

        let mut msg = Message::new(Viewtype::Sticker);
        msg.set_file(file.to_str().unwrap(), None);

        let sent_msg = alice.send_msg(alice_chat.id, &mut msg).await;
        let mime = sent_msg.payload();
        assert_eq!(mime.match_indices("Chat-Content: sticker").count(), 1);

        let msg = bob.recv_msg(&sent_msg).await;
        assert_eq!(msg.chat_id, bob_chat.id);
        assert_eq!(msg.get_viewtype(), Viewtype::Sticker);
        assert_eq!(msg.get_filename(), Some(filename.to_string()));
        assert_eq!(msg.get_width(), w);
        assert_eq!(msg.get_height(), h);
        assert!(msg.get_filebytes(&bob).await?.unwrap() > 250);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sticker_png() -> Result<()> {
        test_sticker(
            "sticker.png",
            include_bytes!("../test-data/image/avatar64x64.png"),
            64,
            64,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sticker_jpeg() -> Result<()> {
        test_sticker(
            "sticker.jpg",
            include_bytes!("../test-data/image/avatar1000x1000.jpg"),
            1000,
            1000,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sticker_gif() -> Result<()> {
        test_sticker(
            "sticker.gif",
            include_bytes!("../test-data/image/image100x50.gif"),
            100,
            50,
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sticker_forward() -> Result<()> {
        // create chats
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let alice_chat = alice.create_chat(&bob).await;
        let bob_chat = bob.create_chat(&alice).await;

        // create sticker
        let file_name = "sticker.jpg";
        let bytes = include_bytes!("../test-data/image/avatar1000x1000.jpg");
        let file = alice.get_blobdir().join(file_name);
        tokio::fs::write(&file, bytes).await?;
        let mut msg = Message::new(Viewtype::Sticker);
        msg.set_file(file.to_str().unwrap(), None);

        // send sticker to bob
        let sent_msg = alice.send_msg(alice_chat.get_id(), &mut msg).await;
        let msg = bob.recv_msg(&sent_msg).await;

        // forward said sticker to alice
        forward_msgs(&bob, &[msg.id], bob_chat.get_id()).await?;
        let forwarded_msg = bob.pop_sent_msg().await;

        let msg = alice.recv_msg(&forwarded_msg).await;
        // forwarded sticker should not have forwarded-flag
        assert!(!msg.is_forwarded());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_forward() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let alice_chat = alice.create_chat(&bob).await;
        let bob_chat = bob.create_chat(&alice).await;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("Hi Bob".to_owned()));
        let sent_msg = alice.send_msg(alice_chat.get_id(), &mut msg).await;
        let msg = bob.recv_msg(&sent_msg).await;

        forward_msgs(&bob, &[msg.id], bob_chat.get_id()).await?;

        let forwarded_msg = bob.pop_sent_msg().await;
        let msg = alice.recv_msg(&forwarded_msg).await;
        assert!(msg.get_text().unwrap() == "Hi Bob");
        assert!(msg.is_forwarded());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_forward_info_msg() -> Result<()> {
        let t = TestContext::new_alice().await;

        let chat_id1 = create_group_chat(&t, ProtectionStatus::Unprotected, "a").await?;
        send_text_msg(&t, chat_id1, "msg one".to_string()).await?;
        let bob_id = Contact::create(&t, "", "bob@example.net").await?;
        add_contact_to_chat(&t, chat_id1, bob_id).await?;
        let msg1 = t.get_last_msg_in(chat_id1).await;
        assert!(msg1.is_info());
        assert!(msg1.get_text().unwrap().contains("bob@example.net"));

        let chat_id2 = ChatId::create_for_contact(&t, bob_id).await?;
        assert_eq!(get_chat_msgs(&t, chat_id2, 0).await?.len(), 0);
        forward_msgs(&t, &[msg1.id], chat_id2).await?;
        let msg2 = t.get_last_msg_in(chat_id2).await;
        assert!(!msg2.is_info()); // forwarded info-messages lose their info-state
        assert_eq!(msg2.get_info_type(), SystemMessage::Unknown);
        assert_ne!(msg2.from_id, ContactId::INFO);
        assert_ne!(msg2.to_id, ContactId::INFO);
        assert_eq!(msg2.get_text().unwrap(), msg1.get_text().unwrap());
        assert!(msg2.is_forwarded());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_forward_quote() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let alice_chat = alice.create_chat(&bob).await;
        let bob_chat = bob.create_chat(&alice).await;

        // Alice sends a message to Bob.
        let sent_msg = alice.send_text(alice_chat.id, "Hi Bob").await;
        let received_msg = bob.recv_msg(&sent_msg).await;

        // Bob quotes received message and sends a reply to Alice.
        let mut reply = Message::new(Viewtype::Text);
        reply.set_text(Some("Reply".to_owned()));
        reply.set_quote(&bob, Some(&received_msg)).await?;
        let sent_reply = bob.send_msg(bob_chat.id, &mut reply).await;
        let received_reply = alice.recv_msg(&sent_reply).await;

        // Alice forwards a reply.
        forward_msgs(&alice, &[received_reply.id], alice_chat.get_id()).await?;
        let forwarded_msg = alice.pop_sent_msg().await;
        let alice_forwarded_msg = bob.recv_msg(&forwarded_msg).await;
        assert!(alice_forwarded_msg.quoted_message(&alice).await?.is_none());
        assert_eq!(
            alice_forwarded_msg.quoted_text(),
            Some("Hi Bob".to_string())
        );

        let bob_forwarded_msg = bob.get_last_msg().await;
        assert!(bob_forwarded_msg.quoted_message(&bob).await?.is_none());
        assert_eq!(bob_forwarded_msg.quoted_text(), Some("Hi Bob".to_string()));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_forward_group() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let alice_chat = alice.create_chat(&bob).await;
        let bob_chat = bob.create_chat(&alice).await;

        // Alice creates a group with Bob.
        let alice_group_chat_id =
            create_group_chat(&alice, ProtectionStatus::Unprotected, "Group").await?;
        let bob_id = Contact::create(&alice, "Bob", "bob@example.net").await?;
        let claire_id = Contact::create(&alice, "Claire", "claire@example.net").await?;
        add_contact_to_chat(&alice, alice_group_chat_id, bob_id).await?;
        add_contact_to_chat(&alice, alice_group_chat_id, claire_id).await?;
        let sent_group_msg = alice
            .send_text(alice_group_chat_id, "Hi Bob and Claire")
            .await;
        let bob_group_chat_id = bob.recv_msg(&sent_group_msg).await.chat_id;

        // Alice deletes a message on her device.
        // This is needed to make assignment of further messages received in this group
        // based on `References:` header harder.
        // Previously this exposed a bug, so this is a regression test.
        message::delete_msgs(&alice, &[sent_group_msg.sender_msg_id]).await?;

        // Alice sends a message to Bob.
        let sent_msg = alice.send_text(alice_chat.id, "Hi Bob").await;
        let received_msg = bob.recv_msg(&sent_msg).await;
        assert_eq!(received_msg.get_text(), Some("Hi Bob".to_string()));
        assert_eq!(received_msg.chat_id, bob_chat.id);

        // Alice sends another message to Bob, this has first message as a parent.
        let sent_msg = alice.send_text(alice_chat.id, "Hello Bob").await;
        let received_msg = bob.recv_msg(&sent_msg).await;
        assert_eq!(received_msg.get_text(), Some("Hello Bob".to_string()));
        assert_eq!(received_msg.chat_id, bob_chat.id);

        // Bob forwards message to a group chat with Alice.
        forward_msgs(&bob, &[received_msg.id], bob_group_chat_id).await?;
        let forwarded_msg = bob.pop_sent_msg().await;
        alice.recv_msg(&forwarded_msg).await;

        let received_forwarded_msg = alice.get_last_msg_in(alice_group_chat_id).await;
        assert!(received_forwarded_msg.is_forwarded());
        assert_eq!(received_forwarded_msg.chat_id, alice_group_chat_id);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_only_minimal_data_are_forwarded() -> Result<()> {
        // send a message from Alice to a group with Bob
        let alice = TestContext::new_alice().await;
        alice
            .set_config(Config::Displayname, Some("secretname"))
            .await?;
        let bob_id = Contact::create(&alice, "bob", "bob@example.net").await?;
        let group_id =
            create_group_chat(&alice, ProtectionStatus::Unprotected, "secretgrpname").await?;
        add_contact_to_chat(&alice, group_id, bob_id).await?;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("bla foo".to_owned()));
        let sent_msg = alice.send_msg(group_id, &mut msg).await;
        assert!(sent_msg.payload().contains("secretgrpname"));
        assert!(sent_msg.payload().contains("secretname"));
        assert!(sent_msg.payload().contains("alice"));

        // Bob forwards that message to Claire -
        // Claire should not get information about Alice for the original Group
        let bob = TestContext::new_bob().await;
        let orig_msg = bob.recv_msg(&sent_msg).await;
        let claire_id = Contact::create(&bob, "claire", "claire@foo").await?;
        let single_id = ChatId::create_for_contact(&bob, claire_id).await?;
        let group_id = create_group_chat(&bob, ProtectionStatus::Unprotected, "group2").await?;
        add_contact_to_chat(&bob, group_id, claire_id).await?;
        let broadcast_id = create_broadcast_list(&bob).await?;
        add_contact_to_chat(&bob, broadcast_id, claire_id).await?;
        for chat_id in &[single_id, group_id, broadcast_id] {
            forward_msgs(&bob, &[orig_msg.id], *chat_id).await?;
            let sent_msg = bob.pop_sent_msg().await;
            assert!(sent_msg
                .payload()
                .contains("---------- Forwarded message ----------"));
            assert!(!sent_msg.payload().contains("secretgrpname"));
            assert!(!sent_msg.payload().contains("secretname"));
            assert!(!sent_msg.payload().contains("alice"));
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_resend_own_message() -> Result<()> {
        // Alice creates group with Bob and sends an initial message
        let alice = TestContext::new_alice().await;
        let alice_grp = create_group_chat(&alice, ProtectionStatus::Unprotected, "grp").await?;
        add_contact_to_chat(
            &alice,
            alice_grp,
            Contact::create(&alice, "", "bob@example.net").await?,
        )
        .await?;
        let sent1 = alice.send_text(alice_grp, "alice->bob").await;

        // Alice adds Claire to group and resends her own initial message
        add_contact_to_chat(
            &alice,
            alice_grp,
            Contact::create(&alice, "", "claire@example.org").await?,
        )
        .await?;
        let sent2 = alice.pop_sent_msg().await;
        resend_msgs(&alice, &[sent1.sender_msg_id]).await?;
        let sent3 = alice.pop_sent_msg().await;

        // Bob receives all messages
        let bob = TestContext::new_bob().await;
        let msg = bob.recv_msg(&sent1).await;
        assert_eq!(msg.get_text().unwrap(), "alice->bob");
        assert_eq!(get_chat_contacts(&bob, msg.chat_id).await?.len(), 2);
        assert_eq!(get_chat_msgs(&bob, msg.chat_id, 0).await?.len(), 1);
        bob.recv_msg(&sent2).await;
        assert_eq!(get_chat_contacts(&bob, msg.chat_id).await?.len(), 3);
        assert_eq!(get_chat_msgs(&bob, msg.chat_id, 0).await?.len(), 2);
        let received = bob.recv_msg_opt(&sent3).await;
        // No message should actually be added since we already know this message:
        assert!(received.is_none());
        assert_eq!(get_chat_contacts(&bob, msg.chat_id).await?.len(), 3);
        assert_eq!(get_chat_msgs(&bob, msg.chat_id, 0).await?.len(), 2);

        // Claire does not receive the first message, however, due to resending, she has a similar view as Alice and Bob
        let claire = TestContext::new().await;
        claire.configure_addr("claire@example.org").await;
        claire.recv_msg(&sent2).await;
        let msg = claire.recv_msg(&sent3).await;
        assert_eq!(msg.get_text().unwrap(), "alice->bob");
        assert_eq!(get_chat_contacts(&claire, msg.chat_id).await?.len(), 3);
        assert_eq!(get_chat_msgs(&claire, msg.chat_id, 0).await?.len(), 2);
        let msg_from = Contact::get_by_id(&claire, msg.get_from_id()).await?;
        assert_eq!(msg_from.get_addr(), "alice@example.org");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_resend_foreign_message_fails() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let alice_grp = create_group_chat(&alice, ProtectionStatus::Unprotected, "grp").await?;
        add_contact_to_chat(
            &alice,
            alice_grp,
            Contact::create(&alice, "", "bob@example.net").await?,
        )
        .await?;
        let sent1 = alice.send_text(alice_grp, "alice->bob").await;

        let bob = TestContext::new_bob().await;
        let msg = bob.recv_msg(&sent1).await;
        assert!(resend_msgs(&bob, &[msg.id]).await.is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_resend_opportunistically_encryption() -> Result<()> {
        // Alice creates group with Bob and sends an initial message
        let alice = TestContext::new_alice().await;
        let alice_grp = create_group_chat(&alice, ProtectionStatus::Unprotected, "grp").await?;
        add_contact_to_chat(
            &alice,
            alice_grp,
            Contact::create(&alice, "", "bob@example.net").await?,
        )
        .await?;
        let sent1 = alice.send_text(alice_grp, "alice->bob").await;

        // Bob now can send an encrypted message
        let bob = TestContext::new_bob().await;
        let msg = bob.recv_msg(&sent1).await;
        assert!(!msg.get_showpadlock());

        msg.chat_id.accept(&bob).await?;
        let sent2 = bob.send_text(msg.chat_id, "bob->alice").await;
        let msg = bob.get_last_msg().await;
        assert!(msg.get_showpadlock());

        // Bob adds Claire and resends his last message: this will drop encryption in opportunistic chats
        add_contact_to_chat(
            &bob,
            msg.chat_id,
            Contact::create(&bob, "", "claire@example.org").await?,
        )
        .await?;
        let _sent3 = bob.pop_sent_msg().await;
        resend_msgs(&bob, &[sent2.sender_msg_id]).await?;
        let _sent4 = bob.pop_sent_msg().await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_resend_info_message_fails() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let alice_grp = create_group_chat(&alice, ProtectionStatus::Unprotected, "grp").await?;
        add_contact_to_chat(
            &alice,
            alice_grp,
            Contact::create(&alice, "", "bob@example.net").await?,
        )
        .await?;
        alice.send_text(alice_grp, "alice->bob").await;

        add_contact_to_chat(
            &alice,
            alice_grp,
            Contact::create(&alice, "", "claire@example.org").await?,
        )
        .await?;
        let sent2 = alice.pop_sent_msg().await;
        assert!(resend_msgs(&alice, &[sent2.sender_msg_id]).await.is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_can_send_group() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = Contact::create(&alice, "", "bob@f.br").await?;
        let chat_id = ChatId::create_for_contact(&alice, bob).await?;
        let chat = Chat::load_from_db(&alice, chat_id).await?;
        assert!(chat.can_send(&alice).await?);
        let chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "foo").await?;
        assert_eq!(
            Chat::load_from_db(&alice, chat_id)
                .await?
                .can_send(&alice)
                .await?,
            true
        );
        remove_contact_from_chat(&alice, chat_id, ContactId::SELF).await?;
        assert_eq!(
            Chat::load_from_db(&alice, chat_id)
                .await?
                .can_send(&alice)
                .await?,
            false
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_broadcast() -> Result<()> {
        // create two context, send two messages so both know the other
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let chat_alice = alice.create_chat(&bob).await;
        send_text_msg(&alice, chat_alice.id, "hi!".to_string()).await?;
        bob.recv_msg(&alice.pop_sent_msg().await).await;

        let chat_bob = bob.create_chat(&alice).await;
        send_text_msg(&bob, chat_bob.id, "ho!".to_string()).await?;
        let msg = alice.recv_msg(&bob.pop_sent_msg().await).await;
        assert!(msg.get_showpadlock());

        // test broadcast list
        let broadcast_id = create_broadcast_list(&alice).await?;
        add_contact_to_chat(
            &alice,
            broadcast_id,
            get_chat_contacts(&alice, chat_bob.id).await?.pop().unwrap(),
        )
        .await?;
        let chat = Chat::load_from_db(&alice, broadcast_id).await?;
        assert_eq!(chat.typ, Chattype::Broadcast);
        assert_eq!(chat.name, stock_str::broadcast_list(&alice).await);
        assert!(!chat.is_self_talk());

        send_text_msg(&alice, broadcast_id, "ola!".to_string()).await?;
        let msg = alice.get_last_msg().await;
        assert_eq!(msg.chat_id, chat.id);

        let msg = bob.recv_msg(&alice.pop_sent_msg().await).await;
        assert_eq!(msg.get_text(), Some("ola!".to_string()));
        assert!(!msg.get_showpadlock()); // avoid leaking recipients in encryption data
        let chat = Chat::load_from_db(&bob, msg.chat_id).await?;
        assert_eq!(chat.typ, Chattype::Single);
        assert_eq!(chat.id, chat_bob.id);
        assert!(!chat.is_self_talk());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_for_contact_with_blocked() -> Result<()> {
        let t = TestContext::new().await;
        let (contact_id, _) = Contact::add_or_lookup(
            &t,
            "",
            ContactAddress::new("foo@bar.org")?,
            Origin::ManuallyCreated,
        )
        .await?;

        // create a blocked chat
        let chat_id_orig =
            ChatId::create_for_contact_with_blocked(&t, contact_id, Blocked::Yes).await?;
        assert!(!chat_id_orig.is_special());
        let chat = Chat::load_from_db(&t, chat_id_orig).await?;
        assert_eq!(chat.blocked, Blocked::Yes);

        // repeating the call, the same chat must still be blocked
        let chat_id = ChatId::create_for_contact_with_blocked(&t, contact_id, Blocked::Yes).await?;
        assert_eq!(chat_id, chat_id_orig);
        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert_eq!(chat.blocked, Blocked::Yes);

        // already created chats are unblocked if requested
        let chat_id = ChatId::create_for_contact_with_blocked(&t, contact_id, Blocked::Not).await?;
        assert_eq!(chat_id, chat_id_orig);
        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert_eq!(chat.blocked, Blocked::Not);

        // however, already created chats are not re-blocked
        let chat_id = ChatId::create_for_contact_with_blocked(&t, contact_id, Blocked::Yes).await?;
        assert_eq!(chat_id, chat_id_orig);
        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert_eq!(chat.blocked, Blocked::Not);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_chat_get_encryption_info() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let contact_bob = Contact::create(&alice, "Bob", "bob@example.net").await?;
        let contact_fiona = Contact::create(&alice, "", "fiona@example.net").await?;

        let chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "Group").await?;
        assert_eq!(chat_id.get_encryption_info(&alice).await?, "");

        add_contact_to_chat(&alice, chat_id, contact_bob).await?;
        assert_eq!(
            chat_id.get_encryption_info(&alice).await?,
            "No encryption:\n\
            bob@example.net"
        );

        add_contact_to_chat(&alice, chat_id, contact_fiona).await?;
        assert_eq!(
            chat_id.get_encryption_info(&alice).await?,
            "No encryption:\n\
            fiona@example.net\n\
            bob@example.net"
        );

        let direct_chat = bob.create_chat(&alice).await;
        send_text_msg(&bob, direct_chat.id, "Hello!".to_string()).await?;
        alice.recv_msg(&bob.pop_sent_msg().await).await;

        assert_eq!(
            chat_id.get_encryption_info(&alice).await?,
            "No encryption:\n\
            fiona@example.net\n\
            \n\
            End-to-end encryption preferred:\n\
            bob@example.net"
        );

        bob.set_config(Config::E2eeEnabled, Some("0")).await?;
        send_text_msg(&bob, direct_chat.id, "Hello!".to_string()).await?;
        alice.recv_msg(&bob.pop_sent_msg().await).await;

        assert_eq!(
            chat_id.get_encryption_info(&alice).await?,
            "No encryption:\n\
            fiona@example.net\n\
            \n\
            End-to-end encryption available:\n\
            bob@example.net"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_chat_media() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id1 = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let chat_id2 = create_group_chat(&t, ProtectionStatus::Unprotected, "bar").await?;

        assert_eq!(
            get_chat_media(
                &t,
                Some(chat_id1),
                Viewtype::Image,
                Viewtype::Sticker,
                Viewtype::Unknown
            )
            .await?
            .len(),
            0
        );

        async fn send_media(
            t: &TestContext,
            chat_id: ChatId,
            msg_type: Viewtype,
            name: &str,
            bytes: &[u8],
        ) -> Result<MsgId> {
            let file = t.get_blobdir().join(name);
            tokio::fs::write(&file, bytes).await?;
            let mut msg = Message::new(msg_type);
            msg.set_file(file.to_str().unwrap(), None);
            send_msg(t, chat_id, &mut msg).await
        }

        send_media(
            &t,
            chat_id1,
            Viewtype::Image,
            "a.jpg",
            include_bytes!("../test-data/image/rectangle200x180-rotated.jpg"),
        )
        .await?;
        send_media(
            &t,
            chat_id1,
            Viewtype::Sticker,
            "b.png",
            include_bytes!("../test-data/image/avatar64x64.png"),
        )
        .await?;
        send_media(
            &t,
            chat_id2,
            Viewtype::Image,
            "c.jpg",
            include_bytes!("../test-data/image/avatar64x64.png"),
        )
        .await?;
        send_media(
            &t,
            chat_id2,
            Viewtype::Webxdc,
            "d.xdc",
            include_bytes!("../test-data/webxdc/minimal.xdc"),
        )
        .await?;

        assert_eq!(
            get_chat_media(
                &t,
                Some(chat_id1),
                Viewtype::Image,
                Viewtype::Unknown,
                Viewtype::Unknown,
            )
            .await?
            .len(),
            1
        );
        assert_eq!(
            get_chat_media(
                &t,
                Some(chat_id1),
                Viewtype::Sticker,
                Viewtype::Unknown,
                Viewtype::Unknown,
            )
            .await?
            .len(),
            1
        );
        assert_eq!(
            get_chat_media(
                &t,
                Some(chat_id1),
                Viewtype::Sticker,
                Viewtype::Image,
                Viewtype::Unknown,
            )
            .await?
            .len(),
            2
        );
        assert_eq!(
            get_chat_media(
                &t,
                Some(chat_id2),
                Viewtype::Webxdc,
                Viewtype::Unknown,
                Viewtype::Unknown,
            )
            .await?
            .len(),
            1
        );
        assert_eq!(
            get_chat_media(
                &t,
                None,
                Viewtype::Image,
                Viewtype::Unknown,
                Viewtype::Unknown,
            )
            .await?
            .len(),
            2
        );
        assert_eq!(
            get_chat_media(
                &t,
                None,
                Viewtype::Image,
                Viewtype::Sticker,
                Viewtype::Unknown,
            )
            .await?
            .len(),
            3
        );
        assert_eq!(
            get_chat_media(
                &t,
                None,
                Viewtype::Image,
                Viewtype::Sticker,
                Viewtype::Webxdc,
            )
            .await?
            .len(),
            4
        );

        Ok(())
    }
}

//! # Messages and their identifiers

use anyhow::{ensure, Error};
use async_std::path::{Path, PathBuf};
use async_std::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::chat::{self, Chat, ChatId};
use crate::config::Config;
use crate::constants::{
    Blocked, Chattype, VideochatType, Viewtype, DC_CHAT_ID_DEADDROP, DC_CHAT_ID_TRASH,
    DC_CONTACT_ID_INFO, DC_CONTACT_ID_LAST_SPECIAL, DC_CONTACT_ID_SELF, DC_MAX_GET_INFO_LEN,
    DC_MAX_GET_TEXT_LEN, DC_MSG_ID_LAST_SPECIAL,
};
use crate::contact::{Contact, Origin};
use crate::context::Context;
use crate::dc_tools::{
    dc_get_filebytes, dc_get_filemeta, dc_gm2local_offset, dc_read_file, dc_timestamp_to_str,
    dc_truncate, time,
};
use crate::ephemeral::Timer as EphemeralTimer;
use crate::events::EventType;
use crate::job::{self, Action};
use crate::log::LogExt;
use crate::lot::{Lot, LotState, Meaning};
use crate::mimeparser::{FailureReport, SystemMessage};
use crate::param::{Param, Params};
use crate::pgp::split_armored_data;
use crate::stock_str;
use std::collections::BTreeMap;

// In practice, the user additionally cuts the string themselves
// pixel-accurate.
const SUMMARY_CHARACTERS: usize = 160;

/// Message ID, including reserved IDs.
///
/// Some message IDs are reserved to identify special message types.
/// This type can represent both the special as well as normal
/// messages.
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct MsgId(i64);

impl MsgId {
    /// Create a new [MsgId].
    pub fn new(id: i64) -> MsgId {
        MsgId(id)
    }

    /// Create a new unset [MsgId].
    pub fn new_unset() -> MsgId {
        MsgId(0)
    }

    /// Whether the message ID signifies a special message.
    ///
    /// This kind of message ID can not be used for real messages.
    pub fn is_special(self) -> bool {
        self.0 <= DC_MSG_ID_LAST_SPECIAL
    }

    /// Whether the message ID is unset.
    ///
    /// When a message is created it initially has a ID of `0`, which
    /// is filled in by a real message ID once the message is saved in
    /// the database.  This returns true while the message has not
    /// been saved and thus not yet been given an actual message ID.
    ///
    /// When this is `true`, [MsgId::is_special] will also always be
    /// `true`.
    pub fn is_unset(self) -> bool {
        self.0 == 0
    }

    /// Returns message state.
    pub async fn get_state(self, context: &Context) -> crate::sql::Result<MessageState> {
        let result = context
            .sql
            .query_get_value(sqlx::query("SELECT state FROM msgs WHERE id=?").bind(self))
            .await?
            .unwrap_or_default();
        Ok(result)
    }

    /// Returns Some if the message needs to be moved from `folder`.
    /// If yes, returns `ConfiguredInboxFolder`, `ConfiguredMvboxFolder` or `ConfiguredSentboxFolder`,
    /// depending on where the message should be moved
    pub async fn needs_move(
        self,
        context: &Context,
        folder: &str,
    ) -> Result<Option<Config>, Error> {
        use Config::*;
        if context.is_mvbox(folder).await? {
            return Ok(None);
        }

        let msg = Message::load_from_db(context, self).await?;

        if context.is_spam_folder(folder).await? {
            return if msg.chat_blocked == Blocked::Not {
                if self.needs_move_to_mvbox(context, &msg).await? {
                    Ok(Some(ConfiguredMvboxFolder))
                } else {
                    Ok(Some(ConfiguredInboxFolder))
                }
            } else {
                // Blocked/deaddrop message in the spam folder, leave it there
                Ok(None)
            };
        }

        if self.needs_move_to_mvbox(context, &msg).await? {
            Ok(Some(ConfiguredMvboxFolder))
        } else if msg.state.is_outgoing()
                && msg.is_dc_message == MessengerMessage::Yes
                && !msg.is_setupmessage()
                && msg.to_id != DC_CONTACT_ID_SELF // Leave self-chat-messages in the inbox, not sure about this
                && context.is_inbox(folder).await?
                && context.get_config_bool(SentboxMove).await?
                && context.get_config(ConfiguredSentboxFolder).await?.is_some()
        {
            Ok(Some(ConfiguredSentboxFolder))
        } else {
            Ok(None)
        }
    }

    async fn needs_move_to_mvbox(self, context: &Context, msg: &Message) -> Result<bool, Error> {
        if !context.get_config_bool(Config::MvboxMove).await? {
            return Ok(false);
        }

        if msg.is_setupmessage() {
            // do not move setup messages;
            // there may be a non-delta device that wants to handle it
            return Ok(false);
        }

        match msg.is_dc_message {
            MessengerMessage::No => Ok(false),
            MessengerMessage::Yes | MessengerMessage::Reply => Ok(true),
        }
    }

    /// Put message into trash chat and delete message text.
    ///
    /// It means the message is deleted locally, but not on the server.
    /// We keep some infos to
    /// 1. not download the same message again
    /// 2. be able to delete the message on the server if we want to
    pub async fn trash(self, context: &Context) -> crate::sql::Result<()> {
        let chat_id = DC_CHAT_ID_TRASH;
        context.sql.execute(
            sqlx::query(
                // If you change which information is removed here, also change delete_expired_messages() and
                // which information dc_receive_imf::add_parts() still adds to the db if the chat_id is TRASH
                "UPDATE msgs SET chat_id=?, txt='', subject='', txt_raw='', mime_headers='', from_id=0, to_id=0, param='' WHERE id=?")
                .bind(chat_id)
                .bind(self)
        ).await?;

        Ok(())
    }

    /// Deletes a message and corresponding MDNs from the database.
    pub async fn delete_from_db(self, context: &Context) -> crate::sql::Result<()> {
        // We don't use transactions yet, so remove MDNs first to make
        // sure they are not left while the message is deleted.
        context
            .sql
            .execute(sqlx::query("DELETE FROM msgs_mdns WHERE msg_id=?;").bind(self))
            .await?;
        context
            .sql
            .execute(sqlx::query("DELETE FROM msgs WHERE id=?;").bind(self))
            .await?;
        Ok(())
    }

    /// Removes IMAP server UID and folder from the database record.
    ///
    /// It is used to avoid trying to remove the message from the
    /// server multiple times when there are multiple message records
    /// pointing to the same server UID.
    pub(crate) async fn unlink(self, context: &Context) -> crate::sql::Result<()> {
        context
            .sql
            .execute(
                sqlx::query(
                    "UPDATE msgs \
             SET server_folder='', server_uid=0 \
             WHERE id=?",
                )
                .bind(self),
            )
            .await?;
        Ok(())
    }

    /// Bad evil escape hatch.
    ///
    /// Avoid using this, eventually types should be cleaned up enough
    /// that it is no longer necessary.
    pub fn to_i64(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for MsgId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Msg#{}", self.0)
    }
}

/// Message ID was invalid.
///
/// This usually occurs when trying to use a message ID of
/// [DC_MSG_ID_LAST_SPECIAL] or below in a situation where this is not
/// possible.
#[derive(Debug, thiserror::Error)]
#[error("Invalid Message ID.")]
pub struct InvalidMsgId;

#[derive(
    Debug, Copy, Clone, PartialEq, FromPrimitive, ToPrimitive, Serialize, Deserialize, sqlx::Type,
)]
#[repr(i8)]
pub(crate) enum MessengerMessage {
    No = 0,
    Yes = 1,

    /// No, but reply to messenger message.
    Reply = 2,
}

impl Default for MessengerMessage {
    fn default() -> Self {
        Self::No
    }
}

/// An object representing a single message in memory.
/// The message object is not updated.
/// If you want an update, you have to recreate the object.
///
/// to check if a mail was sent, use dc_msg_is_sent()
/// approx. max. length returned by dc_msg_get_text()
/// approx. max. length returned by dc_get_msg_info()
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    pub(crate) id: MsgId,
    pub(crate) from_id: i64,
    pub(crate) to_id: i64,
    pub(crate) chat_id: ChatId,
    pub(crate) viewtype: Viewtype,
    pub(crate) state: MessageState,
    pub(crate) hidden: bool,
    pub(crate) timestamp_sort: i64,
    pub(crate) timestamp_sent: i64,
    pub(crate) timestamp_rcvd: i64,
    pub(crate) ephemeral_timer: EphemeralTimer,
    pub(crate) ephemeral_timestamp: i64,
    pub(crate) text: Option<String>,
    pub(crate) subject: String,
    pub(crate) rfc724_mid: String,
    pub(crate) in_reply_to: Option<String>,
    pub(crate) server_folder: Option<String>,
    pub(crate) server_uid: u32,
    pub(crate) is_dc_message: MessengerMessage,
    pub(crate) mime_modified: bool,
    pub(crate) chat_blocked: Blocked,
    pub(crate) location_id: u32,
    pub(crate) error: Option<String>,
    pub(crate) param: Params,
}

impl Message {
    pub fn new(viewtype: Viewtype) -> Self {
        Message {
            viewtype,
            ..Default::default()
        }
    }

    pub async fn load_from_db(context: &Context, id: MsgId) -> Result<Message, Error> {
        ensure!(
            !id.is_special(),
            "Can not load special message IDs from DB."
        );
        let row = context
            .sql
            .fetch_one(
                sqlx::query(concat!(
                    "SELECT",
                    "    m.id AS id,",
                    "    rfc724_mid AS rfc724mid,",
                    "    m.mime_in_reply_to AS mime_in_reply_to,",
                    "    m.server_folder AS server_folder,",
                    "    m.server_uid AS server_uid,",
                    "    m.chat_id AS chat_id,",
                    "    m.from_id AS from_id,",
                    "    m.to_id AS to_id,",
                    "    m.timestamp AS timestamp,",
                    "    m.timestamp_sent AS timestamp_sent,",
                    "    m.timestamp_rcvd AS timestamp_rcvd,",
                    "    m.ephemeral_timer AS ephemeral_timer,",
                    "    m.ephemeral_timestamp AS ephemeral_timestamp,",
                    "    m.type AS type,",
                    "    m.state AS state,",
                    "    m.error AS error,",
                    "    m.msgrmsg AS msgrmsg,",
                    "    m.mime_modified AS mime_modified,",
                    "    m.txt AS txt,",
                    "    m.subject AS subject,",
                    "    m.param AS param,",
                    "    m.hidden AS hidden,",
                    "    m.location_id AS location,",
                    "    c.blocked AS blocked",
                    " FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id",
                    " WHERE m.id=?;"
                ))
                .bind(id),
            )
            .await?;

        let mut msg = Message::default();
        msg.id = row.try_get("id")?;
        msg.rfc724_mid = row.try_get("rfc724mid")?;
        msg.in_reply_to = row.try_get("mime_in_reply_to")?;
        msg.server_folder = row.try_get("server_folder")?;
        msg.server_uid = row.try_get::<i64, _>("server_uid")? as u32;
        msg.chat_id = row.try_get("chat_id")?;
        msg.from_id = row.try_get("from_id")?;
        msg.to_id = row.try_get("to_id")?;
        msg.timestamp_sort = row.try_get("timestamp")?;
        msg.timestamp_sent = row.try_get("timestamp_sent")?;
        msg.timestamp_rcvd = row.try_get("timestamp_rcvd")?;
        msg.ephemeral_timer = row.try_get("ephemeral_timer")?;
        msg.ephemeral_timestamp = row.try_get("ephemeral_timestamp")?;
        msg.viewtype = row.try_get("type")?;
        msg.state = row.try_get("state")?;
        let error: String = row.try_get("error")?;
        msg.error = Some(error).filter(|error| !error.is_empty());
        msg.is_dc_message = row.try_get("msgrmsg")?;

        let text;
        if let Ok(Some(buf)) = row.try_get::<Option<&[u8]>, _>("txt") {
            if let Ok(t) = String::from_utf8(buf.to_vec()) {
                text = t;
            } else {
                warn!(
                    context,
                    concat!(
                        "dc_msg_load_from_db: could not get ",
                        "text column as non-lossy utf8 id {}"
                    ),
                    id
                );
                text = String::from_utf8_lossy(buf).into_owned();
            }
        } else {
            text = "".to_string();
        }
        msg.text = Some(text);

        msg.param = row
            .try_get::<String, _>("param")?
            .parse()
            .unwrap_or_default();
        msg.hidden = row.try_get("hidden")?;
        msg.location_id = row.try_get("location")?;
        msg.chat_blocked = row
            .try_get::<Option<Blocked>, _>("blocked")?
            .unwrap_or_default();

        Ok(msg)
    }

    pub fn get_filemime(&self) -> Option<String> {
        if let Some(m) = self.param.get(Param::MimeType) {
            return Some(m.to_string());
        } else if let Some(file) = self.param.get(Param::File) {
            if let Some((_, mime)) = guess_msgtype_from_suffix(Path::new(file)) {
                return Some(mime.to_string());
            }
            // we have a file but no mimetype, let's use a generic one
            return Some("application/octet-stream".to_string());
        }
        // no mimetype and no file
        None
    }

    pub fn get_file(&self, context: &Context) -> Option<PathBuf> {
        self.param.get_path(Param::File, context).unwrap_or(None)
    }

    pub async fn try_calc_and_set_dimensions(&mut self, context: &Context) -> Result<(), Error> {
        if chat::msgtype_has_file(self.viewtype) {
            let file_param = self.param.get_path(Param::File, context)?;
            if let Some(path_and_filename) = file_param {
                if (self.viewtype == Viewtype::Image || self.viewtype == Viewtype::Gif)
                    && !self.param.exists(Param::Width)
                {
                    self.param.set_int(Param::Width, 0);
                    self.param.set_int(Param::Height, 0);

                    if let Ok(buf) = dc_read_file(context, path_and_filename).await {
                        if let Ok((width, height)) = dc_get_filemeta(&buf) {
                            self.param.set_int(Param::Width, width as i32);
                            self.param.set_int(Param::Height, height as i32);
                        }
                    }

                    if !self.id.is_unset() {
                        self.update_param(context).await;
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a message has a location bound to it.
    /// These messages are also returned by dc_get_locations()
    /// and the UI may decide to display a special icon beside such messages,
    ///
    /// @memberof Message
    /// @param msg The message object.
    /// @return 1=Message has location bound to it, 0=No location bound to message.
    pub fn has_location(&self) -> bool {
        self.location_id != 0
    }

    /// Set any location that should be bound to the message object.
    /// The function is useful to add a marker to the map
    /// at a position different from the self-location.
    /// You should not call this function
    /// if you want to bind the current self-location to a message;
    /// this is done by dc_set_location() and dc_send_locations_to_chat().
    ///
    /// Typically results in the event #DC_EVENT_LOCATION_CHANGED with
    /// contact_id set to DC_CONTACT_ID_SELF.
    ///
    /// @param latitude North-south position of the location.
    /// @param longitude East-west position of the location.
    pub fn set_location(&mut self, latitude: f64, longitude: f64) {
        if latitude == 0.0 && longitude == 0.0 {
            return;
        }

        self.param.set_float(Param::SetLatitude, latitude);
        self.param.set_float(Param::SetLongitude, longitude);
    }

    pub fn get_timestamp(&self) -> i64 {
        if 0 != self.timestamp_sent {
            self.timestamp_sent
        } else {
            self.timestamp_sort
        }
    }

    pub fn get_id(&self) -> MsgId {
        self.id
    }

    pub fn get_from_id(&self) -> i64 {
        self.from_id
    }

    /// get the chat-id,
    /// if the message is a contact request, the DC_CHAT_ID_DEADDROP is returned.
    pub fn get_chat_id(&self) -> ChatId {
        if self.chat_blocked != Blocked::Not {
            DC_CHAT_ID_DEADDROP
        } else {
            self.chat_id
        }
    }

    /// get the chat-id, also when the message is still a contact request.
    /// DC_CHAT_ID_DEADDROP is never returned.
    pub fn get_real_chat_id(&self) -> ChatId {
        self.chat_id
    }

    pub fn get_viewtype(&self) -> Viewtype {
        self.viewtype
    }

    pub fn get_state(&self) -> MessageState {
        self.state
    }

    pub fn get_received_timestamp(&self) -> i64 {
        self.timestamp_rcvd
    }

    pub fn get_sort_timestamp(&self) -> i64 {
        self.timestamp_sort
    }

    pub fn get_text(&self) -> Option<String> {
        self.text
            .as_ref()
            .map(|text| dc_truncate(text, DC_MAX_GET_TEXT_LEN).to_string())
    }

    pub fn get_subject(&self) -> &str {
        &self.subject
    }

    pub fn get_filename(&self) -> Option<String> {
        self.param
            .get(Param::File)
            .and_then(|file| Path::new(file).file_name())
            .map(|name| name.to_string_lossy().to_string())
    }

    pub async fn get_filebytes(&self, context: &Context) -> u64 {
        match self.param.get_path(Param::File, context) {
            Ok(Some(path)) => dc_get_filebytes(context, &path).await,
            Ok(None) => 0,
            Err(_) => 0,
        }
    }

    pub fn get_width(&self) -> i32 {
        self.param.get_int(Param::Width).unwrap_or_default()
    }

    pub fn get_height(&self) -> i32 {
        self.param.get_int(Param::Height).unwrap_or_default()
    }

    pub fn get_duration(&self) -> i32 {
        self.param.get_int(Param::Duration).unwrap_or_default()
    }

    pub fn get_showpadlock(&self) -> bool {
        self.param.get_int(Param::GuaranteeE2ee).unwrap_or_default() != 0
    }

    pub fn get_ephemeral_timer(&self) -> EphemeralTimer {
        self.ephemeral_timer
    }

    pub fn get_ephemeral_timestamp(&self) -> i64 {
        self.ephemeral_timestamp
    }

    pub async fn get_summary(&mut self, context: &Context, chat: Option<&Chat>) -> Lot {
        let mut ret = Lot::new();

        let chat_loaded: Chat;
        let chat = if let Some(chat) = chat {
            chat
        } else if let Ok(chat) = Chat::load_from_db(context, self.chat_id).await {
            chat_loaded = chat;
            &chat_loaded
        } else {
            return ret;
        };

        let contact = if self.from_id != DC_CONTACT_ID_SELF {
            match chat.typ {
                Chattype::Group | Chattype::Mailinglist => {
                    Contact::get_by_id(context, self.from_id).await.ok()
                }
                Chattype::Single | Chattype::Undefined => None,
            }
        } else {
            None
        };

        ret.fill(self, chat, contact.as_ref(), context).await;

        ret
    }

    pub async fn get_summarytext(&self, context: &Context, approx_characters: usize) -> String {
        get_summarytext_by_raw(
            self.viewtype,
            self.text.as_ref(),
            &self.param,
            approx_characters,
            context,
        )
        .await
    }

    // It's a little unfortunate that the UI has to first call dc_msg_get_override_sender_name() and then if it was NULL, call
    // dc_contact_get_display_name() but this was the best solution:
    // - We could load a Contact struct from the db here to call get_display_name() instead of returning None, but then we had a db
    //   call everytime (and this fn is called a lot while the user is scrolling through a group), so performance would be bad
    // - We could pass both a Contact struct and a Message struct in the FFI, but at least on Android we would need to handle raw
    //   C-data in the Java code (i.e. a `long` storing a C pointer)
    // - We can't make a param `SenderDisplayname` for messages as sometimes the display name of a contact changes, and we want to show
    //   the same display name over all messages from the same sender.
    pub fn get_override_sender_name(&self) -> Option<String> {
        if let Some(name) = self.param.get(Param::OverrideSenderDisplayname) {
            Some(name.to_string())
        } else {
            None
        }
    }

    // Exposing this function over the ffi instead of get_override_sender_name() would mean that at least Android Java code has
    // to handle raw C-data (as it is done for dc_msg_get_summary())
    pub fn get_sender_name(&self, contact: &Contact) -> String {
        self.get_override_sender_name()
            .unwrap_or_else(|| contact.get_display_name().to_string())
    }

    pub fn has_deviating_timestamp(&self) -> bool {
        let cnv_to_local = dc_gm2local_offset();
        let sort_timestamp = self.get_sort_timestamp() as i64 + cnv_to_local;
        let send_timestamp = self.get_timestamp() as i64 + cnv_to_local;

        sort_timestamp / 86400 != send_timestamp / 86400
    }

    pub fn is_sent(&self) -> bool {
        self.state as i32 >= MessageState::OutDelivered as i32
    }

    pub fn is_forwarded(&self) -> bool {
        0 != self.param.get_int(Param::Forwarded).unwrap_or_default()
    }

    pub fn is_info(&self) -> bool {
        let cmd = self.param.get_cmd();
        self.from_id == DC_CONTACT_ID_INFO
            || self.to_id == DC_CONTACT_ID_INFO
            || cmd != SystemMessage::Unknown && cmd != SystemMessage::AutocryptSetupMessage
    }

    pub fn get_info_type(&self) -> SystemMessage {
        self.param.get_cmd()
    }

    pub fn is_system_message(&self) -> bool {
        let cmd = self.param.get_cmd();
        cmd != SystemMessage::Unknown
    }

    /// Whether the message is still being created.
    ///
    /// Messages with attachments might be created before the
    /// attachment is ready.  In this case some more restrictions on
    /// the attachment apply, e.g. if the file to be attached is still
    /// being written to or otherwise will still change it can not be
    /// copied to the blobdir.  Thus those attachments need to be
    /// created immediately in the blobdir with a valid filename.
    pub fn is_increation(&self) -> bool {
        chat::msgtype_has_file(self.viewtype) && self.state == MessageState::OutPreparing
    }

    pub fn is_setupmessage(&self) -> bool {
        if self.viewtype != Viewtype::File {
            return false;
        }

        self.param.get_cmd() == SystemMessage::AutocryptSetupMessage
    }

    pub async fn get_setupcodebegin(&self, context: &Context) -> Option<String> {
        if !self.is_setupmessage() {
            return None;
        }

        if let Some(filename) = self.get_file(context) {
            if let Ok(ref buf) = dc_read_file(context, filename).await {
                if let Ok((typ, headers, _)) = split_armored_data(buf) {
                    if typ == pgp::armor::BlockType::Message {
                        return headers.get(crate::pgp::HEADER_SETUPCODE).cloned();
                    }
                }
            }
        }

        None
    }

    // add room to a webrtc_instance as defined by the corresponding config-value;
    // the result may still be prefixed by the type
    pub fn create_webrtc_instance(instance: &str, room: &str) -> String {
        let (videochat_type, mut url) = Message::parse_webrtc_instance(instance);

        // make sure, there is a scheme in the url
        if !url.contains(':') {
            url = format!("https://{}", url);
        }

        // add/replace room
        let url = if url.contains("$ROOM") {
            url.replace("$ROOM", room)
        } else if url.contains("$NOROOM") {
            // there are some usecases where a separate room is not needed to use a service
            // eg. if you let in people manually anyway, see discussion at
            // https://support.delta.chat/t/videochat-with-webex/1412/4 .
            // hacks as hiding the room behind `#` are not reliable, therefore,
            // these services are supported by adding the string `$NOROOM` to the url.
            url.replace("$NOROOM", "")
        } else {
            // if there nothing that would separate the room, add a slash as a separator;
            // this way, urls can be given as "https://meet.jit.si" as well as "https://meet.jit.si/"
            let maybe_slash = if url.ends_with('/')
                || url.ends_with('?')
                || url.ends_with('#')
                || url.ends_with('=')
            {
                ""
            } else {
                "/"
            };
            format!("{}{}{}", url, maybe_slash, room)
        };

        // re-add and normalize type
        match videochat_type {
            VideochatType::BasicWebrtc => format!("basicwebrtc:{}", url),
            VideochatType::Jitsi => format!("jitsi:{}", url),
            VideochatType::Unknown => url,
        }
    }

    /// split a webrtc_instance as defined by the corresponding config-value into a type and a url
    pub fn parse_webrtc_instance(instance: &str) -> (VideochatType, String) {
        let instance: String = instance.split_whitespace().collect();
        let mut split = instance.splitn(2, ':');
        let type_str = split.next().unwrap_or_default().to_lowercase();
        let url = split.next();
        match type_str.as_str() {
            "basicwebrtc" => (
                VideochatType::BasicWebrtc,
                url.unwrap_or_default().to_string(),
            ),
            "jitsi" => (VideochatType::Jitsi, url.unwrap_or_default().to_string()),
            _ => (VideochatType::Unknown, instance.to_string()),
        }
    }

    pub fn get_videochat_url(&self) -> Option<String> {
        if self.viewtype == Viewtype::VideochatInvitation {
            if let Some(instance) = self.param.get(Param::WebrtcRoom) {
                return Some(Message::parse_webrtc_instance(instance).1);
            }
        }
        None
    }

    pub fn get_videochat_type(&self) -> Option<VideochatType> {
        if self.viewtype == Viewtype::VideochatInvitation {
            if let Some(instance) = self.param.get(Param::WebrtcRoom) {
                return Some(Message::parse_webrtc_instance(instance).0);
            }
        }
        None
    }

    pub fn set_text(&mut self, text: Option<String>) {
        self.text = text;
    }

    pub fn set_file(&mut self, file: impl AsRef<str>, filemime: Option<&str>) {
        self.param.set(Param::File, file);
        if let Some(filemime) = filemime {
            self.param.set(Param::MimeType, filemime);
        }
    }

    /// Set different sender name for a message.
    /// This overrides the name set by the `set_config()`-option `displayname`.
    pub fn set_override_sender_name(&mut self, name: Option<String>) {
        if let Some(name) = name {
            self.param.set(Param::OverrideSenderDisplayname, name);
        } else {
            self.param.remove(Param::OverrideSenderDisplayname);
        }
    }

    pub fn set_dimension(&mut self, width: i32, height: i32) {
        self.param.set_int(Param::Width, width);
        self.param.set_int(Param::Height, height);
    }

    pub fn set_duration(&mut self, duration: i32) {
        self.param.set_int(Param::Duration, duration);
    }

    pub async fn latefiling_mediasize(
        &mut self,
        context: &Context,
        width: i32,
        height: i32,
        duration: i32,
    ) {
        if width > 0 && height > 0 {
            self.param.set_int(Param::Width, width);
            self.param.set_int(Param::Height, height);
        }
        if duration > 0 {
            self.param.set_int(Param::Duration, duration);
        }
        self.update_param(context).await;
    }

    /// Sets message quote.
    ///
    /// Message-Id is used to set Reply-To field, message text is used for quote.
    ///
    /// Encryption is required if quoted message was encrypted.
    ///
    /// The message itself is not required to exist in the database,
    /// it may even be deleted from the database by the time the message is prepared.
    pub async fn set_quote(&mut self, context: &Context, quote: &Message) -> Result<(), Error> {
        ensure!(
            !quote.rfc724_mid.is_empty(),
            "Message without Message-Id cannot be quoted"
        );
        self.in_reply_to = Some(quote.rfc724_mid.clone());

        if quote
            .param
            .get_bool(Param::GuaranteeE2ee)
            .unwrap_or_default()
        {
            self.param.set(Param::GuaranteeE2ee, "1");
        }

        let text = quote.get_text().unwrap_or_default();
        self.param.set(
            Param::Quote,
            if text.is_empty() {
                // Use summary, similar to "Image" to avoid sending empty quote.
                quote.get_summarytext(context, 500).await
            } else {
                text
            },
        );

        Ok(())
    }

    pub fn quoted_text(&self) -> Option<String> {
        self.param.get(Param::Quote).map(|s| s.to_string())
    }

    pub async fn quoted_message(&self, context: &Context) -> Result<Option<Message>, Error> {
        if self.param.get(Param::Quote).is_some() {
            if let Some(in_reply_to) = &self.in_reply_to {
                if let Some((_, _, msg_id)) = rfc724_mid_exists(context, in_reply_to).await? {
                    let msg = Message::load_from_db(context, msg_id).await?;
                    return if msg.chat_id.is_trash() {
                        // If message is already moved to trash chat, pretend it does not exist.
                        Ok(None)
                    } else {
                        Ok(Some(msg))
                    };
                }
            }
        }
        Ok(None)
    }

    pub async fn update_param(&self, context: &Context) {
        context
            .sql
            .execute(
                sqlx::query("UPDATE msgs SET param=? WHERE id=?;")
                    .bind(self.param.to_string())
                    .bind(self.id),
            )
            .await
            .ok_or_log(context);
    }

    pub(crate) async fn update_subject(&self, context: &Context) {
        context
            .sql
            .execute(
                sqlx::query("UPDATE msgs SET subject=? WHERE id=?;")
                    .bind(&self.subject)
                    .bind(&self.id),
            )
            .await
            .ok_or_log(context);
    }

    /// Gets the error status of the message.
    ///
    /// A message can have an associated error status if something went wrong when sending or
    /// receiving message itself.  The error status is free-form text and should not be further parsed,
    /// rather it's presence is meant to indicate *something* went wrong with the message and the
    /// text of the error is detailed information on what.
    ///
    /// Some common reasons error can be associated with messages are:
    /// * Lack of valid signature on an e2ee message, usually for received messages.
    /// * Failure to decrypt an e2ee message, usually for received messages.
    /// * When a message could not be delivered to one or more recipients the non-delivery
    ///    notification text can be stored in the error status.
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

#[derive(Display, Debug, FromPrimitive)]
pub enum ContactRequestDecision {
    StartChat = 0,
    Block = 1,
    NotNow = 2,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    Serialize,
    Deserialize,
    sqlx::Type,
)]
#[repr(i32)]
pub enum MessageState {
    Undefined = 0,

    /// Incoming *fresh* message. Fresh messages are neither noticed
    /// nor seen and are typically shown in notifications.
    InFresh = 10,

    /// Incoming *noticed* message. E.g. chat opened but message not
    /// yet read - noticed messages are not counted as unread but did
    /// not marked as read nor resulted in MDNs.
    InNoticed = 13,

    /// Incoming message, really *seen* by the user. Marked as read on
    /// IMAP and MDN may be sent.
    InSeen = 16,

    /// For files which need time to be prepared before they can be
    /// sent, the message enters this state before
    /// OutPending.
    OutPreparing = 18,

    /// Message saved as draft.
    OutDraft = 19,

    /// The user has pressed the "send" button but the message is not
    /// yet sent and is pending in some way. Maybe we're offline (no
    /// checkmark).
    OutPending = 20,

    /// *Unrecoverable* error (*recoverable* errors result in pending
    /// messages).
    OutFailed = 24,

    /// Outgoing message successfully delivered to server (one
    /// checkmark). Note, that already delivered messages may get into
    /// the OutFailed state if we get such a hint from the server.
    OutDelivered = 26,

    /// Outgoing message read by the recipient (two checkmarks; this
    /// requires goodwill on the receiver's side)
    OutMdnRcvd = 28,
}

impl Default for MessageState {
    fn default() -> Self {
        MessageState::Undefined
    }
}

impl std::fmt::Display for MessageState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Undefined => "Undefined",
                Self::InFresh => "Fresh",
                Self::InNoticed => "Noticed",
                Self::InSeen => "Seen",
                Self::OutPreparing => "Preparing",
                Self::OutDraft => "Draft",
                Self::OutPending => "Pending",
                Self::OutFailed => "Failed",
                Self::OutDelivered => "Delivered",
                Self::OutMdnRcvd => "Read",
            }
        )
    }
}

impl From<MessageState> for LotState {
    fn from(s: MessageState) -> Self {
        use MessageState::*;
        match s {
            Undefined => LotState::Undefined,
            InFresh => LotState::MsgInFresh,
            InNoticed => LotState::MsgInNoticed,
            InSeen => LotState::MsgInSeen,
            OutPreparing => LotState::MsgOutPreparing,
            OutDraft => LotState::MsgOutDraft,
            OutPending => LotState::MsgOutPending,
            OutFailed => LotState::MsgOutFailed,
            OutDelivered => LotState::MsgOutDelivered,
            OutMdnRcvd => LotState::MsgOutMdnRcvd,
        }
    }
}

impl MessageState {
    pub fn can_fail(self) -> bool {
        use MessageState::*;
        matches!(
            self,
            OutPreparing | OutPending | OutDelivered | OutMdnRcvd // OutMdnRcvd can still fail because it could be a group message and only some recipients failed.
        )
    }
    pub fn is_outgoing(self) -> bool {
        use MessageState::*;
        matches!(
            self,
            OutPreparing | OutDraft | OutPending | OutFailed | OutDelivered | OutMdnRcvd
        )
    }
}

impl Lot {
    /* library-internal */
    /* in practice, the user additionally cuts the string himself pixel-accurate */
    pub async fn fill(
        &mut self,
        msg: &mut Message,
        chat: &Chat,
        contact: Option<&Contact>,
        context: &Context,
    ) {
        if msg.state == MessageState::OutDraft {
            self.text1 = Some(stock_str::draft(context).await);
            self.text1_meaning = Meaning::Text1Draft;
        } else if msg.from_id == DC_CONTACT_ID_SELF {
            if msg.is_info() || chat.is_self_talk() {
                self.text1 = None;
                self.text1_meaning = Meaning::None;
            } else {
                self.text1 = Some(stock_str::self_msg(context).await);
                self.text1_meaning = Meaning::Text1Self;
            }
        } else {
            match chat.typ {
                Chattype::Group | Chattype::Mailinglist => {
                    if msg.is_info() || contact.is_none() {
                        self.text1 = None;
                        self.text1_meaning = Meaning::None;
                    } else {
                        self.text1 = msg
                            .get_override_sender_name()
                            .or_else(|| contact.map(|contact| msg.get_sender_name(contact)));
                        self.text1_meaning = Meaning::Text1Username;
                    }
                }
                Chattype::Single | Chattype::Undefined => {
                    self.text1 = None;
                    self.text1_meaning = Meaning::None;
                }
            }
        }

        let mut text2 = get_summarytext_by_raw(
            msg.viewtype,
            msg.text.as_ref(),
            &msg.param,
            SUMMARY_CHARACTERS,
            context,
        )
        .await;

        if text2.is_empty() && msg.quoted_text().is_some() {
            text2 = stock_str::reply_noun(context).await
        }

        self.text2 = Some(text2);

        self.timestamp = msg.get_timestamp();
        self.state = msg.state.into();
    }
}

/// Call this when the user decided about a deaddrop message ("Do you want to chat with NAME?").
///
/// If the decision is `StartChat`, this will create a new chat and return the chat id.
/// If the decision is `Block`, this will usually block the sender.
/// If the decision is `NotNow`, this will usually mark all messages from this sender as read.
///
/// If the message belongs to a mailing list, makes sure that all messages from this mailing list are
/// blocked or marked as noticed.
///
/// The user should be asked whether they want to chat with the _contact_ belonging to the message;
/// the group names may be really weird when taken from the subject of implicit (= ad-hoc)
/// groups and this may look confusing. Moreover, this function also scales up the origin of the contact.
///
/// If the chat belongs to a mailing list, you can also ask
/// "Would you like to read MAILING LIST NAME in Delta Chat?"
/// (use `Message.get_real_chat_id()` to get the chat-id for the contact request
/// and then `Chat.is_mailing_list()`, `Chat.get_name()` and so on)
pub async fn decide_on_contact_request(
    context: &Context,
    msg_id: MsgId,
    decision: ContactRequestDecision,
) -> Option<ChatId> {
    let msg = match Message::load_from_db(context, msg_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(context, "Can't load message: {}", e);
            return None;
        }
    };

    let chat = match Chat::load_from_db(context, msg.chat_id).await {
        Ok(c) => c,
        Err(e) => {
            warn!(context, "Can't load chat: {}", e);
            return None;
        }
    };

    let mut created_chat_id = None;
    use ContactRequestDecision::*;
    match (decision, chat.is_mailing_list()) {
        (StartChat, _) => match chat::create_by_msg_id(context, msg.id).await {
            Ok(id) => created_chat_id = Some(id),
            Err(e) => warn!(context, "decide_on_contact_request error: {}", e),
        },

        (Block, false) => Contact::block(context, msg.from_id).await,
        (Block, true) => {
            if !msg.chat_id.set_blocked(context, Blocked::Manually).await {
                warn!(context, "Block mailing list failed.")
            }
        }

        (NotNow, false) => Contact::mark_noticed(context, msg.from_id).await,
        (NotNow, true) => {
            if let Err(e) = chat::marknoticed_chat(context, msg.chat_id).await {
                warn!(context, "Marknoticed failed: {}", e)
            }
        }
    }

    // Multiple chats may have changed, so send 0s
    // (performance is not so important because this function is not called very often)
    context.emit_event(EventType::MsgsChanged {
        chat_id: ChatId::new(0),
        msg_id: MsgId::new(0),
    });
    created_chat_id
}

pub async fn get_msg_info(context: &Context, msg_id: MsgId) -> Result<String, Error> {
    let msg = Message::load_from_db(context, msg_id).await?;
    let rawtxt: Option<String> = context
        .sql
        .query_get_value(sqlx::query("SELECT txt_raw FROM msgs WHERE id=?;").bind(msg_id))
        .await?;

    let mut ret = String::new();

    if rawtxt.is_none() {
        ret += &format!("Cannot load message {}.", msg_id);
        return Ok(ret);
    }
    let rawtxt = rawtxt.unwrap_or_default();
    let rawtxt = dc_truncate(rawtxt.trim(), DC_MAX_GET_INFO_LEN);

    let fts = dc_timestamp_to_str(msg.get_timestamp());
    ret += &format!("Sent: {}", fts);

    let name = Contact::load_from_db(context, msg.from_id)
        .await
        .map(|contact| contact.get_name_n_addr())
        .unwrap_or_default();

    ret += &format!(" by {}", name);
    ret += "\n";

    if msg.from_id != DC_CONTACT_ID_SELF {
        let s = dc_timestamp_to_str(if 0 != msg.timestamp_rcvd {
            msg.timestamp_rcvd
        } else {
            msg.timestamp_sort
        });
        ret += &format!("Received: {}", &s);
        ret += "\n";
    }

    if let EphemeralTimer::Enabled { duration } = msg.ephemeral_timer {
        ret += &format!("Ephemeral timer: {}\n", duration);
    }

    if msg.ephemeral_timestamp != 0 {
        ret += &format!(
            "Expires: {}\n",
            dc_timestamp_to_str(msg.ephemeral_timestamp)
        );
    }

    if msg.from_id == DC_CONTACT_ID_INFO || msg.to_id == DC_CONTACT_ID_INFO {
        // device-internal message, no further details needed
        return Ok(ret);
    }

    if let Ok(mut rows) = context
        .sql
        .fetch(
            sqlx::query("SELECT contact_id, timestamp_sent FROM msgs_mdns WHERE msg_id=?;")
                .bind(msg_id),
        )
        .await
        .map(|rows| {
            rows.map(|row| -> sqlx::Result<_> {
                let row = row?;
                let contact_id: i64 = row.try_get(0)?;
                let ts: i64 = row.try_get(1)?;
                Ok((contact_id, ts))
            })
        })
    {
        while let Some(row) = rows.next().await {
            let (contact_id, ts) = row?;

            let fts = dc_timestamp_to_str(ts);
            ret += &format!("Read: {}", fts);

            let name = Contact::load_from_db(context, contact_id)
                .await
                .map(|contact| contact.get_name_n_addr())
                .unwrap_or_default();

            ret += &format!(" by {}", name);
            ret += "\n";
        }
    }

    ret += &format!("State: {}", msg.state);

    if msg.has_location() {
        ret += ", Location sent";
    }

    let e2ee_errors = msg.param.get_int(Param::ErroneousE2ee).unwrap_or_default();

    if 0 != e2ee_errors {
        if 0 != e2ee_errors & 0x2 {
            ret += ", Encrypted, no valid signature";
        }
    } else if 0 != msg.param.get_int(Param::GuaranteeE2ee).unwrap_or_default() {
        ret += ", Encrypted";
    }

    ret += "\n";

    if let Some(error) = msg.error.as_ref() {
        ret += &format!("Error: {}", error);
    }

    if let Some(path) = msg.get_file(context) {
        let bytes = dc_get_filebytes(context, &path).await;
        ret += &format!("\nFile: {}, {}, bytes\n", path.display(), bytes);
    }

    if msg.viewtype != Viewtype::Text {
        ret += "Type: ";
        ret += &format!("{}", msg.viewtype);
        ret += "\n";
        ret += &format!("Mimetype: {}\n", &msg.get_filemime().unwrap_or_default());
    }
    let w = msg.param.get_int(Param::Width).unwrap_or_default();
    let h = msg.param.get_int(Param::Height).unwrap_or_default();
    if w != 0 || h != 0 {
        ret += &format!("Dimension: {} x {}\n", w, h,);
    }
    let duration = msg.param.get_int(Param::Duration).unwrap_or_default();
    if duration != 0 {
        ret += &format!("Duration: {} ms\n", duration,);
    }
    if !rawtxt.is_empty() {
        ret += &format!("\n{}\n", rawtxt);
    }
    if !msg.rfc724_mid.is_empty() {
        ret += &format!("\nMessage-ID: {}", msg.rfc724_mid);
    }
    if let Some(ref server_folder) = msg.server_folder {
        if !server_folder.is_empty() {
            ret += &format!("\nLast seen as: {}/{}", server_folder, msg.server_uid);
        }
    }

    Ok(ret)
}

pub fn guess_msgtype_from_suffix(path: &Path) -> Option<(Viewtype, &str)> {
    let extension: &str = &path.extension()?.to_str()?.to_lowercase();
    let info = match extension {
        // before using viewtype other than Viewtype::File,
        // make sure, all target UIs support that type in the context of the used viewer/player.
        // if in doubt, it is better to default to Viewtype::File that passes handing to an external app.
        // (cmp. https://developer.android.com/guide/topics/media/media-formats )
        "3gp" => (Viewtype::Video, "video/3gpp"),
        "aac" => (Viewtype::Audio, "audio/aac"),
        "avi" => (Viewtype::Video, "video/x-msvideo"),
        "doc" => (Viewtype::File, "application/msword"),
        "docx" => (
            Viewtype::File,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        "epub" => (Viewtype::File, "application/epub+zip"),
        "flac" => (Viewtype::Audio, "audio/flac"),
        "gif" => (Viewtype::Gif, "image/gif"),
        "html" => (Viewtype::File, "text/html"),
        "htm" => (Viewtype::File, "text/html"),
        "ico" => (Viewtype::File, "image/vnd.microsoft.icon"),
        "jar" => (Viewtype::File, "application/java-archive"),
        "jpeg" => (Viewtype::Image, "image/jpeg"),
        "jpe" => (Viewtype::Image, "image/jpeg"),
        "jpg" => (Viewtype::Image, "image/jpeg"),
        "json" => (Viewtype::File, "application/json"),
        "mov" => (Viewtype::Video, "video/quicktime"),
        "m4a" => (Viewtype::Audio, "audio/m4a"),
        "mp3" => (Viewtype::Audio, "audio/mpeg"),
        "mp4" => (Viewtype::Video, "video/mp4"),
        "odp" => (
            Viewtype::File,
            "application/vnd.oasis.opendocument.presentation",
        ),
        "ods" => (
            Viewtype::File,
            "application/vnd.oasis.opendocument.spreadsheet",
        ),
        "odt" => (Viewtype::File, "application/vnd.oasis.opendocument.text"),
        "oga" => (Viewtype::Audio, "audio/ogg"),
        "ogg" => (Viewtype::Audio, "audio/ogg"),
        "ogv" => (Viewtype::File, "video/ogg"),
        "opus" => (Viewtype::File, "audio/ogg"), // not supported eg. on Android 4
        "otf" => (Viewtype::File, "font/otf"),
        "pdf" => (Viewtype::File, "application/pdf"),
        "png" => (Viewtype::Image, "image/png"),
        "rar" => (Viewtype::File, "application/vnd.rar"),
        "rtf" => (Viewtype::File, "application/rtf"),
        "spx" => (Viewtype::File, "audio/ogg"), // Ogg Speex Profile
        "svg" => (Viewtype::File, "image/svg+xml"),
        "tgs" => (Viewtype::Sticker, "application/x-tgsticker"),
        "tiff" => (Viewtype::File, "image/tiff"),
        "tif" => (Viewtype::File, "image/tiff"),
        "ttf" => (Viewtype::File, "font/ttf"),
        "vcard" => (Viewtype::File, "text/vcard"),
        "vcf" => (Viewtype::File, "text/vcard"),
        "wav" => (Viewtype::File, "audio/wav"),
        "weba" => (Viewtype::File, "audio/webm"),
        "webm" => (Viewtype::Video, "video/webm"),
        "webp" => (Viewtype::Image, "image/webp"), // iOS via SDWebImage, Android since 4.0
        "wmv" => (Viewtype::Video, "video/x-ms-wmv"),
        "xhtml" => (Viewtype::File, "application/xhtml+xml"),
        "xlsx" => (
            Viewtype::File,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        "xml" => (Viewtype::File, "application/vnd.ms-excel"),
        "zip" => (Viewtype::File, "application/zip"),
        _ => {
            return None;
        }
    };
    Some(info)
}

pub async fn get_mime_headers(context: &Context, msg_id: MsgId) -> Result<Option<String>, Error> {
    let headers = context
        .sql
        .query_get_value(sqlx::query("SELECT mime_headers FROM msgs WHERE id=?;").bind(msg_id))
        .await?;
    Ok(headers)
}

pub async fn delete_msgs(context: &Context, msg_ids: &[MsgId]) {
    for msg_id in msg_ids.iter() {
        if let Ok(msg) = Message::load_from_db(context, *msg_id).await {
            if msg.location_id > 0 {
                delete_poi_location(context, msg.location_id).await;
            }
        }
        if let Err(err) = msg_id.trash(context).await {
            error!(context, "Unable to trash message {}: {}", msg_id, err);
        }
        job::add(
            context,
            job::Job::new(Action::DeleteMsgOnImap, msg_id.to_i64(), Params::new(), 0),
        )
        .await;
    }

    if !msg_ids.is_empty() {
        context.emit_event(EventType::MsgsChanged {
            chat_id: ChatId::new(0),
            msg_id: MsgId::new(0),
        });
        job::kill_action(context, Action::Housekeeping).await;
        job::add(
            context,
            job::Job::new(Action::Housekeeping, 0, Params::new(), 10),
        )
        .await;
    }
}

async fn delete_poi_location(context: &Context, location_id: i64) -> bool {
    context
        .sql
        .execute(
            sqlx::query("DELETE FROM locations WHERE independent = 1 AND id=?;").bind(location_id),
        )
        .await
        .is_ok()
}

pub async fn markseen_msgs(context: &Context, msg_ids: Vec<MsgId>) -> bool {
    if msg_ids.is_empty() {
        return false;
    }
    let stmt = concat!(
        "SELECT",
        "    m.chat_id AS chat_id,",
        "    m.state AS state,",
        "    c.blocked AS blocked",
        " FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id",
        " WHERE m.id=? AND m.chat_id>9"
    );
    let mut msgs = Vec::with_capacity(msg_ids.len());
    for id in msg_ids.into_iter() {
        match context
            .sql
            .fetch_optional(sqlx::query(stmt).bind(id))
            .await
            .and_then(|row| {
                if let Some(row) = row {
                    Ok(Some((
                        row.try_get::<ChatId, _>("chat_id")?,
                        row.try_get::<MessageState, _>("state")?,
                        row.try_get::<Option<Blocked>, _>("blocked")?
                            .unwrap_or_default(),
                    )))
                } else {
                    Ok(None)
                }
            }) {
            Ok(Some((chat_id, state, blocked))) => msgs.push((id, chat_id, state, blocked)),
            Ok(None) => {}
            Err(err) => {
                warn!(context, "failed to markseen msgs: {:?}", err);
            }
        }
    }

    let mut updated_chat_ids = BTreeMap::new();

    for (id, curr_chat_id, curr_state, curr_blocked) in msgs.into_iter() {
        if let Err(err) = id.start_ephemeral_timer(context).await {
            error!(
                context,
                "Failed to start ephemeral timer for message {}: {}", id, err
            );
            continue;
        }

        if curr_blocked == Blocked::Not {
            if curr_state == MessageState::InFresh || curr_state == MessageState::InNoticed {
                update_msg_state(context, id, MessageState::InSeen).await;
                info!(context, "Seen message {}.", id);

                job::add(
                    context,
                    job::Job::new(Action::MarkseenMsgOnImap, id.to_i64(), Params::new(), 0),
                )
                .await;
                updated_chat_ids.insert(curr_chat_id, true);
            }
        } else if curr_state == MessageState::InFresh {
            update_msg_state(context, id, MessageState::InNoticed).await;
            updated_chat_ids.insert(DC_CHAT_ID_DEADDROP, true);
        }
    }

    for updated_chat_id in updated_chat_ids.keys() {
        context.emit_event(EventType::MsgsNoticed(*updated_chat_id));
    }

    true
}

pub async fn update_msg_state(context: &Context, msg_id: MsgId, state: MessageState) -> bool {
    context
        .sql
        .execute(
            sqlx::query("UPDATE msgs SET state=? WHERE id=?;")
                .bind(state)
                .bind(msg_id),
        )
        .await
        .is_ok()
}

/// Returns a summary text.
pub async fn get_summarytext_by_raw(
    viewtype: Viewtype,
    text: Option<impl AsRef<str>>,
    param: &Params,
    approx_characters: usize,
    context: &Context,
) -> String {
    let mut append_text = true;
    let prefix = match viewtype {
        Viewtype::Image => stock_str::image(context).await,
        Viewtype::Gif => stock_str::gif(context).await,
        Viewtype::Sticker => stock_str::sticker(context).await,
        Viewtype::Video => stock_str::video(context).await,
        Viewtype::Voice => stock_str::voice_message(context).await,
        Viewtype::Audio | Viewtype::File => {
            if param.get_cmd() == SystemMessage::AutocryptSetupMessage {
                append_text = false;
                stock_str::ac_setup_msg_subject(context).await
            } else {
                let file_name: String = param
                    .get_path(Param::File, context)
                    .unwrap_or(None)
                    .and_then(|path| {
                        path.file_name()
                            .map(|fname| fname.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| String::from("ErrFileName"));
                let label = if viewtype == Viewtype::Audio {
                    stock_str::audio(context).await
                } else {
                    stock_str::file(context).await
                };
                format!("{} – {}", label, file_name)
            }
        }
        Viewtype::VideochatInvitation => {
            append_text = false;
            stock_str::videochat_invitation(context).await
        }
        _ => {
            if param.get_cmd() != SystemMessage::LocationOnly {
                "".to_string()
            } else {
                append_text = false;
                stock_str::location(context).await
            }
        }
    };

    if !append_text {
        return prefix;
    }

    let summary = if let Some(text) = text {
        if text.as_ref().is_empty() {
            prefix
        } else if prefix.is_empty() {
            dc_truncate(text.as_ref(), approx_characters).to_string()
        } else {
            let tmp = format!("{} – {}", prefix, text.as_ref());
            dc_truncate(&tmp, approx_characters).to_string()
        }
    } else {
        prefix
    };

    summary.split_whitespace().join(" ")
}

// as we do not cut inside words, this results in about 32-42 characters.
// Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
// It should also be very clear, the subject is _not_ the whole message.
// The value is also used for CC:-summaries

// Context functions to work with messages

pub async fn exists(context: &Context, msg_id: MsgId) -> anyhow::Result<bool> {
    if msg_id.is_special() {
        return Ok(false);
    }

    let chat_id: Option<ChatId> = context
        .sql
        .query_get_value(sqlx::query("SELECT chat_id FROM msgs WHERE id=?;").bind(msg_id))
        .await?;

    if let Some(chat_id) = chat_id {
        Ok(!chat_id.is_trash())
    } else {
        Ok(false)
    }
}

pub async fn set_msg_failed(context: &Context, msg_id: MsgId, error: Option<impl AsRef<str>>) {
    if let Ok(mut msg) = Message::load_from_db(context, msg_id).await {
        let error = error.map(|e| e.as_ref().to_string()).unwrap_or_default();
        if msg.state.can_fail() {
            msg.state = MessageState::OutFailed;
            warn!(context, "{} failed: {}", msg_id, error);
        } else {
            warn!(
                context,
                "{} seems to have failed ({}), but state is {}", msg_id, error, msg.state
            )
        }

        match context
            .sql
            .execute(
                sqlx::query("UPDATE msgs SET state=?, error=? WHERE id=?;")
                    .bind(msg.state)
                    .bind(error)
                    .bind(msg_id),
            )
            .await
        {
            Ok(_) => context.emit_event(EventType::MsgFailed {
                chat_id: msg.chat_id,
                msg_id,
            }),
            Err(e) => {
                warn!(context, "{:?}", e);
            }
        }
    }
}

/// returns Some if an event should be send
pub async fn handle_mdn(
    context: &Context,
    from_id: i64,
    rfc724_mid: &str,
    timestamp_sent: i64,
) -> anyhow::Result<Option<(ChatId, MsgId)>> {
    if from_id <= DC_CONTACT_ID_LAST_SPECIAL || rfc724_mid.is_empty() {
        return Ok(None);
    }

    let res = context
        .sql
        .fetch_one(
            sqlx::query(concat!(
                "SELECT",
                "    m.id AS msg_id,",
                "    c.id AS chat_id,",
                "    c.type AS type,",
                "    m.state AS state",
                " FROM msgs m LEFT JOIN chats c ON m.chat_id=c.id",
                " WHERE rfc724_mid=? AND from_id=1",
                " ORDER BY m.id;"
            ))
            .bind(rfc724_mid),
        )
        .await
        .and_then(|row| {
            Ok((
                row.try_get::<MsgId, _>("msg_id")?,
                row.try_get::<ChatId, _>("chat_id")?,
                row.try_get::<Chattype, _>("type")?,
                row.try_get::<MessageState, _>("state")?,
            ))
        });

    if let Err(ref err) = res {
        info!(context, "Failed to select MDN {:?}", err);
    }

    if let Ok((msg_id, chat_id, chat_type, msg_state)) = res {
        let mut read_by_all = false;

        if msg_state == MessageState::OutPreparing
            || msg_state == MessageState::OutPending
            || msg_state == MessageState::OutDelivered
        {
            let mdn_already_in_table = context
                .sql
                .exists(
                    sqlx::query("SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=? AND contact_id=?;")
                        .bind(msg_id)
                        .bind(from_id),
                )
                .await
                .unwrap_or_default();

            if !mdn_already_in_table {
                context.sql.execute(
                    sqlx::query("INSERT INTO msgs_mdns (msg_id, contact_id, timestamp_sent) VALUES (?, ?, ?);")
                        .bind(msg_id)
                        .bind(from_id)
                        .bind(timestamp_sent)
                )
                    .await
                           .unwrap_or_default(); // TODO: better error handling
            }

            // Normal chat? that's quite easy.
            if chat_type == Chattype::Single {
                update_msg_state(context, msg_id, MessageState::OutMdnRcvd).await;
                read_by_all = true;
            } else {
                // send event about new state
                let ist_cnt = context
                    .sql
                    .count(
                        sqlx::query("SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=?;").bind(msg_id),
                    )
                    .await?;

                // Groupsize:  Min. MDNs
                // 1 S         n/a
                // 2 SR        1
                // 3 SRR       2
                // 4 SRRR      2
                // 5 SRRRR     3
                // 6 SRRRRR    3
                //
                // (S=Sender, R=Recipient)

                // for rounding, SELF is already included!
                let soll_cnt = (chat::get_chat_contact_cnt(context, chat_id).await? + 1) / 2;
                if ist_cnt >= soll_cnt {
                    update_msg_state(context, msg_id, MessageState::OutMdnRcvd).await;
                    read_by_all = true;
                } // else wait for more receipts
            }
        }
        return if read_by_all {
            Ok(Some((chat_id, msg_id)))
        } else {
            Ok(None)
        };
    }
    Ok(None)
}

/// Marks a message as failed after an ndn (non-delivery-notification) arrived.
/// Where appropriate, also adds an info message telling the user which of the recipients of a group message failed.
pub(crate) async fn handle_ndn(
    context: &Context,
    failed: &FailureReport,
    error: Option<impl AsRef<str>>,
) -> anyhow::Result<()> {
    if failed.rfc724_mid.is_empty() {
        return Ok(());
    }

    // The NDN might be for a message-id that had attachments and was sent from a non-Delta Chat client.
    // In this case we need to mark multiple "msgids" as failed that all refer to the same message-id.
    let mut rows = context
        .sql
        .fetch(
            sqlx::query(concat!(
                "SELECT",
                "    m.id AS msg_id,",
                "    c.id AS chat_id,",
                "    c.type AS type",
                " FROM msgs m LEFT JOIN chats c ON m.chat_id=c.id",
                " WHERE rfc724_mid=? AND from_id=1",
            ))
            .bind(&failed.rfc724_mid),
        )
        .await?;

    let mut first = true;
    while let Some(row) = rows.next().await {
        let row = row?;
        let msg_id = row.try_get::<MsgId, _>("msg_id")?;
        let chat_id = row.try_get::<ChatId, _>("chat_id")?;
        let chat_type = row.try_get::<Chattype, _>("type")?;

        set_msg_failed(context, msg_id, error.as_ref()).await;
        if first {
            // Add only one info msg for all failed messages
            ndn_maybe_add_info_msg(context, failed, chat_id, chat_type).await?;
        }
        first = false;
    }

    Ok(())
}

async fn ndn_maybe_add_info_msg(
    context: &Context,
    failed: &FailureReport,
    chat_id: ChatId,
    chat_type: Chattype,
) -> anyhow::Result<()> {
    match chat_type {
        Chattype::Group => {
            if let Some(failed_recipient) = &failed.failed_recipient {
                let contact_id =
                    Contact::lookup_id_by_addr(context, failed_recipient, Origin::Unknown)
                        .await?
                        .ok_or_else(|| {
                            Error::msg("ndn_maybe_add_info_msg: Contact ID not found")
                        })?;
                let contact = Contact::load_from_db(context, contact_id).await?;
                // Tell the user which of the recipients failed if we know that (because in
                // a group, this might otherwise be unclear)
                let text = stock_str::failed_sending_to(context, contact.get_display_name()).await;
                chat::add_info_msg(context, chat_id, text).await;
                context.emit_event(EventType::ChatModified(chat_id));
            }
        }
        Chattype::Mailinglist => {
            // ndn_maybe_add_info_msg() is about the case when delivery to the group failed.
            // If we get an NDN for the mailing list, just issue a warning.
            warn!(context, "ignoring NDN for mailing list.");
        }
        Chattype::Single | Chattype::Undefined => {}
    }
    Ok(())
}

/// The number of messages assigned to real chat (!=deaddrop, !=trash)
pub async fn get_real_msg_cnt(context: &Context) -> usize {
    match context
        .sql
        .count(
            "SELECT COUNT(*) \
         FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE m.id>9 AND m.chat_id>9 AND c.blocked=0;",
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!(context, "dc_get_real_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub async fn get_deaddrop_msg_cnt(context: &Context) -> usize {
    match context
        .sql
        .count(
            "SELECT COUNT(*) \
         FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE c.blocked=2;",
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!(context, "dc_get_deaddrop_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub async fn estimate_deletion_cnt(
    context: &Context,
    from_server: bool,
    seconds: i64,
) -> Result<usize, Error> {
    let self_chat_id = chat::lookup_by_contact_id(context, DC_CONTACT_ID_SELF)
        .await
        .unwrap_or_default()
        .0;
    let threshold_timestamp = time() - seconds;

    let cnt = if from_server {
        context
            .sql
            .count(
                sqlx::query(
                    "SELECT COUNT(*)
             FROM msgs m
             WHERE m.id > ?
               AND timestamp < ?
               AND chat_id != ?
               AND server_uid != 0;",
                )
                .bind(DC_MSG_ID_LAST_SPECIAL)
                .bind(threshold_timestamp)
                .bind(self_chat_id),
            )
            .await?
    } else {
        context
            .sql
            .count(
                sqlx::query(
                    "SELECT COUNT(*)
             FROM msgs m
             WHERE m.id > ?
               AND timestamp < ?
               AND chat_id != ?
               AND chat_id != ? AND hidden = 0;",
                )
                .bind(DC_MSG_ID_LAST_SPECIAL)
                .bind(threshold_timestamp)
                .bind(self_chat_id)
                .bind(DC_CHAT_ID_TRASH),
            )
            .await?
    };
    Ok(cnt)
}

/// Counts number of database records pointing to specified
/// Message-ID.
///
/// Unlinked messages are excluded.
pub async fn rfc724_mid_cnt(context: &Context, rfc724_mid: &str) -> usize {
    // check the number of messages with the same rfc724_mid
    match context
        .sql
        .count(
            sqlx::query("SELECT COUNT(*) FROM msgs WHERE rfc724_mid=? AND NOT server_uid = 0")
                .bind(rfc724_mid),
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!(context, "dc_get_rfc724_mid_cnt() failed. {}", err);
            0
        }
    }
}

pub(crate) async fn rfc724_mid_exists(
    context: &Context,
    rfc724_mid: &str,
) -> Result<Option<(String, u32, MsgId)>, Error> {
    let rfc724_mid = rfc724_mid.trim_start_matches('<').trim_end_matches('>');
    if rfc724_mid.is_empty() {
        warn!(context, "Empty rfc724_mid passed to rfc724_mid_exists");
        return Ok(None);
    }

    let row = context
        .sql
        .fetch_optional(
            sqlx::query("SELECT server_folder, server_uid, id FROM msgs WHERE rfc724_mid=?")
                .bind(rfc724_mid),
        )
        .await?;
    if let Some(row) = row {
        let server_folder = row.try_get::<Option<String>, _>(0)?.unwrap_or_default();
        let server_uid = row.try_get::<i64, _>(1)? as u32;
        let msg_id: MsgId = row.try_get(2)?;

        Ok(Some((server_folder, server_uid, msg_id)))
    } else {
        Ok(None)
    }
}

pub async fn update_server_uid(
    context: &Context,
    rfc724_mid: &str,
    server_folder: impl AsRef<str>,
    server_uid: u32,
) {
    match context
        .sql
        .execute(
            sqlx::query(
                "UPDATE msgs SET server_folder=?, server_uid=? \
             WHERE rfc724_mid=?",
            )
            .bind(server_folder.as_ref())
            .bind(server_uid as i64)
            .bind(rfc724_mid),
        )
        .await
    {
        Ok(_) => {}
        Err(err) => {
            warn!(context, "msg: failed to update server_uid: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ChatItem;
    use crate::constants::DC_CONTACT_ID_DEVICE;
    use crate::dc_receive_imf::dc_receive_imf;
    use crate::test_utils as test;
    use crate::test_utils::TestContext;

    #[test]
    fn test_guess_msgtype_from_suffix() {
        assert_eq!(
            guess_msgtype_from_suffix(Path::new("foo/bar-sth.mp3")),
            Some((Viewtype::Audio, "audio/mpeg"))
        );
    }

    // chat_msg means that the message was sent by Delta Chat
    // The tuples are (folder, mvbox_move, chat_msg, expected_destination)
    const COMBINATIONS_ACCEPTED_CHAT: &[(&str, bool, bool, &str)] = &[
        ("INBOX", false, false, "INBOX"),
        ("INBOX", false, true, "INBOX"),
        ("INBOX", true, false, "INBOX"),
        ("INBOX", true, true, "DeltaChat"),
        ("Sent", false, false, "Sent"),
        ("Sent", false, true, "Sent"),
        ("Sent", true, false, "Sent"),
        ("Sent", true, true, "DeltaChat"),
        ("Spam", false, false, "INBOX"), // Move classical emails in accepted chats from Spam to Inbox, not 100% sure on this, we could also just never move non-chat-msgs
        ("Spam", false, true, "INBOX"),
        ("Spam", true, false, "INBOX"), // Move classical emails in accepted chats from Spam to Inbox, not 100% sure on this, we could also just never move non-chat-msgs
        ("Spam", true, true, "DeltaChat"),
    ];

    // These are the same as above, but all messages in Spam stay in Spam
    const COMBINATIONS_DEADDROP: &[(&str, bool, bool, &str)] = &[
        ("INBOX", false, false, "INBOX"),
        ("INBOX", false, true, "INBOX"),
        ("INBOX", true, false, "INBOX"),
        ("INBOX", true, true, "DeltaChat"),
        ("Sent", false, false, "Sent"),
        ("Sent", false, true, "Sent"),
        ("Sent", true, false, "Sent"),
        ("Sent", true, true, "DeltaChat"),
        ("Spam", false, false, "Spam"),
        ("Spam", false, true, "Spam"),
        ("Spam", true, false, "Spam"),
        ("Spam", true, true, "Spam"),
    ];

    #[async_std::test]
    async fn test_needs_move_incoming_accepted() {
        for (folder, mvbox_move, chat_msg, expected_destination) in COMBINATIONS_ACCEPTED_CHAT {
            check_needs_move_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                expected_destination,
                true,
                false,
                false,
                false,
            )
            .await;
        }
    }

    #[async_std::test]
    async fn test_needs_move_incoming_deaddrop() {
        for (folder, mvbox_move, chat_msg, expected_destination) in COMBINATIONS_DEADDROP {
            check_needs_move_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                expected_destination,
                false,
                false,
                false,
                false,
            )
            .await;
        }
    }

    #[async_std::test]
    async fn test_needs_move_outgoing() {
        for sentbox_move in &[true, false] {
            // Test outgoing emails
            for (folder, mvbox_move, chat_msg, mut expected_destination) in
                COMBINATIONS_ACCEPTED_CHAT
            {
                if *folder == "INBOX" && !mvbox_move && *chat_msg && *sentbox_move {
                    expected_destination = "Sent"
                }
                check_needs_move_combination(
                    folder,
                    *mvbox_move,
                    *chat_msg,
                    expected_destination,
                    true,
                    true,
                    false,
                    *sentbox_move,
                )
                .await;
            }
        }
    }

    #[async_std::test]
    async fn test_needs_move_setupmsg() {
        // Test setupmessages
        for (folder, mvbox_move, chat_msg, _expected_destination) in COMBINATIONS_ACCEPTED_CHAT {
            check_needs_move_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                if folder == &"Spam" { "INBOX" } else { folder }, // Never move setup messages, except if they are in "Spam"
                false,
                true,
                true,
                false,
            )
            .await;
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn check_needs_move_combination(
        folder: &str,
        mvbox_move: bool,
        chat_msg: bool,
        expected_destination: &str,
        accepted_chat: bool,
        outgoing: bool,
        setupmessage: bool,
        sentbox_move: bool,
    ) {
        println!("Testing: For folder {}, mvbox_move {}, chat_msg {}, accepted {}, outgoing {}, setupmessage {}",
                               folder, mvbox_move, chat_msg, accepted_chat, outgoing, setupmessage);

        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ConfiguredSpamFolder, Some("Spam"))
            .await
            .unwrap();
        t.ctx
            .set_config(Config::ConfiguredMvboxFolder, Some("DeltaChat"))
            .await
            .unwrap();
        t.ctx
            .set_config(Config::ConfiguredSentboxFolder, Some("Sent"))
            .await
            .unwrap();
        t.ctx
            .set_config(Config::MvboxMove, Some(if mvbox_move { "1" } else { "0" }))
            .await
            .unwrap();
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();
        t.ctx
            .set_config_bool(Config::SentboxMove, sentbox_move)
            .await
            .unwrap();

        if accepted_chat {
            let contact_id = Contact::create(&t.ctx, "", "bob@example.net")
                .await
                .unwrap();
            chat::create_by_contact_id(&t.ctx, contact_id)
                .await
                .unwrap();
        }
        let temp;
        dc_receive_imf(
            &t.ctx,
            if setupmessage {
                include_bytes!("../test-data/message/AutocryptSetupMessage.eml")
            } else {
                temp = format!(
                    "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    {}\
                    Subject: foo\n\
                    Message-ID: <aehtri@example.com>\n\
                    {}\
                    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                    \n\
                    hello\n",
                    if outgoing {
                        "From: alice@example.com\nTo: bob@example.net\n"
                    } else {
                        "From: bob@example.net\nTo: alice@example.com\n"
                    },
                    if chat_msg { "Chat-Version: 1.0\n" } else { "" },
                );
                temp.as_bytes()
            },
            folder,
            1,
            false,
        )
        .await
        .unwrap();

        let msg = t.get_last_msg().await;
        let actual = if let Some(config) = msg.id.needs_move(&t.ctx, folder).await.unwrap() {
            t.ctx.get_config(config).await.unwrap()
        } else {
            None
        };
        let expected = if expected_destination == folder {
            None
        } else {
            Some(expected_destination)
        };
        assert_eq!(expected, actual.as_deref(), "For folder {}, mvbox_move {}, chat_msg {}, accepted {}, outgoing {}, setupmessage {}: expected {:?} , got {:?}",
                                                     folder, mvbox_move, chat_msg, accepted_chat, outgoing, setupmessage, expected, actual);
    }

    #[async_std::test]
    async fn test_prepare_message_and_send() {
        use crate::config::Config;

        let d = test::TestContext::new().await;
        let ctx = &d.ctx;

        ctx.set_config(Config::ConfiguredAddr, Some("self@example.com"))
            .await
            .unwrap();

        let chat = d.create_chat_with_contact("", "dest@example.com").await;

        let mut msg = Message::new(Viewtype::Text);

        let msg_id = chat::prepare_msg(ctx, chat.id, &mut msg).await.unwrap();

        let _msg2 = Message::load_from_db(ctx, msg_id).await.unwrap();
        assert_eq!(_msg2.get_filemime(), None);
    }

    /// Tests that message cannot be prepared if account has no configured address.
    #[async_std::test]
    async fn test_prepare_not_configured() {
        let d = test::TestContext::new().await;
        let ctx = &d.ctx;

        let chat = d.create_chat_with_contact("", "dest@example.com").await;

        let mut msg = Message::new(Viewtype::Text);

        assert!(chat::prepare_msg(ctx, chat.id, &mut msg).await.is_err());
    }

    #[async_std::test]
    async fn test_get_summarytext_by_raw() {
        let d = test::TestContext::new().await;
        let ctx = &d.ctx;

        let some_text = Some(" bla \t\n\tbla\n\t".to_string());
        let empty_text = Some("".to_string());
        let no_text: Option<String> = None;

        let mut some_file = Params::new();
        some_file.set(Param::File, "foo.bar");

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Text, some_text.as_ref(), &Params::new(), 50, ctx)
                .await,
            "bla bla" // for simple text, the type is not added to the summary
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Image, no_text.as_ref(), &some_file, 50, ctx).await,
            "Image" // file names are not added for images
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Video, no_text.as_ref(), &some_file, 50, ctx).await,
            "Video" // file names are not added for videos
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Gif, no_text.as_ref(), &some_file, 50, ctx,).await,
            "GIF" // file names are not added for GIFs
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Sticker, no_text.as_ref(), &some_file, 50, ctx,).await,
            "Sticker" // file names are not added for stickers
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, empty_text.as_ref(), &some_file, 50, ctx,)
                .await,
            "Voice message" // file names are not added for voice messages, empty text is skipped
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, no_text.as_ref(), &some_file, 50, ctx).await,
            "Voice message" // file names are not added for voice messages
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, some_text.as_ref(), &some_file, 50, ctx).await,
            "Voice message \u{2013} bla bla" // `\u{2013}` explicitly checks for "EN DASH"
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, no_text.as_ref(), &some_file, 50, ctx).await,
            "Audio \u{2013} foo.bar" // file name is added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, empty_text.as_ref(), &some_file, 50, ctx,)
                .await,
            "Audio \u{2013} foo.bar" // file name is added for audio, empty text is not added
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, some_text.as_ref(), &some_file, 50, ctx).await,
            "Audio \u{2013} foo.bar \u{2013} bla bla" // file name and text added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::File, some_text.as_ref(), &some_file, 50, ctx).await,
            "File \u{2013} foo.bar \u{2013} bla bla" // file name is added for files
        );

        let mut asm_file = Params::new();
        asm_file.set(Param::File, "foo.bar");
        asm_file.set_cmd(SystemMessage::AutocryptSetupMessage);
        assert_eq!(
            get_summarytext_by_raw(Viewtype::File, no_text.as_ref(), &asm_file, 50, ctx).await,
            "Autocrypt Setup Message" // file name is not added for autocrypt setup messages
        );
    }

    #[async_std::test]
    async fn test_parse_webrtc_instance() {
        let (webrtc_type, url) = Message::parse_webrtc_instance("basicwebrtc:https://foo/bar");
        assert_eq!(webrtc_type, VideochatType::BasicWebrtc);
        assert_eq!(url, "https://foo/bar");

        let (webrtc_type, url) = Message::parse_webrtc_instance("bAsIcwEbrTc:url");
        assert_eq!(webrtc_type, VideochatType::BasicWebrtc);
        assert_eq!(url, "url");

        let (webrtc_type, url) = Message::parse_webrtc_instance("https://foo/bar?key=val#key=val");
        assert_eq!(webrtc_type, VideochatType::Unknown);
        assert_eq!(url, "https://foo/bar?key=val#key=val");

        let (webrtc_type, url) = Message::parse_webrtc_instance("jitsi:https://j.si/foo");
        assert_eq!(webrtc_type, VideochatType::Jitsi);
        assert_eq!(url, "https://j.si/foo");
    }

    #[async_std::test]
    async fn test_create_webrtc_instance() {
        // webrtc_instance may come from an input field of the ui, be pretty tolerant on input
        let instance = Message::create_webrtc_instance("https://meet.jit.si/", "123");
        assert_eq!(instance, "https://meet.jit.si/123");

        let instance = Message::create_webrtc_instance("https://meet.jit.si", "456");
        assert_eq!(instance, "https://meet.jit.si/456");

        let instance = Message::create_webrtc_instance("meet.jit.si", "789");
        assert_eq!(instance, "https://meet.jit.si/789");

        let instance = Message::create_webrtc_instance("bla.foo?", "123");
        assert_eq!(instance, "https://bla.foo?123");

        let instance = Message::create_webrtc_instance("jitsi:bla.foo#", "456");
        assert_eq!(instance, "jitsi:https://bla.foo#456");

        let instance = Message::create_webrtc_instance("bla.foo#room=", "789");
        assert_eq!(instance, "https://bla.foo#room=789");

        let instance = Message::create_webrtc_instance("https://bla.foo#room", "123");
        assert_eq!(instance, "https://bla.foo#room/123");

        let instance = Message::create_webrtc_instance("bla.foo#room$ROOM", "123");
        assert_eq!(instance, "https://bla.foo#room123");

        let instance = Message::create_webrtc_instance("bla.foo#room=$ROOM&after=cont", "234");
        assert_eq!(instance, "https://bla.foo#room=234&after=cont");

        let instance = Message::create_webrtc_instance("  meet.jit .si ", "789");
        assert_eq!(instance, "https://meet.jit.si/789");

        let instance = Message::create_webrtc_instance(" basicwebrtc: basic . stuff\n ", "12345ab");
        assert_eq!(instance, "basicwebrtc:https://basic.stuff/12345ab");
    }

    #[async_std::test]
    async fn test_create_webrtc_instance_noroom() {
        // webrtc_instance may come from an input field of the ui, be pretty tolerant on input
        let instance = Message::create_webrtc_instance("bla.foo$NOROOM", "123");
        assert_eq!(instance, "https://bla.foo");

        let instance = Message::create_webrtc_instance(" bla . foo $NOROOM ", "456");
        assert_eq!(instance, "https://bla.foo");

        let instance = Message::create_webrtc_instance(" $NOROOM bla . foo  ", "789");
        assert_eq!(instance, "https://bla.foo");

        let instance = Message::create_webrtc_instance(" bla.foo  / $NOROOM ? a = b ", "123");
        assert_eq!(instance, "https://bla.foo/?a=b");

        // $ROOM has a higher precedence
        let instance = Message::create_webrtc_instance("bla.foo/?$NOROOM=$ROOM", "123");
        assert_eq!(instance, "https://bla.foo/?$NOROOM=123");
    }

    #[async_std::test]
    async fn test_get_width_height() {
        let t = test::TestContext::new().await;

        // test that get_width() and get_height() are returning some dimensions for images;
        // (as the device-chat contains a welcome-images, we check that)
        t.update_device_chats().await.ok();
        let (device_chat_id, _) =
            chat::create_or_lookup_by_contact_id(&t, DC_CONTACT_ID_DEVICE, Blocked::Not)
                .await
                .unwrap();

        let mut has_image = false;
        let chatitems = chat::get_chat_msgs(&t, device_chat_id, 0, None)
            .await
            .unwrap();
        for chatitem in chatitems {
            if let ChatItem::Message { msg_id } = chatitem {
                if let Ok(msg) = Message::load_from_db(&t, msg_id).await {
                    if msg.get_viewtype() == Viewtype::Image {
                        has_image = true;
                        // just check that width/height are inside some reasonable ranges
                        assert!(msg.get_width() > 100);
                        assert!(msg.get_height() > 100);
                        assert!(msg.get_width() < 4000);
                        assert!(msg.get_height() < 4000);
                    }
                }
            }
        }
        assert!(has_image);
    }

    #[async_std::test]
    async fn test_quote() {
        use crate::config::Config;

        let d = test::TestContext::new().await;
        let ctx = &d.ctx;

        ctx.set_config(Config::ConfiguredAddr, Some("self@example.com"))
            .await
            .unwrap();

        let chat = d.create_chat_with_contact("", "dest@example.com").await;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("Quoted message".to_string()));

        // Prepare message for sending, so it gets a Message-Id.
        assert!(msg.rfc724_mid.is_empty());
        let msg_id = chat::prepare_msg(ctx, chat.id, &mut msg).await.unwrap();
        let msg = Message::load_from_db(ctx, msg_id).await.unwrap();
        assert!(!msg.rfc724_mid.is_empty());

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.set_quote(ctx, &msg).await.expect("can't set quote");
        assert!(msg2.quoted_text() == msg.get_text());

        let quoted_msg = msg2
            .quoted_message(ctx)
            .await
            .expect("error while retrieving quoted message")
            .expect("quoted message not found");
        assert!(quoted_msg.get_text() == msg2.quoted_text());
    }

    #[async_std::test]
    async fn test_get_chat_id() {
        // Alice receives a message that pops up as a contact request
        let alice = TestContext::new_alice().await;
        dc_receive_imf(
            &alice,
            b"From: Bob <bob@example.com>\n\
                    To: alice@example.com\n\
                    Chat-Version: 1.0\n\
                    Message-ID: <123@example.com>\n\
                    Date: Fri, 29 Jan 2021 21:37:55 +0000\n\
                    \n\
                    hello\n",
            "INBOX",
            123,
            false,
        )
        .await
        .unwrap();

        // check chat-id of this message
        let msg = alice.get_last_msg().await;
        assert!(msg.get_chat_id().is_deaddrop());
        assert!(msg.get_chat_id().is_special());
        assert!(!msg.get_real_chat_id().is_deaddrop());
        assert!(!msg.get_real_chat_id().is_special());
        assert_eq!(msg.get_text().unwrap(), "hello".to_string());
    }

    #[async_std::test]
    async fn test_set_override_sender_name() {
        // send message with overridden sender name
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let chat = alice.create_chat(&bob).await;
        let contact_id = *chat::get_chat_contacts(&alice, chat.id)
            .await
            .first()
            .unwrap();
        let contact = Contact::load_from_db(&alice, contact_id).await.unwrap();

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("bla blubb".to_string()));
        msg.set_override_sender_name(Some("over ride".to_string()));
        assert_eq!(
            msg.get_override_sender_name(),
            Some("over ride".to_string())
        );
        assert_eq!(msg.get_sender_name(&contact), "over ride".to_string());
        assert_ne!(contact.get_display_name(), "over ride".to_string());
        chat::send_msg(&alice, chat.id, &mut msg).await.unwrap();

        // bob receives that message
        let chat = bob.create_chat(&alice).await;
        let contact_id = *chat::get_chat_contacts(&bob, chat.id)
            .await
            .first()
            .unwrap();
        let contact = Contact::load_from_db(&bob, contact_id).await.unwrap();
        bob.recv_msg(&alice.pop_sent_msg().await).await;
        let msg = bob.get_last_msg_in(chat.id).await;
        assert_eq!(msg.text, Some("bla blubb".to_string()));
        assert_eq!(
            msg.get_override_sender_name(),
            Some("over ride".to_string())
        );
        assert_eq!(msg.get_sender_name(&contact), "over ride".to_string());
        assert_ne!(contact.get_display_name(), "over ride".to_string());

        // explicitly check that the message does not create a mailing list
        // (mailing lists may also use `Sender:`-header)
        let chat = Chat::load_from_db(&bob, msg.chat_id).await.unwrap();
        assert_ne!(chat.typ, Chattype::Mailinglist);
    }
}

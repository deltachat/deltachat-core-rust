//! # Messages and their identifiers

use async_std::path::{Path, PathBuf};
use deltachat_derive::{FromSql, ToSql};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::chat::{self, Chat, ChatId};
use crate::config::Config;
use crate::constants::*;
use crate::contact::*;
use crate::context::*;
use crate::dc_tools::*;
use crate::error::{ensure, Error};
use crate::events::EventType;
use crate::job::{self, Action};
use crate::lot::{Lot, LotState, Meaning};
use crate::mimeparser::{FailureReport, SystemMessage};
use crate::param::*;
use crate::pgp::*;
use crate::stock::StockMessage;
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
    Debug, Copy, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MsgId(u32);

impl MsgId {
    /// Create a new [MsgId].
    pub fn new(id: u32) -> MsgId {
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
            .query_get_value_result("SELECT state FROM msgs WHERE id=?", paramsv![self])
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
        if context.is_mvbox(folder).await {
            return Ok(None);
        }

        let msg = Message::load_from_db(context, self).await?;

        if context.is_spam_folder(folder).await {
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
        } else {
            if msg.state.is_outgoing()
                && msg.is_dc_message == MessengerMessage::Yes
                && !msg.is_setupmessage()
                && msg.to_id != DC_CONTACT_ID_SELF // Leave self-chat-messages in the inbox, not sure about this
                && context.is_inbox(folder).await
                && context.get_config(ConfiguredSentboxFolder).await.is_some()
            {
                Ok(Some(ConfiguredSentboxFolder))
            } else {
                Ok(None)
            }
        }
    }

    async fn needs_move_to_mvbox(self, context: &Context, msg: &Message) -> Result<bool, Error> {
        if !context.get_config_bool(Config::MvboxMove).await {
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
    /// It means the message is deleted locally, but not on the server
    /// yet.
    pub async fn trash(self, context: &Context) -> crate::sql::Result<()> {
        let chat_id = ChatId::new(DC_CHAT_ID_TRASH);
        context
            .sql
            .execute(
                "UPDATE msgs SET chat_id=?, txt='', txt_raw='' WHERE id=?",
                paramsv![chat_id, self],
            )
            .await?;

        Ok(())
    }

    /// Deletes a message and corresponding MDNs from the database.
    pub async fn delete_from_db(self, context: &Context) -> crate::sql::Result<()> {
        // We don't use transactions yet, so remove MDNs first to make
        // sure they are not left while the message is deleted.
        context
            .sql
            .execute("DELETE FROM msgs_mdns WHERE msg_id=?;", paramsv![self])
            .await?;
        context
            .sql
            .execute("DELETE FROM msgs WHERE id=?;", paramsv![self])
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
                "UPDATE msgs \
             SET server_folder='', server_uid=0 \
             WHERE id=?",
                paramsv![self],
            )
            .await?;
        Ok(())
    }

    /// Bad evil escape hatch.
    ///
    /// Avoid using this, eventually types should be cleaned up enough
    /// that it is no longer necessary.
    pub fn to_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for MsgId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Msg#{}", self.0)
    }
}

/// Allow converting [MsgId] to an SQLite type.
///
/// This allows you to directly store [MsgId] into the database.
///
/// # Errors
///
/// This **does** ensure that no special message IDs are written into
/// the database and the conversion will fail if this is not the case.
impl rusqlite::types::ToSql for MsgId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        if self.0 <= DC_MSG_ID_LAST_SPECIAL {
            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                InvalidMsgId,
            )));
        }
        let val = rusqlite::types::Value::Integer(self.0 as i64);
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

/// Allow converting an SQLite integer directly into [MsgId].
impl rusqlite::types::FromSql for MsgId {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        // Would be nice if we could use match here, but alas.
        i64::column_result(value).and_then(|val| {
            if 0 <= val && val <= std::u32::MAX as i64 {
                Ok(MsgId::new(val as u32))
            } else {
                Err(rusqlite::types::FromSqlError::OutOfRange(val))
            }
        })
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
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
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
    pub(crate) from_id: u32,
    pub(crate) to_id: u32,
    pub(crate) chat_id: ChatId,
    pub(crate) viewtype: Viewtype,
    pub(crate) state: MessageState,
    pub(crate) hidden: bool,
    pub(crate) timestamp_sort: i64,
    pub(crate) timestamp_sent: i64,
    pub(crate) timestamp_rcvd: i64,
    pub(crate) ephemeral_timer: u32,
    pub(crate) ephemeral_timestamp: i64,
    pub(crate) text: Option<String>,
    pub(crate) rfc724_mid: String,
    pub(crate) in_reply_to: Option<String>,
    pub(crate) server_folder: Option<String>,
    pub(crate) server_uid: u32,
    pub(crate) is_dc_message: MessengerMessage,
    pub(crate) chat_blocked: Blocked,
    pub(crate) location_id: u32,
    error: Option<String>,
    pub(crate) param: Params,
}

impl Message {
    pub fn new(viewtype: Viewtype) -> Self {
        let mut msg = Message::default();
        msg.viewtype = viewtype;

        msg
    }

    pub async fn load_from_db(context: &Context, id: MsgId) -> Result<Message, Error> {
        ensure!(
            !id.is_special(),
            "Can not load special message IDs from DB."
        );
        let msg = context
            .sql
            .query_row(
                concat!(
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
                    "    m.txt AS txt,",
                    "    m.param AS param,",
                    "    m.hidden AS hidden,",
                    "    m.location_id AS location,",
                    "    c.blocked AS blocked",
                    " FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id",
                    " WHERE m.id=?;"
                ),
                paramsv![id],
                |row| {
                    let mut msg = Message::default();
                    // msg.id = row.get::<_, AnyMsgId>("id")?;
                    msg.id = row.get("id")?;
                    msg.rfc724_mid = row.get::<_, String>("rfc724mid")?;
                    msg.in_reply_to = row.get::<_, Option<String>>("mime_in_reply_to")?;
                    msg.server_folder = row.get::<_, Option<String>>("server_folder")?;
                    msg.server_uid = row.get("server_uid")?;
                    msg.chat_id = row.get("chat_id")?;
                    msg.from_id = row.get("from_id")?;
                    msg.to_id = row.get("to_id")?;
                    msg.timestamp_sort = row.get("timestamp")?;
                    msg.timestamp_sent = row.get("timestamp_sent")?;
                    msg.timestamp_rcvd = row.get("timestamp_rcvd")?;
                    msg.ephemeral_timer = row.get("ephemeral_timer")?;
                    msg.ephemeral_timestamp = row.get("ephemeral_timestamp")?;
                    msg.viewtype = row.get("type")?;
                    msg.state = row.get("state")?;
                    let error: String = row.get("error")?;
                    msg.error = Some(error).filter(|error| !error.is_empty());
                    msg.is_dc_message = row.get("msgrmsg")?;

                    let text;
                    if let rusqlite::types::ValueRef::Text(buf) = row.get_raw("txt") {
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

                    msg.param = row.get::<_, String>("param")?.parse().unwrap_or_default();
                    msg.hidden = row.get("hidden")?;
                    msg.location_id = row.get("location")?;
                    msg.chat_blocked = row
                        .get::<_, Option<Blocked>>("blocked")?
                        .unwrap_or_default();

                    Ok(msg)
                },
            )
            .await?;

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

    pub fn get_from_id(&self) -> u32 {
        self.from_id
    }

    pub fn get_chat_id(&self) -> ChatId {
        if self.chat_blocked != Blocked::Not {
            ChatId::new(DC_CHAT_ID_DEADDROP)
        } else {
            self.chat_id
        }
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

    pub fn get_ephemeral_timer(&self) -> u32 {
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

        let contact = if self.from_id != DC_CONTACT_ID_SELF as u32 && chat.typ == Chattype::Group {
            Contact::get_by_id(context, self.from_id).await.ok()
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
        self.from_id == DC_CONTACT_ID_INFO as u32
            || self.to_id == DC_CONTACT_ID_INFO as u32
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
            url.replace("$ROOM", &room)
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
                let rfc724_mid = in_reply_to.trim_start_matches('<').trim_end_matches('>');
                if !rfc724_mid.is_empty() {
                    if let Some((_, _, msg_id)) = rfc724_mid_exists(context, rfc724_mid).await? {
                        return Ok(Some(Message::load_from_db(context, msg_id).await?));
                    }
                }
            }
        }
        Ok(None)
    }

    pub async fn update_param(&mut self, context: &Context) -> bool {
        context
            .sql
            .execute(
                "UPDATE msgs SET param=? WHERE id=?;",
                paramsv![self.param.to_string(), self.id],
            )
            .await
            .is_ok()
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

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    ToSql,
    FromSql,
    Serialize,
    Deserialize,
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
            self.text1 = Some(
                context
                    .stock_str(StockMessage::Draft)
                    .await
                    .to_owned()
                    .into(),
            );
            self.text1_meaning = Meaning::Text1Draft;
        } else if msg.from_id == DC_CONTACT_ID_SELF {
            if msg.is_info() || chat.is_self_talk() {
                self.text1 = None;
                self.text1_meaning = Meaning::None;
            } else {
                self.text1 = Some(
                    context
                        .stock_str(StockMessage::SelfMsg)
                        .await
                        .to_owned()
                        .into(),
                );
                self.text1_meaning = Meaning::Text1Self;
            }
        } else if chat.typ == Chattype::Group {
            if msg.is_info() || contact.is_none() {
                self.text1 = None;
                self.text1_meaning = Meaning::None;
            } else {
                if chat.id.is_deaddrop() {
                    if let Some(contact) = contact {
                        self.text1 = Some(contact.get_display_name().into());
                    } else {
                        self.text1 = None;
                    }
                } else if let Some(contact) = contact {
                    self.text1 = Some(contact.get_first_name().into());
                } else {
                    self.text1 = None;
                }
                self.text1_meaning = Meaning::Text1Username;
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
            text2 = context
                .stock_str(StockMessage::ReplyNoun)
                .await
                .into_owned()
        }

        self.text2 = Some(text2);

        self.timestamp = msg.get_timestamp();
        self.state = msg.state.into();
    }
}

pub async fn get_msg_info(context: &Context, msg_id: MsgId) -> String {
    let mut ret = String::new();

    let msg = Message::load_from_db(context, msg_id).await;
    if msg.is_err() {
        return ret;
    }

    let msg = msg.unwrap_or_default();

    let rawtxt: Option<String> = context
        .sql
        .query_get_value(
            context,
            "SELECT txt_raw FROM msgs WHERE id=?;",
            paramsv![msg_id],
        )
        .await;

    if rawtxt.is_none() {
        ret += &format!("Cannot load message {}.", msg_id);
        return ret;
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

    if msg.from_id != DC_CONTACT_ID_SELF as u32 {
        let s = dc_timestamp_to_str(if 0 != msg.timestamp_rcvd {
            msg.timestamp_rcvd
        } else {
            msg.timestamp_sort
        });
        ret += &format!("Received: {}", &s);
        ret += "\n";
    }

    if msg.ephemeral_timer != 0 {
        ret += &format!("Ephemeral timer: {}\n", msg.ephemeral_timer);
    }

    if msg.ephemeral_timestamp != 0 {
        ret += &format!(
            "Expires: {}\n",
            dc_timestamp_to_str(msg.ephemeral_timestamp)
        );
    }

    if msg.from_id == DC_CONTACT_ID_INFO || msg.to_id == DC_CONTACT_ID_INFO {
        // device-internal message, no further details needed
        return ret;
    }

    if let Ok(rows) = context
        .sql
        .query_map(
            "SELECT contact_id, timestamp_sent FROM msgs_mdns WHERE msg_id=?;",
            paramsv![msg_id],
            |row| {
                let contact_id: i32 = row.get(0)?;
                let ts: i64 = row.get(1)?;
                Ok((contact_id, ts))
            },
            |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
        )
        .await
    {
        for (contact_id, ts) in rows {
            let fts = dc_timestamp_to_str(ts);
            ret += &format!("Read: {}", fts);

            let name = Contact::load_from_db(context, contact_id as u32)
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
        if server_folder != "" {
            ret += &format!("\nLast seen as: {}/{}", server_folder, msg.server_uid);
        }
    }

    ret
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

pub async fn get_mime_headers(context: &Context, msg_id: MsgId) -> Option<String> {
    context
        .sql
        .query_get_value(
            context,
            "SELECT mime_headers FROM msgs WHERE id=?;",
            paramsv![msg_id],
        )
        .await
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
            job::Job::new(Action::DeleteMsgOnImap, msg_id.to_u32(), Params::new(), 0),
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

async fn delete_poi_location(context: &Context, location_id: u32) -> bool {
    context
        .sql
        .execute(
            "DELETE FROM locations WHERE independent = 1 AND id=?;",
            paramsv![location_id as i32],
        )
        .await
        .is_ok()
}

pub async fn markseen_msgs(context: &Context, msg_ids: Vec<MsgId>) -> bool {
    if msg_ids.is_empty() {
        return false;
    }

    let msgs = context
        .sql
        .with_conn(move |conn| {
            let mut stmt = conn.prepare_cached(concat!(
                "SELECT",
                "    m.chat_id AS chat_id,",
                "    m.state AS state,",
                "    c.blocked AS blocked",
                " FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id",
                " WHERE m.id=? AND m.chat_id>9"
            ))?;

            let mut msgs = Vec::with_capacity(msg_ids.len());
            for id in msg_ids.into_iter() {
                let query_res = stmt.query_row(paramsv![id], |row| {
                    Ok((
                        row.get::<_, ChatId>("chat_id")?,
                        row.get::<_, MessageState>("state")?,
                        row.get::<_, Option<Blocked>>("blocked")?
                            .unwrap_or_default(),
                    ))
                });
                if let Err(rusqlite::Error::QueryReturnedNoRows) = query_res {
                    continue;
                }
                let (chat_id, state, blocked) = query_res.map_err(Into::<anyhow::Error>::into)?;
                msgs.push((id, chat_id, state, blocked));
            }

            Ok(msgs)
        })
        .await
        .unwrap_or_default();

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
                    job::Job::new(Action::MarkseenMsgOnImap, id.to_u32(), Params::new(), 0),
                )
                .await;
                updated_chat_ids.insert(curr_chat_id, true);
            }
        } else if curr_state == MessageState::InFresh {
            update_msg_state(context, id, MessageState::InNoticed).await;
            updated_chat_ids.insert(ChatId::new(DC_CHAT_ID_DEADDROP), true);
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
            "UPDATE msgs SET state=? WHERE id=?;",
            paramsv![state, msg_id],
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
        Viewtype::Image => context.stock_str(StockMessage::Image).await.into_owned(),
        Viewtype::Gif => context.stock_str(StockMessage::Gif).await.into_owned(),
        Viewtype::Sticker => context.stock_str(StockMessage::Sticker).await.into_owned(),
        Viewtype::Video => context.stock_str(StockMessage::Video).await.into_owned(),
        Viewtype::Voice => context
            .stock_str(StockMessage::VoiceMessage)
            .await
            .into_owned(),
        Viewtype::Audio | Viewtype::File => {
            if param.get_cmd() == SystemMessage::AutocryptSetupMessage {
                append_text = false;
                context
                    .stock_str(StockMessage::AcSetupMsgSubject)
                    .await
                    .to_string()
            } else {
                let file_name: String = param
                    .get_path(Param::File, context)
                    .unwrap_or(None)
                    .and_then(|path| {
                        path.file_name()
                            .map(|fname| fname.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| String::from("ErrFileName"));
                let label = context
                    .stock_str(if viewtype == Viewtype::Audio {
                        StockMessage::Audio
                    } else {
                        StockMessage::File
                    })
                    .await;
                format!("{} – {}", label, file_name)
            }
        }
        Viewtype::VideochatInvitation => {
            append_text = false;
            context
                .stock_str(StockMessage::VideochatInvitation)
                .await
                .into_owned()
        }
        _ => {
            if param.get_cmd() != SystemMessage::LocationOnly {
                "".to_string()
            } else {
                append_text = false;
                context.stock_str(StockMessage::Location).await.to_string()
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

pub async fn exists(context: &Context, msg_id: MsgId) -> bool {
    if msg_id.is_special() {
        return false;
    }

    let chat_id: Option<ChatId> = context
        .sql
        .query_get_value(
            context,
            "SELECT chat_id FROM msgs WHERE id=?;",
            paramsv![msg_id],
        )
        .await;

    if let Some(chat_id) = chat_id {
        !chat_id.is_trash()
    } else {
        false
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
                "UPDATE msgs SET state=?, error=? WHERE id=?;",
                paramsv![msg.state, error, msg_id],
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
    from_id: u32,
    rfc724_mid: &str,
    timestamp_sent: i64,
) -> Option<(ChatId, MsgId)> {
    if from_id <= DC_CONTACT_ID_LAST_SPECIAL || rfc724_mid.is_empty() {
        return None;
    }

    let res = context
        .sql
        .query_row(
            concat!(
                "SELECT",
                "    m.id AS msg_id,",
                "    c.id AS chat_id,",
                "    c.type AS type,",
                "    m.state AS state",
                " FROM msgs m LEFT JOIN chats c ON m.chat_id=c.id",
                " WHERE rfc724_mid=? AND from_id=1",
                " ORDER BY m.id;"
            ),
            paramsv![rfc724_mid],
            |row| {
                Ok((
                    row.get::<_, MsgId>("msg_id")?,
                    row.get::<_, ChatId>("chat_id")?,
                    row.get::<_, Chattype>("type")?,
                    row.get::<_, MessageState>("state")?,
                ))
            },
        )
        .await;
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
                    "SELECT contact_id FROM msgs_mdns WHERE msg_id=? AND contact_id=?;",
                    paramsv![msg_id, from_id as i32,],
                )
                .await
                .unwrap_or_default();

            if !mdn_already_in_table {
                context.sql.execute(
                    "INSERT INTO msgs_mdns (msg_id, contact_id, timestamp_sent) VALUES (?, ?, ?);",
                    paramsv![msg_id, from_id as i32, timestamp_sent],
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
                    .query_get_value::<isize>(
                        context,
                        "SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=?;",
                        paramsv![msg_id],
                    )
                    .await
                    .unwrap_or_default() as usize;
                /*
                Groupsize:  Min. MDNs

                1 S         n/a
                2 SR        1
                3 SRR       2
                4 SRRR      2
                5 SRRRR     3
                6 SRRRRR    3

                (S=Sender, R=Recipient)
                 */
                // for rounding, SELF is already included!
                let soll_cnt = (chat::get_chat_contact_cnt(context, chat_id).await + 1) / 2;
                if ist_cnt >= soll_cnt {
                    update_msg_state(context, msg_id, MessageState::OutMdnRcvd).await;
                    read_by_all = true;
                } // else wait for more receipts
            }
        }
        return if read_by_all {
            Some((chat_id, msg_id))
        } else {
            None
        };
    }
    None
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
    let msgs: Vec<_> = context
        .sql
        .query_map(
            concat!(
                "SELECT",
                "    m.id AS msg_id,",
                "    c.id AS chat_id,",
                "    c.type AS type",
                " FROM msgs m LEFT JOIN chats c ON m.chat_id=c.id",
                " WHERE rfc724_mid=? AND from_id=1",
            ),
            paramsv![failed.rfc724_mid],
            |row| {
                Ok((
                    row.get::<_, MsgId>("msg_id")?,
                    row.get::<_, ChatId>("chat_id")?,
                    row.get::<_, Chattype>("type")?,
                ))
            },
            |rows| Ok(rows.collect::<Vec<_>>()),
        )
        .await?;

    for (i, msg) in msgs.into_iter().enumerate() {
        let (msg_id, chat_id, chat_type) = msg?;
        set_msg_failed(context, msg_id, error.as_ref()).await;
        if i == 0 {
            // Add only one info msg for all failed messages
            ndn_maybe_add_info_msg(context, failed, chat_id, chat_type).await?;
        }
    }

    Ok(())
}

async fn ndn_maybe_add_info_msg(
    context: &Context,
    failed: &FailureReport,
    chat_id: ChatId,
    chat_type: Chattype,
) -> anyhow::Result<()> {
    if chat_type == Chattype::Group {
        if let Some(failed_recipient) = &failed.failed_recipient {
            let contact_id =
                Contact::lookup_id_by_addr(context, failed_recipient, Origin::Unknown).await;
            let contact = Contact::load_from_db(context, contact_id).await?;
            // Tell the user which of the recipients failed if we know that (because in a group, this might otherwise be unclear)
            let text = context
                .stock_string_repl_str(StockMessage::FailedSendingTo, contact.get_display_name())
                .await;
            chat::add_info_msg(context, chat_id, text).await;
            context.emit_event(EventType::ChatModified(chat_id));
        }
    }
    Ok(())
}

/// The number of messages assigned to real chat (!=deaddrop, !=trash)
pub async fn get_real_msg_cnt(context: &Context) -> i32 {
    match context
        .sql
        .query_row(
            "SELECT COUNT(*) \
         FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE m.id>9 AND m.chat_id>9 AND c.blocked=0;",
            paramsv![],
            |row| row.get(0),
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
        .query_row(
            "SELECT COUNT(*) \
         FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE c.blocked=2;",
            paramsv![],
            |row| row.get::<_, isize>(0),
        )
        .await
    {
        Ok(res) => res as usize,
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

    let cnt: isize = if from_server {
        context
            .sql
            .query_row(
                "SELECT COUNT(*)
             FROM msgs m
             WHERE m.id > ?
               AND timestamp < ?
               AND chat_id != ?
               AND server_uid != 0;",
                paramsv![DC_MSG_ID_LAST_SPECIAL, threshold_timestamp, self_chat_id],
                |row| row.get(0),
            )
            .await?
    } else {
        context
            .sql
            .query_row(
                "SELECT COUNT(*)
             FROM msgs m
             WHERE m.id > ?
               AND timestamp < ?
               AND chat_id != ?
               AND chat_id != ? AND hidden = 0;",
                paramsv![
                    DC_MSG_ID_LAST_SPECIAL,
                    threshold_timestamp,
                    self_chat_id,
                    ChatId::new(DC_CHAT_ID_TRASH)
                ],
                |row| row.get(0),
            )
            .await?
    };
    Ok(cnt as usize)
}

/// Counts number of database records pointing to specified
/// Message-ID.
///
/// Unlinked messages are excluded.
pub async fn rfc724_mid_cnt(context: &Context, rfc724_mid: &str) -> i32 {
    // check the number of messages with the same rfc724_mid
    match context
        .sql
        .query_row(
            "SELECT COUNT(*) FROM msgs WHERE rfc724_mid=? AND NOT server_uid = 0",
            paramsv![rfc724_mid],
            |row| row.get(0),
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
    if rfc724_mid.is_empty() {
        warn!(context, "Empty rfc724_mid passed to rfc724_mid_exists");
        return Ok(None);
    }

    let res = context
        .sql
        .query_row_optional(
            "SELECT server_folder, server_uid, id FROM msgs WHERE rfc724_mid=?",
            paramsv![rfc724_mid],
            |row| {
                let server_folder = row.get::<_, Option<String>>(0)?.unwrap_or_default();
                let server_uid = row.get(1)?;
                let msg_id: MsgId = row.get(2)?;

                Ok((server_folder, server_uid, msg_id))
            },
        )
        .await?;

    Ok(res)
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
            "UPDATE msgs SET server_folder=?, server_uid=? \
             WHERE rfc724_mid=?",
            paramsv![server_folder.as_ref(), server_uid, rfc724_mid],
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
    use crate::test_utils as test;
    use crate::test_utils::*;

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
            )
            .await;
        }
    }

    #[async_std::test]
    async fn test_needs_move_incoming_deaddrop() {
        for (folder, mvbox_move, chat_msg, mut expected_destination) in COMBINATIONS_DEADDROP {
            if *folder == "INBOX" && !mvbox_move && *chat_msg {
                expected_destination = "Sent"
            }
            check_needs_move_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                expected_destination,
                false,
                false,
                false,
            )
            .await;
        }
    }

    #[async_std::test]
    async fn test_needs_move_outgoing() {
        // Test outgoing emails
        for (folder, mvbox_move, chat_msg, expected_destination) in COMBINATIONS_ACCEPTED_CHAT {
            check_needs_move_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                expected_destination,
                true,
                true,
                false,
            )
            .await;
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
                true,
                true,
                true,
            )
            .await;
        }
    }

    async fn check_needs_move_combination(
        folder: &str,
        mvbox_move: bool,
        chat_msg: bool,
        expected_destination: &str,
        accepted_chat: bool,
        outgoing: bool,
        setupmessage: bool,
    ) {
        use crate::dc_receive_imf::dc_receive_imf;
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
        // We do not need to set the ConfiguredSentboxFolder as for moving messages, the sentbox is treated the same as an unknown folder.
        t.ctx
            .set_config(Config::MvboxMove, Some(if mvbox_move { "1" } else { "0" }))
            .await
            .unwrap();
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
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
                    "{}\
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
            Some(t.ctx.get_config(config).await.unwrap())
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

        let contact = Contact::create(ctx, "", "dest@example.com")
            .await
            .expect("failed to create contact");

        let res = ctx
            .set_config(Config::ConfiguredAddr, Some("self@example.com"))
            .await;
        assert!(res.is_ok());

        let chat = chat::create_by_contact_id(ctx, contact).await.unwrap();

        let mut msg = Message::new(Viewtype::Text);

        let msg_id = chat::prepare_msg(ctx, chat, &mut msg).await.unwrap();

        let _msg2 = Message::load_from_db(ctx, msg_id).await.unwrap();
        assert_eq!(_msg2.get_filemime(), None);
    }

    /// Tests that message cannot be prepared if account has no configured address.
    #[async_std::test]
    async fn test_prepare_not_configured() {
        let d = test::TestContext::new().await;
        let ctx = &d.ctx;

        let contact = Contact::create(ctx, "", "dest@example.com")
            .await
            .expect("failed to create contact");

        let chat = chat::create_by_contact_id(ctx, contact).await.unwrap();

        let mut msg = Message::new(Viewtype::Text);

        assert!(chat::prepare_msg(ctx, chat, &mut msg).await.is_err());
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
            get_summarytext_by_raw(Viewtype::Text, some_text.as_ref(), &Params::new(), 50, &ctx)
                .await,
            "bla bla" // for simple text, the type is not added to the summary
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Image, no_text.as_ref(), &some_file, 50, &ctx).await,
            "Image" // file names are not added for images
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Video, no_text.as_ref(), &some_file, 50, &ctx).await,
            "Video" // file names are not added for videos
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Gif, no_text.as_ref(), &some_file, 50, &ctx,).await,
            "GIF" // file names are not added for GIFs
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Sticker, no_text.as_ref(), &some_file, 50, &ctx,)
                .await,
            "Sticker" // file names are not added for stickers
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, empty_text.as_ref(), &some_file, 50, &ctx,)
                .await,
            "Voice message" // file names are not added for voice messages, empty text is skipped
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, no_text.as_ref(), &some_file, 50, &ctx).await,
            "Voice message" // file names are not added for voice messages
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, some_text.as_ref(), &some_file, 50, &ctx).await,
            "Voice message \u{2013} bla bla" // `\u{2013}` explicitly checks for "EN DASH"
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, no_text.as_ref(), &some_file, 50, &ctx).await,
            "Audio \u{2013} foo.bar" // file name is added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, empty_text.as_ref(), &some_file, 50, &ctx,)
                .await,
            "Audio \u{2013} foo.bar" // file name is added for audio, empty text is not added
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, some_text.as_ref(), &some_file, 50, &ctx).await,
            "Audio \u{2013} foo.bar \u{2013} bla bla" // file name and text added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::File, some_text.as_ref(), &some_file, 50, &ctx).await,
            "File \u{2013} foo.bar \u{2013} bla bla" // file name is added for files
        );

        let mut asm_file = Params::new();
        asm_file.set(Param::File, "foo.bar");
        asm_file.set_cmd(SystemMessage::AutocryptSetupMessage);
        assert_eq!(
            get_summarytext_by_raw(Viewtype::File, no_text.as_ref(), &asm_file, 50, &ctx).await,
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
    async fn test_get_width_height() {
        let t = test::TestContext::new().await;

        // test that get_width() and get_height() are returning some dimensions for images;
        // (as the device-chat contains a welcome-images, we check that)
        t.ctx.update_device_chats().await.ok();
        let (device_chat_id, _) =
            chat::create_or_lookup_by_contact_id(&t.ctx, DC_CONTACT_ID_DEVICE, Blocked::Not)
                .await
                .unwrap();

        let mut has_image = false;
        let chatitems = chat::get_chat_msgs(&t.ctx, device_chat_id, 0, None).await;
        for chatitem in chatitems {
            if let ChatItem::Message { msg_id } = chatitem {
                if let Ok(msg) = Message::load_from_db(&t.ctx, msg_id).await {
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

        let contact = Contact::create(ctx, "", "dest@example.com")
            .await
            .expect("failed to create contact");

        let res = ctx
            .set_config(Config::ConfiguredAddr, Some("self@example.com"))
            .await;
        assert!(res.is_ok());

        let chat = chat::create_by_contact_id(ctx, contact).await.unwrap();

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("Quoted message".to_string()));

        // Prepare message for sending, so it gets a Message-Id.
        assert!(msg.rfc724_mid.is_empty());
        let msg_id = chat::prepare_msg(ctx, chat, &mut msg).await.unwrap();
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
}

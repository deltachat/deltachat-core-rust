//! # Messages and their identifiers.

use std::collections::BTreeSet;
use std::convert::TryInto;

use anyhow::{ensure, format_err, Context as _, Result};
use async_std::path::{Path, PathBuf};
use deltachat_derive::{FromSql, ToSql};
use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};

use crate::chat::{self, Chat, ChatId};
use crate::constants::{
    Blocked, Chattype, VideochatType, Viewtype, DC_CHAT_ID_TRASH, DC_CONTACT_ID_INFO,
    DC_CONTACT_ID_SELF, DC_DESIRED_TEXT_LEN, DC_MSG_ID_LAST_SPECIAL,
};
use crate::contact::{Contact, Origin};
use crate::context::Context;
use crate::dc_tools::{
    dc_create_smeared_timestamp, dc_get_filebytes, dc_get_filemeta, dc_gm2local_offset,
    dc_read_file, dc_timestamp_to_str, dc_truncate, time,
};
use crate::download::DownloadState;
use crate::ephemeral::Timer as EphemeralTimer;
use crate::events::EventType;
use crate::job::{self, Action};
use crate::log::LogExt;
use crate::mimeparser::{parse_message_id, FailureReport, SystemMessage};
use crate::param::{Param, Params};
use crate::pgp::split_armored_data;
use crate::scheduler::InterruptInfo;
use crate::stock_str;
use crate::summary::Summary;

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
    pub async fn get_state(self, context: &Context) -> Result<MessageState> {
        let result = context
            .sql
            .query_get_value("SELECT state FROM msgs WHERE id=?", paramsv![self])
            .await?
            .unwrap_or_default();
        Ok(result)
    }

    /// Put message into trash chat and delete message text.
    ///
    /// It means the message is deleted locally, but not on the server.
    /// We keep some infos to
    /// 1. not download the same message again
    /// 2. be able to delete the message on the server if we want to
    pub async fn trash(self, context: &Context) -> Result<()> {
        let chat_id = DC_CHAT_ID_TRASH;
        context
            .sql
            .execute(
                // If you change which information is removed here, also change delete_expired_messages() and
                // which information dc_receive_imf::add_parts() still adds to the db if the chat_id is TRASH
                r#"
UPDATE msgs 
SET 
  chat_id=?, txt='', 
  subject='', txt_raw='', 
  mime_headers='', 
  from_id=0, to_id=0, 
  param='' 
WHERE id=?;
"#,
                paramsv![chat_id, self],
            )
            .await?;

        Ok(())
    }

    /// Deletes a message and corresponding MDNs from the database.
    pub async fn delete_from_db(self, context: &Context) -> Result<()> {
        // We don't use transactions yet, so remove MDNs first to make
        // sure they are not left while the message is deleted.
        context
            .sql
            .execute("DELETE FROM msgs_mdns WHERE msg_id=?;", paramsv![self])
            .await?;
        context
            .sql
            .execute(
                "DELETE FROM msgs_status_updates WHERE msg_id=?;",
                paramsv![self],
            )
            .await?;
        context
            .sql
            .execute("DELETE FROM msgs WHERE id=?;", paramsv![self])
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
            return Err(rusqlite::Error::ToSqlConversionFailure(
                format_err!("Invalid MsgId").into(),
            ));
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
    pub(crate) download_state: DownloadState,
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

    pub async fn load_from_db(context: &Context, id: MsgId) -> Result<Message> {
        ensure!(
            !id.is_special(),
            "Can not load special message ID {} from DB",
            id
        );
        let msg = context
            .sql
            .query_row(
                concat!(
                    "SELECT",
                    "    m.id AS id,",
                    "    rfc724_mid AS rfc724mid,",
                    "    m.mime_in_reply_to AS mime_in_reply_to,",
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
                    "    m.download_state AS download_state,",
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
                ),
                paramsv![id],
                |row| {
                    let text = match row.get_ref("txt")? {
                        rusqlite::types::ValueRef::Text(buf) => {
                            match String::from_utf8(buf.to_vec()) {
                                Ok(t) => t,
                                Err(_) => {
                                    warn!(
                                        context,
                                        concat!(
                                            "dc_msg_load_from_db: could not get ",
                                            "text column as non-lossy utf8 id {}"
                                        ),
                                        id
                                    );
                                    String::from_utf8_lossy(buf).into_owned()
                                }
                            }
                        }
                        _ => String::new(),
                    };
                    let msg = Message {
                        id: row.get("id")?,
                        rfc724_mid: row.get::<_, String>("rfc724mid")?,
                        in_reply_to: row
                            .get::<_, Option<String>>("mime_in_reply_to")?
                            .and_then(|in_reply_to| parse_message_id(&in_reply_to).ok()),
                        chat_id: row.get("chat_id")?,
                        from_id: row.get("from_id")?,
                        to_id: row.get("to_id")?,
                        timestamp_sort: row.get("timestamp")?,
                        timestamp_sent: row.get("timestamp_sent")?,
                        timestamp_rcvd: row.get("timestamp_rcvd")?,
                        ephemeral_timer: row.get("ephemeral_timer")?,
                        ephemeral_timestamp: row.get("ephemeral_timestamp")?,
                        viewtype: row.get("type")?,
                        state: row.get("state")?,
                        download_state: row.get("download_state")?,
                        error: Some(row.get::<_, String>("error")?)
                            .filter(|error| !error.is_empty()),
                        is_dc_message: row.get("msgrmsg")?,
                        mime_modified: row.get("mime_modified")?,
                        text: Some(text),
                        subject: row.get("subject")?,
                        param: row.get::<_, String>("param")?.parse().unwrap_or_default(),
                        hidden: row.get("hidden")?,
                        location_id: row.get("location")?,
                        chat_blocked: row
                            .get::<_, Option<Blocked>>("blocked")?
                            .unwrap_or_default(),
                    };
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

    pub async fn try_calc_and_set_dimensions(&mut self, context: &Context) -> Result<()> {
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

    /// Returns the chat ID.
    pub fn get_chat_id(&self) -> ChatId {
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
        self.text.as_ref().map(|s| s.to_string())
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

    /// Returns true if message is Auto-Submitted.
    pub fn is_bot(&self) -> bool {
        self.param.get_bool(Param::Bot).unwrap_or_default()
    }

    pub fn get_ephemeral_timer(&self) -> EphemeralTimer {
        self.ephemeral_timer
    }

    pub fn get_ephemeral_timestamp(&self) -> i64 {
        self.ephemeral_timestamp
    }

    /// Returns message summary for display in the search results.
    pub async fn get_summary(&self, context: &Context, chat: Option<&Chat>) -> Result<Summary> {
        let chat_loaded: Chat;
        let chat = if let Some(chat) = chat {
            chat
        } else {
            let chat = Chat::load_from_db(context, self.chat_id).await?;
            chat_loaded = chat;
            &chat_loaded
        };

        let contact = if self.from_id != DC_CONTACT_ID_SELF {
            match chat.typ {
                Chattype::Group | Chattype::Broadcast | Chattype::Mailinglist => {
                    Some(Contact::get_by_id(context, self.from_id).await?)
                }
                Chattype::Single | Chattype::Undefined => None,
            }
        } else {
            None
        };

        Ok(Summary::new(context, self, chat, contact.as_ref()).await)
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
        self.param
            .get(Param::OverrideSenderDisplayname)
            .map(|name| name.to_string())
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
            // <https://support.delta.chat/t/videochat-with-webex/1412/4>.
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
    pub async fn set_quote(&mut self, context: &Context, quote: Option<&Message>) -> Result<()> {
        if let Some(quote) = quote {
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
                    quote
                        .get_summary(context, None)
                        .await?
                        .truncated_text(500)
                        .to_string()
                } else {
                    text
                },
            );
        } else {
            self.in_reply_to = None;
            self.param.remove(Param::Quote);
        }

        Ok(())
    }

    pub fn quoted_text(&self) -> Option<String> {
        self.param.get(Param::Quote).map(|s| s.to_string())
    }

    pub async fn quoted_message(&self, context: &Context) -> Result<Option<Message>> {
        if self.param.get(Param::Quote).is_some() && !self.is_forwarded() {
            return self.parent(context).await;
        }
        Ok(None)
    }

    pub(crate) async fn parent(&self, context: &Context) -> Result<Option<Message>> {
        if let Some(in_reply_to) = &self.in_reply_to {
            if let Some(msg_id) = rfc724_mid_exists(context, in_reply_to).await? {
                let msg = Message::load_from_db(context, msg_id).await?;
                return if msg.chat_id.is_trash() {
                    // If message is already moved to trash chat, pretend it does not exist.
                    Ok(None)
                } else {
                    Ok(Some(msg))
                };
            }
        }
        Ok(None)
    }

    /// Force the message to be sent in plain text.
    pub fn force_plaintext(&mut self) {
        self.param.set_int(Param::ForcePlaintext, 1);
    }

    pub async fn update_param(&self, context: &Context) {
        context
            .sql
            .execute(
                "UPDATE msgs SET param=? WHERE id=?;",
                paramsv![self.param.to_string(), self.id],
            )
            .await
            .ok_or_log(context);
    }

    pub(crate) async fn update_subject(&self, context: &Context) {
        context
            .sql
            .execute(
                "UPDATE msgs SET subject=? WHERE id=?;",
                paramsv![self.subject, self.id],
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
#[repr(u32)]
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

pub async fn get_msg_info(context: &Context, msg_id: MsgId) -> Result<String> {
    let msg = Message::load_from_db(context, msg_id).await?;
    let rawtxt: Option<String> = context
        .sql
        .query_get_value("SELECT txt_raw FROM msgs WHERE id=?;", paramsv![msg_id])
        .await?;

    let mut ret = String::new();

    if rawtxt.is_none() {
        ret += &format!("Cannot load message {}.", msg_id);
        return Ok(ret);
    }
    let rawtxt = rawtxt.unwrap_or_default();
    let rawtxt = dc_truncate(rawtxt.trim(), DC_DESIRED_TEXT_LEN);

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

            let name = Contact::load_from_db(context, contact_id.try_into()?)
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
    let hop_info: Option<String> = context
        .sql
        .query_get_value("SELECT hop_info FROM msgs WHERE id=?;", paramsv![msg_id])
        .await?;

    ret += "\n\n";
    ret += &hop_info.unwrap_or_else(|| "No Hop Info".to_owned());

    Ok(ret)
}

pub fn guess_msgtype_from_suffix(path: &Path) -> Option<(Viewtype, &str)> {
    let extension: &str = &path.extension()?.to_str()?.to_lowercase();
    let info = match extension {
        // before using viewtype other than Viewtype::File,
        // make sure, all target UIs support that type in the context of the used viewer/player.
        // if in doubt, it is better to default to Viewtype::File that passes handing to an external app.
        // (cmp. <https://developer.android.com/guide/topics/media/media-formats>)
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
        "w30" => (Viewtype::W30, "application/w30+zip"),
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

/// Get the raw mime-headers of the given message.
/// Raw headers are saved for incoming messages
/// only if `dc_set_config(context, "save_mime_headers", "1")`
/// was called before.
///
/// Returns an empty vector if there are no headers saved for the given message,
/// e.g. because of save_mime_headers is not set
/// or the message is not incoming.
pub async fn get_mime_headers(context: &Context, msg_id: MsgId) -> Result<Vec<u8>> {
    let headers = context
        .sql
        .query_row(
            "SELECT mime_headers FROM msgs WHERE id=?;",
            paramsv![msg_id],
            |row| {
                row.get(0).or_else(|err| match row.get_ref(0)? {
                    ValueRef::Null => Ok(Vec::new()),
                    ValueRef::Text(text) => Ok(text.to_vec()),
                    ValueRef::Blob(blob) => Ok(blob.to_vec()),
                    ValueRef::Integer(_) | ValueRef::Real(_) => Err(err),
                })
            },
        )
        .await?;
    Ok(headers)
}

pub async fn delete_msgs(context: &Context, msg_ids: &[MsgId]) -> Result<()> {
    for msg_id in msg_ids.iter() {
        let msg = Message::load_from_db(context, *msg_id).await?;
        if msg.location_id > 0 {
            delete_poi_location(context, msg.location_id).await;
        }
        msg_id
            .trash(context)
            .await
            .with_context(|| format!("Unable to trash message {}", msg_id))?;
        context
            .sql
            .execute(
                "UPDATE imap SET target='' WHERE rfc724_mid=?",
                paramsv![msg.rfc724_mid],
            )
            .await?;
    }

    if !msg_ids.is_empty() {
        context.emit_event(EventType::MsgsChanged {
            chat_id: ChatId::new(0),
            msg_id: MsgId::new(0),
        });
        job::kill_action(context, Action::Housekeeping).await?;
        job::add(
            context,
            job::Job::new(Action::Housekeeping, 0, Params::new(), 10),
        )
        .await?;
    }

    // Interrupt Inbox loop to start message deletion.
    context.interrupt_inbox(InterruptInfo::new(false)).await;
    Ok(())
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

pub async fn markseen_msgs(context: &Context, msg_ids: Vec<MsgId>) -> Result<()> {
    if msg_ids.is_empty() {
        return Ok(());
    }

    let conn = context.sql.get_conn().await?;
    let msgs = async_std::task::spawn_blocking(move || -> Result<_> {
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
        drop(stmt);
        drop(conn);
        Ok(msgs)
    })
    .await?;

    let mut updated_chat_ids = BTreeSet::new();

    for (id, curr_chat_id, curr_state, curr_blocked) in msgs.into_iter() {
        if let Err(err) = id.start_ephemeral_timer(context).await {
            error!(
                context,
                "Failed to start ephemeral timer for message {}: {}", id, err
            );
            continue;
        }

        if curr_blocked == Blocked::Not
            && (curr_state == MessageState::InFresh || curr_state == MessageState::InNoticed)
        {
            update_msg_state(context, id, MessageState::InSeen).await?;
            info!(context, "Seen message {}.", id);

            job::add(
                context,
                job::Job::new(Action::MarkseenMsgOnImap, id.to_u32(), Params::new(), 0),
            )
            .await?;
            updated_chat_ids.insert(curr_chat_id);
        }
    }

    for updated_chat_id in updated_chat_ids {
        context.emit_event(EventType::MsgsNoticed(updated_chat_id));
    }

    Ok(())
}

pub(crate) async fn update_msg_state(
    context: &Context,
    msg_id: MsgId,
    state: MessageState,
) -> Result<()> {
    context
        .sql
        .execute(
            "UPDATE msgs SET state=? WHERE id=?;",
            paramsv![state, msg_id],
        )
        .await?;
    Ok(())
}

// as we do not cut inside words, this results in about 32-42 characters.
// Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
// It should also be very clear, the subject is _not_ the whole message.
// The value is also used for CC:-summaries

// Context functions to work with messages

pub async fn exists(context: &Context, msg_id: MsgId) -> Result<bool> {
    if msg_id.is_special() {
        return Ok(false);
    }

    let chat_id: Option<ChatId> = context
        .sql
        .query_get_value("SELECT chat_id FROM msgs WHERE id=?;", paramsv![msg_id])
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
) -> Result<Option<(ChatId, MsgId)>> {
    if from_id == DC_CONTACT_ID_SELF {
        warn!(
            context,
            "ignoring MDN sent to self, this is a bug on the sender device"
        );

        // This is not an error on our side,
        // we successfully ignored an invalid MDN and return `Ok`.
        return Ok(None);
    }

    let res = context
        .sql
        .query_row_optional(
            concat!(
                "SELECT",
                "    m.id AS msg_id,",
                "    c.id AS chat_id,",
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
                    row.get::<_, MessageState>("state")?,
                ))
            },
        )
        .await?;

    let (msg_id, chat_id, msg_state) = if let Some(res) = res {
        res
    } else {
        info!(
            context,
            "handle_mdn found no message with Message-ID {:?} sent by us in the database",
            rfc724_mid
        );
        return Ok(None);
    };

    if !context
        .sql
        .exists(
            "SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=? AND contact_id=?;",
            paramsv![msg_id, from_id as i32,],
        )
        .await?
    {
        context
            .sql
            .execute(
                "INSERT INTO msgs_mdns (msg_id, contact_id, timestamp_sent) VALUES (?, ?, ?);",
                paramsv![msg_id, from_id as i32, timestamp_sent],
            )
            .await?;
    }

    if msg_state == MessageState::OutPreparing
        || msg_state == MessageState::OutPending
        || msg_state == MessageState::OutDelivered
    {
        update_msg_state(context, msg_id, MessageState::OutMdnRcvd).await?;
        Ok(Some((chat_id, msg_id)))
    } else {
        Ok(None)
    }
}

/// Marks a message as failed after an ndn (non-delivery-notification) arrived.
/// Where appropriate, also adds an info message telling the user which of the recipients of a group message failed.
pub(crate) async fn handle_ndn(
    context: &Context,
    failed: &FailureReport,
    error: Option<impl AsRef<str>>,
) -> Result<()> {
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

    let mut first = true;
    for msg in msgs.into_iter() {
        let (msg_id, chat_id, chat_type) = msg?;
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
) -> Result<()> {
    match chat_type {
        Chattype::Group | Chattype::Broadcast => {
            if let Some(failed_recipient) = &failed.failed_recipient {
                let contact_id =
                    Contact::lookup_id_by_addr(context, failed_recipient, Origin::Unknown)
                        .await?
                        .context("contact ID not found")?;
                let contact = Contact::load_from_db(context, contact_id).await?;
                // Tell the user which of the recipients failed if we know that (because in
                // a group, this might otherwise be unclear)
                let text = stock_str::failed_sending_to(context, contact.get_display_name()).await;
                chat::add_info_msg(
                    context,
                    chat_id,
                    &text,
                    dc_create_smeared_timestamp(context).await,
                )
                .await?;
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

/// The number of messages assigned to unblocked chats
pub async fn get_unblocked_msg_cnt(context: &Context) -> usize {
    match context
        .sql
        .count(
            "SELECT COUNT(*) \
         FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE m.id>9 AND m.chat_id>9 AND c.blocked=0;",
            paramsv![],
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!(context, "dc_get_unblocked_msg_cnt() failed. {}", err);
            0
        }
    }
}

/// Returns the number of messages in contact request chats.
pub async fn get_request_msg_cnt(context: &Context) -> usize {
    match context
        .sql
        .count(
            "SELECT COUNT(*) \
         FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE c.blocked=2;",
            paramsv![],
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!(context, "dc_get_request_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub async fn estimate_deletion_cnt(
    context: &Context,
    from_server: bool,
    seconds: i64,
) -> Result<usize> {
    let self_chat_id = ChatId::lookup_by_contact(context, DC_CONTACT_ID_SELF)
        .await?
        .unwrap_or_default();
    let threshold_timestamp = time() - seconds;

    let cnt = if from_server {
        context
            .sql
            .count(
                "SELECT COUNT(*)
             FROM msgs m
             WHERE m.id > ?
               AND timestamp < ?
               AND chat_id != ?
               AND EXISTS (SELECT * FROM imap WHERE rfc724_mid=m.rfc724_mid);",
                paramsv![DC_MSG_ID_LAST_SPECIAL, threshold_timestamp, self_chat_id],
            )
            .await?
    } else {
        context
            .sql
            .count(
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
                    DC_CHAT_ID_TRASH
                ],
            )
            .await?
    };
    Ok(cnt)
}

pub(crate) async fn rfc724_mid_exists(
    context: &Context,
    rfc724_mid: &str,
) -> Result<Option<MsgId>> {
    let rfc724_mid = rfc724_mid.trim_start_matches('<').trim_end_matches('>');
    if rfc724_mid.is_empty() {
        warn!(context, "Empty rfc724_mid passed to rfc724_mid_exists");
        return Ok(None);
    }

    let res = context
        .sql
        .query_row_optional(
            "SELECT id FROM msgs WHERE rfc724_mid=?",
            paramsv![rfc724_mid],
            |row| {
                let msg_id: MsgId = row.get(0)?;

                Ok(msg_id)
            },
        )
        .await?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{marknoticed_chat, ChatItem};
    use crate::chatlist::Chatlist;
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
        assert_eq!(
            guess_msgtype_from_suffix(Path::new("foo/file.html")),
            Some((Viewtype::File, "text/html"))
        );
        assert_eq!(
            guess_msgtype_from_suffix(Path::new("foo/file.w30")),
            Some((Viewtype::W30, "application/w30+zip"))
        );
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
        let device_chat_id = ChatId::get_for_contact(&t, DC_CONTACT_ID_DEVICE)
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
        msg2.set_quote(ctx, Some(&msg))
            .await
            .expect("can't set quote");
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
                    To: alice@example.org\n\
                    Chat-Version: 1.0\n\
                    Message-ID: <123@example.com>\n\
                    Date: Fri, 29 Jan 2021 21:37:55 +0000\n\
                    \n\
                    hello\n",
            "INBOX",
            false,
        )
        .await
        .unwrap();

        // check chat-id of this message
        let msg = alice.get_last_msg().await;
        assert!(!msg.get_chat_id().is_special());
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
            .unwrap()
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
            .unwrap()
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

    #[async_std::test]
    async fn test_markseen_msgs() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let alice_chat = alice.create_chat(&bob).await;
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("this is the text!".to_string()));

        // alice sends to bob,
        assert_eq!(Chatlist::try_load(&bob, 0, None, None).await?.len(), 0);
        bob.recv_msg(&alice.send_msg(alice_chat.id, &mut msg).await)
            .await;
        let msg1 = bob.get_last_msg().await;
        let bob_chat_id = msg1.chat_id;
        bob.recv_msg(&alice.send_msg(alice_chat.id, &mut msg).await)
            .await;
        let msg2 = bob.get_last_msg().await;
        assert_eq!(msg1.chat_id, msg2.chat_id);
        let chats = Chatlist::try_load(&bob, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        let msgs = chat::get_chat_msgs(&bob, bob_chat_id, 0, None).await?;
        assert_eq!(msgs.len(), 2);
        assert_eq!(bob.get_fresh_msgs().await?.len(), 0);

        // that has no effect in contact request
        markseen_msgs(&bob, vec![msg1.id, msg2.id]).await?;

        assert_eq!(Chatlist::try_load(&bob, 0, None, None).await?.len(), 1);
        let bob_chat = Chat::load_from_db(&bob, bob_chat_id).await?;
        assert_eq!(bob_chat.blocked, Blocked::Request);

        let msgs = chat::get_chat_msgs(&bob, bob_chat_id, 0, None).await?;
        assert_eq!(msgs.len(), 2);
        bob_chat_id.accept(&bob).await.unwrap();

        // bob sends to alice,
        // alice knows bob and messages appear in normal chat
        alice
            .recv_msg(&bob.send_msg(bob_chat_id, &mut msg).await)
            .await;
        let msg1 = alice.get_last_msg().await;
        alice
            .recv_msg(&bob.send_msg(bob_chat_id, &mut msg).await)
            .await;
        let msg2 = alice.get_last_msg().await;
        let chats = Chatlist::try_load(&alice, 0, None, None).await?;
        assert_eq!(chats.len(), 1);
        assert_eq!(chats.get_chat_id(0)?, alice_chat.id);
        assert_eq!(chats.get_chat_id(0)?, msg1.chat_id);
        assert_eq!(chats.get_chat_id(0)?, msg2.chat_id);
        assert_eq!(alice_chat.id.get_fresh_msg_cnt(&alice).await?, 2);
        assert_eq!(alice.get_fresh_msgs().await?.len(), 2);

        // no message-ids, that should have no effect
        markseen_msgs(&alice, vec![]).await?;

        // bad message-id, that should have no effect
        markseen_msgs(&alice, vec![MsgId::new(123456)]).await?;

        assert_eq!(alice_chat.id.get_fresh_msg_cnt(&alice).await?, 2);
        assert_eq!(alice.get_fresh_msgs().await?.len(), 2);

        // mark the most recent as seen
        markseen_msgs(&alice, vec![msg2.id]).await?;

        assert_eq!(alice_chat.id.get_fresh_msg_cnt(&alice).await?, 1);
        assert_eq!(alice.get_fresh_msgs().await?.len(), 1);

        // user scrolled up - mark both as seen
        markseen_msgs(&alice, vec![msg1.id, msg2.id]).await?;

        assert_eq!(alice_chat.id.get_fresh_msg_cnt(&alice).await?, 0);
        assert_eq!(alice.get_fresh_msgs().await?.len(), 0);

        Ok(())
    }

    #[async_std::test]
    async fn test_get_state() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let alice_chat = alice.create_chat(&bob).await;
        let bob_chat = bob.create_chat(&alice).await;

        // check both get_state() functions,
        // the one requiring a id and the one requiring an object
        async fn assert_state(t: &Context, msg_id: MsgId, state: MessageState) {
            assert_eq!(msg_id.get_state(t).await.unwrap(), state);
            assert_eq!(
                Message::load_from_db(t, msg_id).await.unwrap().get_state(),
                state
            );
        }

        // check outgoing messages states on sender side
        let mut alice_msg = Message::new(Viewtype::Text);
        alice_msg.set_text(Some("hi!".to_string()));
        assert_eq!(alice_msg.get_state(), MessageState::Undefined); // message not yet in db, assert_state() won't work

        alice_chat
            .id
            .set_draft(&alice, Some(&mut alice_msg))
            .await?;
        let mut alice_msg = alice_chat.id.get_draft(&alice).await?.unwrap();
        assert_state(&alice, alice_msg.id, MessageState::OutDraft).await;

        let msg_id = chat::send_msg(&alice, alice_chat.id, &mut alice_msg).await?;
        assert_eq!(msg_id, alice_msg.id);
        assert_state(&alice, alice_msg.id, MessageState::OutPending).await;

        let payload = alice.pop_sent_msg().await;
        assert_state(&alice, alice_msg.id, MessageState::OutDelivered).await;

        update_msg_state(&alice, alice_msg.id, MessageState::OutMdnRcvd).await?;
        assert_state(&alice, alice_msg.id, MessageState::OutMdnRcvd).await;

        set_msg_failed(&alice, alice_msg.id, Some("badly failed")).await;
        assert_state(&alice, alice_msg.id, MessageState::OutFailed).await;

        // check incoming message states on receiver side
        bob.recv_msg(&payload).await;
        let bob_msg = bob.get_last_msg().await;
        assert_eq!(bob_chat.id, bob_msg.chat_id);
        assert_state(&bob, bob_msg.id, MessageState::InFresh).await;

        marknoticed_chat(&bob, bob_msg.chat_id).await?;
        assert_state(&bob, bob_msg.id, MessageState::InNoticed).await;

        markseen_msgs(&bob, vec![bob_msg.id]).await?;
        assert_state(&bob, bob_msg.id, MessageState::InSeen).await;

        Ok(())
    }

    #[async_std::test]
    async fn test_is_bot() -> Result<()> {
        let alice = TestContext::new_alice().await;

        // Alice receives a message from Bob the bot.
        dc_receive_imf(
            &alice,
            b"From: Bob <bob@example.com>\n\
                    To: alice@example.org\n\
                    Chat-Version: 1.0\n\
                    Message-ID: <123@example.com>\n\
                    Auto-Submitted: auto-generated\n\
                    Date: Fri, 29 Jan 2021 21:37:55 +0000\n\
                    \n\
                    hello\n",
            "INBOX",
            false,
        )
        .await?;
        let msg = alice.get_last_msg().await;
        assert_eq!(msg.get_text().unwrap(), "hello".to_string());
        assert!(msg.is_bot());

        // Alice receives a message from Bob who is not the bot anymore.
        dc_receive_imf(
            &alice,
            b"From: Bob <bob@example.com>\n\
                    To: alice@example.org\n\
                    Chat-Version: 1.0\n\
                    Message-ID: <456@example.com>\n\
                    Date: Fri, 29 Jan 2021 21:37:55 +0000\n\
                    \n\
                    hello again\n",
            "INBOX",
            false,
        )
        .await?;
        let msg = alice.get_last_msg().await;
        assert_eq!(msg.get_text().unwrap(), "hello again".to_string());
        assert!(!msg.is_bot());

        Ok(())
    }
}

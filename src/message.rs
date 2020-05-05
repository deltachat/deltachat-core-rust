//! # Messages and their identifiers

use std::path::{Path, PathBuf};

use deltachat_derive::{FromSql, ToSql};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::chat::{self, Chat, ChatId};
use crate::constants::*;
use crate::contact::*;
use crate::context::*;
use crate::dc_tools::*;
use crate::error::{ensure, Error};
use crate::events::Event;
use crate::job::*;
use crate::lot::{Lot, LotState, Meaning};
use crate::mimeparser::SystemMessage;
use crate::param::*;
use crate::pgp::*;
use crate::sql;
use crate::stock::StockMessage;

lazy_static! {
    static ref UNWRAP_RE: regex::Regex = regex::Regex::new(r"\s+").unwrap();
}

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

    /// Whether the message ID is the special marker1 marker.
    ///
    /// See the docs of the `dc_get_chat_msgs` C API for details.
    pub fn is_marker1(self) -> bool {
        self.0 == DC_MSG_ID_MARKER1
    }

    /// Whether the message ID is the special day marker.
    ///
    /// See the docs of the `dc_get_chat_msgs` C API for details.
    pub fn is_daymarker(self) -> bool {
        self.0 == DC_MSG_ID_DAYMARKER
    }

    /// Put message into trash chat and delete message text.
    ///
    /// It means the message is deleted locally, but not on the server
    /// yet.
    pub fn trash(self, context: &Context) -> crate::sql::Result<()> {
        let chat_id = ChatId::new(DC_CHAT_ID_TRASH);
        sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET chat_id=?, txt='', txt_raw='' WHERE id=?",
            params![chat_id, self],
        )
    }

    /// Deletes a message and corresponding MDNs from the database.
    pub fn delete_from_db(self, context: &Context) -> crate::sql::Result<()> {
        // We don't use transactions yet, so remove MDNs first to make
        // sure they are not left while the message is deleted.
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM msgs_mdns WHERE msg_id=?;",
            params![self],
        )?;
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM msgs WHERE id=?;",
            params![self],
        )?;
        Ok(())
    }

    /// Removes IMAP server UID and folder from the database record.
    ///
    /// It is used to avoid trying to remove the message from the
    /// server multiple times when there are multiple message records
    /// pointing to the same server UID.
    pub(crate) fn unlink(self, context: &Context) -> sql::Result<()> {
        sql::execute(
            context,
            &context.sql,
            "UPDATE msgs \
             SET server_folder='', server_uid=0 \
             WHERE id=?",
            params![self],
        )
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
        // Would be nice if we could use match here, but no computed values in ranges.
        if self.0 == DC_MSG_ID_MARKER1 {
            write!(f, "Msg#Marker1")
        } else if self.0 == DC_MSG_ID_DAYMARKER {
            write!(f, "Msg#DayMarker")
        } else if self.0 <= DC_MSG_ID_LAST_SPECIAL {
            write!(f, "Msg#UnknownSpecial")
        } else {
            write!(f, "Msg#{}", self.0)
        }
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
    pub(crate) text: Option<String>,
    pub(crate) rfc724_mid: String,
    pub(crate) in_reply_to: Option<String>,
    pub(crate) server_folder: Option<String>,
    pub(crate) server_uid: u32,
    pub(crate) is_dc_message: MessengerMessage,
    pub(crate) starred: bool,
    pub(crate) chat_blocked: Blocked,
    pub(crate) location_id: u32,
    pub(crate) param: Params,
}

impl Message {
    pub fn new(viewtype: Viewtype) -> Self {
        let mut msg = Message::default();
        msg.viewtype = viewtype;

        msg
    }

    pub fn load_from_db(context: &Context, id: MsgId) -> Result<Message, Error> {
        ensure!(
            !id.is_special(),
            "Can not load special message IDs from DB."
        );
        context
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
                    "    m.type AS type,",
                    "    m.state AS state,",
                    "    m.msgrmsg AS msgrmsg,",
                    "    m.txt AS txt,",
                    "    m.param AS param,",
                    "    m.starred AS starred,",
                    "    m.hidden AS hidden,",
                    "    m.location_id AS location,",
                    "    c.blocked AS blocked",
                    " FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id",
                    " WHERE m.id=?;"
                ),
                params![id],
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
                    msg.viewtype = row.get("type")?;
                    msg.state = row.get("state")?;
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
                    msg.starred = row.get("starred")?;
                    msg.hidden = row.get("hidden")?;
                    msg.location_id = row.get("location")?;
                    msg.chat_blocked = row
                        .get::<_, Option<Blocked>>("blocked")?
                        .unwrap_or_default();

                    Ok(msg)
                },
            )
            .map_err(Into::into)
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

    pub fn try_calc_and_set_dimensions(&mut self, context: &Context) -> Result<(), Error> {
        if chat::msgtype_has_file(self.viewtype) {
            let file_param = self.param.get_path(Param::File, context)?;
            if let Some(path_and_filename) = file_param {
                if (self.viewtype == Viewtype::Image || self.viewtype == Viewtype::Gif)
                    && !self.param.exists(Param::Width)
                {
                    self.param.set_int(Param::Width, 0);
                    self.param.set_int(Param::Height, 0);

                    if let Ok(buf) = dc_read_file(context, path_and_filename) {
                        if let Ok((width, height)) = dc_get_filemeta(&buf) {
                            self.param.set_int(Param::Width, width as i32);
                            self.param.set_int(Param::Height, height as i32);
                        }
                    }

                    if !self.id.is_unset() {
                        self.save_param_to_disk(context);
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
            .map(|text| dc_truncate(text, 30000).to_string())
    }

    pub fn get_filename(&self) -> Option<String> {
        self.param
            .get(Param::File)
            .and_then(|file| Path::new(file).file_name())
            .map(|name| name.to_string_lossy().to_string())
    }

    pub fn get_filebytes(&self, context: &Context) -> u64 {
        self.param
            .get_path(Param::File, context)
            .unwrap_or(None)
            .map(|path| dc_get_filebytes(context, &path))
            .unwrap_or_default()
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

    pub fn get_summary(&mut self, context: &Context, chat: Option<&Chat>) -> Lot {
        let mut ret = Lot::new();

        let chat_loaded: Chat;
        let chat = if let Some(chat) = chat {
            chat
        } else if let Ok(chat) = Chat::load_from_db(context, self.chat_id) {
            chat_loaded = chat;
            &chat_loaded
        } else {
            return ret;
        };

        let contact = if self.from_id != DC_CONTACT_ID_SELF as u32
            && (chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup)
        {
            Contact::get_by_id(context, self.from_id).ok()
        } else {
            None
        };

        ret.fill(self, chat, contact.as_ref(), context);

        ret
    }

    pub fn get_summarytext(&self, context: &Context, approx_characters: usize) -> String {
        get_summarytext_by_raw(
            self.viewtype,
            self.text.as_ref(),
            &self.param,
            approx_characters,
            context,
        )
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

    pub fn is_starred(&self) -> bool {
        self.starred
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

    pub fn get_setupcodebegin(&self, context: &Context) -> Option<String> {
        if !self.is_setupmessage() {
            return None;
        }

        if let Some(filename) = self.get_file(context) {
            if let Ok(ref buf) = dc_read_file(context, filename) {
                if let Ok((typ, headers, _)) = split_armored_data(buf) {
                    if typ == pgp::armor::BlockType::Message {
                        return headers.get(crate::pgp::HEADER_SETUPCODE).cloned();
                    }
                }
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

    pub fn latefiling_mediasize(
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
        self.save_param_to_disk(context);
    }

    pub fn save_param_to_disk(&mut self, context: &Context) -> bool {
        sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET param=? WHERE id=?;",
            params![self.param.to_string(), self.id],
        )
        .is_ok()
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
        match self {
            MessageState::OutPreparing | MessageState::OutPending | MessageState::OutDelivered => {
                true
            }
            _ => false,
        }
    }
}

impl Lot {
    /* library-internal */
    /* in practice, the user additionally cuts the string himself pixel-accurate */
    pub fn fill(
        &mut self,
        msg: &mut Message,
        chat: &Chat,
        contact: Option<&Contact>,
        context: &Context,
    ) {
        if msg.state == MessageState::OutDraft {
            self.text1 = Some(context.stock_str(StockMessage::Draft).to_owned().into());
            self.text1_meaning = Meaning::Text1Draft;
        } else if msg.from_id == DC_CONTACT_ID_SELF {
            if msg.is_info() || chat.is_self_talk() {
                self.text1 = None;
                self.text1_meaning = Meaning::None;
            } else {
                self.text1 = Some(context.stock_str(StockMessage::SelfMsg).to_owned().into());
                self.text1_meaning = Meaning::Text1Self;
            }
        } else if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
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

        self.text2 = Some(get_summarytext_by_raw(
            msg.viewtype,
            msg.text.as_ref(),
            &msg.param,
            SUMMARY_CHARACTERS,
            context,
        ));

        self.timestamp = msg.get_timestamp();
        self.state = msg.state.into();
    }
}

pub fn get_msg_info(context: &Context, msg_id: MsgId) -> String {
    let mut ret = String::new();

    let msg = Message::load_from_db(context, msg_id);
    if msg.is_err() {
        return ret;
    }

    let msg = msg.unwrap_or_default();

    let rawtxt: Option<String> = context.sql.query_get_value(
        context,
        "SELECT txt_raw FROM msgs WHERE id=?;",
        params![msg_id],
    );

    if rawtxt.is_none() {
        ret += &format!("Cannot load message {}.", msg_id);
        return ret;
    }
    let rawtxt = rawtxt.unwrap_or_default();
    let rawtxt = dc_truncate(rawtxt.trim(), 100_000);

    let fts = dc_timestamp_to_str(msg.get_timestamp());
    ret += &format!("Sent: {}", fts);

    let name = Contact::load_from_db(context, msg.from_id)
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

    if msg.from_id == DC_CONTACT_ID_INFO || msg.to_id == DC_CONTACT_ID_INFO {
        // device-internal message, no further details needed
        return ret;
    }

    if let Ok(rows) = context.sql.query_map(
        "SELECT contact_id, timestamp_sent FROM msgs_mdns WHERE msg_id=?;",
        params![msg_id],
        |row| {
            let contact_id: i32 = row.get(0)?;
            let ts: i64 = row.get(1)?;
            Ok((contact_id, ts))
        },
        |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
    ) {
        for (contact_id, ts) in rows {
            let fts = dc_timestamp_to_str(ts);
            ret += &format!("Read: {}", fts);

            let name = Contact::load_from_db(context, contact_id as u32)
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
    if let Some(err) = msg.param.get(Param::Error) {
        ret += &format!("Error: {}", err)
    }

    if let Some(path) = msg.get_file(context) {
        let bytes = dc_get_filebytes(context, &path);
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
        "mp3" => (Viewtype::Audio, "audio/mpeg"),
        "aac" => (Viewtype::Audio, "audio/aac"),
        "mp4" => (Viewtype::Video, "video/mp4"),
        "webm" => (Viewtype::Video, "video/webm"),
        "jpg" => (Viewtype::Image, "image/jpeg"),
        "jpeg" => (Viewtype::Image, "image/jpeg"),
        "jpe" => (Viewtype::Image, "image/jpeg"),
        "png" => (Viewtype::Image, "image/png"),
        "webp" => (Viewtype::Image, "image/webp"),
        "gif" => (Viewtype::Gif, "image/gif"),
        "vcf" => (Viewtype::File, "text/vcard"),
        "vcard" => (Viewtype::File, "text/vcard"),
        _ => {
            return None;
        }
    };
    Some(info)
}

pub fn get_mime_headers(context: &Context, msg_id: MsgId) -> Option<String> {
    context.sql.query_get_value(
        context,
        "SELECT mime_headers FROM msgs WHERE id=?;",
        params![msg_id],
    )
}

pub fn delete_msgs(context: &Context, msg_ids: &[MsgId]) {
    for msg_id in msg_ids {
        if let Ok(msg) = Message::load_from_db(context, *msg_id) {
            if msg.location_id > 0 {
                delete_poi_location(context, msg.location_id);
            }
        }
        if let Err(err) = msg_id.trash(context) {
            error!(context, "Unable to trash message {}: {}", msg_id, err);
        }
        job_add(
            context,
            Action::DeleteMsgOnImap,
            msg_id.to_u32() as i32,
            Params::new(),
            0,
        );
    }

    if !msg_ids.is_empty() {
        context.call_cb(Event::MsgsChanged {
            chat_id: ChatId::new(0),
            msg_id: MsgId::new(0),
        });
        job_kill_action(context, Action::Housekeeping);
        job_add(context, Action::Housekeeping, 0, Params::new(), 10);
    };
}

fn delete_poi_location(context: &Context, location_id: u32) -> bool {
    sql::execute(
        context,
        &context.sql,
        "DELETE FROM locations WHERE independent = 1 AND id=?;",
        params![location_id as i32],
    )
    .is_ok()
}

pub fn markseen_msgs(context: &Context, msg_ids: &[MsgId]) -> bool {
    if msg_ids.is_empty() {
        return false;
    }

    let msgs = context.sql.prepare(
        concat!(
            "SELECT",
            "    m.state AS state,",
            "    c.blocked AS blocked",
            " FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id",
            " WHERE m.id=? AND m.chat_id>9"
        ),
        |mut stmt, _| {
            let mut res = Vec::with_capacity(msg_ids.len());
            for id in msg_ids.iter() {
                let query_res = stmt.query_row(params![*id], |row| {
                    Ok((
                        row.get::<_, MessageState>("state")?,
                        row.get::<_, Option<Blocked>>("blocked")?
                            .unwrap_or_default(),
                    ))
                });
                if let Err(rusqlite::Error::QueryReturnedNoRows) = query_res {
                    continue;
                }
                let (state, blocked) = query_res?;
                res.push((id, state, blocked));
            }

            Ok(res)
        },
    );

    if msgs.is_err() {
        warn!(context, "markseen_msgs failed: {:?}", msgs);
        return false;
    }
    let mut send_event = false;
    let msgs = msgs.unwrap_or_default();

    for (id, curr_state, curr_blocked) in msgs.into_iter() {
        if curr_blocked == Blocked::Not {
            if curr_state == MessageState::InFresh || curr_state == MessageState::InNoticed {
                update_msg_state(context, *id, MessageState::InSeen);
                info!(context, "Seen message {}.", id);

                job_add(
                    context,
                    Action::MarkseenMsgOnImap,
                    id.to_u32() as i32,
                    Params::new(),
                    0,
                );
                send_event = true;
            }
        } else if curr_state == MessageState::InFresh {
            update_msg_state(context, *id, MessageState::InNoticed);
            send_event = true;
        }
    }

    if send_event {
        context.call_cb(Event::MsgsChanged {
            chat_id: ChatId::new(0),
            msg_id: MsgId::new(0),
        });
    }

    true
}

pub fn update_msg_state(context: &Context, msg_id: MsgId, state: MessageState) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET state=? WHERE id=?;",
        params![state, msg_id],
    )
    .is_ok()
}

pub fn star_msgs(context: &Context, msg_ids: &[MsgId], star: bool) -> bool {
    if msg_ids.is_empty() {
        return false;
    }
    context
        .sql
        .prepare("UPDATE msgs SET starred=? WHERE id=?;", |mut stmt, _| {
            for msg_id in msg_ids.iter() {
                stmt.execute(params![star as i32, *msg_id])?;
            }
            Ok(())
        })
        .is_ok()
}

/// Returns a summary test.
pub fn get_summarytext_by_raw(
    viewtype: Viewtype,
    text: Option<impl AsRef<str>>,
    param: &Params,
    approx_characters: usize,
    context: &Context,
) -> String {
    let mut append_text = true;
    let prefix = match viewtype {
        Viewtype::Image => context.stock_str(StockMessage::Image).into_owned(),
        Viewtype::Gif => context.stock_str(StockMessage::Gif).into_owned(),
        Viewtype::Sticker => context.stock_str(StockMessage::Sticker).into_owned(),
        Viewtype::Video => context.stock_str(StockMessage::Video).into_owned(),
        Viewtype::Voice => context.stock_str(StockMessage::VoiceMessage).into_owned(),
        Viewtype::Audio | Viewtype::File => {
            if param.get_cmd() == SystemMessage::AutocryptSetupMessage {
                append_text = false;
                context
                    .stock_str(StockMessage::AcSetupMsgSubject)
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
                let label = context.stock_str(if viewtype == Viewtype::Audio {
                    StockMessage::Audio
                } else {
                    StockMessage::File
                });
                format!("{} – {}", label, file_name)
            }
        }
        _ => {
            if param.get_cmd() != SystemMessage::LocationOnly {
                "".to_string()
            } else {
                append_text = false;
                context.stock_str(StockMessage::Location).to_string()
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

    UNWRAP_RE.replace_all(&summary, " ").to_string()
}

// as we do not cut inside words, this results in about 32-42 characters.
// Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
// It should also be very clear, the subject is _not_ the whole message.
// The value is also used for CC:-summaries

// Context functions to work with messages

pub fn exists(context: &Context, msg_id: MsgId) -> bool {
    if msg_id.is_special() {
        return false;
    }

    let chat_id: Option<ChatId> = context.sql.query_get_value(
        context,
        "SELECT chat_id FROM msgs WHERE id=?;",
        params![msg_id],
    );

    if let Some(chat_id) = chat_id {
        !chat_id.is_trash()
    } else {
        false
    }
}

pub fn set_msg_failed(context: &Context, msg_id: MsgId, error: Option<impl AsRef<str>>) {
    if let Ok(mut msg) = Message::load_from_db(context, msg_id) {
        if msg.state.can_fail() {
            msg.state = MessageState::OutFailed;
        }
        if let Some(error) = error {
            msg.param.set(Param::Error, error.as_ref());
            warn!(context, "Message failed: {}", error.as_ref());
        }

        if sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET state=?, param=? WHERE id=?;",
            params![msg.state, msg.param.to_string(), msg_id],
        )
        .is_ok()
        {
            context.call_cb(Event::MsgFailed {
                chat_id: msg.chat_id,
                msg_id,
            });
        }
    }
}

/// returns Some if an event should be send
pub fn mdn_from_ext(
    context: &Context,
    from_id: u32,
    rfc724_mid: &str,
    timestamp_sent: i64,
) -> Option<(ChatId, MsgId)> {
    if from_id <= DC_MSG_ID_LAST_SPECIAL || rfc724_mid.is_empty() {
        return None;
    }

    let res = context.sql.query_row(
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
        params![rfc724_mid],
        |row| {
            Ok((
                row.get::<_, MsgId>("msg_id")?,
                row.get::<_, ChatId>("chat_id")?,
                row.get::<_, Chattype>("type")?,
                row.get::<_, MessageState>("state")?,
            ))
        },
    );
    if let Err(ref err) = res {
        info!(context, "Failed to select MDN {:?}", err);
    }

    if let Ok((msg_id, chat_id, chat_type, msg_state)) = res {
        let mut read_by_all = false;

        // if already marked as MDNS_RCVD msgstate_can_fail() returns false.
        // however, it is important, that ret_msg_id is set above as this
        // will allow the caller eg. to move the message away
        if msg_state.can_fail() {
            let mdn_already_in_table = context
                .sql
                .exists(
                    "SELECT contact_id FROM msgs_mdns WHERE msg_id=? AND contact_id=?;",
                    params![msg_id, from_id as i32,],
                )
                .unwrap_or_default();

            if !mdn_already_in_table {
                context.sql.execute(
                    "INSERT INTO msgs_mdns (msg_id, contact_id, timestamp_sent) VALUES (?, ?, ?);",
                    params![msg_id, from_id as i32, timestamp_sent],
                ).unwrap_or_default(); // TODO: better error handling
            }

            // Normal chat? that's quite easy.
            if chat_type == Chattype::Single {
                update_msg_state(context, msg_id, MessageState::OutMdnRcvd);
                read_by_all = true;
            } else {
                // send event about new state
                let ist_cnt = context
                    .sql
                    .query_get_value::<_, isize>(
                        context,
                        "SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=?;",
                        params![msg_id],
                    )
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
                let soll_cnt = (chat::get_chat_contact_cnt(context, chat_id) + 1) / 2;
                if ist_cnt >= soll_cnt {
                    update_msg_state(context, msg_id, MessageState::OutMdnRcvd);
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

/// The number of messages assigned to real chat (!=deaddrop, !=trash)
pub fn get_real_msg_cnt(context: &Context) -> i32 {
    match context.sql.query_row(
        "SELECT COUNT(*) \
         FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE m.id>9 AND m.chat_id>9 AND c.blocked=0;",
        rusqlite::NO_PARAMS,
        |row| row.get(0),
    ) {
        Ok(res) => res,
        Err(err) => {
            error!(context, "dc_get_real_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub fn get_deaddrop_msg_cnt(context: &Context) -> usize {
    match context.sql.query_row(
        "SELECT COUNT(*) \
         FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE c.blocked=2;",
        rusqlite::NO_PARAMS,
        |row| row.get::<_, isize>(0),
    ) {
        Ok(res) => res as usize,
        Err(err) => {
            error!(context, "dc_get_deaddrop_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub fn estimate_deletion_cnt(
    context: &Context,
    from_server: bool,
    seconds: i64,
) -> Result<usize, Error> {
    let self_chat_id = chat::lookup_by_contact_id(context, DC_CONTACT_ID_SELF)
        .unwrap_or_default()
        .0;
    let threshold_timestamp = time() - seconds;

    let cnt: isize = if from_server {
        context.sql.query_row(
            "SELECT COUNT(*)
             FROM msgs m
             WHERE m.id > ?
               AND timestamp < ?
               AND chat_id != ?
               AND server_uid != 0;",
            params![DC_MSG_ID_LAST_SPECIAL, threshold_timestamp, self_chat_id],
            |row| row.get(0),
        )?
    } else {
        context.sql.query_row(
            "SELECT COUNT(*)
             FROM msgs m
             WHERE m.id > ?
               AND timestamp < ?
               AND chat_id != ?
               AND chat_id != ? AND hidden = 0;",
            params![
                DC_MSG_ID_LAST_SPECIAL,
                threshold_timestamp,
                self_chat_id,
                ChatId::new(DC_CHAT_ID_TRASH)
            ],
            |row| row.get(0),
        )?
    };
    Ok(cnt as usize)
}

/// Counts number of database records pointing to specified
/// Message-ID.
///
/// Unlinked messages are excluded.
pub fn rfc724_mid_cnt(context: &Context, rfc724_mid: &str) -> i32 {
    // check the number of messages with the same rfc724_mid
    match context.sql.query_row(
        "SELECT COUNT(*) FROM msgs WHERE rfc724_mid=? AND NOT server_uid = 0",
        &[rfc724_mid],
        |row| row.get(0),
    ) {
        Ok(res) => res,
        Err(err) => {
            error!(context, "dc_get_rfc724_mid_cnt() failed. {}", err);
            0
        }
    }
}

pub(crate) fn rfc724_mid_exists(
    context: &Context,
    rfc724_mid: &str,
) -> Result<Option<(String, u32, MsgId)>, Error> {
    ensure!(!rfc724_mid.is_empty(), "empty rfc724_mid");

    context
        .sql
        .query_row_optional(
            "SELECT server_folder, server_uid, id FROM msgs WHERE rfc724_mid=?",
            &[rfc724_mid],
            |row| {
                let server_folder = row.get::<_, Option<String>>(0)?.unwrap_or_default();
                let server_uid = row.get(1)?;
                let msg_id: MsgId = row.get(2)?;

                Ok((server_folder, server_uid, msg_id))
            },
        )
        .map_err(Into::into)
}

pub fn update_server_uid(
    context: &Context,
    rfc724_mid: &str,
    server_folder: impl AsRef<str>,
    server_uid: u32,
) {
    match context.sql.execute(
        "UPDATE msgs SET server_folder=?, server_uid=? \
         WHERE rfc724_mid=?",
        params![server_folder.as_ref(), server_uid, rfc724_mid],
    ) {
        Ok(_) => {}
        Err(err) => {
            warn!(context, "msg: failed to update server_uid: {}", err);
        }
    }
}

#[allow(dead_code)]
pub fn dc_empty_server(context: &Context, flags: u32) {
    job_kill_action(context, Action::EmptyServer);
    job_add(context, Action::EmptyServer, flags as i32, Params::new(), 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils as test;

    #[test]
    fn test_guess_msgtype_from_suffix() {
        assert_eq!(
            guess_msgtype_from_suffix(Path::new("foo/bar-sth.mp3")),
            Some((Viewtype::Audio, "audio/mpeg"))
        );
    }

    #[test]
    pub fn test_prepare_message_and_send() {
        use crate::config::Config;

        let d = test::dummy_context();
        let ctx = &d.ctx;

        let contact =
            Contact::create(ctx, "", "dest@example.com").expect("failed to create contact");

        let res = ctx.set_config(Config::ConfiguredAddr, Some("self@example.com"));
        assert!(res.is_ok());

        let chat = chat::create_by_contact_id(ctx, contact).unwrap();

        let mut msg = Message::new(Viewtype::Text);

        let msg_id = chat::prepare_msg(ctx, chat, &mut msg).unwrap();

        let _msg2 = Message::load_from_db(ctx, msg_id).unwrap();
        assert_eq!(_msg2.get_filemime(), None);
    }

    #[test]
    pub fn test_get_summarytext_by_raw() {
        let d = test::dummy_context();
        let ctx = &d.ctx;

        let some_text = Some("bla bla".to_string());
        let empty_text = Some("".to_string());
        let no_text: Option<String> = None;

        let mut some_file = Params::new();
        some_file.set(Param::File, "foo.bar");

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Text, some_text.as_ref(), &Params::new(), 50, &ctx),
            "bla bla" // for simple text, the type is not added to the summary
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Image, no_text.as_ref(), &some_file, 50, &ctx,),
            "Image" // file names are not added for images
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Video, no_text.as_ref(), &some_file, 50, &ctx,),
            "Video" // file names are not added for videos
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Gif, no_text.as_ref(), &some_file, 50, &ctx,),
            "GIF" // file names are not added for GIFs
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Sticker, no_text.as_ref(), &some_file, 50, &ctx,),
            "Sticker" // file names are not added for stickers
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, empty_text.as_ref(), &some_file, 50, &ctx,),
            "Voice message" // file names are not added for voice messages, empty text is skipped
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, no_text.as_ref(), &mut some_file, 50, &ctx),
            "Voice message" // file names are not added for voice messages
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Voice, some_text.as_ref(), &some_file, 50, &ctx),
            "Voice message \u{2013} bla bla" // `\u{2013}` explicitly checks for "EN DASH"
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, no_text.as_ref(), &mut some_file, 50, &ctx),
            "Audio \u{2013} foo.bar" // file name is added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, empty_text.as_ref(), &some_file, 50, &ctx,),
            "Audio \u{2013} foo.bar" // file name is added for audio, empty text is not added
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::Audio, some_text.as_ref(), &some_file, 50, &ctx),
            "Audio \u{2013} foo.bar \u{2013} bla bla" // file name and text added for audio
        );

        assert_eq!(
            get_summarytext_by_raw(Viewtype::File, some_text.as_ref(), &mut some_file, 50, &ctx),
            "File \u{2013} foo.bar \u{2013} bla bla" // file name is added for files
        );

        let mut asm_file = Params::new();
        asm_file.set(Param::File, "foo.bar");
        asm_file.set_cmd(SystemMessage::AutocryptSetupMessage);
        assert_eq!(
            get_summarytext_by_raw(Viewtype::File, no_text.as_ref(), &mut asm_file, 50, &ctx),
            "Autocrypt Setup Message" // file name is not added for autocrypt setup messages
        );
    }
}

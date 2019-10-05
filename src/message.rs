use std::path::{Path, PathBuf};
use std::ptr;

use deltachat_derive::{FromSql, ToSql};

use crate::chat::{self, Chat};
use crate::constants::*;
use crate::contact::*;
use crate::context::*;
use crate::dc_mimeparser::SystemMessage;
use crate::dc_tools::*;
use crate::error::Error;
use crate::events::Event;
use crate::job::*;
use crate::lot::{Lot, LotState, Meaning};
use crate::param::*;
use crate::pgp::*;
use crate::sql;
use crate::stock::StockMessage;

/// In practice, the user additionally cuts the string himself pixel-accurate.
const SUMMARY_CHARACTERS: usize = 160;

/// An object representing a single message in memory.
/// The message object is not updated.
/// If you want an update, you have to recreate the object.
///
/// to check if a mail was sent, use dc_msg_is_sent()
/// approx. max. length returned by dc_msg_get_text()
/// approx. max. length returned by dc_get_msg_info()
#[derive(Debug, Clone, Default)]
pub struct Message {
    pub(crate) id: u32,
    pub(crate) from_id: u32,
    pub(crate) to_id: u32,
    pub(crate) chat_id: u32,
    pub(crate) move_state: MoveState,
    pub(crate) type_0: Viewtype,
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
    // TODO: enum
    pub(crate) is_dc_message: u32,
    pub(crate) starred: bool,
    pub(crate) chat_blocked: Blocked,
    pub(crate) location_id: u32,
    pub(crate) param: Params,
}

impl Message {
    pub fn new(viewtype: Viewtype) -> Self {
        let mut msg = Message::default();
        msg.type_0 = viewtype;

        msg
    }

    pub fn load_from_db(context: &Context, id: u32) -> Result<Message, Error> {
        context.sql.query_row(
        "SELECT  \
         m.id,rfc724_mid,m.mime_in_reply_to,m.server_folder,m.server_uid,m.move_state,m.chat_id,  \
         m.from_id,m.to_id,m.timestamp,m.timestamp_sent,m.timestamp_rcvd, m.type,m.state,m.msgrmsg,m.txt,  \
         m.param,m.starred,m.hidden,m.location_id, c.blocked  \
         FROM msgs m \
         LEFT JOIN chats c ON c.id=m.chat_id WHERE m.id=?;",
        params![id as i32],
        |row| {
            let mut msg = Message::default();
            msg.id = row.get::<_, i32>(0)? as u32;
            msg.rfc724_mid = row.get::<_, String>(1)?;
            msg.in_reply_to = row.get::<_, Option<String>>(2)?;
            msg.server_folder = row.get::<_, Option<String>>(3)?;
            msg.server_uid = row.get(4)?;
            msg.move_state = row.get(5)?;
            msg.chat_id = row.get(6)?;
            msg.from_id = row.get(7)?;
            msg.to_id = row.get(8)?;
            msg.timestamp_sort = row.get(9)?;
            msg.timestamp_sent = row.get(10)?;
            msg.timestamp_rcvd = row.get(11)?;
            msg.type_0 = row.get(12)?;
            msg.state = row.get(13)?;
            msg.is_dc_message = row.get(14)?;

            let text;
            if let rusqlite::types::ValueRef::Text(buf) = row.get_raw(15) {
                if let Ok(t) = String::from_utf8(buf.to_vec()) {
                    text = t;
                } else {
                    warn!(context, "dc_msg_load_from_db: could not get text column as non-lossy utf8 id {}", id);
                    text = String::from_utf8_lossy(buf).into_owned();
                }
            } else {
                text = "".to_string();
            }
            msg.text = Some(text);

            msg.param = row.get::<_, String>(16)?.parse().unwrap_or_default();
            msg.starred = row.get(17)?;
            msg.hidden = row.get(18)?;
            msg.location_id = row.get(19)?;
            msg.chat_blocked = row.get::<_, Option<Blocked>>(20)?.unwrap_or_default();
            Ok(msg)
        })
    }

    pub fn delete_from_db(context: &Context, msg_id: u32) {
        if let Ok(msg) = Message::load_from_db(context, msg_id) {
            sql::execute(
                context,
                &context.sql,
                "DELETE FROM msgs WHERE id=?;",
                params![msg.id as i32],
            )
            .ok();
            sql::execute(
                context,
                &context.sql,
                "DELETE FROM msgs_mdns WHERE msg_id=?;",
                params![msg.id as i32],
            )
            .ok();
        }
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
        self.param
            .get(Param::File)
            .map(|f| dc_get_abs_path(context, f))
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

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_from_id(&self) -> u32 {
        self.from_id
    }

    pub fn get_chat_id(&self) -> u32 {
        if self.chat_blocked != Blocked::Not {
            1
        } else {
            self.chat_id
        }
    }

    pub fn get_viewtype(&self) -> Viewtype {
        self.type_0
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
            .map(|text| dc_truncate(text, 30000, false).to_string())
    }

    pub fn get_filename(&self) -> Option<String> {
        self.param
            .get(Param::File)
            .and_then(|file| Path::new(file).file_name())
            .map(|name| name.to_string_lossy().to_string())
    }

    pub fn get_filebytes(&self, context: &Context) -> u64 {
        self.param
            .get(Param::File)
            .map(|file| dc_get_filebytes(context, &file))
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
        self.param.get_int(Param::GuranteeE2ee).unwrap_or_default() != 0
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

        let contact = if self.from_id != DC_CONTACT_ID_SELF as libc::c_uint
            && ((*chat).typ == Chattype::Group || (*chat).typ == Chattype::VerifiedGroup)
        {
            Contact::get_by_id(context, self.from_id).ok()
        } else {
            None
        };

        ret.fill(self, chat, contact.as_ref(), context);

        ret
    }

    pub fn get_summarytext(&mut self, context: &Context, approx_characters: usize) -> String {
        get_summarytext_by_raw(
            self.type_0,
            self.text.as_ref(),
            &mut self.param,
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
        self.from_id == DC_CONTACT_ID_DEVICE as libc::c_uint
            || self.to_id == DC_CONTACT_ID_DEVICE as libc::c_uint
            || cmd != SystemMessage::Unknown && cmd != SystemMessage::AutocryptSetupMessage
    }

    pub fn is_increation(&self) -> bool {
        chat::msgtype_has_file(self.type_0) && self.state == MessageState::OutPreparing
    }

    pub fn is_setupmessage(&self) -> bool {
        if self.type_0 != Viewtype::File {
            return false;
        }

        self.param.get_cmd() == SystemMessage::AutocryptSetupMessage
    }

    pub fn get_setupcodebegin(&self, context: &Context) -> Option<String> {
        if !self.is_setupmessage() {
            return None;
        }

        if let Some(filename) = self.get_file(context) {
            if let Ok(mut buf) = dc_read_file(context, filename) {
                unsafe {
                    // just a pointer inside buf, MUST NOT be free()'d
                    let mut buf_headerline = String::default();
                    // just a pointer inside buf, MUST NOT be free()'d
                    let mut buf_setupcodebegin = ptr::null();

                    if dc_split_armored_data(
                        buf.as_mut_ptr().cast(),
                        &mut buf_headerline,
                        &mut buf_setupcodebegin,
                        ptr::null_mut(),
                        ptr::null_mut(),
                    ) && buf_headerline == "-----BEGIN PGP MESSAGE-----"
                        && !buf_setupcodebegin.is_null()
                    {
                        return Some(to_string_lossy(buf_setupcodebegin));
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
            params![self.param.to_string(), self.id as i32],
        )
        .is_ok()
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql)]
#[repr(i32)]
pub enum MessageState {
    Undefined = 0,
    InFresh = 10,
    InNoticed = 13,
    InSeen = 16,
    OutPreparing = 18,
    OutDraft = 19,
    OutPending = 20,
    OutFailed = 24,
    OutDelivered = 26,
    OutMdnRcvd = 28,
}

impl Default for MessageState {
    fn default() -> Self {
        MessageState::Undefined
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
                if chat.id == DC_CHAT_ID_DEADDROP {
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
            msg.type_0,
            msg.text.as_ref(),
            &mut msg.param,
            SUMMARY_CHARACTERS,
            context,
        ));

        self.timestamp = msg.get_timestamp();
        self.state = msg.state.into();
    }
}

pub fn get_msg_info(context: &Context, msg_id: u32) -> String {
    let mut ret = String::new();

    let msg = Message::load_from_db(context, msg_id);
    if msg.is_err() {
        return ret;
    }

    let msg = msg.unwrap_or_default();

    let rawtxt: Option<String> = context.sql.query_get_value(
        context,
        "SELECT txt_raw FROM msgs WHERE id=?;",
        params![msg_id as i32],
    );

    if rawtxt.is_none() {
        ret += &format!("Cannot load message #{}.", msg_id as usize);
        return ret;
    }
    let rawtxt = rawtxt.unwrap_or_default();
    let rawtxt = dc_truncate(rawtxt.trim(), 100000, false);

    let fts = dc_timestamp_to_str(msg.get_timestamp());
    ret += &format!("Sent: {}", fts);

    let name = Contact::load_from_db(context, msg.from_id)
        .map(|contact| contact.get_name_n_addr())
        .unwrap_or_default();

    ret += &format!(" by {}", name);
    ret += "\n";

    if msg.from_id != DC_CONTACT_ID_SELF as libc::c_uint {
        let s = dc_timestamp_to_str(if 0 != msg.timestamp_rcvd {
            msg.timestamp_rcvd
        } else {
            msg.timestamp_sort
        });
        ret += &format!("Received: {}", &s);
        ret += "\n";
    }

    if msg.from_id == 2 || msg.to_id == 2 {
        // device-internal message, no further details needed
        return ret;
    }

    if let Ok(rows) = context.sql.query_map(
        "SELECT contact_id, timestamp_sent FROM msgs_mdns WHERE msg_id=?;",
        params![msg_id as i32],
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

    ret += "State: ";
    use MessageState::*;
    match msg.state {
        InFresh => ret += "Fresh",
        InNoticed => ret += "Noticed",
        InSeen => ret += "Seen",
        OutDelivered => ret += "Delivered",
        OutFailed => ret += "Failed",
        OutMdnRcvd => ret += "Read",
        OutPending => ret += "Pending",
        OutPreparing => ret += "Preparing",
        _ => ret += &format!("{}", msg.state),
    }

    if msg.has_location() {
        ret += ", Location sent";
    }

    let e2ee_errors = msg.param.get_int(Param::ErroneousE2ee).unwrap_or_default();

    if 0 != e2ee_errors {
        if 0 != e2ee_errors & 0x2 {
            ret += ", Encrypted, no valid signature";
        }
    } else if 0 != msg.param.get_int(Param::GuranteeE2ee).unwrap_or_default() {
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

    if msg.type_0 != Viewtype::Text {
        ret += "Type: ";
        ret += &format!("{}", msg.type_0);
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

pub fn get_mime_headers(context: &Context, msg_id: u32) -> Option<String> {
    context.sql.query_get_value(
        context,
        "SELECT mime_headers FROM msgs WHERE id=?;",
        params![msg_id as i32],
    )
}

pub fn delete_msgs(context: &Context, msg_ids: &[u32]) {
    for msg_id in msg_ids.iter() {
        update_msg_chat_id(context, *msg_id, DC_CHAT_ID_TRASH);
        job_add(
            context,
            Action::DeleteMsgOnImap,
            *msg_id as libc::c_int,
            Params::new(),
            0,
        );
    }

    if !msg_ids.is_empty() {
        context.call_cb(Event::MsgsChanged {
            chat_id: 0,
            msg_id: 0,
        });
        job_kill_action(context, Action::Housekeeping);
        job_add(context, Action::Housekeeping, 0, Params::new(), 10);
    };
}

fn update_msg_chat_id(context: &Context, msg_id: u32, chat_id: u32) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET chat_id=? WHERE id=?;",
        params![chat_id as i32, msg_id as i32],
    )
    .is_ok()
}

pub fn markseen_msgs(context: &Context, msg_ids: &[u32]) -> bool {
    if msg_ids.is_empty() {
        return false;
    }

    let msgs = context.sql.prepare(
        "SELECT m.state, c.blocked  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  WHERE m.id=? AND m.chat_id>9",
        |mut stmt, _| {
            let mut res = Vec::with_capacity(msg_ids.len());
            for id in msg_ids.iter() {
                let query_res = stmt.query_row(params![*id as i32], |row| {
                    Ok((row.get::<_, MessageState>(0)?, row.get::<_, Option<Blocked>>(1)?.unwrap_or_default()))
                });
                if let Err(rusqlite::Error::QueryReturnedNoRows) = query_res {
                    continue;
                }
                let (state, blocked) = query_res?;
                res.push((id, state, blocked));
            }

            Ok(res)
        }
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
                info!(context, "Seen message #{}.", id);

                job_add(
                    context,
                    Action::MarkseenMsgOnImap,
                    *id as i32,
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
            chat_id: 0,
            msg_id: 0,
        });
    }

    true
}

pub fn update_msg_state(context: &Context, msg_id: u32, state: MessageState) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET state=? WHERE id=?;",
        params![state, msg_id as i32],
    )
    .is_ok()
}

pub fn star_msgs(context: &Context, msg_ids: &[u32], star: bool) -> bool {
    if msg_ids.is_empty() {
        return false;
    }
    context
        .sql
        .prepare("UPDATE msgs SET starred=? WHERE id=?;", |mut stmt, _| {
            for msg_id in msg_ids.iter() {
                stmt.execute(params![star as i32, *msg_id as i32])?;
            }
            Ok(())
        })
        .is_ok()
}

/// Returns a summary test.
pub fn get_summarytext_by_raw(
    viewtype: Viewtype,
    text: Option<impl AsRef<str>>,
    param: &mut Params,
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
                let file_name: String = if let Some(file_path) = param.get(Param::File) {
                    if let Some(file_name) = Path::new(file_path).file_name() {
                        Some(file_name.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| "ErrFileName".to_string());

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

    if let Some(text) = text {
        if prefix.is_empty() {
            dc_truncate(text.as_ref(), approx_characters, true).to_string()
        } else {
            let tmp = format!("{} – {}", prefix, text.as_ref());
            dc_truncate(&tmp, approx_characters, true).to_string()
        }
    } else {
        prefix
    }
}

// as we do not cut inside words, this results in about 32-42 characters.
// Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
// It should also be very clear, the subject is _not_ the whole message.
// The value is also used for CC:-summaries

// Context functions to work with messages

pub fn exists(context: &Context, msg_id: u32) -> bool {
    if msg_id <= DC_CHAT_ID_LAST_SPECIAL {
        return false;
    }

    let chat_id: Option<u32> = context.sql.query_get_value(
        context,
        "SELECT chat_id FROM msgs WHERE id=?;",
        params![msg_id],
    );

    if let Some(chat_id) = chat_id {
        chat_id != DC_CHAT_ID_TRASH
    } else {
        false
    }
}

pub fn update_msg_move_state(context: &Context, rfc724_mid: &str, state: MoveState) -> bool {
    // we update the move_state for all messages belonging to a given Message-ID
    // so that the state stay intact when parts are deleted
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET move_state=? WHERE rfc724_mid=?;",
        params![state as i32, rfc724_mid],
    )
    .is_ok()
}

pub fn set_msg_failed(context: &Context, msg_id: u32, error: Option<impl AsRef<str>>) {
    if let Ok(mut msg) = Message::load_from_db(context, msg_id) {
        if msg.state.can_fail() {
            msg.state = MessageState::OutFailed;
        }
        if let Some(error) = error {
            msg.param.set(Param::Error, error.as_ref());
            error!(context, "{}", error.as_ref());
        }

        if sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET state=?, param=? WHERE id=?;",
            params![msg.state, msg.param.to_string(), msg_id as i32],
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

/// returns true if an event should be send
pub fn mdn_from_ext(
    context: &Context,
    from_id: u32,
    rfc724_mid: &str,
    timestamp_sent: i64,
    ret_chat_id: &mut u32,
    ret_msg_id: &mut u32,
) -> bool {
    if from_id <= 9 || rfc724_mid.is_empty() || *ret_chat_id != 0 || *ret_msg_id != 0 {
        return false;
    }

    let mut read_by_all = false;

    if let Ok((msg_id, chat_id, chat_type, msg_state)) = context.sql.query_row(
        "SELECT m.id, c.id, c.type, m.state FROM msgs m  \
         LEFT JOIN chats c ON m.chat_id=c.id  \
         WHERE rfc724_mid=? AND from_id=1  \
         ORDER BY m.id;",
        params![rfc724_mid],
        |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, Chattype>(2)?,
                row.get::<_, MessageState>(3)?,
            ))
        },
    ) {
        *ret_msg_id = msg_id as u32;
        *ret_chat_id = chat_id as u32;

        // if already marked as MDNS_RCVD msgstate_can_fail() returns false.
        // however, it is important, that ret_msg_id is set above as this
        // will allow the caller eg. to move the message away
        if msg_state.can_fail() {
            let mdn_already_in_table = context
                .sql
                .exists(
                    "SELECT contact_id FROM msgs_mdns WHERE msg_id=? AND contact_id=?;",
                    params![*ret_msg_id as i32, from_id as i32,],
                )
                .unwrap_or_default();

            if !mdn_already_in_table {
                context.sql.execute(
                    "INSERT INTO msgs_mdns (msg_id, contact_id, timestamp_sent) VALUES (?, ?, ?);",
                    params![*ret_msg_id as i32, from_id as i32, timestamp_sent],
                ).unwrap_or_default(); // TODO: better error handling
            }

            // Normal chat? that's quite easy.
            if chat_type == Chattype::Single {
                update_msg_state(context, *ret_msg_id, MessageState::OutMdnRcvd);
                read_by_all = true;
            } else {
                // send event about new state
                let ist_cnt = context
                    .sql
                    .query_get_value::<_, isize>(
                        context,
                        "SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=?;",
                        params![*ret_msg_id as i32],
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
                let soll_cnt = (chat::get_chat_contact_cnt(context, *ret_chat_id) + 1) / 2;
                if ist_cnt >= soll_cnt {
                    update_msg_state(context, *ret_msg_id, MessageState::OutMdnRcvd);
                    read_by_all = true;
                } // else wait for more receipts
            }
        }
    }

    read_by_all
}

/// The number of messages assigned to real chat (!=deaddrop, !=trash)
pub fn get_real_msg_cnt(context: &Context) -> libc::c_int {
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

pub fn get_deaddrop_msg_cnt(context: &Context) -> libc::size_t {
    match context.sql.query_row(
        "SELECT COUNT(*) \
         FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE c.blocked=2;",
        rusqlite::NO_PARAMS,
        |row| row.get::<_, isize>(0),
    ) {
        Ok(res) => res as libc::size_t,
        Err(err) => {
            error!(context, "dc_get_deaddrop_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub fn rfc724_mid_cnt(context: &Context, rfc724_mid: &str) -> libc::c_int {
    // check the number of messages with the same rfc724_mid
    match context.sql.query_row(
        "SELECT COUNT(*) FROM msgs WHERE rfc724_mid=?;",
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
) -> Result<(String, u32, u32), Error> {
    ensure!(!rfc724_mid.is_empty(), "empty rfc724_mid");

    context.sql.query_row(
        "SELECT server_folder, server_uid, id FROM msgs WHERE rfc724_mid=?",
        &[rfc724_mid],
        |row| {
            let server_folder = row.get::<_, Option<String>>(0)?.unwrap_or_default();
            let server_uid = row.get(1)?;
            let msg_id = row.get(2)?;

            Ok((server_folder, server_uid, msg_id))
        },
    )
}

pub fn update_server_uid(
    context: &Context,
    rfc724_mid: &str,
    server_folder: impl AsRef<str>,
    server_uid: u32,
) {
    match context.sql.execute(
        "UPDATE msgs SET server_folder=?, server_uid=? WHERE rfc724_mid=?;",
        params![server_folder.as_ref(), server_uid, rfc724_mid],
    ) {
        Ok(_) => {}
        Err(err) => {
            warn!(context, "msg: failed to update server_uid: {}", err);
        }
    }
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
}

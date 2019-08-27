use std::ffi::CString;
use std::path::Path;
use std::ptr;

use deltachat_derive::{FromSql, ToSql};
use phf::phf_map;

use crate::chat::{self, Chat};
use crate::constants::*;
use crate::contact::*;
use crate::context::*;
use crate::dc_tools::*;
use crate::error::Error;
use crate::job::*;
use crate::lot::{Lot, LotState, Meaning};
use crate::param::*;
use crate::pgp::*;
use crate::sql;
use crate::stock::StockMessage;
use crate::types::*;
use crate::x::*;

/// In practice, the user additionally cuts the string himself pixel-accurate.
const SUMMARY_CHARACTERS: usize = 160;

#[repr(i32)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql)]
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
        } else if msg.from_id == DC_CONTACT_ID_SELF as u32 {
            if 0 != dc_msg_is_info(msg) || chat.is_self_talk() {
                self.text1 = None;
                self.text1_meaning = Meaning::None;
            } else {
                self.text1 = Some(context.stock_str(StockMessage::SelfMsg).to_owned().into());
                self.text1_meaning = Meaning::Text1Self;
            }
        } else if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
            if 0 != dc_msg_is_info(msg) || contact.is_none() {
                self.text1 = None;
                self.text1_meaning = Meaning::None;
            } else {
                if chat.id == DC_CHAT_ID_DEADDROP as u32 {
                    if let Some(contact) = contact {
                        self.text1 = Some(contact.get_display_name().into());
                    } else {
                        self.text1 = None;
                    }
                } else {
                    if let Some(contact) = contact {
                        self.text1 = Some(contact.get_first_name().into());
                    } else {
                        self.text1 = None;
                    }
                }
                self.text1_meaning = Meaning::Text1Username;
            }
        }

        self.text2 = Some(dc_msg_get_summarytext_by_raw(
            msg.type_0,
            msg.text.as_ref(),
            &mut msg.param,
            SUMMARY_CHARACTERS,
            context,
        ));

        self.timestamp = dc_msg_get_timestamp(msg);
        self.state = msg.state.into();
    }
}

/// An object representing a single message in memory.
/// The message object is not updated.
/// If you want an update, you have to recreate the object.
///
/// to check if a mail was sent, use dc_msg_is_sent()
/// approx. max. length returned by dc_msg_get_text()
/// approx. max. length returned by dc_get_msg_info()
#[derive(Clone)]
pub struct Message<'a> {
    pub id: u32,
    pub from_id: u32,
    pub to_id: u32,
    pub chat_id: u32,
    pub move_state: MoveState,
    pub type_0: Viewtype,
    pub state: MessageState,
    pub hidden: bool,
    pub timestamp_sort: i64,
    pub timestamp_sent: i64,
    pub timestamp_rcvd: i64,
    pub text: Option<String>,
    pub context: &'a Context,
    pub rfc724_mid: *mut libc::c_char,
    pub in_reply_to: *mut libc::c_char,
    pub server_folder: Option<String>,
    pub server_uid: u32,
    // TODO: enum
    pub is_dc_message: u32,
    pub starred: bool,
    pub chat_blocked: Blocked,
    pub location_id: u32,
    pub param: Params,
}

// handle messages
pub unsafe fn dc_get_msg_info(context: &Context, msg_id: u32) -> *mut libc::c_char {
    let mut p: *mut libc::c_char;
    let mut ret = String::new();

    let msg = dc_msg_load_from_db(context, msg_id);
    if msg.is_err() {
        return ptr::null_mut();
    }

    let msg = msg.unwrap();

    let rawtxt: Option<String> = context.sql.query_row_col(
        context,
        "SELECT txt_raw FROM msgs WHERE id=?;",
        params![msg_id as i32],
        0,
    );

    if rawtxt.is_none() {
        ret += &format!("Cannot load message #{}.", msg_id as usize);
        return ret.strdup();
    }
    let rawtxt = rawtxt.unwrap();
    let rawtxt = dc_truncate(rawtxt.trim(), 100000, false);

    let fts = dc_timestamp_to_str(dc_msg_get_timestamp(&msg));
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
        return ret.strdup();
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

    if dc_msg_has_location(&msg) {
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
    match msg.param.get(Param::Error) {
        Some(err) => ret += &format!("Error: {}", err),
        _ => {}
    }

    p = dc_msg_get_file(&msg);
    if !p.is_null() && 0 != *p.offset(0isize) as libc::c_int {
        ret += &format!(
            "\nFile: {}, {}, bytes\n",
            as_str(p),
            dc_get_filebytes(context, as_path(p)) as libc::c_int,
        );
    }
    free(p as *mut libc::c_void);

    if msg.type_0 != Viewtype::Text {
        ret += "Type: ";
        ret += &format!("{}", msg.type_0);
        ret += "\n";
        p = dc_msg_get_filemime(&msg);
        ret += &format!("Mimetype: {}\n", as_str(p));
        free(p as *mut libc::c_void);
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
    if !msg.rfc724_mid.is_null() && 0 != *msg.rfc724_mid.offset(0) as libc::c_int {
        ret += &format!("\nMessage-ID: {}", as_str(msg.rfc724_mid));
    }
    if let Some(ref server_folder) = msg.server_folder {
        if server_folder != "" {
            ret += &format!("\nLast seen as: {}/{}", server_folder, msg.server_uid);
        }
    }

    ret.strdup()
}

pub unsafe fn dc_msg_new_untyped<'a>(context: &'a Context) -> Message<'a> {
    dc_msg_new(context, Viewtype::Unknown)
}

pub fn dc_msg_new<'a>(context: &'a Context, viewtype: Viewtype) -> Message<'a> {
    Message {
        id: 0,
        from_id: 0,
        to_id: 0,
        chat_id: 0,
        move_state: MoveState::Undefined,
        type_0: viewtype,
        state: MessageState::Undefined,
        hidden: false,
        timestamp_sort: 0,
        timestamp_sent: 0,
        timestamp_rcvd: 0,
        text: None,
        context,
        rfc724_mid: std::ptr::null_mut(),
        in_reply_to: std::ptr::null_mut(),
        server_folder: None,
        server_uid: 0,
        is_dc_message: 0,
        starred: false,
        chat_blocked: Blocked::Not,
        location_id: 0,
        param: Params::new(),
    }
}

impl<'a> Drop for Message<'a> {
    fn drop(&mut self) {
        unsafe {
            free(self.rfc724_mid.cast());
            free(self.in_reply_to.cast());
        }
    }
}

pub unsafe fn dc_msg_get_filemime(msg: &Message) -> *mut libc::c_char {
    if let Some(m) = msg.param.get(Param::MimeType) {
        return m.strdup();
    } else if let Some(file) = msg.param.get(Param::File) {
        if let Some((_, mime)) = dc_msg_guess_msgtype_from_suffix(Path::new(file)) {
            return mime.strdup();
        }
    }

    "application/octet-stream".strdup()
}

pub fn dc_msg_guess_msgtype_from_suffix(path: &Path) -> Option<(Viewtype, &str)> {
    static KNOWN: phf::Map<&'static str, (Viewtype, &'static str)> = phf_map! {
        "mp3"   => (Viewtype::Audio, "audio/mpeg"),
        "aac"   => (Viewtype::Audio, "audio/aac"),
        "mp4"   => (Viewtype::Video, "video/mp4"),
        "jpg"   => (Viewtype::Image, "image/jpeg"),
        "jpeg"  => (Viewtype::Image, "image/jpeg"),
        "png"   => (Viewtype::Image, "image/png"),
        "webp"  => (Viewtype::Image, "image/webp"),
        "gif"   => (Viewtype::Gif,   "image/gif"),
        "vcf"   => (Viewtype::File,  "text/vcard"),
        "vcard" => (Viewtype::File,  "text/vcard"),
    };
    let extension: &str = &path.extension()?.to_str()?.to_lowercase();

    KNOWN.get(extension).map(|x| *x)
}

pub unsafe fn dc_msg_get_file(msg: &Message) -> *mut libc::c_char {
    let mut file_abs = 0 as *mut libc::c_char;

    if let Some(file_rel) = msg.param.get(Param::File) {
        file_abs = dc_get_abs_path(msg.context, file_rel);
    }
    if !file_abs.is_null() {
        file_abs
    } else {
        dc_strdup(0 as *const libc::c_char)
    }
}

/**
 * Check if a message has a location bound to it.
 * These messages are also returned by dc_get_locations()
 * and the UI may decide to display a special icon beside such messages,
 *
 * @memberof Message
 * @param msg The message object.
 * @return 1=Message has location bound to it, 0=No location bound to message.
 */
pub fn dc_msg_has_location(msg: &Message) -> bool {
    msg.location_id != 0
}

/**
 * Set any location that should be bound to the message object.
 * The function is useful to add a marker to the map
 * at a position different from the self-location.
 * You should not call this function
 * if you want to bind the current self-location to a message;
 * this is done by dc_set_location() and dc_send_locations_to_chat().
 *
 * Typically results in the event #DC_EVENT_LOCATION_CHANGED with
 * contact_id set to DC_CONTACT_ID_SELF.
 *
 * @memberof Message
 * @param msg The message object.
 * @param latitude North-south position of the location.
 * @param longitude East-west position of the location.
 * @return None.
 */
pub fn dc_msg_set_location(msg: &mut Message, latitude: libc::c_double, longitude: libc::c_double) {
    if latitude == 0.0 && longitude == 0.0 {
        return;
    }

    msg.param.set_float(Param::SetLatitude, latitude);
    msg.param.set_float(Param::SetLongitude, longitude);
}

pub fn dc_msg_get_timestamp(msg: &Message) -> i64 {
    if 0 != msg.timestamp_sent {
        msg.timestamp_sent
    } else {
        msg.timestamp_sort
    }
}

pub fn dc_msg_load_from_db<'a>(context: &'a Context, id: u32) -> Result<Message<'a>, Error> {
    context.sql.query_row(
        "SELECT  \
         m.id,rfc724_mid,m.mime_in_reply_to,m.server_folder,m.server_uid,m.move_state,m.chat_id,  \
         m.from_id,m.to_id,m.timestamp,m.timestamp_sent,m.timestamp_rcvd, m.type,m.state,m.msgrmsg,m.txt,  \
         m.param,m.starred,m.hidden,m.location_id, c.blocked  \
         FROM msgs m \
         LEFT JOIN chats c ON c.id=m.chat_id WHERE m.id=?;",
        params![id as i32],
        |row| {
            unsafe {
                let mut msg = dc_msg_new_untyped(context);
                msg.context = context;
                msg.id = row.get::<_, i32>(0)? as u32;
                msg.rfc724_mid = row.get::<_, String>(1)?.strdup();
                msg.in_reply_to = match row.get::<_, Option<String>>(2)? {
                    Some(s) => s.strdup(),
                    None => std::ptr::null_mut(),
                };
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
                        warn!(context, 0, "dc_msg_load_from_db: could not get text column as non-lossy utf8 id {}", id);
                        text = String::from_utf8_lossy(buf).into_owned();
                    }
                } else {
                    warn!(context, 0, "dc_msg_load_from_db: could not get text column for id {}", id);
                    text = "[ Could not read from db ]".to_string();
                }
                msg.text = Some(text);

                msg.param = row.get::<_, String>(16)?.parse().unwrap_or_default();
                msg.starred = row.get(17)?;
                msg.hidden = row.get(18)?;
                msg.location_id = row.get(19)?;
                msg.chat_blocked = row.get::<_, Option<Blocked>>(20)?.unwrap_or_default();
                if msg.chat_blocked == Blocked::Deaddrop {
                    if let Some(ref text) = msg.text {
                        let ptr = text.strdup();

                        dc_truncate_n_unwrap_str(ptr, 256, 0);

                        msg.text = Some(to_string(ptr));
                        free(ptr.cast());
                    }
                };
                Ok(msg)
            }
        })
}

pub unsafe fn dc_get_mime_headers(context: &Context, msg_id: u32) -> *mut libc::c_char {
    let headers: Option<String> = context.sql.query_row_col(
        context,
        "SELECT mime_headers FROM msgs WHERE id=?;",
        params![msg_id as i32],
        0,
    );

    if let Some(headers) = headers {
        let h = CString::yolo(headers);
        dc_strdup_keep_null(h.as_ptr())
    } else {
        std::ptr::null_mut()
    }
}

pub unsafe fn dc_delete_msgs(context: &Context, msg_ids: *const u32, msg_cnt: libc::c_int) {
    if msg_ids.is_null() || msg_cnt <= 0i32 {
        return;
    }
    let mut i: libc::c_int = 0i32;
    while i < msg_cnt {
        dc_update_msg_chat_id(context, *msg_ids.offset(i as isize), 3i32 as u32);
        job_add(
            context,
            Action::DeleteMsgOnImap,
            *msg_ids.offset(i as isize) as libc::c_int,
            Params::new(),
            0,
        );
        i += 1
    }

    if 0 != msg_cnt {
        context.call_cb(Event::MSGS_CHANGED, 0 as uintptr_t, 0 as uintptr_t);
        job_kill_action(context, Action::Housekeeping);
        job_add(context, Action::Housekeeping, 0, Params::new(), 10);
    };
}

fn dc_update_msg_chat_id(context: &Context, msg_id: u32, chat_id: u32) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET chat_id=? WHERE id=?;",
        params![chat_id as i32, msg_id as i32],
    )
    .is_ok()
}

pub fn dc_markseen_msgs(context: &Context, msg_ids: *const u32, msg_cnt: usize) -> bool {
    if msg_ids.is_null() || msg_cnt <= 0 {
        return false;
    }
    let msgs = context.sql.prepare(
        "SELECT m.state, c.blocked  FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id  WHERE m.id=? AND m.chat_id>9",
        |mut stmt, _| {
            let mut res = Vec::with_capacity(msg_cnt);
            for i in 0..msg_cnt {
                let id = unsafe { *msg_ids.offset(i as isize) };
                let query_res = stmt.query_row(params![id as i32], |row| {
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
        warn!(context, 0, "markseen_msgs failed: {:?}", msgs);
        return false;
    }
    let mut send_event = false;
    let msgs = msgs.unwrap();

    for (id, curr_state, curr_blocked) in msgs.into_iter() {
        if curr_blocked == Blocked::Not {
            if curr_state == MessageState::InFresh || curr_state == MessageState::InNoticed {
                dc_update_msg_state(context, id, MessageState::InSeen);
                info!(context, 0, "Seen message #{}.", id);

                job_add(
                    context,
                    Action::MarkseenMsgOnImap,
                    id as i32,
                    Params::new(),
                    0,
                );
                send_event = true;
            }
        } else if curr_state == MessageState::InFresh {
            dc_update_msg_state(context, id, MessageState::InNoticed);
            send_event = true;
        }
    }

    if send_event {
        context.call_cb(Event::MSGS_CHANGED, 0, 0);
    }

    true
}

pub fn dc_update_msg_state(context: &Context, msg_id: u32, state: MessageState) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET state=? WHERE id=?;",
        params![state, msg_id as i32],
    )
    .is_ok()
}

pub fn dc_star_msgs(
    context: &Context,
    msg_ids: *const u32,
    msg_cnt: libc::c_int,
    star: libc::c_int,
) -> bool {
    if msg_ids.is_null() || msg_cnt <= 0 || star != 0 && star != 1 {
        return false;
    }
    context
        .sql
        .prepare("UPDATE msgs SET starred=? WHERE id=?;", |mut stmt, _| {
            for i in 0..msg_cnt {
                stmt.execute(params![star, unsafe { *msg_ids.offset(i as isize) as i32 }])?;
            }
            Ok(())
        })
        .is_ok()
}

pub fn dc_get_msg<'a>(context: &'a Context, msg_id: u32) -> Result<Message<'a>, Error> {
    dc_msg_load_from_db(context, msg_id)
}

pub fn dc_msg_get_id(msg: &Message) -> u32 {
    msg.id
}

pub fn dc_msg_get_from_id(msg: &Message) -> u32 {
    msg.from_id
}

pub fn dc_msg_get_chat_id(msg: &Message) -> u32 {
    if msg.chat_blocked != Blocked::Not {
        1
    } else {
        msg.chat_id
    }
}

pub fn dc_msg_get_viewtype(msg: &Message) -> Viewtype {
    msg.type_0
}

pub fn dc_msg_get_state(msg: &Message) -> MessageState {
    msg.state
}

pub fn dc_msg_get_received_timestamp(msg: &Message) -> i64 {
    msg.timestamp_rcvd
}

pub fn dc_msg_get_sort_timestamp(msg: &Message) -> i64 {
    msg.timestamp_sort
}

pub unsafe fn dc_msg_get_text(msg: &Message) -> *mut libc::c_char {
    if let Some(ref text) = msg.text {
        dc_truncate(text, 30000, false).strdup()
    } else {
        ptr::null_mut()
    }
}

#[allow(non_snake_case)]
pub unsafe fn dc_msg_get_filename(msg: &Message) -> *mut libc::c_char {
    let mut ret = 0 as *mut libc::c_char;

    if let Some(file) = msg.param.get(Param::File) {
        ret = dc_get_filename(file);
    }
    if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    }
}

pub unsafe fn dc_msg_get_filebytes(msg: &Message) -> uint64_t {
    if let Some(file) = msg.param.get(Param::File) {
        return dc_get_filebytes(msg.context, &file);
    }

    0
}

pub fn dc_msg_get_width(msg: &Message) -> libc::c_int {
    msg.param.get_int(Param::Width).unwrap_or_default()
}

pub fn dc_msg_get_height(msg: &Message) -> libc::c_int {
    msg.param.get_int(Param::Height).unwrap_or_default()
}

pub fn dc_msg_get_duration(msg: &Message) -> libc::c_int {
    msg.param.get_int(Param::Duration).unwrap_or_default()
}

// TODO should return bool /rtn
pub fn dc_msg_get_showpadlock(msg: &Message) -> libc::c_int {
    if msg.param.get_int(Param::GuranteeE2ee).unwrap_or_default() != 0 {
        return 1;
    }

    0
}

pub unsafe fn dc_msg_get_summary<'a>(msg: &mut Message<'a>, chat: Option<&Chat<'a>>) -> Lot {
    let mut ret = Lot::new();

    let chat_loaded: Chat;
    let chat = if let Some(chat) = chat {
        chat
    } else {
        if let Ok(chat) = Chat::load_from_db(msg.context, msg.chat_id) {
            chat_loaded = chat;
            &chat_loaded
        } else {
            return ret;
        }
    };

    let contact = if msg.from_id != DC_CONTACT_ID_SELF as libc::c_uint
        && ((*chat).typ == Chattype::Group || (*chat).typ == Chattype::VerifiedGroup)
    {
        Contact::get_by_id((*chat).context, msg.from_id).ok()
    } else {
        None
    };

    ret.fill(msg, chat, contact.as_ref(), msg.context);

    ret
}

pub unsafe fn dc_msg_get_summarytext(
    msg: &mut Message,
    approx_characters: usize,
) -> *mut libc::c_char {
    dc_msg_get_summarytext_by_raw(
        msg.type_0,
        msg.text.as_ref(),
        &mut msg.param,
        approx_characters,
        msg.context,
    )
    .strdup()
}

/// Returns a summary test.
pub fn dc_msg_get_summarytext_by_raw(
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
        Viewtype::Video => context.stock_str(StockMessage::Video).into_owned(),
        Viewtype::Voice => context.stock_str(StockMessage::VoiceMessage).into_owned(),
        Viewtype::Audio | Viewtype::File => {
            if param.get_int(Param::Cmd) == Some(6) {
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
                .unwrap_or("ErrFileName".to_string());

                let label = context.stock_str(if viewtype == Viewtype::Audio {
                    StockMessage::Audio
                } else {
                    StockMessage::File
                });
                format!("{} – {}", label, file_name)
            }
        }
        _ => {
            if param.get_int(Param::Cmd) != Some(9) {
                "".to_string()
            } else {
                append_text = false;
                context.stock_str(StockMessage::Location).to_string()
            }
        }
    };
    let ret = if append_text && text.is_some() {
        let text = text.unwrap();
        if !prefix.is_empty() {
            let tmp = format!("{} – {}", prefix, text.as_ref());
            dc_truncate(&tmp, approx_characters, true).to_string()
        } else {
            dc_truncate(text.as_ref(), approx_characters, true).to_string()
        }
    } else {
        prefix
    };

    ret
}

pub unsafe fn dc_msg_has_deviating_timestamp(msg: &Message) -> libc::c_int {
    let cnv_to_local = dc_gm2local_offset();
    let sort_timestamp = dc_msg_get_sort_timestamp(msg) as i64 + cnv_to_local;
    let send_timestamp = dc_msg_get_timestamp(msg) as i64 + cnv_to_local;

    (sort_timestamp / 86400 != send_timestamp / 86400) as libc::c_int
}

// TODO should return bool /rtn
pub fn dc_msg_is_sent(msg: &Message) -> libc::c_int {
    if msg.state as i32 >= MessageState::OutDelivered as i32 {
        1
    } else {
        0
    }
}

pub fn dc_msg_is_starred(msg: &Message) -> bool {
    msg.starred
}

// TODO should return bool /rtn
pub fn dc_msg_is_forwarded(msg: &Message) -> libc::c_int {
    if 0 != msg.param.get_int(Param::Forwarded).unwrap_or_default() {
        1
    } else {
        0
    }
}

// TODO should return bool /rtn
pub fn dc_msg_is_info(msg: &Message) -> libc::c_int {
    let cmd = msg.param.get_int(Param::Cmd).unwrap_or_default();
    if msg.from_id == 2i32 as libc::c_uint
        || msg.to_id == 2i32 as libc::c_uint
        || 0 != cmd && cmd != 6i32
    {
        return 1;
    }

    0
}

// TODO should return bool /rtn
pub fn dc_msg_is_increation(msg: &Message) -> libc::c_int {
    if chat::msgtype_has_file(msg.type_0) && msg.state == MessageState::OutPreparing {
        1
    } else {
        0
    }
}

pub fn dc_msg_is_setupmessage(msg: &Message) -> bool {
    if msg.type_0 != Viewtype::File {
        return false;
    }

    msg.param.get_int(Param::Cmd) == Some(6)
}

pub unsafe fn dc_msg_get_setupcodebegin(msg: &Message) -> *mut libc::c_char {
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf_bytes: size_t = 0i32 as size_t;
    // just a pointer inside buf, MUST NOT be free()'d
    let mut buf_headerline: *const libc::c_char = 0 as *const libc::c_char;
    // just a pointer inside buf, MUST NOT be free()'d
    let mut buf_setupcodebegin: *const libc::c_char = 0 as *const libc::c_char;
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if dc_msg_is_setupmessage(msg) {
        filename = dc_msg_get_file(msg);
        if !(filename.is_null() || *filename.offset(0isize) as libc::c_int == 0i32) {
            if !(0
                == dc_read_file(
                    msg.context,
                    filename,
                    &mut buf as *mut *mut libc::c_char as *mut *mut libc::c_void,
                    &mut buf_bytes,
                )
                || buf.is_null()
                || buf_bytes <= 0)
            {
                if dc_split_armored_data(
                    buf,
                    &mut buf_headerline,
                    &mut buf_setupcodebegin,
                    0 as *mut *const libc::c_char,
                    0 as *mut *const libc::c_char,
                ) && strcmp(
                    buf_headerline,
                    b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
                ) == 0
                    && !buf_setupcodebegin.is_null()
                {
                    ret = dc_strdup(buf_setupcodebegin)
                }
            }
        }
    }
    free(filename as *mut libc::c_void);
    free(buf as *mut libc::c_void);
    if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    }
}

pub fn dc_msg_set_text(msg: &mut Message, text: *const libc::c_char) {
    msg.text = if text.is_null() {
        None
    } else {
        Some(to_string(text))
    };
}

pub fn dc_msg_set_file(
    msg: &mut Message,
    file: *const libc::c_char,
    filemime: *const libc::c_char,
) {
    if !file.is_null() {
        msg.param.set(Param::File, as_str(file));
    }
    if !filemime.is_null() {
        msg.param.set(Param::MimeType, as_str(filemime));
    }
}

pub fn dc_msg_set_dimension(msg: &mut Message, width: libc::c_int, height: libc::c_int) {
    msg.param.set_int(Param::Width, width);
    msg.param.set_int(Param::Height, height);
}

pub fn dc_msg_set_duration(msg: &mut Message, duration: libc::c_int) {
    msg.param.set_int(Param::Duration, duration);
}

pub fn dc_msg_latefiling_mediasize(
    msg: &mut Message,
    width: libc::c_int,
    height: libc::c_int,
    duration: libc::c_int,
) {
    if width > 0 && height > 0 {
        msg.param.set_int(Param::Width, width);
        msg.param.set_int(Param::Height, height);
    }
    if duration > 0 {
        msg.param.set_int(Param::Duration, duration);
    }
    dc_msg_save_param_to_disk(msg);
}

pub fn dc_msg_save_param_to_disk(msg: &mut Message) -> bool {
    sql::execute(
        msg.context,
        &msg.context.sql,
        "UPDATE msgs SET param=? WHERE id=?;",
        params![msg.param.to_string(), msg.id as i32],
    )
    .is_ok()
}

pub fn dc_msg_new_load<'a>(context: &'a Context, msg_id: u32) -> Result<Message<'a>, Error> {
    dc_msg_load_from_db(context, msg_id)
}

pub fn dc_delete_msg_from_db(context: &Context, msg_id: u32) {
    if let Ok(msg) = dc_msg_load_from_db(context, msg_id) {
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

/* as we do not cut inside words, this results in about 32-42 characters.
Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
It should also be very clear, the subject is _not_ the whole message.
The value is also used for CC:-summaries */

// Context functions to work with messages

pub unsafe fn dc_msg_exists(context: &Context, msg_id: u32) -> libc::c_int {
    if msg_id <= 9 {
        return 0;
    }

    let chat_id: Option<i32> = context.sql.query_row_col(
        context,
        "SELECT chat_id FROM msgs WHERE id=?;",
        params![msg_id as i32],
        0,
    );

    if let Some(chat_id) = chat_id {
        if chat_id != 3 {
            return 1;
        }
    }

    0
}

pub fn dc_update_msg_move_state(
    context: &Context,
    rfc724_mid: *const libc::c_char,
    state: MoveState,
) -> bool {
    // we update the move_state for all messages belonging to a given Message-ID
    // so that the state stay intact when parts are deleted
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET move_state=? WHERE rfc724_mid=?;",
        params![state as i32, as_str(rfc724_mid)],
    )
    .is_ok()
}

pub fn dc_set_msg_failed(context: &Context, msg_id: u32, error: Option<impl AsRef<str>>) {
    if let Ok(mut msg) = dc_msg_load_from_db(context, msg_id) {
        if msg.state.can_fail() {
            msg.state = MessageState::OutFailed;
        }
        if let Some(error) = error {
            msg.param.set(Param::Error, error.as_ref());
            error!(context, 0, "{}", error.as_ref());
        }

        if sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET state=?, param=? WHERE id=?;",
            params![msg.state, msg.param.to_string(), msg_id as i32],
        )
        .is_ok()
        {
            context.call_cb(
                Event::MSG_FAILED,
                msg.chat_id as uintptr_t,
                msg_id as uintptr_t,
            );
        }
    }
}

/* returns 1 if an event should be send */
pub unsafe fn dc_mdn_from_ext(
    context: &Context,
    from_id: u32,
    rfc724_mid: *const libc::c_char,
    timestamp_sent: i64,
    ret_chat_id: *mut u32,
    ret_msg_id: *mut u32,
) -> libc::c_int {
    if from_id <= 9
        || rfc724_mid.is_null()
        || ret_chat_id.is_null()
        || ret_msg_id.is_null()
        || *ret_chat_id != 0
        || *ret_msg_id != 0
    {
        return 0;
    }

    let mut read_by_all = 0;

    if let Ok((msg_id, chat_id, chat_type, msg_state)) = context.sql.query_row(
        "SELECT m.id, c.id, c.type, m.state FROM msgs m  \
         LEFT JOIN chats c ON m.chat_id=c.id  \
         WHERE rfc724_mid=? AND from_id=1  \
         ORDER BY m.id;",
        params![as_str(rfc724_mid)],
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

        /* if already marked as MDNS_RCVD msgstate_can_fail() returns false.
        however, it is important, that ret_msg_id is set above as this
        will allow the caller eg. to move the message away */
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
                ).unwrap(); // TODO: better error handling
            }

            // Normal chat? that's quite easy.
            if chat_type == Chattype::Single {
                dc_update_msg_state(context, *ret_msg_id, MessageState::OutMdnRcvd);
                read_by_all = 1;
            } else {
                /* send event about new state */
                let ist_cnt: i32 = context
                    .sql
                    .query_row_col(
                        context,
                        "SELECT COUNT(*) FROM msgs_mdns WHERE msg_id=?;",
                        params![*ret_msg_id as i32],
                        0,
                    )
                    .unwrap_or_default();
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
                    dc_update_msg_state(context, *ret_msg_id, MessageState::OutMdnRcvd);
                    read_by_all = 1;
                } /* else wait for more receipts */
            }
        }
    }

    read_by_all
}

/* the number of messages assigned to real chat (!=deaddrop, !=trash) */
pub fn dc_get_real_msg_cnt(context: &Context) -> libc::c_int {
    match context.sql.query_row(
        "SELECT COUNT(*) \
         FROM msgs m  LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE m.id>9 AND m.chat_id>9 AND c.blocked=0;",
        rusqlite::NO_PARAMS,
        |row| row.get(0),
    ) {
        Ok(res) => res,
        Err(err) => {
            error!(context, 0, "dc_get_real_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub fn dc_get_deaddrop_msg_cnt(context: &Context) -> size_t {
    match context.sql.query_row(
        "SELECT COUNT(*) \
         FROM msgs m LEFT JOIN chats c ON c.id=m.chat_id \
         WHERE c.blocked=2;",
        rusqlite::NO_PARAMS,
        |row| row.get::<_, isize>(0),
    ) {
        Ok(res) => res as size_t,
        Err(err) => {
            error!(context, 0, "dc_get_deaddrop_msg_cnt() failed. {}", err);
            0
        }
    }
}

pub fn dc_rfc724_mid_cnt(context: &Context, rfc724_mid: *const libc::c_char) -> libc::c_int {
    /* check the number of messages with the same rfc724_mid */
    match context.sql.query_row(
        "SELECT COUNT(*) FROM msgs WHERE rfc724_mid=?;",
        &[as_str(rfc724_mid)],
        |row| row.get(0),
    ) {
        Ok(res) => res,
        Err(err) => {
            error!(context, 0, "dc_get_rfc724_mid_cnt() failed. {}", err);
            0
        }
    }
}

pub fn dc_rfc724_mid_exists(
    context: &Context,
    rfc724_mid: *const libc::c_char,
    ret_server_folder: *mut *mut libc::c_char,
    ret_server_uid: *mut u32,
) -> u32 {
    if rfc724_mid.is_null() || unsafe { *rfc724_mid.offset(0) as libc::c_int } == 0 {
        return 0;
    }
    match context.sql.query_row(
        "SELECT server_folder, server_uid, id FROM msgs WHERE rfc724_mid=?",
        &[as_str(rfc724_mid)],
        |row| {
            if !ret_server_folder.is_null() {
                unsafe { *ret_server_folder = row.get::<_, String>(0)?.strdup() };
            }
            if !ret_server_uid.is_null() {
                unsafe { *ret_server_uid = row.get(1)? };
            }
            row.get(2)
        },
    ) {
        Ok(res) => res,
        Err(_err) => {
            if !ret_server_folder.is_null() {
                unsafe { *ret_server_folder = 0 as *mut libc::c_char };
            }
            if !ret_server_uid.is_null() {
                unsafe { *ret_server_uid = 0 };
            }

            0
        }
    }
}

pub fn dc_update_server_uid(
    context: &Context,
    rfc724_mid: *const libc::c_char,
    server_folder: impl AsRef<str>,
    server_uid: u32,
) {
    match context.sql.execute(
        "UPDATE msgs SET server_folder=?, server_uid=? WHERE rfc724_mid=?;",
        params![server_folder.as_ref(), server_uid, as_str(rfc724_mid)],
    ) {
        Ok(_) => {}
        Err(err) => {
            warn!(context, 0, "msg: failed to update server_uid: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils as test;

    #[test]
    fn test_dc_msg_guess_msgtype_from_suffix() {
        assert_eq!(
            dc_msg_guess_msgtype_from_suffix(Path::new("foo/bar-sth.mp3")),
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

        let mut msg = dc_msg_new(ctx, Viewtype::Text);

        let msg_id = chat::prepare_msg(ctx, chat, &mut msg).unwrap();

        let _msg2 = dc_get_msg(ctx, msg_id).unwrap();
    }
}

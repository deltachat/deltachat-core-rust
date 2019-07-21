use crate::constants::*;
use crate::context::*;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_job::*;
use crate::dc_lot::dc_lot_t;
use crate::dc_lot::*;
use crate::dc_param::*;
use crate::dc_stock::*;
use crate::dc_tools::*;
use crate::pgp::*;
use crate::sql;
use crate::types::*;
use crate::x::*;

/* * the structure behind dc_msg_t */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_msg_t<'a> {
    pub magic: uint32_t,
    pub id: uint32_t,
    pub from_id: uint32_t,
    pub to_id: uint32_t,
    pub chat_id: uint32_t,
    pub move_state: dc_move_state_t,
    pub type_0: libc::c_int,
    pub state: libc::c_int,
    pub hidden: libc::c_int,
    pub timestamp_sort: i64,
    pub timestamp_sent: i64,
    pub timestamp_rcvd: i64,
    pub text: *mut libc::c_char,
    pub context: &'a Context,
    pub rfc724_mid: *mut libc::c_char,
    pub in_reply_to: *mut libc::c_char,
    pub server_folder: *mut libc::c_char,
    pub server_uid: uint32_t,
    pub is_dc_message: libc::c_int,
    pub starred: libc::c_int,
    pub chat_blocked: libc::c_int,
    pub location_id: uint32_t,
    pub param: *mut dc_param_t,
}

// handle messages
pub unsafe fn dc_get_msg_info(context: &Context, msg_id: u32) -> *mut libc::c_char {
    let msg = dc_msg_new_untyped(context);
    let contact_from = dc_contact_new(context);
    let mut p: *mut libc::c_char;
    let mut ret = String::new();

    dc_msg_load_from_db(msg, context, msg_id);
    dc_contact_load_from_db(contact_from, &context.sql, (*msg).from_id);

    let rawtxt: Option<String> = context.sql.query_row_col(
        context,
        "SELECT txt_raw FROM msgs WHERE id=?;",
        params![msg_id as i32],
        0,
    );

    if rawtxt.is_none() {
        ret += &format!("Cannot load message #{}.", msg_id as usize);
        dc_msg_unref(msg);
        dc_contact_unref(contact_from);
        return to_cstring(ret);
    }
    let rawtxt = rawtxt.unwrap();
    let rawtxt = dc_truncate_str(rawtxt.trim(), 100000);

    let fts = dc_timestamp_to_str_safe(dc_msg_get_timestamp(msg));
    ret += &format!("Sent: {}", fts);

    p = dc_contact_get_name_n_addr(contact_from);
    ret += &format!(" by {}", to_string(p));
    free(p as *mut libc::c_void);
    ret += "\n";

    if (*msg).from_id != 1 as libc::c_uint {
        p = dc_timestamp_to_str(if 0 != (*msg).timestamp_rcvd {
            (*msg).timestamp_rcvd
        } else {
            (*msg).timestamp_sort
        });
        ret += &format!("Received: {}", as_str(p));
        free(p as *mut libc::c_void);
        ret += "\n";
    }

    if (*msg).from_id == 2 || (*msg).to_id == 2 {
        // device-internal message, no further details needed
        dc_msg_unref(msg);
        dc_contact_unref(contact_from);
        return to_cstring(ret);
    }

    context
        .sql
        .query_map(
            "SELECT contact_id, timestamp_sent FROM msgs_mdns WHERE msg_id=?;",
            params![msg_id as i32],
            |row| {
                let contact_id: i32 = row.get(0)?;
                let ts: i64 = row.get(1)?;
                Ok((contact_id, ts))
            },
            |rows| {
                for row in rows {
                    let (contact_id, ts) = row?;
                    let fts = dc_timestamp_to_str_safe(ts);
                    ret += &format!("Read: {}", fts);

                    let contact = dc_contact_new(context);
                    dc_contact_load_from_db(contact, &context.sql, contact_id as u32);

                    p = dc_contact_get_name_n_addr(contact);
                    ret += &format!(" by {}", as_str(p));
                    free(p as *mut libc::c_void);
                    dc_contact_unref(contact);

                    ret += "\n";
                }
                Ok(())
            },
        )
        .unwrap(); // TODO: better error handling

    ret += "State: ";
    match (*msg).state {
        10 => ret += "Fresh",
        13 => ret += "Noticed",
        16 => ret += "Seen",
        26 => ret += "Delivered",
        24 => ret += "Failed",
        28 => ret += "Read",
        20 => ret += "Pending",
        18 => ret += "Preparing",
        _ => ret += &format!("{}", (*msg).state),
    }

    if dc_msg_has_location(msg) {
        ret += ", Location sent";
    }

    let e2ee_errors = dc_param_get_int((*msg).param, DC_PARAM_ERRONEOUS_E2EE as i32, 0);

    if 0 != e2ee_errors {
        if 0 != e2ee_errors & 0x2 {
            ret += ", Encrypted, no valid signature";
        }
    } else if 0 != dc_param_get_int((*msg).param, DC_PARAM_GUARANTEE_E2EE as i32, 0) {
        ret += ", Encrypted";
    }

    ret += "\n";
    p = dc_param_get(
        (*msg).param,
        DC_PARAM_ERROR as i32,
        0 as *const libc::c_char,
    );
    if !p.is_null() {
        ret += &format!("Error: {}", as_str(p));
        free(p as *mut libc::c_void);
    }
    p = dc_msg_get_file(msg);
    if !p.is_null() && 0 != *p.offset(0isize) as libc::c_int {
        ret += &format!(
            "\nFile: {}, {}, bytes\n",
            as_str(p),
            dc_get_filebytes(context, p) as libc::c_int,
        );
    }
    free(p as *mut libc::c_void);

    if (*msg).type_0 != 10 {
        ret += "Type: ";
        match (*msg).type_0 {
            40 => ret += "Audio",
            60 => ret += "File",
            21 => ret += "GIF",
            20 => ret += "Image",
            50 => ret += "Video",
            41 => ret += "Voice",
            _ => ret += &format!("{}", (*msg).type_0),
        }
        ret += "\n";
        p = dc_msg_get_filemime(msg);
        ret += &format!("Mimetype: {}\n", as_str(p));
        free(p as *mut libc::c_void);
    }
    let w = dc_param_get_int((*msg).param, DC_PARAM_WIDTH as i32, 0);
    let h = dc_param_get_int((*msg).param, DC_PARAM_HEIGHT as i32, 0);
    if w != 0 || h != 0 {
        ret += &format!("Dimension: {} x {}\n", w, h,);
    }
    let duration = dc_param_get_int((*msg).param, DC_PARAM_DURATION as i32, 0);
    if duration != 0 {
        ret += &format!("Duration: {} ms\n", duration,);
    }
    if !rawtxt.is_empty() {
        ret += &format!("\n{}\n", rawtxt);
    }
    if !(*msg).rfc724_mid.is_null() && 0 != *(*msg).rfc724_mid.offset(0) as libc::c_int {
        ret += &format!("\nMessage-ID: {}", (*msg).rfc724_mid as libc::c_int);
    }
    if !(*msg).server_folder.is_null() && 0 != *(*msg).server_folder.offset(0) as libc::c_int {
        ret += &format!(
            "\nLast seen as: {}/{}",
            to_string((*msg).server_folder),
            (*msg).server_uid as libc::c_int,
        );
    }

    dc_msg_unref(msg);
    dc_contact_unref(contact_from);
    to_cstring(ret)
}

pub unsafe fn dc_msg_new_untyped<'a>(context: &'a Context) -> *mut dc_msg_t<'a> {
    dc_msg_new(context, 0i32)
}

/* *
 * @class dc_msg_t
 *
 * An object representing a single message in memory.
 * The message object is not updated.
 * If you want an update, you have to recreate the object.
 */
// to check if a mail was sent, use dc_msg_is_sent()
// approx. max. length returned by dc_msg_get_text()
// approx. max. length returned by dc_get_msg_info()
pub unsafe fn dc_msg_new<'a>(context: &'a Context, viewtype: libc::c_int) -> *mut dc_msg_t<'a> {
    let mut msg: *mut dc_msg_t;
    msg = calloc(1, ::std::mem::size_of::<dc_msg_t>()) as *mut dc_msg_t;
    assert!(!msg.is_null());
    (*msg).context = context;
    (*msg).magic = 0x11561156i32 as uint32_t;
    (*msg).type_0 = viewtype;
    (*msg).state = 0i32;
    (*msg).param = dc_param_new();

    msg
}

pub unsafe fn dc_msg_unref(mut msg: *mut dc_msg_t) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_msg_empty(msg);
    dc_param_unref((*msg).param);
    (*msg).magic = 0i32 as uint32_t;
    free(msg as *mut libc::c_void);
}

pub unsafe fn dc_msg_empty(mut msg: *mut dc_msg_t) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    free((*msg).text as *mut libc::c_void);
    (*msg).text = 0 as *mut libc::c_char;
    free((*msg).rfc724_mid as *mut libc::c_void);
    (*msg).rfc724_mid = 0 as *mut libc::c_char;
    free((*msg).in_reply_to as *mut libc::c_void);
    (*msg).in_reply_to = 0 as *mut libc::c_char;
    free((*msg).server_folder as *mut libc::c_void);
    (*msg).server_folder = 0 as *mut libc::c_char;
    dc_param_set_packed((*msg).param, 0 as *const libc::c_char);
    (*msg).hidden = 0i32;
}

pub unsafe fn dc_msg_get_filemime(msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut file: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        ret = dc_param_get(
            (*msg).param,
            DC_PARAM_MIMETYPE as i32,
            0 as *const libc::c_char,
        );
        if ret.is_null() {
            file = dc_param_get((*msg).param, DC_PARAM_FILE as i32, 0 as *const libc::c_char);
            if !file.is_null() {
                dc_msg_guess_msgtype_from_suffix(file, 0 as *mut libc::c_int, &mut ret);
                if ret.is_null() {
                    ret = dc_strdup(
                        b"application/octet-stream\x00" as *const u8 as *const libc::c_char,
                    )
                }
            }
        }
    }
    free(file as *mut libc::c_void);
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}

pub unsafe fn dc_msg_guess_msgtype_from_suffix(
    pathNfilename: *const libc::c_char,
    mut ret_msgtype: *mut libc::c_int,
    mut ret_mime: *mut *mut libc::c_char,
) {
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dummy_msgtype: libc::c_int = 0i32;
    let mut dummy_buf: *mut libc::c_char = 0 as *mut libc::c_char;
    if !pathNfilename.is_null() {
        if ret_msgtype.is_null() {
            ret_msgtype = &mut dummy_msgtype
        }
        if ret_mime.is_null() {
            ret_mime = &mut dummy_buf
        }
        *ret_msgtype = 0i32;
        *ret_mime = 0 as *mut libc::c_char;
        suffix = dc_get_filesuffix_lc(pathNfilename);
        if !suffix.is_null() {
            if strcmp(suffix, b"mp3\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = DC_MSG_AUDIO as libc::c_int;
                *ret_mime = dc_strdup(b"audio/mpeg\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"aac\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = DC_MSG_AUDIO as libc::c_int;
                *ret_mime = dc_strdup(b"audio/aac\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"mp4\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = DC_MSG_VIDEO as libc::c_int;
                *ret_mime = dc_strdup(b"video/mp4\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"jpg\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"jpeg\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                *ret_msgtype = DC_MSG_IMAGE as libc::c_int;
                *ret_mime = dc_strdup(b"image/jpeg\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"png\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = DC_MSG_IMAGE as libc::c_int;
                *ret_mime = dc_strdup(b"image/png\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"webp\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = DC_MSG_IMAGE as libc::c_int;
                *ret_mime = dc_strdup(b"image/webp\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"gif\x00" as *const u8 as *const libc::c_char) == 0i32 {
                *ret_msgtype = DC_MSG_GIF as libc::c_int;
                *ret_mime = dc_strdup(b"image/gif\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"vcf\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"vcard\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                *ret_msgtype = DC_MSG_FILE as libc::c_int;
                *ret_mime = dc_strdup(b"text/vcard\x00" as *const u8 as *const libc::c_char)
            }
        }
    }
    free(suffix as *mut libc::c_void);
    free(dummy_buf as *mut libc::c_void);
}

pub unsafe fn dc_msg_get_file(msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut file_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut file_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        file_rel = dc_param_get((*msg).param, DC_PARAM_FILE as i32, 0 as *const libc::c_char);
        if !file_rel.is_null() {
            file_abs = dc_get_abs_path((*msg).context, file_rel)
        }
    }
    free(file_rel as *mut libc::c_void);
    return if !file_abs.is_null() {
        file_abs
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}

/**
 * Check if a message has a location bound to it.
 * These messages are also returned by dc_get_locations()
 * and the UI may decide to display a special icon beside such messages,
 *
 * @memberof dc_msg_t
 * @param msg The message object.
 * @return 1=Message has location bound to it, 0=No location bound to message.
 */
pub unsafe fn dc_msg_has_location(msg: *const dc_msg_t) -> bool {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return false;
    }

    ((*msg).location_id != 0i32 as libc::c_uint)
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
 * @memberof dc_msg_t
 * @param msg The message object.
 * @param latitude North-south position of the location.
 * @param longitude East-west position of the location.
 * @return None.
 */
pub unsafe fn dc_msg_set_location(
    msg: *const dc_msg_t,
    latitude: libc::c_double,
    longitude: libc::c_double,
) {
    if msg.is_null()
        || (*msg).magic != 0x11561156i32 as libc::c_uint
        || (latitude == 0.0 && longitude == 0.0)
    {
        return;
    }

    dc_param_set_float((*msg).param, DC_PARAM_SET_LATITUDE as libc::c_int, latitude);
    dc_param_set_float(
        (*msg).param,
        DC_PARAM_SET_LONGITUDE as libc::c_int,
        longitude,
    );
}

pub unsafe fn dc_msg_get_timestamp(msg: *const dc_msg_t) -> i64 {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0;
    }
    return if 0 != (*msg).timestamp_sent {
        (*msg).timestamp_sent
    } else {
        (*msg).timestamp_sort
    };
}

pub fn dc_msg_load_from_db<'a>(msg: *mut dc_msg_t<'a>, context: &'a Context, id: u32) -> bool {
    if msg.is_null() {
        return false;
    }

    let res = context.sql.query_row(
        "SELECT  \
         m.id,rfc724_mid,m.mime_in_reply_to,m.server_folder,m.server_uid,m.move_state,m.chat_id,  \
         m.from_id,m.to_id,m.timestamp,m.timestamp_sent,m.timestamp_rcvd, m.type,m.state,m.msgrmsg,m.txt,  \
         m.param,m.starred,m.hidden,m.location_id, c.blocked  \
         FROM msgs m \
         LEFT JOIN chats c ON c.id=m.chat_id WHERE m.id=?;",
        params![id as i32],
        |row| {
            unsafe {
                (*msg).context = context;
                dc_msg_empty(msg);

                (*msg).id = row.get::<_, i32>(0)? as u32;
                (*msg).rfc724_mid = to_cstring(row.get::<_, String>(1)?);
                (*msg).in_reply_to = match row.get::<_, Option<String>>(2)? {
                    Some(s) => to_cstring(s),
                    None => std::ptr::null_mut(),
                };
                (*msg).server_folder = to_cstring(row.get::<_, String>(3)?);
                (*msg).server_uid = row.get(4)?;
                (*msg).move_state = row.get(5)?;
                (*msg).chat_id = row.get(6)?;
                (*msg).from_id = row.get(7)?;
                (*msg).to_id = row.get(8)?;
                (*msg).timestamp_sort = row.get(9)?;
                (*msg).timestamp_sent = row.get(10)?;
                (*msg).timestamp_rcvd = row.get(11)?;
                (*msg).type_0 = row.get(12)?;
                (*msg).state = row.get(13)?;
                (*msg).is_dc_message = row.get(14)?;
                (*msg).text = to_cstring(row.get::<_, String>(15).unwrap_or_default());
                dc_param_set_packed(
                    (*msg).param,
                    to_cstring(row.get::<_, String>(16)?)
                );
                (*msg).starred = row.get(17)?;
                (*msg).hidden = row.get(18)?;
                (*msg).location_id = row.get(19)?;
                (*msg).chat_blocked = row.get::<_, Option<i32>>(20)?.unwrap_or_default();
                if (*msg).chat_blocked == 2 {
                    dc_truncate_n_unwrap_str((*msg).text, 256, 0);
                }
            }
            Ok(())
        }
    );

    match res {
        Ok(_) => true,
        Err(err) => false,
    }
}

pub unsafe fn dc_get_mime_headers(context: &Context, msg_id: uint32_t) -> *mut libc::c_char {
    let headers: Option<String> = context.sql.query_row_col(
        context,
        "SELECT mime_headers FROM msgs WHERE id=?;",
        params![msg_id as i32],
        0,
    );

    if let Some(headers) = headers {
        let h = to_cstring(headers);
        let res = dc_strdup_keep_null(h);
        free(h as *mut _);
        res
    } else {
        std::ptr::null_mut()
    }
}

pub unsafe fn dc_delete_msgs(context: &Context, msg_ids: *const uint32_t, msg_cnt: libc::c_int) {
    if msg_ids.is_null() || msg_cnt <= 0i32 {
        return;
    }
    let mut i: libc::c_int = 0i32;
    while i < msg_cnt {
        dc_update_msg_chat_id(context, *msg_ids.offset(i as isize), 3i32 as uint32_t);
        dc_job_add(
            context,
            110i32,
            *msg_ids.offset(i as isize) as libc::c_int,
            0 as *const libc::c_char,
            0i32,
        );
        i += 1
    }

    if 0 != msg_cnt {
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
        dc_job_kill_action(context, 105i32);
        dc_job_add(context, 105i32, 0i32, 0 as *const libc::c_char, 10i32);
    };
}

pub fn dc_update_msg_chat_id(context: &Context, msg_id: u32, chat_id: u32) -> bool {
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
                let (state, blocked) = stmt.query_row(params![id as i32], |row| {
                    Ok((row.get::<_, i32>(0)?, row.get::<_, Option<i32>>(1)?.unwrap_or_default()))
                })?;
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
        if curr_blocked == 0 {
            if curr_state == 10 || curr_state == 13 {
                dc_update_msg_state(context, id, 16);
                info!(context, 0, "Seen message #{}.", id);

                unsafe { dc_job_add(context, 130, id as i32, 0 as *const libc::c_char, 0) };
                send_event = true;
            }
        } else if curr_state == 10 {
            dc_update_msg_state(context, id, 13);
            send_event = true;
        }
    }

    if send_event {
        context.call_cb(Event::MSGS_CHANGED, 0, 0);
    }

    true
}

pub fn dc_update_msg_state(context: &Context, msg_id: uint32_t, state: libc::c_int) -> bool {
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

pub unsafe fn dc_get_msg<'a>(context: &'a Context, msg_id: uint32_t) -> *mut dc_msg_t<'a> {
    let mut success: libc::c_int = 0i32;
    let obj: *mut dc_msg_t = dc_msg_new_untyped(context);
    if dc_msg_load_from_db(obj, context, msg_id) {
        success = 1i32
    }
    if 0 != success {
        obj
    } else {
        dc_msg_unref(obj);
        0 as *mut dc_msg_t
    }
}

pub unsafe fn dc_msg_get_id(msg: *const dc_msg_t) -> uint32_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }

    (*msg).id
}

pub unsafe fn dc_msg_get_from_id(msg: *const dc_msg_t) -> uint32_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }

    (*msg).from_id
}

pub unsafe fn dc_msg_get_chat_id(msg: *const dc_msg_t) -> uint32_t {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    return if 0 != (*msg).chat_blocked {
        1i32 as libc::c_uint
    } else {
        (*msg).chat_id
    };
}

pub unsafe fn dc_msg_get_viewtype(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }

    (*msg).type_0
}

pub unsafe fn dc_msg_get_state(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }

    (*msg).state
}

pub unsafe fn dc_msg_get_received_timestamp(msg: *const dc_msg_t) -> i64 {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0;
    }

    (*msg).timestamp_rcvd
}

pub unsafe fn dc_msg_get_sort_timestamp(msg: *const dc_msg_t) -> i64 {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0;
    }

    (*msg).timestamp_sort
}

pub unsafe fn dc_msg_get_text(msg: *const dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }

    let res = dc_truncate_str(as_str((*msg).text), 30000);
    to_cstring(res)
}

pub unsafe fn dc_msg_get_filename(msg: *const dc_msg_t) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        pathNfilename = dc_param_get((*msg).param, DC_PARAM_FILE as i32, 0 as *const libc::c_char);
        if !pathNfilename.is_null() {
            ret = dc_get_filename(pathNfilename)
        }
    }
    free(pathNfilename as *mut libc::c_void);
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}

pub unsafe fn dc_msg_get_filebytes(msg: *const dc_msg_t) -> uint64_t {
    let mut ret: uint64_t = 0i32 as uint64_t;
    let mut file: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        file = dc_param_get((*msg).param, DC_PARAM_FILE as i32, 0 as *const libc::c_char);
        if !file.is_null() {
            ret = dc_get_filebytes((*msg).context, file)
        }
    }
    free(file as *mut libc::c_void);

    ret
}

pub unsafe fn dc_msg_get_width(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }

    dc_param_get_int((*msg).param, DC_PARAM_WIDTH as i32, 0)
}

pub unsafe fn dc_msg_get_height(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }

    dc_param_get_int((*msg).param, DC_PARAM_HEIGHT as i32, 0)
}

pub unsafe fn dc_msg_get_duration(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0;
    }

    dc_param_get_int((*msg).param, DC_PARAM_DURATION as i32, 0)
}

// TODO should return bool /rtn
pub unsafe fn dc_msg_get_showpadlock(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    if dc_param_get_int((*msg).param, DC_PARAM_GUARANTEE_E2EE as i32, 0) != 0i32 {
        return 1i32;
    }

    0
}

pub unsafe fn dc_msg_get_summary<'a>(
    msg: *const dc_msg_t<'a>,
    mut chat: *const Chat<'a>,
) -> *mut dc_lot_t {
    let current_block: u64;
    let ret: *mut dc_lot_t = dc_lot_new();
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    let mut chat_to_delete: *mut Chat = 0 as *mut Chat;
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        if chat.is_null() {
            chat_to_delete = dc_get_chat((*msg).context, (*msg).chat_id);
            if chat_to_delete.is_null() {
                current_block = 15204159476013091401;
            } else {
                chat = chat_to_delete;
                current_block = 7815301370352969686;
            }
        } else {
            current_block = 7815301370352969686;
        }
        match current_block {
            15204159476013091401 => {}
            _ => {
                if (*msg).from_id != 1i32 as libc::c_uint
                    && ((*chat).type_0 == 120i32 || (*chat).type_0 == 130i32)
                {
                    contact = dc_get_contact((*chat).context, (*msg).from_id)
                }
                dc_lot_fill(ret, msg, chat, contact, (*msg).context);
            }
        }
    }
    dc_contact_unref(contact);
    dc_chat_unref(chat_to_delete);

    ret
}

pub unsafe fn dc_msg_get_summarytext(
    msg: *const dc_msg_t,
    approx_characters: libc::c_int,
) -> *mut libc::c_char {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }

    dc_msg_get_summarytext_by_raw(
        (*msg).type_0,
        (*msg).text,
        (*msg).param,
        approx_characters,
        (*msg).context,
    )
}

/* the returned value must be free()'d */
pub unsafe fn dc_msg_get_summarytext_by_raw(
    type_0: libc::c_int,
    text: *const libc::c_char,
    param: *mut dc_param_t,
    approx_characters: libc::c_int,
    context: &Context,
) -> *mut libc::c_char {
    /* get a summary text, result must be free()'d, never returns NULL. */
    let mut ret;
    let mut prefix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut label: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut append_text: libc::c_int = 1i32;
    match type_0 {
        20 => prefix = dc_stock_str(context, 9i32),
        21 => prefix = dc_stock_str(context, 23i32),
        50 => prefix = dc_stock_str(context, 10i32),
        41 => prefix = dc_stock_str(context, 7i32),
        40 | 60 => {
            if dc_param_get_int(param, DC_PARAM_CMD as i32, 0) == 6i32 {
                prefix = dc_stock_str(context, 42i32);
                append_text = 0i32
            } else {
                pathNfilename = dc_param_get(
                    param,
                    DC_PARAM_FILE as i32,
                    b"ErrFilename\x00" as *const u8 as *const libc::c_char,
                );
                value = dc_get_filename(pathNfilename);
                label = dc_stock_str(
                    context,
                    if type_0 == DC_MSG_AUDIO as libc::c_int {
                        11i32
                    } else {
                        12i32
                    },
                );
                prefix = dc_mprintf(
                    b"%s \xe2\x80\x93 %s\x00" as *const u8 as *const libc::c_char,
                    label,
                    value,
                )
            }
        }
        _ => {
            if dc_param_get_int(param, DC_PARAM_CMD as i32, 0) == 9i32 {
                prefix = dc_stock_str(context, 66i32);
                append_text = 0i32
            }
        }
    }
    if 0 != append_text
        && !prefix.is_null()
        && !text.is_null()
        && 0 != *text.offset(0isize) as libc::c_int
    {
        ret = dc_mprintf(
            b"%s \xe2\x80\x93 %s\x00" as *const u8 as *const libc::c_char,
            prefix,
            text,
        );
        dc_truncate_n_unwrap_str(ret, approx_characters, 1i32);
    } else if 0 != append_text && !text.is_null() && 0 != *text.offset(0isize) as libc::c_int {
        ret = dc_strdup(text);
        dc_truncate_n_unwrap_str(ret, approx_characters, 1i32);
    } else {
        ret = prefix;
        prefix = 0 as *mut libc::c_char
    }
    free(prefix as *mut libc::c_void);
    free(pathNfilename as *mut libc::c_void);
    free(label as *mut libc::c_void);
    free(value as *mut libc::c_void);
    if ret.is_null() {
        ret = dc_strdup(0 as *const libc::c_char)
    }

    ret
}

pub unsafe fn dc_msg_has_deviating_timestamp(msg: *const dc_msg_t) -> libc::c_int {
    let cnv_to_local = dc_gm2local_offset();
    let sort_timestamp = dc_msg_get_sort_timestamp(msg) as i64 + cnv_to_local;
    let send_timestamp = dc_msg_get_timestamp(msg) as i64 + cnv_to_local;

    (sort_timestamp / 86400 != send_timestamp / 86400) as libc::c_int
}

// TODO should return bool /rtn
pub unsafe fn dc_msg_is_sent(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return if (*msg).state >= 26i32 { 1i32 } else { 0i32 };
}

// TODO should return bool /rtn
pub unsafe fn dc_msg_is_starred(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return if 0 != (*msg).starred { 1i32 } else { 0i32 };
}

// TODO should return bool /rtn
pub unsafe fn dc_msg_is_forwarded(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    return if 0 != dc_param_get_int((*msg).param, DC_PARAM_FORWARDED as i32, 0) {
        1i32
    } else {
        0i32
    };
}

// TODO should return bool /rtn
pub unsafe fn dc_msg_is_info(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }
    let cmd: libc::c_int = dc_param_get_int((*msg).param, DC_PARAM_CMD as i32, 0);
    if (*msg).from_id == 2i32 as libc::c_uint
        || (*msg).to_id == 2i32 as libc::c_uint
        || 0 != cmd && cmd != 6i32
    {
        return 1i32;
    }

    0
}

// TODO should return bool /rtn
pub unsafe fn dc_msg_is_increation(msg: *const dc_msg_t) -> libc::c_int {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return 0i32;
    }

    (((*msg).type_0 == DC_MSG_IMAGE as libc::c_int
        || (*msg).type_0 == DC_MSG_GIF as libc::c_int
        || (*msg).type_0 == DC_MSG_AUDIO as libc::c_int
        || (*msg).type_0 == DC_MSG_VOICE as libc::c_int
        || (*msg).type_0 == DC_MSG_VIDEO as libc::c_int
        || (*msg).type_0 == DC_MSG_FILE as libc::c_int)
        && (*msg).state == 18i32) as libc::c_int
}

pub unsafe fn dc_msg_is_setupmessage(msg: *const dc_msg_t) -> bool {
    if msg.is_null()
        || (*msg).magic != 0x11561156i32 as libc::c_uint
        || (*msg).type_0 != DC_MSG_FILE as libc::c_int
    {
        return false;
    }

    if dc_param_get_int((*msg).param, DC_PARAM_CMD as i32, 0) == 6 {
        true
    } else {
        true
    }
}

pub unsafe fn dc_msg_get_setupcodebegin(msg: *const dc_msg_t) -> *mut libc::c_char {
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
                    (*msg).context,
                    filename,
                    &mut buf as *mut *mut libc::c_char as *mut *mut libc::c_void,
                    &mut buf_bytes,
                )
                || buf.is_null()
                || buf_bytes <= 0)
            {
                if !(0
                    == dc_split_armored_data(
                        buf,
                        &mut buf_headerline,
                        &mut buf_setupcodebegin,
                        0 as *mut *const libc::c_char,
                        0 as *mut *const libc::c_char,
                    )
                    || strcmp(
                        buf_headerline,
                        b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
                    ) != 0i32
                    || buf_setupcodebegin.is_null())
                {
                    ret = dc_strdup(buf_setupcodebegin)
                }
            }
        }
    }
    free(filename as *mut libc::c_void);
    free(buf as *mut libc::c_void);
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}

pub unsafe fn dc_msg_set_text(mut msg: *mut dc_msg_t, text: *const libc::c_char) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    free((*msg).text as *mut libc::c_void);
    (*msg).text = dc_strdup(text);
}

// TODO should return bool /rtn
pub unsafe fn dc_msg_set_file(
    msg: *mut dc_msg_t,
    file: *const libc::c_char,
    filemime: *const libc::c_char,
) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_param_set((*msg).param, DC_PARAM_FILE as i32, file);
    dc_param_set((*msg).param, DC_PARAM_MIMETYPE as i32, filemime);
}

pub unsafe fn dc_msg_set_dimension(msg: *mut dc_msg_t, width: libc::c_int, height: libc::c_int) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_param_set_int((*msg).param, DC_PARAM_WIDTH as i32, width);
    dc_param_set_int((*msg).param, DC_PARAM_HEIGHT as i32, height);
}

pub unsafe fn dc_msg_set_duration(msg: *mut dc_msg_t, duration: libc::c_int) {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return;
    }
    dc_param_set_int((*msg).param, DC_PARAM_DURATION as i32, duration);
}

pub unsafe fn dc_msg_latefiling_mediasize(
    msg: *mut dc_msg_t,
    width: libc::c_int,
    height: libc::c_int,
    duration: libc::c_int,
) {
    if !(msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint) {
        if width > 0i32 && height > 0i32 {
            dc_param_set_int((*msg).param, DC_PARAM_WIDTH as i32, width);
            dc_param_set_int((*msg).param, DC_PARAM_HEIGHT as i32, height);
        }
        if duration > 0i32 {
            dc_param_set_int((*msg).param, DC_PARAM_DURATION as i32, duration);
        }
        dc_msg_save_param_to_disk(msg);
    };
}

pub unsafe fn dc_msg_save_param_to_disk(msg: *mut dc_msg_t) -> bool {
    if msg.is_null() || (*msg).magic != 0x11561156i32 as libc::c_uint {
        return false;
    }

    sql::execute(
        (*msg).context,
        &(*msg).context.sql,
        "UPDATE msgs SET param=? WHERE id=?;",
        params![as_str((*(*msg).param).packed), (*msg).id as i32],
    )
    .is_ok()
}

pub unsafe fn dc_msg_new_load<'a>(context: &'a Context, msg_id: uint32_t) -> *mut dc_msg_t<'a> {
    let msg = dc_msg_new_untyped(context);
    dc_msg_load_from_db(msg, context, msg_id);
    msg
}

pub unsafe fn dc_delete_msg_from_db(context: &Context, msg_id: uint32_t) {
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    if dc_msg_load_from_db(msg, context, msg_id) {
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM msgs WHERE id=?;",
            params![(*msg).id as i32],
        );
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM msgs_mdns WHERE msg_id=?;",
            params![(*msg).id as i32],
        );
    }
    dc_msg_unref(msg);
}

/* as we do not cut inside words, this results in about 32-42 characters.
Do not use too long subjects - we add a tag after the subject which gets truncated by the clients otherwise.
It should also be very clear, the subject is _not_ the whole message.
The value is also used for CC:-summaries */

// Context functions to work with messages

pub unsafe fn dc_msg_exists(context: &Context, msg_id: uint32_t) -> libc::c_int {
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
    state: dc_move_state_t,
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

pub unsafe fn dc_set_msg_failed(context: &Context, msg_id: uint32_t, error: *const libc::c_char) {
    let mut msg = dc_msg_new_untyped(context);

    if dc_msg_load_from_db(msg, context, msg_id) {
        if 18 == (*msg).state || 20 == (*msg).state || 26 == (*msg).state {
            (*msg).state = 24
        }
        if !error.is_null() {
            dc_param_set((*msg).param, DC_PARAM_ERROR as i32, error);
            error!(context, 0, "{}", as_str(error),);
        }

        if sql::execute(
            context,
            &context.sql,
            "UPDATE msgs SET state=?, param=? WHERE id=?;",
            params![(*msg).state, as_str((*(*msg).param).packed), msg_id as i32],
        )
        .is_ok()
        {
            context.call_cb(
                Event::MSG_FAILED,
                (*msg).chat_id as uintptr_t,
                msg_id as uintptr_t,
            );
        }
    }

    dc_msg_unref(msg);
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
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
            ))
        },
    ) {
        *ret_msg_id = msg_id as u32;
        *ret_chat_id = chat_id as u32;

        if !(msg_state != 18 && msg_state != 20 && msg_state != 26) {
            /* eg. already marked as MDNS_RCVD. however, it is importent, that the message ID is set above as this will allow the caller eg. to move the message away */
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
            if chat_type == 100 {
                dc_update_msg_state(context, *ret_msg_id, 28);
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
                let soll_cnt = (dc_get_chat_contact_cnt(context, *ret_chat_id) + 1) / 2;
                if !(ist_cnt < soll_cnt) {
                    /* wait for more receipts */
                    dc_update_msg_state(context, *ret_msg_id, 28);
                    read_by_all = 1;
                }
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
    ret_server_uid: *mut uint32_t,
) -> uint32_t {
    if rfc724_mid.is_null() || unsafe { *rfc724_mid.offset(0) as libc::c_int } == 0 {
        return 0;
    }
    match context.sql.query_row(
        "SELECT server_folder, server_uid, id FROM msgs WHERE rfc724_mid=?",
        &[as_str(rfc724_mid)],
        |row| {
            if !ret_server_folder.is_null() {
                unsafe { *ret_server_folder = to_cstring(row.get::<_, String>(0)?) };
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
    server_uid: uint32_t,
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
    use std::ffi::CStr;

    #[test]
    fn test_dc_msg_guess_msgtype_from_suffix() {
        unsafe {
            let mut type_0: libc::c_int = 0;
            let mut mime_0: *mut libc::c_char = 0 as *mut libc::c_char;

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.mp3\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_AUDIO as libc::c_int);
            assert_eq!(as_str(mime_0 as *const libc::c_char), "audio/mpeg");
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.aac\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_AUDIO as libc::c_int);
            assert_eq!(as_str(mime_0 as *const libc::c_char), "audio/aac");
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.mp4\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_VIDEO as libc::c_int);
            assert_eq!(as_str(mime_0 as *const libc::c_char), "video/mp4");
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.jpg\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_IMAGE as libc::c_int);
            assert_eq!(
                CStr::from_ptr(mime_0 as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "image/jpeg"
            );
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.jpeg\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_IMAGE as libc::c_int);
            assert_eq!(
                CStr::from_ptr(mime_0 as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "image/jpeg"
            );
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.png\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_IMAGE as libc::c_int);
            assert_eq!(
                CStr::from_ptr(mime_0 as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "image/png"
            );
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.webp\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_IMAGE as libc::c_int);
            assert_eq!(
                CStr::from_ptr(mime_0 as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "image/webp"
            );
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.gif\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_GIF as libc::c_int);
            assert_eq!(
                CStr::from_ptr(mime_0 as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "image/gif"
            );
            free(mime_0 as *mut libc::c_void);

            dc_msg_guess_msgtype_from_suffix(
                b"foo/bar-sth.vcf\x00" as *const u8 as *const libc::c_char,
                &mut type_0,
                &mut mime_0,
            );
            assert_eq!(type_0, DC_MSG_FILE as libc::c_int);
            assert_eq!(
                CStr::from_ptr(mime_0 as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "text/vcard"
            );
            free(mime_0 as *mut libc::c_void);
        }
    }
}

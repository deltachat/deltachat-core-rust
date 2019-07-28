use std::ffi::CString;

use crate::constants::Event;
use crate::context::*;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_job::*;
use crate::dc_msg::*;
use crate::dc_saxparser::*;
use crate::dc_tools::*;
use crate::param::*;
use crate::sql;
use crate::stock::StockMessage;
use crate::types::*;
use crate::x::*;

// location handling
#[derive(Clone, Default)]
pub struct dc_location {
    pub location_id: uint32_t,
    pub latitude: libc::c_double,
    pub longitude: libc::c_double,
    pub accuracy: libc::c_double,
    pub timestamp: i64,
    pub contact_id: uint32_t,
    pub msg_id: uint32_t,
    pub chat_id: uint32_t,
    pub marker: Option<String>,
    pub independent: uint32_t,
}

impl dc_location {
    pub fn new() -> Self {
        dc_location {
            location_id: 0,
            latitude: 0.0,
            longitude: 0.0,
            accuracy: 0.0,
            timestamp: 0,
            contact_id: 0,
            msg_id: 0,
            chat_id: 0,
            marker: None,
            independent: 0,
        }
    }
}

#[derive(Clone)]
pub struct dc_kml_t {
    pub addr: *mut libc::c_char,
    pub locations: *mut dc_array_t,
    pub tag: libc::c_int,
    pub curr: dc_location,
}

impl dc_kml_t {
    pub fn new() -> Self {
        dc_kml_t {
            addr: std::ptr::null_mut(),
            locations: std::ptr::null_mut(),
            tag: 0,
            curr: dc_location::new(),
        }
    }
}

// location streaming
pub unsafe fn dc_send_locations_to_chat(
    context: &Context,
    chat_id: uint32_t,
    seconds: libc::c_int,
) {
    let now = time();
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let is_sending_locations_before: bool;
    if !(seconds < 0i32 || chat_id <= 9i32 as libc::c_uint) {
        is_sending_locations_before = dc_is_sending_locations_to_chat(context, chat_id);
        if sql::execute(
            context,
            &context.sql,
            "UPDATE chats    \
             SET locations_send_begin=?,        \
             locations_send_until=?  \
             WHERE id=?",
            params![
                if 0 != seconds { now } else { 0 },
                if 0 != seconds {
                    now + seconds as i64
                } else {
                    0
                },
                chat_id as i32,
            ],
        )
        .is_ok()
        {
            if 0 != seconds && !is_sending_locations_before {
                msg = dc_msg_new(context, 10i32);
                (*msg).text = to_cstring(context.stock_system_msg(
                    StockMessage::MsgLocationEnabled,
                    "",
                    "",
                    0,
                ));
                (*msg).param.set_int(Param::Cmd, 8);
                dc_send_msg(context, chat_id, msg);
            } else if 0 == seconds && is_sending_locations_before {
                let stock_str = CString::new(context.stock_system_msg(
                    StockMessage::MsgLocationDisabled,
                    "",
                    "",
                    0,
                ))
                .unwrap();
                dc_add_device_msg(context, chat_id, stock_str.as_ptr());
            }
            context.call_cb(
                Event::CHAT_MODIFIED,
                chat_id as uintptr_t,
                0i32 as uintptr_t,
            );
            if 0 != seconds {
                schedule_MAYBE_SEND_LOCATIONS(context, 0i32);
                dc_job_add(
                    context,
                    5007i32,
                    chat_id as libc::c_int,
                    None,
                    seconds + 1i32,
                );
            }
        }
    }
    dc_msg_unref(msg);
}

/*******************************************************************************
 * job to send locations out to all chats that want them
 ******************************************************************************/
unsafe fn schedule_MAYBE_SEND_LOCATIONS(context: &Context, flags: libc::c_int) {
    if 0 != flags & 0x1 || !dc_job_action_exists(context, 5005) {
        dc_job_add(context, 5005, 0, None, 60);
    };
}

pub fn dc_is_sending_locations_to_chat(context: &Context, chat_id: u32) -> bool {
    context
        .sql
        .exists(
            "SELECT id  FROM chats  WHERE (? OR id=?)   AND locations_send_until>?;",
            params![if chat_id == 0 { 1 } else { 0 }, chat_id as i32, time()],
        )
        .unwrap_or_default()
}

pub fn dc_set_location(
    context: &Context,
    latitude: libc::c_double,
    longitude: libc::c_double,
    accuracy: libc::c_double,
) -> libc::c_int {
    if latitude == 0.0 && longitude == 0.0 {
        return 1;
    }

    context.sql.query_map(
        "SELECT id FROM chats WHERE locations_send_until>?;",
        params![time()], |row| row.get::<_, i32>(0),
        |chats| {
            let mut continue_streaming = false;

            for chat in chats {
                let chat_id = chat?;
                context.sql.execute(
                    "INSERT INTO locations  \
                     (latitude, longitude, accuracy, timestamp, chat_id, from_id) VALUES (?,?,?,?,?,?);",
                    params![
                        latitude,
                        longitude,
                        accuracy,
                        time(),
                        chat_id,
                        1,
                    ]
                )?;
                continue_streaming = true;
            }
            if continue_streaming {
                context.call_cb(Event::LOCATION_CHANGED, 1, 0);
            };
            unsafe { schedule_MAYBE_SEND_LOCATIONS(context, 0) };
            Ok(continue_streaming as libc::c_int)
        }
    ).unwrap_or_default()
}

pub fn dc_get_locations(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
    timestamp_from: i64,
    mut timestamp_to: i64,
) -> *mut dc_array_t {
    if timestamp_to == 0 {
        timestamp_to = time() + 10;
    }

    context
        .sql
        .query_map(
            "SELECT l.id, l.latitude, l.longitude, l.accuracy, l.timestamp, l.independent, \
             m.id, l.from_id, l.chat_id, m.txt \
             FROM locations l  LEFT JOIN msgs m ON l.id=m.location_id  WHERE (? OR l.chat_id=?) \
             AND (? OR l.from_id=?) \
             AND (l.independent=1 OR (l.timestamp>=? AND l.timestamp<=?)) \
             ORDER BY l.timestamp DESC, l.id DESC, m.id DESC;",
            params![
                if chat_id == 0 { 1 } else { 0 },
                chat_id as i32,
                if contact_id == 0 { 1 } else { 0 },
                contact_id as i32,
                timestamp_from,
                timestamp_to,
            ],
            |row| {
                let msg_id = row.get(6)?;
                let txt: String = row.get(9)?;
                let marker = if msg_id != 0 && is_marker(&txt) {
                    Some(txt)
                } else {
                    None
                };

                let loc = dc_location {
                    location_id: row.get(0)?,
                    latitude: row.get(1)?,
                    longitude: row.get(2)?,
                    accuracy: row.get(3)?,
                    timestamp: row.get(4)?,
                    independent: row.get(5)?,
                    msg_id: msg_id,
                    contact_id: row.get(7)?,
                    chat_id: row.get(8)?,
                    marker: marker,
                };
                Ok(loc)
            },
            |locations| {
                let mut ret = dc_array_t::new_locations(500);

                for location in locations {
                    ret.add_location(location?);
                }
                Ok(ret.as_ptr())
            },
        )
        .unwrap_or_else(|_| std::ptr::null_mut())
}

fn is_marker(txt: &str) -> bool {
    txt.len() == 1 && txt.chars().next().unwrap() != ' '
}

pub fn dc_delete_all_locations(context: &Context) -> bool {
    if sql::execute(context, &context.sql, "DELETE FROM locations;", params![]).is_err() {
        return false;
    }
    context.call_cb(Event::LOCATION_CHANGED, 0, 0);
    true
}

pub fn dc_get_location_kml(
    context: &Context,
    chat_id: uint32_t,
    last_added_location_id: *mut uint32_t,
) -> *mut libc::c_char {
    let mut success: libc::c_int = 0;
    let now = time();
    let mut location_count: libc::c_int = 0;
    let mut ret = String::new();

    let self_addr = context
        .sql
        .get_config(context, "configured_addr")
        .unwrap_or_default();

    if let Ok((locations_send_begin, locations_send_until, locations_last_sent)) = context.sql.query_row(
        "SELECT locations_send_begin, locations_send_until, locations_last_sent  FROM chats  WHERE id=?;",
        params![chat_id as i32], |row| {
            let send_begin: i64 = row.get(0)?;
            let send_until: i64 = row.get(1)?;
            let last_sent: i64 = row.get(2)?;

            Ok((send_begin, send_until, last_sent))
        }
    ) {
        if !(locations_send_begin == 0 || now > locations_send_until) {
            ret += &format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document addr=\"{}\">\n",
                self_addr,
            );

            context.sql.query_map(
                "SELECT id, latitude, longitude, accuracy, timestamp\
                 FROM locations  WHERE from_id=? \
                 AND timestamp>=? \
                 AND (timestamp>=? OR timestamp=(SELECT MAX(timestamp) FROM locations WHERE from_id=?)) \
                 AND independent=0 \
                 GROUP BY timestamp \
                 ORDER BY timestamp;",
                params![1, locations_send_begin, locations_last_sent, 1],
                |row| {
                    let location_id: i32 = row.get(0)?;
                    let latitude: f64 = row.get(1)?;
                    let longitude: f64 = row.get(2)?;
                    let accuracy: f64 = row.get(3)?;
                    let timestamp = unsafe { get_kml_timestamp(row.get(4)?) };

                    Ok((location_id, latitude, longitude, accuracy, timestamp))
                },
                |rows| {
                    for row in rows {
                        let (location_id, latitude, longitude, accuracy, timestamp) = row?;
                        ret += &format!(
                            "<Placemark><Timestamp><when>{}</when></Timestamp><Point><coordinates accuracy=\"{}\">{},{}</coordinates></Point></Placemark>\n\x00",
                            as_str(timestamp),
                            accuracy,
                            longitude,
                            latitude
                        );
                        location_count += 1;
                        if !last_added_location_id.is_null() {
                            unsafe { *last_added_location_id = location_id as u32 };
                        }
                        unsafe { free(timestamp as *mut libc::c_void) };
                    }
                    Ok(())
                }
            ).unwrap(); // TODO: better error handling
        }
    }

    if location_count > 0 {
        ret += "</Document>\n</kml>";
        success = 1;
    }

    if 0 != success {
        unsafe { to_cstring(ret) }
    } else {
        std::ptr::null_mut()
    }
}

/*******************************************************************************
 * create kml-files
 ******************************************************************************/
unsafe fn get_kml_timestamp(utc: i64) -> *mut libc::c_char {
    // Returns a string formatted as YYYY-MM-DDTHH:MM:SSZ. The trailing `Z` indicates UTC.
    let res = chrono::NaiveDateTime::from_timestamp(utc, 0)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    to_cstring(res)
}

pub unsafe fn dc_get_message_kml(
    timestamp: i64,
    latitude: libc::c_double,
    longitude: libc::c_double,
) -> *mut libc::c_char {
    let timestamp_str = get_kml_timestamp(timestamp);
    let latitude_str = dc_ftoa(latitude);
    let longitude_str = dc_ftoa(longitude);

    let ret = dc_mprintf(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <kml xmlns=\"http://www.opengis.net/kml/2.2\">\n\
         <Document>\n\
         <Placemark>\
         <Timestamp><when>%s</when></Timestamp>\
         <Point><coordinates>%s,%s</coordinates></Point>\
         </Placemark>\n\
         </Document>\n\
         </kml>\x00" as *const u8 as *const libc::c_char,
        timestamp_str,
        longitude_str, // reverse order!
        latitude_str,
    );

    free(latitude_str as *mut libc::c_void);
    free(longitude_str as *mut libc::c_void);
    free(timestamp_str as *mut libc::c_void);

    ret
}

pub fn dc_set_kml_sent_timestamp(context: &Context, chat_id: u32, timestamp: i64) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET locations_last_sent=? WHERE id=?;",
        params![timestamp, chat_id as i32],
    )
    .is_ok()
}

pub fn dc_set_msg_location_id(context: &Context, msg_id: u32, location_id: u32) -> bool {
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET location_id=? WHERE id=?;",
        params![location_id, msg_id as i32],
    )
    .is_ok()
}

pub unsafe fn dc_save_locations(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
    locations: *const dc_array_t,
    independent: libc::c_int,
) -> u32 {
    if chat_id <= 9 || locations.is_null() {
        return 0;
    }

    context
        .sql
        .prepare2(
            "SELECT id FROM locations WHERE timestamp=? AND from_id=?",
            "INSERT INTO locations\
             (timestamp, from_id, chat_id, latitude, longitude, accuracy, independent) \
             VALUES (?,?,?,?,?,?,?);",
            |mut stmt_test, mut stmt_insert, conn| {
                let mut newest_timestamp = 0;
                let mut newest_location_id = 0;

                for i in 0..dc_array_get_cnt(locations) {
                    let location = dc_array_get_ptr(locations, i as size_t) as *mut dc_location;

                    let exists =
                        stmt_test.exists(params![(*location).timestamp, contact_id as i32])?;

                    if 0 != independent || !exists {
                        stmt_insert.execute(params![
                            (*location).timestamp,
                            contact_id as i32,
                            chat_id as i32,
                            (*location).latitude,
                            (*location).longitude,
                            (*location).accuracy,
                            independent,
                        ])?;

                        if (*location).timestamp > newest_timestamp {
                            newest_timestamp = (*location).timestamp;
                            newest_location_id = sql::get_rowid2_with_conn(
                                context,
                                conn,
                                "locations",
                                "timestamp",
                                (*location).timestamp,
                                "from_id",
                                contact_id as i32,
                            );
                        }
                    }
                }
                Ok(newest_location_id)
            },
        )
        .unwrap_or_default()
}

pub unsafe fn dc_kml_parse(
    context: &Context,
    content: *const libc::c_char,
    content_bytes: size_t,
) -> dc_kml_t {
    let mut kml = dc_kml_t::new();
    let mut content_nullterminated: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut saxparser: dc_saxparser_t = dc_saxparser_t {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };

    if content_bytes > (1 * 1024 * 1024) {
        warn!(
            context,
            0, "A kml-files with {} bytes is larger than reasonably expected.", content_bytes,
        );
    } else {
        content_nullterminated = dc_null_terminate(content, content_bytes as libc::c_int);
        if !content_nullterminated.is_null() {
            kml.locations = dc_array_new_locations(100);
            dc_saxparser_init(
                &mut saxparser,
                &mut kml as *mut dc_kml_t as *mut libc::c_void,
            );
            dc_saxparser_set_tag_handler(
                &mut saxparser,
                Some(kml_starttag_cb),
                Some(kml_endtag_cb),
            );
            dc_saxparser_set_text_handler(&mut saxparser, Some(kml_text_cb));
            dc_saxparser_parse(&mut saxparser, content_nullterminated);
        }
    }

    free(content_nullterminated as *mut libc::c_void);

    kml
}

unsafe fn kml_text_cb(userdata: *mut libc::c_void, text: *const libc::c_char, _len: libc::c_int) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if 0 != (*kml).tag & (0x4 | 0x10) {
        let mut val: *mut libc::c_char = dc_strdup(text);
        dc_str_replace(
            &mut val,
            b"\n\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b"\r\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b"\t\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b" \x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != (*kml).tag & 0x4 && strlen(val) >= 19 {
            // YYYY-MM-DDTHH:MM:SSZ
            // 0   4  7  10 13 16 19
            let val_r = as_str(val);
            match chrono::NaiveDateTime::parse_from_str(val_r, "%Y-%m-%dT%H:%M:%SZ") {
                Ok(res) => {
                    (*kml).curr.timestamp = res.timestamp();
                    if (*kml).curr.timestamp > time() {
                        (*kml).curr.timestamp = time();
                    }
                }
                Err(_err) => {
                    (*kml).curr.timestamp = time();
                }
            }
        } else if 0 != (*kml).tag & 0x10 {
            let mut comma: *mut libc::c_char = strchr(val, ',' as i32);
            if !comma.is_null() {
                let longitude: *mut libc::c_char = val;
                let latitude: *mut libc::c_char = comma.offset(1isize);
                *comma = 0 as libc::c_char;
                comma = strchr(latitude, ',' as i32);
                if !comma.is_null() {
                    *comma = 0 as libc::c_char
                }
                (*kml).curr.latitude = dc_atof(latitude);
                (*kml).curr.longitude = dc_atof(longitude)
            }
        }
        free(val as *mut libc::c_void);
    };
}

unsafe fn kml_endtag_cb(userdata: *mut libc::c_void, tag: *const libc::c_char) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0 {
        if 0 != (*kml).tag & 0x1
            && 0 != (*kml).curr.timestamp
            && 0. != (*kml).curr.latitude
            && 0. != (*kml).curr.longitude
        {
            let location = (*kml).curr.clone();
            (*(*kml).locations).add_location(location);
        }
        (*kml).tag = 0
    };
}

/*******************************************************************************
 * parse kml-files
 ******************************************************************************/
unsafe fn kml_starttag_cb(
    userdata: *mut libc::c_void,
    tag: *const libc::c_char,
    attr: *mut *mut libc::c_char,
) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if strcmp(tag, b"document\x00" as *const u8 as *const libc::c_char) == 0 {
        let addr: *const libc::c_char =
            dc_attr_find(attr, b"addr\x00" as *const u8 as *const libc::c_char);
        if !addr.is_null() {
            (*kml).addr = dc_strdup(addr)
        }
    } else if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0 {
        (*kml).tag = 0x1;
        (*kml).curr.timestamp = 0;
        (*kml).curr.latitude = 0 as libc::c_double;
        (*kml).curr.longitude = 0.0f64;
        (*kml).curr.accuracy = 0.0f64
    } else if strcmp(tag, b"timestamp\x00" as *const u8 as *const libc::c_char) == 0
        && 0 != (*kml).tag & 0x1
    {
        (*kml).tag = 0x1 | 0x2
    } else if strcmp(tag, b"when\x00" as *const u8 as *const libc::c_char) == 0
        && 0 != (*kml).tag & 0x2
    {
        (*kml).tag = 0x1 | 0x2 | 0x4
    } else if strcmp(tag, b"point\x00" as *const u8 as *const libc::c_char) == 0
        && 0 != (*kml).tag & 0x1
    {
        (*kml).tag = 0x1 | 0x8
    } else if strcmp(tag, b"coordinates\x00" as *const u8 as *const libc::c_char) == 0
        && 0 != (*kml).tag & 0x8
    {
        (*kml).tag = 0x1 | 0x8 | 0x10;
        let accuracy: *const libc::c_char =
            dc_attr_find(attr, b"accuracy\x00" as *const u8 as *const libc::c_char);
        if !accuracy.is_null() {
            (*kml).curr.accuracy = dc_atof(accuracy)
        }
    };
}

pub unsafe fn dc_kml_unref(kml: *mut dc_kml_t) {
    if kml.is_null() {
        return;
    }
    dc_array_unref((*kml).locations);
    free((*kml).addr as *mut libc::c_void);
}

pub unsafe fn dc_job_do_DC_JOB_MAYBE_SEND_LOCATIONS(context: &Context, _job: *mut dc_job_t) {
    let now = time();
    let mut continue_streaming: libc::c_int = 1;
    info!(
        context,
        0, " ----------------- MAYBE_SEND_LOCATIONS -------------- ",
    );

    context
        .sql
        .query_map(
            "SELECT id, locations_send_begin, locations_last_sent \
             FROM chats \
             WHERE locations_send_until>?;",
            params![now],
            |row| {
                let chat_id: i32 = row.get(0)?;
                let locations_send_begin: i64 = row.get(1)?;
                let locations_last_sent: i64 = row.get(2)?;
                continue_streaming = 1;

                // be a bit tolerant as the timer may not align exactly with time(NULL)
                if now - locations_last_sent < (60 - 3) {
                    Ok(None)
                } else {
                    Ok(Some((chat_id, locations_send_begin, locations_last_sent)))
                }
            },
            |rows| {
                context.sql.prepare(
                    "SELECT id \
                     FROM locations \
                     WHERE from_id=? \
                     AND timestamp>=? \
                     AND timestamp>? \
                     AND independent=0 \
                     ORDER BY timestamp;",
                    |mut stmt_locations, _| {
                        for (chat_id, locations_send_begin, locations_last_sent) in
                            rows.filter_map(|r| match r {
                                Ok(Some(v)) => Some(v),
                                _ => None,
                            })
                        {
                            // TODO: do I need to reset?
                            if !stmt_locations
                                .exists(params![1, locations_send_begin, locations_last_sent,])
                                .unwrap_or_default()
                            {
                                // if there is no new location, there's nothing to send.
                                // however, maybe we want to bypass this test eg. 15 minutes
                                continue;
                            }
                            // pending locations are attached automatically to every message,
                            // so also to this empty text message.
                            // DC_CMD_LOCATION is only needed to create a nicer subject.
                            //
                            // for optimisation and to avoid flooding the sending queue,
                            // we could sending these messages only if we're really online.
                            // the easiest way to determine this, is to check for an empty message queue.
                            // (might not be 100%, however, as positions are sent combined later
                            // and dc_set_location() is typically called periodically, this is ok)
                            let mut msg = dc_msg_new(context, 10);
                            (*msg).hidden = 1;
                            (*msg).param.set_int(Param::Cmd, 9);
                            dc_send_msg(context, chat_id as u32, msg);
                            dc_msg_unref(msg);
                        }
                        Ok(())
                    },
                )
            },
        )
        .unwrap(); // TODO: Better error handling

    if 0 != continue_streaming {
        schedule_MAYBE_SEND_LOCATIONS(context, 0x1);
    }
}

pub unsafe fn dc_job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(context: &Context, job: &mut dc_job_t) {
    // this function is called when location-streaming _might_ have ended for a chat.
    // the function checks, if location-streaming is really ended;
    // if so, a device-message is added if not yet done.

    let chat_id = (*job).foreign_id;

    if let Ok((send_begin, send_until)) = context.sql.query_row(
        "SELECT locations_send_begin, locations_send_until  FROM chats  WHERE id=?",
        params![chat_id as i32],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ) {
        if !(send_begin != 0 && time() <= send_until) {
            // still streaming -
            // may happen as several calls to dc_send_locations_to_chat()
            // do not un-schedule pending DC_MAYBE_SEND_LOC_ENDED jobs
            if !(send_begin == 0 && send_until == 0) {
                // not streaming, device-message already sent
                if context.sql.execute(
                    "UPDATE chats    SET locations_send_begin=0, locations_send_until=0  WHERE id=?",
                    params![chat_id as i32],
                ).is_ok() {
                    let stock_str = CString::new(context.stock_system_msg(StockMessage::MsgLocationDisabled, "", "", 0)).unwrap();
                    dc_add_device_msg(context, chat_id, stock_str.as_ptr());
                    context.call_cb(
                        Event::CHAT_MODIFIED,
                        chat_id as usize,
                        0,
                    );
                }
            }
        }
    }
}

use crate::constants::Event;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_context::*;
use crate::dc_job::*;
use crate::dc_log::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_saxparser::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

// location handling
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_location_t {
    pub location_id: uint32_t,
    pub latitude: libc::c_double,
    pub longitude: libc::c_double,
    pub accuracy: libc::c_double,
    pub timestamp: time_t,
    pub contact_id: uint32_t,
    pub msg_id: uint32_t,
    pub chat_id: uint32_t,
    pub marker: *mut libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_kml_t {
    pub addr: *mut libc::c_char,
    pub locations: *mut dc_array_t,
    pub tag: libc::c_int,
    pub curr: dc_location_t,
}

// location streaming
pub unsafe fn dc_send_locations_to_chat(
    context: &dc_context_t,
    chat_id: uint32_t,
    seconds: libc::c_int,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let now: time_t = time(0 as *mut time_t);
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut stock_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let is_sending_locations_before: libc::c_int;
    if !(seconds < 0i32 || chat_id <= 9i32 as libc::c_uint) {
        is_sending_locations_before = dc_is_sending_locations_to_chat(context, chat_id);
        stmt =
            dc_sqlite3_prepare(
                context,
                &context.sql.clone().read().unwrap(),
                b"UPDATE chats    SET locations_send_begin=?,        locations_send_until=?  WHERE id=?\x00"
                    as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(
            stmt,
            1i32,
            (if 0 != seconds { now } else { 0 }) as sqlite3_int64,
        );
        sqlite3_bind_int64(
            stmt,
            2i32,
            (if 0 != seconds {
                now + seconds as time_t
            } else {
                0
            }) as sqlite3_int64,
        );
        sqlite3_bind_int(stmt, 3i32, chat_id as libc::c_int);
        sqlite3_step(stmt);
        if 0 != seconds && 0 == is_sending_locations_before {
            msg = dc_msg_new(context, 10i32);
            (*msg).text = dc_stock_system_msg(
                context,
                64i32,
                0 as *const libc::c_char,
                0 as *const libc::c_char,
                0i32 as uint32_t,
            );
            dc_param_set_int((*msg).param, 'S' as i32, 8i32);
            dc_send_msg(context, chat_id, msg);
        } else if 0 == seconds && 0 != is_sending_locations_before {
            stock_str = dc_stock_system_msg(
                context,
                65i32,
                0 as *const libc::c_char,
                0 as *const libc::c_char,
                0i32 as uint32_t,
            );
            dc_add_device_msg(context, chat_id, stock_str);
        }
        (context.cb)(
            context,
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
                0 as *const libc::c_char,
                seconds + 1i32,
            );
        }
    }
    free(stock_str as *mut libc::c_void);
    dc_msg_unref(msg);
    sqlite3_finalize(stmt);
}

/*******************************************************************************
 * job to send locations out to all chats that want them
 ******************************************************************************/
unsafe fn schedule_MAYBE_SEND_LOCATIONS(context: &dc_context_t, flags: libc::c_int) {
    if 0 != flags & 0x1i32 || 0 == dc_job_action_exists(context, 5005i32) {
        dc_job_add(context, 5005i32, 0i32, 0 as *const libc::c_char, 60i32);
    };
}

pub unsafe fn dc_is_sending_locations_to_chat(
    context: &dc_context_t,
    chat_id: uint32_t,
) -> libc::c_int {
    let mut is_sending_locations: libc::c_int = 0i32;
    let stmt: *mut sqlite3_stmt;

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT id  FROM chats  WHERE (? OR id=?)   AND locations_send_until>?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(
        stmt,
        1i32,
        if chat_id == 0i32 as libc::c_uint {
            1i32
        } else {
            0i32
        },
    );
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_bind_int64(stmt, 3i32, time(0 as *mut time_t) as sqlite3_int64);
    if !(sqlite3_step(stmt) != 100i32) {
        is_sending_locations = 1i32
    }

    sqlite3_finalize(stmt);

    is_sending_locations
}

pub unsafe fn dc_set_location(
    context: &dc_context_t,
    latitude: libc::c_double,
    longitude: libc::c_double,
    accuracy: libc::c_double,
) -> libc::c_int {
    let mut stmt_chats: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stmt_insert: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut continue_streaming: libc::c_int = 0i32;
    if latitude == 0.0f64 && longitude == 0.0f64 {
        continue_streaming = 1i32
    } else {
        stmt_chats = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT id FROM chats WHERE locations_send_until>?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int64(stmt_chats, 1i32, time(0 as *mut time_t) as sqlite3_int64);
        while sqlite3_step(stmt_chats) == 100i32 {
            let chat_id: uint32_t = sqlite3_column_int(stmt_chats, 0i32) as uint32_t;
            stmt_insert =
                dc_sqlite3_prepare(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"INSERT INTO locations  (latitude, longitude, accuracy, timestamp, chat_id, from_id) VALUES (?,?,?,?,?,?);\x00"
                        as *const u8 as *const libc::c_char);
            sqlite3_bind_double(stmt_insert, 1i32, latitude);
            sqlite3_bind_double(stmt_insert, 2i32, longitude);
            sqlite3_bind_double(stmt_insert, 3i32, accuracy);
            sqlite3_bind_int64(stmt_insert, 4i32, time(0 as *mut time_t) as sqlite3_int64);
            sqlite3_bind_int(stmt_insert, 5i32, chat_id as libc::c_int);
            sqlite3_bind_int(stmt_insert, 6i32, 1i32);
            sqlite3_step(stmt_insert);
            continue_streaming = 1i32
        }
        if 0 != continue_streaming {
            (context.cb)(
                context,
                Event::LOCATION_CHANGED,
                1i32 as uintptr_t,
                0i32 as uintptr_t,
            );
            schedule_MAYBE_SEND_LOCATIONS(context, 0i32);
        }
    }
    sqlite3_finalize(stmt_chats);
    sqlite3_finalize(stmt_insert);

    continue_streaming
}

pub unsafe fn dc_get_locations(
    context: &dc_context_t,
    chat_id: uint32_t,
    contact_id: uint32_t,
    timestamp_from: time_t,
    mut timestamp_to: time_t,
) -> *mut dc_array_t {
    let ret: *mut dc_array_t = dc_array_new_typed(1i32, 500i32 as size_t);
    let stmt: *mut sqlite3_stmt;

    if timestamp_to == 0 {
        timestamp_to = time(0 as *mut time_t) + 10;
    }
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT l.id, l.latitude, l.longitude, l.accuracy, l.timestamp, l.independent, \
              m.id, l.from_id, l.chat_id, m.txt \
              FROM locations l  LEFT JOIN msgs m ON l.id=m.location_id  WHERE (? OR l.chat_id=?) \
              AND (? OR l.from_id=?) \
              AND (l.independent=1 OR (l.timestamp>=? AND l.timestamp<=?)) \
              ORDER BY l.timestamp DESC, l.id DESC, m.id DESC;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(
        stmt,
        1i32,
        if chat_id == 0i32 as libc::c_uint {
            1i32
        } else {
            0i32
        },
    );
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_bind_int(
        stmt,
        3i32,
        if contact_id == 0i32 as libc::c_uint {
            1i32
        } else {
            0i32
        },
    );
    sqlite3_bind_int(stmt, 4i32, contact_id as libc::c_int);
    sqlite3_bind_int(stmt, 5i32, timestamp_from as libc::c_int);
    sqlite3_bind_int(stmt, 6i32, timestamp_to as libc::c_int);
    while sqlite3_step(stmt) == 100i32 {
        let mut loc: *mut _dc_location =
            calloc(1, ::std::mem::size_of::<_dc_location>()) as *mut _dc_location;
        if loc.is_null() {
            break;
        }
        (*loc).location_id = sqlite3_column_double(stmt, 0i32) as uint32_t;
        (*loc).latitude = sqlite3_column_double(stmt, 1i32);
        (*loc).longitude = sqlite3_column_double(stmt, 2i32);
        (*loc).accuracy = sqlite3_column_double(stmt, 3i32);
        (*loc).timestamp = sqlite3_column_int64(stmt, 4i32) as time_t;
        (*loc).independent = sqlite3_column_int(stmt, 5i32) as uint32_t;
        (*loc).msg_id = sqlite3_column_int(stmt, 6i32) as uint32_t;
        (*loc).contact_id = sqlite3_column_int(stmt, 7i32) as uint32_t;
        (*loc).chat_id = sqlite3_column_int(stmt, 8i32) as uint32_t;

        if 0 != (*loc).msg_id {
            let txt: *const libc::c_char = sqlite3_column_text(stmt, 9i32) as *const libc::c_char;
            if 0 != is_marker(txt) {
                (*loc).marker = strdup(txt)
            }
        }
        dc_array_add_ptr(ret, loc as *mut libc::c_void);
    }

    sqlite3_finalize(stmt);

    ret
}

// TODO should be bool /rtn
unsafe fn is_marker(txt: *const libc::c_char) -> libc::c_int {
    if !txt.is_null() {
        let len: libc::c_int = dc_utf8_strlen(txt) as libc::c_int;
        if len == 1i32 && *txt.offset(0isize) as libc::c_int != ' ' as i32 {
            return 1i32;
        }
    }

    0
}

pub unsafe fn dc_delete_all_locations(context: &dc_context_t) {
    let stmt: *mut sqlite3_stmt;

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"DELETE FROM locations;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    (context.cb)(
        context,
        Event::LOCATION_CHANGED,
        0i32 as uintptr_t,
        0i32 as uintptr_t,
    );

    sqlite3_finalize(stmt);
}

pub unsafe fn dc_get_location_kml(
    context: &dc_context_t,
    chat_id: uint32_t,
    last_added_location_id: *mut uint32_t,
) -> *mut libc::c_char {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt;
    let self_addr: *mut libc::c_char;
    let now: time_t = time(0 as *mut time_t);
    let locations_send_begin: time_t;
    let locations_send_until: time_t;
    let locations_last_sent: time_t;
    let mut location_count: libc::c_int = 0i32;
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 1000i32);

    self_addr = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        b"\x00" as *const u8 as *const libc::c_char,
    );
    stmt =
        dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT locations_send_begin, locations_send_until, locations_last_sent  FROM chats  WHERE id=?;\x00"
                as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(stmt) != 100i32) {
        locations_send_begin = sqlite3_column_int64(stmt, 0i32) as time_t;
        locations_send_until = sqlite3_column_int64(stmt, 1i32) as time_t;
        locations_last_sent = sqlite3_column_int64(stmt, 2i32) as time_t;
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if !(locations_send_begin == 0 || now > locations_send_until) {
            dc_strbuilder_catf(&mut ret as *mut dc_strbuilder_t,
                                   b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document addr=\"%s\">\n\x00"
                                       as *const u8 as *const libc::c_char,
                                   self_addr);
            stmt = dc_sqlite3_prepare(
                context,
                    &context.sql.clone().read().unwrap(),
                    b"SELECT id, latitude, longitude, accuracy, timestamp\
                          FROM locations  WHERE from_id=? \
                          AND timestamp>=? \
                          AND (timestamp>=? OR timestamp=(SELECT MAX(timestamp) FROM locations WHERE from_id=?)) \
                          AND independent=0 \
                          GROUP BY timestamp \
                          ORDER BY timestamp;\x00" as *const u8
                        as *const libc::c_char,
                );

            sqlite3_bind_int(stmt, 1i32, 1i32);
            sqlite3_bind_int64(stmt, 2i32, locations_send_begin as sqlite3_int64);
            sqlite3_bind_int64(stmt, 3i32, locations_last_sent as sqlite3_int64);
            sqlite3_bind_int(stmt, 4i32, 1i32);
            while sqlite3_step(stmt) == 100i32 {
                let location_id: uint32_t = sqlite3_column_int(stmt, 0i32) as uint32_t;
                let latitude: *mut libc::c_char = dc_ftoa(sqlite3_column_double(stmt, 1i32));
                let longitude: *mut libc::c_char = dc_ftoa(sqlite3_column_double(stmt, 2i32));
                let accuracy: *mut libc::c_char = dc_ftoa(sqlite3_column_double(stmt, 3i32));
                let timestamp: *mut libc::c_char =
                    get_kml_timestamp(sqlite3_column_int64(stmt, 4i32) as time_t);
                dc_strbuilder_catf(&mut ret as *mut dc_strbuilder_t,
                                       b"<Placemark><Timestamp><when>%s</when></Timestamp><Point><coordinates accuracy=\"%s\">%s,%s</coordinates></Point></Placemark>\n\x00"
                                           as *const u8 as
                                           *const libc::c_char, timestamp,
                                       accuracy, longitude, latitude);
                location_count += 1;
                if !last_added_location_id.is_null() {
                    *last_added_location_id = location_id
                }
                free(latitude as *mut libc::c_void);
                free(longitude as *mut libc::c_void);
                free(accuracy as *mut libc::c_void);
                free(timestamp as *mut libc::c_void);
            }
            if !(location_count == 0i32) {
                dc_strbuilder_cat(
                    &mut ret,
                    b"</Document>\n</kml>\x00" as *const u8 as *const libc::c_char,
                );
                success = 1i32
            }
        }
    }

    sqlite3_finalize(stmt);
    free(self_addr as *mut libc::c_void);
    if 0 == success {
        free(ret.buf as *mut libc::c_void);
    }
    return if 0 != success {
        ret.buf
    } else {
        0 as *mut libc::c_char
    };
}

/*******************************************************************************
 * create kml-files
 ******************************************************************************/
unsafe fn get_kml_timestamp(mut utc: time_t) -> *mut libc::c_char {
    // Returns a string formatted as YYYY-MM-DDTHH:MM:SSZ. The trailing `Z` indicates UTC.
    let mut wanted_struct: tm = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: 0 as *mut libc::c_char,
    };
    memcpy(
        &mut wanted_struct as *mut tm as *mut libc::c_void,
        gmtime(&mut utc) as *const libc::c_void,
        ::std::mem::size_of::<tm>(),
    );

    dc_mprintf(
        b"%04i-%02i-%02iT%02i:%02i:%02iZ\x00" as *const u8 as *const libc::c_char,
        wanted_struct.tm_year as libc::c_int + 1900i32,
        wanted_struct.tm_mon as libc::c_int + 1i32,
        wanted_struct.tm_mday as libc::c_int,
        wanted_struct.tm_hour as libc::c_int,
        wanted_struct.tm_min as libc::c_int,
        wanted_struct.tm_sec as libc::c_int,
    )
}

pub unsafe fn dc_get_message_kml(
    timestamp: time_t,
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

pub unsafe fn dc_set_kml_sent_timestamp(
    context: &dc_context_t,
    chat_id: uint32_t,
    timestamp: time_t,
) {
    let stmt: *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"UPDATE chats SET locations_last_sent=? WHERE id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_set_msg_location_id(
    context: &dc_context_t,
    msg_id: uint32_t,
    location_id: uint32_t,
) {
    let stmt: *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"UPDATE msgs SET location_id=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, location_id as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, msg_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_save_locations(
    context: &dc_context_t,
    chat_id: uint32_t,
    contact_id: uint32_t,
    locations: *const dc_array_t,
    independent: libc::c_int,
) -> uint32_t {
    let mut stmt_test: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stmt_insert: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut newest_timestamp: time_t = 0i32 as time_t;
    let mut newest_location_id: uint32_t = 0i32 as uint32_t;
    if !(chat_id <= 9i32 as libc::c_uint || locations.is_null()) {
        stmt_test = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT id FROM locations WHERE timestamp=? AND from_id=?\x00" as *const u8
                as *const libc::c_char,
        );
        stmt_insert = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"INSERT INTO locations\
                  (timestamp, from_id, chat_id, latitude, longitude, accuracy, independent) \
                  VALUES (?,?,?,?,?,?,?);\x00" as *const u8 as *const libc::c_char,
        );
        let mut i = 0;
        while i < dc_array_get_cnt(locations) {
            let location: *mut dc_location_t =
                dc_array_get_ptr(locations, i as size_t) as *mut dc_location_t;
            sqlite3_reset(stmt_test);
            sqlite3_bind_int64(stmt_test, 1i32, (*location).timestamp as sqlite3_int64);
            sqlite3_bind_int(stmt_test, 2i32, contact_id as libc::c_int);
            if independent | sqlite3_step(stmt_test) != 100i32 {
                sqlite3_reset(stmt_insert);
                sqlite3_bind_int64(stmt_insert, 1i32, (*location).timestamp as sqlite3_int64);
                sqlite3_bind_int(stmt_insert, 2i32, contact_id as libc::c_int);
                sqlite3_bind_int(stmt_insert, 3i32, chat_id as libc::c_int);
                sqlite3_bind_double(stmt_insert, 4i32, (*location).latitude);
                sqlite3_bind_double(stmt_insert, 5i32, (*location).longitude);
                sqlite3_bind_double(stmt_insert, 6i32, (*location).accuracy);
                sqlite3_bind_double(stmt_insert, 7i32, independent as libc::c_double);
                sqlite3_step(stmt_insert);
            }
            if (*location).timestamp > newest_timestamp {
                newest_timestamp = (*location).timestamp;
                newest_location_id = dc_sqlite3_get_rowid2(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"locations\x00" as *const u8 as *const libc::c_char,
                    b"timestamp\x00" as *const u8 as *const libc::c_char,
                    (*location).timestamp as uint64_t,
                    b"from_id\x00" as *const u8 as *const libc::c_char,
                    contact_id,
                )
            }
            i += 1
        }
    }
    sqlite3_finalize(stmt_test);
    sqlite3_finalize(stmt_insert);

    newest_location_id
}

pub unsafe fn dc_kml_parse(
    context: &dc_context_t,
    content: *const libc::c_char,
    content_bytes: size_t,
) -> *mut dc_kml_t {
    let mut kml: *mut dc_kml_t = calloc(1, ::std::mem::size_of::<dc_kml_t>()) as *mut dc_kml_t;
    let mut content_nullterminated: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut saxparser: dc_saxparser_t = dc_saxparser_t {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };

    if content_bytes > (1 * 1024 * 1024) {
        dc_log_warning(
            context,
            0,
            b"A kml-files with %i bytes is larger than reasonably expected.\x00" as *const u8
                as *const libc::c_char,
            content_bytes,
        );
    } else {
        content_nullterminated = dc_null_terminate(content, content_bytes as libc::c_int);
        if !content_nullterminated.is_null() {
            (*kml).locations = dc_array_new_typed(1, 100 as size_t);
            dc_saxparser_init(&mut saxparser, kml as *mut libc::c_void);
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
            let mut tmval: tm = tm {
                tm_sec: 0,
                tm_min: 0,
                tm_hour: 0,
                tm_mday: 0,
                tm_mon: 0,
                tm_year: 0,
                tm_wday: 0,
                tm_yday: 0,
                tm_isdst: 0,
                tm_gmtoff: 0,
                tm_zone: 0 as *mut libc::c_char,
            };
            memset(
                &mut tmval as *mut tm as *mut libc::c_void,
                0,
                ::std::mem::size_of::<tm>(),
            );
            *val.offset(4isize) = 0i32 as libc::c_char;
            tmval.tm_year = atoi(val) - 1900i32;
            *val.offset(7isize) = 0i32 as libc::c_char;
            tmval.tm_mon = atoi(val.offset(5isize)) - 1i32;
            *val.offset(10isize) = 0i32 as libc::c_char;
            tmval.tm_mday = atoi(val.offset(8isize));
            *val.offset(13isize) = 0i32 as libc::c_char;
            tmval.tm_hour = atoi(val.offset(11isize));
            *val.offset(16isize) = 0i32 as libc::c_char;
            tmval.tm_min = atoi(val.offset(14isize));
            *val.offset(19isize) = 0i32 as libc::c_char;
            tmval.tm_sec = atoi(val.offset(17isize));
            (*kml).curr.timestamp = mkgmtime(&mut tmval);
            if (*kml).curr.timestamp > time(0 as *mut time_t) {
                (*kml).curr.timestamp = time(0 as *mut time_t)
            }
        } else if 0 != (*kml).tag & 0x10i32 {
            let mut comma: *mut libc::c_char = strchr(val, ',' as i32);
            if !comma.is_null() {
                let longitude: *mut libc::c_char = val;
                let latitude: *mut libc::c_char = comma.offset(1isize);
                *comma = 0i32 as libc::c_char;
                comma = strchr(latitude, ',' as i32);
                if !comma.is_null() {
                    *comma = 0i32 as libc::c_char
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
    if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if 0 != (*kml).tag & 0x1i32
            && 0 != (*kml).curr.timestamp
            && 0. != (*kml).curr.latitude
            && 0. != (*kml).curr.longitude
        {
            let location: *mut dc_location_t =
                calloc(1, ::std::mem::size_of::<dc_location_t>()) as *mut dc_location_t;
            *location = (*kml).curr;
            dc_array_add_ptr((*kml).locations, location as *mut libc::c_void);
        }
        (*kml).tag = 0i32
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
    if strcmp(tag, b"document\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let addr: *const libc::c_char =
            dc_attr_find(attr, b"addr\x00" as *const u8 as *const libc::c_char);
        if !addr.is_null() {
            (*kml).addr = dc_strdup(addr)
        }
    } else if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*kml).tag = 0x1i32;
        (*kml).curr.timestamp = 0i32 as time_t;
        (*kml).curr.latitude = 0i32 as libc::c_double;
        (*kml).curr.longitude = 0.0f64;
        (*kml).curr.accuracy = 0.0f64
    } else if strcmp(tag, b"timestamp\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x1i32
    {
        (*kml).tag = 0x1i32 | 0x2i32
    } else if strcmp(tag, b"when\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x2i32
    {
        (*kml).tag = 0x1i32 | 0x2i32 | 0x4i32
    } else if strcmp(tag, b"point\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x1i32
    {
        (*kml).tag = 0x1i32 | 0x8i32
    } else if strcmp(tag, b"coordinates\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x8i32
    {
        (*kml).tag = 0x1i32 | 0x8i32 | 0x10i32;
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
    free(kml as *mut libc::c_void);
}

pub unsafe fn dc_job_do_DC_JOB_MAYBE_SEND_LOCATIONS(context: &dc_context_t, _job: *mut dc_job_t) {
    let stmt_chats: *mut sqlite3_stmt;
    let mut stmt_locations: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let now: time_t = time(0 as *mut time_t);
    let mut continue_streaming: libc::c_int = 1i32;
    dc_log_info(
        context,
        0i32,
        b" ----------------- MAYBE_SEND_LOCATIONS -------------- \x00" as *const u8
            as *const libc::c_char,
    );
    stmt_chats = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT id, locations_send_begin, locations_last_sent \
              FROM chats \
              WHERE locations_send_until>?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt_chats, 1i32, now as sqlite3_int64);
    while sqlite3_step(stmt_chats) == 100i32 {
        let chat_id: uint32_t = sqlite3_column_int(stmt_chats, 0i32) as uint32_t;
        let locations_send_begin: time_t = sqlite3_column_int64(stmt_chats, 1i32) as time_t;
        let locations_last_sent: time_t = sqlite3_column_int64(stmt_chats, 2i32) as time_t;
        continue_streaming = 1i32;
        // be a bit tolerant as the timer may not align exactly with time(NULL)
        if now - locations_last_sent < (60 - 3) {
            continue;
        }
        if stmt_locations.is_null() {
            stmt_locations = dc_sqlite3_prepare(
                context,
                &context.sql.clone().read().unwrap(),
                b"SELECT id \
                  FROM locations \
                  WHERE from_id=? \
                  AND timestamp>=? \
                  AND timestamp>? \
                  AND independent=0 \
                  ORDER BY timestamp;\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            sqlite3_reset(stmt_locations);
        }
        sqlite3_bind_int(stmt_locations, 1i32, 1i32);
        sqlite3_bind_int64(stmt_locations, 2i32, locations_send_begin as sqlite3_int64);
        sqlite3_bind_int64(stmt_locations, 3i32, locations_last_sent as sqlite3_int64);
        // if there is no new location, there's nothing to send.
        // however, maybe we want to bypass this test eg. 15 minutes
        if sqlite3_step(stmt_locations) != 100i32 {
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
        let mut msg: *mut dc_msg_t = dc_msg_new(context, 10i32);
        (*msg).hidden = 1i32;
        dc_param_set_int((*msg).param, 'S' as i32, 9i32);
        dc_send_msg(context, chat_id, msg);
        dc_msg_unref(msg);
    }
    if 0 != continue_streaming {
        schedule_MAYBE_SEND_LOCATIONS(context, 0x1i32);
    }
    sqlite3_finalize(stmt_chats);
    sqlite3_finalize(stmt_locations);
}

pub unsafe fn dc_job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(context: &dc_context_t, job: &mut dc_job_t) {
    // this function is called when location-streaming _might_ have ended for a chat.
    // the function checks, if location-streaming is really ended;
    // if so, a device-message is added if not yet done.
    let chat_id: uint32_t = (*job).foreign_id;
    let locations_send_begin: time_t;
    let locations_send_until: time_t;
    let mut stmt;
    let mut stock_str: *mut libc::c_char = 0 as *mut libc::c_char;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT locations_send_begin, locations_send_until  FROM chats  WHERE id=?\x00"
            as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(stmt) != 100i32) {
        locations_send_begin = sqlite3_column_int64(stmt, 0i32) as time_t;
        locations_send_until = sqlite3_column_int64(stmt, 1i32) as time_t;
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if !(locations_send_begin != 0 && time(0 as *mut time_t) <= locations_send_until) {
            // still streaming -
            // may happen as several calls to dc_send_locations_to_chat()
            // do not un-schedule pending DC_MAYBE_SEND_LOC_ENDED jobs
            if !(locations_send_begin == 0 && locations_send_until == 0) {
                // not streaming, device-message already sent
                stmt =
                    dc_sqlite3_prepare(
                        context,
                        &context.sql.clone().read().unwrap(),
                        b"UPDATE chats    SET locations_send_begin=0, locations_send_until=0  WHERE id=?\x00"
                            as *const u8 as
                            *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                sqlite3_step(stmt);
                stock_str = dc_stock_system_msg(
                    context,
                    65i32,
                    0 as *const libc::c_char,
                    0 as *const libc::c_char,
                    0i32 as uint32_t,
                );
                dc_add_device_msg(context, chat_id, stock_str);
                (context.cb)(
                    context,
                    Event::CHAT_MODIFIED,
                    chat_id as uintptr_t,
                    0i32 as uintptr_t,
                );
            }
        }
    }
    sqlite3_finalize(stmt);
    free(stock_str as *mut libc::c_void);
}

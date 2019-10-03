use bitflags::bitflags;
use quick_xml;
use quick_xml::events::{BytesEnd, BytesStart, BytesText};

use crate::chat;
use crate::config::Config;
use crate::constants::*;
use crate::context::*;
use crate::dc_tools::*;
use crate::error::Error;
use crate::events::Event;
use crate::job::*;
use crate::message::Message;
use crate::param::*;
use crate::sql;
use crate::stock::StockMessage;

// location handling
#[derive(Debug, Clone, Default)]
pub struct Location {
    pub location_id: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
    pub timestamp: i64,
    pub contact_id: u32,
    pub msg_id: u32,
    pub chat_id: u32,
    pub marker: Option<String>,
    pub independent: u32,
}

impl Location {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Kml {
    pub addr: Option<String>,
    pub locations: Vec<Location>,
    tag: KmlTag,
    pub curr: Location,
}

bitflags! {
    #[derive(Default)]
    struct KmlTag: i32 {
        const UNDEFINED = 0x00;
        const PLACEMARK = 0x01;
        const TIMESTAMP = 0x02;
        const WHEN = 0x04;
        const POINT = 0x08;
        const COORDINATES = 0x10;
    }
}

impl Kml {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn parse(context: &Context, content: impl AsRef<str>) -> Result<Self, Error> {
        ensure!(
            content.as_ref().len() <= (1024 * 1024),
            "A kml-files with {} bytes is larger than reasonably expected.",
            content.as_ref().len()
        );

        let mut reader = quick_xml::Reader::from_str(content.as_ref());
        reader.trim_text(true);

        let mut kml = Kml::new();
        kml.locations = Vec::with_capacity(100);

        let mut buf = Vec::new();

        loop {
            match reader.read_event(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e)) => kml.starttag_cb(e, &reader),
                Ok(quick_xml::events::Event::End(ref e)) => kml.endtag_cb(e),
                Ok(quick_xml::events::Event::Text(ref e)) => kml.text_cb(e, &reader),
                Err(e) => {
                    error!(
                        context,
                        "Location parsing: Error at position {}: {:?}",
                        reader.buffer_position(),
                        e
                    );
                }
                Ok(quick_xml::events::Event::Eof) => break,
                _ => (),
            }
            buf.clear();
        }

        Ok(kml)
    }

    fn text_cb<B: std::io::BufRead>(&mut self, event: &BytesText, reader: &quick_xml::Reader<B>) {
        if self.tag.contains(KmlTag::WHEN) || self.tag.contains(KmlTag::COORDINATES) {
            let val = event.unescape_and_decode(reader).unwrap_or_default();

            let val = val
                .replace("\n", "")
                .replace("\r", "")
                .replace("\t", "")
                .replace(" ", "");

            if self.tag.contains(KmlTag::WHEN) && val.len() >= 19 {
                // YYYY-MM-DDTHH:MM:SSZ
                // 0   4  7  10 13 16 19
                match chrono::NaiveDateTime::parse_from_str(&val, "%Y-%m-%dT%H:%M:%SZ") {
                    Ok(res) => {
                        self.curr.timestamp = res.timestamp();
                        if self.curr.timestamp > time() {
                            self.curr.timestamp = time();
                        }
                    }
                    Err(_err) => {
                        self.curr.timestamp = time();
                    }
                }
            } else if self.tag.contains(KmlTag::COORDINATES) {
                let parts = val.splitn(2, ',').collect::<Vec<_>>();
                if parts.len() == 2 {
                    self.curr.longitude = parts[0].parse().unwrap_or_default();
                    self.curr.latitude = parts[1].parse().unwrap_or_default();
                }
            }
        }
    }

    fn endtag_cb(&mut self, event: &BytesEnd) {
        let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

        if tag == "placemark" {
            if self.tag.contains(KmlTag::PLACEMARK)
                && 0 != self.curr.timestamp
                && 0. != self.curr.latitude
                && 0. != self.curr.longitude
            {
                self.locations
                    .push(std::mem::replace(&mut self.curr, Location::new()));
            }
            self.tag = KmlTag::UNDEFINED;
        };
    }

    fn starttag_cb<B: std::io::BufRead>(
        &mut self,
        event: &BytesStart,
        reader: &quick_xml::Reader<B>,
    ) {
        let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();
        if tag == "document" {
            if let Some(addr) = event.attributes().find(|attr| {
                attr.as_ref()
                    .map(|a| String::from_utf8_lossy(a.key).trim().to_lowercase() == "addr")
                    .unwrap_or_default()
            }) {
                self.addr = addr.unwrap().unescape_and_decode_value(reader).ok();
            }
        } else if tag == "placemark" {
            self.tag = KmlTag::PLACEMARK;
            self.curr.timestamp = 0;
            self.curr.latitude = 0.0;
            self.curr.longitude = 0.0;
            self.curr.accuracy = 0.0
        } else if tag == "timestamp" && self.tag.contains(KmlTag::PLACEMARK) {
            self.tag = KmlTag::PLACEMARK | KmlTag::TIMESTAMP
        } else if tag == "when" && self.tag.contains(KmlTag::TIMESTAMP) {
            self.tag = KmlTag::PLACEMARK | KmlTag::TIMESTAMP | KmlTag::WHEN
        } else if tag == "point" && self.tag.contains(KmlTag::PLACEMARK) {
            self.tag = KmlTag::PLACEMARK | KmlTag::POINT
        } else if tag == "coordinates" && self.tag.contains(KmlTag::POINT) {
            self.tag = KmlTag::PLACEMARK | KmlTag::POINT | KmlTag::COORDINATES;
            if let Some(acc) = event.attributes().find(|attr| {
                attr.as_ref()
                    .map(|a| String::from_utf8_lossy(a.key).trim().to_lowercase() == "accuracy")
                    .unwrap_or_default()
            }) {
                let v = acc
                    .unwrap()
                    .unescape_and_decode_value(reader)
                    .unwrap_or_default();

                self.curr.accuracy = v.trim().parse().unwrap_or_default();
            }
        }
    }
}

// location streaming
pub fn send_locations_to_chat(context: &Context, chat_id: u32, seconds: i64) {
    let now = time();
    let mut msg: Message;
    let is_sending_locations_before: bool;
    if !(seconds < 0 || chat_id <= 9i32 as libc::c_uint) {
        is_sending_locations_before = is_sending_locations_to_chat(context, chat_id);
        if sql::execute(
            context,
            &context.sql,
            "UPDATE chats    \
             SET locations_send_begin=?,        \
             locations_send_until=?  \
             WHERE id=?",
            params![
                if 0 != seconds { now } else { 0 },
                if 0 != seconds { now + seconds } else { 0 },
                chat_id as i32,
            ],
        )
        .is_ok()
        {
            if 0 != seconds && !is_sending_locations_before {
                msg = Message::new(Viewtype::Text);
                msg.text =
                    Some(context.stock_system_msg(StockMessage::MsgLocationEnabled, "", "", 0));
                msg.param.set_int(Param::Cmd, 8);
                chat::send_msg(context, chat_id, &mut msg).unwrap_or_default();
            } else if 0 == seconds && is_sending_locations_before {
                let stock_str =
                    context.stock_system_msg(StockMessage::MsgLocationDisabled, "", "", 0);
                chat::add_device_msg(context, chat_id, stock_str);
            }
            context.call_cb(Event::ChatModified(chat_id));
            if 0 != seconds {
                schedule_MAYBE_SEND_LOCATIONS(context, 0i32);
                job_add(
                    context,
                    Action::MaybeSendLocationsEnded,
                    chat_id as libc::c_int,
                    Params::new(),
                    seconds + 1,
                );
            }
        }
    }
}

#[allow(non_snake_case)]
fn schedule_MAYBE_SEND_LOCATIONS(context: &Context, flags: i32) {
    if 0 != flags & 0x1 || !job_action_exists(context, Action::MaybeSendLocations) {
        job_add(context, Action::MaybeSendLocations, 0, Params::new(), 60);
    };
}

pub fn is_sending_locations_to_chat(context: &Context, chat_id: u32) -> bool {
    context
        .sql
        .exists(
            "SELECT id  FROM chats  WHERE (? OR id=?)   AND locations_send_until>?;",
            params![if chat_id == 0 { 1 } else { 0 }, chat_id as i32, time()],
        )
        .unwrap_or_default()
}

pub fn set(context: &Context, latitude: f64, longitude: f64, accuracy: f64) -> libc::c_int {
    if latitude == 0.0 && longitude == 0.0 {
        return 1;
    }
    let mut continue_streaming = false;

    if let Ok(chats) = context.sql.query_map(
        "SELECT id FROM chats WHERE locations_send_until>?;",
        params![time()],
        |row| row.get::<_, i32>(0),
        |chats| chats.collect::<Result<Vec<_>, _>>().map_err(Into::into),
    ) {
        for chat_id in chats {
            if let Err(err) = context.sql.execute(
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
            ) {
                warn!(context, "failed to store location {:?}", err);
            } else {
                continue_streaming = true;
            }
        }
        if continue_streaming {
            context.call_cb(Event::LocationChanged(Some(1)));
        };
        schedule_MAYBE_SEND_LOCATIONS(context, 0);
    }

    continue_streaming as libc::c_int
}

pub fn get_range(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
    timestamp_from: i64,
    mut timestamp_to: i64,
) -> Vec<Location> {
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

                let loc = Location {
                    location_id: row.get(0)?,
                    latitude: row.get(1)?,
                    longitude: row.get(2)?,
                    accuracy: row.get(3)?,
                    timestamp: row.get(4)?,
                    independent: row.get(5)?,
                    msg_id,
                    contact_id: row.get(7)?,
                    chat_id: row.get(8)?,
                    marker,
                };
                Ok(loc)
            },
            |locations| {
                let mut ret = Vec::new();

                for location in locations {
                    ret.push(location?);
                }
                Ok(ret)
            },
        )
        .unwrap_or_default()
}

fn is_marker(txt: &str) -> bool {
    txt.len() == 1 && !txt.starts_with(' ')
}

pub fn delete_all(context: &Context) -> Result<(), Error> {
    sql::execute(context, &context.sql, "DELETE FROM locations;", params![])?;
    context.call_cb(Event::LocationChanged(None));
    Ok(())
}

pub fn get_kml(context: &Context, chat_id: u32) -> Result<(String, u32), Error> {
    let now = time();
    let mut location_count = 0;
    let mut ret = String::new();
    let mut last_added_location_id = 0;

    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_default();

    let (locations_send_begin, locations_send_until, locations_last_sent) = context.sql.query_row(
        "SELECT locations_send_begin, locations_send_until, locations_last_sent  FROM chats  WHERE id=?;",
        params![chat_id as i32], |row| {
            let send_begin: i64 = row.get(0)?;
            let send_until: i64 = row.get(1)?;
            let last_sent: i64 = row.get(2)?;

            Ok((send_begin, send_until, last_sent))
        })?;

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
                let timestamp = get_kml_timestamp(row.get(4)?);

                Ok((location_id, latitude, longitude, accuracy, timestamp))
            },
            |rows| {
                for row in rows {
                    let (location_id, latitude, longitude, accuracy, timestamp) = row?;
                    ret += &format!(
                        "<Placemark><Timestamp><when>{}</when></Timestamp><Point><coordinates accuracy=\"{}\">{},{}</coordinates></Point></Placemark>\n\x00",
                        timestamp,
                        accuracy,
                        longitude,
                        latitude
                    );
                    location_count += 1;
                    last_added_location_id = location_id as u32;
                }
                Ok(())
            }
        )?;
    }

    ensure!(location_count > 0, "No locations processed");
    ret += "</Document>\n</kml>";

    Ok((ret, last_added_location_id))
}

fn get_kml_timestamp(utc: i64) -> String {
    // Returns a string formatted as YYYY-MM-DDTHH:MM:SSZ. The trailing `Z` indicates UTC.
    chrono::NaiveDateTime::from_timestamp(utc, 0)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

pub fn get_message_kml(timestamp: i64, latitude: f64, longitude: f64) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <kml xmlns=\"http://www.opengis.net/kml/2.2\">\n\
         <Document>\n\
         <Placemark>\
         <Timestamp><when>{}</when></Timestamp>\
         <Point><coordinates>{:.2},{:.2}</coordinates></Point>\
         </Placemark>\n\
         </Document>\n\
         </kml>",
        get_kml_timestamp(timestamp),
        longitude,
        latitude,
    )
}

pub fn set_kml_sent_timestamp(
    context: &Context,
    chat_id: u32,
    timestamp: i64,
) -> Result<(), Error> {
    sql::execute(
        context,
        &context.sql,
        "UPDATE chats SET locations_last_sent=? WHERE id=?;",
        params![timestamp, chat_id as i32],
    )?;

    Ok(())
}

pub fn set_msg_location_id(context: &Context, msg_id: u32, location_id: u32) -> Result<(), Error> {
    sql::execute(
        context,
        &context.sql,
        "UPDATE msgs SET location_id=? WHERE id=?;",
        params![location_id, msg_id as i32],
    )?;

    Ok(())
}

pub fn save(
    context: &Context,
    chat_id: u32,
    contact_id: u32,
    locations: &[Location],
    independent: i32,
) -> Result<u32, Error> {
    ensure!(chat_id > 9, "Invalid chat id");
    context.sql.prepare2(
        "SELECT id FROM locations WHERE timestamp=? AND from_id=?",
        "INSERT INTO locations\
         (timestamp, from_id, chat_id, latitude, longitude, accuracy, independent) \
         VALUES (?,?,?,?,?,?,?);",
        |mut stmt_test, mut stmt_insert, conn| {
            let mut newest_timestamp = 0;
            let mut newest_location_id = 0;

            for location in locations {
                let exists = stmt_test.exists(params![location.timestamp, contact_id as i32])?;

                if 0 != independent || !exists {
                    stmt_insert.execute(params![
                        location.timestamp,
                        contact_id as i32,
                        chat_id as i32,
                        location.latitude,
                        location.longitude,
                        location.accuracy,
                        independent,
                    ])?;

                    if location.timestamp > newest_timestamp {
                        newest_timestamp = location.timestamp;
                        newest_location_id = sql::get_rowid2_with_conn(
                            context,
                            conn,
                            "locations",
                            "timestamp",
                            location.timestamp,
                            "from_id",
                            contact_id as i32,
                        );
                    }
                }
            }
            Ok(newest_location_id)
        },
    )
}

#[allow(non_snake_case)]
pub fn job_do_DC_JOB_MAYBE_SEND_LOCATIONS(context: &Context, _job: &Job) {
    let now = time();
    let mut continue_streaming: libc::c_int = 1;
    info!(
        context,
        " ----------------- MAYBE_SEND_LOCATIONS -------------- ",
    );

    if let Ok(rows) = context.sql.query_map(
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
            rows.filter_map(|v| v.transpose())
                .collect::<Result<Vec<_>, _>>()
                .map_err(Into::into)
        },
    ) {
        let msgs = context
            .sql
            .prepare(
                "SELECT id \
                 FROM locations \
                 WHERE from_id=? \
                 AND timestamp>=? \
                 AND timestamp>? \
                 AND independent=0 \
                 ORDER BY timestamp;",
                |mut stmt_locations, _| {
                    let msgs = rows
                        .into_iter()
                        .filter_map(|(chat_id, locations_send_begin, locations_last_sent)| {
                            if !stmt_locations
                                .exists(params![1, locations_send_begin, locations_last_sent,])
                                .unwrap_or_default()
                            {
                                // if there is no new location, there's nothing to send.
                                // however, maybe we want to bypass this test eg. 15 minutes
                                None
                            } else {
                                // pending locations are attached automatically to every message,
                                // so also to this empty text message.
                                // DC_CMD_LOCATION is only needed to create a nicer subject.
                                //
                                // for optimisation and to avoid flooding the sending queue,
                                // we could sending these messages only if we're really online.
                                // the easiest way to determine this, is to check for an empty message queue.
                                // (might not be 100%, however, as positions are sent combined later
                                // and dc_set_location() is typically called periodically, this is ok)
                                let mut msg = Message::new(Viewtype::Text);
                                msg.hidden = true;
                                msg.param.set_int(Param::Cmd, 9);
                                Some((chat_id, msg))
                            }
                        })
                        .collect::<Vec<_>>();
                    Ok(msgs)
                },
            )
            .unwrap_or_default(); // TODO: Better error handling

        for (chat_id, mut msg) in msgs.into_iter() {
            // TODO: better error handling
            chat::send_msg(context, chat_id as u32, &mut msg).unwrap_or_default();
        }
    }
    if 0 != continue_streaming {
        schedule_MAYBE_SEND_LOCATIONS(context, 0x1);
    }
}

#[allow(non_snake_case)]
pub fn job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(context: &Context, job: &mut Job) {
    // this function is called when location-streaming _might_ have ended for a chat.
    // the function checks, if location-streaming is really ended;
    // if so, a device-message is added if not yet done.

    let chat_id = job.foreign_id;

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
                    let stock_str = context.stock_system_msg(StockMessage::MsgLocationDisabled, "", "", 0);
                    chat::add_device_msg(context, chat_id, stock_str);
                    context.call_cb(Event::ChatModified(chat_id));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::dummy_context;

    #[test]
    fn test_kml_parse() {
        let context = dummy_context();

        let xml =
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document addr=\"user@example.org\">\n<Placemark><Timestamp><when>2019-03-06T21:09:57Z</when></Timestamp><Point><coordinates accuracy=\"32.000000\">9.423110,53.790302</coordinates></Point></Placemark>\n<PlaceMARK>\n<Timestamp><WHEN > \n\t2018-12-13T22:11:12Z\t</WHEN></Timestamp><Point><coordinates aCCuracy=\"2.500000\"> 19.423110 \t , \n 63.790302\n </coordinates></Point></PlaceMARK>\n</Document>\n</kml>";

        let kml = Kml::parse(&context.ctx, &xml).expect("parsing failed");

        assert!(kml.addr.is_some());
        assert_eq!(kml.addr.as_ref().unwrap(), "user@example.org",);

        let locations_ref = &kml.locations;
        assert_eq!(locations_ref.len(), 2);

        assert!(locations_ref[0].latitude > 53.6f64);
        assert!(locations_ref[0].latitude < 53.8f64);
        assert!(locations_ref[0].longitude > 9.3f64);
        assert!(locations_ref[0].longitude < 9.5f64);
        assert!(locations_ref[0].accuracy > 31.9f64);
        assert!(locations_ref[0].accuracy < 32.1f64);
        assert_eq!(locations_ref[0].timestamp, 1551906597);

        assert!(locations_ref[1].latitude > 63.6f64);
        assert!(locations_ref[1].latitude < 63.8f64);
        assert!(locations_ref[1].longitude > 19.3f64);
        assert!(locations_ref[1].longitude < 19.5f64);
        assert!(locations_ref[1].accuracy > 2.4f64);
        assert!(locations_ref[1].accuracy < 2.6f64);
        assert_eq!(locations_ref[1].timestamp, 1544739072);
    }
}

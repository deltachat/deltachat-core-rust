use std::collections::{HashMap, HashSet};

use deltachat_derive::{FromSql, ToSql};
use mailparse::MailHeaderMap;

use crate::aheader::Aheader;
use crate::blob::BlobObject;
use crate::config::Config;
use crate::constants::Viewtype;
use crate::contact::*;
use crate::context::Context;
use crate::dc_simplify::*;
use crate::dc_tools::*;
use crate::e2ee;
use crate::error::Result;
use crate::job::{job_add, Action};
use crate::location;
use crate::message;
use crate::message::MsgId;
use crate::param::*;
use crate::peerstate::Peerstate;
use crate::securejoin::handle_degrade_event;
use crate::stock::StockMessage;

#[derive(Debug)]
pub struct MimeParser<'a> {
    pub context: &'a Context,
    pub parts: Vec<Part>,
    pub header: HashMap<String, String>,
    pub subject: Option<String>,
    pub is_send_by_messenger: bool,
    pub decrypting_failed: bool,
    pub encrypted: bool,
    pub signatures: HashSet<String>,
    pub gossipped_addr: HashSet<String>,
    pub is_forwarded: bool,
    pub is_system_message: SystemMessage,
    pub location_kml: Option<location::Kml>,
    pub message_kml: Option<location::Kml>,
    reports: Vec<Report>,
    parsed_header_protected: bool,
    mdns_enabled: bool,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql)]
#[repr(i32)]
pub enum SystemMessage {
    Unknown = 0,
    GroupNameChanged = 2,
    GroupImageChanged = 3,
    MemberAddedToGroup = 4,
    MemberRemovedFromGroup = 5,
    AutocryptSetupMessage = 6,
    SecurejoinMessage = 7,
    LocationStreamingEnabled = 8,
    LocationOnly = 9,
}

impl Default for SystemMessage {
    fn default() -> Self {
        SystemMessage::Unknown
    }
}

const DC_MIMETYPE_MP_ALTERNATIVE: i32 = 10;
const DC_MIMETYPE_MP_RELATED: i32 = 20;
const DC_MIMETYPE_MP_MIXED: i32 = 30;
const DC_MIMETYPE_MP_NOT_DECRYPTABLE: i32 = 40;
const DC_MIMETYPE_MP_REPORT: i32 = 45;
const DC_MIMETYPE_MP_SIGNED: i32 = 46;
const DC_MIMETYPE_MP_OTHER: i32 = 50;
const DC_MIMETYPE_TEXT_PLAIN: i32 = 60;
const DC_MIMETYPE_TEXT_HTML: i32 = 70;
const DC_MIMETYPE_IMAGE: i32 = 80;
const DC_MIMETYPE_AUDIO: i32 = 90;
const DC_MIMETYPE_VIDEO: i32 = 100;
const DC_MIMETYPE_FILE: i32 = 110;
const DC_MIMETYPE_AC_SETUP_FILE: i32 = 111;

impl<'a> MimeParser<'a> {
    pub fn from_bytes(context: &'a Context, body: &[u8]) -> Result<Self> {
        let mail = mailparse::parse_mail(body)?;
        let mdns_enabled = context.get_config_bool(Config::MdnsEnabled);

        let mut parser = MimeParser {
            parts: Vec::new(),
            header: Default::default(),
            parsed_header_protected: false,
            subject: None,
            is_send_by_messenger: false,
            decrypting_failed: false,
            encrypted: false,
            signatures: Default::default(),
            gossipped_addr: Default::default(),
            is_forwarded: false,
            context,
            reports: Vec::new(),
            is_system_message: SystemMessage::Unknown,
            location_kml: None,
            message_kml: None,
            mdns_enabled,
        };

        let mail_raw;
        let mail = match e2ee::try_decrypt(parser.context, &mail) {
            Ok((raw, signatures, message_time)) => {
                // Valid autocrypt message, encrypted
                parser.encrypted = raw.is_some();
                parser.signatures = signatures;

                if let Some(raw) = raw {
                    mail_raw = raw;
                    let decrypted_mail = mailparse::parse_mail(&mail_raw)?;

                    // we have a decrypted mail, that is valid, check for gossip headers

                    let gossip_headers =
                        decrypted_mail.headers.get_all_values("Autocrypt-Gossip")?;
                    if !gossip_headers.is_empty() {
                        parser.gossipped_addr =
                            update_gossip_peerstates(context, message_time, &mail, gossip_headers)?;
                    }

                    decrypted_mail
                } else {
                    // Message was not encrypted
                    mail
                }
            }
            Err(err) => {
                // continue with the current, still encrypted, mime tree.
                // unencrypted parts will be replaced by an error message
                // that is added as "the message" to the chat then.
                //
                // if we just return here, the header is missing
                // and the caller cannot display the message
                // and try to assign the message to a chat
                warn!(parser.context, "decryption failed: {}", err);
                mail
            }
        };

        parser.hash_header(&mail.headers);
        parser.parse_mime_recursive(&mail)?;
        parser.parse_headers()?;

        Ok(parser)
    }

    fn parse_headers(&mut self) -> Result<()> {
        if let Some(field) = self.lookup_field("Subject") {
            self.subject = Some(field.clone());
        }

        if let Some(_) = self.lookup_field("Chat-Version") {
            self.is_send_by_messenger = true
        }

        if let Some(_) = self.lookup_field("Autocrypt-Setup-Message") {
            let has_setup_file = self
                .parts
                .iter()
                .any(|p| p.mimetype == DC_MIMETYPE_AC_SETUP_FILE);

            if has_setup_file {
                self.is_system_message = SystemMessage::AutocryptSetupMessage;

                // TODO: replace the following code with this
                // once drain_filter stabilizes.
                //
                // See https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain_filter
                // and https://github.com/rust-lang/rust/issues/43244
                //
                // mimeparser
                //    .parts
                //    .drain_filter(|part| part.int_mimetype != 111)
                //    .for_each(|part| dc_mimepart_unref(part));

                let mut i = 0;
                while i != self.parts.len() {
                    if self.parts[i].mimetype != 111 {
                        self.parts.remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
        } else if let Some(value) = self.lookup_field("Chat-Content") {
            if value == "location-streaming-enabled" {
                self.is_system_message = SystemMessage::LocationStreamingEnabled;
            }
        }
        if let Some(_) = self.lookup_field("Chat-Group-Image") {
            if !self.parts.is_empty() {
                let textpart = &self.parts[0];
                if textpart.typ == Viewtype::Text && self.parts.len() >= 2 {
                    let imgpart = &mut self.parts[1];
                    if imgpart.typ == Viewtype::Image {
                        imgpart.is_meta = true;
                    }
                }
            }
        }
        if self.is_send_by_messenger && self.parts.len() == 2 {
            let need_drop = {
                let textpart = &self.parts[0];
                let filepart = &self.parts[1];
                textpart.typ == Viewtype::Text
                    && (filepart.typ == Viewtype::Image
                        || filepart.typ == Viewtype::Gif
                        || filepart.typ == Viewtype::Sticker
                        || filepart.typ == Viewtype::Audio
                        || filepart.typ == Viewtype::Voice
                        || filepart.typ == Viewtype::Video
                        || filepart.typ == Viewtype::File)
                    && !filepart.is_meta
            };

            if need_drop {
                let mut filepart = self.parts.swap_remove(1);

                // insert new one
                filepart.msg = self.parts[0].msg.as_ref().map(|s| s.to_string());

                // forget the one we use now
                self.parts[0].msg = None;

                // swap new with old
                std::mem::replace(&mut self.parts[0], filepart);
            }
        }
        if let Some(ref subject) = self.subject {
            let mut prepend_subject: libc::c_int = 1i32;
            if !self.decrypting_failed {
                let colon = subject.find(':');
                if colon == Some(2)
                    || colon == Some(3)
                    || self.is_send_by_messenger
                    || subject.contains("Chat:")
                {
                    prepend_subject = 0i32
                }
            }
            if 0 != prepend_subject {
                let subj = if let Some(n) = subject.find('[') {
                    &subject[0..n]
                } else {
                    subject
                }
                .trim();

                if !subj.is_empty() {
                    for part in self.parts.iter_mut() {
                        if part.typ == Viewtype::Text {
                            let new_txt = format!(
                                "{} – {}",
                                subj,
                                part.msg.as_ref().expect("missing msg part")
                            );
                            part.msg = Some(new_txt);
                            break;
                        }
                    }
                }
            }
        }
        if self.is_forwarded {
            for part in self.parts.iter_mut() {
                part.param.set_int(Param::Forwarded, 1);
            }
        }
        if self.parts.len() == 1 {
            if self.parts[0].typ == Viewtype::Audio {
                if let Some(_) = self.lookup_field("Chat-Voice-Message") {
                    let part_mut = &mut self.parts[0];
                    part_mut.typ = Viewtype::Voice;
                }
            }
            if self.parts[0].typ == Viewtype::Image {
                if let Some(value) = self.lookup_field("Chat-Content") {
                    if value == "sticker" {
                        let part_mut = &mut self.parts[0];
                        part_mut.typ = Viewtype::Sticker;
                    }
                }
            }
            let part = &self.parts[0];
            if part.typ == Viewtype::Audio
                || part.typ == Viewtype::Voice
                || part.typ == Viewtype::Video
            {
                if let Some(field_0) = self.lookup_field("Chat-Duration") {
                    let duration_ms = field_0.parse().unwrap_or_default();
                    if duration_ms > 0 && duration_ms < 24 * 60 * 60 * 1000 {
                        let part_mut = &mut self.parts[0];
                        part_mut.param.set_int(Param::Duration, duration_ms);
                    }
                }
            }
        }
        if !self.decrypting_failed {
            if let Some(dn_field) = self.lookup_field("Chat-Disposition-Notification-To") {
                if self.get_last_nonmeta().is_some() {
                    let addrs = mailparse::addrparse(&dn_field).unwrap();

                    if let Some(dn_to_addr) = addrs.first() {
                        if let Some(from_field) = self.lookup_field("From") {
                            let value = from_field;
                            let from_addrs = mailparse::addrparse(&value).unwrap();
                            if let Some(from_addr) = from_addrs.first() {
                                if from_addr == dn_to_addr {
                                    if let Some(part_4) = self.get_last_nonmeta_mut() {
                                        part_4.param.set_int(Param::WantsMdn, 1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        /* Cleanup - and try to create at least an empty part if there are no parts yet */
        if self.get_last_nonmeta().is_none() && self.reports.is_empty() {
            let mut part_5 = Part::default();
            part_5.typ = Viewtype::Text;
            part_5.msg = Some("".into());

            if let Some(ref subject) = self.subject {
                if !self.is_send_by_messenger {
                    part_5.msg = Some(subject.to_string())
                }
            }
            self.parts.push(part_5);
        }

        Ok(())
    }

    pub fn get_last_nonmeta(&self) -> Option<&Part> {
        self.parts.iter().rev().find(|part| !part.is_meta)
    }

    pub fn get_last_nonmeta_mut(&mut self) -> Option<&mut Part> {
        self.parts.iter_mut().rev().find(|part| !part.is_meta)
    }

    pub fn lookup_field(&self, field_name: &str) -> Option<&String> {
        self.header.get(&field_name.to_lowercase())
    }

    fn parse_mime_recursive(&mut self, mail: &mailparse::ParsedMail<'_>) -> Result<bool> {
        if mail.ctype.params.get("protected-headers").is_some() {
            if mail.ctype.mimetype == "text/rfc822-headers" {
                info!(
                    self.context,
                    "Protected headers found in text/rfc822-headers attachment: Will be ignored.",
                );
                return Ok(false);
            }

            if !self.parsed_header_protected {
                // use the most outer protected header - this is typically
                // created in sync with the normal, unprotected header

                self.parsed_header_protected = true;
                self.hash_header(&mail.headers);
            } else {
                info!(
                    self.context,
                    "Protected headers found in MIME header: Will be ignored as we already found an outer one."
                );
            }
        }

        // multiple = multipart/ or message/
        enum MimeS {
            Multiple,
            Single,
            Message,
        }

        let mimetype = mail.ctype.mimetype.to_lowercase();

        let m = if mimetype.starts_with("multipart") {
            if mail.ctype.params.get("boundary").is_some() {
                MimeS::Multiple
            } else {
                MimeS::Single
            }
        } else if mimetype.starts_with("message") {
            if mimetype == "message/rfc822" {
                MimeS::Message
            } else {
                MimeS::Single
            }
        } else {
            MimeS::Single
        };

        match m {
            MimeS::Multiple => self.handle_multiple(mail),
            MimeS::Message => {
                let raw = mail.get_body_raw()?;
                if raw.is_empty() {
                    return Ok(false);
                }
                let mail = mailparse::parse_mail(&raw).unwrap();

                self.parse_mime_recursive(&mail)
            }
            MimeS::Single => self.add_single_part_if_known(mail),
        }
    }

    fn handle_multiple(&mut self, mail: &mailparse::ParsedMail<'_>) -> Result<bool> {
        let mut any_part_added = false;
        match mailmime_get_mime_type(mail) {
            /* Most times, mutlipart/alternative contains true alternatives
            as text/plain and text/html.  If we find a multipart/mixed
            inside mutlipart/alternative, we use this (happens eg in
            apple mail: "plaintext" as an alternative to "html+PDF attachment") */
            (DC_MIMETYPE_MP_ALTERNATIVE, _) => {
                for cur_data in &mail.subparts {
                    if mailmime_get_mime_type(cur_data).0 == DC_MIMETYPE_MP_MIXED {
                        any_part_added = self.parse_mime_recursive(cur_data)?;
                        break;
                    }
                }
                if !any_part_added {
                    /* search for text/plain and add this */
                    for cur_data in &mail.subparts {
                        if mailmime_get_mime_type(cur_data).0 == DC_MIMETYPE_TEXT_PLAIN {
                            any_part_added = self.parse_mime_recursive(cur_data)?;
                            break;
                        }
                    }
                }
                if !any_part_added {
                    /* `text/plain` not found - use the first part */
                    for cur_part in &mail.subparts {
                        if self.parse_mime_recursive(cur_part)? {
                            any_part_added = true;
                            break;
                        }
                    }
                }
            }
            (DC_MIMETYPE_MP_RELATED, _) => {
                /* add the "root part" - the other parts may be referenced which is
                not interesting for us (eg. embedded images) we assume he "root part"
                being the first one, which may not be always true ...
                however, most times it seems okay. */
                if let Some(first) = mail.subparts.iter().next() {
                    any_part_added = self.parse_mime_recursive(first)?;
                }
            }
            (DC_MIMETYPE_MP_NOT_DECRYPTABLE, _) => {
                let mut part = Part::default();
                part.typ = Viewtype::Text;
                let msg_body = self.context.stock_str(StockMessage::CantDecryptMsgBody);

                let txt = format!("[{}]", msg_body);
                part.msg_raw = Some(txt.clone());
                part.msg = Some(txt);

                self.parts.push(part);
                any_part_added = true;
                self.decrypting_failed = true;
            }
            (DC_MIMETYPE_MP_SIGNED, _) => {
                /* RFC 1847: "The multipart/signed content type
                contains exactly two body parts.  The first body
                part is the body part over which the digital signature was created [...]
                The second body part contains the control information necessary to
                verify the digital signature." We simpliy take the first body part and
                skip the rest.  (see
                https://k9mail.github.io/2016/11/24/OpenPGP-Considerations-Part-I.html
                for background information why we use encrypted+signed) */
                if let Some(first) = mail.subparts.iter().next() {
                    any_part_added = self.parse_mime_recursive(first)?;
                }
            }
            (DC_MIMETYPE_MP_REPORT, _) => {
                /* RFC 6522: the first part is for humans, the second for machines */
                if mail.subparts.len() >= 2 {
                    let ct = mail.get_content_disposition()?;
                    if let Some(report_type) = ct.params.get("report-type") {
                        if report_type == "disposition-notification" {
                            if let Some(report) = self.process_report(mail)? {
                                self.reports.push(report);
                            }
                        } else {
                            /* eg. `report-type=delivery-status`;
                            maybe we should show them as a little error icon */
                            if let Some(first) = mail.subparts.iter().next() {
                                any_part_added = self.parse_mime_recursive(first)?;
                            }
                        }
                    }
                }
            }
            _ => {
                /* eg. DC_MIMETYPE_MP_MIXED - add all parts (in fact,
                AddSinglePartIfKnown() later check if the parts are really supported)
                HACK: the following lines are a hack for clients who use
                multipart/mixed instead of multipart/alternative for
                combined text/html messages (eg. Stock Android "Mail" does so).
                So, if we detect such a message below, we skip the HTML
                part.  However, not sure, if there are useful situations to use
                plain+html in multipart/mixed - if so, we should disable the hack. */
                let mut skip_part = -1;
                let mut html_part = -1;
                let mut plain_cnt = 0;
                let mut html_cnt = 0;

                for (i, cur_data) in mail.subparts.iter().enumerate() {
                    match mailmime_get_mime_type(cur_data) {
                        (DC_MIMETYPE_TEXT_PLAIN, _) => {
                            plain_cnt += 1;
                        }
                        (DC_MIMETYPE_TEXT_HTML, _) => {
                            html_part = i as isize;
                            html_cnt += 1;
                        }
                        _ => {}
                    }
                }
                if plain_cnt == 1 && html_cnt == 1 {
                    warn!(
                        self.context,
                        "HACK: multipart/mixed message found with PLAIN and HTML, we\'ll skip the HTML part as this seems to be unwanted."
                    );
                    skip_part = html_part;
                }

                for (i, cur_data) in mail.subparts.iter().enumerate() {
                    if i as isize != skip_part {
                        if self.parse_mime_recursive(cur_data)? {
                            any_part_added = true;
                        }
                    }
                }
            }
        }

        Ok(any_part_added)
    }

    fn add_single_part_if_known(&mut self, mail: &mailparse::ParsedMail<'_>) -> Result<bool> {
        // return true if a part was added
        let (mime_type, msg_type) = mailmime_get_mime_type(mail);
        let raw_mime = mail.ctype.mimetype.to_lowercase();

        if !raw_mime.starts_with("text") {
            // MAILMIME_DATA_FILE indicates, the data is in a file; AFAIK this is not used on parsing
            return Ok(false);
        }

        let old_part_count = self.parts.len();

        // regard `Content-Transfer-Encoding:`
        match mime_type {
            DC_MIMETYPE_TEXT_PLAIN | DC_MIMETYPE_TEXT_HTML => {
                let decoded_data = match mail.get_body() {
                    Ok(decoded_data) => decoded_data,
                    Err(err) => {
                        warn!(self.context, "Invalid body parsed {:?}", err);
                        // Note that it's not always an error - might be no data
                        return Ok(false);
                    }
                };

                // check header directly as is_send_by_messenger is not yet set up
                let is_msgrmsg = self.lookup_field("Chat-Version").is_some();

                let mut simplifier = Simplify::new();
                let simplified_txt = if decoded_data.is_empty() {
                    "".into()
                } else {
                    let is_html = mime_type == DC_MIMETYPE_TEXT_HTML;
                    simplifier.simplify(&decoded_data, is_html, is_msgrmsg)
                };

                if !simplified_txt.is_empty() {
                    let mut part = Part::default();
                    part.typ = Viewtype::Text;
                    part.mimetype = mime_type;
                    part.msg = Some(simplified_txt);
                    part.msg_raw = Some(decoded_data);
                    self.do_add_single_part(part);
                }

                if simplifier.is_forwarded {
                    self.is_forwarded = true;
                }
            }
            DC_MIMETYPE_IMAGE
            | DC_MIMETYPE_AUDIO
            | DC_MIMETYPE_VIDEO
            | DC_MIMETYPE_FILE
            | DC_MIMETYPE_AC_SETUP_FILE => {
                // try to get file name from
                //    `Content-Disposition: ... filename*=...`
                // or `Content-Disposition: ... filename*0*=... filename*1*=... filename*2*=...`
                // or `Content-Disposition: ... filename=...`

                let ct = mail.get_content_disposition()?;
                let mut desired_filename = ct
                    .params
                    .iter()
                    .filter(|(key, _value)| key.starts_with("filename"))
                    .fold(String::new(), |mut acc, (_key, value)| {
                        acc += value;
                        acc
                    });

                if desired_filename.is_empty() {
                    if let Some(param) = ct.params.get("name") {
                        // might be a wrongly encoded filename
                        desired_filename = param.to_string();
                    }
                }

                // if there is still no filename, guess one
                if desired_filename.is_empty() {
                    if let Some(subtype) = mail.ctype.mimetype.split('/').skip(1).next() {
                        desired_filename = format!("file.{}", subtype,);
                    } else {
                        return Ok(false);
                    }
                }
                self.do_add_single_file_part(
                    msg_type,
                    mime_type,
                    &raw_mime,
                    &mail.get_body_raw()?,
                    &desired_filename,
                );
            }
            _ => {}
        }

        // add object? (we do not add all objects, eg. signatures etc. are ignored)
        Ok(self.parts.len() > old_part_count)
    }

    fn do_add_single_file_part(
        &mut self,
        msg_type: Viewtype,
        mime_type: libc::c_int,
        raw_mime: &String,
        decoded_data: &[u8],
        filename: &str,
    ) {
        if decoded_data.is_empty() {
            return;
        }
        // treat location/message kml file attachments specially
        if filename.ends_with(".kml") {
            // XXX what if somebody sends eg an "location-highlights.kml"
            // attachment unrelated to location streaming?
            if filename.starts_with("location") || filename.starts_with("message") {
                let parsed = location::Kml::parse(self.context, decoded_data)
                    .map_err(|err| {
                        warn!(self.context, "failed to parse kml part: {}", err);
                    })
                    .ok();
                if filename.starts_with("location") {
                    self.location_kml = parsed;
                } else {
                    self.message_kml = parsed;
                }
                return;
            }
        }
        /* we have a regular file attachment,
        write decoded data to new blob object */

        let blob = match BlobObject::create(self.context, filename, decoded_data) {
            Ok(blob) => blob,
            Err(err) => {
                error!(
                    self.context,
                    "Could not add blob for mime part {}, error {}", filename, err
                );
                return;
            }
        };

        /* create and register Mime part referencing the new Blob object */
        let mut part = Part::default();
        part.typ = msg_type;
        part.mimetype = mime_type;
        part.bytes = decoded_data.len() as libc::c_int;
        part.param.set(Param::File, blob.as_name());
        part.param.set(Param::MimeType, raw_mime);

        if mime_type == DC_MIMETYPE_IMAGE {
            if let Ok((width, height)) = dc_get_filemeta(decoded_data) {
                part.param.set_int(Param::Width, width as i32);
                part.param.set_int(Param::Height, height as i32);
            }
        }
        self.do_add_single_part(part);
    }

    fn do_add_single_part(&mut self, mut part: Part) {
        if self.encrypted {
            if self.signatures.len() > 0 {
                part.param.set_int(Param::GuaranteeE2ee, 1);
            } else {
                // XXX if the message was encrypted but not signed
                // it's not neccessarily an error we need to signal.
                // we could just treat it as if it was not encrypted.
                part.param.set_int(Param::ErroneousE2ee, 0x2);
            }
        }
        self.parts.push(part);
    }

    pub fn is_mailinglist_message(&self) -> bool {
        if let Some(_) = self.lookup_field("List-Id") {
            return true;
        }

        if let Some(precedence) = self.lookup_field("Precedence") {
            if precedence == "list" || precedence == "bulk" {
                return true;
            }
        }

        false
    }

    pub fn sender_equals_recipient(&self) -> bool {
        /* get From: and check there is exactly one sender */
        if let Some(field) = self.lookup_field("From") {
            if let Ok(addrs) = mailparse::addrparse(field) {
                if addrs.len() != 1 {
                    return false;
                }
                if let mailparse::MailAddr::Single(ref info) = addrs[0] {
                    let from_addr_norm = addr_normalize(&info.addr);
                    let recipients = get_recipients(self.header.iter());
                    if recipients.len() == 1 && recipients.contains(from_addr_norm) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn repl_msg_by_error(&mut self, error_msg: impl AsRef<str>) {
        if self.parts.is_empty() {
            return;
        }

        let part = &mut self.parts[0];
        part.typ = Viewtype::Text;
        part.msg = Some(format!("[{}]", error_msg.as_ref()));
        self.parts.truncate(1);

        assert_eq!(self.parts.len(), 1);
    }

    pub fn get_rfc724_mid(&self) -> Option<String> {
        // get Message-ID from header
        if let Some(field) = self.lookup_field("Message-ID") {
            return parse_message_id(field);
        }
        None
    }

    fn hash_header(&mut self, fields: &[mailparse::MailHeader<'_>]) {
        for field in fields {
            if let Ok(key) = field.get_key() {
                // lowercasing all headers is technically not correct, but makes things work better
                let key = key.to_lowercase();
                if !self.header.contains_key(&key) || // key already exists, only overwrite known types (protected headers)
                    is_known(&key) || key.starts_with("chat-")
                {
                    if let Ok(value) = field.get_value() {
                        self.header.insert(key, value);
                    }
                }
            }
        }
    }

    fn process_report(&self, report: &mailparse::ParsedMail<'_>) -> Result<Option<Report>> {
        let ct = report.get_content_disposition()?;
        let report_type = ct.params.get("report-type");
        if report_type.is_none() {
            return Ok(None);
        }
        let report_type = report_type.unwrap();
        if report_type != "disposition-notification" || report.subparts.len() < 2 {
            // the first part is for humans, the second for machines
            return Ok(None);
        }

        // to get a clear functionality, do not show incoming MDNs if the options is disabled
        if !self.mdns_enabled {
            return Ok(None);
        }
        // 1. get content
        let raw = report.subparts[1].get_body_raw()?;
        let report_details = mailparse::parse_mail(&raw)?;

        // 2. parse as mailheaders
        let report_body = report_details.get_body_raw()?;
        let (report_fields, _) = mailparse::parse_headers(&report_body)?;

        // 3. retrieve information

        // must be present
        if let Some(_disposition) = report_fields.get_first_value("Disposition").ok().flatten() {
            if let Some(original_message_id) = report_fields
                .get_first_value("Original-Message-ID")
                .ok()
                .flatten()
                .and_then(|v| parse_message_id(&v))
            {
                return Ok(Some(Report {
                    original_message_id,
                }));
            }
        }

        Ok(None)
    }

    // Handle reports (mainly MDNs)
    pub fn handle_reports(
        &self,
        from_id: u32,
        sent_timestamp: i64,
        rr_event_to_send: &mut Vec<(u32, MsgId)>,
        server_folder: impl AsRef<str>,
        server_uid: u32,
    ) {
        for report in &self.reports {
            let mut mdn_consumed = false;

            if let Some((chat_id, msg_id)) = message::mdn_from_ext(
                self.context,
                from_id,
                &report.original_message_id,
                sent_timestamp,
            ) {
                rr_event_to_send.push((chat_id, msg_id));
                mdn_consumed = true;
            }

            if self.is_send_by_messenger || mdn_consumed {
                let mut param = Params::new();
                param.set(Param::ServerFolder, server_folder.as_ref());
                param.set_int(Param::ServerUid, server_uid as i32);
                if self.is_send_by_messenger && self.context.get_config_bool(Config::MvboxMove) {
                    param.set_int(Param::AlsoMove, 1);
                }
                job_add(self.context, Action::MarkseenMdnOnImap, 0, param, 0);
            }
        }
    }
}

fn update_gossip_peerstates(
    context: &Context,
    message_time: i64,
    mail: &mailparse::ParsedMail<'_>,
    gossip_headers: Vec<String>,
) -> Result<HashSet<String>> {
    // XXX split the parsing from the modification part
    let mut recipients: Option<HashSet<String>> = None;
    let mut gossipped_addr: HashSet<String> = Default::default();

    for value in &gossip_headers {
        let gossip_header = value.parse::<Aheader>();

        if let Ok(ref header) = gossip_header {
            if recipients.is_none() {
                recipients = Some(get_recipients(mail.headers.iter().map(|v| {
                    // TODO: error handling
                    (v.get_key().unwrap(), v.get_value().unwrap())
                })));
            }
            if recipients.as_ref().unwrap().contains(&header.addr) {
                let mut peerstate = Peerstate::from_addr(context, &context.sql, &header.addr);
                if let Some(ref mut peerstate) = peerstate {
                    peerstate.apply_gossip(header, message_time);
                    peerstate.save_to_db(&context.sql, false)?;
                } else {
                    let p = Peerstate::from_gossip(context, header, message_time);
                    p.save_to_db(&context.sql, true)?;
                    peerstate = Some(p);
                }
                if let Some(peerstate) = peerstate {
                    if peerstate.degrade_event.is_some() {
                        handle_degrade_event(context, &peerstate)?;
                    }
                }

                gossipped_addr.insert(header.addr.clone());
            } else {
                info!(
                    context,
                    "Ignoring gossipped \"{}\" as the address is not in To/Cc list.", &header.addr,
                );
            }
        }
    }

    Ok(gossipped_addr)
}

#[derive(Debug)]
struct Report {
    original_message_id: String,
}

fn parse_message_id(field: &str) -> Option<String> {
    if let Ok(addrs) = mailparse::addrparse(field) {
        // Assume the message id is a single id in the form of <id>
        if let mailparse::MailAddr::Single(mailparse::SingleInfo { ref addr, .. }) = addrs[0] {
            return Some(addr.clone());
        }
    }
    None
}

fn is_known(key: &str) -> bool {
    match key {
        "return-path" | "date" | "from" | "sender" | "reply-to" | "to" | "cc" | "bcc"
        | "message-id" | "in-reply-to" | "references" | "subject" => true,
        _ => false,
    }
}

#[derive(Default, Debug, Clone)]
pub struct Part {
    pub typ: Viewtype,
    pub is_meta: bool,
    pub mimetype: i32,
    pub msg: Option<String>,
    pub msg_raw: Option<String>,
    pub bytes: i32,
    pub param: Params,
}

fn mailmime_get_mime_type(mail: &mailparse::ParsedMail<'_>) -> (libc::c_int, Viewtype) {
    let unknown_type = (0, Viewtype::Unknown);

    let mimetype = mail.ctype.mimetype.to_lowercase();
    let mut parts = mimetype.split('/');
    let typ = parts.next().expect("invalid mimetype");
    let subtype = parts.next().unwrap_or_default();

    match typ {
        "text" => {
            if !mailmime_is_attachment_disposition(mail) {
                if subtype == "plain" {
                    return (DC_MIMETYPE_TEXT_PLAIN, Viewtype::Text);
                }
                if subtype == "html" {
                    return (DC_MIMETYPE_TEXT_HTML, Viewtype::Text);
                }
            }

            (DC_MIMETYPE_FILE, Viewtype::File)
        }
        "image" => {
            let msg_type = match subtype {
                "gif" => Viewtype::Gif,
                "svg+xml" => {
                    return (DC_MIMETYPE_FILE, Viewtype::File);
                }
                _ => Viewtype::Image,
            };

            (DC_MIMETYPE_IMAGE, msg_type)
        }
        "audio" => (DC_MIMETYPE_AUDIO, Viewtype::Audio),
        "video" => (DC_MIMETYPE_VIDEO, Viewtype::Video),
        "multipart" => {
            let mime_type = match subtype {
                "alternative" => DC_MIMETYPE_MP_ALTERNATIVE,
                "related" => DC_MIMETYPE_MP_RELATED,
                "encrypted" => {
                    // maybe try_decrypt failed to decrypt
                    // or it wasn't in proper Autocrypt format
                    DC_MIMETYPE_MP_NOT_DECRYPTABLE
                }
                "signed" => DC_MIMETYPE_MP_SIGNED,
                "mixed" => DC_MIMETYPE_MP_MIXED,
                "report" => DC_MIMETYPE_MP_REPORT,
                _ => DC_MIMETYPE_MP_OTHER,
            };

            (mime_type, Viewtype::Unknown)
        }
        "message" => {
            // Enacapsulated messages, see https://www.w3.org/Protocols/rfc1341/7_3_Message.html
            // Also used as part "message/disposition-notification" of "multipart/report", which, however, will
            // be handled separatedly.
            // I've not seen any messages using this, so we do not attach these parts (maybe they're used to attach replies,
            // which are unwanted at all).
            // For now, we skip these parts at all; if desired, we could return DC_MIMETYPE_FILE/DC_MSG_FILE
            // for selected and known subparts.
            unknown_type
        }
        "application" => {
            if subtype == "autocrypt-setup" {
                return (DC_MIMETYPE_AC_SETUP_FILE, Viewtype::File);
            }

            (DC_MIMETYPE_FILE, Viewtype::File)
        }
        _ => unknown_type,
    }
}

fn mailmime_is_attachment_disposition(mail: &mailparse::ParsedMail<'_>) -> bool {
    if let Some(ct) = mail.ctype.params.get("Content-Disposition") {
        return ct.to_lowercase().starts_with("attachment");
    }

    false
}

// returned addresses are normalized.
fn get_recipients<'a, S: AsRef<str>, T: Iterator<Item = (S, S)>>(headers: T) -> HashSet<String> {
    let mut recipients: HashSet<String> = Default::default();

    for (hkey, hvalue) in headers {
        let hkey = hkey.as_ref();
        let hvalue = hvalue.as_ref();

        if hkey == "to" || hkey == "cc" {
            if let Ok(addrs) = mailparse::addrparse(hvalue) {
                for addr in addrs.iter() {
                    match addr {
                        mailparse::MailAddr::Single(ref info) => {
                            recipients.insert(addr_normalize(&info.addr).into());
                        }
                        mailparse::MailAddr::Group(ref infos) => {
                            for info in &infos.addrs {
                                recipients.insert(addr_normalize(&info.addr).into());
                            }
                        }
                    }
                }
            }
        }
    }

    recipients
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use proptest::prelude::*;

    #[test]
    fn test_dc_mimeparser_crash() {
        let context = dummy_context();
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();

        assert_eq!(mimeparser.subject, None);
        assert_eq!(mimeparser.parts.len(), 1);
    }

    proptest! {
        #[ignore]
        #[test]
        fn test_dc_mailmime_parse_crash_fuzzy(data in "[!-~\t ]{2000,}") {
            let context = dummy_context();
            // parsing should error out for all these random strings
            assert!(
                MimeParser::from_bytes(&context.ctx, data.as_bytes()).is_err()
            );
        }
    }

    #[test]
    fn test_get_rfc724_mid_exists() {
        let context = dummy_context();
        let raw = include_bytes!("../test-data/message/mail_with_message_id.txt");
        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();

        assert_eq!(
            mimeparser.get_rfc724_mid(),
            Some("2dfdbde7@example.org".into())
        );
    }

    #[test]
    fn test_get_rfc724_mid_not_exists() {
        let context = dummy_context();
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();
        assert_eq!(mimeparser.get_rfc724_mid(), None);
    }

    #[test]
    fn test_mimeparser_with_context() {
        let context = dummy_context();
        let raw = b"From: hello\n\
                    Content-Type: multipart/mixed; boundary=\"==break==\";\n\
                    Subject: outer-subject\n\
                    X-Special-A: special-a\n\
                    Foo: Bar\nChat-Version: 0.0\n\
                    \n\
                    --==break==\n\
                    Content-Type: text/plain; protected-headers=\"v1\";\n\
                    Subject: inner-subject\n\
                    X-Special-B: special-b\n\
                    Foo: Xy\n\
                    Chat-Version: 1.0\n\
                    \n\
                    test1\n\
                    \n\
                    --==break==--\n\
                    \n\
                    \x00";
        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();

        assert_eq!(mimeparser.subject, Some("inner-subject".into()));

        let of = mimeparser.lookup_field("X-Special-A").unwrap();
        assert_eq!(of, "special-a");

        let of = mimeparser.lookup_field("Foo").unwrap();
        assert_eq!(of, "Bar");

        let of = mimeparser.lookup_field("Chat-Version").unwrap();
        assert_eq!(of, "1.0");
        assert_eq!(mimeparser.parts.len(), 1);
    }
}

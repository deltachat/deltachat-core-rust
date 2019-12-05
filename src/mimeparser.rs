use std::collections::{HashMap, HashSet};

use deltachat_derive::{FromSql, ToSql};
use lettre_email::mime::{self, Mime};
use mailparse::{DispositionType, MailHeaderMap};

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
use crate::headerdef::HeaderDef;
use crate::job::{add_job_no_interrupt, Action};
use crate::location;
use crate::message;
use crate::message::MsgId;
use crate::param::*;
use crate::peerstate::Peerstate;
use crate::securejoin::handle_degrade_event;
use crate::stock::StockMessage;
use crate::{bail, ensure};

#[derive(Debug)]
pub struct MimeParser<'a> {
    pub context: &'a Context,
    pub parts: Vec<Part>,
    header: HashMap<String, String>,
    pub decrypting_failed: bool,
    pub signatures: HashSet<String>,
    pub gossipped_addr: HashSet<String>,
    pub is_forwarded: bool,
    pub is_system_message: SystemMessage,
    pub location_kml: Option<location::Kml>,
    pub message_kml: Option<location::Kml>,
    pub user_avatar: AvatarAction,
    pub group_avatar: AvatarAction,
    reports: Vec<Report>,
    mdns_enabled: bool,
    parsed_protected_headers: bool,
}

#[derive(Debug, PartialEq)]
pub enum AvatarAction {
    None,
    Delete,
    Change(String),
}

impl AvatarAction {
    pub fn is_change(&self) -> bool {
        match self {
            AvatarAction::None => false,
            AvatarAction::Delete => false,
            AvatarAction::Change(_) => true,
        }
    }
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

const MIME_AC_SETUP_FILE: &str = "application/autocrypt-setup";

impl<'a> MimeParser<'a> {
    pub fn from_bytes(context: &'a Context, body: &[u8]) -> Result<Self> {
        let mail = mailparse::parse_mail(body)?;
        let mdns_enabled = context.get_config_bool(Config::MdnsEnabled);

        let mut parser = MimeParser {
            parts: Vec::new(),
            header: Default::default(),
            decrypting_failed: false,

            // only non-empty if it was a valid autocrypt message
            signatures: Default::default(),
            gossipped_addr: Default::default(),
            is_forwarded: false,
            context,
            reports: Vec::new(),
            is_system_message: SystemMessage::Unknown,
            location_kml: None,
            message_kml: None,
            user_avatar: AvatarAction::None,
            group_avatar: AvatarAction::None,
            mdns_enabled,
            parsed_protected_headers: false,
        };

        let message_time = mail
            .headers
            .get_first_value("Date")?
            .and_then(|v| mailparse::dateparse(&v).ok())
            .unwrap_or_default();

        // init known headers with what mailparse provided us
        parser.merge_headers(&mail.headers);

        // Memory location for a possible decrypted message.
        let mail_raw;

        let mail = match e2ee::try_decrypt(parser.context, &mail, message_time) {
            Ok((raw, signatures)) => {
                // Valid autocrypt message, encrypted
                parser.signatures = signatures;

                if let Some(raw) = raw {
                    mail_raw = raw;
                    let decrypted_mail = mailparse::parse_mail(&mail_raw)?;
                    if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
                        info!(context, "decrypted message mime-body:");
                        println!("{}", String::from_utf8_lossy(&mail_raw));
                    }

                    // Handle any gossip headers if the mail was encrypted.  See section
                    // "3.6 Key Gossip" of https://autocrypt.org/autocrypt-spec-1.1.0.pdf
                    let gossip_headers =
                        decrypted_mail.headers.get_all_values("Autocrypt-Gossip")?;
                    parser.gossipped_addr =
                        update_gossip_peerstates(context, message_time, &mail, gossip_headers)?;

                    // let known protected headers from the decrypted
                    // part override the unencrypted top-level
                    parser.merge_headers(&decrypted_mail.headers);

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

        parser.parse_mime_recursive(&mail)?;

        parser.parse_headers()?;

        Ok(parser)
    }

    fn parse_headers(&mut self) -> Result<()> {
        if self.get(HeaderDef::AutocryptSetupMessage).is_some() {
            self.parts.drain_filter(|part| {
                part.mimetype.is_some()
                    && part.mimetype.as_ref().unwrap().as_ref() != MIME_AC_SETUP_FILE
            });
            if self.parts.len() == 1 {
                self.is_system_message = SystemMessage::AutocryptSetupMessage;
            } else {
                warn!(self.context, "could not determine ASM mime-part");
            }
        } else if let Some(value) = self.get(HeaderDef::ChatContent) {
            if value == "location-streaming-enabled" {
                self.is_system_message = SystemMessage::LocationStreamingEnabled;
            }
        }

        if let Some(header_value) = self.get(HeaderDef::ChatGroupAvatar).cloned() {
            self.group_avatar = self.avatar_action_from_header(header_value);
        } else if let Some(header_value) = self.get(HeaderDef::ChatGroupImage).cloned() {
            // parse the old group-image headers for versions <=0.973 resp. <=beta.15 (december 2019)
            // grep for #DeprecatedAvatar to get the place where a compatibility header is generated.
            self.group_avatar = self.avatar_action_from_header(header_value);
        }

        if let Some(header_value) = self.get(HeaderDef::ChatUserAvatar).cloned() {
            self.user_avatar = self.avatar_action_from_header(header_value);
        }

        if self.has_chat_version() && self.parts.len() == 2 {
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
                filepart.msg = self.parts[0].msg.clone();

                // forget the one we use now
                self.parts[0].msg = "".to_string();

                // swap new with old
                std::mem::replace(&mut self.parts[0], filepart);
            }
        }
        if let Some(ref subject) = self.get_subject() {
            let mut prepend_subject = 1i32;
            if !self.decrypting_failed {
                let colon = subject.find(':');
                if colon == Some(2)
                    || colon == Some(3)
                    || self.has_chat_version()
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
                            part.msg = format!("{} – {}", subj, part.msg);
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
            if self.parts[0].typ == Viewtype::Audio
                && self.get(HeaderDef::ChatVoiceMessage).is_some()
            {
                let part_mut = &mut self.parts[0];
                part_mut.typ = Viewtype::Voice;
            }
            if self.parts[0].typ == Viewtype::Image {
                if let Some(value) = self.get(HeaderDef::ChatContent) {
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
                if let Some(field_0) = self.get(HeaderDef::ChatDuration) {
                    let duration_ms = field_0.parse().unwrap_or_default();
                    if duration_ms > 0 && duration_ms < 24 * 60 * 60 * 1000 {
                        let part_mut = &mut self.parts[0];
                        part_mut.param.set_int(Param::Duration, duration_ms);
                    }
                }
            }
        }
        if !self.decrypting_failed {
            if let Some(dn_field) = self.get(HeaderDef::ChatDispositionNotificationTo) {
                if self.get_last_nonmeta().is_some() {
                    let addrs = mailparse::addrparse(&dn_field).unwrap();

                    if let Some(dn_to_addr) = addrs.first() {
                        if let Some(from_field) = self.get(HeaderDef::From_) {
                            let from_addrs = mailparse::addrparse(&from_field).unwrap();

                            if let Some(from_addr) = from_addrs.first() {
                                if compare_addrs(from_addr, dn_to_addr) {
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

        // Cleanup - and try to create at least an empty part if there are no parts yet
        if self.get_last_nonmeta().is_none() && self.reports.is_empty() {
            let mut part = Part::default();
            part.typ = Viewtype::Text;

            if let Some(ref subject) = self.get_subject() {
                if !self.has_chat_version() {
                    part.msg = subject.to_string();
                }
            }

            self.parts.push(part);
        }

        Ok(())
    }

    fn avatar_action_from_header(&mut self, header_value: String) -> AvatarAction {
        if header_value == "0" {
            return AvatarAction::Delete;
        } else {
            let mut i = 0;
            while i != self.parts.len() {
                let part = &mut self.parts[i];
                if let Some(part_filename) = &part.org_filename {
                    if part_filename == &header_value {
                        if let Some(blob) = part.param.get(Param::File) {
                            let res = AvatarAction::Change(blob.to_string());
                            self.parts.remove(i);
                            return res;
                        }
                        break;
                    }
                }
                i += 1;
            }
        }
        AvatarAction::None
    }

    pub fn get_last_nonmeta(&self) -> Option<&Part> {
        self.parts.iter().rev().find(|part| !part.is_meta)
    }

    pub fn get_last_nonmeta_mut(&mut self) -> Option<&mut Part> {
        self.parts.iter_mut().rev().find(|part| !part.is_meta)
    }

    pub fn was_encrypted(&self) -> bool {
        !self.signatures.is_empty()
    }

    pub(crate) fn has_chat_version(&self) -> bool {
        self.header.contains_key("chat-version")
    }

    pub(crate) fn has_headers(&self) -> bool {
        !self.header.is_empty()
    }

    pub(crate) fn get_subject(&self) -> Option<String> {
        if let Some(s) = self.get(HeaderDef::Subject) {
            if s.is_empty() {
                return None;
            }
            Some(s.to_string())
        } else {
            None
        }
    }

    pub fn get(&self, headerdef: HeaderDef) -> Option<&String> {
        self.header.get(&headerdef.get_headername())
    }

    fn parse_mime_recursive(&mut self, mail: &mailparse::ParsedMail<'_>) -> Result<bool> {
        if mail.ctype.params.get("protected-headers").is_some() {
            if mail.ctype.mimetype == "text/rfc822-headers" {
                warn!(
                    self.context,
                    "Protected headers found in text/rfc822-headers attachment: Will be ignored.",
                );
                return Ok(false);
            }

            warn!(self.context, "Ignoring nested protected headers");
        }

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
        let mimetype = get_mime_type(mail)?.0;
        match (mimetype.type_(), mimetype.subtype().as_str()) {
            /* Most times, mutlipart/alternative contains true alternatives
            as text/plain and text/html.  If we find a multipart/mixed
            inside mutlipart/alternative, we use this (happens eg in
            apple mail: "plaintext" as an alternative to "html+PDF attachment") */
            (mime::MULTIPART, "alternative") => {
                for cur_data in &mail.subparts {
                    if get_mime_type(cur_data)?.0 == "multipart/mixed" {
                        any_part_added = self.parse_mime_recursive(cur_data)?;
                        break;
                    }
                }
                if !any_part_added {
                    /* search for text/plain and add this */
                    for cur_data in &mail.subparts {
                        if get_mime_type(cur_data)?.0.type_() == mime::TEXT {
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
            (mime::MULTIPART, "related") => {
                /* add the "root part" - the other parts may be referenced which is
                not interesting for us (eg. embedded images) we assume he "root part"
                being the first one, which may not be always true ...
                however, most times it seems okay. */
                if let Some(first) = mail.subparts.iter().next() {
                    any_part_added = self.parse_mime_recursive(first)?;
                }
            }
            (mime::MULTIPART, "encrypted") => {
                // we currently do not try to decrypt non-autocrypt messages
                // at all. If we see an encrypted part, we set
                // decrypting_failed.
                let msg_body = self.context.stock_str(StockMessage::CantDecryptMsgBody);
                let txt = format!("[{}]", msg_body);

                let mut part = Part::default();
                part.typ = Viewtype::Text;
                part.msg_raw = Some(txt.clone());
                part.msg = txt;

                self.parts.push(part);

                any_part_added = true;
                self.decrypting_failed = true;
            }
            (mime::MULTIPART, "signed") => {
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
            (mime::MULTIPART, "report") => {
                /* RFC 6522: the first part is for humans, the second for machines */
                if mail.subparts.len() >= 2 {
                    if let Some(report_type) = mail.ctype.params.get("report-type") {
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
                // Add all parts (in fact, AddSinglePartIfKnown() later check if
                // the parts are really supported)
                for cur_data in mail.subparts.iter() {
                    if self.parse_mime_recursive(cur_data)? {
                        any_part_added = true;
                    }
                }
            }
        }

        Ok(any_part_added)
    }

    fn add_single_part_if_known(&mut self, mail: &mailparse::ParsedMail<'_>) -> Result<bool> {
        // return true if a part was added
        let (mime_type, msg_type) = get_mime_type(mail)?;
        let raw_mime = mail.ctype.mimetype.to_lowercase();

        let filename = get_attachment_filename(mail);

        let old_part_count = self.parts.len();

        if let Ok(filename) = filename {
            self.do_add_single_file_part(
                msg_type,
                mime_type,
                &raw_mime,
                &mail.get_body_raw()?,
                &filename,
            );
        } else {
            match mime_type.type_() {
                mime::IMAGE | mime::AUDIO | mime::VIDEO | mime::APPLICATION => {
                    bail!("missing attachment");
                }
                mime::TEXT | mime::HTML => {
                    let decoded_data = match mail.get_body() {
                        Ok(decoded_data) => decoded_data,
                        Err(err) => {
                            warn!(self.context, "Invalid body parsed {:?}", err);
                            // Note that it's not always an error - might be no data
                            return Ok(false);
                        }
                    };

                    let mut simplifier = Simplify::new();
                    let simplified_txt = if decoded_data.is_empty() {
                        "".into()
                    } else {
                        let is_html = mime_type == mime::TEXT_HTML;
                        simplifier.simplify(&decoded_data, is_html, self.has_chat_version())
                    };

                    if !simplified_txt.is_empty() {
                        let mut part = Part::default();
                        part.typ = Viewtype::Text;
                        part.mimetype = Some(mime_type);
                        part.msg = simplified_txt;
                        part.msg_raw = Some(decoded_data);
                        self.do_add_single_part(part);
                    }

                    if simplifier.is_forwarded {
                        self.is_forwarded = true;
                    }
                }
                _ => {}
            }
        }

        // add object? (we do not add all objects, eg. signatures etc. are ignored)
        Ok(self.parts.len() > old_part_count)
    }

    fn do_add_single_file_part(
        &mut self,
        msg_type: Viewtype,
        mime_type: Mime,
        raw_mime: &str,
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
        info!(self.context, "added blobfile: {:?}", blob.as_name());

        /* create and register Mime part referencing the new Blob object */
        let mut part = Part::default();
        if mime_type.type_() == mime::IMAGE {
            if let Ok((width, height)) = dc_get_filemeta(decoded_data) {
                part.param.set_int(Param::Width, width as i32);
                part.param.set_int(Param::Height, height as i32);
            }
        }

        part.typ = msg_type;
        part.org_filename = Some(filename.to_string());
        part.mimetype = Some(mime_type);
        part.bytes = decoded_data.len();
        part.param.set(Param::File, blob.as_name());
        part.param.set(Param::MimeType, raw_mime);

        self.do_add_single_part(part);
    }

    fn do_add_single_part(&mut self, mut part: Part) {
        if self.was_encrypted() {
            part.param.set_int(Param::GuaranteeE2ee, 1);
        }
        self.parts.push(part);
    }

    pub fn is_mailinglist_message(&self) -> bool {
        if self.get(HeaderDef::ListId).is_some() {
            return true;
        }

        if let Some(precedence) = self.get(HeaderDef::Precedence) {
            precedence == "list" || precedence == "bulk"
        } else {
            false
        }
    }

    pub fn repl_msg_by_error(&mut self, error_msg: impl AsRef<str>) {
        if self.parts.is_empty() {
            return;
        }

        let part = &mut self.parts[0];
        part.typ = Viewtype::Text;
        part.msg = format!("[{}]", error_msg.as_ref());
        self.parts.truncate(1);

        assert_eq!(self.parts.len(), 1);
    }

    pub fn get_rfc724_mid(&self) -> Option<String> {
        if let Some(msgid) = self.get(HeaderDef::MessageId) {
            parse_message_id(msgid)
        } else {
            None
        }
    }

    fn merge_headers(&mut self, fields: &[mailparse::MailHeader<'_>]) {
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
        // to get a clear functionality, do not show incoming MDNs if the options is disabled
        if !self.mdns_enabled {
            return Ok(None);
        }

        // parse as mailheaders
        let report_body = report.subparts[1].get_body_raw()?;
        let (report_fields, _) = mailparse::parse_headers(&report_body)?;

        // must be present
        let disp = HeaderDef::Disposition.get_headername();
        if let Some(_disposition) = report_fields.get_first_value(&disp).ok().flatten() {
            if let Some(original_message_id) = report_fields
                .get_first_value(&HeaderDef::OriginalMessageId.get_headername())
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

            if self.has_chat_version() || mdn_consumed {
                let mut param = Params::new();
                param.set(Param::ServerFolder, server_folder.as_ref());
                param.set_int(Param::ServerUid, server_uid as i32);
                if self.has_chat_version() && self.context.get_config_bool(Config::MvboxMove) {
                    param.set_int(Param::AlsoMove, 1);
                }
                add_job_no_interrupt(self.context, Action::MarkseenMdnOnImap, 0, param, 0);
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
                recipients = Some(get_recipients(mail.headers.iter().filter_map(|v| {
                    let key = v.get_key();
                    let value = v.get_value();
                    if key.is_err() || value.is_err() {
                        return None;
                    }
                    Some((v.get_key().unwrap(), v.get_value().unwrap()))
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
                warn!(
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

#[derive(Debug, Default, Clone)]
pub struct Part {
    pub typ: Viewtype,
    pub is_meta: bool,
    pub mimetype: Option<Mime>,
    pub msg: String,
    pub msg_raw: Option<String>,
    pub bytes: usize,
    pub param: Params,
    org_filename: Option<String>,
}

/// return mimetype and viewtype for a parsed mail
fn get_mime_type(mail: &mailparse::ParsedMail<'_>) -> Result<(Mime, Viewtype)> {
    let mimetype = mail.ctype.mimetype.parse::<Mime>()?;

    let viewtype = match mimetype.type_() {
        mime::TEXT => {
            if !is_attachment_disposition(mail) {
                match mimetype.subtype() {
                    mime::PLAIN | mime::HTML => Viewtype::Text,
                    _ => Viewtype::File,
                }
            } else {
                Viewtype::File
            }
        }
        mime::IMAGE => match mimetype.subtype() {
            mime::GIF => Viewtype::Gif,
            mime::SVG => Viewtype::File,
            _ => Viewtype::Image,
        },
        mime::AUDIO => Viewtype::Audio,
        mime::VIDEO => Viewtype::Video,
        mime::MULTIPART => Viewtype::Unknown,
        mime::MESSAGE => {
            // Enacapsulated messages, see https://www.w3.org/Protocols/rfc1341/7_3_Message.html
            // Also used as part "message/disposition-notification" of "multipart/report", which, however, will
            // be handled separatedly.
            // I've not seen any messages using this, so we do not attach these parts (maybe they're used to attach replies,
            // which are unwanted at all).
            // For now, we skip these parts at all; if desired, we could return DcMimeType::File/DC_MSG_File
            // for selected and known subparts.
            Viewtype::Unknown
        }
        mime::APPLICATION => Viewtype::File,
        _ => Viewtype::Unknown,
    };

    Ok((mimetype, viewtype))
}

fn is_attachment_disposition(mail: &mailparse::ParsedMail<'_>) -> bool {
    if let Ok(ct) = mail.get_content_disposition() {
        return ct.disposition == DispositionType::Attachment
            && ct
                .params
                .iter()
                .any(|(key, _value)| key.starts_with("filename"));
    }

    false
}

fn get_attachment_filename(mail: &mailparse::ParsedMail) -> Result<String> {
    // try to get file name from
    //    `Content-Disposition: ... filename*=...`
    // or `Content-Disposition: ... filename*0*=... filename*1*=... filename*2*=...`
    // or `Content-Disposition: ... filename=...`

    let ct = mail.get_content_disposition()?;
    ensure!(
        ct.disposition == DispositionType::Attachment,
        "disposition not an attachment: {:?}",
        ct.disposition
    );

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
        if let Some(subtype) = mail.ctype.mimetype.split('/').nth(1) {
            desired_filename = format!("file.{}", subtype,);
        } else {
            bail!("could not determine filename: {:?}", ct.disposition);
        }
    }

    Ok(desired_filename)
}

// returned addresses are normalized.
fn get_recipients<S: AsRef<str>, T: Iterator<Item = (S, S)>>(headers: T) -> HashSet<String> {
    let mut recipients: HashSet<String> = Default::default();

    for (hkey, hvalue) in headers {
        let hkey = hkey.as_ref().to_lowercase();
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

/// Check if the only addrs match, ignoring names.
fn compare_addrs(a: &mailparse::MailAddr, b: &mailparse::MailAddr) -> bool {
    match a {
        mailparse::MailAddr::Group(group_a) => match b {
            mailparse::MailAddr::Group(group_b) => group_a
                .addrs
                .iter()
                .zip(group_b.addrs.iter())
                .all(|(a, b)| a.addr == b.addr),
            _ => false,
        },
        mailparse::MailAddr::Single(single_a) => match b {
            mailparse::MailAddr::Single(single_b) => single_a.addr == single_b.addr,
            _ => false,
        },
    }
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

        assert_eq!(mimeparser.get_subject(), None);
        assert_eq!(mimeparser.parts.len(), 1);
    }

    proptest! {
        #[ignore]
        #[test]
        fn test_dc_mailmime_parse_crash_fuzzy(data in "[!-~\t ]{2000,}") {
            let context = dummy_context();

            // just don't crash
            let _ = MimeParser::from_bytes(&context.ctx, data.as_bytes());
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
    fn test_get_recipients() {
        let context = dummy_context();
        let raw = include_bytes!("../test-data/message/mail_with_cc.txt");
        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();
        let recipients = get_recipients(mimeparser.header.iter());
        assert!(recipients.contains("abc@bcd.com"));
        assert!(recipients.contains("def@def.de"));
        assert_eq!(recipients.len(), 2);
    }

    #[test]
    fn test_is_attachment() {
        let raw = include_bytes!("../test-data/message/mail_with_cc.txt");
        let mail = mailparse::parse_mail(raw).unwrap();
        assert!(!is_attachment_disposition(&mail));

        let raw = include_bytes!("../test-data/message/mail_attach_txt.eml");
        let mail = mailparse::parse_mail(raw).unwrap();
        assert!(!is_attachment_disposition(&mail));
        assert!(!is_attachment_disposition(&mail.subparts[0]));
        assert!(is_attachment_disposition(&mail.subparts[1]));
    }

    #[test]
    fn test_get_attachment_filename() {
        let raw = include_bytes!("../test-data/message/html_attach.eml");
        let mail = mailparse::parse_mail(raw).unwrap();
        assert!(get_attachment_filename(&mail).is_err());
        assert!(get_attachment_filename(&mail.subparts[0]).is_err());
        let filename = get_attachment_filename(&mail.subparts[1]).unwrap();
        assert_eq!(filename, "test.html")
    }

    #[test]
    fn test_mailparse_content_type() {
        let ctype =
            mailparse::parse_content_type("text/plain; charset=utf-8; protected-headers=v1;");

        assert_eq!(ctype.mimetype, "text/plain");
        assert_eq!(ctype.charset, "utf-8");
        assert_eq!(
            ctype.params.get("protected-headers"),
            Some(&"v1".to_string())
        );
    }

    #[test]
    fn test_mimeparser_with_context() {
        let context = dummy_context();
        let raw = b"From: hello\n\
                    Content-Type: multipart/mixed; boundary=\"==break==\";\n\
                    Subject: outer-subject\n\
                    Secure-Join-Group: no\n\
                    Test-Header: Bar\nChat-Version: 0.0\n\
                    \n\
                    --==break==\n\
                    Content-Type: text/plain; protected-headers=\"v1\";\n\
                    Subject: inner-subject\n\
                    SecureBar-Join-Group: yes\n\
                    Test-Header: Xy\n\
                    Chat-Version: 1.0\n\
                    \n\
                    test1\n\
                    \n\
                    --==break==--\n\
                    \n\
                    \x00";

        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();

        // non-overwritten headers do not bubble up
        let of = mimeparser.get(HeaderDef::SecureJoinGroup).unwrap();
        assert_eq!(of, "no");

        // unknown headers do not bubble upwards
        let of = mimeparser.get(HeaderDef::_TestHeader).unwrap();
        assert_eq!(of, "Bar");

        // the following fields would bubble up
        // if the test would really use encryption for the protected part
        // however, as this is not the case, the outer things stay valid
        assert_eq!(mimeparser.get_subject(), Some("outer-subject".into()));

        let of = mimeparser.get(HeaderDef::ChatVersion).unwrap();
        assert_eq!(of, "0.0");
        assert_eq!(mimeparser.parts.len(), 1);
    }

    #[test]
    fn test_mimeparser_with_avatars() {
        let t = dummy_context();

        let raw = include_bytes!("../test-data/message/mail_attach_txt.eml");
        let mimeparser = MimeParser::from_bytes(&t.ctx, &raw[..]).unwrap();
        assert_eq!(mimeparser.user_avatar, AvatarAction::None);
        assert_eq!(mimeparser.group_avatar, AvatarAction::None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar.eml");
        let mimeparser = MimeParser::from_bytes(&t.ctx, &raw[..]).unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.is_change());
        assert_eq!(mimeparser.group_avatar, AvatarAction::None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar_deleted.eml");
        let mimeparser = MimeParser::from_bytes(&t.ctx, &raw[..]).unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert_eq!(mimeparser.user_avatar, AvatarAction::Delete);
        assert_eq!(mimeparser.group_avatar, AvatarAction::None);

        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let mimeparser = MimeParser::from_bytes(&t.ctx, &raw[..]).unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.is_change());
        assert!(mimeparser.group_avatar.is_change());

        // if the Chat-User-Avatar header is missing, the avatar become a normal attachment
        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let raw = String::from_utf8_lossy(raw).to_string();
        let raw = raw.replace("Chat-User-Avatar:", "Xhat-Xser-Xvatar:");
        let mimeparser = MimeParser::from_bytes(&t.ctx, raw.as_bytes()).unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Image);
        assert_eq!(mimeparser.user_avatar, AvatarAction::None);
        assert!(mimeparser.group_avatar.is_change());
    }
}

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use deltachat_derive::{FromSql, ToSql};
use lettre_email::mime::{self, Mime};
use mailparse::{addrparse_header, DispositionType, MailHeader, MailHeaderMap, SingleInfo};
use once_cell::sync::Lazy;

use crate::aheader::Aheader;
use crate::blob::BlobObject;
use crate::constants::Viewtype;
use crate::contact::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::dehtml::dehtml;
use crate::e2ee;
use crate::error::{bail, Result};
use crate::events::EventType;
use crate::format_flowed::unformat_flowed;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::key::Fingerprint;
use crate::location;
use crate::message;
use crate::param::*;
use crate::peerstate::Peerstate;
use crate::simplify::*;
use crate::stock::StockMessage;

/// A parsed MIME message.
///
/// This represents the relevant information of a parsed MIME message
/// for deltachat.  The original MIME message might have had more
/// information but this representation should contain everything
/// needed for deltachat's purposes.
///
/// It is created by parsing the raw data of an actual MIME message
/// using the [MimeMessage::from_bytes] constructor.
#[derive(Debug)]
pub struct MimeMessage {
    pub parts: Vec<Part>,
    header: HashMap<String, String>,

    /// Addresses are normalized and lowercased:
    pub recipients: Vec<SingleInfo>,
    pub from: Vec<SingleInfo>,
    pub chat_disposition_notification_to: Option<SingleInfo>,
    pub decrypting_failed: bool,

    /// Set of valid signature fingerprints if a message is an
    /// Autocrypt encrypted and signed message.
    ///
    /// If a message is not encrypted or the signature is not valid,
    /// this set is empty.
    pub signatures: HashSet<Fingerprint>,

    pub gossipped_addr: HashSet<String>,
    pub is_forwarded: bool,
    pub is_system_message: SystemMessage,
    pub location_kml: Option<location::Kml>,
    pub message_kml: Option<location::Kml>,
    pub(crate) user_avatar: Option<AvatarAction>,
    pub(crate) group_avatar: Option<AvatarAction>,
    pub(crate) mdn_reports: Vec<Report>,
    pub(crate) failure_report: Option<FailureReport>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum AvatarAction {
    Delete,
    Change(String),
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

    /// Chat ephemeral message timer is changed.
    EphemeralTimerChanged = 10,

    // Chat protection state changed
    ChatProtectionEnabled = 11,
    ChatProtectionDisabled = 12,
}

impl Default for SystemMessage {
    fn default() -> Self {
        SystemMessage::Unknown
    }
}

const MIME_AC_SETUP_FILE: &str = "application/autocrypt-setup";

impl MimeMessage {
    pub async fn from_bytes(context: &Context, body: &[u8]) -> Result<Self> {
        let mail = mailparse::parse_mail(body)?;

        let message_time = mail
            .headers
            .get_header_value(HeaderDef::Date)
            .and_then(|v| mailparse::dateparse(&v).ok())
            .unwrap_or_default();

        let mut headers = Default::default();
        let mut recipients = Default::default();
        let mut from = Default::default();
        let mut chat_disposition_notification_to = None;

        // init known headers with what mailparse provided us
        MimeMessage::merge_headers(
            context,
            &mut headers,
            &mut recipients,
            &mut from,
            &mut chat_disposition_notification_to,
            &mail.headers,
        );

        // remove headers that are allowed _only_ in the encrypted part
        headers.remove("secure-join-fingerprint");
        headers.remove("chat-verified");

        // Memory location for a possible decrypted message.
        let mail_raw;
        let mut gossipped_addr = Default::default();

        let (mail, signatures, warn_empty_signature) =
            match e2ee::try_decrypt(context, &mail, message_time).await {
                Ok((raw, signatures)) => {
                    if let Some(raw) = raw {
                        // Encrypted, but maybe unsigned message. Only if
                        // `signatures` set is non-empty, it is a valid
                        // autocrypt message.

                        mail_raw = raw;
                        let decrypted_mail = mailparse::parse_mail(&mail_raw)?;
                        if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
                            info!(context, "decrypted message mime-body:");
                            println!("{}", String::from_utf8_lossy(&mail_raw));
                        }

                        // Handle any gossip headers if the mail was encrypted.  See section
                        // "3.6 Key Gossip" of https://autocrypt.org/autocrypt-spec-1.1.0.pdf
                        // but only if the mail was correctly signed:
                        if !signatures.is_empty() {
                            let gossip_headers =
                                decrypted_mail.headers.get_all_values("Autocrypt-Gossip");
                            gossipped_addr = update_gossip_peerstates(
                                context,
                                message_time,
                                &mail,
                                gossip_headers,
                            )
                            .await?;
                        }

                        // let known protected headers from the decrypted
                        // part override the unencrypted top-level

                        // Signature was checked for original From, so we
                        // do not allow overriding it.
                        let mut throwaway_from = from.clone();

                        // We do not want to allow unencrypted subject in encrypted emails because the user might falsely think that the subject is safe.
                        // See https://github.com/deltachat/deltachat-core-rust/issues/1790.
                        headers.remove("subject");

                        MimeMessage::merge_headers(
                            context,
                            &mut headers,
                            &mut recipients,
                            &mut throwaway_from,
                            &mut chat_disposition_notification_to,
                            &decrypted_mail.headers,
                        );

                        (decrypted_mail, signatures, true)
                    } else {
                        // Message was not encrypted
                        (mail, signatures, false)
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
                    warn!(context, "decryption failed: {}", err);
                    (mail, Default::default(), true)
                }
            };

        let mut parser = MimeMessage {
            parts: Vec::new(),
            header: headers,
            recipients,
            from,
            chat_disposition_notification_to,
            decrypting_failed: false,

            // only non-empty if it was a valid autocrypt message
            signatures,
            gossipped_addr,
            is_forwarded: false,
            mdn_reports: Vec::new(),
            is_system_message: SystemMessage::Unknown,
            location_kml: None,
            message_kml: None,
            user_avatar: None,
            group_avatar: None,
            failure_report: None,
        };
        parser.parse_mime_recursive(context, &mail).await?;
        parser.maybe_remove_bad_parts().await;
        parser.heuristically_parse_ndn(context).await;
        parser.parse_headers(context)?;

        if warn_empty_signature && parser.signatures.is_empty() {
            for part in parser.parts.iter_mut() {
                part.error = Some("No valid signature".to_string());
            }
        }

        Ok(parser)
    }

    /// Parses system messages.
    fn parse_system_message_headers(&mut self, context: &Context) -> Result<()> {
        if self.get(HeaderDef::AutocryptSetupMessage).is_some() {
            self.parts = self
                .parts
                .iter()
                .filter(|part| {
                    part.mimetype.is_none()
                        || part.mimetype.as_ref().unwrap().as_ref() == MIME_AC_SETUP_FILE
                })
                .cloned()
                .collect();

            if self.parts.len() == 1 {
                self.is_system_message = SystemMessage::AutocryptSetupMessage;
            } else {
                warn!(context, "could not determine ASM mime-part");
            }
        } else if let Some(value) = self.get(HeaderDef::ChatContent) {
            if value == "location-streaming-enabled" {
                self.is_system_message = SystemMessage::LocationStreamingEnabled;
            } else if value == "ephemeral-timer-changed" {
                self.is_system_message = SystemMessage::EphemeralTimerChanged;
            } else if value == "protection-enabled" {
                self.is_system_message = SystemMessage::ChatProtectionEnabled;
            } else if value == "protection-disabled" {
                self.is_system_message = SystemMessage::ChatProtectionDisabled;
            }
        }
        Ok(())
    }

    /// Parses avatar action headers.
    fn parse_avatar_headers(&mut self) {
        if let Some(header_value) = self.get(HeaderDef::ChatGroupAvatar).cloned() {
            self.group_avatar = self.avatar_action_from_header(header_value);
        }

        if let Some(header_value) = self.get(HeaderDef::ChatUserAvatar).cloned() {
            self.user_avatar = self.avatar_action_from_header(header_value);
        }
    }

    fn parse_videochat_headers(&mut self) {
        if let Some(value) = self.get(HeaderDef::ChatContent).cloned() {
            if value == "videochat-invitation" {
                let instance = self.get(HeaderDef::ChatWebrtcRoom).cloned();
                if let Some(part) = self.parts.first_mut() {
                    part.typ = Viewtype::VideochatInvitation;
                    part.param
                        .set(Param::WebrtcRoom, instance.unwrap_or_default());
                }
            }
        }
    }

    /// Squashes mutlipart chat messages with attachment into single-part messages.
    ///
    /// Delta Chat sends attachments, such as images, in two-part messages, with the first message
    /// containing a description. If such a message is detected, text from the first part can be
    /// moved to the second part, and the first part dropped.
    #[allow(clippy::indexing_slicing)]
    fn squash_attachment_parts(&mut self) {
        if let [textpart, filepart] = &self.parts[..] {
            let need_drop = {
                textpart.typ == Viewtype::Text
                    && (filepart.typ == Viewtype::Image
                        || filepart.typ == Viewtype::Gif
                        || filepart.typ == Viewtype::Sticker
                        || filepart.typ == Viewtype::Audio
                        || filepart.typ == Viewtype::Voice
                        || filepart.typ == Viewtype::Video
                        || filepart.typ == Viewtype::File)
            };

            if need_drop {
                let mut filepart = self.parts.swap_remove(1);

                // insert new one
                filepart.msg = self.parts[0].msg.clone();
                if let Some(quote) = self.parts[0].param.get(Param::Quote) {
                    filepart.param.set(Param::Quote, quote);
                }

                // forget the one we use now
                self.parts[0].msg = "".to_string();

                // swap new with old
                self.parts.push(filepart); // push to the end
                let _ = self.parts.swap_remove(0); // drops first element, replacing it with the last one in O(1)
            }
        }
    }

    /// Processes chat messages with attachments.
    fn parse_attachments(&mut self) {
        // Attachment messages should be squashed into a single part
        // before calling this function.
        if self.parts.len() != 1 {
            return;
        }

        if let Some(mut part) = self.parts.pop() {
            if part.typ == Viewtype::Audio && self.get(HeaderDef::ChatVoiceMessage).is_some() {
                part.typ = Viewtype::Voice;
            }
            if part.typ == Viewtype::Image {
                if let Some(value) = self.get(HeaderDef::ChatContent) {
                    if value == "sticker" {
                        part.typ = Viewtype::Sticker;
                    }
                }
            }
            if part.typ == Viewtype::Audio
                || part.typ == Viewtype::Voice
                || part.typ == Viewtype::Video
            {
                if let Some(field_0) = self.get(HeaderDef::ChatDuration) {
                    let duration_ms = field_0.parse().unwrap_or_default();
                    if duration_ms > 0 && duration_ms < 24 * 60 * 60 * 1000 {
                        part.param.set_int(Param::Duration, duration_ms);
                    }
                }
            }

            self.parts.push(part);
        }
    }

    fn parse_headers(&mut self, context: &Context) -> Result<()> {
        self.parse_system_message_headers(context)?;
        self.parse_avatar_headers();
        self.parse_videochat_headers();
        self.squash_attachment_parts();

        if let Some(ref subject) = self.get_subject() {
            let mut prepend_subject = true;
            if !self.decrypting_failed {
                let colon = subject.find(':');
                if colon == Some(2)
                    || colon == Some(3)
                    || self.has_chat_version()
                    || subject.contains("Chat:")
                {
                    prepend_subject = false
                }
            }
            if prepend_subject {
                let subj = subject
                    .find('[')
                    .and_then(|n| subject.get(..n))
                    .unwrap_or(subject)
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

        self.parse_attachments();

        // See if an MDN is requested from the other side
        if !self.decrypting_failed && !self.parts.is_empty() {
            if let Some(ref dn_to) = self.chat_disposition_notification_to {
                if let Some(ref from) = self.from.get(0) {
                    if from.addr == dn_to.addr {
                        if let Some(part) = self.parts.last_mut() {
                            part.param.set_int(Param::WantsMdn, 1);
                        }
                    }
                }
            }
        }

        // If there were no parts, especially a non-DC mail user may
        // just have send a message in the subject with an empty body.
        // Besides, we want to show something in case our incoming-processing
        // failed to properly handle an incoming message.
        if self.parts.is_empty() && self.mdn_reports.is_empty() {
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

    fn avatar_action_from_header(&mut self, header_value: String) -> Option<AvatarAction> {
        if header_value == "0" {
            Some(AvatarAction::Delete)
        } else {
            let mut i = 0;
            while let Some(part) = self.parts.get_mut(i) {
                if let Some(part_filename) = &part.org_filename {
                    if part_filename == &header_value {
                        if let Some(blob) = part.param.get(Param::File) {
                            let res = Some(AvatarAction::Change(blob.to_string()));
                            self.parts.remove(i);
                            return res;
                        }
                        break;
                    }
                }
                i += 1;
            }
            None
        }
    }

    /// Returns true if the message was encrypted as defined in
    /// Autocrypt standard.
    ///
    /// This means the message was both encrypted and signed with a
    /// valid signature.
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
        self.get(HeaderDef::Subject)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    pub fn get(&self, headerdef: HeaderDef) -> Option<&String> {
        self.header.get(headerdef.get_headername())
    }

    fn parse_mime_recursive<'a>(
        &'a mut self,
        context: &'a Context,
        mail: &'a mailparse::ParsedMail<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a + Send>> {
        use futures::future::FutureExt;

        // Boxed future to deal with recursion
        async move {
            if mail.ctype.params.get("protected-headers").is_some() {
                if mail.ctype.mimetype == "text/rfc822-headers" {
                    warn!(
                        context,
                        "Protected headers found in text/rfc822-headers attachment: Will be ignored.",
                    );
                    return Ok(false);
                }

                warn!(context, "Ignoring nested protected headers");
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
                MimeS::Multiple => self.handle_multiple(context, mail).await,
                MimeS::Message => {
                    let raw = mail.get_body_raw()?;
                    if raw.is_empty() {
                        return Ok(false);
                    }
                    let mail = mailparse::parse_mail(&raw).unwrap();

                    self.parse_mime_recursive(context, &mail).await
                }
                MimeS::Single => self.add_single_part_if_known(context, mail).await,
            }
        }
        .boxed()
    }

    async fn handle_multiple(
        &mut self,
        context: &Context,
        mail: &mailparse::ParsedMail<'_>,
    ) -> Result<bool> {
        let mut any_part_added = false;
        let mimetype = get_mime_type(mail)?.0;
        match (mimetype.type_(), mimetype.subtype().as_str()) {
            /* Most times, mutlipart/alternative contains true alternatives
            as text/plain and text/html.  If we find a multipart/mixed
            inside mutlipart/alternative, we use this (happens eg in
            apple mail: "plaintext" as an alternative to "html+PDF attachment") */
            (mime::MULTIPART, "alternative") => {
                for cur_data in &mail.subparts {
                    if get_mime_type(cur_data)?.0 == "multipart/mixed"
                        || get_mime_type(cur_data)?.0 == "multipart/related"
                    {
                        any_part_added = self.parse_mime_recursive(context, cur_data).await?;
                        break;
                    }
                }
                if !any_part_added {
                    /* search for text/plain and add this */
                    for cur_data in &mail.subparts {
                        if get_mime_type(cur_data)?.0.type_() == mime::TEXT {
                            any_part_added = self.parse_mime_recursive(context, cur_data).await?;
                            break;
                        }
                    }
                }
                if !any_part_added {
                    /* `text/plain` not found - use the first part */
                    for cur_part in &mail.subparts {
                        if self.parse_mime_recursive(context, cur_part).await? {
                            any_part_added = true;
                            break;
                        }
                    }
                }
            }
            (mime::MULTIPART, "encrypted") => {
                // we currently do not try to decrypt non-autocrypt messages
                // at all. If we see an encrypted part, we set
                // decrypting_failed.
                let msg_body = context.stock_str(StockMessage::CantDecryptMsgBody).await;
                let txt = format!("[{}]", msg_body);

                let mut part = Part::default();
                part.typ = Viewtype::Text;
                part.msg_raw = Some(txt.clone());
                part.msg = txt;
                part.error = Some("Decryption failed".to_string());

                self.parts.push(part);

                any_part_added = true;
                self.decrypting_failed = true;
            }
            (mime::MULTIPART, "signed") => {
                /* RFC 1847: "The multipart/signed content type
                contains exactly two body parts.  The first body
                part is the body part over which the digital signature was created [...]
                The second body part contains the control information necessary to
                verify the digital signature." We simply take the first body part and
                skip the rest.  (see
                https://k9mail.github.io/2016/11/24/OpenPGP-Considerations-Part-I.html
                for background information why we use encrypted+signed) */
                if let Some(first) = mail.subparts.get(0) {
                    any_part_added = self.parse_mime_recursive(context, first).await?;
                }
            }
            (mime::MULTIPART, "report") => {
                /* RFC 6522: the first part is for humans, the second for machines */
                if mail.subparts.len() >= 2 {
                    match mail.ctype.params.get("report-type").map(|s| s as &str) {
                        Some("disposition-notification") => {
                            if let Some(report) = self.process_report(context, mail)? {
                                self.mdn_reports.push(report);
                            }

                            // Add MDN part so we can track it, avoid
                            // downloading the message again and
                            // delete if automatic message deletion is
                            // enabled.
                            let mut part = Part::default();
                            part.typ = Viewtype::Unknown;
                            self.parts.push(part);

                            any_part_added = true;
                        }
                        // Some providers, e.g. Tiscali, forget to set the report-type. So, if it's None, assume that it might be delivery-status
                        Some("delivery-status") | None => {
                            if let Some(report) = self.process_delivery_status(context, mail)? {
                                self.failure_report = Some(report);
                            }

                            // Add all parts (we need another part, preferrably text/plain, to show as an error message)
                            for cur_data in mail.subparts.iter() {
                                if self.parse_mime_recursive(context, cur_data).await? {
                                    any_part_added = true;
                                }
                            }
                        }
                        Some(_) => {
                            if let Some(first) = mail.subparts.get(0) {
                                any_part_added = self.parse_mime_recursive(context, first).await?;
                            }
                        }
                    }
                }
            }
            _ => {
                // Add all parts (in fact, AddSinglePartIfKnown() later check if
                // the parts are really supported)
                for cur_data in mail.subparts.iter() {
                    if self.parse_mime_recursive(context, cur_data).await? {
                        any_part_added = true;
                    }
                }
            }
        }

        Ok(any_part_added)
    }

    async fn add_single_part_if_known(
        &mut self,
        context: &Context,
        mail: &mailparse::ParsedMail<'_>,
    ) -> Result<bool> {
        // return true if a part was added
        let (mime_type, msg_type) = get_mime_type(mail)?;
        let raw_mime = mail.ctype.mimetype.to_lowercase();

        let filename = get_attachment_filename(mail)?;

        let old_part_count = self.parts.len();

        match filename {
            Some(filename) => {
                self.do_add_single_file_part(
                    context,
                    msg_type,
                    mime_type,
                    &raw_mime,
                    &mail.get_body_raw()?,
                    &filename,
                )
                .await;
            }
            None => {
                match mime_type.type_() {
                    mime::IMAGE | mime::AUDIO | mime::VIDEO | mime::APPLICATION => {
                        warn!(context, "Missing attachment");
                        return Ok(false);
                    }
                    mime::TEXT | mime::HTML => {
                        let decoded_data = match mail.get_body() {
                            Ok(decoded_data) => decoded_data,
                            Err(err) => {
                                warn!(context, "Invalid body parsed {:?}", err);
                                // Note that it's not always an error - might be no data
                                return Ok(false);
                            }
                        };

                        let mut dehtml_failed = false;

                        let (simplified_txt, is_forwarded, top_quote) = if decoded_data.is_empty() {
                            ("".to_string(), false, None)
                        } else {
                            let is_html = mime_type == mime::TEXT_HTML;
                            let out = if is_html {
                                dehtml(&decoded_data).unwrap_or_else(|| {
                                    dehtml_failed = true;
                                    decoded_data.clone()
                                })
                            } else {
                                decoded_data.clone()
                            };
                            simplify(out, self.has_chat_version())
                        };

                        let is_format_flowed = if let Some(format) = mail.ctype.params.get("format")
                        {
                            format.as_str().to_ascii_lowercase() == "flowed"
                        } else {
                            false
                        };

                        let (simplified_txt, simplified_quote) = if mime_type.type_() == mime::TEXT
                            && mime_type.subtype() == mime::PLAIN
                            && is_format_flowed
                        {
                            let delsp = if let Some(delsp) = mail.ctype.params.get("delsp") {
                                delsp.as_str().to_ascii_lowercase() == "yes"
                            } else {
                                false
                            };
                            let unflowed_text = unformat_flowed(&simplified_txt, delsp);
                            let unflowed_quote = top_quote.map(|q| unformat_flowed(&q, delsp));
                            (unflowed_text, unflowed_quote)
                        } else {
                            (simplified_txt, top_quote)
                        };

                        if !simplified_txt.is_empty() || simplified_quote.is_some() {
                            let mut part = Part::default();
                            part.dehtlm_failed = dehtml_failed;
                            part.typ = Viewtype::Text;
                            part.mimetype = Some(mime_type);
                            part.msg = simplified_txt;
                            if let Some(quote) = simplified_quote {
                                part.param.set(Param::Quote, quote);
                            }
                            part.msg_raw = Some(decoded_data);
                            self.do_add_single_part(part);
                        }

                        if is_forwarded {
                            self.is_forwarded = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        // add object? (we do not add all objects, eg. signatures etc. are ignored)
        Ok(self.parts.len() > old_part_count)
    }

    async fn do_add_single_file_part(
        &mut self,
        context: &Context,
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
                let parsed = location::Kml::parse(context, decoded_data)
                    .map_err(|err| {
                        warn!(context, "failed to parse kml part: {}", err);
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

        let blob = match BlobObject::create(context, filename, decoded_data).await {
            Ok(blob) => blob,
            Err(err) => {
                error!(
                    context,
                    "Could not add blob for mime part {}, error {}", filename, err
                );
                return;
            }
        };
        info!(context, "added blobfile: {:?}", blob.as_name());

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
        self.is_system_message = SystemMessage::Unknown;
        if let Some(part) = self.parts.first_mut() {
            part.typ = Viewtype::Text;
            part.msg = format!("[{}]", error_msg.as_ref());
            self.parts.truncate(1);
        }
    }

    pub fn get_rfc724_mid(&self) -> Option<String> {
        self.get(HeaderDef::MessageId)
            .and_then(|msgid| parse_message_id(msgid).ok())
    }

    fn merge_headers(
        context: &Context,
        headers: &mut HashMap<String, String>,
        recipients: &mut Vec<SingleInfo>,
        from: &mut Vec<SingleInfo>,
        chat_disposition_notification_to: &mut Option<SingleInfo>,
        fields: &[mailparse::MailHeader<'_>],
    ) {
        for field in fields {
            // lowercasing all headers is technically not correct, but makes things work better
            let key = field.get_key().to_lowercase();
            if !headers.contains_key(&key) || // key already exists, only overwrite known types (protected headers)
                    is_known(&key) || key.starts_with("chat-")
            {
                if key == HeaderDef::ChatDispositionNotificationTo.get_headername() {
                    match addrparse_header(field) {
                        Ok(addrlist) => {
                            *chat_disposition_notification_to = addrlist.extract_single_info();
                        }
                        Err(e) => warn!(context, "Could not read {} address: {}", key, e),
                    }
                } else {
                    let value = field.get_value();
                    headers.insert(key.to_string(), value);
                }
            }
        }
        let recipients_new = get_recipients(fields);
        if !recipients_new.is_empty() {
            *recipients = recipients_new;
        }
        let from_new = get_from(fields);
        if !from_new.is_empty() {
            *from = from_new;
        }
    }

    fn process_report(
        &self,
        context: &Context,
        report: &mailparse::ParsedMail<'_>,
    ) -> Result<Option<Report>> {
        // parse as mailheaders
        let report_body = if let Some(subpart) = report.subparts.get(1) {
            subpart.get_body_raw()?
        } else {
            bail!("Report does not have second MIME part");
        };
        let (report_fields, _) = mailparse::parse_headers(&report_body)?;

        // must be present
        if let Some(_disposition) = report_fields.get_header_value(HeaderDef::Disposition) {
            if let Some(original_message_id) = report_fields
                .get_header_value(HeaderDef::OriginalMessageId)
                .and_then(|v| parse_message_id(&v).ok())
            {
                let additional_message_ids = report_fields
                    .get_header_value(HeaderDef::AdditionalMessageIds)
                    .map_or_else(Vec::new, |v| {
                        v.split(' ')
                            .filter_map(|s| parse_message_id(s).ok())
                            .collect()
                    });

                return Ok(Some(Report {
                    original_message_id,
                    additional_message_ids,
                }));
            }
        }
        warn!(
            context,
            "ignoring unknown disposition-notification, Message-Id: {:?}",
            report_fields.get_header_value(HeaderDef::MessageId)
        );

        Ok(None)
    }

    fn process_delivery_status(
        &self,
        context: &Context,
        report: &mailparse::ParsedMail<'_>,
    ) -> Result<Option<FailureReport>> {
        // parse as mailheaders
        if let Some(original_msg) = report
            .subparts
            .iter()
            .find(|p| p.ctype.mimetype.contains("rfc822") || p.ctype.mimetype == "message/global")
        {
            let report_body = original_msg.get_body_raw()?;
            let (report_fields, _) = mailparse::parse_headers(&report_body)?;

            if let Some(original_message_id) = report_fields
                .get_header_value(HeaderDef::MessageId)
                .and_then(|v| parse_message_id(&v).ok())
            {
                let mut to_list = get_all_addresses_from_header(&report.headers, |header_key| {
                    header_key == "x-failed-recipients"
                });
                let to = if to_list.len() == 1 {
                    Some(to_list.pop().unwrap())
                } else {
                    None // We do not know which recipient failed
                };

                return Ok(Some(FailureReport {
                    rfc724_mid: original_message_id,
                    failed_recipient: to.map(|s| s.addr),
                }));
            }

            warn!(
                context,
                "ignoring unknown ndn-notification, Message-Id: {:?}",
                report_fields.get_header_value(HeaderDef::MessageId)
            );
        }

        Ok(None)
    }

    async fn maybe_remove_bad_parts(&mut self) {
        let good_parts = self.parts.iter().filter(|p| !p.dehtlm_failed).count();
        if good_parts == 0 {
            // We have no good part but show at least one bad part in order to show anything at all
            self.parts.truncate(1);
        } else if good_parts < self.parts.len() {
            self.parts.retain(|p| !p.dehtlm_failed);
        }
    }

    /// Some providers like GMX and Yahoo do not send standard NDNs (Non Delivery notifications).
    /// If you improve heuristics here you might also have to change prefetch_should_download() in imap/mod.rs.
    /// Also you should add a test in dc_receive_imf.rs (there already are lots of test_parse_ndn_* tests).
    #[allow(clippy::indexing_slicing)]
    async fn heuristically_parse_ndn(&mut self, context: &Context) {
        let maybe_ndn = if let Some(from) = self.get(HeaderDef::From_) {
            let from = from.to_ascii_lowercase();
            from.contains("mailer-daemon") || from.contains("mail-daemon")
        } else {
            false
        };
        if maybe_ndn && self.failure_report.is_none() {
            static RE: Lazy<regex::Regex> =
                Lazy::new(|| regex::Regex::new(r"Message-ID:(.*)").unwrap());
            for captures in self
                .parts
                .iter()
                .filter_map(|part| part.msg_raw.as_ref())
                .flat_map(|part| part.lines())
                .filter_map(|line| RE.captures(line))
            {
                if let Ok(original_message_id) = parse_message_id(&captures[1]) {
                    if let Ok(Some(_)) =
                        message::rfc724_mid_exists(context, &original_message_id).await
                    {
                        self.failure_report = Some(FailureReport {
                            rfc724_mid: original_message_id,
                            failed_recipient: None,
                        })
                    }
                }
            }
        }
    }

    /// Handle reports
    /// (MDNs = Message Disposition Notification, the message was read
    /// and NDNs = Non delivery notification, the message could not be delivered)
    pub async fn handle_reports(
        &self,
        context: &Context,
        from_id: u32,
        sent_timestamp: i64,
        parts: &[Part],
    ) {
        for report in &self.mdn_reports {
            for original_message_id in
                std::iter::once(&report.original_message_id).chain(&report.additional_message_ids)
            {
                if let Some((chat_id, msg_id)) =
                    message::handle_mdn(context, from_id, original_message_id, sent_timestamp).await
                {
                    context.emit_event(EventType::MsgRead { chat_id, msg_id });
                }
            }
        }

        if let Some(failure_report) = &self.failure_report {
            let error = parts
                .iter()
                .find(|p| p.typ == Viewtype::Text)
                .map(|p| p.msg.clone());
            if let Err(e) = message::handle_ndn(context, failure_report, error).await {
                warn!(context, "Could not handle ndn: {}", e);
            }
        }
    }

    /// Returns timestamp of the parent message.
    ///
    /// If there is no parent message or it is not found in the
    /// database, returns None.
    pub async fn get_parent_timestamp(&self, context: &Context) -> Result<Option<i64>> {
        let parent_timestamp = if let Some(field) = self
            .get(HeaderDef::InReplyTo)
            .and_then(|msgid| parse_message_id(msgid).ok())
        {
            context
                .sql
                .query_get_value_result(
                    "SELECT timestamp FROM msgs WHERE rfc724_mid=?",
                    paramsv![field],
                )
                .await?
        } else {
            None
        };
        Ok(parent_timestamp)
    }
}

async fn update_gossip_peerstates(
    context: &Context,
    message_time: i64,
    mail: &mailparse::ParsedMail<'_>,
    gossip_headers: Vec<String>,
) -> Result<HashSet<String>> {
    // XXX split the parsing from the modification part
    let mut gossipped_addr: HashSet<String> = Default::default();

    for value in &gossip_headers {
        let gossip_header = value.parse::<Aheader>();

        if let Ok(ref header) = gossip_header {
            if get_recipients(&mail.headers)
                .iter()
                .any(|info| info.addr == header.addr.to_lowercase())
            {
                let mut peerstate = Peerstate::from_addr(context, &header.addr).await?;
                if let Some(ref mut peerstate) = peerstate {
                    peerstate.apply_gossip(header, message_time);
                    peerstate.save_to_db(&context.sql, false).await?;
                } else {
                    let p = Peerstate::from_gossip(context, header, message_time);
                    p.save_to_db(&context.sql, true).await?;
                    peerstate = Some(p);
                }
                if let Some(peerstate) = peerstate {
                    peerstate.handle_fingerprint_change(context).await?;
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
pub(crate) struct Report {
    /// Original-Message-ID header
    original_message_id: String,
    /// Additional-Message-IDs
    additional_message_ids: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct FailureReport {
    pub rfc724_mid: String,
    pub failed_recipient: Option<String>,
}

#[allow(clippy::indexing_slicing)]
pub(crate) fn parse_message_ids(ids: &str) -> Result<Vec<String>> {
    // take care with mailparse::msgidparse() that is pretty untolerant eg. wrt missing `<` or `>`
    let mut msgids = Vec::new();
    for id in ids.split_whitespace() {
        let mut id = id.to_string();
        if id.starts_with('<') {
            id = id[1..].to_string();
        }
        if id.ends_with('>') {
            id = id[..id.len() - 1].to_string();
        }
        if !id.is_empty() {
            msgids.push(id);
        }
    }
    Ok(msgids)
}

pub(crate) fn parse_message_id(ids: &str) -> Result<String> {
    if let Some(id) = parse_message_ids(ids)?.first() {
        Ok(id.to_string())
    } else {
        bail!("could not parse message_id: {}", ids);
    }
}

fn is_known(key: &str) -> bool {
    matches!(
        key,
        "return-path"
            | "date"
            | "from"
            | "sender"
            | "reply-to"
            | "to"
            | "cc"
            | "bcc"
            | "message-id"
            | "in-reply-to"
            | "references"
            | "subject"
    )
}

#[derive(Debug, Default, Clone)]
pub struct Part {
    pub typ: Viewtype,
    pub mimetype: Option<Mime>,
    pub msg: String,
    pub msg_raw: Option<String>,
    pub bytes: usize,
    pub param: Params,
    org_filename: Option<String>,
    pub error: Option<String>,
    dehtlm_failed: bool,
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
    let ct = mail.get_content_disposition();
    ct.disposition == DispositionType::Attachment
        && ct
            .params
            .iter()
            .any(|(key, _value)| key.starts_with("filename"))
}

/// Tries to get attachment filename.
///
/// If filename is explitictly specified in Content-Disposition, it is
/// returned. If Content-Disposition is "attachment" but filename is
/// not specified, filename is guessed. If Content-Disposition cannot
/// be parsed, returns an error.
fn get_attachment_filename(mail: &mailparse::ParsedMail) -> Result<Option<String>> {
    // try to get file name from
    //    `Content-Disposition: ... filename*=...`
    // or `Content-Disposition: ... filename*0*=... filename*1*=... filename*2*=...`
    // or `Content-Disposition: ... filename=...`

    let ct = mail.get_content_disposition();

    let desired_filename: Option<String> = ct
        .params
        .iter()
        .filter(|(key, _value)| key.starts_with("filename"))
        .fold(None, |acc, (_key, value)| {
            if let Some(acc) = acc {
                Some(acc + value)
            } else {
                Some(value.to_string())
            }
        });

    let desired_filename =
        desired_filename.or_else(|| ct.params.get("name").map(|s| s.to_string()));

    // MS Outlook is known to specify filename in the "name" attribute of
    // Content-Type and omit Content-Disposition.
    let desired_filename =
        desired_filename.or_else(|| mail.ctype.params.get("name").map(|s| s.to_string()));

    // If there is no filename, but part is an attachment, guess filename
    if ct.disposition == DispositionType::Attachment && desired_filename.is_none() {
        if let Some(subtype) = mail.ctype.mimetype.split('/').nth(1) {
            Ok(Some(format!("file.{}", subtype,)))
        } else {
            bail!(
                "could not determine attachment filename: {:?}",
                ct.disposition
            );
        }
    } else {
        Ok(desired_filename)
    }
}

/// Returned addresses are normalized and lowercased.
pub(crate) fn get_recipients(headers: &[MailHeader]) -> Vec<SingleInfo> {
    get_all_addresses_from_header(headers, |header_key| {
        header_key == "to" || header_key == "cc" || header_key == "bcc"
    })
}

/// Returned addresses are normalized and lowercased.
pub(crate) fn get_from(headers: &[MailHeader]) -> Vec<SingleInfo> {
    get_all_addresses_from_header(headers, |header_key| header_key == "from")
}

fn get_all_addresses_from_header<F>(headers: &[MailHeader], pred: F) -> Vec<SingleInfo>
where
    F: Fn(String) -> bool,
{
    let mut result: Vec<SingleInfo> = Default::default();

    headers
        .iter()
        .filter(|header| pred(header.get_key().to_lowercase()))
        .filter_map(|header| mailparse::addrparse_header(header).ok())
        .for_each(|addrs| {
            for addr in addrs.iter() {
                match addr {
                    mailparse::MailAddr::Single(ref info) => {
                        result.push(SingleInfo {
                            addr: addr_normalize(&info.addr).to_lowercase(),
                            display_name: info.display_name.clone(),
                        });
                    }
                    mailparse::MailAddr::Group(ref infos) => {
                        for info in &infos.addrs {
                            result.push(SingleInfo {
                                addr: addr_normalize(&info.addr).to_lowercase(),
                                display_name: info.display_name.clone(),
                            });
                        }
                    }
                }
            }
        });

    result
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::test_utils::*;

    impl AvatarAction {
        pub fn is_change(&self) -> bool {
            match self {
                AvatarAction::Delete => false,
                AvatarAction::Change(_) => true,
            }
        }
    }

    #[async_std::test]
    async fn test_dc_mimeparser_crash() {
        let context = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();

        assert_eq!(mimeparser.get_subject(), None);
        assert_eq!(mimeparser.parts.len(), 1);
    }

    #[async_std::test]
    async fn test_get_rfc724_mid_exists() {
        let context = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/mail_with_message_id.txt");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();

        assert_eq!(
            mimeparser.get_rfc724_mid(),
            Some("2dfdbde7@example.org".into())
        );
    }

    #[async_std::test]
    async fn test_get_rfc724_mid_not_exists() {
        let context = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(mimeparser.get_rfc724_mid(), None);
    }

    #[test]
    fn test_get_recipients() {
        let raw = include_bytes!("../test-data/message/mail_with_cc.txt");
        let mail = mailparse::parse_mail(&raw[..]).unwrap();
        let recipients = get_recipients(&mail.headers);
        assert!(recipients.iter().any(|info| info.addr == "abc@bcd.com"));
        assert!(recipients.iter().any(|info| info.addr == "def@def.de"));
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
        assert!(get_attachment_filename(&mail).unwrap().is_none());
        assert!(get_attachment_filename(&mail.subparts[0])
            .unwrap()
            .is_none());
        let filename = get_attachment_filename(&mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("test.html".to_string()))
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

    #[async_std::test]
    async fn test_parse_first_addr() {
        let context = TestContext::new().await;
        let raw = b"From: hello@one.org, world@two.org\n\
                    Chat-Disposition-Notification-To: wrong\n\
                    Content-Type: text/plain\n\
                    Chat-Version: 1.0\n\
                    \n\
                    test1\n\
                    ";

        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();

        let of = &mimeparser.from[0];
        assert_eq!(of.addr, "hello@one.org");

        assert!(mimeparser.chat_disposition_notification_to.is_none());
    }

    #[async_std::test]
    async fn test_get_parent_timestamp() {
        let context = TestContext::new().await;
        let raw = b"From: foo@example.org\n\
                    Content-Type: text/plain\n\
                    Chat-Version: 1.0\n\
                    In-Reply-To: <Gr.beZgAF2Nn0-.oyaJOpeuT70@example.org>\n\
                    \n\
                    Some reply\n\
                    ";
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            mimeparser.get_parent_timestamp(&context.ctx).await.unwrap(),
            None
        );
        let timestamp = 1570435529;
        context
            .ctx
            .sql
            .execute(
                "INSERT INTO msgs (rfc724_mid, timestamp) VALUES(?,?)",
                paramsv!["Gr.beZgAF2Nn0-.oyaJOpeuT70@example.org", timestamp],
            )
            .await
            .expect("Failed to write to the database");
        assert_eq!(
            mimeparser.get_parent_timestamp(&context.ctx).await.unwrap(),
            Some(timestamp)
        );
    }

    #[async_std::test]
    async fn test_mimeparser_with_context() {
        let context = TestContext::new().await;
        let raw = b"From: hello\n\
                    Content-Type: multipart/mixed; boundary=\"==break==\";\n\
                    Subject: outer-subject\n\
                    Secure-Join-Group: no\n\
                    Secure-Join-Fingerprint: 123456\n\
                    Test-Header: Bar\n\
                    chat-VERSION: 0.0\n\
                    \n\
                    --==break==\n\
                    Content-Type: text/plain; protected-headers=\"v1\";\n\
                    Subject: inner-subject\n\
                    SecureBar-Join-Group: yes\n\
                    Test-Header: Xy\n\
                    chat-VERSION: 1.0\n\
                    \n\
                    test1\n\
                    \n\
                    --==break==--\n\
                    \n";

        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();

        // non-overwritten headers do not bubble up
        let of = mimeparser.get(HeaderDef::SecureJoinGroup).unwrap();
        assert_eq!(of, "no");

        // unknown headers do not bubble upwards
        let of = mimeparser.get(HeaderDef::_TestHeader).unwrap();
        assert_eq!(of, "Bar");

        // the following fields would bubble up
        // if the test would really use encryption for the protected part
        // however, as this is not the case, the outer things stay valid.
        // for Chat-Version, also the case-insensivity is tested.
        assert_eq!(mimeparser.get_subject(), Some("outer-subject".into()));

        let of = mimeparser.get(HeaderDef::ChatVersion).unwrap();
        assert_eq!(of, "0.0");
        assert_eq!(mimeparser.parts.len(), 1);

        // make sure, headers that are only allowed in the encrypted part
        // cannot be set from the outer part
        assert!(mimeparser.get(HeaderDef::SecureJoinFingerprint).is_none());
    }

    #[async_std::test]
    async fn test_mimeparser_with_avatars() {
        let t = TestContext::new().await;

        let raw = include_bytes!("../test-data/message/mail_attach_txt.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.user_avatar, None);
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.unwrap().is_change());
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar_deleted.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert_eq!(mimeparser.user_avatar, Some(AvatarAction::Delete));
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.unwrap().is_change());
        assert!(mimeparser.group_avatar.unwrap().is_change());

        // if the Chat-User-Avatar header is missing, the avatar become a normal attachment
        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let raw = String::from_utf8_lossy(raw).to_string();
        let raw = raw.replace("Chat-User-Avatar:", "Xhat-Xser-Xvatar:");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw.as_bytes())
            .await
            .unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Image);
        assert_eq!(mimeparser.user_avatar, None);
        assert!(mimeparser.group_avatar.unwrap().is_change());
    }

    #[async_std::test]
    async fn test_mimeparser_with_videochat() {
        let t = TestContext::new().await;

        let raw = include_bytes!("../test-data/message/videochat_invitation.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::VideochatInvitation);
        assert_eq!(
            mimeparser.parts[0]
                .param
                .get(Param::WebrtcRoom)
                .unwrap_or_default(),
            "https://example.org/p2p/?roomname=6HiduoAn4xN"
        );
        assert!(mimeparser.parts[0]
            .msg
            .contains("https://example.org/p2p/?roomname=6HiduoAn4xN"));
        assert_eq!(mimeparser.user_avatar, None);
        assert_eq!(mimeparser.group_avatar, None);
    }

    #[async_std::test]
    async fn test_mimeparser_message_kml() {
        let context = TestContext::new().await;
        let raw = b"Chat-Version: 1.0\n\
From: foo <foo@example.org>\n\
To: bar <bar@example.org>\n\
Subject: Location streaming\n\
Content-Type: multipart/mixed; boundary=\"==break==\"\n\
\n\
\n\
--==break==\n\
Content-Type: text/plain; charset=utf-8\n\
\n\
--\n\
Sent with my Delta Chat Messenger: https://delta.chat\n\
\n\
--==break==\n\
Content-Type: application/vnd.google-earth.kml+xml\n\
Content-Disposition: attachment; filename=\"message.kml\"\n\
\n\
<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n\
<Document addr=\"foo@example.org\">\n\
<Placemark><Timestamp><when>XXX</when></Timestamp><Point><coordinates accuracy=\"48\">0.0,0.0</coordinates></Point></Placemark>\n\
</Document>\n\
</kml>\n\
\n\
--==break==--\n\
;";

        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            mimeparser.get_subject(),
            Some("Location streaming".to_string())
        );
        assert!(mimeparser.location_kml.is_none());
        assert!(mimeparser.message_kml.is_some());

        // There is only one part because message.kml attachment is special
        // and only goes into message_kml.
        assert_eq!(mimeparser.parts.len(), 1);
    }

    #[async_std::test]
    async fn test_parse_mdn() {
        let context = TestContext::new().await;
        let raw = b"Subject: =?utf-8?q?Chat=3A_Message_opened?=\n\
Date: Mon, 10 Jan 2020 00:00:00 +0000\n\
Chat-Version: 1.0\n\
Message-ID: <bar@example.org>\n\
To: Alice <alice@example.org>\n\
From: Bob <bob@example.org>\n\
Content-Type: multipart/report; report-type=disposition-notification;\n\t\
boundary=\"kJBbU58X1xeWNHgBtTbMk80M5qnV4N\"\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N\n\
Content-Type: text/plain; charset=utf-8\n\
\n\
The \"Encrypted message\" message you sent was displayed on the screen of the recipient.\n\
\n\
This is no guarantee the content was read.\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N\n\
Content-Type: message/disposition-notification\n\
\n\
Reporting-UA: Delta Chat 1.0.0-beta.22\n\
Original-Recipient: rfc822;bob@example.org\n\
Final-Recipient: rfc822;bob@example.org\n\
Original-Message-ID: <foo@example.org>\n\
Disposition: manual-action/MDN-sent-automatically; displayed\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N--\n\
";

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Chat: Message opened".to_string())
        );

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.mdn_reports.len(), 1);
    }

    /// Test parsing multiple MDNs combined in a single message.
    ///
    /// RFC 6522 specifically allows MDNs to be nested inside
    /// multipart MIME messages.
    #[async_std::test]
    async fn test_parse_multiple_mdns() {
        let context = TestContext::new().await;
        let raw = b"Subject: =?utf-8?q?Chat=3A_Message_opened?=\n\
Date: Mon, 10 Jan 2020 00:00:00 +0000\n\
Chat-Version: 1.0\n\
Message-ID: <foo@example.org>\n\
To: Alice <alice@example.org>\n\
From: Bob <bob@example.org>\n\
Content-Type: multipart/parallel; boundary=outer\n\
\n\
This is a multipart MDN.\n\
\n\
--outer\n\
Content-Type: multipart/report; report-type=disposition-notification;\n\t\
boundary=kJBbU58X1xeWNHgBtTbMk80M5qnV4N\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N\n\
Content-Type: text/plain; charset=utf-8\n\
\n\
The \"Encrypted message\" message you sent was displayed on the screen of the recipient.\n\
\n\
This is no guarantee the content was read.\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N\n\
Content-Type: message/disposition-notification\n\
\n\
Reporting-UA: Delta Chat 1.0.0-beta.22\n\
Original-Recipient: rfc822;bob@example.org\n\
Final-Recipient: rfc822;bob@example.org\n\
Original-Message-ID: <bar@example.org>\n\
Disposition: manual-action/MDN-sent-automatically; displayed\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N--\n\
--outer\n\
Content-Type: multipart/report; report-type=disposition-notification;\n\t\
boundary=zuOJlsTfZAukyawEPVdIgqWjaM9w2W\n\
\n\
\n\
--zuOJlsTfZAukyawEPVdIgqWjaM9w2W\n\
Content-Type: text/plain; charset=utf-8\n\
\n\
The \"Encrypted message\" message you sent was displayed on the screen of the recipient.\n\
\n\
This is no guarantee the content was read.\n\
\n\
\n\
--zuOJlsTfZAukyawEPVdIgqWjaM9w2W\n\
Content-Type: message/disposition-notification\n\
\n\
Reporting-UA: Delta Chat 1.0.0-beta.22\n\
Original-Recipient: rfc822;bob@example.org\n\
Final-Recipient: rfc822;bob@example.org\n\
Original-Message-ID: <baz@example.org>\n\
Disposition: manual-action/MDN-sent-automatically; displayed\n\
\n\
\n\
--zuOJlsTfZAukyawEPVdIgqWjaM9w2W--\n\
--outer--\n\
";

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Chat: Message opened".to_string())
        );

        assert_eq!(message.parts.len(), 2);
        assert_eq!(message.mdn_reports.len(), 2);
    }

    #[async_std::test]
    async fn test_parse_mdn_with_additional_message_ids() {
        let context = TestContext::new().await;
        let raw = b"Subject: =?utf-8?q?Chat=3A_Message_opened?=\n\
Date: Mon, 10 Jan 2020 00:00:00 +0000\n\
Chat-Version: 1.0\n\
Message-ID: <bar@example.org>\n\
To: Alice <alice@example.org>\n\
From: Bob <bob@example.org>\n\
Content-Type: multipart/report; report-type=disposition-notification;\n\t\
boundary=\"kJBbU58X1xeWNHgBtTbMk80M5qnV4N\"\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N\n\
Content-Type: text/plain; charset=utf-8\n\
\n\
The \"Encrypted message\" message you sent was displayed on the screen of the recipient.\n\
\n\
This is no guarantee the content was read.\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N\n\
Content-Type: message/disposition-notification\n\
\n\
Reporting-UA: Delta Chat 1.0.0-beta.22\n\
Original-Recipient: rfc822;bob@example.org\n\
Final-Recipient: rfc822;bob@example.org\n\
Original-Message-ID: <foo@example.org>\n\
Disposition: manual-action/MDN-sent-automatically; displayed\n\
Additional-Message-IDs: <foo@example.com> <foo@example.net>\n\
\n\
\n\
--kJBbU58X1xeWNHgBtTbMk80M5qnV4N--\n\
";

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Chat: Message opened".to_string())
        );

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.mdn_reports.len(), 1);
        assert_eq!(
            message.mdn_reports[0].original_message_id,
            "foo@example.org"
        );
        assert_eq!(
            &message.mdn_reports[0].additional_message_ids,
            &["foo@example.com", "foo@example.net"]
        );
    }

    #[async_std::test]
    async fn test_parse_inline_attachment() {
        let context = TestContext::new().await;
        let raw = br#"Date: Thu, 13 Feb 2020 22:41:20 +0000 (UTC)
From: sender@example.com
To: receiver@example.com
Subject: Mail with inline attachment
MIME-Version: 1.0
Content-Type: multipart/mixed;
	boundary="----=_Part_25_46172632.1581201680436"

------=_Part_25_46172632.1581201680436
Content-Type: text/plain; charset=utf-8

Hello!

------=_Part_25_46172632.1581201680436
Content-Type: application/pdf; name="some_pdf.pdf"
Content-Transfer-Encoding: base64
Content-Disposition: inline; filename="some_pdf.pdf"

JVBERi0xLjUKJcOkw7zDtsOfCjIgMCBvYmoKPDwvTGVuZ3RoIDMgMCBSL0ZpbHRlci9GbGF0ZURl
Y29kZT4+CnN0cmVhbQp4nGVOuwoCMRDs8xVbC8aZvC4Hx4Hno7ATAhZi56MTtPH33YtXiLKQ3ZnM
MDYyMDYxNTE1RTlDOEE4Cj4+CnN0YXJ0eHJlZgo4Mjc4CiUlRU9GCg==
------=_Part_25_46172632.1581201680436--
"#;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Mail with inline attachment".to_string())
        );

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::File);
        assert_eq!(message.parts[0].msg, "Hello!");
    }

    #[async_std::test]
    async fn test_hide_html_without_content() {
        let t = TestContext::new().await;
        let raw = br#"Date: Thu, 13 Feb 2020 22:41:20 +0000 (UTC)
From: sender@example.com
To: receiver@example.com
Subject: Mail with inline attachment
MIME-Version: 1.0
Content-Type: multipart/mixed;
	boundary="----=_Part_25_46172632.1581201680436"

------=_Part_25_46172632.1581201680436
Content-Type: text/html; charset=utf-8

<head>
<meta http-equiv="Content-Type" content="text/html; charset=Windows-1252">
<meta name="GENERATOR" content="MSHTML 11.00.10570.1001"></head>
<body><img align="baseline" alt="" src="cid:1712254131-1" border="0" hspace="0">
</body>

------=_Part_25_46172632.1581201680436
Content-Type: application/pdf; name="some_pdf.pdf"
Content-Transfer-Encoding: base64
Content-Disposition: inline; filename="some_pdf.pdf"

JVBERi0xLjUKJcOkw7zDtsOfCjIgMCBvYmoKPDwvTGVuZ3RoIDMgMCBSL0ZpbHRlci9GbGF0ZURl
Y29kZT4+CnN0cmVhbQp4nGVOuwoCMRDs8xVbC8aZvC4Hx4Hno7ATAhZi56MTtPH33YtXiLKQ3ZnM
MDYyMDYxNTE1RTlDOEE4Cj4+CnN0YXJ0eHJlZgo4Mjc4CiUlRU9GCg==
------=_Part_25_46172632.1581201680436--
"#;

        let message = MimeMessage::from_bytes(&t.ctx, &raw[..]).await.unwrap();

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::File);
        assert_eq!(message.parts[0].msg, "");

        // Make sure the file is there even though the html is wrong:
        let param = &message.parts[0].param;
        let blob: BlobObject = param
            .get_blob(Param::File, &t.ctx, false)
            .await
            .unwrap()
            .unwrap();
        let f = async_std::fs::File::open(blob.to_abs_path()).await.unwrap();
        let size = f.metadata().await.unwrap().len();
        assert_eq!(size, 154);
    }

    #[async_std::test]
    async fn parse_inline_image() {
        let context = TestContext::new().await;
        let raw = br#"Message-ID: <foobar@example.org>
From: foo <foo@example.org>
Subject: example
To: bar@example.org
MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="--11019878869865180"

----11019878869865180
Content-Type: text/plain; charset=utf-8

Test

----11019878869865180
Content-Type: image/jpeg;
 name="JPEG_filename.jpg"
Content-Transfer-Encoding: base64
Content-Disposition: inline;
 filename="JPEG_filename.jpg"

ISVb1L3m7z15Wy5w97a2cJg6W8P8YKOYfWn3PJ/UCSFcvCPtvBhcXieiN3M3ljguzG4XK7BnGgxG
acAQdY8e0cWz1n+zKPNeNn4Iu3GXAXz4/IPksHk54inl1//0Lv8ggZjljfjnf0q1SPftYI7lpZWT
/4aTCkimRrAIcwrQJPnZJRb7BPSC6kfn1QJHMv77mRMz2+4WbdfpyPQQ0CWLJsgVXtBsSMf2Awal
n+zZzhGpXyCbWTEw1ccqZcK5KaiKNqWv51N4yVXw9dzJoCvxbYtCFGZZJdx7c+ObDotaF1/9KY4C
xJjgK9/NgTXCZP1jYm0XIBnJsFSNg0pnMRETttTuGbOVi1/s/F1RGv5RNZsCUt21d9FhkWQQXsd2
rOzDgTdag6BQCN3hSU9eKW/GhNBuMibRN9eS7Sm1y2qFU1HgGJBQfPPRPLKxXaNi++Zt0tnon2IU
8pg5rP/IvStXYQNUQ9SiFdfAUkLU5b1j8ltnka8xl+oXsleSG44GPz6kM0RmwUrGkl4z/+NfHSsI
K+TuvC7qOah0WLFhcsXWn2+dDV1bXuAeC769TkqkpHhdXfUHnVgK3Pv7u3rVPT5AMeFUGxRB2dP4
CWt6wx7fiLp0qS9RrX75g6Gqw7nfCs6EcBERcIPt7DTe8VStJwf3LWqVwxl4gQl46yhfoqwEO+I=


----11019878869865180--
"#;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(message.get_subject(), Some("example".to_string()));

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Image);
        assert_eq!(message.parts[0].msg, "Test");
    }

    #[async_std::test]
    async fn parse_thunderbird_html_embedded_image() {
        let context = TestContext::new().await;
        let raw = br#"To: Alice <alice@example.org>
From: Bob <bob@example.org>
Subject: Test subject
Message-ID: <foobarbaz@example.org>
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:68.0) Gecko/20100101
 Thunderbird/68.7.0
MIME-Version: 1.0
Content-Type: multipart/alternative;
 boundary="------------779C1631600DF3DB8C02E53A"
Content-Language: en-US

This is a multi-part message in MIME format.
--------------779C1631600DF3DB8C02E53A
Content-Type: text/plain; charset=utf-8
Content-Transfer-Encoding: 7bit

Test


--------------779C1631600DF3DB8C02E53A
Content-Type: multipart/related;
 boundary="------------10CC6C2609EB38DA782C5CA9"


--------------10CC6C2609EB38DA782C5CA9
Content-Type: text/html; charset=utf-8
Content-Transfer-Encoding: 7bit

<html>
<head>
<meta http-equiv="content-type" content="text/html; charset=UTF-8">
</head>
<body>
Test<br>
<p><img moz-do-not-send="false" src="cid:part1.9DFA679B.52A88D69@example.org" alt=""></p>
</body>
</html>

--------------10CC6C2609EB38DA782C5CA9
Content-Type: image/png;
 name="1.png"
Content-Transfer-Encoding: base64
Content-ID: <part1.9DFA679B.52A88D69@example.org>
Content-Disposition: inline;
 filename="1.png"

ISVb1L3m7z15Wy5w97a2cJg6W8P8YKOYfWn3PJ/UCSFcvCPtvBhcXieiN3M3ljguzG4XK7BnGgxG
acAQdY8e0cWz1n+zKPNeNn4Iu3GXAXz4/IPksHk54inl1//0Lv8ggZjljfjnf0q1SPftYI7lpZWT
/4aTCkimRrAIcwrQJPnZJRb7BPSC6kfn1QJHMv77mRMz2+4WbdfpyPQQ0CWLJsgVXtBsSMf2Awal
n+zZzhGpXyCbWTEw1ccqZcK5KaiKNqWv51N4yVXw9dzJoCvxbYtCFGZZJdx7c+ObDotaF1/9KY4C
xJjgK9/NgTXCZP1jYm0XIBnJsFSNg0pnMRETttTuGbOVi1/s/F1RGv5RNZsCUt21d9FhkWQQXsd2
rOzDgTdag6BQCN3hSU9eKW/GhNBuMibRN9eS7Sm1y2qFU1HgGJBQfPPRPLKxXaNi++Zt0tnon2IU
8pg5rP/IvStXYQNUQ9SiFdfAUkLU5b1j8ltnka8xl+oXsleSG44GPz6kM0RmwUrGkl4z/+NfHSsI
K+TuvC7qOah0WLFhcsXWn2+dDV1bXuAeC769TkqkpHhdXfUHnVgK3Pv7u3rVPT5AMeFUGxRB2dP4
CWt6wx7fiLp0qS9RrX75g6Gqw7nfCs6EcBERcIPt7DTe8VStJwf3LWqVwxl4gQl46yhfoqwEO+I=
--------------10CC6C2609EB38DA782C5CA9--

--------------779C1631600DF3DB8C02E53A--"#;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(message.get_subject(), Some("Test subject".to_string()));

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Image);
        assert_eq!(message.parts[0].msg, "Test");
    }

    // Outlook specifies filename in the "name" attribute of Content-Type
    #[async_std::test]
    async fn parse_outlook_html_embedded_image() {
        let context = TestContext::new().await;
        let raw = br##"From: Anonymous <anonymous@example.org>
To: Anonymous <anonymous@example.org>
Subject: Delta Chat is great stuff!
Date: Tue, 5 May 2020 01:23:45 +0000
MIME-Version: 1.0
Content-Type: multipart/related;
	boundary="----=_NextPart_000_0003_01D622B3.CA753E60"
X-Mailer: Microsoft Outlook 15.0

This is a multipart message in MIME format.

------=_NextPart_000_0003_01D622B3.CA753E60
Content-Type: multipart/alternative;
	boundary="----=_NextPart_001_0004_01D622B3.CA753E60"


------=_NextPart_001_0004_01D622B3.CA753E60
Content-Type: text/plain;
	charset="us-ascii"
Content-Transfer-Encoding: 7bit




------=_NextPart_001_0004_01D622B3.CA753E60
Content-Type: text/html;
	charset="us-ascii"
Content-Transfer-Encoding: quoted-printable

<html>
<body>
<p>
Test<img src="cid:image001.jpg@01D622B3.C9D8D750">
</p>
</body>
</html>
------=_NextPart_001_0004_01D622B3.CA753E60--

------=_NextPart_000_0003_01D622B3.CA753E60
Content-Type: image/jpeg;
	name="image001.jpg"
Content-Transfer-Encoding: base64
Content-ID: <image001.jpg@01D622B3.C9D8D750>

ISVb1L3m7z15Wy5w97a2cJg6W8P8YKOYfWn3PJ/UCSFcvCPtvBhcXieiN3M3ljguzG4XK7BnGgxG
acAQdY8e0cWz1n+zKPNeNn4Iu3GXAXz4/IPksHk54inl1//0Lv8ggZjljfjnf0q1SPftYI7lpZWT
/4aTCkimRrAIcwrQJPnZJRb7BPSC6kfn1QJHMv77mRMz2+4WbdfpyPQQ0CWLJsgVXtBsSMf2Awal
n+zZzhGpXyCbWTEw1ccqZcK5KaiKNqWv51N4yVXw9dzJoCvxbYtCFGZZJdx7c+ObDotaF1/9KY4C
xJjgK9/NgTXCZP1jYm0XIBnJsFSNg0pnMRETttTuGbOVi1/s/F1RGv5RNZsCUt21d9FhkWQQXsd2
rOzDgTdag6BQCN3hSU9eKW/GhNBuMibRN9eS7Sm1y2qFU1HgGJBQfPPRPLKxXaNi++Zt0tnon2IU
8pg5rP/IvStXYQNUQ9SiFdfAUkLU5b1j8ltnka8xl+oXsleSG44GPz6kM0RmwUrGkl4z/+NfHSsI
K+TuvC7qOah0WLFhcsXWn2+dDV1bXuAeC769TkqkpHhdXfUHnVgK3Pv7u3rVPT5AMeFUGxRB2dP4
CWt6wx7fiLp0qS9RrX75g6Gqw7nfCs6EcBERcIPt7DTe8VStJwf3LWqVwxl4gQl46yhfoqwEO+I=

------=_NextPart_000_0003_01D622B3.CA753E60--
"##;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Delta Chat is great stuff!".to_string())
        );

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Image);
        assert_eq!(message.parts[0].msg, "Test");
    }

    #[test]
    fn test_parse_message_id() {
        let test = parse_message_id("<foobar>");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "foobar");

        let test = parse_message_id("<foo> <bar>");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "foo");

        let test = parse_message_id("  < foo > <bar>");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "foo");

        let test = parse_message_id("foo");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "foo");

        let test = parse_message_id(" foo ");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "foo");

        let test = parse_message_id("foo bar");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "foo");

        let test = parse_message_id("  foo  bar ");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "foo");

        let test = parse_message_id("");
        assert!(test.is_err());

        let test = parse_message_id(" ");
        assert!(test.is_err());

        let test = parse_message_id("<>");
        assert!(test.is_err());

        let test = parse_message_id("<> bar");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), "bar");
    }

    #[test]
    fn test_parse_message_ids() {
        let test = parse_message_ids("  foo  bar <foobar>").unwrap();
        assert_eq!(test.len(), 3);
        assert_eq!(test[0], "foo");
        assert_eq!(test[1], "bar");
        assert_eq!(test[2], "foobar");

        let test = parse_message_ids("  < foobar >").unwrap();
        assert_eq!(test.len(), 1);
        assert_eq!(test[0], "foobar");

        let test = parse_message_ids("").unwrap();
        assert!(test.is_empty());

        let test = parse_message_ids(" ").unwrap();
        assert!(test.is_empty());

        let test = parse_message_ids("  < ").unwrap();
        assert!(test.is_empty());
    }

    #[async_std::test]
    async fn parse_format_flowed_quote() {
        let context = TestContext::new().await;
        let raw = br##"Content-Type: text/plain; charset=utf-8; format=flowed; delsp=no
Subject: Re: swipe-to-reply
MIME-Version: 1.0
In-Reply-To: <bar@example.org>
Date: Tue, 06 Oct 2020 00:00:00 +0000
Chat-Version: 1.0
Message-ID: <foo@example.org>
To: bob <bob@example.org>
From: alice <alice@example.org>

> Long 
> quote.

Reply
"##;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Re: swipe-to-reply".to_string())
        );

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Text);
        assert_eq!(
            message.parts[0].param.get(Param::Quote).unwrap(),
            "Long quote."
        );
        assert_eq!(message.parts[0].msg, "Reply");
    }

    #[async_std::test]
    async fn parse_quote_without_reply() {
        let context = TestContext::new().await;
        let raw = br##"Content-Type: text/plain; charset=utf-8; format=flowed; delsp=no
Subject: Re: swipe-to-reply
MIME-Version: 1.0
In-Reply-To: <bar@example.org>
Date: Tue, 06 Oct 2020 00:00:00 +0000
Message-ID: <foo@example.org>
To: bob <bob@example.org>
From: alice <alice@example.org>

> Just a quote.
"##;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Re: swipe-to-reply".to_string())
        );

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Text);
        assert_eq!(
            message.parts[0].param.get(Param::Quote).unwrap(),
            "Just a quote."
        );
        assert_eq!(message.parts[0].msg, "");
    }

    #[async_std::test]
    async fn parse_quote_top_posting() {
        let context = TestContext::new().await;
        let raw = br##"Content-Type: text/plain; charset=utf-8; format=flowed; delsp=no
Subject: Re: top posting
MIME-Version: 1.0
In-Reply-To: <bar@example.org>
Message-ID: <foo@example.org>
To: bob <bob@example.org>
From: alice <alice@example.org>

A reply.

On 2020-10-25, Bob wrote:
> A quote.
"##;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(message.get_subject(), Some("Re: top posting".to_string()));

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Text);
        assert_eq!(
            message.parts[0].param.get(Param::Quote).unwrap(),
            "A quote."
        );
        assert_eq!(message.parts[0].msg, "A reply.");
    }

    #[async_std::test]
    async fn test_attachment_quote() {
        let context = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/quote_attach.eml");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();

        assert_eq!(mimeparser.get_subject().unwrap(), "Message from Alice");
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].msg, "Reply");
        assert_eq!(
            mimeparser.parts[0].param.get(Param::Quote).unwrap(),
            "Quote"
        );
        assert_eq!(mimeparser.parts[0].typ, Viewtype::File);
    }
}

//! # MIME message parsing module.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use anyhow::{bail, Result};
use deltachat_derive::{FromSql, ToSql};
use lettre_email::mime::{self, Mime};
use mailparse::{addrparse_header, DispositionType, MailHeader, MailHeaderMap, SingleInfo};
use once_cell::sync::Lazy;

use crate::aheader::Aheader;
use crate::blob::BlobObject;
use crate::constants::{Viewtype, DC_DESIRED_TEXT_LEN, DC_ELLIPSIS};
use crate::contact::addr_normalize;
use crate::context::Context;
use crate::dc_tools::{dc_get_filemeta, dc_truncate};
use crate::dehtml::dehtml;
use crate::e2ee;
use crate::events::EventType;
use crate::format_flowed::unformat_flowed;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::key::Fingerprint;
use crate::location;
use crate::message;
use crate::param::{Param, Params};
use crate::peerstate::Peerstate;
use crate::simplify::simplify;
use crate::stock_str;

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

    /// Standard USENET signature, if any.
    pub(crate) footer: Option<String>,

    // if this flag is set, the parts/text/etc. are just close to the original mime-message;
    // clients should offer a way to view the original message in this case
    pub is_mime_modified: bool,

    /// The decrypted, raw mime structure.
    ///
    /// This is non-empty only if the message was actually encrypted.  It is used
    /// for e.g. late-parsing HTML.
    pub decoded_data: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum AvatarAction {
    Delete,
    Change(String),
}

#[derive(Debug, PartialEq)]
pub(crate) enum MailinglistType {
    /// The message belongs to a mailing list and has a `ListId:`-header
    /// that should be used to get a unique id.
    ListIdBased,

    /// The message belongs to a mailing list, but there is no `ListId:`-header;
    /// `Sender:`-header should be used to get a unique id.
    /// This method is used by implementations as Majordomo.
    /// Note, that the `Sender:` header alone is not sufficient to detect these lists,
    /// `get_mailinglist_type()` check additional conditions therefore.
    SenderBased,

    /// The message does not belong to a mailing list.
    None,
}

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql,
)]
#[repr(u32)]
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

        // Parse IMF headers.
        MimeMessage::merge_headers(
            context,
            &mut headers,
            &mut recipients,
            &mut from,
            &mut chat_disposition_notification_to,
            &mail.headers,
        );

        // Parse hidden headers.
        let mimetype = mail.ctype.mimetype.parse::<Mime>()?;
        if mimetype.type_() == mime::MULTIPART && mimetype.subtype().as_str() == "mixed" {
            if let Some(part) = mail.subparts.first() {
                for field in &part.headers {
                    let key = field.get_key().to_lowercase();

                    // For now only Chat-User-Avatar can be hidden.
                    if !headers.contains_key(&key) && key == "chat-user-avatar" {
                        headers.insert(key.to_string(), field.get_value());
                    }
                }
            }
        }

        // remove headers that are allowed _only_ in the encrypted part
        headers.remove("secure-join-fingerprint");
        headers.remove("chat-verified");

        // Memory location for a possible decrypted message.
        let mut mail_raw = Vec::new();
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
                        // "3.6 Key Gossip" of <https://autocrypt.org/autocrypt-spec-1.1.0.pdf>
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
                        // See <https://github.com/deltachat/deltachat-core-rust/issues/1790>.
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
            footer: None,
            is_mime_modified: false,
            decoded_data: Vec::new(),
        };
        parser.parse_mime_recursive(context, &mail, false).await?;
        parser.maybe_remove_bad_parts();
        parser.maybe_remove_inline_mailinglist_footer();
        parser.heuristically_parse_ndn(context).await;
        parser.parse_headers(context).await?;

        if warn_empty_signature && parser.signatures.is_empty() {
            for part in parser.parts.iter_mut() {
                part.error = Some("No valid signature".to_string());
            }
        }

        if parser.is_mime_modified {
            parser.decoded_data = mail_raw;
        }

        Ok(parser)
    }

    /// Parses system messages.
    fn parse_system_message_headers(&mut self, context: &Context) {
        if self.get_header(HeaderDef::AutocryptSetupMessage).is_some() {
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
        } else if let Some(value) = self.get_header(HeaderDef::ChatContent) {
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
    }

    /// Parses avatar action headers.
    async fn parse_avatar_headers(&mut self, context: &Context) {
        if let Some(header_value) = self.get_header(HeaderDef::ChatGroupAvatar).cloned() {
            self.group_avatar = self.avatar_action_from_header(context, header_value).await;
        }

        if let Some(header_value) = self.get_header(HeaderDef::ChatUserAvatar).cloned() {
            self.user_avatar = self.avatar_action_from_header(context, header_value).await;
        }
    }

    fn parse_videochat_headers(&mut self) {
        if let Some(value) = self.get_header(HeaderDef::ChatContent).cloned() {
            if value == "videochat-invitation" {
                let instance = self.get_header(HeaderDef::ChatWebrtcRoom).cloned();
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
            if part.typ == Viewtype::Audio && self.get_header(HeaderDef::ChatVoiceMessage).is_some()
            {
                part.typ = Viewtype::Voice;
            }
            if part.typ == Viewtype::Image || part.typ == Viewtype::Gif {
                if let Some(value) = self.get_header(HeaderDef::ChatContent) {
                    if value == "sticker" {
                        part.typ = Viewtype::Sticker;
                    }
                }
            }
            if part.typ == Viewtype::Audio
                || part.typ == Viewtype::Voice
                || part.typ == Viewtype::Video
            {
                if let Some(field_0) = self.get_header(HeaderDef::ChatDuration) {
                    let duration_ms = field_0.parse().unwrap_or_default();
                    if duration_ms > 0 && duration_ms < 24 * 60 * 60 * 1000 {
                        part.param.set_int(Param::Duration, duration_ms);
                    }
                }
            }

            self.parts.push(part);
        }
    }

    async fn parse_headers(&mut self, context: &Context) -> Result<()> {
        self.parse_system_message_headers(context);
        self.parse_avatar_headers(context).await;
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

            // For mailing lists, always add the subject because sometimes there are different topics
            // and otherwise it might be hard to keep track:
            if self.is_mailinglist_message() {
                prepend_subject = true;
            }

            if prepend_subject && !subject.is_empty() {
                let part_with_text = self.parts.iter_mut().find(|part| !part.msg.is_empty());
                if let Some(mut part) = part_with_text {
                    part.msg = format!("{} – {}", subject, part.msg);
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
                if let Some(from) = self.from.get(0) {
                    // Check that the message is not outgoing.
                    if !context.is_self_addr(&from.addr).await? {
                        if from.addr.to_lowercase() == dn_to.addr.to_lowercase() {
                            if let Some(part) = self.parts.last_mut() {
                                part.param.set_int(Param::WantsMdn, 1);
                            }
                        } else {
                            warn!(
                                context,
                                "{} requested a read receipt to {}, ignoring",
                                from.addr,
                                dn_to.addr
                            );
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
            let mut part = Part {
                typ: Viewtype::Text,
                ..Default::default()
            };

            if let Some(ref subject) = self.get_subject() {
                if !self.has_chat_version() {
                    part.msg = subject.to_string();
                }
            }

            self.parts.push(part);
        }

        if self.header.contains_key("auto-submitted") {
            for part in &mut self.parts {
                part.param.set(Param::Bot, "1");
            }
        }

        Ok(())
    }

    async fn avatar_action_from_header(
        &mut self,
        context: &Context,
        header_value: String,
    ) -> Option<AvatarAction> {
        if header_value == "0" {
            Some(AvatarAction::Delete)
        } else if let Some(avatar) = header_value
            .split_ascii_whitespace()
            .collect::<String>()
            .strip_prefix("base64:")
            .map(base64::decode)
        {
            // Avatar sent directly in the header as base64.
            if let Ok(decoded_data) = avatar {
                let extension = if let Ok(format) = image::guess_format(&decoded_data) {
                    if let Some(ext) = format.extensions_str().first() {
                        format!(".{}", ext)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                match BlobObject::create(context, &format!("avatar{}", extension), &decoded_data)
                    .await
                {
                    Ok(blob) => Some(AvatarAction::Change(blob.as_name().to_string())),
                    Err(err) => {
                        warn!(
                            context,
                            "Could not save decoded avatar to blob file: {}", err
                        );
                        None
                    }
                }
            } else {
                None
            }
        } else {
            // Avatar sent in attachment, as previous versions of Delta Chat did.

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
        self.get_header(HeaderDef::Subject)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    pub fn get_header(&self, headerdef: HeaderDef) -> Option<&String> {
        self.header.get(headerdef.get_headername())
    }

    fn parse_mime_recursive<'a>(
        &'a mut self,
        context: &'a Context,
        mail: &'a mailparse::ParsedMail<'a>,
        is_related: bool,
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

            let is_related = is_related || mimetype == "multipart/related";
            match m {
                MimeS::Multiple => self.handle_multiple(context, mail, is_related).await,
                MimeS::Message => {
                    let raw = mail.get_body_raw()?;
                    if raw.is_empty() {
                        return Ok(false);
                    }
                    let mail = mailparse::parse_mail(&raw).unwrap();

                    self.parse_mime_recursive(context, &mail, is_related).await
                }
                MimeS::Single => self.add_single_part_if_known(context, mail, is_related).await,
            }
        }
        .boxed()
    }

    async fn handle_multiple(
        &mut self,
        context: &Context,
        mail: &mailparse::ParsedMail<'_>,
        is_related: bool,
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
                        any_part_added = self
                            .parse_mime_recursive(context, cur_data, is_related)
                            .await?;
                        break;
                    }
                }
                if !any_part_added {
                    /* search for text/plain and add this */
                    for cur_data in &mail.subparts {
                        if get_mime_type(cur_data)?.0.type_() == mime::TEXT {
                            any_part_added = self
                                .parse_mime_recursive(context, cur_data, is_related)
                                .await?;
                            break;
                        }
                    }
                }
                if !any_part_added {
                    /* `text/plain` not found - use the first part */
                    for cur_part in &mail.subparts {
                        if self
                            .parse_mime_recursive(context, cur_part, is_related)
                            .await?
                        {
                            any_part_added = true;
                            break;
                        }
                    }
                }
                if any_part_added && mail.subparts.len() > 1 {
                    // there are other alternative parts, likely HTML,
                    // so we might have missed some content on simplifying.
                    // set mime-modified to force the ui to display a show-message button.
                    self.is_mime_modified = true;
                }
            }
            (mime::MULTIPART, "encrypted") => {
                // we currently do not try to decrypt non-autocrypt messages
                // at all. If we see an encrypted part, we set
                // decrypting_failed.
                let msg_body = stock_str::cant_decrypt_msg_body(context).await;
                let txt = format!("[{}]", msg_body);

                let part = Part {
                    typ: Viewtype::Text,
                    msg_raw: Some(txt.clone()),
                    msg: txt,
                    error: Some("Decryption failed".to_string()),
                    ..Default::default()
                };
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
                <https://k9mail.github.io/2016/11/24/OpenPGP-Considerations-Part-I.html>
                for background information why we use encrypted+signed) */
                if let Some(first) = mail.subparts.get(0) {
                    any_part_added = self
                        .parse_mime_recursive(context, first, is_related)
                        .await?;
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
                            let part = Part {
                                typ: Viewtype::Unknown,
                                ..Default::default()
                            };
                            self.parts.push(part);

                            any_part_added = true;
                        }
                        // Some providers, e.g. Tiscali, forget to set the report-type. So, if it's None, assume that it might be delivery-status
                        Some("delivery-status") | None => {
                            if let Some(report) = self.process_delivery_status(context, mail)? {
                                self.failure_report = Some(report);
                            }

                            // Add all parts (we need another part, preferably text/plain, to show as an error message)
                            for cur_data in mail.subparts.iter() {
                                if self
                                    .parse_mime_recursive(context, cur_data, is_related)
                                    .await?
                                {
                                    any_part_added = true;
                                }
                            }
                        }
                        Some(_) => {
                            if let Some(first) = mail.subparts.get(0) {
                                any_part_added = self
                                    .parse_mime_recursive(context, first, is_related)
                                    .await?;
                            }
                        }
                    }
                }
            }
            _ => {
                // Add all parts (in fact, AddSinglePartIfKnown() later check if
                // the parts are really supported)
                for cur_data in mail.subparts.iter() {
                    if self
                        .parse_mime_recursive(context, cur_data, is_related)
                        .await?
                    {
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
        is_related: bool,
    ) -> Result<bool> {
        // return true if a part was added
        let (mime_type, msg_type) = get_mime_type(mail)?;
        let raw_mime = mail.ctype.mimetype.to_lowercase();

        let filename = get_attachment_filename(context, mail)?;

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
                    is_related,
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

                        let (simplified_txt, is_forwarded, is_cut, top_quote, footer) =
                            if decoded_data.is_empty() {
                                ("".to_string(), false, false, None, None)
                            } else {
                                let is_html = mime_type == mime::TEXT_HTML;
                                let out = if is_html {
                                    self.is_mime_modified = true;
                                    dehtml(&decoded_data).unwrap_or_else(|| {
                                        dehtml_failed = true;
                                        decoded_data.clone()
                                    })
                                } else {
                                    decoded_data.clone()
                                };
                                simplify(out, self.has_chat_version())
                            };

                        self.is_mime_modified = self.is_mime_modified
                            || ((is_forwarded || is_cut || top_quote.is_some())
                                && !self.has_chat_version());

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

                        let simplified_txt = if simplified_txt.chars().count()
                            > DC_DESIRED_TEXT_LEN + DC_ELLIPSIS.len()
                        {
                            self.is_mime_modified = true;
                            dc_truncate(&*simplified_txt, DC_DESIRED_TEXT_LEN).to_string()
                        } else {
                            simplified_txt
                        };

                        if !simplified_txt.is_empty() || simplified_quote.is_some() {
                            let mut part = Part {
                                dehtml_failed,
                                typ: Viewtype::Text,
                                mimetype: Some(mime_type),
                                msg: simplified_txt,
                                ..Default::default()
                            };
                            if let Some(quote) = simplified_quote {
                                part.param.set(Param::Quote, quote);
                            }
                            part.msg_raw = Some(decoded_data);
                            self.do_add_single_part(part);
                        }

                        if is_forwarded {
                            self.is_forwarded = true;
                        }

                        self.footer = footer;
                    }
                    _ => {}
                }
            }
        }

        // add object? (we do not add all objects, eg. signatures etc. are ignored)
        Ok(self.parts.len() > old_part_count)
    }

    #[allow(clippy::too_many_arguments)]
    async fn do_add_single_file_part(
        &mut self,
        context: &Context,
        msg_type: Viewtype,
        mime_type: Mime,
        raw_mime: &str,
        decoded_data: &[u8],
        filename: &str,
        is_related: bool,
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
        part.is_related = is_related;

        self.do_add_single_part(part);
    }

    fn do_add_single_part(&mut self, mut part: Part) {
        if self.was_encrypted() {
            part.param.set_int(Param::GuaranteeE2ee, 1);
        }
        self.parts.push(part);
    }

    pub(crate) fn get_mailinglist_type(&self) -> MailinglistType {
        if self.get_header(HeaderDef::ListId).is_some() {
            return MailinglistType::ListIdBased;
        } else if self.get_header(HeaderDef::Sender).is_some() {
            // the `Sender:`-header alone is no indicator for mailing list
            // as also used for bot-impersonation via `set_override_sender_name()`
            if let Some(precedence) = self.get_header(HeaderDef::Precedence) {
                if precedence == "list" || precedence == "bulk" {
                    return MailinglistType::SenderBased;
                }
            }
        }
        MailinglistType::None
    }

    pub(crate) fn is_mailinglist_message(&self) -> bool {
        match self.get_mailinglist_type() {
            MailinglistType::ListIdBased | MailinglistType::SenderBased => true,
            MailinglistType::None => false,
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
        self.get_header(HeaderDef::XMicrosoftOriginalMessageId)
            .or_else(|| self.get_header(HeaderDef::MessageId))
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

    fn maybe_remove_bad_parts(&mut self) {
        let good_parts = self.parts.iter().filter(|p| !p.dehtml_failed).count();
        if good_parts == 0 {
            // We have no good part but show at least one bad part in order to show anything at all
            self.parts.truncate(1);
        } else if good_parts < self.parts.len() {
            self.parts.retain(|p| !p.dehtml_failed);
        }

        // remove images that are descendants of multipart/related but the first one:
        // - for newsletters or so, that is often the logo
        // - for user-generated html-mails, that may be some drag'n'drop photo,
        //   so, the recipient sees at least the first image directly
        // - all other images can be accessed by "show full message"
        // - to ensure, there is such a button, we do removal only if
        //   `is_mime_modified` is set
        if !self.has_chat_version() && self.is_mime_modified {
            fn is_related_image(p: &&Part) -> bool {
                (p.typ == Viewtype::Image || p.typ == Viewtype::Gif) && p.is_related
            }
            let related_image_cnt = self.parts.iter().filter(is_related_image).count();
            if related_image_cnt > 1 {
                let mut is_first_image = true;
                self.parts.retain(|p| {
                    let retain = is_first_image || !is_related_image(&p);
                    if p.typ == Viewtype::Image || p.typ == Viewtype::Gif {
                        is_first_image = false;
                    }
                    retain
                });
            }
        }
    }

    /// Remove unwanted, additional text parts used for mailing list footer.
    /// Some mailinglist software add footers as separate mimeparts
    /// eg. when the user-edited-content is html.
    /// As these footers would appear as repeated, separate text-bubbles,
    /// we remove them.
    fn maybe_remove_inline_mailinglist_footer(&mut self) {
        if self.is_mailinglist_message() {
            let text_part_cnt = self
                .parts
                .iter()
                .filter(|p| p.typ == Viewtype::Text)
                .count();
            if text_part_cnt == 2 {
                if let Some(last_part) = self.parts.last() {
                    if last_part.typ == Viewtype::Text {
                        self.parts.pop();
                    }
                }
            }
        }
    }

    /// Some providers like GMX and Yahoo do not send standard NDNs (Non Delivery notifications).
    /// If you improve heuristics here you might also have to change prefetch_should_download() in imap/mod.rs.
    /// Also you should add a test in dc_receive_imf.rs (there already are lots of test_parse_ndn_* tests).
    #[allow(clippy::indexing_slicing)]
    async fn heuristically_parse_ndn(&mut self, context: &Context) {
        let maybe_ndn = if let Some(from) = self.get_header(HeaderDef::From_) {
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
                match message::handle_mdn(context, from_id, original_message_id, sent_timestamp)
                    .await
                {
                    Ok(Some((chat_id, msg_id))) => {
                        context.emit_event(EventType::MsgRead { chat_id, msg_id });
                    }
                    Ok(None) => {}
                    Err(err) => {
                        warn!(context, "failed to handle_mdn: {:#}", err);
                    }
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
            .get_header(HeaderDef::InReplyTo)
            .and_then(|msgid| parse_message_id(msgid).ok())
        {
            context
                .sql
                .query_get_value(
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
                    let p = Peerstate::from_gossip(header, message_time);
                    p.save_to_db(&context.sql, true).await?;
                    peerstate = Some(p);
                }
                if let Some(peerstate) = peerstate {
                    peerstate
                        .handle_fingerprint_change(context, message_time)
                        .await?;
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
pub(crate) fn parse_message_ids(ids: &str) -> Vec<String> {
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
    msgids
}

pub(crate) fn parse_message_id(ids: &str) -> Result<String> {
    if let Some(id) = parse_message_ids(ids).first() {
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
    dehtml_failed: bool,

    /// the part is a child or a descendant of multipart/related.
    /// typically, these are images that are referenced from text/html part
    /// and should not displayed inside chat.
    ///
    /// note that multipart/related may contain further multipart nestings
    /// and all of them needs to be marked with `is_related`.
    is_related: bool,
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
            // Enacapsulated messages, see <https://www.w3.org/Protocols/rfc1341/7_3_Message.html>
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
/// If filename is explicitly specified in Content-Disposition, it is
/// returned. If Content-Disposition is "attachment" but filename is
/// not specified, filename is guessed. If Content-Disposition cannot
/// be parsed, returns an error.
fn get_attachment_filename(
    context: &Context,
    mail: &mailparse::ParsedMail,
) -> Result<Option<String>> {
    let ct = mail.get_content_disposition();

    // try to get file name as "encoded-words" from
    // `Content-Disposition: ... filename=...`
    let mut desired_filename = ct.params.get("filename").map(|s| s.to_string());

    if desired_filename.is_none() {
        if let Some(name) = ct.params.get("filename*").map(|s| s.to_string()) {
            // be graceful and just use the original name.
            // some MUA, including Delta Chat up to core1.50,
            // use `filename*` mistakenly for simple encoded-words without following rfc2231
            warn!(context, "apostrophed encoding invalid: {}", name);
            desired_filename = Some(name);
        }
    }

    // if no filename is set, try `Content-Disposition: ... name=...`
    if desired_filename.is_none() {
        desired_filename = ct.params.get("name").map(|s| s.to_string());
    }

    // MS Outlook is known to specify filename in the "name" attribute of
    // Content-Type and omit Content-Disposition.
    if desired_filename.is_none() {
        desired_filename = mail.ctype.params.get("name").map(|s| s.to_string());
    }

    // If there is no filename, but part is an attachment, guess filename
    if desired_filename.is_none() && ct.disposition == DispositionType::Attachment {
        if let Some(subtype) = mail.ctype.mimetype.split('/').nth(1) {
            desired_filename = Some(format!("file.{}", subtype,));
        } else {
            bail!(
                "could not determine attachment filename: {:?}",
                ct.disposition
            );
        };
    }

    Ok(desired_filename)
}

/// Returned addresses are normalized and lowercased.
pub(crate) fn get_recipients(headers: &[MailHeader]) -> Vec<SingleInfo> {
    get_all_addresses_from_header(headers, |header_key| {
        header_key == "to" || header_key == "cc"
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
    use crate::{
        chatlist::Chatlist,
        config::Config,
        constants::Blocked,
        dc_receive_imf::dc_receive_imf,
        message::{Message, MessageState, MessengerMessage},
        test_utils::TestContext,
    };
    use mailparse::ParsedMail;

    impl AvatarAction {
        pub fn is_change(&self) -> bool {
            match self {
                AvatarAction::Delete => false,
                AvatarAction::Change(_) => true,
            }
        }
    }

    #[async_std::test]
    async fn test_mimeparser_fromheader() {
        let ctx = TestContext::new_alice().await;

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: g@c.de\n\nhi")
            .await
            .unwrap();
        let contact = mimemsg.from.first().unwrap();
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, None);

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From:   g@c.de  \n\nhi")
            .await
            .unwrap();
        let contact = mimemsg.from.first().unwrap();
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, None);

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: <g@c.de>\n\nhi")
            .await
            .unwrap();
        let contact = mimemsg.from.first().unwrap();
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, None);

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: Goetz C <g@c.de>\n\nhi")
            .await
            .unwrap();
        let contact = mimemsg.from.first().unwrap();
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Goetz C".to_string()));

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: \"Goetz C\" <g@c.de>\n\nhi")
            .await
            .unwrap();
        let contact = mimemsg.from.first().unwrap();
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Goetz C".to_string()));

        let mimemsg =
            MimeMessage::from_bytes(&ctx, b"From: =?utf-8?q?G=C3=B6tz?= C <g@c.de>\n\nhi")
                .await
                .unwrap();
        let contact = mimemsg.from.first().unwrap();
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Götz C".to_string()));

        // although RFC 2047 says, encoded-words shall not appear inside quoted-string,
        // this combination is used in the wild eg. by MailMate
        let mimemsg =
            MimeMessage::from_bytes(&ctx, b"From: \"=?utf-8?q?G=C3=B6tz?= C\" <g@c.de>\n\nhi")
                .await
                .unwrap();
        let contact = mimemsg.from.first().unwrap();
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Götz C".to_string()));
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

    fn load_mail_with_attachment<'a>(t: &'a TestContext, raw: &'a [u8]) -> ParsedMail<'a> {
        let mail = mailparse::parse_mail(raw).unwrap();
        assert!(get_attachment_filename(t, &mail).unwrap().is_none());
        assert!(get_attachment_filename(t, &mail.subparts[0])
            .unwrap()
            .is_none());
        mail
    }

    #[async_std::test]
    async fn test_get_attachment_filename() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_simple.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("test.html".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_encoded_words() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßnahmen Okt. 2020.html".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_encoded_words_binary() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words_binary.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some(" § 165 Abs".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_encoded_words_windows1251() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words_windows1251.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("file Что нового 2020.pdf".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_encoded_words_cont() {
        // test continued encoded-words and also test apostropes work that way
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words_cont.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßn'ah'men Okt. 2020.html".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_encoded_words_bad_delimiter() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words_bad_delimiter.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        // not decoded as a space is missing after encoded-words part
        assert_eq!(filename, Some("=?utf-8?q?foo?=.bar".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_apostrophed() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßnahmen Okt. 2021.html".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_apostrophed_cont() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_cont.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßnahmen März 2022.html".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_apostrophed_windows1251() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_windows1251.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("программирование.HTM".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_apostrophed_cp1252() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_cp1252.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Auftragsbestätigung.pdf".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_apostrophed_invalid() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_invalid.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("somedäüta.html.zip".to_string()))
    }

    #[async_std::test]
    async fn test_get_attachment_filename_combined() {
        // test that if `filename` and `filename*0` are given, the filename is not doubled
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_combined.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßnahmen Okt. 2020.html".to_string()))
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
        let of = mimeparser.get_header(HeaderDef::SecureJoinGroup).unwrap();
        assert_eq!(of, "no");

        // unknown headers do not bubble upwards
        let of = mimeparser.get_header(HeaderDef::_TestHeader).unwrap();
        assert_eq!(of, "Bar");

        // the following fields would bubble up
        // if the test would really use encryption for the protected part
        // however, as this is not the case, the outer things stay valid.
        // for Chat-Version, also the case-insensivity is tested.
        assert_eq!(mimeparser.get_subject(), Some("outer-subject".into()));

        let of = mimeparser.get_header(HeaderDef::ChatVersion).unwrap();
        assert_eq!(of, "0.0");
        assert_eq!(mimeparser.parts.len(), 1);

        // make sure, headers that are only allowed in the encrypted part
        // cannot be set from the outer part
        assert!(mimeparser
            .get_header(HeaderDef::SecureJoinFingerprint)
            .is_none());
    }

    #[async_std::test]
    async fn test_mimeparser_with_avatars() {
        let t = TestContext::new().await;

        let raw = include_bytes!("../test-data/message/mail_attach_txt.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.user_avatar, None);
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.unwrap().is_change());
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar_deleted.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert_eq!(mimeparser.user_avatar, Some(AvatarAction::Delete));
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..]).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.unwrap().is_change());
        assert!(mimeparser.group_avatar.unwrap().is_change());

        // if the Chat-User-Avatar header is missing, the avatar become a normal attachment
        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let raw = String::from_utf8_lossy(raw).to_string();
        let raw = raw.replace("Chat-User-Avatar:", "Xhat-Xser-Xvatar:");
        let mimeparser = MimeMessage::from_bytes(&t, raw.as_bytes()).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Image);
        assert_eq!(mimeparser.user_avatar, None);
        assert!(mimeparser.group_avatar.unwrap().is_change());
    }

    #[async_std::test]
    async fn test_mimeparser_with_videochat() {
        let t = TestContext::new().await;

        let raw = include_bytes!("../test-data/message/videochat_invitation.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..]).await.unwrap();
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
        assert_eq!(message.parts[0].msg, "Mail with inline attachment – Hello!");
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

        let message = MimeMessage::from_bytes(&t, &raw[..]).await.unwrap();

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::File);
        assert_eq!(message.parts[0].msg, "");

        // Make sure the file is there even though the html is wrong:
        let param = &message.parts[0].param;
        let blob: BlobObject = param
            .get_blob(Param::File, &t, false)
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
        assert_eq!(message.parts[0].msg, "example – Test");
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
        assert_eq!(message.parts[0].msg, "Test subject – Test");
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
        assert_eq!(message.parts[0].msg, "Delta Chat is great stuff! – Test");
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
        let test = parse_message_ids("  foo  bar <foobar>");
        assert_eq!(test.len(), 3);
        assert_eq!(test[0], "foo");
        assert_eq!(test[1], "bar");
        assert_eq!(test[2], "foobar");

        let test = parse_message_ids("  < foobar >");
        assert_eq!(test.len(), 1);
        assert_eq!(test[0], "foobar");

        let test = parse_message_ids("");
        assert!(test.is_empty());

        let test = parse_message_ids(" ");
        assert!(test.is_empty());

        let test = parse_message_ids("  < ");
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

    #[async_std::test]
    async fn test_quote_div() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/gmx-quote.eml");
        let mimeparser = MimeMessage::from_bytes(&t, raw).await.unwrap();
        assert_eq!(mimeparser.parts[0].msg, "YIPPEEEEEE\n\nMulti-line");
        assert_eq!(mimeparser.parts[0].param.get(Param::Quote).unwrap(), "Now?");
    }

    #[async_std::test]
    async fn test_allinkl_blockquote() {
        // all-inkl.com puts quotes into `<blockquote> </blockquote>`.
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/allinkl-quote.eml");
        let mimeparser = MimeMessage::from_bytes(&t, raw).await.unwrap();
        assert!(mimeparser.parts[0].msg.starts_with("It's 1.0."));
        assert_eq!(
            mimeparser.parts[0].param.get(Param::Quote).unwrap(),
            "What's the version?"
        );
    }

    #[async_std::test]
    async fn test_add_subj_to_multimedia_msg() {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("2")).await.unwrap();
        dc_receive_imf(
            &t.ctx,
            include_bytes!("../test-data/message/subj_with_multimedia_msg.eml"),
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        let msg_id = chats.get_msg_id(0).unwrap().unwrap();
        let msg = Message::load_from_db(&t.ctx, msg_id).await.unwrap();

        assert_eq!(
            msg.text.as_ref().unwrap(),
            "subj with important info – body text"
        );
        assert_eq!(msg.viewtype, Viewtype::Image);
        assert_eq!(msg.error(), None);
        assert_eq!(msg.is_dc_message, MessengerMessage::No);
        assert_eq!(msg.chat_blocked, Blocked::Request);
        assert_eq!(msg.state, MessageState::InFresh);
        assert_eq!(msg.get_filebytes(&t).await, 2115);
        assert!(msg.get_file(&t).is_some());
        assert_eq!(msg.get_filename().unwrap(), "avatar64x64.png");
        assert_eq!(msg.get_width(), 64);
        assert_eq!(msg.get_height(), 64);
        assert_eq!(msg.get_filemime().unwrap(), "image/png");
    }

    #[async_std::test]
    async fn test_mime_modified_plain() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_plain_unspecified.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw).await.unwrap();
        assert!(!mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "This message does not have Content-Type nor Subject."
        );
    }

    #[async_std::test]
    async fn test_mime_modified_alt_plain_html() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_alt_plain_html.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw).await.unwrap();
        assert!(mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – this is plain"
        );
    }

    #[async_std::test]
    async fn test_mime_modified_alt_plain() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_alt_plain.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw).await.unwrap();
        assert!(!mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – \
        mime-modified should not be set set as there is no html and no special stuff;\n\
        although not being a delta-message.\n\
        test some special html-characters as < > and & but also \" and ' :)"
        );
    }

    #[async_std::test]
    async fn test_mime_modified_alt_html() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_alt_html.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw).await.unwrap();
        assert!(mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – mime-modified *set*; simplify is always regarded as lossy."
        );
    }

    #[async_std::test]
    async fn test_mime_modified_html() {
        let t = TestContext::new().await;
        let raw = include_bytes!("../test-data/message/text_html.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw).await.unwrap();
        assert!(mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – mime-modified *set*; simplify is always regarded as lossy."
        );
    }

    #[async_std::test]
    async fn test_mime_modified_large_plain() {
        let t = TestContext::new().await;

        static REPEAT_TXT: &str = "this text with 42 chars is just repeated.\n";
        static REPEAT_CNT: usize = 2000; // results in a text of 84k, should be more than DC_DESIRED_TEXT_LEN
        let long_txt = format!("From: alice@c.de\n\n{}", REPEAT_TXT.repeat(REPEAT_CNT));

        let mimemsg = MimeMessage::from_bytes(&t, long_txt.as_ref())
            .await
            .unwrap();
        assert_eq!(long_txt.matches("just repeated").count(), REPEAT_CNT);
        assert!(long_txt.len() > DC_DESIRED_TEXT_LEN);
        assert!(mimemsg.is_mime_modified);
        assert!(
            mimemsg.parts[0].msg.matches("just repeated").count()
                <= DC_DESIRED_TEXT_LEN / REPEAT_TXT.len()
        );
        assert!(mimemsg.parts[0].msg.len() <= DC_DESIRED_TEXT_LEN + DC_ELLIPSIS.len());
    }

    #[async_std::test]
    async fn test_x_microsoft_original_message_id() {
        let t = TestContext::new().await;
        let message = MimeMessage::from_bytes(&t, b"Date: Wed, 17 Feb 2021 15:45:15 +0000\n\
                Chat-Version: 1.0\n\
                Message-ID: <DBAPR03MB1180CE51A1BFE265BD018D4790869@DBAPR03MB6691.eurprd03.prod.outlook.com>\n\
                To: Bob <bob@example.org>\n\
                From: Alice <alice@example.org>\n\
                Subject: Message from Alice\n\
                Content-Type: text/plain\n\
                X-Microsoft-Original-Message-ID: <Mr.6Dx7ITn4w38.n9j7epIcuQI@outlook.com>\n\
                MIME-Version: 1.0\n\
                \n\
                Does it work with outlook now?\n\
                ")
            .await
            .unwrap();
        assert_eq!(
            message.get_rfc724_mid(),
            Some("Mr.6Dx7ITn4w38.n9j7epIcuQI@outlook.com".to_string())
        );
    }

    #[async_std::test]
    async fn test_long_in_reply_to() -> Result<()> {
        let t = TestContext::new_alice().await;

        // A message with a long Message-ID.
        // Long message-IDs are generated by Mailjet.
        let raw = br###"Date: Thu, 28 Jan 2021 00:26:57 +0000
Chat-Version: 1.0\n\
Message-ID: <ABCDEFGH.1234567_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA@mailjet.com>
To: Bob <bob@example.org>
From: Alice <alice@example.org>
Subject: ...

Some quote.
"###;
        dc_receive_imf(&t, raw, "INBOX", 1, false).await?;

        // Delta Chat generates In-Reply-To with a starting tab when Message-ID is too long.
        let raw = br###"In-Reply-To:
	<ABCDEFGH.1234567_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA@mailjet.com>
Date: Thu, 28 Jan 2021 00:26:57 +0000
Chat-Version: 1.0\n\
Message-ID: <foobar@example.org>
To: Alice <alice@example.org>
From: Bob <bob@example.org>
Subject: ...

> Some quote.

Some reply
"###;

        dc_receive_imf(&t, raw, "INBOX", 2, false).await?;

        let msg = t.get_last_msg().await;
        assert_eq!(msg.get_text().unwrap(), "Some reply");
        let quoted_message = msg.quoted_message(&t).await?.unwrap();
        assert_eq!(quoted_message.get_text().unwrap(), "Some quote.");

        Ok(())
    }

    // Test that WantsMdn parameter is not set on outgoing messages.
    #[async_std::test]
    async fn test_outgoing_wants_mdn() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let raw = br###"Date: Thu, 28 Jan 2021 00:26:57 +0000
Chat-Version: 1.0\n\
Message-ID: <foobarbaz@example.org>
To: Bob <bob@example.org>
From: Alice <alice@example.com>
Subject: subject
Chat-Disposition-Notification-To: alice@example.com

Message.
"###;

        // Bob receives message.
        dc_receive_imf(&bob, raw, "INBOX", 1, false).await?;
        let msg = bob.get_last_msg().await;
        // Message is incoming.
        assert!(msg.param.get_bool(Param::WantsMdn).unwrap());

        // Alice receives copy-to-self.
        dc_receive_imf(&alice, raw, "INBOX", 1, false).await?;
        let msg = alice.get_last_msg().await;
        // Message is outgoing, don't send read receipt to self.
        assert!(msg.param.get_bool(Param::WantsMdn).is_none());

        Ok(())
    }
}

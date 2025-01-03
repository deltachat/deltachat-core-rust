//! # MIME message parsing module.

use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str;
use std::str::FromStr;

use anyhow::{bail, Context as _, Result};
use deltachat_contact_tools::{addr_cmp, addr_normalize, sanitize_bidi_characters};
use deltachat_derive::{FromSql, ToSql};
use format_flowed::unformat_flowed;
use lettre_email::mime::Mime;
use mailparse::{addrparse_header, DispositionType, MailHeader, MailHeaderMap, SingleInfo};
use rand::distributions::{Alphanumeric, DistString};

use crate::aheader::{Aheader, EncryptPreference};
use crate::authres::handle_authres;
use crate::blob::BlobObject;
use crate::chat::ChatId;
use crate::config::Config;
use crate::constants;
use crate::contact::ContactId;
use crate::context::Context;
use crate::decrypt::{
    get_autocrypt_peerstate, get_encrypted_mime, keyring_from_peerstate, try_decrypt,
    validate_detached_signature,
};
use crate::dehtml::dehtml;
use crate::events::EventType;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::key::{self, load_self_secret_keyring, DcKey, Fingerprint, SignedPublicKey};
use crate::message::{self, get_vcard_summary, set_msg_failed, Message, MsgId, Viewtype};
use crate::param::{Param, Params};
use crate::peerstate::Peerstate;
use crate::simplify::{simplify, SimplifiedText};
use crate::sync::SyncItems;
use crate::tools::{
    get_filemeta, parse_receive_headers, smeared_time, truncate_msg_text, validate_id,
};
use crate::{chatlist_events, location, stock_str, tools};

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
pub(crate) struct MimeMessage {
    /// Parsed MIME parts.
    pub parts: Vec<Part>,

    /// Message headers.
    headers: HashMap<String, String>,

    /// Addresses are normalized and lowercase
    pub recipients: Vec<SingleInfo>,

    /// `From:` address.
    pub from: SingleInfo,

    /// Whether the From address was repeated in the signed part
    /// (and we know that the signer intended to send from this address)
    pub from_is_signed: bool,
    /// Whether the message is incoming or outgoing (self-sent).
    pub incoming: bool,
    /// The List-Post address is only set for mailing lists. Users can send
    /// messages to this address to post them to the list.
    pub list_post: Option<String>,
    pub chat_disposition_notification_to: Option<SingleInfo>,
    pub autocrypt_header: Option<Aheader>,
    pub peerstate: Option<Peerstate>,
    pub decrypting_failed: bool,

    /// Set of valid signature fingerprints if a message is an
    /// Autocrypt encrypted and signed message.
    ///
    /// If a message is not encrypted or the signature is not valid,
    /// this set is empty.
    pub signatures: HashSet<Fingerprint>,
    /// The mail recipient addresses for which gossip headers were applied
    /// and their respective gossiped keys,
    /// regardless of whether they modified any peerstates.
    pub gossiped_keys: HashMap<String, SignedPublicKey>,

    /// True if the message is a forwarded message.
    pub is_forwarded: bool,
    pub is_system_message: SystemMessage,
    pub location_kml: Option<location::Kml>,
    pub message_kml: Option<location::Kml>,
    pub(crate) sync_items: Option<SyncItems>,
    pub(crate) webxdc_status_update: Option<String>,
    pub(crate) user_avatar: Option<AvatarAction>,
    pub(crate) group_avatar: Option<AvatarAction>,
    pub(crate) mdn_reports: Vec<Report>,
    pub(crate) delivery_report: Option<DeliveryReport>,

    /// Standard USENET signature, if any.
    ///
    /// `None` means no text part was received, empty string means a text part without a footer is
    /// received.
    pub(crate) footer: Option<String>,

    /// If set, this is a modified MIME message; clients should offer a way to view the original
    /// MIME message in this case.
    pub is_mime_modified: bool,

    /// Decrypted, raw MIME structure. Nonempty iff `is_mime_modified` and the message was actually
    /// encrypted.
    pub decoded_data: Vec<u8>,

    /// Hop info for debugging.
    pub(crate) hop_info: String,

    /// Whether the message is auto-generated.
    ///
    /// If chat message (with `Chat-Version` header) is auto-generated,
    /// the contact sending this should be marked as bot.
    ///
    /// If non-chat message is auto-generated,
    /// it could be a holiday notice auto-reply,
    /// in which case the message should be marked as bot-generated,
    /// but the contact should not be.
    pub(crate) is_bot: Option<bool>,

    /// When the message was received, in secs since epoch.
    pub(crate) timestamp_rcvd: i64,
    /// Sender timestamp in secs since epoch. Allowed to be in the future due to unsynchronized
    /// clocks, but not too much.
    pub(crate) timestamp_sent: i64,
}

#[derive(Debug, PartialEq)]
pub(crate) enum AvatarAction {
    Delete,
    Change(String),
}

/// System message type.
#[derive(
    Debug, Default, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql,
)]
#[repr(u32)]
pub enum SystemMessage {
    /// Unknown type of system message.
    #[default]
    Unknown = 0,

    /// Group name changed.
    GroupNameChanged = 2,

    /// Group avatar changed.
    GroupImageChanged = 3,

    /// Member was added to the group.
    MemberAddedToGroup = 4,

    /// Member was removed from the group.
    MemberRemovedFromGroup = 5,

    /// Autocrypt Setup Message.
    AutocryptSetupMessage = 6,

    /// Secure-join message.
    SecurejoinMessage = 7,

    /// Location streaming is enabled.
    LocationStreamingEnabled = 8,

    /// Location-only message.
    LocationOnly = 9,

    /// Chat ephemeral message timer is changed.
    EphemeralTimerChanged = 10,

    /// "Messages are guaranteed to be end-to-end encrypted from now on."
    ChatProtectionEnabled = 11,

    /// "%1$s sent a message from another device."
    ChatProtectionDisabled = 12,

    /// Message can't be sent because of `Invalid unencrypted mail to <>`
    /// which is sent by chatmail servers.
    InvalidUnencryptedMail = 13,

    /// 1:1 chats info message telling that SecureJoin has started and the user should wait for it
    /// to complete.
    SecurejoinWait = 14,

    /// 1:1 chats info message telling that SecureJoin is still running, but the user may already
    /// send messages.
    SecurejoinWaitTimeout = 15,

    /// Self-sent-message that contains only json used for multi-device-sync;
    /// if possible, we attach that to other messages as for locations.
    MultiDeviceSync = 20,

    /// Sync message that contains a json payload
    /// sent to the other webxdc instances
    /// These messages are not shown in the chat.
    WebxdcStatusUpdate = 30,

    /// Webxdc info added with `info` set in `send_webxdc_status_update()`.
    WebxdcInfoMessage = 32,

    /// This message contains a users iroh node address.
    IrohNodeAddr = 40,
}

const MIME_AC_SETUP_FILE: &str = "application/autocrypt-setup";

impl MimeMessage {
    /// Parse a mime message.
    ///
    /// If `partial` is set, it contains the full message size in bytes
    /// and `body` contains the header only.
    pub(crate) async fn from_bytes(
        context: &Context,
        body: &[u8],
        partial: Option<u32>,
    ) -> Result<Self> {
        let mail = mailparse::parse_mail(body)?;

        let timestamp_rcvd = smeared_time(context);
        let mut timestamp_sent =
            Self::get_timestamp_sent(&mail.headers, timestamp_rcvd, timestamp_rcvd);
        let mut hop_info = parse_receive_headers(&mail.get_headers());

        let mut headers = Default::default();
        let mut recipients = Default::default();
        let mut from = Default::default();
        let mut list_post = Default::default();
        let mut chat_disposition_notification_to = None;

        // Parse IMF headers.
        MimeMessage::merge_headers(
            context,
            &mut headers,
            &mut recipients,
            &mut from,
            &mut list_post,
            &mut chat_disposition_notification_to,
            &mail.headers,
        );

        // Parse hidden headers.
        let mimetype = mail.ctype.mimetype.parse::<Mime>()?;
        let (part, mimetype) =
            if mimetype.type_() == mime::MULTIPART && mimetype.subtype().as_str() == "signed" {
                if let Some(part) = mail.subparts.first() {
                    // We don't remove "subject" from `headers` because currently just signed
                    // messages are shown as unencrypted anyway.

                    timestamp_sent =
                        Self::get_timestamp_sent(&mail.headers, timestamp_sent, timestamp_rcvd);
                    MimeMessage::merge_headers(
                        context,
                        &mut headers,
                        &mut recipients,
                        &mut from,
                        &mut list_post,
                        &mut chat_disposition_notification_to,
                        &part.headers,
                    );
                    (part, part.ctype.mimetype.parse::<Mime>()?)
                } else {
                    // If it's a partially fetched message, there are no subparts.
                    (&mail, mimetype)
                }
            } else {
                // Currently we do not sign unencrypted messages by default.
                (&mail, mimetype)
            };
        if mimetype.type_() == mime::MULTIPART && mimetype.subtype().as_str() == "mixed" {
            if let Some(part) = part.subparts.first() {
                for field in &part.headers {
                    let key = field.get_key().to_lowercase();

                    // For now only avatar headers can be hidden.
                    if !headers.contains_key(&key)
                        && (key == "chat-user-avatar" || key == "chat-group-avatar")
                    {
                        headers.insert(key.to_string(), field.get_value());
                    }
                }
            }
        }

        // Overwrite Message-ID with X-Microsoft-Original-Message-ID.
        // However if we later find Message-ID in the protected part,
        // it will overwrite both.
        if let Some(microsoft_message_id) =
            headers.remove(HeaderDef::XMicrosoftOriginalMessageId.get_headername())
        {
            headers.insert(
                HeaderDef::MessageId.get_headername().to_string(),
                microsoft_message_id,
            );
        }

        // Remove headers that are allowed _only_ in the encrypted+signed part. It's ok to leave
        // them in signed-only emails, but has no value currently.
        Self::remove_secured_headers(&mut headers);

        let mut from = from.context("No from in message")?;
        let private_keyring = load_self_secret_keyring(context).await?;

        let allow_aeap = get_encrypted_mime(&mail).is_some();

        let dkim_results = handle_authres(context, &mail, &from.addr).await?;

        let mut gossiped_keys = Default::default();
        let mut from_is_signed = false;
        hop_info += "\n\n";
        hop_info += &dkim_results.to_string();

        let incoming = !context.is_self_addr(&from.addr).await?;

        let mut aheader_value: Option<String> = mail.headers.get_header_value(HeaderDef::Autocrypt);

        let mail_raw; // Memory location for a possible decrypted message.
        let decrypted_msg; // Decrypted signed OpenPGP message.

        let (mail, encrypted) =
            match tokio::task::block_in_place(|| try_decrypt(&mail, &private_keyring)) {
                Ok(Some(msg)) => {
                    mail_raw = msg.get_content()?.unwrap_or_default();

                    let decrypted_mail = mailparse::parse_mail(&mail_raw)?;
                    if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
                        info!(
                            context,
                            "decrypted message mime-body:\n{}",
                            String::from_utf8_lossy(&mail_raw),
                        );
                    }

                    decrypted_msg = Some(msg);
                    if let Some(protected_aheader_value) = decrypted_mail
                        .headers
                        .get_header_value(HeaderDef::Autocrypt)
                    {
                        aheader_value = Some(protected_aheader_value);
                    }

                    (Ok(decrypted_mail), true)
                }
                Ok(None) => {
                    mail_raw = Vec::new();
                    decrypted_msg = None;
                    (Ok(mail), false)
                }
                Err(err) => {
                    mail_raw = Vec::new();
                    decrypted_msg = None;
                    warn!(context, "decryption failed: {:#}", err);
                    (Err(err), false)
                }
            };

        let autocrypt_header = if !incoming {
            None
        } else if let Some(aheader_value) = aheader_value {
            match Aheader::from_str(&aheader_value) {
                Ok(header) if addr_cmp(&header.addr, &from.addr) => Some(header),
                Ok(header) => {
                    warn!(
                        context,
                        "Autocrypt header address {:?} is not {:?}.", header.addr, from.addr
                    );
                    None
                }
                Err(err) => {
                    warn!(context, "Failed to parse Autocrypt header: {:#}.", err);
                    None
                }
            }
        } else {
            None
        };

        // The peerstate that will be used to validate the signatures.
        let mut peerstate = get_autocrypt_peerstate(
            context,
            &from.addr,
            autocrypt_header.as_ref(),
            timestamp_sent,
            allow_aeap,
        )
        .await?;

        let public_keyring = match peerstate.is_none() && !incoming {
            true => key::load_self_public_keyring(context).await?,
            false => keyring_from_peerstate(peerstate.as_ref()),
        };

        let mut signatures = if let Some(ref decrypted_msg) = decrypted_msg {
            crate::pgp::valid_signature_fingerprints(decrypted_msg, &public_keyring)?
        } else {
            HashSet::new()
        };

        let mail = mail.as_ref().map(|mail| {
            let (content, signatures_detached) = validate_detached_signature(mail, &public_keyring)
                .unwrap_or((mail, Default::default()));
            signatures.extend(signatures_detached);
            content
        });
        if let (Ok(mail), true) = (mail, encrypted) {
            timestamp_sent =
                Self::get_timestamp_sent(&mail.headers, timestamp_sent, timestamp_rcvd);
            if !signatures.is_empty() {
                // Handle any gossip headers if the mail was encrypted. See section
                // "3.6 Key Gossip" of <https://autocrypt.org/autocrypt-spec-1.1.0.pdf>
                // but only if the mail was correctly signed. Probably it's ok to not require
                // encryption here, but let's follow the standard.
                let gossip_headers = mail.headers.get_all_values("Autocrypt-Gossip");
                gossiped_keys = update_gossip_peerstates(
                    context,
                    timestamp_sent,
                    &from.addr,
                    &recipients,
                    gossip_headers,
                )
                .await?;
                // Remove unsigned opportunistically protected headers from messages considered
                // Autocrypt-encrypted / displayed with padlock.
                // For "Subject" see <https://github.com/deltachat/deltachat-core-rust/issues/1790>.
                for h in [
                    HeaderDef::Subject,
                    HeaderDef::ChatGroupId,
                    HeaderDef::ChatGroupName,
                    HeaderDef::ChatGroupNameChanged,
                    HeaderDef::ChatGroupAvatar,
                    HeaderDef::ChatGroupMemberRemoved,
                    HeaderDef::ChatGroupMemberAdded,
                ] {
                    headers.remove(h.get_headername());
                }
            }

            // let known protected headers from the decrypted
            // part override the unencrypted top-level

            // Signature was checked for original From, so we
            // do not allow overriding it.
            let mut inner_from = None;

            MimeMessage::merge_headers(
                context,
                &mut headers,
                &mut recipients,
                &mut inner_from,
                &mut list_post,
                &mut chat_disposition_notification_to,
                &mail.headers,
            );

            if let Some(inner_from) = inner_from {
                if !addr_cmp(&inner_from.addr, &from.addr) {
                    // There is a From: header in the encrypted
                    // part, but it doesn't match the outer one.
                    // This _might_ be because the sender's mail server
                    // replaced the sending address, e.g. in a mailing list.
                    // Or it's because someone is doing some replay attack.
                    // Resending encrypted messages via mailing lists
                    // without reencrypting is not useful anyway,
                    // so we return an error below.
                    warn!(
                        context,
                        "From header in encrypted part doesn't match the outer one",
                    );

                    // Return an error from the parser.
                    // This will result in creating a tombstone
                    // and no further message processing
                    // as if the MIME structure is broken.
                    bail!("From header is forged");
                }
                from = inner_from;
                from_is_signed = !signatures.is_empty();
            }
        }
        if signatures.is_empty() {
            Self::remove_secured_headers(&mut headers);

            // If it is not a read receipt, degrade encryption.
            if let (Some(peerstate), Ok(mail)) = (&mut peerstate, mail) {
                if timestamp_sent > peerstate.last_seen_autocrypt
                    && mail.ctype.mimetype != "multipart/report"
                {
                    peerstate.degrade_encryption(timestamp_sent);
                }
            }
        }
        if !encrypted {
            signatures.clear();
        }
        if let Some(peerstate) = &mut peerstate {
            if peerstate.prefer_encrypt != EncryptPreference::Mutual && !signatures.is_empty() {
                peerstate.prefer_encrypt = EncryptPreference::Mutual;
                peerstate.save_to_db(&context.sql).await?;
            }
        }

        let mut parser = MimeMessage {
            parts: Vec::new(),
            headers,
            recipients,
            list_post,
            from,
            from_is_signed,
            incoming,
            chat_disposition_notification_to,
            autocrypt_header,
            peerstate,
            decrypting_failed: mail.is_err(),

            // only non-empty if it was a valid autocrypt message
            signatures,
            gossiped_keys,
            is_forwarded: false,
            mdn_reports: Vec::new(),
            is_system_message: SystemMessage::Unknown,
            location_kml: None,
            message_kml: None,
            sync_items: None,
            webxdc_status_update: None,
            user_avatar: None,
            group_avatar: None,
            delivery_report: None,
            footer: None,
            is_mime_modified: false,
            decoded_data: Vec::new(),
            hop_info,
            is_bot: None,
            timestamp_rcvd,
            timestamp_sent,
        };

        match partial {
            Some(org_bytes) => {
                parser
                    .create_stub_from_partial_download(context, org_bytes)
                    .await?;
            }
            None => match mail {
                Ok(mail) => {
                    parser.parse_mime_recursive(context, mail, false).await?;
                }
                Err(err) => {
                    let msg_body = stock_str::cant_decrypt_msg_body(context).await;
                    let txt = format!("[{msg_body}]");

                    let part = Part {
                        typ: Viewtype::Text,
                        msg_raw: Some(txt.clone()),
                        msg: txt,
                        // Don't change the error prefix for now,
                        // receive_imf.rs:lookup_chat_by_reply() checks it.
                        error: Some(format!("Decrypting failed: {err:#}")),
                        ..Default::default()
                    };
                    parser.parts.push(part);
                }
            },
        };

        let is_location_only = parser.location_kml.is_some() && parser.parts.is_empty();
        if parser.mdn_reports.is_empty()
            && !is_location_only
            && parser.sync_items.is_none()
            && parser.webxdc_status_update.is_none()
        {
            let is_bot =
                parser.headers.get("auto-submitted") == Some(&"auto-generated".to_string());
            parser.is_bot = Some(is_bot);
        }
        parser.maybe_remove_bad_parts();
        parser.maybe_remove_inline_mailinglist_footer();
        parser.heuristically_parse_ndn(context).await;
        parser.parse_headers(context).await?;

        if parser.is_mime_modified {
            parser.decoded_data = mail_raw;
        }

        Ok(parser)
    }

    fn get_timestamp_sent(
        hdrs: &[mailparse::MailHeader<'_>],
        default: i64,
        timestamp_rcvd: i64,
    ) -> i64 {
        hdrs.get_header_value(HeaderDef::Date)
            .and_then(|v| mailparse::dateparse(&v).ok())
            .map_or(default, |value| {
                min(value, timestamp_rcvd + constants::TIMESTAMP_SENT_TOLERANCE)
            })
    }

    /// Parses system messages.
    fn parse_system_message_headers(&mut self, context: &Context) {
        if self.get_header(HeaderDef::AutocryptSetupMessage).is_some() && !self.incoming {
            self.parts.retain(|part| {
                part.mimetype.is_none()
                    || part.mimetype.as_ref().unwrap().as_ref() == MIME_AC_SETUP_FILE
            });

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
            } else if value == "group-avatar-changed" {
                self.is_system_message = SystemMessage::GroupImageChanged;
            }
        } else if self.get_header(HeaderDef::ChatGroupMemberRemoved).is_some() {
            self.is_system_message = SystemMessage::MemberRemovedFromGroup;
        } else if self.get_header(HeaderDef::ChatGroupMemberAdded).is_some() {
            self.is_system_message = SystemMessage::MemberAddedToGroup;
        } else if self.get_header(HeaderDef::ChatGroupNameChanged).is_some() {
            self.is_system_message = SystemMessage::GroupNameChanged;
        }
    }

    /// Parses avatar action headers.
    async fn parse_avatar_headers(&mut self, context: &Context) {
        if let Some(header_value) = self.get_header(HeaderDef::ChatGroupAvatar) {
            self.group_avatar = self
                .avatar_action_from_header(context, header_value.to_string())
                .await;
        }

        if let Some(header_value) = self.get_header(HeaderDef::ChatUserAvatar) {
            self.user_avatar = self
                .avatar_action_from_header(context, header_value.to_string())
                .await;
        }
    }

    fn parse_videochat_headers(&mut self) {
        if let Some(value) = self.get_header(HeaderDef::ChatContent) {
            if value == "videochat-invitation" {
                let instance = self
                    .get_header(HeaderDef::ChatWebrtcRoom)
                    .map(|s| s.to_string());
                if let Some(part) = self.parts.first_mut() {
                    part.typ = Viewtype::VideochatInvitation;
                    part.param
                        .set(Param::WebrtcRoom, instance.unwrap_or_default());
                }
            }
        }
    }

    /// Squashes mutitpart chat messages with attachment into single-part messages.
    ///
    /// Delta Chat sends attachments, such as images, in two-part messages, with the first message
    /// containing a description. If such a message is detected, text from the first part can be
    /// moved to the second part, and the first part dropped.
    fn squash_attachment_parts(&mut self) {
        if self.parts.len() == 2
            && self.parts.first().map(|textpart| textpart.typ) == Some(Viewtype::Text)
            && self
                .parts
                .get(1)
                .is_some_and(|filepart| match filepart.typ {
                    Viewtype::Image
                    | Viewtype::Gif
                    | Viewtype::Sticker
                    | Viewtype::Audio
                    | Viewtype::Voice
                    | Viewtype::Video
                    | Viewtype::Vcard
                    | Viewtype::File
                    | Viewtype::Webxdc => true,
                    Viewtype::Unknown | Viewtype::Text | Viewtype::VideochatInvitation => false,
                })
        {
            let mut parts = std::mem::take(&mut self.parts);
            let Some(mut filepart) = parts.pop() else {
                // Should never happen.
                return;
            };
            let Some(textpart) = parts.pop() else {
                // Should never happen.
                return;
            };

            filepart.msg.clone_from(&textpart.msg);
            if let Some(quote) = textpart.param.get(Param::Quote) {
                filepart.param.set(Param::Quote, quote);
            }

            self.parts = vec![filepart];
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
        if self.delivery_report.is_none() {
            self.squash_attachment_parts();
        }

        if !context.get_config_bool(Config::Bot).await? {
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
                if self.is_mailinglist_message() && !self.has_chat_version() {
                    prepend_subject = true;
                }

                if prepend_subject && !subject.is_empty() {
                    let part_with_text = self
                        .parts
                        .iter_mut()
                        .find(|part| !part.msg.is_empty() && !part.is_reaction);
                    if let Some(part) = part_with_text {
                        part.msg = format!("{} – {}", subject, part.msg);
                    }
                }
            }
        }

        if self.is_forwarded {
            for part in &mut self.parts {
                part.param.set_int(Param::Forwarded, 1);
            }
        }

        self.parse_attachments();

        // See if an MDN is requested from the other side
        if !self.decrypting_failed && !self.parts.is_empty() {
            if let Some(ref dn_to) = self.chat_disposition_notification_to {
                // Check that the message is not outgoing.
                let from = &self.from.addr;
                if !context.is_self_addr(from).await? {
                    if from.to_lowercase() == dn_to.addr.to_lowercase() {
                        if let Some(part) = self.parts.last_mut() {
                            part.param.set_int(Param::WantsMdn, 1);
                        }
                    } else {
                        warn!(
                            context,
                            "{} requested a read receipt to {}, ignoring", from, dn_to.addr
                        );
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
                if !self.has_chat_version() && self.webxdc_status_update.is_none() {
                    part.msg = subject.to_string();
                }
            }

            self.do_add_single_part(part);
        }

        if self.is_bot == Some(true) {
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
        } else if let Some(base64) = header_value
            .split_ascii_whitespace()
            .collect::<String>()
            .strip_prefix("base64:")
        {
            // Add random suffix to the filename
            // to prevent the UI from accidentally using
            // cached "avatar.jpg".
            let suffix = Alphanumeric
                .sample_string(&mut rand::thread_rng(), 7)
                .to_lowercase();

            match BlobObject::store_from_base64(context, base64, &format!("avatar-{suffix}")).await
            {
                Ok(path) => Some(AvatarAction::Change(path)),
                Err(err) => {
                    warn!(
                        context,
                        "Could not decode and save avatar to blob file: {:#}", err,
                    );
                    None
                }
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

    /// Returns whether the email contains a `chat-version` header.
    /// This indicates that the email is a DC-email.
    pub(crate) fn has_chat_version(&self) -> bool {
        self.headers.contains_key("chat-version")
    }

    pub(crate) fn get_subject(&self) -> Option<String> {
        self.get_header(HeaderDef::Subject)
            .map(|s| s.trim_start())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    pub fn get_header(&self, headerdef: HeaderDef) -> Option<&str> {
        self.headers
            .get(headerdef.get_headername())
            .map(|s| s.as_str())
    }

    /// Returns `Chat-Group-ID` header value if it is a valid group ID.
    pub fn get_chat_group_id(&self) -> Option<&str> {
        self.get_header(HeaderDef::ChatGroupId)
            .filter(|s| validate_id(s))
    }

    async fn parse_mime_recursive<'a>(
        &'a mut self,
        context: &'a Context,
        mail: &'a mailparse::ParsedMail<'a>,
        is_related: bool,
    ) -> Result<bool> {
        enum MimeS {
            Multiple,
            Single,
            Message,
        }

        let mimetype = mail.ctype.mimetype.to_lowercase();

        let m = if mimetype.starts_with("multipart") {
            if mail.ctype.params.contains_key("boundary") {
                MimeS::Multiple
            } else {
                MimeS::Single
            }
        } else if mimetype.starts_with("message") {
            if mimetype == "message/rfc822" && !is_attachment_disposition(mail) {
                MimeS::Message
            } else {
                MimeS::Single
            }
        } else {
            MimeS::Single
        };

        let is_related = is_related || mimetype == "multipart/related";
        match m {
            MimeS::Multiple => Box::pin(self.handle_multiple(context, mail, is_related)).await,
            MimeS::Message => {
                let raw = mail.get_body_raw()?;
                if raw.is_empty() {
                    return Ok(false);
                }
                let mail = mailparse::parse_mail(&raw).context("failed to parse mail")?;

                Box::pin(self.parse_mime_recursive(context, &mail, is_related)).await
            }
            MimeS::Single => {
                self.add_single_part_if_known(context, mail, is_related)
                    .await
            }
        }
    }

    async fn handle_multiple(
        &mut self,
        context: &Context,
        mail: &mailparse::ParsedMail<'_>,
        is_related: bool,
    ) -> Result<bool> {
        let mut any_part_added = false;
        let mimetype = get_mime_type(mail, &get_attachment_filename(context, mail)?)?.0;
        match (mimetype.type_(), mimetype.subtype().as_str()) {
            /* Most times, multipart/alternative contains true alternatives
            as text/plain and text/html.  If we find a multipart/mixed
            inside multipart/alternative, we use this (happens eg in
            apple mail: "plaintext" as an alternative to "html+PDF attachment") */
            (mime::MULTIPART, "alternative") => {
                for cur_data in &mail.subparts {
                    let mime_type =
                        get_mime_type(cur_data, &get_attachment_filename(context, cur_data)?)?.0;
                    if mime_type == "multipart/mixed" || mime_type == "multipart/related" {
                        any_part_added = self
                            .parse_mime_recursive(context, cur_data, is_related)
                            .await?;
                        break;
                    }
                }
                if !any_part_added {
                    /* search for text/plain and add this */
                    for cur_data in &mail.subparts {
                        if get_mime_type(cur_data, &get_attachment_filename(context, cur_data)?)?
                            .0
                            .type_()
                            == mime::TEXT
                        {
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
            (mime::MULTIPART, "signed") => {
                /* RFC 1847: "The multipart/signed content type
                contains exactly two body parts.  The first body
                part is the body part over which the digital signature was created [...]
                The second body part contains the control information necessary to
                verify the digital signature." We simply take the first body part and
                skip the rest.  (see
                <https://k9mail.app/2016/11/24/OpenPGP-Considerations-Part-I.html>
                for background information why we use encrypted+signed) */
                if let Some(first) = mail.subparts.first() {
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
                                self.delivery_report = Some(report);
                            }

                            // Add all parts (we need another part, preferably text/plain, to show as an error message)
                            for cur_data in &mail.subparts {
                                if self
                                    .parse_mime_recursive(context, cur_data, is_related)
                                    .await?
                                {
                                    any_part_added = true;
                                }
                            }
                        }
                        Some("multi-device-sync") => {
                            if let Some(second) = mail.subparts.get(1) {
                                self.add_single_part_if_known(context, second, is_related)
                                    .await?;
                            }
                        }
                        Some("status-update") => {
                            if let Some(second) = mail.subparts.get(1) {
                                self.add_single_part_if_known(context, second, is_related)
                                    .await?;
                            }
                        }
                        Some(_) => {
                            for cur_data in &mail.subparts {
                                if self
                                    .parse_mime_recursive(context, cur_data, is_related)
                                    .await?
                                {
                                    any_part_added = true;
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // Add all parts (in fact, AddSinglePartIfKnown() later check if
                // the parts are really supported)
                for cur_data in &mail.subparts {
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

    /// Returns true if any part was added, false otherwise.
    async fn add_single_part_if_known(
        &mut self,
        context: &Context,
        mail: &mailparse::ParsedMail<'_>,
        is_related: bool,
    ) -> Result<bool> {
        // return true if a part was added
        let filename = get_attachment_filename(context, mail)?;
        let (mime_type, msg_type) = get_mime_type(mail, &filename)?;
        let raw_mime = mail.ctype.mimetype.to_lowercase();

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
                .await?;
            }
            None => {
                match mime_type.type_() {
                    mime::IMAGE | mime::AUDIO | mime::VIDEO | mime::APPLICATION => {
                        warn!(context, "Missing attachment");
                        return Ok(false);
                    }
                    mime::TEXT
                        if mail.get_content_disposition().disposition
                            == DispositionType::Extension("reaction".to_string()) =>
                    {
                        // Reaction.
                        let decoded_data = match mail.get_body() {
                            Ok(decoded_data) => decoded_data,
                            Err(err) => {
                                warn!(context, "Invalid body parsed {:#}", err);
                                // Note that it's not always an error - might be no data
                                return Ok(false);
                            }
                        };

                        let part = Part {
                            typ: Viewtype::Text,
                            mimetype: Some(mime_type),
                            msg: decoded_data,
                            is_reaction: true,
                            ..Default::default()
                        };
                        self.do_add_single_part(part);
                        return Ok(true);
                    }
                    mime::TEXT | mime::HTML => {
                        let decoded_data = match mail.get_body() {
                            Ok(decoded_data) => decoded_data,
                            Err(err) => {
                                warn!(context, "Invalid body parsed {:#}", err);
                                // Note that it's not always an error - might be no data
                                return Ok(false);
                            }
                        };

                        let is_plaintext = mime_type == mime::TEXT_PLAIN;
                        let mut dehtml_failed = false;

                        let SimplifiedText {
                            text: simplified_txt,
                            is_forwarded,
                            is_cut,
                            top_quote,
                            footer,
                        } = if decoded_data.is_empty() {
                            Default::default()
                        } else {
                            let is_html = mime_type == mime::TEXT_HTML;
                            if is_html {
                                self.is_mime_modified = true;
                                if let Some(text) = dehtml(&decoded_data) {
                                    text
                                } else {
                                    dehtml_failed = true;
                                    SimplifiedText {
                                        text: decoded_data.clone(),
                                        ..Default::default()
                                    }
                                }
                            } else {
                                simplify(decoded_data.clone(), self.has_chat_version())
                            }
                        };

                        self.is_mime_modified = self.is_mime_modified
                            || ((is_forwarded || is_cut || top_quote.is_some())
                                && !self.has_chat_version());

                        let is_format_flowed = if let Some(format) = mail.ctype.params.get("format")
                        {
                            format.as_str().eq_ignore_ascii_case("flowed")
                        } else {
                            false
                        };

                        let (simplified_txt, simplified_quote) = if mime_type.type_() == mime::TEXT
                            && mime_type.subtype() == mime::PLAIN
                            && is_format_flowed
                        {
                            let delsp = if let Some(delsp) = mail.ctype.params.get("delsp") {
                                delsp.as_str().eq_ignore_ascii_case("yes")
                            } else {
                                false
                            };
                            let unflowed_text = unformat_flowed(&simplified_txt, delsp);
                            let unflowed_quote = top_quote.map(|q| unformat_flowed(&q, delsp));
                            (unflowed_text, unflowed_quote)
                        } else {
                            (simplified_txt, top_quote)
                        };

                        let (simplified_txt, was_truncated) =
                            truncate_msg_text(context, simplified_txt).await?;
                        if was_truncated {
                            self.is_mime_modified = was_truncated;
                        }

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

                        if self.footer.is_none() && is_plaintext {
                            self.footer = Some(footer.unwrap_or_default());
                        }
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
    ) -> Result<()> {
        if decoded_data.is_empty() {
            return Ok(());
        }
        if let Some(peerstate) = &mut self.peerstate {
            if peerstate.prefer_encrypt != EncryptPreference::Mutual
                && mime_type.type_() == mime::APPLICATION
                && mime_type.subtype().as_str() == "pgp-keys"
                && Self::try_set_peer_key_from_file_part(context, peerstate, decoded_data).await?
            {
                return Ok(());
            }
        }
        let mut part = Part::default();
        let msg_type = if context
            .is_webxdc_file(filename, decoded_data)
            .await
            .unwrap_or(false)
        {
            Viewtype::Webxdc
        } else if filename.ends_with(".kml") {
            // XXX what if somebody sends eg an "location-highlights.kml"
            // attachment unrelated to location streaming?
            if filename.starts_with("location") || filename.starts_with("message") {
                let parsed = location::Kml::parse(decoded_data)
                    .map_err(|err| {
                        warn!(context, "failed to parse kml part: {:#}", err);
                    })
                    .ok();
                if filename.starts_with("location") {
                    self.location_kml = parsed;
                } else {
                    self.message_kml = parsed;
                }
                return Ok(());
            }
            msg_type
        } else if filename == "multi-device-sync.json" {
            if !context.get_config_bool(Config::SyncMsgs).await? {
                return Ok(());
            }
            let serialized = String::from_utf8_lossy(decoded_data)
                .parse()
                .unwrap_or_default();
            self.sync_items = context
                .parse_sync_items(serialized)
                .map_err(|err| {
                    warn!(context, "failed to parse sync data: {:#}", err);
                })
                .ok();
            return Ok(());
        } else if filename == "status-update.json" {
            let serialized = String::from_utf8_lossy(decoded_data)
                .parse()
                .unwrap_or_default();
            self.webxdc_status_update = Some(serialized);
            return Ok(());
        } else if msg_type == Viewtype::Vcard {
            if let Some(summary) = get_vcard_summary(decoded_data) {
                part.param.set(Param::Summary1, summary);
                msg_type
            } else {
                Viewtype::File
            }
        } else {
            msg_type
        };

        /* we have a regular file attachment,
        write decoded data to new blob object */

        let blob = match BlobObject::create(context, filename, decoded_data).await {
            Ok(blob) => blob,
            Err(err) => {
                error!(
                    context,
                    "Could not add blob for mime part {}, error {:#}", filename, err
                );
                return Ok(());
            }
        };
        info!(context, "added blobfile: {:?}", blob.as_name());

        if mime_type.type_() == mime::IMAGE {
            if let Ok((width, height)) = get_filemeta(decoded_data) {
                part.param.set_int(Param::Width, width as i32);
                part.param.set_int(Param::Height, height as i32);
            }
        }

        part.typ = msg_type;
        part.org_filename = Some(filename.to_string());
        part.mimetype = Some(mime_type);
        part.bytes = decoded_data.len();
        part.param.set(Param::File, blob.as_name());
        part.param.set(Param::Filename, filename);
        part.param.set(Param::MimeType, raw_mime);
        part.is_related = is_related;

        self.do_add_single_part(part);
        Ok(())
    }

    /// Returns whether a key from the attachment was set as peer's pubkey.
    async fn try_set_peer_key_from_file_part(
        context: &Context,
        peerstate: &mut Peerstate,
        decoded_data: &[u8],
    ) -> Result<bool> {
        let key = match str::from_utf8(decoded_data) {
            Err(err) => {
                warn!(context, "PGP key attachment is not a UTF-8 file: {}", err);
                return Ok(false);
            }
            Ok(key) => key,
        };
        let key = match SignedPublicKey::from_asc(key) {
            Err(err) => {
                warn!(
                    context,
                    "PGP key attachment is not an ASCII-armored file: {:#}", err
                );
                return Ok(false);
            }
            Ok((key, _)) => key,
        };
        if let Err(err) = key.verify() {
            warn!(context, "attached PGP key verification failed: {}", err);
            return Ok(false);
        }
        if !key.details.users.iter().any(|user| {
            user.id
                .id()
                .ends_with((String::from("<") + &peerstate.addr + ">").as_bytes())
        }) {
            return Ok(false);
        }
        if let Some(curr_key) = &peerstate.public_key {
            if key != *curr_key && peerstate.prefer_encrypt != EncryptPreference::Reset {
                // We don't want to break the existing Autocrypt setup. Yes, it's unlikely that a
                // user have an Autocrypt-capable MUA and also attaches a key, but if that's the
                // case, let 'em first disable Autocrypt and then change the key by attaching it.
                warn!(
                    context,
                    "not using attached PGP key for peer '{}' because another one is already set \
                    with prefer-encrypt={}",
                    peerstate.addr,
                    peerstate.prefer_encrypt,
                );
                return Ok(false);
            }
        }
        peerstate.public_key = Some(key);
        info!(
            context,
            "using attached PGP key for peer '{}' with prefer-encrypt=mutual", peerstate.addr,
        );
        peerstate.prefer_encrypt = EncryptPreference::Mutual;
        peerstate.save_to_db(&context.sql).await?;
        Ok(true)
    }

    fn do_add_single_part(&mut self, mut part: Part) {
        if self.was_encrypted() {
            part.param.set_int(Param::GuaranteeE2ee, 1);
        }
        self.parts.push(part);
    }

    pub(crate) fn get_mailinglist_header(&self) -> Option<&str> {
        if let Some(list_id) = self.get_header(HeaderDef::ListId) {
            // The message belongs to a mailing list and has a `ListId:`-header
            // that should be used to get a unique id.
            return Some(list_id);
        } else if let Some(sender) = self.get_header(HeaderDef::Sender) {
            // the `Sender:`-header alone is no indicator for mailing list
            // as also used for bot-impersonation via `set_override_sender_name()`
            if let Some(precedence) = self.get_header(HeaderDef::Precedence) {
                if precedence == "list" || precedence == "bulk" {
                    // The message belongs to a mailing list, but there is no `ListId:`-header;
                    // `Sender:`-header is be used to get a unique id.
                    // This method is used by implementations as Majordomo.
                    return Some(sender);
                }
            }
        }
        None
    }

    pub(crate) fn is_mailinglist_message(&self) -> bool {
        self.get_mailinglist_header().is_some()
    }

    /// Detects Schleuder mailing list by List-Help header.
    pub(crate) fn is_schleuder_message(&self) -> bool {
        if let Some(list_help) = self.get_header(HeaderDef::ListHelp) {
            list_help == "<https://schleuder.org/>"
        } else {
            false
        }
    }

    pub fn replace_msg_by_error(&mut self, error_msg: &str) {
        self.is_system_message = SystemMessage::Unknown;
        if let Some(part) = self.parts.first_mut() {
            part.typ = Viewtype::Text;
            part.msg = format!("[{error_msg}]");
            self.parts.truncate(1);
        }
    }

    pub(crate) fn get_rfc724_mid(&self) -> Option<String> {
        self.get_header(HeaderDef::MessageId)
            .and_then(|msgid| parse_message_id(msgid).ok())
    }

    fn remove_secured_headers(headers: &mut HashMap<String, String>) {
        headers.remove("secure-join-fingerprint");
        headers.remove("secure-join-auth");
        headers.remove("chat-verified");
        headers.remove("autocrypt-gossip");

        // Secure-Join is secured unless it is an initial "vc-request"/"vg-request".
        if let Some(secure_join) = headers.remove("secure-join") {
            if secure_join == "vc-request" || secure_join == "vg-request" {
                headers.insert("secure-join".to_string(), secure_join);
            }
        }
    }

    fn merge_headers(
        context: &Context,
        headers: &mut HashMap<String, String>,
        recipients: &mut Vec<SingleInfo>,
        from: &mut Option<SingleInfo>,
        list_post: &mut Option<String>,
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
        if from_new.is_some() {
            *from = from_new;
        }
        let list_post_new = get_list_post(fields);
        if list_post_new.is_some() {
            *list_post = list_post_new;
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
        if report_fields
            .get_header_value(HeaderDef::Disposition)
            .is_none()
        {
            warn!(
                context,
                "Ignoring unknown disposition-notification, Message-Id: {:?}.",
                report_fields.get_header_value(HeaderDef::MessageId)
            );
            return Ok(None);
        };

        let original_message_id = report_fields
            .get_header_value(HeaderDef::OriginalMessageId)
            // MS Exchange doesn't add an Original-Message-Id header. Instead, they put
            // the original message id into the In-Reply-To header:
            .or_else(|| report.headers.get_header_value(HeaderDef::InReplyTo))
            .and_then(|v| parse_message_id(&v).ok());
        let additional_message_ids = report_fields
            .get_header_value(HeaderDef::AdditionalMessageIds)
            .map_or_else(Vec::new, |v| {
                v.split(' ')
                    .filter_map(|s| parse_message_id(s).ok())
                    .collect()
            });

        Ok(Some(Report {
            original_message_id,
            additional_message_ids,
        }))
    }

    fn process_delivery_status(
        &self,
        context: &Context,
        report: &mailparse::ParsedMail<'_>,
    ) -> Result<Option<DeliveryReport>> {
        // Assume failure.
        let mut failure = true;

        if let Some(status_part) = report.subparts.get(1) {
            // RFC 3464 defines `message/delivery-status`
            // RFC 6533 defines `message/global-delivery-status`
            if status_part.ctype.mimetype != "message/delivery-status"
                && status_part.ctype.mimetype != "message/global-delivery-status"
            {
                warn!(context, "Second part of Delivery Status Notification is not message/delivery-status or message/global-delivery-status, ignoring");
                return Ok(None);
            }

            let status_body = status_part.get_body_raw()?;

            // Skip per-message fields.
            let (_, sz) = mailparse::parse_headers(&status_body)?;

            // Parse first set of per-recipient fields
            if let Some(status_body) = status_body.get(sz..) {
                let (status_fields, _) = mailparse::parse_headers(status_body)?;
                if let Some(action) = status_fields.get_first_value("action") {
                    if action != "failed" {
                        info!(context, "DSN with {:?} action", action);
                        failure = false;
                    }
                } else {
                    warn!(context, "DSN without action");
                }
            } else {
                warn!(context, "DSN without per-recipient fields");
            }
        } else {
            // No message/delivery-status part.
            return Ok(None);
        }

        // parse as mailheaders
        if let Some(original_msg) = report.subparts.get(2).filter(|p| {
            p.ctype.mimetype.contains("rfc822")
                || p.ctype.mimetype == "message/global"
                || p.ctype.mimetype == "message/global-headers"
        }) {
            let report_body = original_msg.get_body_raw()?;
            let (report_fields, _) = mailparse::parse_headers(&report_body)?;

            if let Some(original_message_id) = report_fields
                .get_header_value(HeaderDef::MessageId)
                .and_then(|v| parse_message_id(&v).ok())
            {
                return Ok(Some(DeliveryReport {
                    rfc724_mid: original_message_id,
                    failure,
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
    ///
    /// We make an exception for Schleuder mailing lists
    /// because they typically create messages with two text parts,
    /// one for headers and one for the actual contents.
    fn maybe_remove_inline_mailinglist_footer(&mut self) {
        if self.is_mailinglist_message() && !self.is_schleuder_message() {
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
    /// Also you should add a test in receive_imf.rs (there already are lots of test_parse_ndn_* tests).
    async fn heuristically_parse_ndn(&mut self, context: &Context) {
        let maybe_ndn = if let Some(from) = self.get_header(HeaderDef::From_) {
            let from = from.to_ascii_lowercase();
            from.contains("mailer-daemon") || from.contains("mail-daemon")
        } else {
            false
        };
        if maybe_ndn && self.delivery_report.is_none() {
            for original_message_id in self
                .parts
                .iter()
                .filter_map(|part| part.msg_raw.as_ref())
                .flat_map(|part| part.lines())
                .filter_map(|line| line.split_once("Message-ID:"))
                .filter_map(|(_, message_id)| parse_message_id(message_id).ok())
            {
                if let Ok(Some(_)) = message::rfc724_mid_exists(context, &original_message_id).await
                {
                    self.delivery_report = Some(DeliveryReport {
                        rfc724_mid: original_message_id,
                        failure: true,
                    })
                }
            }
        }
    }

    /// Handle reports
    /// (MDNs = Message Disposition Notification, the message was read
    /// and NDNs = Non delivery notification, the message could not be delivered)
    pub async fn handle_reports(&self, context: &Context, from_id: ContactId, parts: &[Part]) {
        for report in &self.mdn_reports {
            for original_message_id in report
                .original_message_id
                .iter()
                .chain(&report.additional_message_ids)
            {
                if let Err(err) =
                    handle_mdn(context, from_id, original_message_id, self.timestamp_sent).await
                {
                    warn!(context, "Could not handle MDN: {err:#}.");
                }
            }
        }

        if let Some(delivery_report) = &self.delivery_report {
            if delivery_report.failure {
                let error = parts
                    .iter()
                    .find(|p| p.typ == Viewtype::Text)
                    .map(|p| p.msg.clone());
                if let Err(err) = handle_ndn(context, delivery_report, error).await {
                    warn!(context, "Could not handle NDN: {err:#}.");
                }
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
                .query_get_value("SELECT timestamp FROM msgs WHERE rfc724_mid=?", (field,))
                .await?
        } else {
            None
        };
        Ok(parent_timestamp)
    }
}

/// Parses `Autocrypt-Gossip` headers from the email and applies them to peerstates.
/// Params:
/// from: The address which sent the message currently being parsed
///
/// Returns the set of mail recipient addresses for which valid gossip headers were found.
async fn update_gossip_peerstates(
    context: &Context,
    message_time: i64,
    from: &str,
    recipients: &[SingleInfo],
    gossip_headers: Vec<String>,
) -> Result<HashMap<String, SignedPublicKey>> {
    // XXX split the parsing from the modification part
    let mut gossiped_keys: HashMap<String, SignedPublicKey> = Default::default();

    for value in &gossip_headers {
        let header = match value.parse::<Aheader>() {
            Ok(header) => header,
            Err(err) => {
                warn!(context, "Failed parsing Autocrypt-Gossip header: {}", err);
                continue;
            }
        };

        if !recipients
            .iter()
            .any(|info| addr_cmp(&info.addr, &header.addr))
        {
            warn!(
                context,
                "Ignoring gossiped \"{}\" as the address is not in To/Cc list.", &header.addr,
            );
            continue;
        }
        if addr_cmp(from, &header.addr) {
            // Non-standard, but anyway we can't update the cached peerstate here.
            warn!(
                context,
                "Ignoring gossiped \"{}\" as it equals the From address", &header.addr,
            );
            continue;
        }

        let peerstate;
        if let Some(mut p) = Peerstate::from_addr(context, &header.addr).await? {
            p.apply_gossip(&header, message_time);
            p.save_to_db(&context.sql).await?;
            peerstate = p;
        } else {
            let p = Peerstate::from_gossip(&header, message_time);
            p.save_to_db(&context.sql).await?;
            peerstate = p;
        };
        peerstate
            .handle_fingerprint_change(context, message_time)
            .await?;

        gossiped_keys.insert(header.addr.to_lowercase(), header.public_key);
    }

    Ok(gossiped_keys)
}

/// Message Disposition Notification (RFC 8098)
#[derive(Debug)]
pub(crate) struct Report {
    /// Original-Message-ID header
    ///
    /// It MUST be present if the original message has a Message-ID according to RFC 8098.
    /// In case we can't find it (shouldn't happen), this is None.
    original_message_id: Option<String>,
    /// Additional-Message-IDs
    additional_message_ids: Vec<String>,
}

/// Delivery Status Notification (RFC 3464, RFC 6533)
#[derive(Debug)]
pub(crate) struct DeliveryReport {
    pub rfc724_mid: String,
    pub failure: bool,
}

pub(crate) fn parse_message_ids(ids: &str) -> Vec<String> {
    // take care with mailparse::msgidparse() that is pretty untolerant eg. wrt missing `<` or `>`
    let mut msgids = Vec::new();
    for id in ids.split_whitespace() {
        let mut id = id.to_string();
        if let Some(id_without_prefix) = id.strip_prefix('<') {
            id = id_without_prefix.to_string();
        };
        if let Some(id_without_suffix) = id.strip_suffix('>') {
            id = id_without_suffix.to_string();
        };
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

/// Returns true if the header overwrites outer header
/// when it comes from protected headers.
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
            | "secure-join"
    )
}

/// Parsed MIME part.
#[derive(Debug, Default, Clone)]
pub struct Part {
    /// Type of the MIME part determining how it should be displayed.
    pub typ: Viewtype,

    /// MIME type.
    pub mimetype: Option<Mime>,

    /// Message text to be displayed in the chat.
    pub msg: String,

    /// Message text to be displayed in message info.
    pub msg_raw: Option<String>,

    /// Size of the MIME part in bytes.
    pub bytes: usize,

    /// Parameters.
    pub param: Params,

    /// Attachment filename.
    pub(crate) org_filename: Option<String>,

    /// An error detected during parsing.
    pub error: Option<String>,

    /// True if conversion from HTML to plaintext failed.
    pub(crate) dehtml_failed: bool,

    /// the part is a child or a descendant of multipart/related.
    /// typically, these are images that are referenced from text/html part
    /// and should not displayed inside chat.
    ///
    /// note that multipart/related may contain further multipart nestings
    /// and all of them needs to be marked with `is_related`.
    pub(crate) is_related: bool,

    /// Part is an RFC 9078 reaction.
    pub(crate) is_reaction: bool,
}

/// Returns the mimetype and viewtype for a parsed mail.
///
/// This only looks at the metadata, not at the content;
/// the viewtype may later be corrected in `do_add_single_file_part()`.
fn get_mime_type(
    mail: &mailparse::ParsedMail<'_>,
    filename: &Option<String>,
) -> Result<(Mime, Viewtype)> {
    let mimetype = mail.ctype.mimetype.parse::<Mime>()?;

    let viewtype = match mimetype.type_() {
        mime::TEXT => match mimetype.subtype() {
            mime::VCARD => Viewtype::Vcard,
            mime::PLAIN | mime::HTML if !is_attachment_disposition(mail) => Viewtype::Text,
            _ => Viewtype::File,
        },
        mime::IMAGE => match mimetype.subtype() {
            mime::GIF => Viewtype::Gif,
            mime::SVG => Viewtype::File,
            _ => Viewtype::Image,
        },
        mime::AUDIO => Viewtype::Audio,
        mime::VIDEO => Viewtype::Video,
        mime::MULTIPART => Viewtype::Unknown,
        mime::MESSAGE => {
            if is_attachment_disposition(mail) {
                Viewtype::File
            } else {
                // Enacapsulated messages, see <https://www.w3.org/Protocols/rfc1341/7_3_Message.html>
                // Also used as part "message/disposition-notification" of "multipart/report", which, however, will
                // be handled separately.
                // I've not seen any messages using this, so we do not attach these parts (maybe they're used to attach replies,
                // which are unwanted at all).
                // For now, we skip these parts at all; if desired, we could return DcMimeType::File/DC_MSG_File
                // for selected and known subparts.
                Viewtype::Unknown
            }
        }
        mime::APPLICATION => match mimetype.subtype() {
            mime::OCTET_STREAM => match filename {
                Some(filename) => match message::guess_msgtype_from_suffix(Path::new(&filename)) {
                    Some((viewtype, _)) => viewtype,
                    None => Viewtype::File,
                },
                None => Viewtype::File,
            },
            _ => Viewtype::File,
        },
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
            desired_filename = Some(format!("file.{subtype}",));
        } else {
            bail!(
                "could not determine attachment filename: {:?}",
                ct.disposition
            );
        };
    }

    let desired_filename = desired_filename.map(|filename| sanitize_bidi_characters(&filename));

    Ok(desired_filename)
}

/// Returned addresses are normalized and lowercased.
pub(crate) fn get_recipients(headers: &[MailHeader]) -> Vec<SingleInfo> {
    let to_addresses = get_all_addresses_from_header(headers, "to");
    let cc_addresses = get_all_addresses_from_header(headers, "cc");

    let mut res = to_addresses;
    res.extend(cc_addresses);
    res
}

/// Returned addresses are normalized and lowercased.
pub(crate) fn get_from(headers: &[MailHeader]) -> Option<SingleInfo> {
    let all = get_all_addresses_from_header(headers, "from");
    tools::single_value(all)
}

/// Returned addresses are normalized and lowercased.
pub(crate) fn get_list_post(headers: &[MailHeader]) -> Option<String> {
    get_all_addresses_from_header(headers, "list-post")
        .into_iter()
        .next()
        .map(|s| s.addr)
}

/// Extracts all addresses from the header named `header`.
///
/// If multiple headers with the same name are present,
/// the last one is taken.
/// This is because DKIM-Signatures apply to the last
/// headers, and more headers may be added
/// to the beginning of the messages
/// without invalidating the signature
/// unless the header is "oversigned",
/// i.e. included in the signature more times
/// than it appears in the mail.
fn get_all_addresses_from_header(headers: &[MailHeader], header: &str) -> Vec<SingleInfo> {
    let mut result: Vec<SingleInfo> = Default::default();

    if let Some(header) = headers
        .iter()
        .rev()
        .find(|h| h.get_key().to_lowercase() == header)
    {
        if let Ok(addrs) = mailparse::addrparse_header(header) {
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
        }
    }

    result
}

async fn handle_mdn(
    context: &Context,
    from_id: ContactId,
    rfc724_mid: &str,
    timestamp_sent: i64,
) -> Result<()> {
    if from_id == ContactId::SELF {
        warn!(
            context,
            "Ignoring MDN sent to self, this is a bug on the sender device."
        );

        // This is not an error on our side,
        // we successfully ignored an invalid MDN and return `Ok`.
        return Ok(());
    }

    let Some((msg_id, chat_id, has_mdns, is_dup)) = context
        .sql
        .query_row_optional(
            concat!(
                "SELECT",
                "    m.id AS msg_id,",
                "    c.id AS chat_id,",
                "    mdns.contact_id AS mdn_contact",
                " FROM msgs m ",
                " LEFT JOIN chats c ON m.chat_id=c.id",
                " LEFT JOIN msgs_mdns mdns ON mdns.msg_id=m.id",
                " WHERE rfc724_mid=? AND from_id=1",
                " ORDER BY msg_id DESC, mdn_contact=? DESC",
                " LIMIT 1",
            ),
            (&rfc724_mid, from_id),
            |row| {
                let msg_id: MsgId = row.get("msg_id")?;
                let chat_id: ChatId = row.get("chat_id")?;
                let mdn_contact: Option<ContactId> = row.get("mdn_contact")?;
                Ok((
                    msg_id,
                    chat_id,
                    mdn_contact.is_some(),
                    mdn_contact == Some(from_id),
                ))
            },
        )
        .await?
    else {
        info!(
            context,
            "Ignoring MDN, found no message with Message-ID {rfc724_mid:?} sent by us in the database.",
        );
        return Ok(());
    };

    if is_dup {
        return Ok(());
    }
    context
        .sql
        .execute(
            "INSERT INTO msgs_mdns (msg_id, contact_id, timestamp_sent) VALUES (?, ?, ?)",
            (msg_id, from_id, timestamp_sent),
        )
        .await?;
    if !has_mdns {
        context.emit_event(EventType::MsgRead { chat_id, msg_id });
        // note(treefit): only matters if it is the last message in chat (but probably too expensive to check, debounce also solves it)
        chatlist_events::emit_chatlist_item_changed(context, chat_id);
    }
    Ok(())
}

/// Marks a message as failed after an ndn (non-delivery-notification) arrived.
/// Where appropriate, also adds an info message telling the user which of the recipients of a group message failed.
async fn handle_ndn(
    context: &Context,
    failed: &DeliveryReport,
    error: Option<String>,
) -> Result<()> {
    if failed.rfc724_mid.is_empty() {
        return Ok(());
    }

    // The NDN might be for a message-id that had attachments and was sent from a non-Delta Chat client.
    // In this case we need to mark multiple "msgids" as failed that all refer to the same message-id.
    let msgs: Vec<_> = context
        .sql
        .query_map(
            "SELECT id FROM msgs
                WHERE rfc724_mid=? AND from_id=1",
            (&failed.rfc724_mid,),
            |row| {
                let msg_id: MsgId = row.get(0)?;
                Ok(msg_id)
            },
            |rows| Ok(rows.collect::<Vec<_>>()),
        )
        .await?;

    let error = if let Some(error) = error {
        error
    } else {
        "Delivery to at least one recipient failed.".to_string()
    };
    let err_msg = &error;

    for msg in msgs {
        let msg_id = msg?;
        let mut message = Message::load_from_db(context, msg_id).await?;
        let aggregated_error = message
            .error
            .as_ref()
            .map(|err| format!("{}\n\n{}", err, err_msg));
        set_msg_failed(
            context,
            &mut message,
            aggregated_error.as_ref().unwrap_or(err_msg),
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use mailparse::ParsedMail;

    use super::*;
    use crate::{
        chat,
        chatlist::Chatlist,
        constants::{Blocked, DC_DESIRED_TEXT_LEN, DC_ELLIPSIS},
        message::{MessageState, MessengerMessage},
        receive_imf::receive_imf,
        test_utils::{TestContext, TestContextManager},
        tools::time,
    };

    impl AvatarAction {
        pub fn is_change(&self) -> bool {
            match self {
                AvatarAction::Delete => false,
                AvatarAction::Change(_) => true,
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mimeparser_fromheader() {
        let ctx = TestContext::new_alice().await;

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: g@c.de\n\nhi", None)
            .await
            .unwrap();
        let contact = mimemsg.from;
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, None);

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From:   g@c.de  \n\nhi", None)
            .await
            .unwrap();
        let contact = mimemsg.from;
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, None);

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: <g@c.de>\n\nhi", None)
            .await
            .unwrap();
        let contact = mimemsg.from;
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, None);

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: Goetz C <g@c.de>\n\nhi", None)
            .await
            .unwrap();
        let contact = mimemsg.from;
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Goetz C".to_string()));

        let mimemsg = MimeMessage::from_bytes(&ctx, b"From: \"Goetz C\" <g@c.de>\n\nhi", None)
            .await
            .unwrap();
        let contact = mimemsg.from;
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Goetz C".to_string()));

        let mimemsg =
            MimeMessage::from_bytes(&ctx, b"From: =?utf-8?q?G=C3=B6tz?= C <g@c.de>\n\nhi", None)
                .await
                .unwrap();
        let contact = mimemsg.from;
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Götz C".to_string()));

        // although RFC 2047 says, encoded-words shall not appear inside quoted-string,
        // this combination is used in the wild eg. by MailMate
        let mimemsg = MimeMessage::from_bytes(
            &ctx,
            b"From: \"=?utf-8?q?G=C3=B6tz?= C\" <g@c.de>\n\nhi",
            None,
        )
        .await
        .unwrap();
        let contact = mimemsg.from;
        assert_eq!(contact.addr, "g@c.de");
        assert_eq!(contact.display_name, Some("Götz C".to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mimeparser_crash() {
        let context = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();

        assert_eq!(mimeparser.get_subject(), None);
        assert_eq!(mimeparser.parts.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_rfc724_mid_exists() {
        let context = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/mail_with_message_id.txt");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();

        assert_eq!(
            mimeparser.get_rfc724_mid(),
            Some("2dfdbde7@example.org".into())
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_rfc724_mid_not_exists() {
        let context = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/issue_523.txt");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

        // If some header is present multiple times,
        // only the last one must be used.
        let raw = b"From: alice@example.org\n\
                    TO: mallory@example.com\n\
                    To: mallory@example.net\n\
                    To: bob@example.net\n\
                    Content-Type: text/plain\n\
                    Chat-Version: 1.0\n\
                    \n\
                    Hello\n\
                    ";
        let mail = mailparse::parse_mail(&raw[..]).unwrap();
        let recipients = get_recipients(&mail.headers);
        assert!(recipients.iter().any(|info| info.addr == "bob@example.net"));
        assert_eq!(recipients.len(), 1);
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_simple.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("test.html".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_encoded_words() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßnahmen Okt. 2020.html".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_encoded_words_binary() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words_binary.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some(" § 165 Abs".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_encoded_words_windows1251() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_encoded_words_windows1251.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("file Что нового 2020.pdf".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_apostrophed() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßnahmen Okt. 2021.html".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_apostrophed_cont() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_cont.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Maßnahmen März 2022.html".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_apostrophed_windows1251() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_windows1251.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("программирование.HTM".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_apostrophed_cp1252() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_cp1252.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("Auftragsbestätigung.pdf".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_attachment_filename_apostrophed_invalid() {
        let t = TestContext::new().await;
        let mail = load_mail_with_attachment(
            &t,
            include_bytes!("../test-data/message/attach_filename_apostrophed_invalid.eml"),
        );
        let filename = get_attachment_filename(&t, &mail.subparts[1]).unwrap();
        assert_eq!(filename, Some("somedäüta.html.zip".to_string()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_first_addr() {
        let context = TestContext::new().await;
        let raw = b"From: hello@one.org, world@two.org\n\
                    Chat-Disposition-Notification-To: wrong\n\
                    Content-Type: text/plain\n\
                    Chat-Version: 1.0\n\
                    \n\
                    test1\n\
                    ";

        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None).await;

        assert!(mimeparser.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_parent_timestamp() {
        let context = TestContext::new_alice().await;
        let raw = b"From: foo@example.org\n\
                    Content-Type: text/plain\n\
                    Chat-Version: 1.0\n\
                    In-Reply-To: <Gr.beZgAF2Nn0-.oyaJOpeuT70@example.org>\n\
                    \n\
                    Some reply\n\
                    ";
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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
                ("Gr.beZgAF2Nn0-.oyaJOpeuT70@example.org", timestamp),
            )
            .await
            .expect("Failed to write to the database");
        assert_eq!(
            mimeparser.get_parent_timestamp(&context.ctx).await.unwrap(),
            Some(timestamp)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mimeparser_with_context() {
        let context = TestContext::new_alice().await;
        let raw = b"From: hello@example.org\n\
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

        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();

        // non-overwritten headers do not bubble up
        let of = mimeparser.get_header(HeaderDef::SecureJoinGroup).unwrap();
        assert_eq!(of, "no");

        // unknown headers do not bubble upwards
        let of = mimeparser.get_header(HeaderDef::TestHeader).unwrap();
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mimeparser_with_avatars() {
        let t = TestContext::new_alice().await;

        let raw = include_bytes!("../test-data/message/mail_attach_txt.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..], None).await.unwrap();
        assert_eq!(mimeparser.user_avatar, None);
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..], None).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.unwrap().is_change());
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_avatar_deleted.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..], None).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert_eq!(mimeparser.user_avatar, Some(AvatarAction::Delete));
        assert_eq!(mimeparser.group_avatar, None);

        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..], None).await.unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Text);
        assert!(mimeparser.user_avatar.unwrap().is_change());
        assert!(mimeparser.group_avatar.unwrap().is_change());

        // if the Chat-User-Avatar header is missing, the avatar become a normal attachment
        let raw = include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml");
        let raw = String::from_utf8_lossy(raw).to_string();
        let raw = raw.replace("Chat-User-Avatar:", "Xhat-Xser-Xvatar:");
        let mimeparser = MimeMessage::from_bytes(&t, raw.as_bytes(), None)
            .await
            .unwrap();
        assert_eq!(mimeparser.parts.len(), 1);
        assert_eq!(mimeparser.parts[0].typ, Viewtype::Image);
        assert_eq!(mimeparser.user_avatar, None);
        assert!(mimeparser.group_avatar.unwrap().is_change());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mimeparser_with_videochat() {
        let t = TestContext::new_alice().await;

        let raw = include_bytes!("../test-data/message/videochat_invitation.eml");
        let mimeparser = MimeMessage::from_bytes(&t, &raw[..], None).await.unwrap();
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mimeparser_message_kml() {
        let context = TestContext::new_alice().await;
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

        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_mdn() {
        let context = TestContext::new_alice().await;
        let raw = b"Subject: =?utf-8?q?Chat=3A_Message_opened?=\n\
Date: Mon, 10 Jan 2020 00:00:00 +0000\n\
Chat-Version: 1.0\n\
Message-ID: <bar@example.org>\n\
To: Alice <alice@example.org>\n\
From: Bob <bob@example.org>\n\
Auto-Submitted: auto-replied\n\
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Chat: Message opened".to_string())
        );

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.mdn_reports.len(), 1);
        assert_eq!(message.is_bot, None);
    }

    /// Test parsing multiple MDNs combined in a single message.
    ///
    /// RFC 6522 specifically allows MDNs to be nested inside
    /// multipart MIME messages.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_multiple_mdns() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(
            message.get_subject(),
            Some("Chat: Message opened".to_string())
        );

        assert_eq!(message.parts.len(), 2);
        assert_eq!(message.mdn_reports.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_mdn_with_additional_message_ids() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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
            Some("foo@example.org".to_string())
        );
        assert_eq!(
            &message.mdn_reports[0].additional_message_ids,
            &["foo@example.com", "foo@example.net"]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_inline_attachment() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_hide_html_without_content() {
        let t = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&t, &raw[..], None).await.unwrap();

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::File);
        assert_eq!(message.parts[0].msg, "");

        // Make sure the file is there even though the html is wrong:
        let param = &message.parts[0].param;
        let blob: BlobObject = param.get_blob(Param::File, &t).await.unwrap().unwrap();
        let f = tokio::fs::File::open(blob.to_abs_path()).await.unwrap();
        let size = f.metadata().await.unwrap().len();
        assert_eq!(size, 154);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parse_inline_image() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(message.get_subject(), Some("example".to_string()));

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Image);
        assert_eq!(message.parts[0].msg, "example – Test");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parse_thunderbird_html_embedded_image() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(message.get_subject(), Some("Test subject".to_string()));

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Image);
        assert_eq!(message.parts[0].msg, "Test subject – Test");
    }

    // Outlook specifies filename in the "name" attribute of Content-Type
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parse_outlook_html_embedded_image() {
        let context = TestContext::new_alice().await;
        let raw = br#"From: Anonymous <anonymous@example.org>
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
"#;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parse_format_flowed_quote() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parse_quote_without_reply() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parse_quote_top_posting() {
        let context = TestContext::new_alice().await;
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

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_attachment_quote() {
        let context = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/quote_attach.eml");
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_quote_div() {
        let t = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/gmx-quote.eml");
        let mimeparser = MimeMessage::from_bytes(&t, raw, None).await.unwrap();
        assert_eq!(mimeparser.parts[0].msg, "YIPPEEEEEE\n\nMulti-line");
        assert_eq!(mimeparser.parts[0].param.get(Param::Quote).unwrap(), "Now?");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_allinkl_blockquote() {
        // all-inkl.com puts quotes into `<blockquote> </blockquote>`.
        let t = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/allinkl-quote.eml");
        let mimeparser = MimeMessage::from_bytes(&t, raw, None).await.unwrap();
        assert!(mimeparser.parts[0].msg.starts_with("It's 1.0."));
        assert_eq!(
            mimeparser.parts[0].param.get(Param::Quote).unwrap(),
            "What's the version?"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_subj_to_multimedia_msg() {
        let t = TestContext::new_alice().await;
        receive_imf(
            &t.ctx,
            include_bytes!("../test-data/message/subj_with_multimedia_msg.eml"),
            false,
        )
        .await
        .unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        let msg_id = chats.get_msg_id(0).unwrap().unwrap();
        let msg = Message::load_from_db(&t.ctx, msg_id).await.unwrap();

        assert_eq!(msg.text, "subj with important info – body text");
        assert_eq!(msg.viewtype, Viewtype::Image);
        assert_eq!(msg.error(), None);
        assert_eq!(msg.is_dc_message, MessengerMessage::No);
        assert_eq!(msg.chat_blocked, Blocked::Request);
        assert_eq!(msg.state, MessageState::InFresh);
        assert_eq!(msg.get_filebytes(&t).await.unwrap().unwrap(), 2115);
        assert!(msg.get_file(&t).is_some());
        assert_eq!(msg.get_filename().unwrap(), "avatar64x64.png");
        assert_eq!(msg.get_width(), 64);
        assert_eq!(msg.get_height(), 64);
        assert_eq!(msg.get_filemime().unwrap(), "image/png");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mime_modified_plain() {
        let t = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/text_plain_unspecified.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw, None).await.unwrap();
        assert!(!mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "This message does not have Content-Type nor Subject."
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mime_modified_alt_plain_html() {
        let t = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/text_alt_plain_html.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw, None).await.unwrap();
        assert!(mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – this is plain"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mime_modified_alt_plain() {
        let t = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/text_alt_plain.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw, None).await.unwrap();
        assert!(!mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – \
        mime-modified should not be set set as there is no html and no special stuff;\n\
        although not being a delta-message.\n\
        test some special html-characters as < > and & but also \" and ' :)"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mime_modified_alt_html() {
        let t = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/text_alt_html.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw, None).await.unwrap();
        assert!(mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – mime-modified *set*; simplify is always regarded as lossy."
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mime_modified_html() {
        let t = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/text_html.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, raw, None).await.unwrap();
        assert!(mimeparser.is_mime_modified);
        assert_eq!(
            mimeparser.parts[0].msg,
            "mime-modified test – mime-modified *set*; simplify is always regarded as lossy."
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mime_modified_large_plain() -> Result<()> {
        let t = TestContext::new_alice().await;
        let t1 = TestContext::new_alice().await;

        static REPEAT_TXT: &str = "this text with 42 chars is just repeated.\n";
        static REPEAT_CNT: usize = DC_DESIRED_TEXT_LEN / REPEAT_TXT.len() + 2;
        let long_txt = format!("From: alice@c.de\n\n{}", REPEAT_TXT.repeat(REPEAT_CNT));
        assert_eq!(long_txt.matches("just repeated").count(), REPEAT_CNT);
        assert!(long_txt.len() > DC_DESIRED_TEXT_LEN);

        {
            let mimemsg = MimeMessage::from_bytes(&t, long_txt.as_ref(), None).await?;
            assert!(mimemsg.is_mime_modified);
            assert!(
                mimemsg.parts[0].msg.matches("just repeated").count()
                    <= DC_DESIRED_TEXT_LEN / REPEAT_TXT.len()
            );
            assert!(mimemsg.parts[0].msg.len() <= DC_DESIRED_TEXT_LEN + DC_ELLIPSIS.len());
        }

        for draft in [false, true] {
            let chat = t.get_self_chat().await;
            let mut msg = Message::new_text(long_txt.clone());
            if draft {
                chat.id.set_draft(&t, Some(&mut msg)).await?;
            }
            let sent_msg = t.send_msg(chat.id, &mut msg).await;
            let msg = t.get_last_msg_in(chat.id).await;
            assert!(msg.has_html());
            let html = msg.id.get_html(&t).await?.unwrap();
            assert_eq!(html.matches("<!DOCTYPE html>").count(), 1);
            assert_eq!(html.matches("just repeated.<br/>").count(), REPEAT_CNT);
            assert!(
                msg.text.matches("just repeated.").count()
                    <= DC_DESIRED_TEXT_LEN / REPEAT_TXT.len()
            );
            assert!(msg.text.len() <= DC_DESIRED_TEXT_LEN + DC_ELLIPSIS.len());

            let msg = t1.recv_msg(&sent_msg).await;
            assert!(msg.has_html());
            assert_eq!(msg.id.get_html(&t1).await?.unwrap(), html);
        }

        t.set_config(Config::Bot, Some("1")).await?;
        {
            let mimemsg = MimeMessage::from_bytes(&t, long_txt.as_ref(), None).await?;
            assert!(!mimemsg.is_mime_modified);
            assert_eq!(
                format!("{}\n", mimemsg.parts[0].msg),
                REPEAT_TXT.repeat(REPEAT_CNT)
            );
        }

        Ok(())
    }

    /// Tests that sender status (signature) does not appear
    /// in HTML view of a long message.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_large_message_no_signature() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let bob = &tcm.bob().await;

        alice
            .set_config(Config::Selfstatus, Some("Some signature"))
            .await?;
        let chat = alice.create_chat(bob).await;
        let txt = "Hello!\n".repeat(500);
        let sent = alice.send_text(chat.id, &txt).await;
        let msg = bob.recv_msg(&sent).await;

        assert_eq!(msg.has_html(), true);
        let html = msg.id.get_html(bob).await?.unwrap();
        assert_eq!(html.contains("Some signature"), false);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_x_microsoft_original_message_id() {
        let t = TestContext::new_alice().await;
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
                ", None)
            .await
            .unwrap();
        assert_eq!(
            message.get_rfc724_mid(),
            Some("Mr.6Dx7ITn4w38.n9j7epIcuQI@outlook.com".to_string())
        );
    }

    /// Tests that X-Microsoft-Original-Message-ID does not overwrite encrypted Message-ID.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_x_microsoft_original_message_id_precedence() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        let bob_chat_id = tcm.send_recv_accept(&alice, &bob, "hi").await.chat_id;
        chat::send_text_msg(&bob, bob_chat_id, "hi!".to_string()).await?;
        let mut sent_msg = bob.pop_sent_msg().await;

        // Insert X-Microsoft-Original-Message-ID.
        // It should be ignored because there is a Message-ID in the encrypted part.
        sent_msg.payload = sent_msg.payload.replace(
            "Message-ID:",
            "X-Microsoft-Original-Message-ID: <fake-message-id@example.net>\r\nMessage-ID:",
        );

        let msg = alice.recv_msg(&sent_msg).await;
        assert!(!msg.rfc724_mid.contains("fake-message-id"));
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_long_in_reply_to() -> Result<()> {
        let t = TestContext::new_alice().await;

        // A message with a long Message-ID.
        // Long message-IDs are generated by Mailjet.
        let raw = br"Date: Thu, 28 Jan 2021 00:26:57 +0000
Chat-Version: 1.0\n\
Message-ID: <ABCDEFGH.1234567_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA@mailjet.com>
To: Bob <bob@example.org>
From: Alice <alice@example.org>
Subject: ...

Some quote.
";
        receive_imf(&t, raw, false).await?;

        // Delta Chat generates In-Reply-To with a starting tab when Message-ID is too long.
        let raw = br"In-Reply-To:
	<ABCDEFGH.1234567_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA@mailjet.com>
Date: Thu, 28 Jan 2021 00:26:57 +0000
Chat-Version: 1.0\n\
Message-ID: <foobar@example.org>
To: Alice <alice@example.org>
From: Bob <bob@example.org>
Subject: ...

> Some quote.

Some reply
";

        receive_imf(&t, raw, false).await?;

        let msg = t.get_last_msg().await;
        assert_eq!(msg.get_text(), "Some reply");
        let quoted_message = msg.quoted_message(&t).await?.unwrap();
        assert_eq!(quoted_message.get_text(), "Some quote.");

        Ok(())
    }

    // Test that WantsMdn parameter is not set on outgoing messages.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_outgoing_wants_mdn() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let raw = br"Date: Thu, 28 Jan 2021 00:26:57 +0000
Chat-Version: 1.0\n\
Message-ID: <foobarbaz@example.org>
To: Bob <bob@example.org>
From: Alice <alice@example.org>
Subject: subject
Chat-Disposition-Notification-To: alice@example.org

Message.
";

        // Bob receives message.
        receive_imf(&bob, raw, false).await?;
        let msg = bob.get_last_msg().await;
        // Message is incoming.
        assert!(msg.param.get_bool(Param::WantsMdn).unwrap());

        // Alice receives copy-to-self.
        receive_imf(&alice, raw, false).await?;
        let msg = alice.get_last_msg().await;
        // Message is outgoing, don't send read receipt to self.
        assert!(msg.param.get_bool(Param::WantsMdn).is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ignore_read_receipt_to_self() -> Result<()> {
        let alice = TestContext::new_alice().await;

        // Alice receives BCC-self copy of a message sent to Bob.
        receive_imf(
            &alice,
            "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.org\n\
                 To: bob@example.net\n\
                 Subject: foo\n\
                 Message-ID: first@example.com\n\
                 Chat-Version: 1.0\n\
                 Chat-Disposition-Notification-To: alice@example.org\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n"
                .as_bytes(),
            false,
        )
        .await?;
        let msg = alice.get_last_msg().await;
        assert_eq!(msg.state, MessageState::OutRcvd);

        // Due to a bug in the old version running on the other device, Alice receives a read
        // receipt from self.
        receive_imf(
            &alice,
                "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.org\n\
                 To: alice@example.org\n\
                 Subject: message opened\n\
                 Date: Sun, 22 Mar 2020 23:37:57 +0000\n\
                 Chat-Version: 1.0\n\
                 Message-ID: second@example.com\n\
                 Content-Type: multipart/report; report-type=disposition-notification; boundary=\"SNIPP\"\n\
                 \n\
                 \n\
                 --SNIPP\n\
                 Content-Type: text/plain; charset=utf-8\n\
                 \n\
                 Read receipts do not guarantee sth. was read.\n\
                 \n\
                 \n\
                 --SNIPP\n\
                 Content-Type: message/disposition-notification\n\
                 \n\
                 Original-Recipient: rfc822;bob@example.com\n\
                 Final-Recipient: rfc822;bob@example.com\n\
                 Original-Message-ID: <first@example.com>\n\
                 Disposition: manual-action/MDN-sent-automatically; displayed\n\
                 \n\
                 \n\
                 --SNIPP--"
            .as_bytes(),
            false,
        )
        .await?;

        // Check that the state has not changed to `MessageState::OutMdnRcvd`.
        let msg = Message::load_from_db(&alice, msg.id).await?;
        assert_eq!(msg.state, MessageState::OutRcvd);

        Ok(())
    }

    /// Test parsing of MDN sent by MS Exchange.
    ///
    /// It does not have required Original-Message-ID field, so it is useless, but we want to
    /// recognize it as MDN nevertheless to avoid displaying it in the chat as normal message.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ms_exchange_mdn() -> Result<()> {
        let t = TestContext::new_alice().await;

        let original =
            include_bytes!("../test-data/message/ms_exchange_report_original_message.eml");
        receive_imf(&t, original, false).await?;
        let original_msg_id = t.get_last_msg().await.id;

        // 1. Test mimeparser directly
        let mdn =
            include_bytes!("../test-data/message/ms_exchange_report_disposition_notification.eml");
        let mimeparser = MimeMessage::from_bytes(&t.ctx, mdn, None).await?;
        assert_eq!(mimeparser.mdn_reports.len(), 1);
        assert_eq!(
            mimeparser.mdn_reports[0].original_message_id.as_deref(),
            Some("d5904dc344eeb5deaf9bb44603f0c716@posteo.de")
        );
        assert!(mimeparser.mdn_reports[0].additional_message_ids.is_empty());

        // 2. Test that marking the original msg as read works
        receive_imf(&t, mdn, false).await?;

        assert_eq!(
            original_msg_id.get_state(&t).await?,
            MessageState::OutMdnRcvd
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_receive_eml() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let mime_message = MimeMessage::from_bytes(
            &alice,
            include_bytes!("../test-data/message/attached-eml.eml"),
            None,
        )
        .await?;

        assert_eq!(mime_message.parts.len(), 1);
        assert_eq!(mime_message.parts[0].typ, Viewtype::File);
        assert_eq!(
            mime_message.parts[0].mimetype,
            Some("message/rfc822".parse().unwrap(),)
        );
        assert_eq!(
            mime_message.parts[0].msg,
            "this is a classic email – I attached the .EML file".to_string()
        );
        assert_eq!(
            mime_message.parts[0].param.get(Param::File),
            Some("$BLOBDIR/.eml")
        );

        assert_eq!(mime_message.parts[0].org_filename, Some(".eml".to_string()));

        Ok(())
    }

    /// Tests parsing of MIME message containing RFC 9078 reaction.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_reaction() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let mime_message = MimeMessage::from_bytes(
            &alice,
            "To: alice@example.org\n\
From: bob@example.net\n\
Date: Today, 29 February 2021 00:00:10 -800\n\
Message-ID: 56789@example.net\n\
In-Reply-To: 12345@example.org\n\
Subject: Meeting\n\
Mime-Version: 1.0 (1.0)\n\
Content-Type: text/plain; charset=utf-8\n\
Content-Disposition: reaction\n\
\n\
\u{1F44D}"
                .as_bytes(),
            None,
        )
        .await?;

        assert_eq!(mime_message.parts.len(), 1);
        assert_eq!(mime_message.parts[0].is_reaction, true);
        assert_eq!(
            mime_message
                .get_header(HeaderDef::InReplyTo)
                .and_then(|msgid| parse_message_id(msgid).ok())
                .unwrap(),
            "12345@example.org"
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_jpeg_as_application_octet_stream() -> Result<()> {
        let context = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/jpeg-as-application-octet-stream.eml");

        let msg = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(msg.parts.len(), 1);
        assert_eq!(msg.parts[0].typ, Viewtype::Image);

        receive_imf(&context, &raw[..], false).await?;
        let msg = context.get_last_msg().await;
        assert_eq!(msg.get_viewtype(), Viewtype::Image);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_schleuder() -> Result<()> {
        let context = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/schleuder.eml");

        let msg = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(msg.parts.len(), 2);

        // Header part.
        assert_eq!(msg.parts[0].typ, Viewtype::Text);

        // Actual contents part.
        assert_eq!(msg.parts[1].typ, Viewtype::Text);
        assert_eq!(msg.parts[1].msg, "hello,\nbye");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_tlsrpt() -> Result<()> {
        let context = TestContext::new_alice().await;
        let raw = include_bytes!("../test-data/message/tlsrpt.eml");

        let msg = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(msg.parts.len(), 1);

        assert_eq!(msg.parts[0].typ, Viewtype::File);
        assert_eq!(msg.parts[0].msg, "Report Domain: nine.testrun.org Submitter: google.com Report-ID: <2024.01.20T00.00.00Z+nine.testrun.org@google.com> – This is an aggregate TLS report from google.com");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_time_in_future() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let beginning_time = time();

        // Receive a message with a date far in the future (year 3004)
        // I'm just going to assume that no one uses this code after the year 3000
        let mime_message = MimeMessage::from_bytes(
            &alice,
            b"To: alice@example.org\n\
              From: bob@example.net\n\
              Date: Today, 29 February 3004 00:00:10 -800\n\
              Message-ID: 56789@example.net\n\
              Subject: Meeting\n\
              Mime-Version: 1.0 (1.0)\n\
              Content-Type: text/plain; charset=utf-8\n\
              \n\
              Hi",
            None,
        )
        .await?;

        // We do allow the time to be in the future a bit (because of unsynchronized clocks),
        // but only 60 seconds:
        assert!(mime_message.timestamp_sent <= time() + 60);
        assert!(mime_message.timestamp_sent >= beginning_time + 60);
        assert!(mime_message.timestamp_rcvd <= time());

        Ok(())
    }

    /// Tests that subject is not prepended to the message
    /// when bot receives it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_bot_no_subject() {
        let context = TestContext::new().await;
        context.set_config(Config::Bot, Some("1")).await.unwrap();
        let raw = br#"Message-ID: <foobar@example.org>
From: foo <foo@example.org>
Subject: Some subject
To: bar@example.org
MIME-Version: 1.0
Content-Type: text/plain; charset=utf-8

/help
"#;

        let message = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(message.get_subject(), Some("Some subject".to_string()));

        assert_eq!(message.parts.len(), 1);
        assert_eq!(message.parts[0].typ, Viewtype::Text);
        // Not "Some subject – /help"
        assert_eq!(message.parts[0].msg, "/help");
    }

    /// Tests that Delta Chat takes the last header value
    /// rather than the first one if multiple headers
    /// are present.
    ///
    /// DKIM signature applies to the last N headers
    /// if header name is included N times in
    /// DKIM-Signature.
    ///
    /// If the client takes the first header
    /// rather than the last, it can be fooled
    /// into using unsigned header
    /// when signed one is present
    /// but not protected by oversigning.
    ///
    /// See
    /// <https://www.zone.eu/blog/2024/05/17/bimi-and-dmarc-cant-save-you/>
    /// for reference.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_take_last_header() {
        let context = TestContext::new().await;

        // Mallory added second From: header.
        let raw = b"From: mallory@example.org\n\
                    From: alice@example.org\n\
                    Content-Type: text/plain\n\
                    Chat-Version: 1.0\n\
                    \n\
                    Hello\n\
                    ";

        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..], None)
            .await
            .unwrap();
        assert_eq!(
            mimeparser.get_header(HeaderDef::From_).unwrap(),
            "alice@example.org"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_protect_autocrypt() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let bob = &tcm.bob().await;

        alice
            .set_config_bool(Config::ProtectAutocrypt, true)
            .await?;
        bob.set_config_bool(Config::ProtectAutocrypt, true).await?;

        let msg = tcm.send_recv_accept(alice, bob, "Hello!").await;
        assert_eq!(msg.get_showpadlock(), false);

        let msg = tcm.send_recv(bob, alice, "Hi!").await;
        assert_eq!(msg.get_showpadlock(), true);

        Ok(())
    }
}

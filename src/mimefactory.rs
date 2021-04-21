use std::convert::TryInto;

use anyhow::{bail, ensure, format_err, Result};
use async_std::prelude::*;
use chrono::TimeZone;
use lettre_email::{mime, Address, Header, MimeMultipartType, PartBuilder};
use sqlx::Row;

use crate::blob::BlobObject;
use crate::chat::{self, Chat};
use crate::config::Config;
use crate::constants::{Chattype, Viewtype, DC_FROM_HANDSHAKE};
use crate::contact::Contact;
use crate::context::{get_version_str, Context};
use crate::dc_tools::IsNoneOrEmpty;
use crate::dc_tools::{
    dc_create_outgoing_rfc724_mid, dc_create_smeared_timestamp, dc_get_filebytes,
    remove_subject_prefix, time,
};
use crate::e2ee::EncryptHelper;
use crate::ephemeral::Timer as EphemeralTimer;
use crate::format_flowed::{format_flowed, format_flowed_quote};
use crate::html::new_html_mimepart;
use crate::location;
use crate::message::{self, Message, MsgId};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::peerstate::{Peerstate, PeerstateVerifiedStatus};
use crate::simplify::escape_message_footer_marks;
use crate::stock_str;

// attachments of 25 mb brutto should work on the majority of providers
// (brutto examples: web.de=50, 1&1=40, t-online.de=32, gmail=25, posteo=50, yahoo=25, all-inkl=100).
// as an upper limit, we double the size; the core won't send messages larger than this
// to get the netto sizes, we subtract 1 mb header-overhead and the base64-overhead.
pub const RECOMMENDED_FILE_SIZE: u64 = 24 * 1024 * 1024 / 4 * 3;
const UPPER_LIMIT_FILE_SIZE: u64 = 49 * 1024 * 1024 / 4 * 3;

#[derive(Debug, Clone)]
pub enum Loaded {
    Message { chat: Chat },
    Mdn { additional_msg_ids: Vec<String> },
}

/// Helper to construct mime messages.
#[derive(Debug, Clone)]
pub struct MimeFactory<'a> {
    from_addr: String,
    from_displayname: String,

    /// Goes to the `Sender:`-header, if set.
    /// For overridden names, `sender_displayname` is set to the
    /// config-name while `from_displayname` is set to the overridden name.
    /// From the perspective of the receiver,
    /// a set `Sender:`-header is used as an indicator that the name is overridden;
    /// names are alsways read from the `From:`-header.
    sender_displayname: Option<String>,

    selfstatus: String,

    /// Vector of pairs of recipient name and address
    recipients: Vec<(String, String)>,

    timestamp: i64,
    loaded: Loaded,
    msg: &'a Message,
    in_reply_to: String,
    references: String,
    req_mdn: bool,
    last_added_location_id: u32,
    attach_selfavatar: bool,
}

/// Result of rendering a message, ready to be submitted to a send job.
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub message: Vec<u8>,
    // pub envelope: Envelope,
    pub is_encrypted: bool,
    pub is_gossiped: bool,
    pub last_added_location_id: u32,

    /// Message ID (Message in the sense of Email)
    pub rfc724_mid: String,
    pub subject: String,
}

#[derive(Debug, Clone, Default)]
struct MessageHeaders {
    /// Opportunistically protected headers.
    ///
    /// These headers are placed into encrypted part *if* the message is encrypted. Place headers
    /// which are not needed before decryption (e.g. Chat-Group-Name) or are not interesting if the
    /// message cannot be decrypted (e.g. Chat-Disposition-Notification-To) here.
    ///
    /// If the message is not encrypted, these headers are placed into IMF header section, so make
    /// sure that the message will be encrypted if you place any sensitive information here.
    pub protected: Vec<Header>,

    /// Headers that must go into IMF header section.
    ///
    /// These are standard headers such as Date, In-Reply-To, References, which cannot be placed
    /// anywhere else according to the standard. Placing headers here also allows them to be fetched
    /// individually over IMAP without downloading the message body. This is why Chat-Version is
    /// placed here.
    pub unprotected: Vec<Header>,

    /// Headers that MUST NOT go into IMF header section.
    ///
    /// These are large headers which may hit the header section size limit on the server, such as
    /// Chat-User-Avatar with a base64-encoded image inside.
    pub hidden: Vec<Header>,
}

impl<'a> MimeFactory<'a> {
    pub async fn from_msg(
        context: &Context,
        msg: &'a Message,
        attach_selfavatar: bool,
    ) -> Result<MimeFactory<'a>> {
        let chat = Chat::load_from_db(context, msg.chat_id).await?;

        let from_addr = context
            .get_config(Config::ConfiguredAddr)
            .await?
            .unwrap_or_default();

        let config_displayname = context
            .get_config(Config::Displayname)
            .await?
            .unwrap_or_default();
        let (from_displayname, sender_displayname) =
            if let Some(override_name) = msg.param.get(Param::OverrideSenderDisplayname) {
                (override_name.to_string(), Some(config_displayname))
            } else {
                (config_displayname, None)
            };

        let mut recipients = Vec::with_capacity(5);
        let mut req_mdn = false;

        if chat.is_self_talk() {
            recipients.push((from_displayname.to_string(), from_addr.to_string()));
        } else {
            let mut rows = context
                .sql
                .fetch(
                    sqlx::query(
                        "SELECT c.authname, c.addr  \
                 FROM chats_contacts cc  \
                 LEFT JOIN contacts c ON cc.contact_id=c.id  \
                 WHERE cc.chat_id=? AND cc.contact_id>9;",
                    )
                    .bind(msg.chat_id),
                )
                .await?;
            while let Some(row) = rows.next().await {
                let row = row?;
                let authname: String = row.try_get(0)?;
                let addr: String = row.try_get(1)?;
                if !recipients_contain_addr(&recipients, &addr) {
                    recipients.push((authname, addr));
                }
            }

            if !msg.is_system_message() && context.get_config_bool(Config::MdnsEnabled).await? {
                req_mdn = true;
            }
        }
        let row = context
            .sql
            .fetch_one(
                sqlx::query("SELECT mime_in_reply_to, mime_references FROM msgs WHERE id=?")
                    .bind(msg.id),
            )
            .await?;
        let (in_reply_to, references) = (
            render_rfc724_mid_list(row.try_get(0)?),
            render_rfc724_mid_list(row.try_get(1)?),
        );

        let default_str = stock_str::status_line(context).await;
        let factory = MimeFactory {
            from_addr,
            from_displayname,
            sender_displayname,
            selfstatus: context
                .get_config(Config::Selfstatus)
                .await?
                .unwrap_or(default_str),
            recipients,
            timestamp: msg.timestamp_sort,
            loaded: Loaded::Message { chat },
            msg,
            in_reply_to,
            references,
            req_mdn,
            last_added_location_id: 0,
            attach_selfavatar,
        };
        Ok(factory)
    }

    pub async fn from_mdn(
        context: &Context,
        msg: &'a Message,
        additional_msg_ids: Vec<String>,
    ) -> Result<MimeFactory<'a>> {
        ensure!(!msg.chat_id.is_special(), "Invalid chat id");

        let contact = Contact::load_from_db(context, msg.from_id).await?;
        let from_addr = context
            .get_config(Config::ConfiguredAddr)
            .await?
            .unwrap_or_default();
        let from_displayname = context
            .get_config(Config::Displayname)
            .await?
            .unwrap_or_default();
        let default_str = stock_str::status_line(context).await;
        let selfstatus = context
            .get_config(Config::Selfstatus)
            .await?
            .unwrap_or(default_str);
        let timestamp = dc_create_smeared_timestamp(context).await;

        let res = MimeFactory::<'a> {
            from_addr,
            from_displayname,
            sender_displayname: None,
            selfstatus,
            recipients: vec![(
                contact.get_authname().to_string(),
                contact.get_addr().to_string(),
            )],
            timestamp,
            loaded: Loaded::Mdn { additional_msg_ids },
            msg,
            in_reply_to: String::default(),
            references: String::default(),
            req_mdn: false,
            last_added_location_id: 0,
            attach_selfavatar: false,
        };

        Ok(res)
    }

    async fn peerstates_for_recipients(
        &self,
        context: &Context,
    ) -> Result<Vec<(Option<Peerstate>, &str)>> {
        let self_addr = context
            .get_config(Config::ConfiguredAddr)
            .await?
            .ok_or_else(|| format_err!("Not configured"))?;

        let mut res = Vec::new();
        for (_, addr) in self
            .recipients
            .iter()
            .filter(|(_, addr)| addr != &self_addr)
        {
            res.push((Peerstate::from_addr(context, addr).await?, addr.as_str()));
        }

        Ok(res)
    }

    fn is_e2ee_guaranteed(&self) -> bool {
        match &self.loaded {
            Loaded::Message { chat } => {
                if chat.is_protected() {
                    return true;
                }

                !self
                    .msg
                    .param
                    .get_bool(Param::ForcePlaintext)
                    .unwrap_or_default()
                    && self
                        .msg
                        .param
                        .get_bool(Param::GuaranteeE2ee)
                        .unwrap_or_default()
            }
            Loaded::Mdn { .. } => false,
        }
    }

    fn min_verified(&self) -> PeerstateVerifiedStatus {
        match &self.loaded {
            Loaded::Message { chat } => {
                if chat.is_protected() {
                    PeerstateVerifiedStatus::BidirectVerified
                } else {
                    PeerstateVerifiedStatus::Unverified
                }
            }
            Loaded::Mdn { .. } => PeerstateVerifiedStatus::Unverified,
        }
    }

    fn should_force_plaintext(&self) -> bool {
        match &self.loaded {
            Loaded::Message { chat } => {
                if chat.is_protected() {
                    false
                } else {
                    self.msg
                        .param
                        .get_bool(Param::ForcePlaintext)
                        .unwrap_or_default()
                }
            }
            Loaded::Mdn { .. } => true,
        }
    }

    fn should_skip_autocrypt(&self) -> bool {
        match &self.loaded {
            Loaded::Message { .. } => self
                .msg
                .param
                .get_bool(Param::SkipAutocrypt)
                .unwrap_or_default(),
            Loaded::Mdn { .. } => true,
        }
    }

    async fn should_do_gossip(&self, context: &Context) -> Result<bool> {
        match &self.loaded {
            Loaded::Message { chat } => {
                // beside key- and member-changes, force re-gossip every 48 hours
                let gossiped_timestamp = chat.get_gossiped_timestamp(context).await?;
                if time() > gossiped_timestamp + (2 * 24 * 60 * 60) {
                    Ok(true)
                } else {
                    Ok(self.msg.param.get_cmd() == SystemMessage::MemberAddedToGroup)
                }
            }
            Loaded::Mdn { .. } => Ok(false),
        }
    }

    fn grpimage(&self) -> Option<String> {
        match &self.loaded {
            Loaded::Message { chat } => {
                let cmd = self.msg.param.get_cmd();

                match cmd {
                    SystemMessage::MemberAddedToGroup => {
                        return chat.param.get(Param::ProfileImage).map(Into::into);
                    }
                    SystemMessage::GroupImageChanged => {
                        return self.msg.param.get(Param::Arg).map(Into::into)
                    }
                    _ => {}
                }

                if self
                    .msg
                    .param
                    .get_bool(Param::AttachGroupImage)
                    .unwrap_or_default()
                {
                    return chat.param.get(Param::ProfileImage).map(Into::into);
                }

                None
            }
            Loaded::Mdn { .. } => None,
        }
    }

    async fn subject_str(&self, context: &Context) -> anyhow::Result<String> {
        let quoted_msg_subject = self.msg.quoted_message(context).await?.map(|m| m.subject);

        let subject = match self.loaded {
            Loaded::Message { ref chat } => {
                if self.msg.param.get_cmd() == SystemMessage::AutocryptSetupMessage {
                    return Ok(stock_str::ac_setup_msg_subject(context).await);
                }

                if !self.msg.subject.is_empty() {
                    return Ok(self.msg.subject.clone());
                }

                if chat.typ == Chattype::Group && quoted_msg_subject.is_none_or_empty() {
                    // If we have a `quoted_msg_subject`, we use the subject of the quoted message
                    // instead of the group name
                    let re = if self.in_reply_to.is_empty() {
                        ""
                    } else {
                        "Re: "
                    };
                    return Ok(format!("{}{}", re, chat.name));
                }

                let parent_subject = if quoted_msg_subject.is_none_or_empty() {
                    chat.param.get(Param::LastSubject)
                } else {
                    quoted_msg_subject.as_deref()
                };

                if let Some(last_subject) = parent_subject {
                    format!("Re: {}", remove_subject_prefix(last_subject))
                } else {
                    let self_name = match context.get_config(Config::Displayname).await? {
                        Some(name) => name,
                        None => context.get_config(Config::Addr).await?.unwrap_or_default(),
                    };

                    stock_str::subject_for_new_contact(context, self_name).await
                }
            }
            Loaded::Mdn { .. } => stock_str::read_rcpt(context).await,
        };

        Ok(subject)
    }

    pub fn recipients(&self) -> Vec<String> {
        self.recipients
            .iter()
            .map(|(_, addr)| addr.clone())
            .collect()
    }

    pub async fn render(mut self, context: &Context) -> Result<RenderedEmail> {
        let mut headers: MessageHeaders = Default::default();

        let from = Address::new_mailbox_with_name(
            self.from_displayname.to_string(),
            self.from_addr.clone(),
        );

        let mut to = Vec::new();
        for (name, addr) in self.recipients.iter() {
            if name.is_empty() {
                to.push(Address::new_mailbox(addr.clone()));
            } else {
                to.push(Address::new_mailbox_with_name(
                    name.to_string(),
                    addr.clone(),
                ));
            }
        }

        if to.is_empty() {
            to.push(from.clone());
        }

        headers
            .unprotected
            .push(Header::new("MIME-Version".into(), "1.0".into()));

        if !self.references.is_empty() {
            headers
                .unprotected
                .push(Header::new("References".into(), self.references.clone()));
        }

        if !self.in_reply_to.is_empty() {
            headers
                .unprotected
                .push(Header::new("In-Reply-To".into(), self.in_reply_to.clone()));
        }

        let date = chrono::Utc
            .from_local_datetime(&chrono::NaiveDateTime::from_timestamp(self.timestamp, 0))
            .unwrap()
            .to_rfc2822();

        headers.unprotected.push(Header::new("Date".into(), date));

        headers
            .unprotected
            .push(Header::new("Chat-Version".to_string(), "1.0".to_string()));

        if let Loaded::Mdn { .. } = self.loaded {
            headers.unprotected.push(Header::new(
                "Auto-Submitted".to_string(),
                "auto-replied".to_string(),
            ));
        }

        if self.req_mdn {
            // we use "Chat-Disposition-Notification-To"
            // because replies to "Disposition-Notification-To" are weird in many cases
            // eg. are just freetext and/or do not follow any standard.
            headers.protected.push(Header::new(
                "Chat-Disposition-Notification-To".into(),
                self.from_addr.clone(),
            ));
        }

        let min_verified = self.min_verified();
        let grpimage = self.grpimage();
        let force_plaintext = self.should_force_plaintext();
        let skip_autocrypt = self.should_skip_autocrypt();
        let subject_str = self.subject_str(context).await?;
        let e2ee_guaranteed = self.is_e2ee_guaranteed();
        let encrypt_helper = EncryptHelper::new(context).await?;

        let encoded_subject = if subject_str
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ')
        // We do not use needs_encoding() here because needs_encoding() returns true if the string contains a space
        // but we do not want to encode all subjects just because they contain a space.
        {
            subject_str.clone()
        } else {
            encode_words(&subject_str)
        };

        if !skip_autocrypt {
            // unless determined otherwise we add the Autocrypt header
            let aheader = encrypt_helper.get_aheader().to_string();
            headers
                .unprotected
                .push(Header::new("Autocrypt".into(), aheader));
        }

        headers
            .protected
            .push(Header::new("Subject".into(), encoded_subject));

        let rfc724_mid = match self.loaded {
            Loaded::Message { .. } => self.msg.rfc724_mid.clone(),
            Loaded::Mdn { .. } => dc_create_outgoing_rfc724_mid(None, &self.from_addr),
        };

        let ephemeral_timer = self.msg.chat_id.get_ephemeral_timer(context).await?;
        if let EphemeralTimer::Enabled { duration } = ephemeral_timer {
            headers.protected.push(Header::new(
                "Ephemeral-Timer".to_string(),
                duration.to_string(),
            ));
        }

        headers.unprotected.push(Header::new(
            "Message-ID".into(),
            render_rfc724_mid(&rfc724_mid),
        ));

        headers
            .unprotected
            .push(Header::new_with_value("To".into(), to).unwrap());
        headers
            .unprotected
            .push(Header::new_with_value("From".into(), vec![from]).unwrap());
        if let Some(sender_displayname) = &self.sender_displayname {
            let sender =
                Address::new_mailbox_with_name(sender_displayname.clone(), self.from_addr.clone());
            headers
                .unprotected
                .push(Header::new_with_value("Sender".into(), vec![sender]).unwrap());
        }

        let mut is_gossiped = false;

        let (main_part, parts) = match self.loaded {
            Loaded::Message { .. } => {
                self.render_message(context, &mut headers, &grpimage)
                    .await?
            }
            Loaded::Mdn { .. } => (self.render_mdn(context).await?, Vec::new()),
        };

        let peerstates = self.peerstates_for_recipients(context).await?;
        let should_encrypt =
            encrypt_helper.should_encrypt(context, e2ee_guaranteed, &peerstates)?;
        let is_encrypted = should_encrypt && !force_plaintext;

        let message = if parts.is_empty() {
            // Single part, render as regular message.
            main_part
        } else {
            // Multiple parts, render as multipart.
            parts.into_iter().fold(
                PartBuilder::new()
                    .message_type(MimeMultipartType::Mixed)
                    .child(main_part.build()),
                |message, part| message.child(part.build()),
            )
        };

        // Store protected headers in the inner message.
        let message = headers
            .protected
            .into_iter()
            .fold(message, |message, header| message.header(header));

        let outer_message = if is_encrypted {
            // Add hidden headers to encrypted payload.
            let mut message = headers
                .hidden
                .into_iter()
                .fold(message, |message, header| message.header(header));

            // Add gossip headers in chats with multiple recipients
            if peerstates.len() > 1 && self.should_do_gossip(context).await? {
                for peerstate in peerstates.iter().filter_map(|(state, _)| state.as_ref()) {
                    if peerstate.peek_key(min_verified).is_some() {
                        if let Some(header) = peerstate.render_gossip_header(min_verified) {
                            message =
                                message.header(Header::new("Autocrypt-Gossip".into(), header));
                            is_gossiped = true;
                        }
                    }
                }
            }

            // Set the appropriate Content-Type for the inner message.
            let mut existing_ct = message
                .get_header("Content-Type".to_string())
                .and_then(|h| h.get_value::<String>().ok())
                .unwrap_or_else(|| "text/plain; charset=utf-8;".to_string());

            if !existing_ct.ends_with(';') {
                existing_ct += ";";
            }
            let message = message.replace_header(Header::new(
                "Content-Type".to_string(),
                format!("{} protected-headers=\"v1\";", existing_ct),
            ));

            // Set the appropriate Content-Type for the outer message
            let outer_message = PartBuilder::new().header((
                "Content-Type".to_string(),
                "multipart/encrypted; protocol=\"application/pgp-encrypted\"".to_string(),
            ));

            if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
                info!(context, "mimefactory: outgoing message mime:");
                let raw_message = message.clone().build().as_string();
                println!("{}", raw_message);
            }

            let encrypted = encrypt_helper
                .encrypt(context, min_verified, message, peerstates)
                .await?;

            outer_message
                .child(
                    // Autocrypt part 1
                    PartBuilder::new()
                        .content_type(&"application/pgp-encrypted".parse::<mime::Mime>().unwrap())
                        .header(("Content-Description", "PGP/MIME version identification"))
                        .body("Version: 1\r\n")
                        .build(),
                )
                .child(
                    // Autocrypt part 2
                    PartBuilder::new()
                        .content_type(
                            &"application/octet-stream; name=\"encrypted.asc\""
                                .parse::<mime::Mime>()
                                .unwrap(),
                        )
                        .header(("Content-Description", "OpenPGP encrypted message"))
                        .header(("Content-Disposition", "inline; filename=\"encrypted.asc\";"))
                        .body(encrypted)
                        .build(),
                )
                .header(("Subject".to_string(), "...".to_string()))
        } else if headers.hidden.is_empty() {
            message
        } else {
            let message = headers
                .hidden
                .into_iter()
                .fold(message, |message, header| message.header(header));

            PartBuilder::new()
                .message_type(MimeMultipartType::Mixed)
                .child(message.build())
        };

        // Store the unprotected headers on the outer message.
        let outer_message = headers
            .unprotected
            .into_iter()
            .fold(outer_message, |message, header| message.header(header));

        let MimeFactory {
            last_added_location_id,
            ..
        } = self;

        Ok(RenderedEmail {
            message: outer_message.build().as_string().into_bytes(),
            // envelope: Envelope::new,
            is_encrypted,
            is_gossiped,
            last_added_location_id,
            rfc724_mid,
            subject: subject_str,
        })
    }

    fn get_message_kml_part(&self) -> Option<PartBuilder> {
        let latitude = self.msg.param.get_float(Param::SetLatitude)?;
        let longitude = self.msg.param.get_float(Param::SetLongitude)?;

        let kml_file = location::get_message_kml(self.msg.timestamp_sort, latitude, longitude);
        let part = PartBuilder::new()
            .content_type(
                &"application/vnd.google-earth.kml+xml"
                    .parse::<mime::Mime>()
                    .unwrap(),
            )
            .header((
                "Content-Disposition",
                "attachment; filename=\"message.kml\"",
            ))
            .body(kml_file);
        Some(part)
    }

    async fn get_location_kml_part(&mut self, context: &Context) -> Result<PartBuilder> {
        let (kml_content, last_added_location_id) =
            location::get_kml(context, self.msg.chat_id).await?;
        let part = PartBuilder::new()
            .content_type(
                &"application/vnd.google-earth.kml+xml"
                    .parse::<mime::Mime>()
                    .unwrap(),
            )
            .header((
                "Content-Disposition",
                "attachment; filename=\"location.kml\"",
            ))
            .body(kml_content);
        if !self.msg.param.exists(Param::SetLatitude) {
            // otherwise, the independent location is already filed
            self.last_added_location_id = last_added_location_id;
        }
        Ok(part)
    }

    #[allow(clippy::cognitive_complexity)]
    async fn render_message(
        &mut self,
        context: &Context,
        headers: &mut MessageHeaders,
        grpimage: &Option<String>,
    ) -> Result<(PartBuilder, Vec<PartBuilder>)> {
        let chat = match &self.loaded {
            Loaded::Message { chat } => chat,
            Loaded::Mdn { .. } => bail!("Attempt to render MDN as a message"),
        };
        let command = self.msg.param.get_cmd();
        let mut placeholdertext = None;
        let mut meta_part = None;

        if chat.is_protected() {
            headers
                .protected
                .push(Header::new("Chat-Verified".to_string(), "1".to_string()));
        }

        if chat.typ == Chattype::Group {
            headers
                .protected
                .push(Header::new("Chat-Group-ID".into(), chat.grpid.clone()));

            let encoded = encode_words(&chat.name);
            headers
                .protected
                .push(Header::new("Chat-Group-Name".into(), encoded));

            match command {
                SystemMessage::MemberRemovedFromGroup => {
                    let email_to_remove = self.msg.param.get(Param::Arg).unwrap_or_default();
                    if !email_to_remove.is_empty() {
                        headers.protected.push(Header::new(
                            "Chat-Group-Member-Removed".into(),
                            email_to_remove.into(),
                        ));
                    }
                }
                SystemMessage::MemberAddedToGroup => {
                    let email_to_add = self.msg.param.get(Param::Arg).unwrap_or_default();
                    if !email_to_add.is_empty() {
                        headers.protected.push(Header::new(
                            "Chat-Group-Member-Added".into(),
                            email_to_add.into(),
                        ));
                    }
                    if 0 != self.msg.param.get_int(Param::Arg2).unwrap_or_default()
                        & DC_FROM_HANDSHAKE
                    {
                        info!(
                            context,
                            "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>",
                            "vg-member-added",
                        );
                        headers.protected.push(Header::new(
                            "Secure-Join".to_string(),
                            "vg-member-added".to_string(),
                        ));
                    }
                }
                SystemMessage::GroupNameChanged => {
                    let old_name = self.msg.param.get(Param::Arg).unwrap_or_default();
                    headers.protected.push(Header::new(
                        "Chat-Group-Name-Changed".into(),
                        maybe_encode_words(old_name),
                    ));
                }
                SystemMessage::GroupImageChanged => {
                    headers.protected.push(Header::new(
                        "Chat-Content".to_string(),
                        "group-avatar-changed".to_string(),
                    ));
                    if grpimage.is_none() {
                        headers.protected.push(Header::new(
                            "Chat-Group-Avatar".to_string(),
                            "0".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }

        match command {
            SystemMessage::LocationStreamingEnabled => {
                headers.protected.push(Header::new(
                    "Chat-Content".into(),
                    "location-streaming-enabled".into(),
                ));
            }
            SystemMessage::EphemeralTimerChanged => {
                headers.protected.push(Header::new(
                    "Chat-Content".to_string(),
                    "ephemeral-timer-changed".to_string(),
                ));
            }
            SystemMessage::LocationOnly => {
                // This should prevent automatic replies,
                // such as non-delivery reports.
                //
                // See https://tools.ietf.org/html/rfc3834
                //
                // Adding this header without encryption leaks some
                // information about the message contents, but it can
                // already be easily guessed from message timing and size.
                headers.unprotected.push(Header::new(
                    "Auto-Submitted".to_string(),
                    "auto-generated".to_string(),
                ));
            }
            SystemMessage::AutocryptSetupMessage => {
                headers
                    .unprotected
                    .push(Header::new("Autocrypt-Setup-Message".into(), "v1".into()));

                placeholdertext = Some(stock_str::ac_setup_msg_body(context).await);
            }
            SystemMessage::SecurejoinMessage => {
                let msg = &self.msg;
                let step = msg.param.get(Param::Arg).unwrap_or_default();
                if !step.is_empty() {
                    info!(
                        context,
                        "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>", step,
                    );
                    headers
                        .protected
                        .push(Header::new("Secure-Join".into(), step.into()));

                    let param2 = msg.param.get(Param::Arg2).unwrap_or_default();
                    if !param2.is_empty() {
                        headers.protected.push(Header::new(
                            if step == "vg-request-with-auth" || step == "vc-request-with-auth" {
                                "Secure-Join-Auth".into()
                            } else {
                                "Secure-Join-Invitenumber".into()
                            },
                            param2.into(),
                        ));
                    }

                    let fingerprint = msg.param.get(Param::Arg3).unwrap_or_default();
                    if !fingerprint.is_empty() {
                        headers.protected.push(Header::new(
                            "Secure-Join-Fingerprint".into(),
                            fingerprint.into(),
                        ));
                    }
                    if let Some(id) = msg.param.get(Param::Arg4) {
                        headers
                            .protected
                            .push(Header::new("Secure-Join-Group".into(), id.into()));
                    };
                }
            }
            SystemMessage::ChatProtectionEnabled => {
                headers.protected.push(Header::new(
                    "Chat-Content".to_string(),
                    "protection-enabled".to_string(),
                ));
            }
            SystemMessage::ChatProtectionDisabled => {
                headers.protected.push(Header::new(
                    "Chat-Content".to_string(),
                    "protection-disabled".to_string(),
                ));
            }
            _ => {}
        }

        if let Some(grpimage) = grpimage {
            info!(context, "setting group image '{}'", grpimage);
            let mut meta = Message {
                viewtype: Viewtype::Image,
                ..Default::default()
            };
            meta.param.set(Param::File, grpimage);

            let (mail, filename_as_sent) = build_body_file(context, &meta, "group-image").await?;
            meta_part = Some(mail);
            headers
                .protected
                .push(Header::new("Chat-Group-Avatar".into(), filename_as_sent));
        }

        if self.msg.viewtype == Viewtype::Sticker {
            headers
                .protected
                .push(Header::new("Chat-Content".into(), "sticker".into()));
        } else if self.msg.viewtype == Viewtype::VideochatInvitation {
            headers.protected.push(Header::new(
                "Chat-Content".into(),
                "videochat-invitation".into(),
            ));
            headers.protected.push(Header::new(
                "Chat-Webrtc-Room".into(),
                self.msg
                    .param
                    .get(Param::WebrtcRoom)
                    .unwrap_or_default()
                    .into(),
            ));
        }

        if self.msg.viewtype == Viewtype::Voice
            || self.msg.viewtype == Viewtype::Audio
            || self.msg.viewtype == Viewtype::Video
        {
            if self.msg.viewtype == Viewtype::Voice {
                headers
                    .protected
                    .push(Header::new("Chat-Voice-Message".into(), "1".into()));
            }
            let duration_ms = self.msg.param.get_int(Param::Duration).unwrap_or_default();
            if duration_ms > 0 {
                let dur = duration_ms.to_string();
                headers
                    .protected
                    .push(Header::new("Chat-Duration".into(), dur));
            }
        }

        // add text part - we even add empty text and force a MIME-multipart-message as:
        // - some Apps have problems with Non-text in the main part (eg. "Mail" from stock Android)
        // - we can add "forward hints" this way
        // - it looks better

        let afwd_email = self.msg.param.exists(Param::Forwarded);
        let fwdhint = if afwd_email {
            Some(
                "---------- Forwarded message ----------\r\n\
                 From: Delta Chat\r\n\
                 \r\n"
                    .to_string(),
            )
        } else {
            None
        };
        let final_text = {
            if let Some(ref text) = placeholdertext {
                text
            } else if let Some(ref text) = self.msg.text {
                text
            } else {
                ""
            }
        };

        let quoted_text = self
            .msg
            .quoted_text()
            .map(|quote| format_flowed_quote(&quote) + "\r\n\r\n");
        let flowed_text = format_flowed(final_text);

        let footer = &self.selfstatus;
        let message_text = format!(
            "{}{}{}{}{}{}",
            fwdhint.unwrap_or_default(),
            quoted_text.unwrap_or_default(),
            escape_message_footer_marks(&flowed_text),
            if !final_text.is_empty() && !footer.is_empty() {
                "\r\n\r\n"
            } else {
                ""
            },
            if !footer.is_empty() { "-- \r\n" } else { "" },
            footer
        );

        // Message is sent as text/plain, with charset = utf-8
        let mut main_part = PartBuilder::new()
            .header((
                "Content-Type".to_string(),
                "text/plain; charset=utf-8; format=flowed; delsp=no".to_string(),
            ))
            .body(message_text);
        let mut parts = Vec::new();

        // add HTML-part, this is needed only if a HTML-message from a non-delta-client is forwarded;
        // for simplificity and to avoid conversion errors, we're generating the HTML-part from the original message.
        if self.msg.has_html() {
            let html = if let Some(orig_msg_id) = self.msg.param.get_int(Param::Forwarded) {
                MsgId::new(orig_msg_id.try_into()?)
                    .get_html(context)
                    .await?
            } else {
                self.msg.param.get(Param::SendHtml).map(|s| s.to_string())
            };
            if let Some(html) = html {
                main_part = PartBuilder::new()
                    .message_type(MimeMultipartType::Alternative)
                    .child(main_part.build())
                    .child(new_html_mimepart(html).await.build());
            }
        }

        // add attachment part
        if chat::msgtype_has_file(self.msg.viewtype) {
            if !is_file_size_okay(context, self.msg).await? {
                bail!(
                    "Message exceeds the recommended {} MB.",
                    RECOMMENDED_FILE_SIZE / 1_000_000,
                );
            } else {
                let (file_part, _) = build_body_file(context, self.msg, "").await?;
                parts.push(file_part);
            }
        }

        if let Some(meta_part) = meta_part {
            parts.push(meta_part);
        }

        if let Some(msg_kml_part) = self.get_message_kml_part() {
            parts.push(msg_kml_part);
        }

        if location::is_sending_locations_to_chat(context, Some(self.msg.chat_id)).await {
            match self.get_location_kml_part(context).await {
                Ok(part) => parts.push(part),
                Err(err) => {
                    warn!(context, "mimefactory: could not send location: {}", err);
                }
            }
        }

        if self.attach_selfavatar {
            match context.get_config(Config::Selfavatar).await? {
                Some(path) => match build_selfavatar_file(context, &path) {
                    Ok(avatar) => headers.hidden.push(Header::new(
                        "Chat-User-Avatar".into(),
                        format!("base64:{}", avatar),
                    )),
                    Err(err) => warn!(context, "mimefactory: cannot attach selfavatar: {}", err),
                },
                None => headers
                    .protected
                    .push(Header::new("Chat-User-Avatar".into(), "0".into())),
            }
        }

        Ok((main_part, parts))
    }

    /// Render an MDN
    async fn render_mdn(&mut self, context: &Context) -> Result<PartBuilder> {
        // RFC 6522, this also requires the `report-type` parameter which is equal
        // to the MIME subtype of the second body part of the multipart/report
        //
        // currently, we do not send MDNs encrypted:
        // - in a multi-device-setup that is not set up properly, MDNs would disturb the communication as they
        //   are send automatically which may lead to spreading outdated Autocrypt headers.
        // - they do not carry any information but the Message-ID
        // - this save some KB
        // - in older versions, we did not encrypt messages to ourself when they to to SMTP - however, if these messages
        //   are forwarded for any reasons (eg. gmail always forwards to IMAP), we have no chance to decrypt them;
        //   this issue is fixed with 0.9.4

        let additional_msg_ids = match &self.loaded {
            Loaded::Message { .. } => bail!("Attempt to render a message as MDN"),
            Loaded::Mdn {
                additional_msg_ids, ..
            } => additional_msg_ids,
        };

        let mut message = PartBuilder::new().header((
            "Content-Type".to_string(),
            "multipart/report; report-type=disposition-notification".to_string(),
        ));

        // first body part: always human-readable, always REQUIRED by RFC 6522
        let p1 = if 0
            != self
                .msg
                .param
                .get_int(Param::GuaranteeE2ee)
                .unwrap_or_default()
        {
            stock_str::encrypted_msg(context).await
        } else {
            self.msg.get_summarytext(context, 32).await
        };
        let p2 = stock_str::read_rcpt_mail_body(context, p1).await;
        let message_text = format!("{}\r\n", p2);
        message = message.child(
            PartBuilder::new()
                .content_type(&mime::TEXT_PLAIN_UTF_8)
                .body(message_text)
                .build(),
        );

        // second body part: machine-readable, always REQUIRED by RFC 6522
        let version = get_version_str();
        let message_text2 = format!(
            "Reporting-UA: Delta Chat {}\r\n\
             Original-Recipient: rfc822;{}\r\n\
             Final-Recipient: rfc822;{}\r\n\
             Original-Message-ID: <{}>\r\n\
             Disposition: manual-action/MDN-sent-automatically; displayed\r\n",
            version, self.from_addr, self.from_addr, self.msg.rfc724_mid
        );

        let extension_fields = if additional_msg_ids.is_empty() {
            "".to_string()
        } else {
            "Additional-Message-IDs: ".to_string()
                + &additional_msg_ids
                    .iter()
                    .map(|mid| render_rfc724_mid(mid))
                    .collect::<Vec<String>>()
                    .join(" ")
                + "\r\n"
        };

        message = message.child(
            PartBuilder::new()
                .content_type(&"message/disposition-notification".parse().unwrap())
                .body(message_text2 + &extension_fields)
                .build(),
        );

        Ok(message)
    }
}

/// Returns base64-encoded buffer `buf` split into 78-bytes long
/// chunks separated by CRLF.
///
/// This line length limit is an
/// [RFC5322 requirement](https://tools.ietf.org/html/rfc5322#section-2.1.1).
fn wrapped_base64_encode(buf: &[u8]) -> String {
    let base64 = base64::encode(&buf);
    let mut chars = base64.chars();
    std::iter::repeat_with(|| chars.by_ref().take(78).collect::<String>())
        .take_while(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\r\n")
}

async fn build_body_file(
    context: &Context,
    msg: &Message,
    base_name: &str,
) -> Result<(PartBuilder, String)> {
    let blob = msg
        .param
        .get_blob(Param::File, context, true)
        .await?
        .ok_or_else(|| format_err!("msg has no filename"))?;
    let suffix = blob.suffix().unwrap_or("dat");

    // Get file name to use for sending.  For privacy purposes, we do
    // not transfer the original filenames eg. for images; these names
    // are normally not needed and contain timestamps, running numbers
    // etc.
    let filename_to_send: String = match msg.viewtype {
        Viewtype::Voice => chrono::Utc
            .timestamp(msg.timestamp_sort, 0)
            .format(&format!("voice-message_%Y-%m-%d_%H-%M-%S.{}", &suffix))
            .to_string(),
        Viewtype::Image | Viewtype::Gif => format!(
            "{}.{}",
            if base_name.is_empty() {
                chrono::Utc
                    .timestamp(msg.timestamp_sort, 0)
                    .format("image_%Y-%m-%d_%H-%M-%S")
                    .to_string()
            } else {
                base_name.to_string()
            },
            &suffix,
        ),
        Viewtype::Video => format!(
            "video_{}.{}",
            chrono::Utc
                .timestamp(msg.timestamp_sort, 0)
                .format("%Y-%m-%d_%H-%M-%S")
                .to_string(),
            &suffix
        ),
        _ => blob.as_file_name().to_string(),
    };

    /* check mimetype */
    let mimetype: mime::Mime = match msg.param.get(Param::MimeType) {
        Some(mtype) => mtype.parse()?,
        None => {
            if let Some(res) = message::guess_msgtype_from_suffix(blob.as_rel_path()) {
                res.1.parse()?
            } else {
                mime::APPLICATION_OCTET_STREAM
            }
        }
    };

    // create mime part, for Content-Disposition, see RFC 2183.
    // `Content-Disposition: attachment` seems not to make a difference to `Content-Disposition: inline`
    // at least on tested Thunderbird and Gma'l in 2017.
    // But I've heard about problems with inline and outl'k, so we just use the attachment-type until we
    // run into other problems ...
    let cd_value = format!(
        "attachment; filename=\"{}\"",
        maybe_encode_words(&filename_to_send)
    );

    let body = std::fs::read(blob.to_abs_path())?;
    let encoded_body = wrapped_base64_encode(&body);

    let mail = PartBuilder::new()
        .content_type(&mimetype)
        .header(("Content-Disposition", cd_value))
        .header(("Content-Transfer-Encoding", "base64"))
        .body(encoded_body);

    Ok((mail, filename_to_send))
}

fn build_selfavatar_file(context: &Context, path: &str) -> Result<String> {
    let blob = BlobObject::from_path(context, path)?;
    let body = std::fs::read(blob.to_abs_path())?;
    let encoded_body = wrapped_base64_encode(&body);
    Ok(encoded_body)
}

fn recipients_contain_addr(recipients: &[(String, String)], addr: &str) -> bool {
    let addr_lc = addr.to_lowercase();
    recipients
        .iter()
        .any(|(_, cur)| cur.to_lowercase() == addr_lc)
}

async fn is_file_size_okay(context: &Context, msg: &Message) -> Result<bool> {
    match msg.param.get_path(Param::File, context)? {
        Some(path) => {
            let bytes = dc_get_filebytes(context, &path).await;
            Ok(bytes <= UPPER_LIMIT_FILE_SIZE)
        }
        None => Ok(false),
    }
}

fn render_rfc724_mid(rfc724_mid: &str) -> String {
    let rfc724_mid = rfc724_mid.trim().to_string();

    if rfc724_mid.chars().next().unwrap_or_default() == '<' {
        rfc724_mid
    } else {
        format!("<{}>", rfc724_mid)
    }
}

fn render_rfc724_mid_list(mid_list: &str) -> String {
    mid_list
        .trim()
        .split_ascii_whitespace()
        .map(render_rfc724_mid)
        .collect::<Vec<String>>()
        .join(" ")
}

/* ******************************************************************************
 * Encode/decode header words, RFC 2047
 ******************************************************************************/

fn encode_words(word: &str) -> String {
    encoded_words::encode(word, None, encoded_words::EncodingFlag::Shortest, None)
}

fn needs_encoding(to_check: impl AsRef<str>) -> bool {
    !to_check.as_ref().chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' || c == '%'
    })
}

fn maybe_encode_words(words: &str) -> String {
    if needs_encoding(words) {
        encode_words(words)
    } else {
        words.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ChatId;

    use crate::contact::Origin;
    use crate::dc_receive_imf::dc_receive_imf;
    use crate::mimeparser::MimeMessage;
    use crate::test_utils::TestContext;
    use crate::{chatlist::Chatlist, test_utils::get_chat_msg};

    use async_std::fs::File;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_render_email_address() {
        let display_name = "ä space";
        let addr = "x@y.org";

        assert!(!display_name.is_ascii());
        assert!(!display_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' '));

        let s = format!(
            "{}",
            Address::new_mailbox_with_name(display_name.to_string(), addr.to_string())
        );

        println!("{}", s);

        assert_eq!(s, "=?utf-8?q?=C3=A4_space?= <x@y.org>");
    }

    #[test]
    fn test_render_email_address_noescape() {
        let display_name = "a space";
        let addr = "x@y.org";

        assert!(display_name.is_ascii());
        assert!(display_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' '));

        let s = format!(
            "{}",
            Address::new_mailbox_with_name(display_name.to_string(), addr.to_string())
        );

        // Addresses should not be unnecessarily be encoded, see https://github.com/deltachat/deltachat-core-rust/issues/1575:
        assert_eq!(s, "a space <x@y.org>");
    }

    #[test]
    fn test_render_rfc724_mid() {
        assert_eq!(
            render_rfc724_mid("kqjwle123@qlwe"),
            "<kqjwle123@qlwe>".to_string()
        );
        assert_eq!(
            render_rfc724_mid("  kqjwle123@qlwe "),
            "<kqjwle123@qlwe>".to_string()
        );
        assert_eq!(
            render_rfc724_mid("<kqjwle123@qlwe>"),
            "<kqjwle123@qlwe>".to_string()
        );
    }

    #[test]
    fn test_render_rc724_mid_list() {
        assert_eq!(render_rfc724_mid_list("123@q "), "<123@q>".to_string());
        assert_eq!(render_rfc724_mid_list(" 123@q "), "<123@q>".to_string());
        assert_eq!(
            render_rfc724_mid_list("123@q 456@d "),
            "<123@q> <456@d>".to_string()
        );
    }

    #[test]
    fn test_wrapped_base64_encode() {
        let input = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let output =
            "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQU\r\n\
             FBQUFBQUFBQQ==";
        assert_eq!(wrapped_base64_encode(input), output);
    }

    #[test]
    fn test_needs_encoding() {
        assert!(!needs_encoding(""));
        assert!(!needs_encoding("foobar"));
        assert!(needs_encoding(" "));
        assert!(needs_encoding("foo bar"));
    }

    #[test]
    fn test_maybe_encode_words() {
        assert_eq!(maybe_encode_words("foobar"), "foobar");
        assert_eq!(maybe_encode_words("-_.~%"), "-_.~%");
        assert_eq!(maybe_encode_words("äöü"), "=?utf-8?b?w6TDtsO8?=");
    }

    #[async_std::test]
    async fn test_subject_from_mua() {
        // 1.: Receive a mail from an MUA
        assert_eq!(
            msg_to_subject_str(
                b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: Bob <bob@example.com>\n\
                To: alice@example.com\n\
                Subject: Antw: Chat: hello\n\
                Message-ID: <2222@example.com>\n\
                Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                \n\
                hello\n"
            )
            .await,
            "Re: Chat: hello"
        );

        assert_eq!(
            msg_to_subject_str(
                b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: Bob <bob@example.com>\n\
                To: alice@example.com\n\
                Subject: Infos: 42\n\
                Message-ID: <2222@example.com>\n\
                Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                \n\
                hello\n"
            )
            .await,
            "Re: Infos: 42"
        );
    }

    #[async_std::test]
    async fn test_subject_from_dc() {
        // 2. Receive a message from Delta Chat
        assert_eq!(
            msg_to_subject_str(
                b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: bob@example.com\n\
                To: alice@example.com\n\
                Subject: Chat: hello\n\
                Chat-Version: 1.0\n\
                Message-ID: <2223@example.com>\n\
                Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                \n\
                hello\n"
            )
            .await,
            "Re: Chat: hello"
        );
    }

    #[async_std::test]
    async fn test_subject_outgoing() {
        // 3. Send the first message to a new contact
        let t = TestContext::new_alice().await;

        assert_eq!(first_subject_str(t).await, "Message from alice@example.com");

        let t = TestContext::new_alice().await;
        t.set_config(Config::Displayname, Some("Alice"))
            .await
            .unwrap();
        assert_eq!(first_subject_str(t).await, "Message from Alice");
    }

    #[async_std::test]
    async fn test_subject_unicode() {
        // 4. Receive messages with unicode characters and make sure that we do not panic (we do not care about the result)
        msg_to_subject_str(
            "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
            From: bob@example.com\n\
            To: alice@example.com\n\
            Subject: äääää\n\
            Chat-Version: 1.0\n\
            Message-ID: <2893@example.com>\n\
            Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
            \n\
            hello\n"
                .as_bytes(),
        )
        .await;

        msg_to_subject_str(
            "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
            From: bob@example.com\n\
            To: alice@example.com\n\
            Subject: aäääää\n\
            Chat-Version: 1.0\n\
            Message-ID: <2893@example.com>\n\
            Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
            \n\
            hello\n"
                .as_bytes(),
        )
        .await;
    }

    #[async_std::test]
    async fn test_subject_mdn() {
        // 5. Receive an mdn (read receipt) and make sure the mdn's subject is not used
        let t = TestContext::new_alice().await;
        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
            From: alice@example.com\n\
            To: bob@example.com\n\
            Subject: Hello, Bob\n\
            Chat-Version: 1.0\n\
            Message-ID: <2893@example.com>\n\
            Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
            \n\
            hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();
        let new_msg = incoming_msg_to_reply_msg(
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: bob@example.com\n\
                 To: alice@example.com\n\
                 Subject: message opened\n\
                 Date: Sun, 22 Mar 2020 23:37:57 +0000\n\
                 Chat-Version: 1.0\n\
                 Message-ID: <Mr.12345678902@example.com>\n\
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
                 Reporting-UA: Delta Chat 1.28.0\n\
                 Original-Recipient: rfc822;bob@example.com\n\
                 Final-Recipient: rfc822;bob@example.com\n\
                 Original-Message-ID: <2893@example.com>\n\
                 Disposition: manual-action/MDN-sent-automatically; displayed\n\
                 \n", &t).await;
        let mf = MimeFactory::from_msg(&t, &new_msg, false).await.unwrap();
        // The subject string should not be "Re: message opened"
        assert_eq!("Re: Hello, Bob", mf.subject_str(&t).await.unwrap());
    }

    #[async_std::test]
    async fn test_subject_in_group() {
        async fn send_msg_get_subject(
            t: &TestContext,
            group_id: ChatId,
            quote: Option<&Message>,
        ) -> String {
            let mut new_msg = Message::new(Viewtype::Text);
            new_msg.set_text(Some("Hi".to_string()));
            if let Some(q) = quote {
                new_msg.set_quote(t, q).await.unwrap();
            }
            let sent = t.send_msg(group_id, &mut new_msg).await;
            get_subject(t, sent).await
        }
        async fn get_subject(t: &TestContext, sent: crate::test_utils::SentMessage) -> String {
            let parsed_subject = t.parse_msg(&sent).await.get_subject().unwrap();

            let sent_msg = Message::load_from_db(t, sent.sender_msg_id).await.unwrap();
            assert_eq!(parsed_subject, sent_msg.subject);

            parsed_subject
        }

        // 6. Test that in a group, replies also take the quoted message's subject, while non-replies use the group title as subject
        let t = TestContext::new_alice().await;
        let group_id =
            chat::create_group_chat(&t, chat::ProtectionStatus::Unprotected, "groupname") // TODO encodings, ä
                .await
                .unwrap();
        let bob = Contact::create(&t, "", "bob@example.org").await.unwrap();
        chat::add_contact_to_chat(&t, group_id, bob).await;

        let subject = send_msg_get_subject(&t, group_id, None).await;
        assert_eq!(subject, "groupname");

        let subject = send_msg_get_subject(&t, group_id, None).await;
        assert_eq!(subject, "Re: groupname");

        dc_receive_imf(
            &t,
            format!(
                "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: bob@example.com\n\
                To: alice@example.com\n\
                Subject: Different subject\n\
                In-Reply-To: {}\n\
                Message-ID: <2893@example.com>\n\
                Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                \n\
                hello\n",
                t.get_last_msg().await.rfc724_mid
            )
            .as_bytes(),
            "INBOX",
            5,
            false,
        )
        .await
        .unwrap();
        let message_from_bob = t.get_last_msg().await;

        let subject = send_msg_get_subject(&t, group_id, None).await;
        assert_eq!(subject, "Re: groupname");

        let subject = send_msg_get_subject(&t, group_id, Some(&message_from_bob)).await;
        let outgoing_quoting_msg = t.get_last_msg().await;
        assert_eq!(subject, "Re: Different subject");

        let subject = send_msg_get_subject(&t, group_id, None).await;
        assert_eq!(subject, "Re: groupname");

        let subject = send_msg_get_subject(&t, group_id, Some(&outgoing_quoting_msg)).await;
        assert_eq!(subject, "Re: Different subject");

        chat::forward_msgs(&t, &[message_from_bob.id], group_id)
            .await
            .unwrap();
        let subject = get_subject(&t, t.pop_sent_msg().await).await;
        assert_eq!(subject, "Fwd: Different subject");
    }

    async fn first_subject_str(t: TestContext) -> String {
        let contact_id =
            Contact::add_or_lookup(&t, "Dave", "dave@example.com", Origin::ManuallyCreated)
                .await
                .unwrap()
                .0;

        let chat_id = chat::create_by_contact_id(&t, contact_id).await.unwrap();

        let mut new_msg = Message::new(Viewtype::Text);
        new_msg.set_text(Some("Hi".to_string()));
        new_msg.chat_id = chat_id;
        chat::prepare_msg(&t, chat_id, &mut new_msg).await.unwrap();

        let mf = MimeFactory::from_msg(&t, &new_msg, false).await.unwrap();

        mf.subject_str(&t).await.unwrap()
    }

    // In `imf_raw`, From has to be bob@example.com, To has to be alice@example.com
    async fn msg_to_subject_str(imf_raw: &[u8]) -> String {
        let subject_str = msg_to_subject_str_inner(imf_raw, false, false, false).await;

        // Check that combinations of true and false reproduce the same subject_str:
        assert_eq!(
            subject_str,
            msg_to_subject_str_inner(imf_raw, true, false, false).await
        );
        assert_eq!(
            subject_str,
            msg_to_subject_str_inner(imf_raw, false, true, false).await
        );
        assert_eq!(
            subject_str,
            msg_to_subject_str_inner(imf_raw, false, true, true).await
        );
        assert_eq!(
            subject_str,
            msg_to_subject_str_inner(imf_raw, true, true, false).await
        );

        // These two combinations are different: If `message_arrives_inbetween` is true, but
        // `reply` is false, the core is actually expected to use the subject of the message
        // that arrived inbetween.
        assert_eq!(
            "Re: Some other, completely unrelated subject",
            msg_to_subject_str_inner(imf_raw, false, false, true).await
        );
        assert_eq!(
            "Re: Some other, completely unrelated subject",
            msg_to_subject_str_inner(imf_raw, true, false, true).await
        );

        // We leave away the combination (true, true, true) here:
        // It would mean that the original message is quoted without sending the quoting message
        // out yet, then the original message is deleted, then another unrelated message arrives
        // and then the message with the quote is sent out. Not very realistic.

        subject_str
    }

    async fn msg_to_subject_str_inner(
        imf_raw: &[u8],
        delete_original_msg: bool,
        reply: bool,
        message_arrives_inbetween: bool,
    ) -> String {
        let t = TestContext::new_alice().await;
        let mut new_msg = incoming_msg_to_reply_msg(imf_raw, &t).await;
        let incoming_msg = get_chat_msg(&t, new_msg.chat_id, 0, 2).await;

        if delete_original_msg {
            incoming_msg.id.delete_from_db(&t).await.unwrap();
        }

        if message_arrives_inbetween {
            dc_receive_imf(
                &t,
                b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    From: Bob <bob@example.com>\n\
                    To: alice@example.com\n\
                    Subject: Some other, completely unrelated subject\n\
                    Message-ID: <3cl4@example.com>\n\
                    Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                    \n\
                    Some other, completely unrelated content\n",
                "INBOX",
                2,
                false,
            )
            .await
            .unwrap();

            let arrived_msg = t.get_last_msg().await;
            assert_eq!(arrived_msg.chat_id, incoming_msg.chat_id);
        }

        if reply {
            new_msg.set_quote(&t, &incoming_msg).await.unwrap();
        }

        let mf = MimeFactory::from_msg(&t, &new_msg, false).await.unwrap();
        mf.subject_str(&t).await.unwrap()
    }

    // Creates a `Message` that replies "Hi" to the incoming email in `imf_raw`.
    async fn incoming_msg_to_reply_msg(imf_raw: &[u8], context: &Context) -> Message {
        context
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();

        dc_receive_imf(context, imf_raw, "INBOX", 1, false)
            .await
            .unwrap();

        let chats = Chatlist::try_load(context, 0, None, None).await.unwrap();

        let chat_id = chat::create_by_msg_id(context, chats.get_msg_id(0).unwrap())
            .await
            .unwrap();

        let mut new_msg = Message::new(Viewtype::Text);
        new_msg.set_text(Some("Hi".to_string()));
        new_msg.chat_id = chat_id;
        chat::prepare_msg(context, chat_id, &mut new_msg)
            .await
            .unwrap();

        new_msg
    }

    #[async_std::test]
    // This test could still be extended
    async fn test_render_reply() {
        let t = TestContext::new_alice().await;
        let context = &t;

        let msg = incoming_msg_to_reply_msg(
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: Charlie <charlie@example.com>\n\
                To: alice@example.com\n\
                Subject: Chat: hello\n\
                Chat-Version: 1.0\n\
                Message-ID: <2223@example.com>\n\
                Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                \n\
                hello\n",
            context,
        )
        .await;

        let mimefactory = MimeFactory::from_msg(&t, &msg, false).await.unwrap();

        let recipients = mimefactory.recipients();
        assert_eq!(recipients, vec!["charlie@example.com"]);

        let rendered_msg = mimefactory.render(context).await.unwrap();

        let mail = mailparse::parse_mail(&rendered_msg.message).unwrap();
        assert_eq!(
            mail.headers
                .iter()
                .find(|h| h.get_key() == "MIME-Version")
                .unwrap()
                .get_value(),
            "1.0"
        );

        let _mime_msg = MimeMessage::from_bytes(context, &rendered_msg.message)
            .await
            .unwrap();
    }

    #[test]
    fn test_no_empty_lines_in_header() {
        // See https://github.com/deltachat/deltachat-core-rust/issues/2118
        let to_tuples = [
            ("Nnnn", "nnn@ttttttttt.de"),
            ("😀 ttttttt", "ttttttt@rrrrrr.net"),
            ("dididididididi", "t@iiiiiii.org"),
            ("Ttttttt", "oooooooooo@abcd.de"),
            ("Mmmmm", "mmmmm@rrrrrr.net"),
            ("Zzzzzz", "rrrrrrrrrrrrr@ttttttttt.net"),
            ("Xyz", "qqqqqqqqqq@rrrrrr.net"),
            ("", "geug@ttttttttt.de"),
            ("qqqqqq", "q@iiiiiii.org"),
            ("bbbb", "bbbb@iiiiiii.org"),
            ("", "fsfs@iiiiiii.org"),
            ("rqrqrqrqr", "rqrqr@iiiiiii.org"),
            ("tttttttt", "tttttttt@iiiiiii.org"),
            ("", "tttttt@rrrrrr.net"),
        ]
        .iter();
        let to: Vec<_> = to_tuples
            .map(|(name, addr)| {
                if name.is_empty() {
                    Address::new_mailbox(addr.to_string())
                } else {
                    Address::new_mailbox_with_name(name.to_string(), addr.to_string())
                }
            })
            .collect();

        let mut message = email::MimeMessage::new_blank_message();
        message.headers.insert(
            (
                "Content-Type".to_string(),
                "text/plain; charset=utf-8; format=flowed; delsp=no".to_string(),
            )
                .into(),
        );
        message
            .headers
            .insert(Header::new_with_value("To".into(), to).unwrap());
        message.body = "Hi".to_string();

        let msg = message.as_string();

        let header_end = msg.find("Hi").unwrap();
        #[allow(clippy::indexing_slicing)]
        let headers = msg[0..header_end].trim();

        assert!(!headers.lines().any(|l| l.trim().is_empty()));
    }

    #[async_std::test]
    async fn test_selfavatar_unencrypted() -> anyhow::Result<()> {
        // create chat with bob, set selfavatar
        let t = TestContext::new_alice().await;
        let chat = t.create_chat_with_contact("bob", "bob@example.org").await;

        let file = t.dir.path().join("avatar.png");
        let bytes = include_bytes!("../test-data/image/avatar64x64.png");
        File::create(&file).await?.write_all(bytes).await?;
        t.set_config(Config::Selfavatar, Some(file.to_str().unwrap()))
            .await?;

        // send message to bob: that should get multipart/mixed because of the avatar moved to inner header;
        // make sure, `Subject:` stays in the outer header (imf header)
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("this is the text!".to_string()));

        let payload = t.send_msg(chat.id, &mut msg).await.payload();
        let mut payload = payload.splitn(3, "\r\n\r\n");
        let outer = payload.next().unwrap();
        let inner = payload.next().unwrap();
        let body = payload.next().unwrap();

        assert_eq!(outer.match_indices("multipart/mixed").count(), 1);
        assert_eq!(outer.match_indices("Subject:").count(), 1);
        assert_eq!(outer.match_indices("Autocrypt:").count(), 1);
        assert_eq!(outer.match_indices("Chat-User-Avatar:").count(), 0);

        assert_eq!(inner.match_indices("text/plain").count(), 1);
        assert_eq!(inner.match_indices("Chat-User-Avatar:").count(), 1);
        assert_eq!(inner.match_indices("Subject:").count(), 0);

        assert_eq!(body.match_indices("this is the text!").count(), 1);

        // if another message is sent, that one must not contain the avatar
        // and no artificial multipart/mixed nesting
        let payload = t.send_msg(chat.id, &mut msg).await.payload();
        let mut payload = payload.splitn(2, "\r\n\r\n");
        let outer = payload.next().unwrap();
        let body = payload.next().unwrap();

        assert_eq!(outer.match_indices("text/plain").count(), 1);
        assert_eq!(outer.match_indices("Subject:").count(), 1);
        assert_eq!(outer.match_indices("Autocrypt:").count(), 1);
        assert_eq!(outer.match_indices("multipart/mixed").count(), 0);
        assert_eq!(outer.match_indices("Chat-User-Avatar:").count(), 0);

        assert_eq!(body.match_indices("this is the text!").count(), 1);
        assert_eq!(body.match_indices("text/plain").count(), 0);
        assert_eq!(body.match_indices("Chat-User-Avatar:").count(), 0);
        assert_eq!(body.match_indices("Subject:").count(), 0);

        Ok(())
    }
}

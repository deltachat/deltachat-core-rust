//! # MIME message production.

use std::convert::TryInto;

use anyhow::{bail, ensure, Context as _, Result};
use base64::Engine as _;
use chrono::TimeZone;
use format_flowed::{format_flowed, format_flowed_quote};
use lettre_email::{mime, Address, Header, MimeMultipartType, PartBuilder};
use tokio::fs;

use crate::blob::BlobObject;
use crate::chat::Chat;
use crate::config::Config;
use crate::constants::{Chattype, DC_FROM_HANDSHAKE};
use crate::contact::Contact;
use crate::context::{get_version_str, Context};
use crate::e2ee::EncryptHelper;
use crate::ephemeral::Timer as EphemeralTimer;
use crate::html::new_html_mimepart;
use crate::location;
use crate::message::{self, Message, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::peerstate::{Peerstate, PeerstateVerifiedStatus};
use crate::simplify::escape_message_footer_marks;
use crate::stock_str;
use crate::tools::IsNoneOrEmpty;
use crate::tools::{
    create_outgoing_rfc724_mid, create_smeared_timestamp, remove_subject_prefix, time,
};

// attachments of 25 mb brutto should work on the majority of providers
// (brutto examples: web.de=50, 1&1=40, t-online.de=32, gmail=25, posteo=50, yahoo=25, all-inkl=100).
// to get the netto sizes, we subtract 1 mb header-overhead and the base64-overhead.
pub const RECOMMENDED_FILE_SIZE: u64 = 24 * 1024 * 1024 / 4 * 3;

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

    /// If the created mime-structure contains sync-items,
    /// the IDs of these items are listed here.
    /// The IDs are returned via `RenderedEmail`
    /// and must be deleted if the message is actually queued for sending.
    sync_ids_to_delete: Option<String>,

    /// True if the avatar should be attached.
    attach_selfavatar: bool,
}

/// Result of rendering a message, ready to be submitted to a send job.
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub message: String,
    // pub envelope: Envelope,
    pub is_encrypted: bool,
    pub is_gossiped: bool,
    pub last_added_location_id: u32,

    /// A comma-separated string of sync-IDs that are used by the rendered email
    /// and must be deleted once the message is actually queued for sending
    /// (deletion must be done by `delete_sync_ids()`).
    /// If the rendered email is not queued for sending, the IDs must not be deleted.
    pub sync_ids_to_delete: Option<String>,

    /// Message ID (Message in the sense of Email)
    pub rfc724_mid: String,

    /// Message subject.
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

        let from_addr = context.get_primary_self_addr().await?;
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
        } else if chat.is_mailing_list() {
            let list_post = chat
                .param
                .get(Param::ListPost)
                .context("Can't write to mailinglist without ListPost param")?;
            recipients.push(("".to_string(), list_post.to_string()));
        } else {
            context
                .sql
                .query_map(
                    "SELECT c.authname, c.addr  \
                 FROM chats_contacts cc  \
                 LEFT JOIN contacts c ON cc.contact_id=c.id  \
                 WHERE cc.chat_id=? AND cc.contact_id>9;",
                    paramsv![msg.chat_id],
                    |row| {
                        let authname: String = row.get(0)?;
                        let addr: String = row.get(1)?;
                        Ok((authname, addr))
                    },
                    |rows| {
                        for row in rows {
                            let (authname, addr) = row?;
                            if !recipients_contain_addr(&recipients, &addr) {
                                recipients.push((authname, addr));
                            }
                        }
                        Ok(())
                    },
                )
                .await?;

            if !msg.is_system_message()
                && msg.param.get_int(Param::Reaction).unwrap_or_default() == 0
                && context.get_config_bool(Config::MdnsEnabled).await?
            {
                req_mdn = true;
            }
        }
        let (in_reply_to, references) = context
            .sql
            .query_row(
                "SELECT mime_in_reply_to, mime_references FROM msgs WHERE id=?",
                paramsv![msg.id],
                |row| {
                    let in_reply_to: String = row.get(0)?;
                    let references: String = row.get(1)?;

                    Ok((
                        render_rfc724_mid_list(&in_reply_to),
                        render_rfc724_mid_list(&references),
                    ))
                },
            )
            .await?;

        let factory = MimeFactory {
            from_addr,
            from_displayname,
            sender_displayname,
            selfstatus: context
                .get_config(Config::Selfstatus)
                .await?
                .unwrap_or_default(),
            recipients,
            timestamp: msg.timestamp_sort,
            loaded: Loaded::Message { chat },
            msg,
            in_reply_to,
            references,
            req_mdn,
            last_added_location_id: 0,
            sync_ids_to_delete: None,
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
        let from_addr = context.get_primary_self_addr().await?;
        let from_displayname = context
            .get_config(Config::Displayname)
            .await?
            .unwrap_or_default();
        let selfstatus = context
            .get_config(Config::Selfstatus)
            .await?
            .unwrap_or_default();
        let timestamp = create_smeared_timestamp(context);

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
            sync_ids_to_delete: None,
            attach_selfavatar: false,
        };

        Ok(res)
    }

    async fn peerstates_for_recipients(
        &self,
        context: &Context,
    ) -> Result<Vec<(Option<Peerstate>, &str)>> {
        let self_addr = context.get_primary_self_addr().await?;

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
                } else if chat.typ == Chattype::Broadcast {
                    // encryption may disclose recipients;
                    // this is probably a worse issue than not opportunistically (!) encrypting
                    true
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
                let gossiped_timestamp = chat.id.get_gossiped_timestamp(context).await?;
                if time() > gossiped_timestamp + (2 * 24 * 60 * 60) {
                    Ok(true)
                } else {
                    let cmd = self.msg.param.get_cmd();
                    // Do gossip in all Securejoin messages not to complicate the code. There's no
                    // need in gossips in "vg-auth-required" messages f.e., but let them be.
                    Ok(cmd == SystemMessage::MemberAddedToGroup
                        || cmd == SystemMessage::SecurejoinMessage)
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
                if !self.msg.subject.is_empty() {
                    return Ok(self.msg.subject.clone());
                }

                if chat.typ == Chattype::Group && quoted_msg_subject.is_none_or_empty() {
                    let re = if self.in_reply_to.is_empty() {
                        ""
                    } else {
                        "Re: "
                    };
                    return Ok(format!("{}{}", re, chat.name));
                }

                if chat.typ != Chattype::Broadcast {
                    let parent_subject = if quoted_msg_subject.is_none_or_empty() {
                        chat.param.get(Param::LastSubject)
                    } else {
                        quoted_msg_subject.as_deref()
                    };
                    if let Some(last_subject) = parent_subject {
                        return Ok(format!("Re: {}", remove_subject_prefix(last_subject)));
                    }
                }

                let self_name = &match context.get_config(Config::Displayname).await? {
                    Some(name) => name,
                    None => context.get_config(Config::Addr).await?.unwrap_or_default(),
                };
                stock_str::subject_for_new_contact(context, self_name).await
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

    /// Consumes a `MimeFactory` and renders it into a message which is then stored in
    /// `smtp`-table to be used by the SMTP loop
    pub async fn render(mut self, context: &Context) -> Result<RenderedEmail> {
        let mut headers: MessageHeaders = Default::default();

        let from = Address::new_mailbox_with_name(
            self.from_displayname.to_string(),
            self.from_addr.clone(),
        );

        let undisclosed_recipients = match &self.loaded {
            Loaded::Message { chat } => chat.typ == Chattype::Broadcast,
            Loaded::Mdn { .. } => false,
        };

        let mut to = Vec::new();
        if undisclosed_recipients {
            to.push(Address::new_group(
                "hidden-recipients".to_string(),
                Vec::new(),
            ));
        } else {
            let email_to_remove =
                if self.msg.param.get_cmd() == SystemMessage::MemberRemovedFromGroup {
                    self.msg.param.get(Param::Arg)
                } else {
                    None
                };

            for (name, addr) in &self.recipients {
                if let Some(email_to_remove) = email_to_remove {
                    if email_to_remove == addr {
                        continue;
                    }
                }

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
        }

        // Start with Internet Message Format headers in the order of the standard example
        // <https://datatracker.ietf.org/doc/html/rfc5322#appendix-A.1.1>.
        let from_header = Header::new_with_value("From".into(), vec![from]).unwrap();
        headers.unprotected.push(from_header.clone());

        if let Some(sender_displayname) = &self.sender_displayname {
            let sender =
                Address::new_mailbox_with_name(sender_displayname.clone(), self.from_addr.clone());
            headers
                .unprotected
                .push(Header::new_with_value("Sender".into(), vec![sender]).unwrap());
        }
        headers
            .unprotected
            .push(Header::new_with_value("To".into(), to).unwrap());

        let subject_str = self.subject_str(context).await?;
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
        headers
            .protected
            .push(Header::new("Subject".into(), encoded_subject));

        let date = chrono::Utc
            .from_local_datetime(
                &chrono::NaiveDateTime::from_timestamp_opt(self.timestamp, 0)
                    .context("can't convert timestamp to NativeDateTime")?,
            )
            .unwrap()
            .to_rfc2822();
        headers.unprotected.push(Header::new("Date".into(), date));

        let rfc724_mid = match self.loaded {
            Loaded::Message { .. } => self.msg.rfc724_mid.clone(),
            Loaded::Mdn { .. } => create_outgoing_rfc724_mid(None, &self.from_addr),
        };
        let rfc724_mid_headervalue = render_rfc724_mid(&rfc724_mid);

        // Amazon's SMTP servers change the `Message-ID`, just as Outlook's SMTP servers do.
        // Outlook's servers add an `X-Microsoft-Original-Message-ID` header with the original `Message-ID`,
        // and when downloading messages we look for this header in order to correctly identify
        // messages.
        // Amazon's servers do not add such a header, so we just add it ourselves.
        if let Some(server) = context.get_config(Config::ConfiguredSendServer).await? {
            if server.ends_with(".amazonaws.com") {
                headers.unprotected.push(Header::new(
                    "X-Microsoft-Original-Message-ID".into(),
                    rfc724_mid_headervalue.clone(),
                ))
            }
        }

        headers
            .unprotected
            .push(Header::new("Message-ID".into(), rfc724_mid_headervalue));

        // Reply headers as in <https://datatracker.ietf.org/doc/html/rfc5322#appendix-A.2>.
        if !self.in_reply_to.is_empty() {
            headers
                .unprotected
                .push(Header::new("In-Reply-To".into(), self.in_reply_to.clone()));
        }
        if !self.references.is_empty() {
            headers
                .unprotected
                .push(Header::new("References".into(), self.references.clone()));
        }

        // Automatic Response headers <https://www.rfc-editor.org/rfc/rfc3834>
        if let Loaded::Mdn { .. } = self.loaded {
            headers.unprotected.push(Header::new(
                "Auto-Submitted".to_string(),
                "auto-replied".to_string(),
            ));
        } else if context.get_config_bool(Config::Bot).await? {
            headers.unprotected.push(Header::new(
                "Auto-Submitted".to_string(),
                "auto-generated".to_string(),
            ));
        }

        // Non-standard headers.
        headers
            .unprotected
            .push(Header::new("Chat-Version".to_string(), "1.0".to_string()));

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
        let e2ee_guaranteed = self.is_e2ee_guaranteed();
        let encrypt_helper = EncryptHelper::new(context).await?;

        if !skip_autocrypt {
            // unless determined otherwise we add the Autocrypt header
            let aheader = encrypt_helper.get_aheader().to_string();
            headers
                .unprotected
                .push(Header::new("Autocrypt".into(), aheader));
        }

        let ephemeral_timer = self.msg.chat_id.get_ephemeral_timer(context).await?;
        if let EphemeralTimer::Enabled { duration } = ephemeral_timer {
            headers.protected.push(Header::new(
                "Ephemeral-Timer".to_string(),
                duration.to_string(),
            ));
        }

        // MIME header <https://datatracker.ietf.org/doc/html/rfc2045>.
        // Content-Type
        headers
            .unprotected
            .push(Header::new("MIME-Version".into(), "1.0".into()));

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
            let part_holder = if self.msg.param.get_cmd() == SystemMessage::MultiDeviceSync {
                PartBuilder::new().header((
                    "Content-Type".to_string(),
                    "multipart/report; report-type=multi-device-sync".to_string(),
                ))
            } else if self.msg.param.get_cmd() == SystemMessage::WebxdcStatusUpdate {
                PartBuilder::new().header((
                    "Content-Type".to_string(),
                    "multipart/report; report-type=status-update".to_string(),
                ))
            } else {
                PartBuilder::new().message_type(MimeMultipartType::Mixed)
            };

            parts
                .into_iter()
                .fold(part_holder.child(main_part.build()), |message, part| {
                    message.child(part.build())
                })
        };

        let outer_message = if is_encrypted {
            headers.protected.push(from_header);

            // Store protected headers in the inner message.
            let message = headers
                .protected
                .into_iter()
                .fold(message, |message, header| message.header(header));

            // Add hidden headers to encrypted payload.
            let mut message = headers
                .hidden
                .into_iter()
                .fold(message, |message, header| message.header(header));

            // Add gossip headers in chats with multiple recipients
            if (peerstates.len() > 1 || context.get_config_bool(Config::BccSelf).await?)
                && self.should_do_gossip(context).await?
            {
                for peerstate in peerstates.iter().filter_map(|(state, _)| state.as_ref()) {
                    if let Some(header) = peerstate.render_gossip_header(min_verified) {
                        message = message.header(Header::new("Autocrypt-Gossip".into(), header));
                        is_gossiped = true;
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
                format!("{existing_ct} protected-headers=\"v1\";"),
            ));

            // Set the appropriate Content-Type for the outer message
            let outer_message = PartBuilder::new().header((
                "Content-Type".to_string(),
                "multipart/encrypted; protocol=\"application/pgp-encrypted\"".to_string(),
            ));

            if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
                info!(
                    context,
                    "mimefactory: unencrypted message mime-body:\n{}",
                    message.clone().build().as_string(),
                );
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
        } else {
            let message = if headers.hidden.is_empty() {
                message
            } else {
                // Store hidden headers in the inner unencrypted message.
                let message = headers
                    .hidden
                    .into_iter()
                    .fold(message, |message, header| message.header(header));

                PartBuilder::new()
                    .message_type(MimeMultipartType::Mixed)
                    .child(message.build())
            };

            // Store protected headers in the outer message.
            let message = headers
                .protected
                .into_iter()
                .fold(message, |message, header| message.header(header));

            if self.should_skip_autocrypt()
                || !context.get_config_bool(Config::SignUnencrypted).await?
            {
                message
            } else {
                let (payload, signature) = encrypt_helper.sign(context, message).await?;
                PartBuilder::new()
                    .header((
                        "Content-Type".to_string(),
                        "multipart/signed; protocol=\"application/pgp-signature\"".to_string(),
                    ))
                    .child(payload)
                    .child(
                        PartBuilder::new()
                            .content_type(
                                &"application/pgp-signature; name=\"signature.asc\""
                                    .parse::<mime::Mime>()
                                    .unwrap(),
                            )
                            .header(("Content-Description", "OpenPGP digital signature"))
                            .header(("Content-Disposition", "attachment; filename=\"signature\";"))
                            .body(signature)
                            .build(),
                    )
            }
        };

        // Store the unprotected headers on the outer message.
        let outer_message = headers
            .unprotected
            .into_iter()
            .fold(outer_message, |message, header| message.header(header));

        if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
            info!(
                context,
                "mimefactory: outgoing message mime-body:\n{}",
                outer_message.clone().build().as_string(),
            );
        }

        let MimeFactory {
            last_added_location_id,
            ..
        } = self;

        Ok(RenderedEmail {
            message: outer_message.build().as_string(),
            // envelope: Envelope::new,
            is_encrypted,
            is_gossiped,
            last_added_location_id,
            sync_ids_to_delete: self.sync_ids_to_delete,
            rfc724_mid,
            subject: subject_str,
        })
    }

    /// Returns MIME part with a `message.kml` attachment.
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

    /// Returns MIME part with a `location.kml` attachment.
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
            // Send group ID unless it is an ad hoc group that has no ID.
            if !chat.grpid.is_empty() {
                headers
                    .protected
                    .push(Header::new("Chat-Group-ID".into(), chat.grpid.clone()));
            }

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
                        // FIXME: Old clients require Secure-Join-Fingerprint header. Remove this
                        // eventually.
                        let fingerprint = Peerstate::from_addr(context, email_to_add)
                            .await?
                            .context("No peerstate found in db")?
                            .public_key_fingerprint
                            .context("No public key fingerprint in db for the member to add")?;
                        headers.protected.push(Header::new(
                            "Secure-Join-Fingerprint".into(),
                            fingerprint.hex(),
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
            SystemMessage::LocationOnly
            | SystemMessage::MultiDeviceSync
            | SystemMessage::WebxdcStatusUpdate => {
                // This should prevent automatic replies,
                // such as non-delivery reports.
                //
                // See <https://tools.ietf.org/html/rfc3834>
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

        let mut quoted_text = self
            .msg
            .quoted_text()
            .map(|quote| format_flowed_quote(&quote) + "\r\n\r\n");
        if quoted_text.is_none() && final_text.starts_with('>') {
            // Insert empty line to avoid receiver treating user-sent quote as topquote inserted by
            // Delta Chat.
            quoted_text = Some("\r\n".to_string());
        }
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

        if self.msg.param.get_int(Param::Reaction).unwrap_or_default() != 0 {
            main_part = main_part.header(("Content-Disposition", "reaction"));
        }

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
                    .child(new_html_mimepart(html).build());
            }
        }

        // add attachment part
        if self.msg.viewtype.has_file() {
            let (file_part, _) = build_body_file(context, self.msg, "").await?;
            parts.push(file_part);
        }

        if let Some(meta_part) = meta_part {
            parts.push(meta_part);
        }

        if let Some(msg_kml_part) = self.get_message_kml_part() {
            parts.push(msg_kml_part);
        }

        if location::is_sending_locations_to_chat(context, Some(self.msg.chat_id)).await? {
            match self.get_location_kml_part(context).await {
                Ok(part) => parts.push(part),
                Err(err) => {
                    warn!(context, "mimefactory: could not send location: {}", err);
                }
            }
        }

        // we do not piggyback sync-files to other self-sent-messages
        // to not risk files becoming too larger and being skipped by download-on-demand.
        if command == SystemMessage::MultiDeviceSync && self.is_e2ee_guaranteed() {
            let json = self.msg.param.get(Param::Arg).unwrap_or_default();
            let ids = self.msg.param.get(Param::Arg2).unwrap_or_default();
            parts.push(context.build_sync_part(json.to_string()));
            self.sync_ids_to_delete = Some(ids.to_string());
        } else if command == SystemMessage::WebxdcStatusUpdate {
            let json = self.msg.param.get(Param::Arg).unwrap_or_default();
            parts.push(context.build_status_update_part(json));
        } else if self.msg.viewtype == Viewtype::Webxdc {
            if let Some(json) = context
                .render_webxdc_status_update_object(self.msg.id, None)
                .await?
            {
                parts.push(context.build_status_update_part(&json));
            }
        }

        if self.attach_selfavatar {
            match context.get_config(Config::Selfavatar).await? {
                Some(path) => match build_selfavatar_file(context, &path).await {
                    Ok(avatar) => headers.hidden.push(Header::new(
                        "Chat-User-Avatar".into(),
                        format!("base64:{avatar}"),
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
            self.msg
                .get_summary(context, None)
                .await?
                .truncated_text(32)
                .to_string()
        };
        let p2 = stock_str::read_rcpt_mail_body(context, &p1).await;
        let message_text = format!("{}\r\n", format_flowed(&p2));
        message = message.child(
            PartBuilder::new()
                .header((
                    "Content-Type".to_string(),
                    "text/plain; charset=utf-8; format=flowed; delsp=no".to_string(),
                ))
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
    let base64 = base64::engine::general_purpose::STANDARD.encode(buf);
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
        .context("msg has no filename")?;
    let suffix = blob.suffix().unwrap_or("dat");

    // Get file name to use for sending.  For privacy purposes, we do
    // not transfer the original filenames eg. for images; these names
    // are normally not needed and contain timestamps, running numbers
    // etc.
    let filename_to_send: String = match msg.viewtype {
        Viewtype::Voice => format!(
            "voice-messsage_{}.{}",
            chrono::Utc
                .timestamp_opt(msg.timestamp_sort, 0)
                .single()
                .map_or_else(
                    || "YY-mm-dd_hh:mm:ss".to_string(),
                    |ts| ts.format("%Y-%m-%d_%H-%M-%S").to_string()
                ),
            &suffix
        ),
        Viewtype::Image | Viewtype::Gif => format!(
            "image_{}.{}",
            if base_name.is_empty() {
                chrono::Utc
                    .timestamp_opt(msg.timestamp_sort, 0)
                    .single()
                    .map_or_else(
                        || "YY-mm-dd_hh:mm:ss".to_string(),
                        |ts| ts.format("%Y-%m-%d_%H-%M-%S").to_string(),
                    )
            } else {
                base_name.to_string()
            },
            &suffix,
        ),
        Viewtype::Video => format!(
            "video_{}.{}",
            chrono::Utc
                .timestamp_opt(msg.timestamp_sort, 0)
                .single()
                .map_or_else(
                    || "YY-mm-dd_hh:mm:ss".to_string(),
                    |ts| ts.format("%Y-%m-%d_%H-%M-%S").to_string()
                ),
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

    let body = fs::read(blob.to_abs_path()).await?;
    let encoded_body = wrapped_base64_encode(&body);

    let mail = PartBuilder::new()
        .content_type(&mimetype)
        .header(("Content-Disposition", cd_value))
        .header(("Content-Transfer-Encoding", "base64"))
        .body(encoded_body);

    Ok((mail, filename_to_send))
}

async fn build_selfavatar_file(context: &Context, path: &str) -> Result<String> {
    let blob = BlobObject::from_path(context, path.as_ref())?;
    let body = fs::read(blob.to_abs_path()).await?;
    let encoded_body = wrapped_base64_encode(&body);
    Ok(encoded_body)
}

fn recipients_contain_addr(recipients: &[(String, String)], addr: &str) -> bool {
    let addr_lc = addr.to_lowercase();
    recipients
        .iter()
        .any(|(_, cur)| cur.to_lowercase() == addr_lc)
}

fn render_rfc724_mid(rfc724_mid: &str) -> String {
    let rfc724_mid = rfc724_mid.trim().to_string();

    if rfc724_mid.chars().next().unwrap_or_default() == '<' {
        rfc724_mid
    } else {
        format!("<{rfc724_mid}>")
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

fn needs_encoding(to_check: &str) -> bool {
    !to_check.chars().all(|c| {
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
    use mailparse::{addrparse_header, MailHeaderMap};

    use super::*;
    use crate::chat::ChatId;
    use crate::chat::{
        self, add_contact_to_chat, create_group_chat, remove_contact_from_chat, send_text_msg,
        ProtectionStatus,
    };
    use crate::chatlist::Chatlist;
    use crate::contact::{ContactAddress, Origin};
    use crate::mimeparser::MimeMessage;
    use crate::receive_imf::receive_imf;
    use crate::test_utils::{get_chat_msg, TestContext};
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

        println!("{s}");

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

        // Addresses should not be unnecessarily be encoded, see <https://github.com/deltachat/deltachat-core-rust/issues/1575>:
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manually_set_subject() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat = t.create_chat_with_contact("bob", "bob@example.org").await;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_subject("Subjeeeeect".to_string());

        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let payload = sent_msg.payload();

        assert_eq!(payload.match_indices("Subject: Subjeeeeect").count(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_subject_from_mua() {
        // 1.: Receive a mail from an MUA
        assert_eq!(
            msg_to_subject_str(
                b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: Bob <bob@example.com>\n\
                To: alice@example.org\n\
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
                To: alice@example.org\n\
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_subject_from_dc() {
        // 2. Receive a message from Delta Chat
        assert_eq!(
            msg_to_subject_str(
                b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: bob@example.com\n\
                To: alice@example.org\n\
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_subject_outgoing() {
        // 3. Send the first message to a new contact
        let t = TestContext::new_alice().await;

        assert_eq!(first_subject_str(t).await, "Message from alice@example.org");

        let t = TestContext::new_alice().await;
        t.set_config(Config::Displayname, Some("Alice"))
            .await
            .unwrap();
        assert_eq!(first_subject_str(t).await, "Message from Alice");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_subject_unicode() {
        // 4. Receive messages with unicode characters and make sure that we do not panic (we do not care about the result)
        msg_to_subject_str(
            "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
            From: bob@example.com\n\
            To: alice@example.org\n\
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
            To: alice@example.org\n\
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_subject_mdn() {
        // 5. Receive an mdn (read receipt) and make sure the mdn's subject is not used
        let t = TestContext::new_alice().await;
        receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
            From: alice@example.org\n\
            To: bob@example.com\n\
            Subject: Hello, Bob\n\
            Chat-Version: 1.0\n\
            Message-ID: <2893@example.com>\n\
            Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
            \n\
            hello\n",
            false,
        )
        .await
        .unwrap();
        let new_msg = incoming_msg_to_reply_msg(
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: bob@example.com\n\
                 To: alice@example.org\n\
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_subject_in_group() -> Result<()> {
        async fn send_msg_get_subject(
            t: &TestContext,
            group_id: ChatId,
            quote: Option<&Message>,
        ) -> Result<String> {
            let mut new_msg = Message::new(Viewtype::Text);
            new_msg.set_text(Some("Hi".to_string()));
            if let Some(q) = quote {
                new_msg.set_quote(t, Some(q)).await?;
            }
            let sent = t.send_msg(group_id, &mut new_msg).await;
            get_subject(t, sent).await
        }
        async fn get_subject(
            t: &TestContext,
            sent: crate::test_utils::SentMessage<'_>,
        ) -> Result<String> {
            let parsed_subject = t.parse_msg(&sent).await.get_subject().unwrap();

            let sent_msg = sent.load_from_db().await;
            assert_eq!(parsed_subject, sent_msg.subject);

            Ok(parsed_subject)
        }

        // 6. Test that in a group, replies also take the quoted message's subject, while non-replies use the group title as subject
        let t = TestContext::new_alice().await;
        let group_id =
            chat::create_group_chat(&t, chat::ProtectionStatus::Unprotected, "groupname") // TODO encodings, ä
                .await
                .unwrap();
        let bob = Contact::create(&t, "", "bob@example.org").await?;
        chat::add_contact_to_chat(&t, group_id, bob).await?;

        let subject = send_msg_get_subject(&t, group_id, None).await?;
        assert_eq!(subject, "groupname");

        let subject = send_msg_get_subject(&t, group_id, None).await?;
        assert_eq!(subject, "Re: groupname");

        receive_imf(
            &t,
            format!(
                "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: bob@example.com\n\
                To: alice@example.org\n\
                Subject: Different subject\n\
                In-Reply-To: {}\n\
                Message-ID: <2893@example.com>\n\
                Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                \n\
                hello\n",
                t.get_last_msg().await.rfc724_mid
            )
            .as_bytes(),
            false,
        )
        .await?;
        let message_from_bob = t.get_last_msg().await;

        let subject = send_msg_get_subject(&t, group_id, None).await?;
        assert_eq!(subject, "Re: groupname");

        let subject = send_msg_get_subject(&t, group_id, Some(&message_from_bob)).await?;
        let outgoing_quoting_msg = t.get_last_msg().await;
        assert_eq!(subject, "Re: Different subject");

        let subject = send_msg_get_subject(&t, group_id, None).await?;
        assert_eq!(subject, "Re: groupname");

        let subject = send_msg_get_subject(&t, group_id, Some(&outgoing_quoting_msg)).await?;
        assert_eq!(subject, "Re: Different subject");

        chat::forward_msgs(&t, &[message_from_bob.id], group_id).await?;
        let subject = get_subject(&t, t.pop_sent_msg().await).await?;
        assert_eq!(subject, "Re: groupname");
        Ok(())
    }

    async fn first_subject_str(t: TestContext) -> String {
        let contact_id = Contact::add_or_lookup(
            &t,
            "Dave",
            ContactAddress::new("dave@example.com").unwrap(),
            Origin::ManuallyCreated,
        )
        .await
        .unwrap()
        .0;

        let chat_id = ChatId::create_for_contact(&t, contact_id).await.unwrap();

        let mut new_msg = Message::new(Viewtype::Text);
        new_msg.set_text(Some("Hi".to_string()));
        new_msg.chat_id = chat_id;
        chat::prepare_msg(&t, chat_id, &mut new_msg).await.unwrap();

        let mf = MimeFactory::from_msg(&t, &new_msg, false).await.unwrap();

        mf.subject_str(&t).await.unwrap()
    }

    // In `imf_raw`, From has to be bob@example.com, To has to be alice@example.org
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
        // that arrived in between.
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
            receive_imf(
                &t,
                b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    From: Bob <bob@example.com>\n\
                    To: alice@example.org\n\
                    Subject: Some other, completely unrelated subject\n\
                    Message-ID: <3cl4@example.com>\n\
                    Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                    \n\
                    Some other, completely unrelated content\n",
                false,
            )
            .await
            .unwrap();

            let arrived_msg = t.get_last_msg().await;
            assert_eq!(arrived_msg.chat_id, incoming_msg.chat_id);
        }

        if reply {
            new_msg.set_quote(&t, Some(&incoming_msg)).await.unwrap();
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

        receive_imf(context, imf_raw, false).await.unwrap();

        let chats = Chatlist::try_load(context, 0, None, None).await.unwrap();

        let chat_id = chats.get_chat_id(0).unwrap();
        chat_id.accept(context).await.unwrap();

        let mut new_msg = Message::new(Viewtype::Text);
        new_msg.set_text(Some("Hi".to_string()));
        new_msg.chat_id = chat_id;
        chat::prepare_msg(context, chat_id, &mut new_msg)
            .await
            .unwrap();

        new_msg
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    // This test could still be extended
    async fn test_render_reply() {
        let t = TestContext::new_alice().await;
        let context = &t;

        let msg = incoming_msg_to_reply_msg(
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: Charlie <charlie@example.com>\n\
                To: alice@example.org\n\
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

        let mail = mailparse::parse_mail(rendered_msg.message.as_bytes()).unwrap();
        assert_eq!(
            mail.headers
                .iter()
                .find(|h| h.get_key() == "MIME-Version")
                .unwrap()
                .get_value(),
            "1.0"
        );

        let _mime_msg = MimeMessage::from_bytes(context, rendered_msg.message.as_bytes(), None)
            .await
            .unwrap();
    }

    #[test]
    fn test_no_empty_lines_in_header() {
        // See <https://github.com/deltachat/deltachat-core-rust/issues/2118>
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_selfavatar_unencrypted() -> anyhow::Result<()> {
        // create chat with bob, set selfavatar
        let t = TestContext::new_alice().await;
        let chat = t.create_chat_with_contact("bob", "bob@example.org").await;

        let file = t.dir.path().join("avatar.png");
        let bytes = include_bytes!("../test-data/image/avatar64x64.png");
        tokio::fs::write(&file, bytes).await?;
        t.set_config(Config::Selfavatar, Some(file.to_str().unwrap()))
            .await?;

        // send message to bob: that should get multipart/mixed because of the avatar moved to inner header;
        // make sure, `Subject:` stays in the outer header (imf header)
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("this is the text!".to_string()));

        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(3, "\r\n\r\n");

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
        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(2, "\r\n\r\n");
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_selfavatar_unencrypted_signed() {
        // create chat with bob, set selfavatar
        let t = TestContext::new_alice().await;
        t.set_config(Config::SignUnencrypted, Some("1"))
            .await
            .unwrap();
        let chat = t.create_chat_with_contact("bob", "bob@example.org").await;

        let file = t.dir.path().join("avatar.png");
        let bytes = include_bytes!("../test-data/image/avatar64x64.png");
        tokio::fs::write(&file, bytes).await.unwrap();
        t.set_config(Config::Selfavatar, Some(file.to_str().unwrap()))
            .await
            .unwrap();

        // send message to bob: that should get multipart/mixed because of the avatar moved to inner header;
        // make sure, `Subject:` stays in the outer header (imf header)
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("this is the text!".to_string()));

        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(4, "\r\n\r\n");

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("multipart/signed").count(), 1);
        assert_eq!(part.match_indices("Subject:").count(), 0);
        assert_eq!(part.match_indices("Autocrypt:").count(), 1);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("multipart/mixed").count(), 1);
        assert_eq!(part.match_indices("Subject:").count(), 1);
        assert_eq!(part.match_indices("Autocrypt:").count(), 0);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("text/plain").count(), 1);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 1);
        assert_eq!(part.match_indices("Subject:").count(), 0);

        let body = payload.next().unwrap();
        assert_eq!(body.match_indices("this is the text!").count(), 1);

        let bob = TestContext::new_bob().await;
        bob.recv_msg(&sent_msg).await;
        let alice_id = Contact::lookup_id_by_addr(&bob.ctx, "alice@example.org", Origin::Unknown)
            .await
            .unwrap()
            .unwrap();
        let alice_contact = Contact::load_from_db(&bob.ctx, alice_id).await.unwrap();
        assert!(alice_contact
            .get_profile_image(&bob.ctx)
            .await
            .unwrap()
            .is_some());

        // if another message is sent, that one must not contain the avatar
        // and no artificial multipart/mixed nesting
        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(3, "\r\n\r\n");

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("multipart/signed").count(), 1);
        assert_eq!(part.match_indices("Subject:").count(), 0);
        assert_eq!(part.match_indices("Autocrypt:").count(), 1);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("text/plain").count(), 1);
        assert_eq!(part.match_indices("Subject:").count(), 1);
        assert_eq!(part.match_indices("Autocrypt:").count(), 0);
        assert_eq!(part.match_indices("multipart/mixed").count(), 0);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let body = payload.next().unwrap();
        assert_eq!(body.match_indices("this is the text!").count(), 1);
        assert_eq!(body.match_indices("text/plain").count(), 0);
        assert_eq!(body.match_indices("Chat-User-Avatar:").count(), 0);
        assert_eq!(body.match_indices("Subject:").count(), 0);

        bob.recv_msg(&sent_msg).await;
        let alice_contact = Contact::load_from_db(&bob.ctx, alice_id).await.unwrap();
        assert!(alice_contact
            .get_profile_image(&bob.ctx)
            .await
            .unwrap()
            .is_some());
    }

    /// Test that removed member address does not go into the `To:` field.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_remove_member_bcc() -> Result<()> {
        // Alice creates a group with Bob and Claire and then removes Bob.
        let alice = TestContext::new_alice().await;

        let bob_id = Contact::create(&alice, "Bob", "bob@example.net").await?;
        let claire_id = Contact::create(&alice, "Claire", "claire@foo.de").await?;

        let alice_chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "foo").await?;
        add_contact_to_chat(&alice, alice_chat_id, bob_id).await?;
        add_contact_to_chat(&alice, alice_chat_id, claire_id).await?;
        send_text_msg(&alice, alice_chat_id, "Creating a group".to_string()).await?;

        remove_contact_from_chat(&alice, alice_chat_id, claire_id).await?;
        let remove = alice.pop_sent_msg().await;
        let remove_payload = remove.payload();
        let parsed = mailparse::parse_mail(remove_payload.as_bytes())?;
        let to = parsed
            .headers
            .get_first_header("To")
            .context("no To: header parsed")?;
        let to = addrparse_header(to)?;
        let mailbox = to
            .extract_single_info()
            .context("to: field does not contain exactly one address")?;
        assert_eq!(mailbox.addr, "bob@example.net");

        Ok(())
    }

    /// Tests that standard IMF header "From:" comes before non-standard "Autocrypt:" header.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_from_before_autocrypt() -> Result<()> {
        // create chat with bob
        let t = TestContext::new_alice().await;
        let chat = t.create_chat_with_contact("bob", "bob@example.org").await;

        // send message to bob: that should get multipart/mixed because of the avatar moved to inner header;
        // make sure, `Subject:` stays in the outer header (imf header)
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("this is the text!".to_string()));

        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let payload = sent_msg.payload();

        assert_eq!(payload.match_indices("Autocrypt:").count(), 1);
        assert_eq!(payload.match_indices("From:").count(), 1);

        assert!(payload.match_indices("From:").next() < payload.match_indices("Autocrypt:").next());

        Ok(())
    }
}

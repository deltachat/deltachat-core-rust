//! # MIME message production.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use base64::Engine as _;
use chrono::TimeZone;
use deltachat_contact_tools::sanitize_bidi_characters;
use mail_builder::headers::address::{Address, EmailAddress};
use mail_builder::headers::HeaderType;
use mail_builder::mime::MimePart;
use tokio::fs;

use crate::blob::BlobObject;
use crate::chat::{self, Chat};
use crate::config::Config;
use crate::constants::{Chattype, DC_FROM_HANDSHAKE};
use crate::contact::{Contact, ContactId, Origin};
use crate::context::Context;
use crate::e2ee::EncryptHelper;
use crate::ephemeral::Timer as EphemeralTimer;
use crate::location;
use crate::message::{self, Message, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::peer_channels::create_iroh_header;
use crate::peerstate::Peerstate;
use crate::simplify::escape_message_footer_marks;
use crate::stock_str;
use crate::tools::IsNoneOrEmpty;
use crate::tools::{
    create_outgoing_rfc724_mid, create_smeared_timestamp, remove_subject_prefix, time,
};
use crate::webxdc::StatusUpdateSerial;

// attachments of 25 mb brutto should work on the majority of providers
// (brutto examples: web.de=50, 1&1=40, t-online.de=32, gmail=25, posteo=50, yahoo=25, all-inkl=100).
// to get the netto sizes, we subtract 1 mb header-overhead and the base64-overhead.
pub const RECOMMENDED_FILE_SIZE: u64 = 24 * 1024 * 1024 / 4 * 3;

#[derive(Debug, Clone)]
pub enum Loaded {
    Message {
        chat: Chat,
        msg: Message,
    },
    Mdn {
        rfc724_mid: String,
        additional_msg_ids: Vec<String>,
    },
}

/// Helper to construct mime messages.
#[derive(Debug, Clone)]
pub struct MimeFactory {
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

    /// Vector of actual recipient addresses.
    ///
    /// This is the list of addresses the message should be sent to.
    /// It is not the same as the `To` header,
    /// because in case of "member removed" message
    /// removed member is in the recipient list,
    /// but not in the `To` header.
    /// In case of broadcast lists there are multiple recipients,
    /// but the `To` header has no members.
    ///
    /// If `bcc_self` configuration is enabled,
    /// this list will be extended with own address later,
    /// but `MimeFactory` is not responsible for this.
    recipients: Vec<String>,

    /// Vector of pairs of recipient name and address that goes into the `To` field.
    ///
    /// The list of actual message recipient addresses may be different,
    /// e.g. if members are hidden for broadcast lists.
    to: Vec<(String, String)>,

    /// Vector of pairs of past group member names and addresses.
    past_members: Vec<(String, String)>,

    /// Timestamps of the members in the same order as in the `recipients`
    /// followed by `past_members`.
    ///
    /// If this is not empty, its length
    /// should be the sum of `recipients` and `past_members` length.
    member_timestamps: Vec<i64>,

    timestamp: i64,
    loaded: Loaded,
    in_reply_to: String,

    /// Space-separated list of Message-IDs for `References` header.
    ///
    /// Each Message-ID in the list
    /// may or may not be enclosed in angle brackets,
    /// angle brackets must be added during message rendering
    /// as needed.
    references: String,

    /// True if the message requests Message Disposition Notification
    /// using `Chat-Disposition-Notification-To` header.
    req_mdn: bool,

    last_added_location_id: Option<u32>,

    /// If the created mime-structure contains sync-items,
    /// the IDs of these items are listed here.
    /// The IDs are returned via `RenderedEmail`
    /// and must be deleted if the message is actually queued for sending.
    sync_ids_to_delete: Option<String>,

    /// True if the avatar should be attached.
    pub attach_selfavatar: bool,
}

/// Result of rendering a message, ready to be submitted to a send job.
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub message: String,
    // pub envelope: Envelope,
    pub is_encrypted: bool,
    pub is_gossiped: bool,
    pub last_added_location_id: Option<u32>,

    /// A comma-separated string of sync-IDs that are used by the rendered email and must be deleted
    /// from `multi_device_sync` once the message is actually queued for sending.
    pub sync_ids_to_delete: Option<String>,

    /// Message ID (Message in the sense of Email)
    pub rfc724_mid: String,

    /// Message subject.
    pub subject: String,
}

fn new_address_with_name(name: &str, address: String) -> Address<'static> {
    Address::new_address(
        if name == address || name.is_empty() {
            None
        } else {
            Some(name.to_string())
        },
        address,
    )
}

impl MimeFactory {
    pub async fn from_msg(context: &Context, msg: Message) -> Result<MimeFactory> {
        let now = time();
        let chat = Chat::load_from_db(context, msg.chat_id).await?;
        let attach_profile_data = Self::should_attach_profile_data(&msg);
        let undisclosed_recipients = chat.typ == Chattype::Broadcast;

        let from_addr = context.get_primary_self_addr().await?;
        let config_displayname = context
            .get_config(Config::Displayname)
            .await?
            .unwrap_or_default();
        let (from_displayname, sender_displayname) =
            if let Some(override_name) = msg.param.get(Param::OverrideSenderDisplayname) {
                (override_name.to_string(), Some(config_displayname))
            } else {
                let name = match attach_profile_data {
                    true => config_displayname,
                    false => "".to_string(),
                };
                (name, None)
            };

        let mut recipients = Vec::new();
        let mut to = Vec::new();
        let mut past_members = Vec::new();
        let mut member_timestamps = Vec::new();
        let mut recipient_ids = HashSet::new();
        let mut req_mdn = false;

        if chat.is_self_talk() {
            if msg.param.get_cmd() == SystemMessage::AutocryptSetupMessage {
                recipients.push(from_addr.to_string());
            }
            to.push((from_displayname.to_string(), from_addr.to_string()));
        } else if chat.is_mailing_list() {
            let list_post = chat
                .param
                .get(Param::ListPost)
                .context("Can't write to mailinglist without ListPost param")?;
            to.push(("".to_string(), list_post.to_string()));
            recipients.push(list_post.to_string());
        } else {
            let email_to_remove = if msg.param.get_cmd() == SystemMessage::MemberRemovedFromGroup {
                msg.param.get(Param::Arg)
            } else {
                None
            };

            context
                .sql
                .query_map(
                    "SELECT c.authname, c.addr, c.id, cc.add_timestamp, cc.remove_timestamp
                     FROM chats_contacts cc
                     LEFT JOIN contacts c ON cc.contact_id=c.id
                     WHERE cc.chat_id=? AND (cc.contact_id>9 OR (cc.contact_id=1 AND ?))",
                    (msg.chat_id, chat.typ == Chattype::Group),
                    |row| {
                        let authname: String = row.get(0)?;
                        let addr: String = row.get(1)?;
                        let id: ContactId = row.get(2)?;
                        let add_timestamp: i64 = row.get(3)?;
                        let remove_timestamp: i64 = row.get(4)?;
                        Ok((authname, addr, id, add_timestamp, remove_timestamp))
                    },
                    |rows| {
                        let mut past_member_timestamps = Vec::new();

                        for row in rows {
                            let (authname, addr, id, add_timestamp, remove_timestamp) = row?;
                            let addr = if id == ContactId::SELF {
                                from_addr.to_string()
                            } else {
                                addr
                            };
                            let name = match attach_profile_data {
                                true => authname,
                                false => "".to_string(),
                            };
                            if add_timestamp >= remove_timestamp {
                                if !recipients_contain_addr(&to, &addr) {
                                    recipients.push(addr.clone());
                                    if !undisclosed_recipients {
                                        to.push((name, addr));
                                        member_timestamps.push(add_timestamp);
                                    }
                                }
                                recipient_ids.insert(id);
                            } else if remove_timestamp.saturating_add(60 * 24 * 3600) > now {
                                // Row is a tombstone,
                                // member is not actually part of the group.
                                if !recipients_contain_addr(&past_members, &addr) {
                                    if let Some(email_to_remove) = email_to_remove {
                                        if email_to_remove == addr {
                                            // This is a "member removed" message,
                                            // we need to notify removed member
                                            // that it was removed.
                                            recipients.push(addr.clone());
                                        }
                                    }
                                    if !undisclosed_recipients {
                                        past_members.push((name, addr));
                                        past_member_timestamps.push(remove_timestamp);
                                    }
                                }
                            }
                        }

                        debug_assert!(member_timestamps.len() >= to.len());

                        if to.len() > 1 {
                            if let Some(position) = to.iter().position(|(_, x)| x == &from_addr) {
                                to.remove(position);
                                member_timestamps.remove(position);
                            }
                        }

                        member_timestamps.extend(past_member_timestamps);
                        Ok(())
                    },
                )
                .await?;
            let recipient_ids: Vec<_> = recipient_ids.into_iter().collect();
            ContactId::scaleup_origin(context, &recipient_ids, Origin::OutgoingTo).await?;

            if !msg.is_system_message()
                && msg.param.get_int(Param::Reaction).unwrap_or_default() == 0
                && context.should_request_mdns().await?
            {
                req_mdn = true;
            }
        }
        let (in_reply_to, references) = context
            .sql
            .query_row(
                "SELECT mime_in_reply_to, IFNULL(mime_references, '')
                 FROM msgs WHERE id=?",
                (msg.id,),
                |row| {
                    let in_reply_to: String = row.get(0)?;
                    let references: String = row.get(1)?;

                    Ok((in_reply_to, render_rfc724_mid_list(&references)))
                },
            )
            .await?;
        let selfstatus = match attach_profile_data {
            true => context
                .get_config(Config::Selfstatus)
                .await?
                .unwrap_or_default(),
            false => "".to_string(),
        };
        let attach_selfavatar = Self::should_attach_selfavatar(context, &msg).await;

        debug_assert!(
            member_timestamps.is_empty()
                || to.len() + past_members.len() == member_timestamps.len()
        );
        let factory = MimeFactory {
            from_addr,
            from_displayname,
            sender_displayname,
            selfstatus,
            recipients,
            to,
            past_members,
            member_timestamps,
            timestamp: msg.timestamp_sort,
            loaded: Loaded::Message { msg, chat },
            in_reply_to,
            references,
            req_mdn,
            last_added_location_id: None,
            sync_ids_to_delete: None,
            attach_selfavatar,
        };
        Ok(factory)
    }

    pub async fn from_mdn(
        context: &Context,
        from_id: ContactId,
        rfc724_mid: String,
        additional_msg_ids: Vec<String>,
    ) -> Result<MimeFactory> {
        let contact = Contact::get_by_id(context, from_id).await?;
        let from_addr = context.get_primary_self_addr().await?;
        let timestamp = create_smeared_timestamp(context);

        let res = MimeFactory {
            from_addr,
            from_displayname: "".to_string(),
            sender_displayname: None,
            selfstatus: "".to_string(),
            recipients: vec![contact.get_addr().to_string()],
            to: vec![("".to_string(), contact.get_addr().to_string())],
            past_members: vec![],
            member_timestamps: vec![],
            timestamp,
            loaded: Loaded::Mdn {
                rfc724_mid,
                additional_msg_ids,
            },
            in_reply_to: String::default(),
            references: String::default(),
            req_mdn: false,
            last_added_location_id: None,
            sync_ids_to_delete: None,
            attach_selfavatar: false,
        };

        Ok(res)
    }

    async fn peerstates_for_recipients(
        &self,
        context: &Context,
    ) -> Result<Vec<(Option<Peerstate>, String)>> {
        let self_addr = context.get_primary_self_addr().await?;

        let mut res = Vec::new();
        for addr in self.recipients.iter().filter(|&addr| *addr != self_addr) {
            res.push((Peerstate::from_addr(context, addr).await?, addr.clone()));
        }

        Ok(res)
    }

    fn is_e2ee_guaranteed(&self) -> bool {
        match &self.loaded {
            Loaded::Message { chat, msg } => {
                !msg.param
                    .get_bool(Param::ForcePlaintext)
                    .unwrap_or_default()
                    && (chat.is_protected()
                        || msg.param.get_bool(Param::GuaranteeE2ee).unwrap_or_default())
            }
            Loaded::Mdn { .. } => false,
        }
    }

    fn verified(&self) -> bool {
        match &self.loaded {
            Loaded::Message { chat, msg } => {
                chat.is_self_talk() ||
                    // Securejoin messages are supposed to verify a key.
                    // In order to do this, it is necessary that they can be sent
                    // to a key that is not yet verified.
                    // This has to work independently of whether the chat is protected right now.
                    chat.is_protected() && msg.get_info_type() != SystemMessage::SecurejoinMessage
            }
            Loaded::Mdn { .. } => false,
        }
    }

    fn should_force_plaintext(&self) -> bool {
        match &self.loaded {
            Loaded::Message { chat, msg } => {
                msg.param
                    .get_bool(Param::ForcePlaintext)
                    .unwrap_or_default()
                    || chat.typ == Chattype::Broadcast
            }
            Loaded::Mdn { .. } => false,
        }
    }

    fn should_skip_autocrypt(&self) -> bool {
        match &self.loaded {
            Loaded::Message { msg, .. } => {
                msg.param.get_bool(Param::SkipAutocrypt).unwrap_or_default()
            }
            Loaded::Mdn { .. } => true,
        }
    }

    async fn should_do_gossip(&self, context: &Context, multiple_recipients: bool) -> Result<bool> {
        match &self.loaded {
            Loaded::Message { chat, msg } => {
                let cmd = msg.param.get_cmd();
                if cmd == SystemMessage::MemberAddedToGroup
                    || cmd == SystemMessage::SecurejoinMessage
                {
                    Ok(true)
                } else if multiple_recipients {
                    // beside key- and member-changes, force a periodic re-gossip.
                    let gossiped_timestamp = chat.id.get_gossiped_timestamp(context).await?;
                    let gossip_period = context.get_config_i64(Config::GossipPeriod).await?;
                    // `gossip_period == 0` is a special case for testing,
                    // enabling gossip in every message.
                    // Otherwise "smeared timestamps" may result in the condition
                    // to fail even if the clock is monotonic.
                    if gossip_period == 0 || time() >= gossiped_timestamp + gossip_period {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Loaded::Mdn { .. } => Ok(false),
        }
    }

    fn should_attach_profile_data(msg: &Message) -> bool {
        msg.param.get_cmd() != SystemMessage::SecurejoinMessage || {
            let step = msg.param.get(Param::Arg).unwrap_or_default();
            // Don't attach profile data at the earlier SecureJoin steps:
            // - The corresponding messages, i.e. "v{c,g}-request" and "v{c,g}-auth-required" are
            //   deleted right after processing, so other devices won't see the avatar etc.
            // - It's also good for privacy because the contact isn't yet verified and these
            //   messages are auto-sent unlike usual unencrypted messages.
            step == "vg-request-with-auth"
                || step == "vc-request-with-auth"
                || step == "vg-member-added"
                || step == "vc-contact-confirm"
        }
    }

    async fn should_attach_selfavatar(context: &Context, msg: &Message) -> bool {
        Self::should_attach_profile_data(msg)
            && match chat::shall_attach_selfavatar(context, msg.chat_id).await {
                Ok(should) => should,
                Err(err) => {
                    warn!(
                        context,
                        "should_attach_selfavatar: cannot get selfavatar state: {err:#}."
                    );
                    false
                }
            }
    }

    fn grpimage(&self) -> Option<String> {
        match &self.loaded {
            Loaded::Message { chat, msg } => {
                let cmd = msg.param.get_cmd();

                match cmd {
                    SystemMessage::MemberAddedToGroup => {
                        return chat.param.get(Param::ProfileImage).map(Into::into);
                    }
                    SystemMessage::GroupImageChanged => {
                        return msg.param.get(Param::Arg).map(Into::into)
                    }
                    _ => {}
                }

                if msg
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

    async fn subject_str(&self, context: &Context) -> Result<String> {
        let subject = match &self.loaded {
            Loaded::Message { ref chat, msg } => {
                let quoted_msg_subject = msg.quoted_message(context).await?.map(|m| m.subject);

                if !msg.subject.is_empty() {
                    return Ok(msg.subject.clone());
                }

                if (chat.typ == Chattype::Group || chat.typ == Chattype::Broadcast)
                    && quoted_msg_subject.is_none_or_empty()
                {
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
                    return Ok(format!("Re: {}", remove_subject_prefix(last_subject)));
                }

                let self_name = match Self::should_attach_profile_data(msg) {
                    true => context.get_config(Config::Displayname).await?,
                    false => None,
                };
                let self_name = &match self_name {
                    Some(name) => name,
                    None => context.get_config(Config::Addr).await?.unwrap_or_default(),
                };
                stock_str::subject_for_new_contact(context, self_name).await
            }
            Loaded::Mdn { .. } => "Receipt Notification".to_string(), // untranslated to no reveal sender's language
        };

        Ok(subject)
    }

    pub fn recipients(&self) -> Vec<String> {
        self.recipients.clone()
    }

    /// Consumes a `MimeFactory` and renders it into a message which is then stored in
    /// `smtp`-table to be used by the SMTP loop
    pub async fn render(mut self, context: &Context) -> Result<RenderedEmail> {
        let mut headers = Vec::<(String, HeaderType<'static>)>::new();

        let from = new_address_with_name(&self.from_displayname, self.from_addr.clone());

        let mut to: Vec<Address<'static>> = Vec::new();
        for (name, addr) in &self.to {
            to.push(Address::new_address(
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                },
                addr.clone(),
            ));
        }

        let mut past_members: Vec<Address<'static>> = Vec::new(); // Contents of `Chat-Group-Past-Members` header.
        for (name, addr) in &self.past_members {
            past_members.push(Address::new_address(
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                },
                addr.clone(),
            ));
        }

        debug_assert!(
            self.member_timestamps.is_empty()
                || to.len() + past_members.len() == self.member_timestamps.len()
        );
        if to.is_empty() {
            to.push(Address::new_group(
                Some("hidden-recipients".to_string()),
                Vec::new(),
            ));
        }

        // Start with Internet Message Format headers in the order of the standard example
        // <https://datatracker.ietf.org/doc/html/rfc5322#appendix-A.1.1>.
        headers.push(("From".to_string(), from.into()));

        if let Some(sender_displayname) = &self.sender_displayname {
            let sender = new_address_with_name(sender_displayname, self.from_addr.clone());
            headers.push(("Sender".to_string(), sender.into()));
        }
        headers.push((
            "To".to_string(),
            mail_builder::headers::address::Address::new_list(to.clone()).into(),
        ));
        if !past_members.is_empty() {
            headers.push((
                "Chat-Group-Past-Members".into(),
                mail_builder::headers::address::Address::new_list(past_members.clone()).into(),
            ));
        }

        if let Loaded::Message { chat, .. } = &self.loaded {
            if chat.typ == Chattype::Group
                && !self.member_timestamps.is_empty()
                && !chat.member_list_is_stale(context).await?
            {
                headers.push((
                    "Chat-Group-Member-Timestamps".into(),
                    mail_builder::headers::raw::Raw::new(
                        self.member_timestamps
                            .iter()
                            .map(|ts| ts.to_string())
                            .collect::<Vec<String>>()
                            .join(" "),
                    )
                    .into(),
                ));
            }
        }

        let subject_str = self.subject_str(context).await?;
        headers.push((
            "Subject".to_string(),
            mail_builder::headers::text::Text::new(subject_str.to_string()).into(),
        ));

        let date = chrono::DateTime::<chrono::Utc>::from_timestamp(self.timestamp, 0)
            .unwrap()
            .to_rfc2822();
        headers.push((
            "Date".to_string(),
            mail_builder::headers::raw::Raw::new(date).into(),
        ));

        let rfc724_mid = match &self.loaded {
            Loaded::Message { msg, .. } => msg.rfc724_mid.clone(),
            Loaded::Mdn { .. } => create_outgoing_rfc724_mid(),
        };
        headers.push((
            "Message-ID".to_string(),
            mail_builder::headers::message_id::MessageId::new(rfc724_mid.clone()).into(),
        ));

        // Reply headers as in <https://datatracker.ietf.org/doc/html/rfc5322#appendix-A.2>.
        if !self.in_reply_to.is_empty() {
            headers.push((
                "In-Reply-To".to_string(),
                mail_builder::headers::message_id::MessageId::new(self.in_reply_to.clone()).into(),
            ));
        }
        if !self.references.is_empty() {
            // TODO: use `mail_builder::headers::message_id::MessageId::new_list`
            headers.push((
                "References".to_string(),
                mail_builder::headers::raw::Raw::new(self.references.clone()).into(),
            ));
        }

        // Automatic Response headers <https://www.rfc-editor.org/rfc/rfc3834>
        if let Loaded::Mdn { .. } = self.loaded {
            headers.push((
                "Auto-Submitted".to_string(),
                mail_builder::headers::raw::Raw::new("auto-replied".to_string()).into(),
            ));
        } else if context.get_config_bool(Config::Bot).await? {
            headers.push((
                "Auto-Submitted".to_string(),
                mail_builder::headers::raw::Raw::new("auto-generated".to_string()).into(),
            ));
        } else if let Loaded::Message { msg, .. } = &self.loaded {
            if msg.param.get_cmd() == SystemMessage::SecurejoinMessage {
                let step = msg.param.get(Param::Arg).unwrap_or_default();
                if step != "vg-request" && step != "vc-request" {
                    headers.push((
                        "Auto-Submitted".to_string(),
                        mail_builder::headers::raw::Raw::new("auto-replied".to_string()).into(),
                    ));
                }
            }
        }

        if let Loaded::Message { chat, .. } = &self.loaded {
            if chat.typ == Chattype::Broadcast {
                let encoded_chat_name = encode_words(&chat.name);
                headers.push((
                    "List-ID".to_string(),
                    mail_builder::headers::raw::Raw::new(format!(
                        "{encoded_chat_name} <{}>",
                        chat.grpid
                    ))
                    .into(),
                ));
            }
        }

        // Non-standard headers.
        headers.push((
            "Chat-Version".to_string(),
            mail_builder::headers::raw::Raw::new("1.0").into(),
        ));

        if self.req_mdn {
            // we use "Chat-Disposition-Notification-To"
            // because replies to "Disposition-Notification-To" are weird in many cases
            // eg. are just freetext and/or do not follow any standard.
            headers.push((
                "Chat-Disposition-Notification-To".to_string(),
                mail_builder::headers::raw::Raw::new(self.from_addr.clone()).into(),
            ));
        }

        let verified = self.verified();
        let grpimage = self.grpimage();
        let skip_autocrypt = self.should_skip_autocrypt();
        let e2ee_guaranteed = self.is_e2ee_guaranteed();
        let encrypt_helper = EncryptHelper::new(context).await?;

        if !skip_autocrypt {
            // unless determined otherwise we add the Autocrypt header
            let aheader = encrypt_helper.get_aheader().to_string();
            headers.push((
                "Autocrypt".to_string(),
                mail_builder::headers::raw::Raw::new(aheader).into(),
            ));
        }

        // Add ephemeral timer for non-MDN messages.
        // For MDNs it does not matter because they are not visible
        // and ignored by the receiver.
        if let Loaded::Message { msg, .. } = &self.loaded {
            let ephemeral_timer = msg.chat_id.get_ephemeral_timer(context).await?;
            if let EphemeralTimer::Enabled { duration } = ephemeral_timer {
                headers.push((
                    "Ephemeral-Timer".to_string(),
                    mail_builder::headers::raw::Raw::new(duration.to_string()).into(),
                ));
            }
        }

        let mut is_gossiped = false;

        let peerstates = self.peerstates_for_recipients(context).await?;
        let is_encrypted = !self.should_force_plaintext()
            && encrypt_helper
                .should_encrypt(context, e2ee_guaranteed, &peerstates)
                .await?;
        let is_securejoin_message = if let Loaded::Message { msg, .. } = &self.loaded {
            msg.param.get_cmd() == SystemMessage::SecurejoinMessage
        } else {
            false
        };

        let message: MimePart<'static> = match &self.loaded {
            Loaded::Message { msg, .. } => {
                let msg = msg.clone();
                let (main_part, mut parts) = self
                    .render_message(context, &mut headers, &grpimage, is_encrypted)
                    .await?;
                if parts.is_empty() {
                    // Single part, render as regular message.
                    main_part
                } else {
                    parts.insert(0, main_part);

                    // Multiple parts, render as multipart.
                    if msg.param.get_cmd() == SystemMessage::MultiDeviceSync {
                        MimePart::new("multipart/report; report-type=multi-device-sync", parts)
                    } else if msg.param.get_cmd() == SystemMessage::WebxdcStatusUpdate {
                        MimePart::new("multipart/report; report-type=status-update", parts)
                    } else {
                        MimePart::new("multipart/mixed", parts)
                    }
                }
            }
            Loaded::Mdn { .. } => self.render_mdn()?,
        };

        // Split headers based on header confidentiality policy.

        // Headers that must go into IMF header section.
        //
        // These are standard headers such as Date, In-Reply-To, References, which cannot be placed
        // anywhere else according to the standard. Placing headers here also allows them to be fetched
        // individually over IMAP without downloading the message body. This is why Chat-Version is
        // placed here.
        let mut unprotected_headers: Vec<(String, HeaderType<'static>)> = Vec::new();

        // Headers that MUST NOT go into IMF header section.
        //
        // These are large headers which may hit the header section size limit on the server, such as
        // Chat-User-Avatar with a base64-encoded image inside. Also there are headers duplicated here
        // that servers mess up with in the IMF header section, like Message-ID.
        //
        // The header should be hidden from MTA
        // by moving it either into protected part
        // in case of encrypted mails
        // or unprotected MIME preamble in case of unencrypted mails.
        let mut hidden_headers: Vec<(String, HeaderType<'static>)> = Vec::new();

        // Opportunistically protected headers.
        //
        // These headers are placed into encrypted part *if* the message is encrypted. Place headers
        // which are not needed before decryption (e.g. Chat-Group-Name) or are not interesting if the
        // message cannot be decrypted (e.g. Chat-Disposition-Notification-To) here.
        //
        // If the message is not encrypted, these headers are placed into IMF header section, so make
        // sure that the message will be encrypted if you place any sensitive information here.
        let mut protected_headers: Vec<(String, HeaderType<'static>)> = Vec::new();

        // MIME header <https://datatracker.ietf.org/doc/html/rfc2045>.
        unprotected_headers.push((
            "MIME-Version".into(),
            mail_builder::headers::raw::Raw::new("1.0").into(),
        ));
        for header @ (original_header_name, _header_value) in &headers {
            let header_name = original_header_name.to_lowercase();
            if header_name == "message-id" {
                unprotected_headers.push(header.clone());
                hidden_headers.push(header.clone());
            } else if header_name == "chat-user-avatar" {
                hidden_headers.push(header.clone());
            } else if header_name == "autocrypt"
                && !context.get_config_bool(Config::ProtectAutocrypt).await?
            {
                unprotected_headers.push(header.clone());
            } else if header_name == "from" {
                // Unencrypted securejoin messages should _not_ include the display name:
                if is_encrypted || !is_securejoin_message {
                    protected_headers.push(header.clone());
                }

                unprotected_headers.push((
                    original_header_name.to_string(),
                    Address::new_address(None::<&'static str>, self.from_addr.clone()).into(),
                ));
            } else if header_name == "to" {
                protected_headers.push(header.clone());
                if is_encrypted {
                    unprotected_headers.push((
                        original_header_name.to_string(),
                        Address::new_list(
                            to.clone()
                                .into_iter()
                                .filter_map(|header| match header {
                                    Address::Address(mb) => Some(Address::Address(EmailAddress {
                                        name: None,
                                        email: mb.email,
                                    })),
                                    _ => None,
                                })
                                .collect::<Vec<_>>(),
                        )
                        .into(),
                    ));
                } else {
                    unprotected_headers.push(header.clone());
                }
            } else if is_encrypted {
                protected_headers.push(header.clone());

                match header_name.as_str() {
                    "subject" => {
                        unprotected_headers.push((
                            "Subject".to_string(),
                            mail_builder::headers::raw::Raw::new("[...]").into(),
                        ));
                    }
                    "date"
                    | "in-reply-to"
                    | "references"
                    | "auto-submitted"
                    | "chat-version"
                    | "autocrypt-setup-message" => {
                        unprotected_headers.push(header.clone());
                    }
                    _ => {
                        // Other headers are removed from unprotected part.
                    }
                }
            } else {
                // Copy the header to the protected headers
                // in case of signed-only message.
                // If the message is not signed, this value will not be used.
                protected_headers.push(header.clone());
                unprotected_headers.push(header.clone())
            }
        }

        let outer_message = if is_encrypted {
            // Store protected headers in the inner message.
            let message = protected_headers
                .into_iter()
                .fold(message, |message, (header, value)| {
                    message.header(header, value)
                });

            // Add hidden headers to encrypted payload.
            let mut message: MimePart<'static> = hidden_headers
                .into_iter()
                .fold(message, |message, (header, value)| {
                    message.header(header, value)
                });

            // Add gossip headers in chats with multiple recipients
            let multiple_recipients =
                peerstates.len() > 1 || context.get_config_bool(Config::BccSelf).await?;
            if self.should_do_gossip(context, multiple_recipients).await? {
                for peerstate in peerstates.iter().filter_map(|(state, _)| state.as_ref()) {
                    if let Some(header) = peerstate.render_gossip_header(verified) {
                        message = message.header(
                            "Autocrypt-Gossip",
                            mail_builder::headers::raw::Raw::new(header),
                        );
                        is_gossiped = true;
                    }
                }
            }

            // Set the appropriate Content-Type for the inner message.
            for (h, ref mut v) in &mut message.headers {
                if h == "Content-Type" {
                    if let mail_builder::headers::HeaderType::ContentType(ref mut ct) = v {
                        *ct = ct.clone().attribute("protected-headers", "v1");
                    }
                }
            }

            // Disable compression for SecureJoin to ensure
            // there are no compression side channels
            // leaking information about the tokens.
            let compress = match &self.loaded {
                Loaded::Message { msg, .. } => {
                    msg.param.get_cmd() != SystemMessage::SecurejoinMessage
                }
                Loaded::Mdn { .. } => true,
            };

            // XXX: additional newline is needed
            // to pass filtermail at
            // <https://github.com/deltachat/chatmail/blob/4d915f9800435bf13057d41af8d708abd34dbfa8/chatmaild/src/chatmaild/filtermail.py#L84-L86>
            let encrypted = encrypt_helper
                .encrypt(context, verified, message, peerstates, compress)
                .await?
                + "\n";

            // Set the appropriate Content-Type for the outer message
            MimePart::new(
                "multipart/encrypted; protocol=\"application/pgp-encrypted\"",
                vec![
                    // Autocrypt part 1
                    MimePart::new("application/pgp-encrypted", "Version: 1\r\n").header(
                        "Content-Description",
                        mail_builder::headers::raw::Raw::new("PGP/MIME version identification"),
                    ),
                    // Autocrypt part 2
                    MimePart::new(
                        "application/octet-stream; name=\"encrypted.asc\"",
                        encrypted,
                    )
                    .header(
                        "Content-Description",
                        mail_builder::headers::raw::Raw::new("OpenPGP encrypted message"),
                    )
                    .header(
                        "Content-Disposition",
                        mail_builder::headers::raw::Raw::new("inline; filename=\"encrypted.asc\";"),
                    ),
                ],
            )
        } else if matches!(self.loaded, Loaded::Mdn { .. }) {
            // Never add outer multipart/mixed wrapper to MDN
            // as multipart/report Content-Type is used to recognize MDNs
            // by Delta Chat receiver and Chatmail servers
            // allowing them to be unencrypted and not contain Autocrypt header
            // without resetting Autocrypt encryption or triggering Chatmail filter
            // that normally only allows encrypted mails.

            // Hidden headers are dropped.
            message
        } else {
            let message = hidden_headers
                .into_iter()
                .fold(message, |message, (header, value)| {
                    message.header(header, value)
                });
            let message = MimePart::new("multipart/mixed", vec![message]);
            let mut message = protected_headers
                .iter()
                .fold(message, |message, (header, value)| {
                    message.header(header.clone(), value.clone())
                });

            if skip_autocrypt || !context.get_config_bool(Config::SignUnencrypted).await? {
                // Deduplicate unprotected headers that also are in the protected headers:
                let protected: HashSet<&str> = HashSet::from_iter(
                    protected_headers
                        .iter()
                        .map(|(header, _value)| header.as_str()),
                );
                unprotected_headers.retain(|(header, _value)| !protected.contains(header.as_str()));

                message
            } else {
                for (h, ref mut v) in &mut message.headers {
                    if h == "Content-Type" {
                        if let mail_builder::headers::HeaderType::ContentType(ref mut ct) = v {
                            *ct = ct.clone().attribute("protected-headers", "v1");
                        }
                    }
                }

                let signature = encrypt_helper.sign(context, &message).await?;
                MimePart::new(
                    "multipart/signed; protocol=\"application/pgp-signature\"; protected",
                    vec![
                        message,
                        MimePart::new(
                            "application/pgp-signature; name=\"signature.asc\"",
                            signature,
                        )
                        .header(
                            "Content-Description",
                            mail_builder::headers::raw::Raw::<'static>::new(
                                "OpenPGP digital signature",
                            ),
                        )
                        .attachment("signature"),
                    ],
                )
            }
        };

        // Store the unprotected headers on the outer message.
        let outer_message = unprotected_headers
            .into_iter()
            .fold(outer_message, |message, (header, value)| {
                message.header(header, value)
            });

        let MimeFactory {
            last_added_location_id,
            ..
        } = self;

        let mut buffer = Vec::new();
        let cursor = Cursor::new(&mut buffer);
        outer_message.clone().write_part(cursor).ok();
        let message = String::from_utf8_lossy(&buffer).to_string();

        Ok(RenderedEmail {
            message,
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
    fn get_message_kml_part(&self) -> Option<MimePart<'static>> {
        let Loaded::Message { msg, .. } = &self.loaded else {
            return None;
        };

        let latitude = msg.param.get_float(Param::SetLatitude)?;
        let longitude = msg.param.get_float(Param::SetLongitude)?;

        let kml_file = location::get_message_kml(msg.timestamp_sort, latitude, longitude);
        let part = MimePart::new("application/vnd.google-earth.kml+xml", kml_file)
            .attachment("message.kml");
        Some(part)
    }

    /// Returns MIME part with a `location.kml` attachment.
    async fn get_location_kml_part(
        &mut self,
        context: &Context,
    ) -> Result<Option<MimePart<'static>>> {
        let Loaded::Message { msg, .. } = &self.loaded else {
            return Ok(None);
        };

        let Some((kml_content, last_added_location_id)) =
            location::get_kml(context, msg.chat_id).await?
        else {
            return Ok(None);
        };

        let part = MimePart::new("application/vnd.google-earth.kml+xml", kml_content)
            .attachment("location.kml");
        if !msg.param.exists(Param::SetLatitude) {
            // otherwise, the independent location is already filed
            self.last_added_location_id = Some(last_added_location_id);
        }
        Ok(Some(part))
    }

    async fn render_message(
        &mut self,
        context: &Context,
        headers: &mut Vec<(String, HeaderType<'static>)>,
        grpimage: &Option<String>,
        is_encrypted: bool,
    ) -> Result<(MimePart<'static>, Vec<MimePart<'static>>)> {
        let Loaded::Message { chat, msg } = &self.loaded else {
            bail!("Attempt to render MDN as a message");
        };
        let chat = chat.clone();
        let msg = msg.clone();
        let command = msg.param.get_cmd();
        let mut placeholdertext = None;

        let send_verified_headers = match chat.typ {
            Chattype::Single => true,
            Chattype::Group => true,
            // Mailinglists and broadcast lists can actually never be verified:
            Chattype::Mailinglist => false,
            Chattype::Broadcast => false,
        };
        if chat.is_protected() && send_verified_headers {
            headers.push((
                "Chat-Verified".to_string(),
                mail_builder::headers::raw::Raw::new("1").into(),
            ));
        }

        if chat.typ == Chattype::Group {
            // Send group ID unless it is an ad hoc group that has no ID.
            if !chat.grpid.is_empty() {
                headers.push((
                    "Chat-Group-ID".to_string(),
                    mail_builder::headers::raw::Raw::new(chat.grpid.clone()).into(),
                ));
            }

            headers.push((
                "Chat-Group-Name".to_string(),
                mail_builder::headers::text::Text::new(chat.name.to_string()).into(),
            ));

            match command {
                SystemMessage::MemberRemovedFromGroup => {
                    let email_to_remove = msg.param.get(Param::Arg).unwrap_or_default();

                    if email_to_remove
                        == context
                            .get_config(Config::ConfiguredAddr)
                            .await?
                            .unwrap_or_default()
                    {
                        placeholdertext = Some(stock_str::msg_group_left_remote(context).await);
                    } else {
                        placeholdertext =
                            Some(stock_str::msg_del_member_remote(context, email_to_remove).await);
                    };

                    if !email_to_remove.is_empty() {
                        headers.push((
                            "Chat-Group-Member-Removed".to_string(),
                            mail_builder::headers::raw::Raw::new(email_to_remove.to_string())
                                .into(),
                        ));
                    }
                }
                SystemMessage::MemberAddedToGroup => {
                    let email_to_add = msg.param.get(Param::Arg).unwrap_or_default();
                    placeholdertext =
                        Some(stock_str::msg_add_member_remote(context, email_to_add).await);

                    if !email_to_add.is_empty() {
                        headers.push((
                            "Chat-Group-Member-Added".to_string(),
                            mail_builder::headers::raw::Raw::new(email_to_add.to_string()).into(),
                        ));
                    }
                    if 0 != msg.param.get_int(Param::Arg2).unwrap_or_default() & DC_FROM_HANDSHAKE {
                        info!(
                            context,
                            "Sending secure-join message {:?}.", "vg-member-added",
                        );
                        headers.push((
                            "Secure-Join".to_string(),
                            mail_builder::headers::raw::Raw::new("vg-member-added".to_string())
                                .into(),
                        ));
                    }
                }
                SystemMessage::GroupNameChanged => {
                    let old_name = msg.param.get(Param::Arg).unwrap_or_default().to_string();
                    headers.push((
                        "Chat-Group-Name-Changed".to_string(),
                        mail_builder::headers::text::Text::new(old_name).into(),
                    ));
                }
                SystemMessage::GroupImageChanged => {
                    headers.push((
                        "Chat-Content".to_string(),
                        mail_builder::headers::text::Text::new("group-avatar-changed").into(),
                    ));
                    if grpimage.is_none() {
                        headers.push((
                            "Chat-Group-Avatar".to_string(),
                            mail_builder::headers::raw::Raw::new("0").into(),
                        ));
                    }
                }
                _ => {}
            }
        }

        match command {
            SystemMessage::LocationStreamingEnabled => {
                headers.push((
                    "Chat-Content".to_string(),
                    mail_builder::headers::raw::Raw::new("location-streaming-enabled").into(),
                ));
            }
            SystemMessage::EphemeralTimerChanged => {
                headers.push((
                    "Chat-Content".to_string(),
                    mail_builder::headers::raw::Raw::new("ephemeral-timer-changed").into(),
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
                headers.push((
                    "Auto-Submitted".to_string(),
                    mail_builder::headers::raw::Raw::new("auto-generated").into(),
                ));
            }
            SystemMessage::AutocryptSetupMessage => {
                headers.push((
                    "Autocrypt-Setup-Message".to_string(),
                    mail_builder::headers::raw::Raw::new("v1").into(),
                ));

                placeholdertext = Some(stock_str::ac_setup_msg_body(context).await);
            }
            SystemMessage::SecurejoinMessage => {
                let step = msg.param.get(Param::Arg).unwrap_or_default();
                if !step.is_empty() {
                    info!(context, "Sending secure-join message {step:?}.");
                    headers.push((
                        "Secure-Join".to_string(),
                        mail_builder::headers::raw::Raw::new(step.to_string()).into(),
                    ));

                    let param2 = msg.param.get(Param::Arg2).unwrap_or_default();
                    if !param2.is_empty() {
                        headers.push((
                            if step == "vg-request-with-auth" || step == "vc-request-with-auth" {
                                "Secure-Join-Auth".to_string()
                            } else {
                                "Secure-Join-Invitenumber".to_string()
                            },
                            mail_builder::headers::text::Text::new(param2.to_string()).into(),
                        ));
                    }

                    let fingerprint = msg.param.get(Param::Arg3).unwrap_or_default();
                    if !fingerprint.is_empty() {
                        headers.push((
                            "Secure-Join-Fingerprint".to_string(),
                            mail_builder::headers::raw::Raw::new(fingerprint.to_string()).into(),
                        ));
                    }
                    if let Some(id) = msg.param.get(Param::Arg4) {
                        headers.push((
                            "Secure-Join-Group".to_string(),
                            mail_builder::headers::raw::Raw::new(id.to_string()).into(),
                        ));
                    };
                }
            }
            SystemMessage::ChatProtectionEnabled => {
                headers.push((
                    "Chat-Content".to_string(),
                    mail_builder::headers::raw::Raw::new("protection-enabled").into(),
                ));
            }
            SystemMessage::ChatProtectionDisabled => {
                headers.push((
                    "Chat-Content".to_string(),
                    mail_builder::headers::raw::Raw::new("protection-disabled").into(),
                ));
            }
            SystemMessage::IrohNodeAddr => {
                headers.push((
                    "Iroh-Node-Addr".to_string(),
                    mail_builder::headers::text::Text::new(serde_json::to_string(
                        &context
                            .get_or_try_init_peer_channel()
                            .await?
                            .get_node_addr()
                            .await?,
                    )?)
                    .into(),
                ));
            }
            _ => {}
        }

        if let Some(grpimage) = grpimage {
            info!(context, "setting group image '{}'", grpimage);
            let avatar = build_avatar_file(context, grpimage)
                .await
                .context("Cannot attach group image")?;
            headers.push((
                "Chat-Group-Avatar".to_string(),
                mail_builder::headers::raw::Raw::new(format!("base64:{avatar}")).into(),
            ));
        }

        if msg.viewtype == Viewtype::Sticker {
            headers.push((
                "Chat-Content".to_string(),
                mail_builder::headers::raw::Raw::new("sticker").into(),
            ));
        } else if msg.viewtype == Viewtype::VideochatInvitation {
            headers.push((
                "Chat-Content".to_string(),
                mail_builder::headers::raw::Raw::new("videochat-invitation").into(),
            ));
            headers.push((
                "Chat-Webrtc-Room".to_string(),
                mail_builder::headers::raw::Raw::new(
                    msg.param
                        .get(Param::WebrtcRoom)
                        .unwrap_or_default()
                        .to_string(),
                )
                .into(),
            ));
        }

        if msg.viewtype == Viewtype::Voice
            || msg.viewtype == Viewtype::Audio
            || msg.viewtype == Viewtype::Video
        {
            if msg.viewtype == Viewtype::Voice {
                headers.push((
                    "Chat-Voice-Message".into(),
                    mail_builder::headers::raw::Raw::new("1").into(),
                ));
            }
            let duration_ms = msg.param.get_int(Param::Duration).unwrap_or_default();
            if duration_ms > 0 {
                let dur = duration_ms.to_string();
                headers.push((
                    "Chat-Duration".to_string(),
                    mail_builder::headers::raw::Raw::new(dur).into(),
                ));
            }
        }

        // add text part - we even add empty text and force a MIME-multipart-message as:
        // - some Apps have problems with Non-text in the main part (eg. "Mail" from stock Android)
        // - we can add "forward hints" this way
        // - it looks better

        let afwd_email = msg.param.exists(Param::Forwarded);
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

        let final_text = placeholdertext.as_deref().unwrap_or(&msg.text);

        let mut quoted_text = None;
        if let Some(msg_quoted_text) = msg.quoted_text() {
            let mut some_quoted_text = String::new();
            for quoted_line in msg_quoted_text.split('\n') {
                some_quoted_text += "> ";
                some_quoted_text += quoted_line;
                some_quoted_text += "\r\n";
            }
            some_quoted_text += "\r\n";
            quoted_text = Some(some_quoted_text)
        }

        if !is_encrypted && msg.param.get_bool(Param::ProtectQuote).unwrap_or_default() {
            // Message is not encrypted but quotes encrypted message.
            quoted_text = Some("> ...\r\n\r\n".to_string());
        }
        if quoted_text.is_none() && final_text.starts_with('>') {
            // Insert empty line to avoid receiver treating user-sent quote as topquote inserted by
            // Delta Chat.
            quoted_text = Some("\r\n".to_string());
        }

        let is_reaction = msg.param.get_int(Param::Reaction).unwrap_or_default() != 0;

        let footer = if is_reaction { "" } else { &self.selfstatus };

        let message_text = format!(
            "{}{}{}{}{}{}",
            fwdhint.unwrap_or_default(),
            quoted_text.unwrap_or_default(),
            escape_message_footer_marks(final_text),
            if !final_text.is_empty() && !footer.is_empty() {
                "\r\n\r\n"
            } else {
                ""
            },
            if !footer.is_empty() { "-- \r\n" } else { "" },
            footer
        );

        let mut main_part = MimePart::new("text/plain", message_text);
        if is_reaction {
            main_part = main_part.header(
                "Content-Disposition",
                mail_builder::headers::raw::Raw::new("reaction"),
            );
        }

        let mut parts = Vec::new();

        // add HTML-part, this is needed only if a HTML-message from a non-delta-client is forwarded;
        // for simplificity and to avoid conversion errors, we're generating the HTML-part from the original message.
        if msg.has_html() {
            let html = if let Some(orig_msg_id) = msg.param.get_int(Param::Forwarded) {
                MsgId::new(orig_msg_id.try_into()?)
                    .get_html(context)
                    .await?
            } else {
                msg.param.get(Param::SendHtml).map(|s| s.to_string())
            };
            if let Some(html) = html {
                main_part = MimePart::new(
                    "multipart/alternative",
                    vec![main_part, MimePart::new("text/html", html)],
                )
            }
        }

        // add attachment part
        if msg.viewtype.has_file() {
            let file_part = build_body_file(context, &msg).await?;
            parts.push(file_part);
        }

        if let Some(msg_kml_part) = self.get_message_kml_part() {
            parts.push(msg_kml_part);
        }

        if location::is_sending_locations_to_chat(context, Some(msg.chat_id)).await? {
            if let Some(part) = self.get_location_kml_part(context).await? {
                parts.push(part);
            }
        }

        // we do not piggyback sync-files to other self-sent-messages
        // to not risk files becoming too larger and being skipped by download-on-demand.
        if command == SystemMessage::MultiDeviceSync && self.is_e2ee_guaranteed() {
            let json = msg.param.get(Param::Arg).unwrap_or_default();
            let ids = msg.param.get(Param::Arg2).unwrap_or_default();
            parts.push(context.build_sync_part(json.to_string()));
            self.sync_ids_to_delete = Some(ids.to_string());
        } else if command == SystemMessage::WebxdcStatusUpdate {
            let json = msg.param.get(Param::Arg).unwrap_or_default();
            parts.push(context.build_status_update_part(json));
        } else if msg.viewtype == Viewtype::Webxdc {
            headers.push((
                "Iroh-Gossip-Topic".to_string(),
                mail_builder::headers::raw::Raw::new(create_iroh_header(context, msg.id).await?)
                    .into(),
            ));
            if let (Some(json), _) = context
                .render_webxdc_status_update_object(
                    msg.id,
                    StatusUpdateSerial::MIN,
                    StatusUpdateSerial::MAX,
                    None,
                )
                .await?
            {
                parts.push(context.build_status_update_part(&json));
            }
        }

        if self.attach_selfavatar {
            match context.get_config(Config::Selfavatar).await? {
                Some(path) => match build_avatar_file(context, &path).await {
                    Ok(avatar) => headers.push((
                        "Chat-User-Avatar".to_string(),
                        mail_builder::headers::raw::Raw::new(format!("base64:{avatar}")).into(),
                    )),
                    Err(err) => warn!(context, "mimefactory: cannot attach selfavatar: {}", err),
                },
                None => headers.push((
                    "Chat-User-Avatar".to_string(),
                    mail_builder::headers::raw::Raw::new("0").into(),
                )),
            }
        }

        Ok((main_part, parts))
    }

    /// Render an MDN
    fn render_mdn(&mut self) -> Result<MimePart<'static>> {
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

        let Loaded::Mdn {
            rfc724_mid,
            additional_msg_ids,
        } = &self.loaded
        else {
            bail!("Attempt to render a message as MDN");
        };

        // first body part: always human-readable, always REQUIRED by RFC 6522.
        // untranslated to no reveal sender's language.
        // moreover, translations in unknown languages are confusing, and clients may not display them at all
        let text_part = MimePart::new("text/plain", "This is a receipt notification.");

        let mut message = MimePart::new(
            "multipart/report; report-type=disposition-notification",
            vec![text_part],
        );

        // second body part: machine-readable, always REQUIRED by RFC 6522
        let message_text2 = format!(
            "Original-Recipient: rfc822;{}\r\n\
             Final-Recipient: rfc822;{}\r\n\
             Original-Message-ID: <{}>\r\n\
             Disposition: manual-action/MDN-sent-automatically; displayed\r\n",
            self.from_addr, self.from_addr, rfc724_mid
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

        message.add_part(MimePart::new(
            "message/disposition-notification",
            message_text2 + &extension_fields,
        ));

        Ok(message)
    }
}

async fn build_body_file(context: &Context, msg: &Message) -> Result<MimePart<'static>> {
    let file_name = msg.get_filename().context("msg has no file")?;
    let suffix = Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("dat");

    let blob = msg
        .param
        .get_blob(Param::File, context)
        .await?
        .context("msg has no file")?;

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
            chrono::Utc
                .timestamp_opt(msg.timestamp_sort, 0)
                .single()
                .map_or_else(
                    || "YY-mm-dd_hh:mm:ss".to_string(),
                    |ts| ts.format("%Y-%m-%d_%H-%M-%S").to_string(),
                ),
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
        _ => file_name,
    };

    /* check mimetype */
    let mimetype = match msg.param.get(Param::MimeType) {
        Some(mtype) => mtype.to_string(),
        None => {
            if let Some((_viewtype, res)) = message::guess_msgtype_from_suffix(msg) {
                res.to_string()
            } else {
                "application/octet-stream".to_string()
            }
        }
    };

    let body = fs::read(blob.to_abs_path()).await?;

    // create mime part, for Content-Disposition, see RFC 2183.
    // `Content-Disposition: attachment` seems not to make a difference to `Content-Disposition: inline`
    // at least on tested Thunderbird and Gma'l in 2017.
    // But I've heard about problems with inline and outl'k, so we just use the attachment-type until we
    // run into other problems ...
    let mail =
        MimePart::new(mimetype, body).attachment(sanitize_bidi_characters(&filename_to_send));

    Ok(mail)
}

async fn build_avatar_file(context: &Context, path: &str) -> Result<String> {
    let blob = match path.starts_with("$BLOBDIR/") {
        true => BlobObject::from_name(context, path.to_string())?,
        false => BlobObject::from_path(context, path.as_ref())?,
    };
    let body = fs::read(blob.to_abs_path()).await?;
    let encoded_body = base64::engine::general_purpose::STANDARD
        .encode(&body)
        .chars()
        .enumerate()
        .fold(String::new(), |mut res, (i, c)| {
            if i % 78 == 77 {
                res.push(' ')
            }
            res.push(c);
            res
        });
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

#[cfg(test)]
mod tests {
    use deltachat_contact_tools::ContactAddress;
    use mailparse::{addrparse_header, MailHeaderMap};
    use std::str;

    use super::*;
    use crate::chat::{
        add_contact_to_chat, create_group_chat, remove_contact_from_chat, send_text_msg, ChatId,
        ProtectionStatus,
    };
    use crate::chatlist::Chatlist;
    use crate::constants;
    use crate::contact::Origin;
    use crate::headerdef::HeaderDef;
    use crate::mimeparser::MimeMessage;
    use crate::receive_imf::receive_imf;
    use crate::test_utils::{get_chat_msg, TestContext, TestContextManager};

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
        let mut new_msg = incoming_msg_to_reply_msg(
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
        chat::send_msg(&t, new_msg.chat_id, &mut new_msg)
            .await
            .unwrap();
        let mf = MimeFactory::from_msg(&t, new_msg).await.unwrap();
        // The subject string should not be "Re: message opened"
        assert_eq!("Re: Hello, Bob", mf.subject_str(&t).await.unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mdn_create_encrypted() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        alice
            .set_config(Config::Displayname, Some("Alice Exampleorg"))
            .await?;
        let bob = tcm.bob().await;
        bob.set_config(Config::Displayname, Some("Bob Examplenet"))
            .await?;
        bob.set_config(Config::Selfstatus, Some("Bob Examplenet"))
            .await?;
        bob.set_config_bool(Config::MdnsEnabled, true).await?;

        let mut msg = Message::new(Viewtype::Text);
        msg.param.set_int(Param::SkipAutocrypt, 1);
        let chat_alice = alice.create_chat(&bob).await.id;
        let sent = alice.send_msg(chat_alice, &mut msg).await;

        let rcvd = bob.recv_msg(&sent).await;
        message::markseen_msgs(&bob, vec![rcvd.id]).await?;
        let mimefactory =
            MimeFactory::from_mdn(&bob, rcvd.from_id, rcvd.rfc724_mid.clone(), vec![]).await?;
        let rendered_msg = mimefactory.render(&bob).await?;

        assert!(!rendered_msg.is_encrypted);
        assert!(!rendered_msg.message.contains("Bob Examplenet"));
        assert!(!rendered_msg.message.contains("Alice Exampleorg"));
        let bob_alice_contact = bob.add_or_lookup_contact(&alice).await;
        assert_eq!(bob_alice_contact.get_authname(), "Alice Exampleorg");

        let rcvd = tcm.send_recv(&alice, &bob, "Heyho").await;
        message::markseen_msgs(&bob, vec![rcvd.id]).await?;

        let mimefactory =
            MimeFactory::from_mdn(&bob, rcvd.from_id, rcvd.rfc724_mid, vec![]).await?;
        let rendered_msg = mimefactory.render(&bob).await?;

        // When encrypted, the MDN should be encrypted as well
        assert!(rendered_msg.is_encrypted);
        assert!(!rendered_msg.message.contains("Bob Examplenet"));
        assert!(!rendered_msg.message.contains("Alice Exampleorg"));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_subject_in_group() -> Result<()> {
        async fn send_msg_get_subject(
            t: &TestContext,
            group_id: ChatId,
            quote: Option<&Message>,
        ) -> Result<String> {
            let mut new_msg = Message::new_text("Hi".to_string());
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
            &ContactAddress::new("dave@example.com").unwrap(),
            Origin::ManuallyCreated,
        )
        .await
        .unwrap()
        .0;

        let chat_id = ChatId::create_for_contact(&t, contact_id).await.unwrap();

        let mut new_msg = Message::new_text("Hi".to_string());
        new_msg.chat_id = chat_id;
        chat::send_msg(&t, chat_id, &mut new_msg).await.unwrap();

        let mf = MimeFactory::from_msg(&t, new_msg).await.unwrap();

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
        let incoming_msg = get_chat_msg(&t, new_msg.chat_id, 0, 1).await;

        if delete_original_msg {
            incoming_msg.id.trash(&t, false).await.unwrap();
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

        chat::send_msg(&t, new_msg.chat_id, &mut new_msg)
            .await
            .unwrap();
        let mf = MimeFactory::from_msg(&t, new_msg).await.unwrap();
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

        let mut new_msg = Message::new_text("Hi".to_string());
        new_msg.chat_id = chat_id;

        new_msg
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    // This test could still be extended
    async fn test_render_reply() {
        let t = TestContext::new_alice().await;
        let context = &t;

        let mut msg = incoming_msg_to_reply_msg(
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
        chat::send_msg(&t, msg.chat_id, &mut msg).await.unwrap();

        let mimefactory = MimeFactory::from_msg(&t, msg).await.unwrap();

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
        let mut msg = Message::new_text("this is the text!".to_string());

        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(3, "\r\n\r\n");

        let outer = payload.next().unwrap();
        let inner = payload.next().unwrap();
        let body = payload.next().unwrap();

        assert_eq!(outer.match_indices("multipart/mixed").count(), 1);
        assert_eq!(outer.match_indices("Message-ID:").count(), 1);
        assert_eq!(outer.match_indices("Subject:").count(), 1);
        assert_eq!(outer.match_indices("Autocrypt:").count(), 1);
        assert_eq!(outer.match_indices("Chat-User-Avatar:").count(), 0);

        assert_eq!(inner.match_indices("text/plain").count(), 1);
        assert_eq!(inner.match_indices("Message-ID:").count(), 1);
        assert_eq!(inner.match_indices("Chat-User-Avatar:").count(), 1);
        assert_eq!(inner.match_indices("Subject:").count(), 0);

        assert_eq!(body.match_indices("this is the text!").count(), 1);

        // if another message is sent, that one must not contain the avatar
        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(3, "\r\n\r\n");
        let outer = payload.next().unwrap();
        let inner = payload.next().unwrap();
        let body = payload.next().unwrap();

        assert_eq!(outer.match_indices("multipart/mixed").count(), 1);
        assert_eq!(outer.match_indices("Message-ID:").count(), 1);
        assert_eq!(outer.match_indices("Subject:").count(), 1);
        assert_eq!(outer.match_indices("Autocrypt:").count(), 1);
        assert_eq!(outer.match_indices("Chat-User-Avatar:").count(), 0);

        assert_eq!(inner.match_indices("text/plain").count(), 1);
        assert_eq!(inner.match_indices("Message-ID:").count(), 1);
        assert_eq!(inner.match_indices("Chat-User-Avatar:").count(), 0);
        assert_eq!(inner.match_indices("Subject:").count(), 0);

        assert_eq!(body.match_indices("this is the text!").count(), 1);

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

        // send message to bob: that should get multipart/signed.
        // `Subject:` is protected by copying it.
        // make sure, `Subject:` stays in the outer header (imf header)
        let mut msg = Message::new_text("this is the text!".to_string());

        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(4, "\r\n\r\n");

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("multipart/signed").count(), 1);
        assert_eq!(part.match_indices("From:").count(), 1);
        assert_eq!(part.match_indices("Message-ID:").count(), 1);
        assert_eq!(part.match_indices("Subject:").count(), 1);
        assert_eq!(part.match_indices("Autocrypt:").count(), 1);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let part = payload.next().unwrap();
        assert_eq!(
            part.match_indices("multipart/mixed; protected-headers=\"v1\"")
                .count(),
            1
        );
        assert_eq!(part.match_indices("From:").count(), 1);
        assert_eq!(part.match_indices("Message-ID:").count(), 0);
        assert_eq!(part.match_indices("Subject:").count(), 1);
        assert_eq!(part.match_indices("Autocrypt:").count(), 0);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("text/plain").count(), 1);
        assert_eq!(part.match_indices("From:").count(), 0);
        assert_eq!(part.match_indices("Message-ID:").count(), 1);
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
        let alice_contact = Contact::get_by_id(&bob.ctx, alice_id).await.unwrap();
        assert!(alice_contact
            .get_profile_image(&bob.ctx)
            .await
            .unwrap()
            .is_some());

        // if another message is sent, that one must not contain the avatar
        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let mut payload = sent_msg.payload().splitn(4, "\r\n\r\n");

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("multipart/signed").count(), 1);
        assert_eq!(part.match_indices("From:").count(), 1);
        assert_eq!(part.match_indices("Message-ID:").count(), 1);
        assert_eq!(part.match_indices("Subject:").count(), 1);
        assert_eq!(part.match_indices("Autocrypt:").count(), 1);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let part = payload.next().unwrap();
        assert_eq!(
            part.match_indices("multipart/mixed; protected-headers=\"v1\"")
                .count(),
            1
        );
        assert_eq!(part.match_indices("From:").count(), 1);
        assert_eq!(part.match_indices("Message-ID:").count(), 0);
        assert_eq!(part.match_indices("Subject:").count(), 1);
        assert_eq!(part.match_indices("Autocrypt:").count(), 0);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);

        let part = payload.next().unwrap();
        assert_eq!(part.match_indices("text/plain").count(), 1);
        assert_eq!(body.match_indices("From:").count(), 0);
        assert_eq!(part.match_indices("Message-ID:").count(), 1);
        assert_eq!(part.match_indices("Chat-User-Avatar:").count(), 0);
        assert_eq!(part.match_indices("Subject:").count(), 0);

        let body = payload.next().unwrap();
        assert_eq!(body.match_indices("this is the text!").count(), 1);

        bob.recv_msg(&sent_msg).await;
        let alice_contact = Contact::get_by_id(&bob.ctx, alice_id).await.unwrap();
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

        let claire_addr = "claire@foo.de";
        let bob_id = Contact::create(&alice, "Bob", "bob@example.net").await?;
        let claire_id = Contact::create(&alice, "Claire", claire_addr).await?;

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
        for to_addr in to.iter() {
            match to_addr {
                mailparse::MailAddr::Single(ref info) => {
                    // Addresses should be of existing members (Alice and Bob) and not Claire.
                    assert_ne!(info.addr, claire_addr);
                }
                mailparse::MailAddr::Group(_) => {
                    panic!("Group addresses are not expected here");
                }
            }
        }

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
        let mut msg = Message::new_text("this is the text!".to_string());

        let sent_msg = t.send_msg(chat.id, &mut msg).await;
        let payload = sent_msg.payload();

        assert_eq!(payload.match_indices("Autocrypt:").count(), 1);
        assert_eq!(payload.match_indices("From:").count(), 1);

        assert!(payload.match_indices("From:").next() < payload.match_indices("Autocrypt:").next());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_protected_headers_directive() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;
        let chat = tcm
            .send_recv_accept(&alice, &bob, "alice->bob")
            .await
            .chat_id;

        // Now Bob can send an encrypted message to Alice.
        let mut msg = Message::new(Viewtype::File);
        // Long messages are truncated and MimeMessage::decoded_data is set for them. We need
        // decoded_data to check presence of the necessary headers.
        msg.set_text("a".repeat(constants::DC_DESIRED_TEXT_LEN + 1));
        msg.set_file_from_bytes(&bob, "foo.bar", "content".as_bytes(), None)?;
        let sent = bob.send_msg(chat, &mut msg).await;
        assert!(msg.get_showpadlock());
        assert!(sent.payload.contains("\r\nSubject: [...]\r\n"));

        let mime = MimeMessage::from_bytes(&alice, sent.payload.as_bytes(), None).await?;
        let mut payload = str::from_utf8(&mime.decoded_data)?.splitn(2, "\r\n\r\n");
        let part = payload.next().unwrap();
        assert_eq!(
            part.match_indices("multipart/mixed; protected-headers=\"v1\"")
                .count(),
            1
        );
        assert_eq!(part.match_indices("Subject:").count(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_dont_remove_self() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let bob = &tcm.bob().await;

        let first_group = alice
            .create_group_with_members(ProtectionStatus::Unprotected, "First group", &[bob])
            .await;
        alice.send_text(first_group, "Hi! I created a group.").await;
        remove_contact_from_chat(alice, first_group, ContactId::SELF).await?;
        alice.pop_sent_msg().await;

        let second_group = alice
            .create_group_with_members(ProtectionStatus::Unprotected, "First group", &[bob])
            .await;
        let sent = alice
            .send_text(second_group, "Hi! I created another group.")
            .await;

        println!("{}", sent.payload);
        let mime_message = MimeMessage::from_bytes(alice, sent.payload.as_bytes(), None)
            .await
            .unwrap();
        assert_eq!(
            mime_message.get_header(HeaderDef::ChatGroupPastMembers),
            None
        );
        assert_eq!(
            mime_message.chat_group_member_timestamps().unwrap().len(),
            1 // There is a timestamp for Bob, not for Alice
        );

        Ok(())
    }
}

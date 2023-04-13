//! Internet Message Format reception pipeline.

use std::cmp::min;
use std::collections::HashSet;
use std::convert::TryFrom;

use anyhow::{bail, ensure, Context as _, Result};
use mailparse::{parse_mail, SingleInfo};
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::chat::{self, Chat, ChatId, ChatIdBlocked, ProtectionStatus};
use crate::config::Config;
use crate::constants::{Blocked, Chattype, ShowEmails, DC_CHAT_ID_TRASH};
use crate::contact::{
    may_be_valid_addr, normalize_name, Contact, ContactAddress, ContactId, Origin, VerifiedStatus,
};
use crate::context::Context;
use crate::debug_logging::maybe_set_logging_xdc_inner;
use crate::download::DownloadState;
use crate::ephemeral::{stock_ephemeral_timer_changed, Timer as EphemeralTimer};
use crate::events::EventType;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::imap::{markseen_on_imap_table, GENERATED_PREFIX};
use crate::location;
use crate::log::LogExt;
use crate::message::{
    self, rfc724_mid_exists, Message, MessageState, MessengerMessage, MsgId, Viewtype,
};
use crate::mimeparser::{
    parse_message_ids, AvatarAction, MailinglistType, MimeMessage, SystemMessage,
};
use crate::param::{Param, Params};
use crate::peerstate::{Peerstate, PeerstateKeyType, PeerstateVerifiedStatus};
use crate::reaction::{set_msg_reaction, Reaction};
use crate::securejoin::{self, handle_securejoin_handshake, observe_securejoin_on_other_device};
use crate::sql;
use crate::stock_str;
use crate::tools::{
    buf_compress, extract_grpid_from_rfc724_mid, smeared_time, strip_rtlo_characters,
};
use crate::{contact, imap};

/// This is the struct that is returned after receiving one email (aka MIME message).
///
/// One email with multiple attachments can end up as multiple chat messages, but they
/// all have the same chat_id, state and sort_timestamp.
#[derive(Debug)]
pub struct ReceivedMsg {
    /// Chat the message is assigned to.
    pub chat_id: ChatId,

    /// Received message state.
    pub state: MessageState,

    /// Message timestamp for sorting.
    pub sort_timestamp: i64,

    /// IDs of inserted rows in messages table.
    pub msg_ids: Vec<MsgId>,

    /// Whether IMAP messages should be immediately deleted.
    pub needs_delete_job: bool,
}

/// Emulates reception of a message from the network.
///
/// This method returns errors on a failure to parse the mail or extract Message-ID. It's only used
/// for tests and REPL tool, not actual message reception pipeline.
pub async fn receive_imf(
    context: &Context,
    imf_raw: &[u8],
    seen: bool,
) -> Result<Option<ReceivedMsg>> {
    let mail = parse_mail(imf_raw).context("can't parse mail")?;
    let rfc724_mid = imap::prefetch_get_or_create_message_id(&mail.headers);
    receive_imf_inner(context, &rfc724_mid, imf_raw, seen, None, false).await
}

/// Receive a message and add it to the database.
///
/// Returns an error on database failure or if the message is broken,
/// e.g. has nonstandard MIME structure.
///
/// If possible, creates a database entry to prevent the message from being
/// downloaded again, sets `chat_id=DC_CHAT_ID_TRASH` and returns `Ok(Some(…))`.
/// If the message is so wrong that we didn't even create a database entry,
/// returns `Ok(None)`.
///
/// If `is_partial_download` is set, it contains the full message size in bytes.
/// Do not confuse that with `replace_partial_download` that will be set when the full message is loaded later.
pub(crate) async fn receive_imf_inner(
    context: &Context,
    rfc724_mid: &str,
    imf_raw: &[u8],
    seen: bool,
    is_partial_download: Option<u32>,
    fetching_existing_messages: bool,
) -> Result<Option<ReceivedMsg>> {
    info!(context, "Receiving message, seen={seen}...");

    if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
        info!(
            context,
            "receive_imf: incoming message mime-body:\n{}",
            String::from_utf8_lossy(imf_raw),
        );
    }

    let mut mime_parser = match MimeMessage::from_bytes(context, imf_raw, is_partial_download).await
    {
        Err(err) => {
            warn!(context, "receive_imf: can't parse MIME: {err:#}.");
            let msg_ids;
            if !rfc724_mid.starts_with(GENERATED_PREFIX) {
                let row_id = context
                    .sql
                    .execute(
                        "INSERT INTO msgs(rfc724_mid, chat_id) VALUES (?,?)",
                        (rfc724_mid, DC_CHAT_ID_TRASH),
                    )
                    .await?;
                msg_ids = vec![MsgId::new(u32::try_from(row_id)?)];
            } else {
                return Ok(None);
                // We don't have an rfc724_mid, there's no point in adding a trash entry
            }

            return Ok(Some(ReceivedMsg {
                chat_id: DC_CHAT_ID_TRASH,
                state: MessageState::Undefined,
                sort_timestamp: 0,
                msg_ids,
                needs_delete_job: false,
            }));
        }
        Ok(mime_parser) => mime_parser,
    };

    // we can not add even an empty record if we have no info whatsoever
    if !mime_parser.has_headers() {
        warn!(context, "receive_imf: no headers found.");
        return Ok(None);
    }

    info!(context, "Received message has Message-Id: {rfc724_mid}");

    // check, if the mail is already in our database.
    // make sure, this check is done eg. before securejoin-processing.
    let replace_partial_download =
        if let Some(old_msg_id) = message::rfc724_mid_exists(context, rfc724_mid).await? {
            let msg = Message::load_from_db(context, old_msg_id).await?;
            if msg.download_state() != DownloadState::Done && is_partial_download.is_none() {
                // the message was partially downloaded before and is fully downloaded now.
                info!(
                    context,
                    "Message already partly in DB, replacing by full message."
                );
                Some(old_msg_id)
            } else {
                // the message was probably moved around.
                info!(context, "Message already in DB, doing nothing.");
                return Ok(None);
            }
        } else {
            None
        };

    let prevent_rename =
        mime_parser.is_mailinglist_message() || mime_parser.get_header(HeaderDef::Sender).is_some();

    // get From: (it can be an address list!) and check if it is known (for known From:'s we add
    // the other To:/Cc: in the 3rd pass)
    // or if From: is equal to SELF (in this case, it is any outgoing messages,
    // we do not check Return-Path any more as this is unreliable, see
    // <https://github.com/deltachat/deltachat-core/issues/150>)
    //
    // If this is a mailing list email (i.e. list_id_header is some), don't change the displayname because in
    // a mailing list the sender displayname sometimes does not belong to the sender email address.
    let (from_id, _from_id_blocked, incoming_origin) =
        match from_field_to_contact_id(context, &mime_parser.from, prevent_rename).await? {
            Some(contact_id_res) => contact_id_res,
            None => {
                warn!(
                    context,
                    "receive_imf: From field does not contain an acceptable address."
                );
                return Ok(None);
            }
        };

    let incoming = from_id != ContactId::SELF;

    let to_ids = add_or_lookup_contacts_by_address_list(
        context,
        &mime_parser.recipients,
        if !incoming {
            Origin::OutgoingTo
        } else if incoming_origin.is_known() {
            Origin::IncomingTo
        } else {
            Origin::IncomingUnknownTo
        },
    )
    .await?;

    let rcvd_timestamp = smeared_time(context);

    // Sender timestamp is allowed to be a bit in the future due to
    // unsynchronized clocks, but not too much.
    let sent_timestamp = mime_parser
        .get_header(HeaderDef::Date)
        .and_then(|value| mailparse::dateparse(value).ok())
        .map_or(rcvd_timestamp, |value| min(value, rcvd_timestamp + 60));

    // Add parts
    let received_msg = add_parts(
        context,
        &mut mime_parser,
        imf_raw,
        incoming,
        &to_ids,
        rfc724_mid,
        sent_timestamp,
        rcvd_timestamp,
        from_id,
        seen || replace_partial_download.is_some(),
        is_partial_download,
        replace_partial_download,
        fetching_existing_messages,
        prevent_rename,
    )
    .await
    .context("add_parts error")?;

    if !from_id.is_special() {
        contact::update_last_seen(context, from_id, sent_timestamp).await?;
    }

    // Update gossiped timestamp for the chat if someone else or our other device sent
    // Autocrypt-Gossip for all recipients in the chat to avoid sending Autocrypt-Gossip ourselves
    // and waste traffic.
    let chat_id = received_msg.chat_id;
    if !chat_id.is_special()
        && mime_parser
            .recipients
            .iter()
            .all(|recipient| mime_parser.gossiped_addr.contains(&recipient.addr))
    {
        info!(
            context,
            "Received message contains Autocrypt-Gossip for all members, updating timestamp."
        );
        if chat_id.get_gossiped_timestamp(context).await? < sent_timestamp {
            chat_id
                .set_gossiped_timestamp(context, sent_timestamp)
                .await?;
        }
    }

    let insert_msg_id = if let Some(msg_id) = received_msg.msg_ids.last() {
        *msg_id
    } else {
        MsgId::new_unset()
    };

    save_locations(context, &mime_parser, chat_id, from_id, insert_msg_id).await?;

    if let Some(ref sync_items) = mime_parser.sync_items {
        if from_id == ContactId::SELF {
            if mime_parser.was_encrypted() {
                if let Err(err) = context.execute_sync_items(sync_items).await {
                    warn!(context, "receive_imf cannot execute sync items: {err:#}.");
                }
            } else {
                warn!(context, "Sync items are not encrypted.");
            }
        } else {
            warn!(context, "Sync items not sent by self.");
        }
    }

    if let Some(ref status_update) = mime_parser.webxdc_status_update {
        if let Err(err) = context
            .receive_status_update(from_id, insert_msg_id, status_update)
            .await
        {
            warn!(context, "receive_imf cannot update status: {err:#}.");
        }
    }

    if let Some(avatar_action) = &mime_parser.user_avatar {
        if from_id != ContactId::UNDEFINED
            && context
                .update_contacts_timestamp(from_id, Param::AvatarTimestamp, sent_timestamp)
                .await?
        {
            if let Err(err) = contact::set_profile_image(
                context,
                from_id,
                avatar_action,
                mime_parser.was_encrypted(),
            )
            .await
            {
                warn!(context, "receive_imf cannot update profile image: {err:#}.");
            };
        }
    }

    // Always update the status, even if there is no footer, to allow removing the status.
    //
    // Ignore MDNs though, as they never contain the signature even if user has set it.
    // Ignore footers from mailinglists as they are often created or modified by the mailinglist software.
    if mime_parser.mdn_reports.is_empty()
        && !mime_parser.is_mailinglist_message()
        && is_partial_download.is_none()
        && from_id != ContactId::UNDEFINED
        && context
            .update_contacts_timestamp(from_id, Param::StatusTimestamp, sent_timestamp)
            .await?
    {
        if let Err(err) = contact::set_status(
            context,
            from_id,
            mime_parser.footer.clone().unwrap_or_default(),
            mime_parser.was_encrypted(),
            mime_parser.has_chat_version(),
        )
        .await
        {
            warn!(context, "Cannot update contact status: {err:#}.");
        }
    }

    // Get user-configured server deletion
    let delete_server_after = context.get_config_delete_server_after().await?;

    if !received_msg.msg_ids.is_empty() {
        if received_msg.needs_delete_job
            || (delete_server_after == Some(0) && is_partial_download.is_none())
        {
            let target = context.get_delete_msgs_target().await?;
            context
                .sql
                .execute(
                    "UPDATE imap SET target=? WHERE rfc724_mid=?",
                    (target, rfc724_mid),
                )
                .await?;
        } else if !mime_parser.mdn_reports.is_empty() && mime_parser.has_chat_version() {
            // This is a Delta Chat MDN. Mark as read.
            markseen_on_imap_table(context, rfc724_mid).await?;
        }
    }

    if replace_partial_download.is_some() {
        context.emit_msgs_changed(chat_id, MsgId::new(0));
    } else if !chat_id.is_trash() {
        let fresh = received_msg.state == MessageState::InFresh;
        for msg_id in &received_msg.msg_ids {
            chat_id.emit_msg_event(context, *msg_id, incoming && fresh);
        }
    }

    mime_parser
        .handle_reports(context, from_id, sent_timestamp, &mime_parser.parts)
        .await;

    Ok(Some(received_msg))
}

/// Converts "From" field to contact id.
///
/// Also returns whether it is blocked or not and its origin.
///
/// * `prevent_rename`: passed through to `add_or_lookup_contacts_by_address_list()`
///
/// Returns `None` if From field does not contain a valid contact address.
pub async fn from_field_to_contact_id(
    context: &Context,
    from: &SingleInfo,
    prevent_rename: bool,
) -> Result<Option<(ContactId, bool, Origin)>> {
    let display_name = if prevent_rename {
        Some("")
    } else {
        from.display_name.as_deref()
    };
    let from_addr = match ContactAddress::new(&from.addr) {
        Ok(from_addr) => from_addr,
        Err(err) => {
            warn!(
                context,
                "Cannot create a contact for the given From field: {err:#}."
            );
            return Ok(None);
        }
    };

    let from_id = add_or_lookup_contact_by_addr(
        context,
        display_name,
        from_addr,
        Origin::IncomingUnknownFrom,
    )
    .await?;

    if from_id == ContactId::SELF {
        Ok(Some((ContactId::SELF, false, Origin::OutgoingBcc)))
    } else {
        let mut from_id_blocked = false;
        let mut incoming_origin = Origin::Unknown;
        if let Ok(contact) = Contact::load_from_db(context, from_id).await {
            from_id_blocked = contact.blocked;
            incoming_origin = contact.origin;
        }
        Ok(Some((from_id, from_id_blocked, incoming_origin)))
    }
}

/// Creates a `ReceivedMsg` from given parts which might consist of
/// multiple messages (if there are multiple attachments).
/// Every entry in `mime_parser.parts` produces a new row in the `msgs` table.
#[allow(clippy::too_many_arguments, clippy::cognitive_complexity)]
async fn add_parts(
    context: &Context,
    mime_parser: &mut MimeMessage,
    imf_raw: &[u8],
    incoming: bool,
    to_ids: &[ContactId],
    rfc724_mid: &str,
    sent_timestamp: i64,
    rcvd_timestamp: i64,
    from_id: ContactId,
    seen: bool,
    is_partial_download: Option<u32>,
    mut replace_msg_id: Option<MsgId>,
    fetching_existing_messages: bool,
    prevent_rename: bool,
) -> Result<ReceivedMsg> {
    let mut chat_id = None;
    let mut chat_id_blocked = Blocked::Not;

    let mut better_msg = None;
    if mime_parser.is_system_message == SystemMessage::LocationStreamingEnabled {
        better_msg = Some(stock_str::msg_location_enabled_by(context, from_id).await);
    }

    let parent = get_parent_message(context, mime_parser).await?;

    let is_dc_message = if mime_parser.has_chat_version() {
        MessengerMessage::Yes
    } else if let Some(parent) = &parent {
        match parent.is_dc_message {
            MessengerMessage::No => MessengerMessage::No,
            MessengerMessage::Yes | MessengerMessage::Reply => MessengerMessage::Reply,
        }
    } else {
        MessengerMessage::No
    };
    // incoming non-chat messages may be discarded

    let is_location_kml = mime_parser.location_kml.is_some();
    let is_mdn = !mime_parser.mdn_reports.is_empty();
    let is_reaction = mime_parser.parts.iter().any(|part| part.is_reaction);
    let show_emails =
        ShowEmails::from_i32(context.get_config_int(Config::ShowEmails).await?).unwrap_or_default();

    let allow_creation;
    if mime_parser.is_system_message != SystemMessage::AutocryptSetupMessage
        && is_dc_message == MessengerMessage::No
    {
        // this message is a classic email not a chat-message nor a reply to one
        match show_emails {
            ShowEmails::Off => {
                info!(context, "Classical email not shown (TRASH).");
                chat_id = Some(DC_CHAT_ID_TRASH);
                allow_creation = false;
            }
            ShowEmails::AcceptedContacts => allow_creation = false,
            ShowEmails::All => allow_creation = !is_mdn,
        }
    } else {
        allow_creation = !is_mdn && !is_reaction;
    }

    // check if the message introduces a new chat:
    // - outgoing messages introduce a chat with the first to: address if they are sent by a messenger
    // - incoming messages introduce a chat only for known contacts if they are sent by a messenger
    // (of course, the user can add other chats manually later)
    let to_id: ContactId;

    let state: MessageState;
    let mut needs_delete_job = false;
    if incoming {
        to_id = ContactId::SELF;

        // Whether the message is a part of securejoin handshake that should be marked as seen
        // automatically.
        let securejoin_seen;

        // handshake may mark contacts as verified and must be processed before chats are created
        if mime_parser.get_header(HeaderDef::SecureJoin).is_some() {
            match handle_securejoin_handshake(context, mime_parser, from_id).await {
                Ok(securejoin::HandshakeMessage::Done) => {
                    chat_id = Some(DC_CHAT_ID_TRASH);
                    needs_delete_job = true;
                    securejoin_seen = true;
                }
                Ok(securejoin::HandshakeMessage::Ignore) => {
                    chat_id = Some(DC_CHAT_ID_TRASH);
                    securejoin_seen = true;
                }
                Ok(securejoin::HandshakeMessage::Propagate) => {
                    // process messages as "member added" normally
                    securejoin_seen = false;
                }
                Err(err) => {
                    warn!(context, "Error in Secure-Join message handling: {err:#}.");
                    chat_id = Some(DC_CHAT_ID_TRASH);
                    securejoin_seen = true;
                }
            }
        } else {
            securejoin_seen = false;
        }

        let test_normal_chat = if from_id == ContactId::UNDEFINED {
            None
        } else {
            ChatIdBlocked::lookup_by_contact(context, from_id).await?
        };

        if chat_id.is_none() && mime_parser.delivery_report.is_some() {
            chat_id = Some(DC_CHAT_ID_TRASH);
            info!(context, "Message is a DSN (TRASH).",);
            markseen_on_imap_table(context, rfc724_mid).await.ok();
        }

        if chat_id.is_none() {
            // try to assign to a chat based on In-Reply-To/References:

            if let Some((new_chat_id, new_chat_id_blocked)) =
                lookup_chat_by_reply(context, mime_parser, &parent, to_ids, from_id).await?
            {
                chat_id = Some(new_chat_id);
                chat_id_blocked = new_chat_id_blocked;
            }
        }

        // signals whether the current user is a bot
        let is_bot = context.get_config_bool(Config::Bot).await?;

        let create_blocked = match test_normal_chat {
            Some(ChatIdBlocked {
                id: _,
                blocked: Blocked::Request,
            }) if is_bot => Blocked::Not,
            Some(ChatIdBlocked { id: _, blocked }) => blocked,
            None => Blocked::Request,
        };

        if chat_id.is_none() {
            // try to create a group

            if let Some((new_chat_id, new_chat_id_blocked)) = create_or_lookup_group(
                context,
                mime_parser,
                if test_normal_chat.is_none() {
                    allow_creation
                } else {
                    true
                },
                create_blocked,
                from_id,
                to_ids,
            )
            .await?
            {
                chat_id = Some(new_chat_id);
                chat_id_blocked = new_chat_id_blocked;
            }
        }

        // if the chat is somehow blocked but we want to create a non-blocked chat,
        // unblock the chat
        if chat_id_blocked != Blocked::Not && create_blocked != Blocked::Yes {
            if let Some(chat_id) = chat_id {
                chat_id.set_blocked(context, create_blocked).await?;
                chat_id_blocked = create_blocked;
            }
        }

        // In lookup_chat_by_reply() and create_or_lookup_group(), it can happen that the message is put into a chat
        // but the From-address is not a member of this chat.
        if let Some(chat_id) = chat_id {
            if !chat::is_contact_in_chat(context, chat_id, from_id).await? {
                let chat = Chat::load_from_db(context, chat_id).await?;
                if chat.is_protected() {
                    let s = stock_str::unknown_sender_for_chat(context).await;
                    mime_parser.repl_msg_by_error(&s);
                } else {
                    // In non-protected chats, just mark the sender as overridden. Therefore, the UI will prepend `~`
                    // to the sender's name, indicating to the user that he/she is not part of the group.
                    let from = &mime_parser.from;
                    let name: &str = from.display_name.as_ref().unwrap_or(&from.addr);
                    for part in &mut mime_parser.parts {
                        part.param.set(Param::OverrideSenderDisplayname, name);
                    }
                }
            }

            better_msg = better_msg.or(apply_group_changes(
                context,
                mime_parser,
                sent_timestamp,
                chat_id,
                from_id,
                to_ids,
            )
            .await?);
        }

        if chat_id.is_none() {
            // check if the message belongs to a mailing list
            match mime_parser.get_mailinglist_type() {
                MailinglistType::ListIdBased => {
                    if let Some(list_id) = mime_parser.get_header(HeaderDef::ListId) {
                        if let Some((new_chat_id, new_chat_id_blocked)) =
                            create_or_lookup_mailinglist(
                                context,
                                allow_creation,
                                list_id,
                                mime_parser,
                            )
                            .await?
                        {
                            chat_id = Some(new_chat_id);
                            chat_id_blocked = new_chat_id_blocked;
                        }
                    }
                }
                MailinglistType::SenderBased => {
                    if let Some(sender) = mime_parser.get_header(HeaderDef::Sender) {
                        if let Some((new_chat_id, new_chat_id_blocked)) =
                            create_or_lookup_mailinglist(
                                context,
                                allow_creation,
                                sender,
                                mime_parser,
                            )
                            .await?
                        {
                            chat_id = Some(new_chat_id);
                            chat_id_blocked = new_chat_id_blocked;
                        }
                    }
                }
                MailinglistType::None => {}
            }
        }

        if let Some(chat_id) = chat_id {
            apply_mailinglist_changes(context, mime_parser, chat_id).await?;
        }

        // if contact renaming is prevented (for mailinglists and bots),
        // we use name from From:-header as override name
        if prevent_rename {
            if let Some(name) = &mime_parser.from.display_name {
                for part in &mut mime_parser.parts {
                    part.param.set(Param::OverrideSenderDisplayname, name);
                }
            }
        }

        if chat_id.is_none() {
            // try to create a normal chat
            let create_blocked = if from_id == ContactId::SELF {
                Blocked::Not
            } else {
                let contact = Contact::load_from_db(context, from_id).await?;
                match contact.is_blocked() {
                    true => Blocked::Yes,
                    false if is_bot => Blocked::Not,
                    false => Blocked::Request,
                }
            };

            if let Some(chat) = test_normal_chat {
                chat_id = Some(chat.id);
                chat_id_blocked = chat.blocked;
            } else if allow_creation {
                if let Ok(chat) = ChatIdBlocked::get_for_contact(context, from_id, create_blocked)
                    .await
                    .context("Failed to get (new) chat for contact")
                    .log_err(context)
                {
                    chat_id = Some(chat.id);
                    chat_id_blocked = chat.blocked;
                }
            }

            if let Some(chat_id) = chat_id {
                if chat_id_blocked != Blocked::Not {
                    if chat_id_blocked != create_blocked {
                        chat_id.set_blocked(context, create_blocked).await?;
                    }
                    if create_blocked == Blocked::Request && parent.is_some() {
                        // we do not want any chat to be created implicitly.  Because of the origin-scale-up,
                        // the contact requests will pop up and this should be just fine.
                        Contact::scaleup_origin_by_id(context, from_id, Origin::IncomingReplyTo)
                            .await?;
                        info!(
                            context,
                            "Message is a reply to a known message, mark sender as known.",
                        );
                    }
                }
            }
        }

        state = if seen
            || fetching_existing_messages
            || is_mdn
            || is_reaction
            || is_location_kml
            || securejoin_seen
            || chat_id_blocked == Blocked::Yes
        {
            MessageState::InSeen
        } else {
            MessageState::InFresh
        };
    } else {
        // Outgoing

        // the mail is on the IMAP server, probably it is also delivered.
        // We cannot recreate other states (read, error).
        state = MessageState::OutDelivered;
        to_id = to_ids.get(0).copied().unwrap_or_default();

        let self_sent =
            from_id == ContactId::SELF && to_ids.len() == 1 && to_ids.contains(&ContactId::SELF);

        // handshake may mark contacts as verified and must be processed before chats are created
        if mime_parser.get_header(HeaderDef::SecureJoin).is_some() {
            match observe_securejoin_on_other_device(context, mime_parser, to_id).await {
                Ok(securejoin::HandshakeMessage::Done)
                | Ok(securejoin::HandshakeMessage::Ignore) => {
                    chat_id = Some(DC_CHAT_ID_TRASH);
                }
                Ok(securejoin::HandshakeMessage::Propagate) => {
                    // process messages as "member added" normally
                    chat_id = None;
                }
                Err(err) => {
                    warn!(context, "Error in Secure-Join watching: {err:#}.");
                    chat_id = Some(DC_CHAT_ID_TRASH);
                }
            }
        } else if mime_parser.sync_items.is_some() && self_sent {
            chat_id = Some(DC_CHAT_ID_TRASH);
        }

        // Mozilla Thunderbird does not set \Draft flag on "Templates", but sets
        // X-Mozilla-Draft-Info header, which can be used to detect both drafts and templates
        // created by Thunderbird.
        let is_draft = mime_parser
            .get_header(HeaderDef::XMozillaDraftInfo)
            .is_some();

        if is_draft {
            // Most mailboxes have a "Drafts" folder where constantly new emails appear but we don't actually want to show them
            info!(context, "Email is probably just a draft (TRASH).");
            chat_id = Some(DC_CHAT_ID_TRASH);
        }

        if chat_id.is_none() {
            // try to assign to a chat based on In-Reply-To/References:

            if let Some((new_chat_id, new_chat_id_blocked)) =
                lookup_chat_by_reply(context, mime_parser, &parent, to_ids, from_id).await?
            {
                chat_id = Some(new_chat_id);
                chat_id_blocked = new_chat_id_blocked;
            }
        }

        if !to_ids.is_empty() {
            if chat_id.is_none() {
                if let Some((new_chat_id, new_chat_id_blocked)) = create_or_lookup_group(
                    context,
                    mime_parser,
                    allow_creation,
                    Blocked::Not,
                    from_id,
                    to_ids,
                )
                .await?
                {
                    chat_id = Some(new_chat_id);
                    chat_id_blocked = new_chat_id_blocked;
                }
            }
            if chat_id.is_none() && allow_creation {
                let to_contact = Contact::load_from_db(context, to_id).await?;
                if let Some(list_id) = to_contact.param.get(Param::ListId) {
                    if let Some((id, _, blocked)) =
                        chat::get_chat_id_by_grpid(context, list_id).await?
                    {
                        chat_id = Some(id);
                        chat_id_blocked = blocked;
                    }
                } else if let Ok(chat) =
                    ChatIdBlocked::get_for_contact(context, to_id, Blocked::Not).await
                {
                    chat_id = Some(chat.id);
                    chat_id_blocked = chat.blocked;
                }
            }

            // automatically unblock chat when the user sends a message
            if chat_id_blocked != Blocked::Not {
                if let Some(chat_id) = chat_id {
                    chat_id.unblock(context).await?;
                    chat_id_blocked = Blocked::Not;
                }
            }
        }

        if let Some(chat_id) = chat_id {
            better_msg = better_msg.or(apply_group_changes(
                context,
                mime_parser,
                sent_timestamp,
                chat_id,
                from_id,
                to_ids,
            )
            .await?);
        }

        if chat_id.is_none() && self_sent {
            // from_id==to_id==ContactId::SELF - this is a self-sent messages,
            // maybe an Autocrypt Setup Message
            if let Ok(chat) = ChatIdBlocked::get_for_contact(context, ContactId::SELF, Blocked::Not)
                .await
                .context("Failed to get (new) chat for contact")
                .log_err(context)
            {
                chat_id = Some(chat.id);
                chat_id_blocked = chat.blocked;
            }

            if let Some(chat_id) = chat_id {
                if Blocked::Not != chat_id_blocked {
                    chat_id.unblock(context).await?;
                    // Not assigning `chat_id_blocked = Blocked::Not` to avoid unused_assignments warning.
                }
            }
        }
    }

    if fetching_existing_messages && mime_parser.decrypting_failed {
        chat_id = Some(DC_CHAT_ID_TRASH);
        // We are only gathering old messages on first start. We do not want to add loads of non-decryptable messages to the chats.
        info!(context, "Existing non-decipherable message (TRASH).");
    }

    if mime_parser.webxdc_status_update.is_some() && mime_parser.parts.len() == 1 {
        if let Some(part) = mime_parser.parts.first() {
            if part.typ == Viewtype::Text && part.msg.is_empty() {
                chat_id = Some(DC_CHAT_ID_TRASH);
                info!(context, "Message is a status update only (TRASH).");
                markseen_on_imap_table(context, rfc724_mid).await.ok();
            }
        }
    }

    let orig_chat_id = chat_id;
    let chat_id = if is_mdn || is_reaction {
        DC_CHAT_ID_TRASH
    } else {
        chat_id.unwrap_or_else(|| {
            info!(context, "No chat id for message (TRASH).");
            DC_CHAT_ID_TRASH
        })
    };

    // Extract ephemeral timer from the message or use the existing timer if the message is not fully downloaded.
    let mut ephemeral_timer = if is_partial_download.is_some() {
        chat_id.get_ephemeral_timer(context).await?
    } else if let Some(value) = mime_parser.get_header(HeaderDef::EphemeralTimer) {
        match value.parse::<EphemeralTimer>() {
            Ok(timer) => timer,
            Err(err) => {
                warn!(context, "Can't parse ephemeral timer \"{value}\": {err:#}.");
                EphemeralTimer::Disabled
            }
        }
    } else {
        EphemeralTimer::Disabled
    };

    let in_fresh = state == MessageState::InFresh;
    let sort_timestamp = calc_sort_timestamp(context, sent_timestamp, chat_id, in_fresh).await?;

    // Apply ephemeral timer changes to the chat.
    //
    // Only apply the timer when there are visible parts (e.g., the message does not consist only
    // of `location.kml` attachment).  Timer changes without visible received messages may be
    // confusing to the user.
    if !chat_id.is_special()
        && !mime_parser.parts.is_empty()
        && chat_id.get_ephemeral_timer(context).await? != ephemeral_timer
    {
        info!(context, "Received new ephemeral timer value {ephemeral_timer:?} for chat {chat_id}, checking if it should be applied.");
        if is_dc_message == MessengerMessage::Yes
            && get_previous_message(context, mime_parser)
                .await?
                .map(|p| p.ephemeral_timer)
                == Some(ephemeral_timer)
            && mime_parser.is_system_message != SystemMessage::EphemeralTimerChanged
        {
            // The message is a Delta Chat message, so we know that previous message according to
            // References header is the last message in the chat as seen by the sender. The timer
            // is the same in both the received message and the last message, so we know that the
            // sender has not seen any change of the timer between these messages. As our timer
            // value is different, it means the sender has not received some timer update that we
            // have seen or sent ourselves, so we ignore incoming timer to prevent a rollback.
            warn!(
                context,
                "Ignoring ephemeral timer change to {ephemeral_timer:?} for chat {chat_id} to avoid rollback.",
            );
        } else if chat_id
            .update_timestamp(context, Param::EphemeralSettingsTimestamp, sent_timestamp)
            .await?
        {
            if let Err(err) = chat_id
                .inner_set_ephemeral_timer(context, ephemeral_timer)
                .await
            {
                warn!(
                    context,
                    "Failed to modify timer for chat {chat_id}: {err:#}."
                );
            } else {
                info!(
                    context,
                    "Updated ephemeral timer to {ephemeral_timer:?} for chat {chat_id}."
                );
                if mime_parser.is_system_message != SystemMessage::EphemeralTimerChanged {
                    chat::add_info_msg(
                        context,
                        chat_id,
                        &stock_ephemeral_timer_changed(context, ephemeral_timer, from_id).await,
                        sort_timestamp,
                    )
                    .await?;
                }
            }
        } else {
            warn!(
                context,
                "Ignoring ephemeral timer change to {ephemeral_timer:?} because it is outdated."
            );
        }
    }

    if mime_parser.is_system_message == SystemMessage::EphemeralTimerChanged {
        better_msg = Some(stock_ephemeral_timer_changed(context, ephemeral_timer, from_id).await);

        // Do not delete the system message itself.
        //
        // This prevents confusion when timer is changed
        // to 1 week, and then changed to 1 hour: after 1
        // hour, only the message about the change to 1
        // week is left.
        ephemeral_timer = EphemeralTimer::Disabled;
    }

    // if a chat is protected and the message is fully downloaded, check additional properties
    if !chat_id.is_special() && is_partial_download.is_none() {
        let chat = Chat::load_from_db(context, chat_id).await?;
        let new_status = match mime_parser.is_system_message {
            SystemMessage::ChatProtectionEnabled => Some(ProtectionStatus::Protected),
            SystemMessage::ChatProtectionDisabled => Some(ProtectionStatus::Unprotected),
            _ => None,
        };

        if chat.is_protected() || new_status.is_some() {
            if let Err(err) = check_verified_properties(context, mime_parser, from_id, to_ids).await
            {
                warn!(context, "Verification problem: {err:#}.");
                let s = format!("{err}. See 'Info' for more details");
                mime_parser.repl_msg_by_error(&s);
            } else {
                // change chat protection only when verification check passes
                if let Some(new_status) = new_status {
                    if chat_id
                        .update_timestamp(
                            context,
                            Param::ProtectionSettingsTimestamp,
                            sent_timestamp,
                        )
                        .await?
                    {
                        if let Err(e) = chat_id.inner_set_protection(context, new_status).await {
                            chat::add_info_msg(
                                context,
                                chat_id,
                                &format!("Cannot set protection: {e}"),
                                sort_timestamp,
                            )
                            .await?;
                            // do not return an error as this would result in retrying the message
                        }
                    }
                    better_msg = Some(context.stock_protection_msg(new_status, from_id).await);
                }
            }
        }
    }

    // Ensure replies to messages are sorted after the parent message.
    //
    // This is useful in a case where sender clocks are not
    // synchronized and parent message has a Date: header with a
    // timestamp higher than reply timestamp.
    //
    // This does not help if parent message arrives later than the
    // reply.
    let parent_timestamp = mime_parser.get_parent_timestamp(context).await?;
    let sort_timestamp = parent_timestamp.map_or(sort_timestamp, |parent_timestamp| {
        std::cmp::max(sort_timestamp, parent_timestamp)
    });

    // if the mime-headers should be saved, find out its size
    // (the mime-header ends with an empty line)
    let save_mime_headers = context.get_config_bool(Config::SaveMimeHeaders).await?;

    let mime_in_reply_to = mime_parser
        .get_header(HeaderDef::InReplyTo)
        .cloned()
        .unwrap_or_default();
    let mime_references = mime_parser
        .get_header(HeaderDef::References)
        .cloned()
        .unwrap_or_default();

    // fine, so far.  now, split the message into simple parts usable as "short messages"
    // and add them to the database (mails sent by other messenger clients should result
    // into only one message; mails sent by other clients may result in several messages
    // (eg. one per attachment))
    let icnt = mime_parser.parts.len();

    let subject = mime_parser.get_subject().unwrap_or_default();

    let is_system_message = mime_parser.is_system_message;

    // if indicated by the parser,
    // we save the full mime-message and add a flag
    // that the ui should show button to display the full message.

    // a flag used to avoid adding "show full message" button to multiple parts of the message.
    let mut save_mime_modified = mime_parser.is_mime_modified;

    let mime_headers = if save_mime_headers || save_mime_modified {
        let headers = if mime_parser.was_encrypted() && !mime_parser.decoded_data.is_empty() {
            mime_parser.decoded_data.clone()
        } else {
            imf_raw.to_vec()
        };
        tokio::task::block_in_place(move || buf_compress(&headers))?
    } else {
        Vec::new()
    };

    let mut created_db_entries = Vec::with_capacity(mime_parser.parts.len());

    for part in &mut mime_parser.parts {
        if part.is_reaction {
            set_msg_reaction(
                context,
                &mime_in_reply_to,
                orig_chat_id.unwrap_or_default(),
                from_id,
                Reaction::from(part.msg.as_str()),
            )
            .await?;
        }

        let mut param = part.param.clone();
        if is_system_message != SystemMessage::Unknown {
            param.set_int(Param::Cmd, is_system_message as i32);
        }

        if let Some(replace_msg_id) = replace_msg_id {
            let placeholder = Message::load_from_db(context, replace_msg_id).await?;
            for key in [
                Param::WebxdcSummary,
                Param::WebxdcSummaryTimestamp,
                Param::WebxdcDocument,
                Param::WebxdcDocumentTimestamp,
            ] {
                if let Some(value) = placeholder.param.get(key) {
                    param.set(key, value);
                }
            }
        }

        let mut txt_raw = "".to_string();
        let (msg, typ): (&str, Viewtype) = if let Some(better_msg) = &better_msg {
            (better_msg, Viewtype::Text)
        } else {
            (&part.msg, part.typ)
        };

        let part_is_empty = part.msg.is_empty() && part.param.get(Param::Quote).is_none();
        let mime_modified = save_mime_modified && !part_is_empty;
        if mime_modified {
            // Avoid setting mime_modified for more than one part.
            save_mime_modified = false;
        }

        if part.typ == Viewtype::Text {
            let msg_raw = part.msg_raw.as_ref().cloned().unwrap_or_default();
            txt_raw = format!("{subject}\n\n{msg_raw}");
        }

        let ephemeral_timestamp = if in_fresh {
            0
        } else {
            match ephemeral_timer {
                EphemeralTimer::Disabled => 0,
                EphemeralTimer::Enabled { duration } => {
                    rcvd_timestamp.saturating_add(duration.into())
                }
            }
        };

        // If you change which information is skipped if the message is trashed,
        // also change `MsgId::trash()` and `delete_expired_messages()`
        let trash = chat_id.is_trash() || (is_location_kml && msg.is_empty());

        let row_id = context
            .sql
            .call_write(|conn| {
                let mut stmt = conn.prepare_cached(
            r#"
INSERT INTO msgs
  (
    id,
    rfc724_mid, chat_id,
    from_id, to_id, timestamp, timestamp_sent, 
    timestamp_rcvd, type, state, msgrmsg, 
    txt, subject, txt_raw, param, 
    bytes, mime_headers, mime_compressed, mime_in_reply_to,
    mime_references, mime_modified, error, ephemeral_timer,
    ephemeral_timestamp, download_state, hop_info
  )
  VALUES (
    ?,
    ?, ?, ?, ?,
    ?, ?, ?, ?,
    ?, ?, ?, ?,
    ?, ?, ?, ?, 1,
    ?, ?, ?, ?,
    ?, ?, ?, ?
  )
ON CONFLICT (id) DO UPDATE
SET rfc724_mid=excluded.rfc724_mid, chat_id=excluded.chat_id,
    from_id=excluded.from_id, to_id=excluded.to_id, timestamp=excluded.timestamp, timestamp_sent=excluded.timestamp_sent,
    timestamp_rcvd=excluded.timestamp_rcvd, type=excluded.type, state=excluded.state, msgrmsg=excluded.msgrmsg,
    txt=excluded.txt, subject=excluded.subject, txt_raw=excluded.txt_raw, param=excluded.param,
    bytes=excluded.bytes, mime_headers=excluded.mime_headers,
    mime_compressed=excluded.mime_compressed, mime_in_reply_to=excluded.mime_in_reply_to,
    mime_references=excluded.mime_references, mime_modified=excluded.mime_modified, error=excluded.error, ephemeral_timer=excluded.ephemeral_timer,
    ephemeral_timestamp=excluded.ephemeral_timestamp, download_state=excluded.download_state, hop_info=excluded.hop_info
"#)?;
                stmt.execute(params![
                    replace_msg_id,
                    rfc724_mid,
                    if trash { DC_CHAT_ID_TRASH } else { chat_id },
                    if trash { ContactId::UNDEFINED } else { from_id },
                    if trash { ContactId::UNDEFINED } else { to_id },
                    sort_timestamp,
                    sent_timestamp,
                    rcvd_timestamp,
                    typ,
                    state,
                    is_dc_message,
                    if trash { "" } else { msg },
                    if trash { "" } else { &subject },
                    // txt_raw might contain invalid utf8
                    if trash { "" } else { &txt_raw },
                    if trash {
                        "".to_string()
                    } else {
                        param.to_string()
                    },
                    part.bytes as isize,
                    if (save_mime_headers || mime_modified) && !trash {
                        mime_headers.clone()
                    } else {
                        Vec::new()
                    },
                    mime_in_reply_to,
                    mime_references,
                    mime_modified,
                    part.error.as_deref().unwrap_or_default(),
                    ephemeral_timer,
                    ephemeral_timestamp,
                    if is_partial_download.is_some() {
                        DownloadState::Available
                    } else {
                        DownloadState::Done
                    },
                    mime_parser.hop_info
                ])?;
                let row_id = conn.last_insert_rowid();
                Ok(row_id)
            })
            .await?;

        // We only replace placeholder with a first part,
        // afterwards insert additional parts.
        replace_msg_id = None;

        created_db_entries.push(MsgId::new(u32::try_from(row_id)?));
    }

    // check all parts whether they contain a new logging webxdc
    for (part, msg_id) in mime_parser.parts.iter().zip(&created_db_entries) {
        maybe_set_logging_xdc_inner(
            context,
            part.typ,
            chat_id,
            part.param
                .get_path(Param::File, context)
                .unwrap_or_default(),
            *msg_id,
        )
        .await?;
    }

    if let Some(replace_msg_id) = replace_msg_id {
        // "Replace" placeholder with a message that has no parts.
        replace_msg_id.delete_from_db(context).await?;
    }

    chat_id.unarchive_if_not_muted(context, state).await?;

    info!(
        context,
        "Message has {icnt} parts and is assigned to chat #{chat_id}."
    );

    // new outgoing message from another device marks the chat as noticed.
    if !incoming && !chat_id.is_special() {
        chat::marknoticed_chat_if_older_than(context, chat_id, sort_timestamp).await?;
    }

    if !is_mdn {
        let mut chat = Chat::load_from_db(context, chat_id).await?;

        // In contrast to most other update-timestamps,
        // use `sort_timestamp` instead of `sent_timestamp` for the subject-timestamp comparison.
        // This way, `LastSubject` actually refers to the most recent message _shown_ in the chat.
        if chat
            .param
            .update_timestamp(Param::SubjectTimestamp, sort_timestamp)?
        {
            // write the last subject even if empty -
            // otherwise a reply may get an outdated subject.
            let subject = mime_parser.get_subject().unwrap_or_default();

            chat.param.set(Param::LastSubject, subject);
            chat.update_param(context).await?;
        }
    }

    if !incoming && is_mdn && is_dc_message == MessengerMessage::Yes {
        // Normally outgoing MDNs sent by us never appear in mailboxes, but Gmail saves all
        // outgoing messages, including MDNs, to the Sent folder. If we detect such saved MDN,
        // delete it.
        needs_delete_job = true;
    }

    Ok(ReceivedMsg {
        chat_id,
        state,
        sort_timestamp,
        msg_ids: created_db_entries,
        needs_delete_job,
    })
}

/// Saves attached locations to the database.
///
/// Emits an event if at least one new location was added.
async fn save_locations(
    context: &Context,
    mime_parser: &MimeMessage,
    chat_id: ChatId,
    from_id: ContactId,
    msg_id: MsgId,
) -> Result<()> {
    if chat_id.is_special() {
        // Do not save locations for trashed messages.
        return Ok(());
    }

    let mut send_event = false;

    if let Some(message_kml) = &mime_parser.message_kml {
        if let Some(newest_location_id) =
            location::save(context, chat_id, from_id, &message_kml.locations, true).await?
        {
            location::set_msg_location_id(context, msg_id, newest_location_id).await?;
            send_event = true;
        }
    }

    if let Some(location_kml) = &mime_parser.location_kml {
        if let Some(addr) = &location_kml.addr {
            let contact = Contact::get_by_id(context, from_id).await?;
            if contact.get_addr().to_lowercase() == addr.to_lowercase() {
                if let Some(newest_location_id) =
                    location::save(context, chat_id, from_id, &location_kml.locations, false)
                        .await?
                {
                    location::set_msg_location_id(context, msg_id, newest_location_id).await?;
                    send_event = true;
                }
            } else {
                warn!(
                    context,
                    "Address in location.kml {:?} is not the same as the sender address {:?}.",
                    addr,
                    contact.get_addr()
                );
            }
        }
    }
    if send_event {
        context.emit_event(EventType::LocationChanged(Some(from_id)));
    }
    Ok(())
}

async fn calc_sort_timestamp(
    context: &Context,
    message_timestamp: i64,
    chat_id: ChatId,
    is_fresh_msg: bool,
) -> Result<i64> {
    let mut sort_timestamp = message_timestamp;

    // get newest non fresh message for this chat
    // update sort_timestamp if less than that
    if is_fresh_msg {
        let last_msg_time: Option<i64> = context
            .sql
            .query_get_value(
                "SELECT MAX(timestamp) FROM msgs WHERE chat_id=? AND state>?",
                (chat_id, MessageState::InFresh),
            )
            .await?;

        if let Some(last_msg_time) = last_msg_time {
            if last_msg_time > sort_timestamp {
                sort_timestamp = last_msg_time;
            }
        }
    }

    Ok(min(sort_timestamp, smeared_time(context)))
}

async fn lookup_chat_by_reply(
    context: &Context,
    mime_parser: &MimeMessage,
    parent: &Option<Message>,
    to_ids: &[ContactId],
    from_id: ContactId,
) -> Result<Option<(ChatId, Blocked)>> {
    // Try to assign message to the same chat as the parent message.

    if let Some(parent) = parent {
        let parent_chat = Chat::load_from_db(context, parent.chat_id).await?;

        if parent.error.is_some() {
            // If the parent msg is undecipherable, then it may have been assigned to the wrong chat
            // (undecipherable group msgs often get assigned to the 1:1 chat with the sender).
            // We don't have any way of finding out whether a msg is undecipherable, so we check for
            // error.is_some() instead.
            return Ok(None);
        }

        if parent_chat.id == DC_CHAT_ID_TRASH {
            return Ok(None);
        }

        // If this was a private message just to self, it was probably a private reply.
        // It should not go into the group then, but into the private chat.
        if is_probably_private_reply(context, to_ids, from_id, mime_parser, parent_chat.id).await? {
            return Ok(None);
        }

        // If the parent chat is a 1:1 chat, and the sender is a classical MUA and added
        // a new person to TO/CC, then the message should not go to the 1:1 chat, but to a
        // newly created ad-hoc group.
        if parent_chat.typ == Chattype::Single
            && !mime_parser.has_chat_version()
            && to_ids.len() > 1
        {
            let mut chat_contacts = chat::get_chat_contacts(context, parent_chat.id).await?;
            chat_contacts.push(ContactId::SELF);
            if to_ids.iter().any(|id| !chat_contacts.contains(id)) {
                return Ok(None);
            }
        }

        info!(
            context,
            "Assigning message to {} as it's a reply to {}.", parent_chat.id, parent.rfc724_mid
        );
        return Ok(Some((parent_chat.id, parent_chat.blocked)));
    }

    Ok(None)
}

/// If this method returns true, the message shall be assigned to the 1:1 chat with the sender.
/// If it returns false, it shall be assigned to the parent chat.
async fn is_probably_private_reply(
    context: &Context,
    to_ids: &[ContactId],
    from_id: ContactId,
    mime_parser: &MimeMessage,
    parent_chat_id: ChatId,
) -> Result<bool> {
    // Usually we don't want to show private replies in the parent chat, but in the
    // 1:1 chat with the sender.
    //
    // There is one exception: Classical MUA replies to two-member groups
    // should be assigned to the group chat. We restrict this exception to classical emails, as chat-group-messages
    // contain a Chat-Group-Id header and can be sorted into the correct chat this way.

    let private_message =
        (to_ids == [ContactId::SELF]) || (from_id == ContactId::SELF && to_ids.len() == 1);
    if !private_message {
        return Ok(false);
    }

    if !mime_parser.has_chat_version() {
        let chat_contacts = chat::get_chat_contacts(context, parent_chat_id).await?;
        if chat_contacts.len() == 2 && chat_contacts.contains(&ContactId::SELF) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// This function tries to extract the group-id from the message and returns the corresponding
/// chat_id. If the chat does not exist, it is created. If there is no group-id and there are more
/// than two members, a new ad hoc group is created.
///
/// On success the function returns the found/created (chat_id, chat_blocked) tuple.
async fn create_or_lookup_group(
    context: &Context,
    mime_parser: &mut MimeMessage,
    allow_creation: bool,
    create_blocked: Blocked,
    from_id: ContactId,
    to_ids: &[ContactId],
) -> Result<Option<(ChatId, Blocked)>> {
    let grpid = if let Some(grpid) = try_getting_grpid(mime_parser) {
        grpid
    } else if allow_creation {
        let mut member_ids: Vec<ContactId> = to_ids.to_vec();
        if !member_ids.contains(&(from_id)) {
            member_ids.push(from_id);
        }
        if !member_ids.contains(&(ContactId::SELF)) {
            member_ids.push(ContactId::SELF);
        }

        let res = create_adhoc_group(context, mime_parser, create_blocked, &member_ids)
            .await
            .context("could not create ad hoc group")?
            .map(|chat_id| (chat_id, create_blocked));
        return Ok(res);
    } else {
        info!(context, "Creating ad-hoc group prevented from caller.");
        return Ok(None);
    };

    let mut chat_id;
    let mut chat_id_blocked;
    if let Some((id, _protected, blocked)) = chat::get_chat_id_by_grpid(context, &grpid).await? {
        chat_id = Some(id);
        chat_id_blocked = blocked;
    } else {
        chat_id = None;
        chat_id_blocked = Default::default();
    }

    // For chat messages, we don't have to guess (is_*probably*_private_reply()) but we know for sure that
    // they belong to the group because of the Chat-Group-Id or Message-Id header
    if let Some(chat_id) = chat_id {
        if !mime_parser.has_chat_version()
            && is_probably_private_reply(context, to_ids, from_id, mime_parser, chat_id).await?
        {
            return Ok(None);
        }
    }

    let create_protected = if mime_parser.get_header(HeaderDef::ChatVerified).is_some() {
        if let Err(err) = check_verified_properties(context, mime_parser, from_id, to_ids).await {
            warn!(context, "Verification problem: {err:#}.");
            let s = format!("{err}. See 'Info' for more details");
            mime_parser.repl_msg_by_error(&s);
        }
        ProtectionStatus::Protected
    } else {
        ProtectionStatus::Unprotected
    };

    async fn self_explicitly_added(
        context: &Context,
        mime_parser: &&mut MimeMessage,
    ) -> Result<bool> {
        let ret = match mime_parser.get_header(HeaderDef::ChatGroupMemberAdded) {
            Some(member_addr) => context.is_self_addr(member_addr).await?,
            None => false,
        };
        Ok(ret)
    }

    if chat_id.is_none()
            && !mime_parser.is_mailinglist_message()
            && !grpid.is_empty()
            && mime_parser.get_header(HeaderDef::ChatGroupName).is_some()
            // otherwise, a pending "quit" message may pop up
            && mime_parser.get_header(HeaderDef::ChatGroupMemberRemoved).is_none()
            // re-create explicitly left groups only if ourself is re-added
            && (!chat::is_group_explicitly_left(context, &grpid).await?
                || self_explicitly_added(context, &mime_parser).await?)
    {
        // Group does not exist but should be created.
        if !allow_creation {
            info!(context, "Creating group forbidden by caller.");
            return Ok(None);
        }

        let grpname = mime_parser
            .get_header(HeaderDef::ChatGroupName)
            .context("Chat-Group-Name vanished")?
            // W/a for "Space added before long group names after MIME serialization/deserialization
            // #3650" issue. DC itself never creates group names with leading/trailing whitespace.
            .trim();
        let new_chat_id = ChatId::create_multiuser_record(
            context,
            Chattype::Group,
            &grpid,
            grpname,
            create_blocked,
            create_protected,
            None,
        )
        .await
        .with_context(|| format!("Failed to create group '{grpname}' for grpid={grpid}"))?;

        chat_id = Some(new_chat_id);
        chat_id_blocked = create_blocked;

        // Create initial member list.
        let mut members = vec![ContactId::SELF];
        if !from_id.is_special() {
            members.push(from_id);
        }
        members.extend(to_ids);
        members.dedup();
        chat::add_to_chat_contacts_table(context, new_chat_id, &members).await?;

        // once, we have protected-chats explained in UI, we can uncomment the following lines.
        // ("verified groups" did not add a message anyway)
        //
        //if create_protected == ProtectionStatus::Protected {
        // set from_id=0 as it is not clear that the sender of this random group message
        // actually really has enabled chat-protection at some point.
        //new_chat_id
        //    .add_protection_msg(context, ProtectionStatus::Protected, false, 0)
        //    .await?;
        //}

        context.emit_event(EventType::ChatModified(new_chat_id));
    }

    if let Some(chat_id) = chat_id {
        Ok(Some((chat_id, chat_id_blocked)))
    } else if mime_parser.decrypting_failed {
        // It is possible that the message was sent to a valid,
        // yet unknown group, which was rejected because
        // Chat-Group-Name, which is in the encrypted part, was
        // not found. We can't create a properly named group in
        // this case, so assign error message to 1:1 chat with the
        // sender instead.
        Ok(None)
    } else {
        // The message was decrypted successfully, but contains a late "quit" or otherwise
        // unwanted message.
        info!(context, "Message belongs to unwanted group (TRASH).");
        Ok(Some((DC_CHAT_ID_TRASH, Blocked::Not)))
    }
}

/// Apply group member list, name, avatar and protection status changes from the MIME message.
///
/// Optionally returns better message to replace the original system message.
async fn apply_group_changes(
    context: &Context,
    mime_parser: &mut MimeMessage,
    sent_timestamp: i64,
    chat_id: ChatId,
    from_id: ContactId,
    to_ids: &[ContactId],
) -> Result<Option<String>> {
    let mut chat = Chat::load_from_db(context, chat_id).await?;
    if chat.typ != Chattype::Group {
        return Ok(None);
    }

    let mut recreate_member_list = false;
    let mut send_event_chat_modified = false;

    let mut better_msg = None;
    let removed_id;
    if let Some(removed_addr) = mime_parser
        .get_header(HeaderDef::ChatGroupMemberRemoved)
        .cloned()
    {
        removed_id = Contact::lookup_id_by_addr(context, &removed_addr, Origin::Unknown).await?;
        recreate_member_list = true;
        match removed_id {
            Some(contact_id) => {
                better_msg = if contact_id == from_id {
                    Some(stock_str::msg_group_left(context, from_id).await)
                } else {
                    Some(stock_str::msg_del_member(context, &removed_addr, from_id).await)
                };
            }
            None => warn!(context, "Removed {removed_addr:?} has no contact_id."),
        }
    } else {
        removed_id = None;
        if let Some(added_member) = mime_parser
            .get_header(HeaderDef::ChatGroupMemberAdded)
            .cloned()
        {
            better_msg = Some(stock_str::msg_add_member(context, &added_member, from_id).await);
            recreate_member_list = true;
        } else if let Some(old_name) = mime_parser
            .get_header(HeaderDef::ChatGroupNameChanged)
            // See create_or_lookup_group() for explanation
            .map(|s| s.trim())
        {
            if let Some(grpname) = mime_parser
                .get_header(HeaderDef::ChatGroupName)
                // See create_or_lookup_group() for explanation
                .map(|grpname| grpname.trim())
                .filter(|grpname| grpname.len() < 200)
            {
                if chat_id
                    .update_timestamp(context, Param::GroupNameTimestamp, sent_timestamp)
                    .await?
                {
                    info!(context, "Updating grpname for chat {chat_id}.");
                    context
                        .sql
                        .execute(
                            "UPDATE chats SET name=? WHERE id=?;",
                            (strip_rtlo_characters(grpname), chat_id),
                        )
                        .await?;
                    send_event_chat_modified = true;
                }

                better_msg =
                    Some(stock_str::msg_grp_name(context, old_name, grpname, from_id).await);
            }
        } else if let Some(value) = mime_parser.get_header(HeaderDef::ChatContent) {
            if value == "group-avatar-changed" {
                if let Some(avatar_action) = &mime_parser.group_avatar {
                    // this is just an explicit message containing the group-avatar,
                    // apart from that, the group-avatar is send along with various other messages
                    better_msg = match avatar_action {
                        AvatarAction::Delete => {
                            Some(stock_str::msg_grp_img_deleted(context, from_id).await)
                        }
                        AvatarAction::Change(_) => {
                            Some(stock_str::msg_grp_img_changed(context, from_id).await)
                        }
                    };
                }
            }
        }
    }

    if !mime_parser.has_chat_version() {
        // If a classical MUA user adds someone to TO/CC, then the DC user shall
        // see this addition and have the new recipient in the member list.
        recreate_member_list = true;
    }

    if mime_parser.get_header(HeaderDef::ChatVerified).is_some() {
        if let Err(err) = check_verified_properties(context, mime_parser, from_id, to_ids).await {
            warn!(context, "Verification problem: {err:#}.");
            let s = format!("{err}. See 'Info' for more details");
            mime_parser.repl_msg_by_error(&s);
        }

        if !chat.is_protected() {
            chat_id
                .inner_set_protection(context, ProtectionStatus::Protected)
                .await?;
            recreate_member_list = true;
        }
    }

    // add members to group/check members
    if recreate_member_list {
        if chat::is_contact_in_chat(context, chat_id, ContactId::SELF).await?
            && !chat::is_contact_in_chat(context, chat_id, from_id).await?
        {
            warn!(
                context,
                "Contact {from_id} attempts to modify group chat {chat_id} member list without being a member."
            );
        } else if chat_id
            .update_timestamp(context, Param::MemberListTimestamp, sent_timestamp)
            .await?
        {
            let mut members_to_add = vec![];
            if removed_id.is_some()
                || !chat::is_contact_in_chat(context, chat_id, ContactId::SELF).await?
            {
                // Members could have been removed while we were
                // absent. We can't use existing member list and need to
                // start from scratch.
                context
                    .sql
                    .execute("DELETE FROM chats_contacts WHERE chat_id=?;", (chat_id,))
                    .await?;

                members_to_add.push(ContactId::SELF);
            }

            if !from_id.is_special() {
                members_to_add.push(from_id);
            }
            members_to_add.extend(to_ids);
            if let Some(removed_id) = removed_id {
                members_to_add.retain(|id| *id != removed_id);
            }
            members_to_add.dedup();

            info!(context, "Adding {members_to_add:?} to chat id={chat_id}.");
            chat::add_to_chat_contacts_table(context, chat_id, &members_to_add).await?;
            send_event_chat_modified = true;
        }
    }

    if let Some(avatar_action) = &mime_parser.group_avatar {
        if !chat::is_contact_in_chat(context, chat_id, ContactId::SELF).await? {
            warn!(
                context,
                "Received group avatar update for group chat {chat_id} we are not a member of."
            );
        } else if !chat::is_contact_in_chat(context, chat_id, from_id).await? {
            warn!(
                context,
                "Contact {from_id} attempts to modify group chat {chat_id} avatar without being a member.",
            );
        } else {
            info!(context, "Group-avatar change for {chat_id}.");
            if chat
                .param
                .update_timestamp(Param::AvatarTimestamp, sent_timestamp)?
            {
                match avatar_action {
                    AvatarAction::Change(profile_image) => {
                        chat.param.set(Param::ProfileImage, profile_image);
                    }
                    AvatarAction::Delete => {
                        chat.param.remove(Param::ProfileImage);
                    }
                };
                chat.update_param(context).await?;
                send_event_chat_modified = true;
            }
        }
    }

    if send_event_chat_modified {
        context.emit_event(EventType::ChatModified(chat_id));
    }
    Ok(better_msg)
}

/// Create or lookup a mailing list chat.
///
/// `list_id_header` contains the Id that must be used for the mailing list
/// and has the form `Name <Id>`, `<Id>` or just `Id`.
/// Depending on the mailing list type, `list_id_header`
/// was picked from `ListId:`-header or the `Sender:`-header.
///
/// `mime_parser` is the corresponding message
/// and is used to figure out the mailing list name from different header fields.
#[allow(clippy::indexing_slicing)]
async fn create_or_lookup_mailinglist(
    context: &Context,
    allow_creation: bool,
    list_id_header: &str,
    mime_parser: &MimeMessage,
) -> Result<Option<(ChatId, Blocked)>> {
    static LIST_ID: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(.+)<(.+)>$").unwrap());
    let (mut name, listid) = match LIST_ID.captures(list_id_header) {
        Some(cap) => (cap[1].trim().to_string(), cap[2].trim().to_string()),
        None => (
            "".to_string(),
            list_id_header
                .trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string(),
        ),
    };

    if let Some((chat_id, _, blocked)) = chat::get_chat_id_by_grpid(context, &listid).await? {
        return Ok(Some((chat_id, blocked)));
    }

    // for mailchimp lists, the name in `ListId` is just a long number.
    // a usable name for these lists is in the `From` header
    // and we can detect these lists by a unique `ListId`-suffix.
    if listid.ends_with(".list-id.mcsv.net") {
        if let Some(display_name) = &mime_parser.from.display_name {
            name = display_name.clone();
        }
    }

    // additional names in square brackets in the subject are preferred
    // (as that part is much more visible, we assume, that names is shorter and comes more to the point,
    // than the sometimes longer part from ListId)
    let subject = mime_parser.get_subject().unwrap_or_default();
    static SUBJECT: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^.{0,5}\[(.+?)\](\s*\[.+\])?").unwrap()); // remove square brackets around first name
    if let Some(cap) = SUBJECT.captures(&subject) {
        name = cap[1].to_string() + cap.get(2).map_or("", |m| m.as_str());
    }

    // if we do not have a name yet and `From` indicates, that this is a notification list,
    // a usable name is often in the `From` header (seen for several parcel service notifications).
    // same, if we do not have a name yet and `List-Id` has a known suffix (`.xt.local`)
    //
    // this pattern is similar to mailchimp above, however,
    // with weaker conditions and does not overwrite existing names.
    if name.is_empty()
        && (mime_parser.from.addr.contains("noreply")
            || mime_parser.from.addr.contains("no-reply")
            || mime_parser.from.addr.starts_with("notifications@")
            || mime_parser.from.addr.starts_with("newsletter@")
            || listid.ends_with(".xt.local"))
    {
        if let Some(display_name) = &mime_parser.from.display_name {
            name = display_name.clone();
        }
    }

    // as a last resort, use the ListId as the name
    // but strip some known, long hash prefixes
    if name.is_empty() {
        // 51231231231231231231231232869f58.xing.com -> xing.com
        static PREFIX_32_CHARS_HEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"([0-9a-fA-F]{32})\.(.{6,})").unwrap());
        if let Some(cap) = PREFIX_32_CHARS_HEX.captures(&listid) {
            name = cap[2].to_string();
        } else {
            name = listid.clone();
        }
    }

    if allow_creation {
        // list does not exist but should be created
        let param = mime_parser.list_post.as_ref().map(|list_post| {
            let mut p = Params::new();
            p.set(Param::ListPost, list_post);
            p.to_string()
        });

        let is_bot = context.get_config_bool(Config::Bot).await?;
        let blocked = if is_bot {
            Blocked::Not
        } else {
            Blocked::Request
        };
        let chat_id = ChatId::create_multiuser_record(
            context,
            Chattype::Mailinglist,
            &listid,
            &name,
            blocked,
            ProtectionStatus::Unprotected,
            param,
        )
        .await
        .with_context(|| {
            format!(
                "failed to create mailinglist '{}' for grpid={}",
                &name, &listid
            )
        })?;

        chat::add_to_chat_contacts_table(context, chat_id, &[ContactId::SELF]).await?;
        Ok(Some((chat_id, blocked)))
    } else {
        info!(context, "Creating list forbidden by caller.");
        Ok(None)
    }
}

/// Set ListId param on the contact and ListPost param the chat.
/// Only called for incoming messages since outgoing messages never have a
/// List-Post header, anyway.
async fn apply_mailinglist_changes(
    context: &Context,
    mime_parser: &MimeMessage,
    chat_id: ChatId,
) -> Result<()> {
    if let Some(list_post) = &mime_parser.list_post {
        let mut chat = Chat::load_from_db(context, chat_id).await?;
        if chat.typ != Chattype::Mailinglist {
            return Ok(());
        }
        let listid = &chat.grpid;

        let list_post = match ContactAddress::new(list_post) {
            Ok(list_post) => list_post,
            Err(err) => {
                warn!(context, "Invalid List-Post: {:#}.", err);
                return Ok(());
            }
        };
        let (contact_id, _) =
            Contact::add_or_lookup(context, "", list_post, Origin::Hidden).await?;
        let mut contact = Contact::load_from_db(context, contact_id).await?;
        if contact.param.get(Param::ListId) != Some(listid) {
            contact.param.set(Param::ListId, listid);
            contact.update_param(context).await?;
        }

        if let Some(old_list_post) = chat.param.get(Param::ListPost) {
            if list_post.as_ref() != old_list_post {
                // Apparently the mailing list is using a different List-Post header in each message.
                // Make the mailing list read-only because we wouldn't know which message the user wants to reply to.
                chat.param.remove(Param::ListPost);
                chat.update_param(context).await?;
            }
        } else {
            chat.param.set(Param::ListPost, list_post);
            chat.update_param(context).await?;
        }
    }

    Ok(())
}

fn try_getting_grpid(mime_parser: &MimeMessage) -> Option<String> {
    if let Some(optional_field) = mime_parser.get_header(HeaderDef::ChatGroupId) {
        return Some(optional_field.clone());
    }

    // Useful for undecipherable messages sent to known group.
    if let Some(extracted_grpid) = extract_grpid(mime_parser, HeaderDef::MessageId) {
        return Some(extracted_grpid.to_string());
    }

    if !mime_parser.has_chat_version() {
        if let Some(extracted_grpid) = extract_grpid(mime_parser, HeaderDef::InReplyTo) {
            return Some(extracted_grpid.to_string());
        } else if let Some(extracted_grpid) = extract_grpid(mime_parser, HeaderDef::References) {
            return Some(extracted_grpid.to_string());
        }
    }

    None
}

/// try extract a grpid from a message-id list header value
fn extract_grpid(mime_parser: &MimeMessage, headerdef: HeaderDef) -> Option<&str> {
    let header = mime_parser.get_header(headerdef)?;
    let parts = header
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    parts.filter_map(extract_grpid_from_rfc724_mid).next()
}

/// Creates ad-hoc group and returns chat ID on success.
async fn create_adhoc_group(
    context: &Context,
    mime_parser: &MimeMessage,
    create_blocked: Blocked,
    member_ids: &[ContactId],
) -> Result<Option<ChatId>> {
    if mime_parser.is_mailinglist_message() {
        info!(
            context,
            "Not creating ad-hoc group for mailing list message."
        );

        return Ok(None);
    }

    if mime_parser.decrypting_failed {
        // Do not create a new ad-hoc group if the message cannot be
        // decrypted.
        //
        // The subject may be encrypted and contain a placeholder such
        // as "...". It can also be a COI group, with encrypted
        // Chat-Group-ID and incompatible Message-ID format.
        //
        // Instead, assign the message to 1:1 chat with the sender.
        warn!(
            context,
            "Not creating ad-hoc group for message that cannot be decrypted."
        );
        return Ok(None);
    }

    if member_ids.len() < 3 {
        info!(context, "Not creating ad-hoc group: too few contacts.");
        return Ok(None);
    }

    // use subject as initial chat name
    let grpname = mime_parser
        .get_subject()
        .unwrap_or_else(|| "Unnamed group".to_string());

    let new_chat_id: ChatId = ChatId::create_multiuser_record(
        context,
        Chattype::Group,
        "", // Ad hoc groups have no ID.
        &grpname,
        create_blocked,
        ProtectionStatus::Unprotected,
        None,
    )
    .await?;
    chat::add_to_chat_contacts_table(context, new_chat_id, member_ids).await?;

    context.emit_event(EventType::ChatModified(new_chat_id));

    Ok(Some(new_chat_id))
}

async fn check_verified_properties(
    context: &Context,
    mimeparser: &MimeMessage,
    from_id: ContactId,
    to_ids: &[ContactId],
) -> Result<()> {
    let contact = Contact::load_from_db(context, from_id).await?;

    ensure!(mimeparser.was_encrypted(), "This message is not encrypted.");

    if mimeparser.get_header(HeaderDef::ChatVerified).is_none() {
        // we do not fail here currently, this would exclude (a) non-deltas
        // and (b) deltas with different protection views across multiple devices.
        // for group creation or protection enabled/disabled, however, Chat-Verified is respected.
        warn!(
            context,
            "{} did not mark message as protected.",
            contact.get_addr()
        );
    }

    // ensure, the contact is verified
    // and the message is signed with a verified key of the sender.
    // this check is skipped for SELF as there is no proper SELF-peerstate
    // and results in group-splits otherwise.
    if from_id != ContactId::SELF {
        let peerstate = Peerstate::from_addr(context, contact.get_addr()).await?;

        if peerstate.is_none()
            || contact.is_verified_ex(context, peerstate.as_ref()).await?
                != VerifiedStatus::BidirectVerified
        {
            bail!(
                "Sender of this message is not verified: {}",
                contact.get_addr()
            );
        }

        if let Some(peerstate) = peerstate {
            ensure!(
                peerstate.has_verified_key(&mimeparser.signatures),
                "The message was sent with non-verified encryption."
            );
        }
    }

    // we do not need to check if we are verified with ourself
    let to_ids = to_ids
        .iter()
        .copied()
        .filter(|id| *id != ContactId::SELF)
        .collect::<Vec<ContactId>>();

    if to_ids.is_empty() {
        return Ok(());
    }

    let rows = context
        .sql
        .query_map(
            &format!(
                "SELECT c.addr, LENGTH(ps.verified_key_fingerprint)  FROM contacts c  \
             LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.id IN({}) ",
                sql::repeat_vars(to_ids.len())
            ),
            rusqlite::params_from_iter(to_ids),
            |row| {
                let to_addr: String = row.get(0)?;
                let is_verified: i32 = row.get(1).unwrap_or(0);
                Ok((to_addr, is_verified != 0))
            },
            |rows| {
                rows.collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            },
        )
        .await?;

    for (to_addr, mut is_verified) in rows {
        info!(
            context,
            "check_verified_properties: {:?} self={:?}.",
            to_addr,
            context.is_self_addr(&to_addr).await
        );
        let peerstate = Peerstate::from_addr(context, &to_addr).await?;

        // mark gossiped keys (if any) as verified
        if mimeparser.gossiped_addr.contains(&to_addr) {
            if let Some(mut peerstate) = peerstate {
                // if we're here, we know the gossip key is verified:
                // - use the gossip-key as verified-key if there is no verified-key
                // - OR if the verified-key does not match public-key or gossip-key
                //   (otherwise a verified key can _only_ be updated through QR scan which might be annoying,
                //   see <https://github.com/nextleap-project/countermitm/issues/46> for a discussion about this point)
                if !is_verified
                    || peerstate.verified_key_fingerprint != peerstate.public_key_fingerprint
                        && peerstate.verified_key_fingerprint != peerstate.gossip_key_fingerprint
                {
                    info!(context, "{} has verified {}.", contact.get_addr(), to_addr);
                    let fp = peerstate.gossip_key_fingerprint.clone();
                    if let Some(fp) = fp {
                        peerstate.set_verified(
                            PeerstateKeyType::GossipKey,
                            fp,
                            PeerstateVerifiedStatus::BidirectVerified,
                            contact.get_addr().to_owned(),
                        )?;
                        peerstate.save_to_db(&context.sql).await?;
                        is_verified = true;
                    }
                }
            }
        }
        if !is_verified {
            bail!(
                "{} is not a member of this protected chat",
                to_addr.to_string()
            );
        }
    }
    Ok(())
}

/// Returns the last message referenced from `References` header if it is in the database.
///
/// For Delta Chat messages it is the last message in the chat of the sender.
///
/// Note that the returned message may be trashed.
async fn get_previous_message(
    context: &Context,
    mime_parser: &MimeMessage,
) -> Result<Option<Message>> {
    if let Some(field) = mime_parser.get_header(HeaderDef::References) {
        if let Some(rfc724mid) = parse_message_ids(field).last() {
            if let Some(msg_id) = rfc724_mid_exists(context, rfc724mid).await? {
                return Ok(Some(Message::load_from_db(context, msg_id).await?));
            }
        }
    }
    Ok(None)
}

/// Given a list of Message-IDs, returns the latest message found in the database.
///
/// Only messages that are not in the trash chat are considered.
async fn get_rfc724_mid_in_list(context: &Context, mid_list: &str) -> Result<Option<Message>> {
    if mid_list.is_empty() {
        return Ok(None);
    }

    for id in parse_message_ids(mid_list).iter().rev() {
        if let Some(msg_id) = rfc724_mid_exists(context, id).await? {
            let msg = Message::load_from_db(context, msg_id).await?;
            if msg.chat_id != DC_CHAT_ID_TRASH {
                return Ok(Some(msg));
            }
        }
    }

    Ok(None)
}

/// Returns the last message referenced from References: header found in the database.
///
/// If none found, tries In-Reply-To: as a fallback for classic MUAs that don't set the
/// References: header.
// TODO also save first entry of References and look for this?
async fn get_parent_message(
    context: &Context,
    mime_parser: &MimeMessage,
) -> Result<Option<Message>> {
    if let Some(field) = mime_parser.get_header(HeaderDef::References) {
        if let Some(msg) = get_rfc724_mid_in_list(context, field).await? {
            return Ok(Some(msg));
        }
    }

    if let Some(field) = mime_parser.get_header(HeaderDef::InReplyTo) {
        if let Some(msg) = get_rfc724_mid_in_list(context, field).await? {
            return Ok(Some(msg));
        }
    }

    Ok(None)
}

pub(crate) async fn get_prefetch_parent_message(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<Option<Message>> {
    if let Some(field) = headers.get_header_value(HeaderDef::References) {
        if let Some(msg) = get_rfc724_mid_in_list(context, &field).await? {
            return Ok(Some(msg));
        }
    }

    if let Some(field) = headers.get_header_value(HeaderDef::InReplyTo) {
        if let Some(msg) = get_rfc724_mid_in_list(context, &field).await? {
            return Ok(Some(msg));
        }
    }

    Ok(None)
}

/// Looks up contact IDs from the database given the list of recipients.
///
/// Returns vector of IDs guaranteed to be unique.
///
/// * param `prevent_rename`: if true, the display_name of this contact will not be changed. Useful for
/// mailing lists: In some mailing lists, many users write from the same address but with different
/// display names. We don't want the display name to change every time the user gets a new email from
/// a mailing list.
async fn add_or_lookup_contacts_by_address_list(
    context: &Context,
    address_list: &[SingleInfo],
    origin: Origin,
) -> Result<Vec<ContactId>> {
    let mut contact_ids = HashSet::new();
    for info in address_list.iter() {
        let addr = &info.addr;
        if !may_be_valid_addr(addr) {
            continue;
        }
        let display_name = info.display_name.as_deref();
        if let Ok(addr) = ContactAddress::new(addr) {
            let contact_id =
                add_or_lookup_contact_by_addr(context, display_name, addr, origin).await?;
            contact_ids.insert(contact_id);
        } else {
            warn!(context, "Contact with address {:?} cannot exist.", addr);
        }
    }

    Ok(contact_ids.into_iter().collect::<Vec<ContactId>>())
}

/// Add contacts to database on receiving messages.
async fn add_or_lookup_contact_by_addr(
    context: &Context,
    display_name: Option<&str>,
    addr: ContactAddress<'_>,
    origin: Origin,
) -> Result<ContactId> {
    if context.is_self_addr(&addr).await? {
        return Ok(ContactId::SELF);
    }
    let display_name_normalized = display_name.map(normalize_name).unwrap_or_default();

    let (contact_id, _modified) =
        Contact::add_or_lookup(context, &display_name_normalized, addr, origin).await?;
    Ok(contact_id)
}

#[cfg(test)]
mod tests;

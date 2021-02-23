use anyhow::{bail, ensure, format_err, Result};
use itertools::join;
use mailparse::SingleInfo;
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};

use crate::chat::{self, Chat, ChatId, ProtectionStatus};
use crate::config::Config;
use crate::constants::{
    Blocked, Chattype, ShowEmails, Viewtype, DC_CHAT_ID_TRASH, DC_CONTACT_ID_LAST_SPECIAL,
    DC_CONTACT_ID_SELF,
};
use crate::contact::{addr_cmp, normalize_name, Contact, Origin, VerifiedStatus};
use crate::context::Context;
use crate::dc_tools::{
    dc_create_smeared_timestamp, dc_extract_grpid_from_rfc724_mid, dc_smeared_time, time,
};
use crate::ephemeral::{stock_ephemeral_timer_changed, Timer as EphemeralTimer};
use crate::events::EventType;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::job::{self, Action};
use crate::message::{self, rfc724_mid_exists, Message, MessageState, MessengerMessage, MsgId};
use crate::mimeparser::{
    parse_message_ids, AvatarAction, MailinglistType, MimeMessage, SystemMessage,
};
use crate::param::{Param, Params};
use crate::peerstate::{Peerstate, PeerstateKeyType, PeerstateVerifiedStatus};
use crate::securejoin::{self, handle_securejoin_handshake, observe_securejoin_on_other_device};
use crate::stock_str;
use crate::{contact, location};

// IndexSet is like HashSet but maintains order of insertion
type ContactIds = indexmap::IndexSet<u32>;

#[derive(Debug, PartialEq, Eq)]
enum CreateEvent {
    MsgsChanged,
    IncomingMsg,
}

/// Receive a message and add it to the database.
///
/// Returns an error on recoverable errors, e.g. database errors. In this case,
/// message parsing should be retried later. If message itself is wrong, logs
/// the error and returns success.
pub async fn dc_receive_imf(
    context: &Context,
    imf_raw: &[u8],
    server_folder: impl AsRef<str>,
    server_uid: u32,
    seen: bool,
) -> Result<()> {
    dc_receive_imf_inner(context, imf_raw, server_folder, server_uid, seen, false).await
}

pub(crate) async fn dc_receive_imf_inner(
    context: &Context,
    imf_raw: &[u8],
    server_folder: impl AsRef<str>,
    server_uid: u32,
    seen: bool,
    fetching_existing_messages: bool,
) -> Result<()> {
    info!(
        context,
        "Receiving message {}/{}, seen={}...",
        if !server_folder.as_ref().is_empty() {
            server_folder.as_ref()
        } else {
            "?"
        },
        server_uid,
        seen
    );

    if std::env::var(crate::DCC_MIME_DEBUG).unwrap_or_default() == "2" {
        info!(context, "dc_receive_imf: incoming message mime-body:");
        println!("{}", String::from_utf8_lossy(imf_raw));
    }

    let mut mime_parser = match MimeMessage::from_bytes(context, imf_raw).await {
        Err(err) => {
            warn!(context, "dc_receive_imf: can't parse MIME: {}", err);
            return Ok(());
        }
        Ok(mime_parser) => mime_parser,
    };

    // we can not add even an empty record if we have no info whatsoever
    if !mime_parser.has_headers() {
        warn!(context, "dc_receive_imf: no headers found");
        return Ok(());
    }

    // the function returns the number of created messages in the database
    let mut chat_id = ChatId::new(0);
    let mut hidden = false;

    let mut needs_delete_job = false;
    let mut insert_msg_id = MsgId::new_unset();

    let mut sent_timestamp = 0;
    let mut created_db_entries = Vec::new();
    let mut create_event_to_send = Some(CreateEvent::MsgsChanged);

    // helper method to handle early exit and memory cleanup
    let cleanup = |context: &Context,
                   create_event_to_send: &Option<CreateEvent>,
                   created_db_entries: Vec<(ChatId, MsgId)>| {
        if let Some(create_event_to_send) = create_event_to_send {
            for (chat_id, msg_id) in created_db_entries {
                let event = match create_event_to_send {
                    CreateEvent::MsgsChanged => EventType::MsgsChanged { msg_id, chat_id },
                    CreateEvent::IncomingMsg => EventType::IncomingMsg { msg_id, chat_id },
                };
                context.emit_event(event);
            }
        }
    };

    if let Some(value) = mime_parser.get(HeaderDef::Date) {
        // is not yet checked against bad times! we do this later if we have the database information.
        sent_timestamp = mailparse::dateparse(value).unwrap_or_default();
    }

    let prevent_rename =
        mime_parser.is_mailinglist_message() || mime_parser.get(HeaderDef::Sender).is_some();

    // get From: (it can be an address list!) and check if it is known (for known From:'s we add
    // the other To:/Cc: in the 3rd pass)
    // or if From: is equal to SELF (in this case, it is any outgoing messages,
    // we do not check Return-Path any more as this is unreliable, see
    // https://github.com/deltachat/deltachat-core/issues/150)
    //
    // If this is a mailing list email (i.e. list_id_header is some), don't change the displayname because in
    // a mailing list the sender displayname sometimes does not belong to the sender email address.
    let (from_id, _from_id_blocked, incoming_origin) =
        from_field_to_contact_id(context, &mime_parser.from, prevent_rename).await?;

    let incoming = from_id != DC_CONTACT_ID_SELF;

    let mut to_ids = ContactIds::new();

    to_ids.extend(
        &dc_add_or_lookup_contacts_by_address_list(
            context,
            &mime_parser.recipients,
            if !incoming {
                Origin::OutgoingTo
            } else if incoming_origin.is_known() {
                Origin::IncomingTo
            } else {
                Origin::IncomingUnknownTo
            },
            prevent_rename,
        )
        .await?,
    );

    // Add parts

    let rfc724_mid = match mime_parser.get_rfc724_mid() {
        Some(x) => x,
        None => {
            // missing Message-IDs may come if the mail was set from this account with another
            // client that relies in the SMTP server to generate one.
            // true eg. for the Webmailer used in all-inkl-KAS
            match dc_create_incoming_rfc724_mid(sent_timestamp, from_id, &to_ids) {
                Some(x) => x,
                None => {
                    bail!("No Message-Id found and could not create incoming rfc724_mid");
                }
            }
        }
    };
    if mime_parser.parts.last().is_some() {
        if let Err(err) = add_parts(
            context,
            &mut mime_parser,
            imf_raw,
            incoming,
            incoming_origin,
            server_folder.as_ref(),
            server_uid,
            &to_ids,
            &rfc724_mid,
            &mut sent_timestamp,
            from_id,
            &mut hidden,
            &mut chat_id,
            seen,
            &mut needs_delete_job,
            &mut insert_msg_id,
            &mut created_db_entries,
            &mut create_event_to_send,
            fetching_existing_messages,
            prevent_rename,
        )
        .await
        {
            cleanup(context, &create_event_to_send, created_db_entries);
            bail!("add_parts error: {:?}", err);
        }
    } else {
        // there are parts in this message, do some basic calculations so that the variables
        // are correct in the further processing
        if sent_timestamp > time() {
            sent_timestamp = time()
        }
    }

    if mime_parser.location_kml.is_some() || mime_parser.message_kml.is_some() {
        save_locations(
            context,
            &mime_parser,
            chat_id,
            from_id,
            insert_msg_id,
            hidden,
        )
        .await;
    }

    if let Some(avatar_action) = &mime_parser.user_avatar {
        match contact::set_profile_image(
            context,
            from_id,
            avatar_action,
            mime_parser.was_encrypted(),
        )
        .await
        {
            Ok(()) => {
                context.emit_event(EventType::ChatModified(chat_id));
            }
            Err(err) => {
                warn!(context, "reveive_imf cannot update profile image: {}", err);
            }
        };
    }

    // Always update the status, even if there is no footer, to allow removing the status.
    if let Err(err) = contact::set_status(
        context,
        from_id,
        mime_parser.footer.clone().unwrap_or_default(),
    )
    .await
    {
        warn!(context, "cannot update contact status: {}", err);
    }

    // Get user-configured server deletion
    let delete_server_after = context.get_config_delete_server_after().await;

    if !created_db_entries.is_empty() {
        if needs_delete_job || delete_server_after == Some(0) {
            for db_entry in &created_db_entries {
                job::add(
                    context,
                    job::Job::new(
                        Action::DeleteMsgOnImap,
                        db_entry.1.to_u32(),
                        Params::new(),
                        0,
                    ),
                )
                .await;
            }
        } else if insert_msg_id
            .needs_move(context, server_folder.as_ref())
            .await
            .unwrap_or_default()
            .is_some()
        {
            // Move message if we don't delete it immediately.
            job::add(
                context,
                job::Job::new(Action::MoveMsg, insert_msg_id.to_u32(), Params::new(), 0),
            )
            .await;
        } else if !mime_parser.mdn_reports.is_empty() && mime_parser.has_chat_version() {
            // This is a Delta Chat MDN. Mark as read.
            job::add(
                context,
                job::Job::new(
                    Action::MarkseenMsgOnImap,
                    insert_msg_id.to_u32(),
                    Params::new(),
                    0,
                ),
            )
            .await;
        }
    }

    info!(
        context,
        "received message {} has Message-Id: {}", server_uid, rfc724_mid
    );

    cleanup(context, &create_event_to_send, created_db_entries);

    mime_parser
        .handle_reports(context, from_id, sent_timestamp, &mime_parser.parts)
        .await;

    Ok(())
}

/// Converts "From" field to contact id.
///
/// Also returns whether it is blocked or not and its origin.
///
/// * `prevent_rename`: passed through to `dc_add_or_lookup_contacts_by_address_list()`
pub async fn from_field_to_contact_id(
    context: &Context,
    from_address_list: &[SingleInfo],
    prevent_rename: bool,
) -> Result<(u32, bool, Origin)> {
    let from_ids = dc_add_or_lookup_contacts_by_address_list(
        context,
        from_address_list,
        Origin::IncomingUnknownFrom,
        prevent_rename,
    )
    .await?;

    if from_ids.contains(&DC_CONTACT_ID_SELF) {
        Ok((DC_CONTACT_ID_SELF, false, Origin::OutgoingBcc))
    } else if !from_ids.is_empty() {
        if from_ids.len() > 1 {
            warn!(
                context,
                "mail has more than one From address, only using first: {:?}", from_address_list
            );
        }
        let from_id = from_ids.get_index(0).cloned().unwrap_or_default();

        let mut from_id_blocked = false;
        let mut incoming_origin = Origin::Unknown;
        if let Ok(contact) = Contact::load_from_db(context, from_id).await {
            from_id_blocked = contact.blocked;
            incoming_origin = contact.origin;
        }
        Ok((from_id, from_id_blocked, incoming_origin))
    } else {
        warn!(
            context,
            "mail has an empty From header: {:?}", from_address_list
        );
        // if there is no from given, from_id stays 0 which is just fine. These messages
        // are very rare, however, we have to add them to the database (they go to the
        // "deaddrop" chat) to avoid a re-download from the server. See also [**]

        Ok((0, false, Origin::Unknown))
    }
}

#[allow(clippy::too_many_arguments, clippy::cognitive_complexity)]
async fn add_parts(
    context: &Context,
    mut mime_parser: &mut MimeMessage,
    imf_raw: &[u8],
    incoming: bool,
    incoming_origin: Origin,
    server_folder: impl AsRef<str>,
    server_uid: u32,
    to_ids: &ContactIds,
    rfc724_mid: &str,
    sent_timestamp: &mut i64,
    from_id: u32,
    hidden: &mut bool,
    chat_id: &mut ChatId,
    seen: bool,
    needs_delete_job: &mut bool,
    insert_msg_id: &mut MsgId,
    created_db_entries: &mut Vec<(ChatId, MsgId)>,
    create_event_to_send: &mut Option<CreateEvent>,
    fetching_existing_messages: bool,
    prevent_rename: bool,
) -> Result<()> {
    let mut state: MessageState;
    let mut chat_id_blocked = Blocked::Not;
    let mut mime_in_reply_to = String::new();
    let mut mime_references = String::new();
    let mut incoming_origin = incoming_origin;

    // check, if the mail is already in our database - if so, just update the folder/uid
    // (if the mail was moved around) and finish. (we may get a mail twice eg. if it is
    // moved between folders. make sure, this check is done eg. before securejoin-processing) */
    if let Some((old_server_folder, old_server_uid, _)) =
        message::rfc724_mid_exists(context, rfc724_mid).await?
    {
        if old_server_folder != server_folder.as_ref() || old_server_uid != server_uid {
            message::update_server_uid(context, rfc724_mid, server_folder.as_ref(), server_uid)
                .await;
        }

        warn!(context, "Message already in DB");
        return Ok(());
    }

    let parent = get_parent_message(context, mime_parser).await?;

    let mut is_dc_message = if mime_parser.has_chat_version() {
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
    let mut allow_creation = true;
    let show_emails =
        ShowEmails::from_i32(context.get_config_int(Config::ShowEmails).await).unwrap_or_default();
    if mime_parser.is_system_message != SystemMessage::AutocryptSetupMessage
        && is_dc_message == MessengerMessage::No
    {
        // this message is a classic email not a chat-message nor a reply to one
        match show_emails {
            ShowEmails::Off => {
                info!(context, "Classical email not shown (TRASH)");
                *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
                allow_creation = false;
            }
            ShowEmails::AcceptedContacts => allow_creation = false,
            ShowEmails::All => {}
        }
    }

    // check if the message introduces a new chat:
    // - outgoing messages introduce a chat with the first to: address if they are sent by a messenger
    // - incoming messages introduce a chat only for known contacts if they are sent by a messenger
    // (of course, the user can add other chats manually later)
    let to_id: u32;

    if incoming {
        state = if seen || fetching_existing_messages {
            MessageState::InSeen
        } else {
            MessageState::InFresh
        };
        to_id = DC_CONTACT_ID_SELF;

        // handshake may mark contacts as verified and must be processed before chats are created
        if mime_parser.get(HeaderDef::SecureJoin).is_some() {
            is_dc_message = MessengerMessage::Yes; // avoid discarding by show_emails setting
            *chat_id = ChatId::new(0);
            allow_creation = true;
            match handle_securejoin_handshake(context, mime_parser, from_id).await {
                Ok(securejoin::HandshakeMessage::Done) => {
                    *hidden = true;
                    *needs_delete_job = true;
                    state = MessageState::InSeen;
                }
                Ok(securejoin::HandshakeMessage::Ignore) => {
                    *hidden = true;
                    state = MessageState::InSeen;
                }
                Ok(securejoin::HandshakeMessage::Propagate) => {
                    // process messages as "member added" normally
                }
                Err(err) => {
                    *hidden = true;
                    warn!(context, "Error in Secure-Join message handling: {}", err);
                    return Ok(());
                }
            }
        }

        let (test_normal_chat_id, test_normal_chat_id_blocked) =
            chat::lookup_by_contact_id(context, from_id)
                .await
                .unwrap_or_default();

        // get the chat_id - a chat_id here is no indicator that the chat is displayed in the normal list,
        // it might also be blocked and displayed in the deaddrop as a result
        if chat_id.is_unset() && mime_parser.failure_report.is_some() {
            *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
            info!(context, "Message belongs to an NDN (TRASH)",);
        }

        if chat_id.is_unset() {
            // try to create a group

            let create_blocked =
                if !test_normal_chat_id.is_unset() && test_normal_chat_id_blocked == Blocked::Not {
                    Blocked::Not
                } else {
                    Blocked::Deaddrop
                };

            let (new_chat_id, new_chat_id_blocked) = create_or_lookup_group(
                context,
                &mut mime_parser,
                if test_normal_chat_id.is_unset() {
                    allow_creation
                } else {
                    true
                },
                create_blocked,
                from_id,
                to_ids,
            )
            .await?;
            *chat_id = new_chat_id;
            chat_id_blocked = new_chat_id_blocked;
            if !chat_id.is_unset()
                && chat_id_blocked != Blocked::Not
                && create_blocked == Blocked::Not
            {
                new_chat_id.unblock(context).await;
                chat_id_blocked = Blocked::Not;
            }
        }

        if chat_id.is_unset() {
            // check if the message belongs to a mailing list
            match mime_parser.get_mailinglist_type() {
                MailinglistType::ListIdBased => {
                    if let Some(list_id) = mime_parser.get(HeaderDef::ListId) {
                        let (new_chat_id, new_chat_id_blocked) = create_or_lookup_mailinglist(
                            context,
                            allow_creation,
                            list_id,
                            mime_parser,
                        )
                        .await;
                        *chat_id = new_chat_id;
                        chat_id_blocked = new_chat_id_blocked;
                    }
                }
                MailinglistType::SenderBased => {
                    if let Some(sender) = mime_parser.get(HeaderDef::Sender) {
                        let (new_chat_id, new_chat_id_blocked) = create_or_lookup_mailinglist(
                            context,
                            allow_creation,
                            sender,
                            mime_parser,
                        )
                        .await;
                        *chat_id = new_chat_id;
                        chat_id_blocked = new_chat_id_blocked;
                    }
                }
                MailinglistType::None => {}
            }
        }

        // if contact renaming is prevented (for mailinglists and bots),
        // we use name from From:-header as override name
        if prevent_rename {
            if let Some(from) = mime_parser.from.first() {
                if let Some(name) = &from.display_name {
                    for part in mime_parser.parts.iter_mut() {
                        part.param.set(Param::OverrideSenderDisplayname, name);
                    }
                }
            }
        }

        if chat_id.is_unset() {
            // try to create a normal chat
            let create_blocked = if from_id == to_id {
                Blocked::Not
            } else {
                Blocked::Deaddrop
            };

            if !test_normal_chat_id.is_unset() {
                *chat_id = test_normal_chat_id;
                chat_id_blocked = test_normal_chat_id_blocked;
            } else if allow_creation {
                let (id, bl) =
                    chat::create_or_lookup_by_contact_id(context, from_id, create_blocked)
                        .await
                        .unwrap_or_default();
                *chat_id = id;
                chat_id_blocked = bl;
            }
            if !chat_id.is_unset() && Blocked::Not != chat_id_blocked {
                if Blocked::Not == create_blocked {
                    chat_id.unblock(context).await;
                    chat_id_blocked = Blocked::Not;
                } else if parent.is_some() {
                    // we do not want any chat to be created implicitly.  Because of the origin-scale-up,
                    // the contact requests will pop up and this should be just fine.
                    Contact::scaleup_origin_by_id(context, from_id, Origin::IncomingReplyTo).await;
                    info!(
                        context,
                        "Message is a reply to a known message, mark sender as known.",
                    );
                    if !incoming_origin.is_known() {
                        incoming_origin = Origin::IncomingReplyTo;
                    }
                }
            }
        }
        if chat_id.is_unset() {
            // maybe from_id is null or sth. else is suspicious, move message to trash
            *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
            info!(context, "No chat id for incoming msg (TRASH)")
        }

        // if the chat_id is blocked,
        // for unknown senders and non-delta-messages set the state to NOTICED
        // to not result in a chatlist-contact-request (this would require the state FRESH)
        if Blocked::Not != chat_id_blocked
            && state == MessageState::InFresh
            && !incoming_origin.is_known()
            && is_dc_message == MessengerMessage::No
            && show_emails != ShowEmails::All
        {
            state = MessageState::InNoticed;
        } else if fetching_existing_messages && Blocked::Deaddrop == chat_id_blocked {
            // The fetched existing message should be shown in the chatlist-contact-request because
            // a new user won't find the contact request in the menu
            state = MessageState::InFresh;
        }
    } else {
        // Outgoing

        // the mail is on the IMAP server, probably it is also delivered.
        // We cannot recreate other states (read, error).
        state = MessageState::OutDelivered;
        to_id = to_ids.get_index(0).cloned().unwrap_or_default();

        // handshake may mark contacts as verified and must be processed before chats are created
        if mime_parser.get(HeaderDef::SecureJoin).is_some() {
            is_dc_message = MessengerMessage::Yes; // avoid discarding by show_emails setting
            *chat_id = ChatId::new(0);
            allow_creation = true;
            match observe_securejoin_on_other_device(context, mime_parser, to_id).await {
                Ok(securejoin::HandshakeMessage::Done)
                | Ok(securejoin::HandshakeMessage::Ignore) => {
                    *hidden = true;
                }
                Ok(securejoin::HandshakeMessage::Propagate) => {
                    // process messages as "member added" normally
                }
                Err(err) => {
                    *hidden = true;
                    warn!(context, "Error in Secure-Join watching: {}", err);
                    return Ok(());
                }
            }
        }

        if !context.is_sentbox(&server_folder).await
            && mime_parser.get(HeaderDef::Received).is_none()
        {
            // Most mailboxes have a "Drafts" folder where constantly new emails appear but we don't actually want to show them
            // So: If it's outgoing AND there is no Received header AND it's not in the sentbox, then ignore the email.
            info!(context, "Email is probably just a draft (TRASH)");
            *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
            allow_creation = false;
        }

        if !to_ids.is_empty() {
            if chat_id.is_unset() {
                let (new_chat_id, new_chat_id_blocked) = create_or_lookup_group(
                    context,
                    &mut mime_parser,
                    allow_creation,
                    Blocked::Not,
                    from_id,
                    to_ids,
                )
                .await?;
                *chat_id = new_chat_id;
                chat_id_blocked = new_chat_id_blocked;
                // automatically unblock chat when the user sends a message
                if !chat_id.is_unset() && chat_id_blocked != Blocked::Not {
                    new_chat_id.unblock(context).await;
                    chat_id_blocked = Blocked::Not;
                }
            }
            if chat_id.is_unset() && allow_creation {
                let create_blocked = if MessengerMessage::No != is_dc_message
                    && !Contact::is_blocked_load(context, to_id).await
                {
                    Blocked::Not
                } else {
                    Blocked::Deaddrop
                };
                let (id, bl) = chat::create_or_lookup_by_contact_id(context, to_id, create_blocked)
                    .await
                    .unwrap_or_default();
                *chat_id = id;
                chat_id_blocked = bl;

                if !chat_id.is_unset()
                    && Blocked::Not != chat_id_blocked
                    && Blocked::Not == create_blocked
                {
                    chat_id.unblock(context).await;
                    chat_id_blocked = Blocked::Not;
                }
            }
        }
        let self_sent = from_id == DC_CONTACT_ID_SELF
            && to_ids.len() == 1
            && to_ids.contains(&DC_CONTACT_ID_SELF);

        if chat_id.is_unset() && self_sent {
            // from_id==to_id==DC_CONTACT_ID_SELF - this is a self-sent messages,
            // maybe an Autocrypt Setup Message
            let (id, bl) =
                chat::create_or_lookup_by_contact_id(context, DC_CONTACT_ID_SELF, Blocked::Not)
                    .await
                    .unwrap_or_default();
            *chat_id = id;
            chat_id_blocked = bl;

            if !chat_id.is_unset() && Blocked::Not != chat_id_blocked {
                chat_id.unblock(context).await;
                chat_id_blocked = Blocked::Not;
            }
        }
        if chat_id.is_unset() {
            *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
            info!(context, "No chat id for outgoing message (TRASH)")
        }
    }

    if fetching_existing_messages && mime_parser.decrypting_failed {
        *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
        // We are only gathering old messages on first start. We do not want to add loads of non-decryptable messages to the chats.
        info!(context, "Existing non-decipherable message. (TRASH)");
    }

    // Extract ephemeral timer from the message.
    let mut ephemeral_timer = if let Some(value) = mime_parser.get(HeaderDef::EphemeralTimer) {
        match value.parse::<EphemeralTimer>() {
            Ok(timer) => timer,
            Err(err) => {
                warn!(
                    context,
                    "can't parse ephemeral timer \"{}\": {}", value, err
                );
                EphemeralTimer::Disabled
            }
        }
    } else {
        EphemeralTimer::Disabled
    };

    let location_kml_is = mime_parser.location_kml.is_some();
    let is_mdn = !mime_parser.mdn_reports.is_empty();

    // Apply ephemeral timer changes to the chat.
    //
    // Only non-hidden timers are applied now. Timers from hidden
    // messages such as read receipts can be useful to detect
    // ephemeral timer support, but timer changes without visible
    // received messages may be confusing to the user.
    if !*hidden
        && !location_kml_is
        && !is_mdn
        && (is_dc_message != MessengerMessage::Yes
            || parent.is_none()
            || parent.unwrap().ephemeral_timer != ephemeral_timer)
        && (*chat_id).get_ephemeral_timer(context).await? != ephemeral_timer
    {
        if let Err(err) = (*chat_id)
            .inner_set_ephemeral_timer(context, ephemeral_timer)
            .await
        {
            warn!(
                context,
                "failed to modify timer for chat {}: {}", chat_id, err
            );
        } else if mime_parser.is_system_message != SystemMessage::EphemeralTimerChanged {
            chat::add_info_msg(
                context,
                *chat_id,
                stock_ephemeral_timer_changed(context, ephemeral_timer, from_id).await,
            )
            .await;
        }
    }

    if mime_parser.is_system_message == SystemMessage::EphemeralTimerChanged {
        set_better_msg(
            mime_parser,
            stock_ephemeral_timer_changed(context, ephemeral_timer, from_id).await,
        );

        // Do not delete the system message itself.
        //
        // This prevents confusion when timer is changed
        // to 1 week, and then changed to 1 hour: after 1
        // hour, only the message about the change to 1
        // week is left.
        ephemeral_timer = EphemeralTimer::Disabled;
    }

    // if a chat is protected, check additional properties
    if !chat_id.is_special() {
        let chat = Chat::load_from_db(context, *chat_id).await?;
        let new_status = match mime_parser.is_system_message {
            SystemMessage::ChatProtectionEnabled => Some(ProtectionStatus::Protected),
            SystemMessage::ChatProtectionDisabled => Some(ProtectionStatus::Unprotected),
            _ => None,
        };

        if chat.is_protected() || new_status.is_some() {
            if let Err(err) =
                check_verified_properties(context, mime_parser, from_id as u32, to_ids).await
            {
                warn!(context, "verification problem: {}", err);
                let s = format!("{}. See 'Info' for more details", err);
                mime_parser.repl_msg_by_error(s);
            } else {
                // change chat protection only when verification check passes
                if let Some(new_status) = new_status {
                    if let Err(e) = chat_id.inner_set_protection(context, new_status).await {
                        chat::add_info_msg(
                            context,
                            *chat_id,
                            format!("Cannot set protection: {}", e),
                        )
                        .await;
                        return Ok(()); // do not return an error as this would result in retrying the message
                    }
                    set_better_msg(
                        mime_parser,
                        context.stock_protection_msg(new_status, from_id).await,
                    );
                }
            }
        }
    }

    // correct message_timestamp, it should not be used before,
    // however, we cannot do this earlier as we need from_id to be set
    let in_fresh = state == MessageState::InFresh;
    let rcvd_timestamp = time();
    let sort_timestamp = calc_sort_timestamp(context, *sent_timestamp, *chat_id, in_fresh).await;

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

    *sent_timestamp = std::cmp::min(*sent_timestamp, rcvd_timestamp);

    // if the mime-headers should be saved, find out its size
    // (the mime-header ends with an empty line)
    let save_mime_headers = context.get_config_bool(Config::SaveMimeHeaders).await;
    if let Some(raw) = mime_parser.get(HeaderDef::InReplyTo) {
        mime_in_reply_to = raw.clone();
    }

    if let Some(raw) = mime_parser.get(HeaderDef::References) {
        mime_references = raw.clone();
    }

    // fine, so far.  now, split the message into simple parts usable as "short messages"
    // and add them to the database (mails sent by other messenger clients should result
    // into only one message; mails sent by other clients may result in several messages
    // (eg. one per attachment))
    let icnt = mime_parser.parts.len();

    let subject = mime_parser.get_subject().unwrap_or_default();

    let mut parts = std::mem::replace(&mut mime_parser.parts, Vec::new());
    let server_folder = server_folder.as_ref().to_string();
    let is_system_message = mime_parser.is_system_message;

    // if indicated by the parser,
    // we save the full mime-message and add a flag
    // that the ui should show button to display the full message.

    // a flag used to avoid adding "show full message" button to multiple parts of the message.
    let mut save_mime_modified = mime_parser.is_mime_modified;

    let mime_headers = if save_mime_headers || save_mime_modified {
        if mime_parser.was_encrypted() {
            Some(String::from_utf8_lossy(&mime_parser.decoded_data).to_string())
        } else {
            Some(String::from_utf8_lossy(imf_raw).to_string())
        }
    } else {
        None
    };

    let sent_timestamp = *sent_timestamp;
    let is_hidden = *hidden;
    let chat_id = *chat_id;

    // TODO: can this clone be avoided?
    let rfc724_mid = rfc724_mid.to_string();

    let (new_parts, ids, is_hidden) = context
        .sql
        .with_conn(move |mut conn| {
            let mut ids = Vec::with_capacity(parts.len());
            let mut is_hidden = is_hidden;

            for part in &mut parts {
                let mut txt_raw = "".to_string();
                let mut stmt = conn.prepare_cached(
                    "INSERT INTO msgs \
         (rfc724_mid, server_folder, server_uid, chat_id, from_id, to_id, timestamp, \
         timestamp_sent, timestamp_rcvd, type, state, msgrmsg,  txt, txt_raw, param, \
         bytes, hidden, mime_headers,  mime_in_reply_to, mime_references, mime_modified, \
         error, ephemeral_timer, ephemeral_timestamp) \
         VALUES (?,?,?,?,?,?,?, ?,?,?,?,?,?,?,?, ?,?,?,?,?,?, ?,?,?);",
                )?;

                let is_location_kml = location_kml_is
                    && icnt == 1
                    && (part.msg == "-location-" || part.msg.is_empty());

                if is_mdn || is_location_kml {
                    is_hidden = true;
                    if incoming {
                        state = MessageState::InSeen; // Set the state to InSeen so that precheck_imf() adds a markseen job after we moved the message
                    }
                }

                let mime_modified = save_mime_modified && !part.msg.is_empty();
                if mime_modified {
                    // Avoid setting mime_modified for more than one part.
                    save_mime_modified = false;
                }

                if part.typ == Viewtype::Text {
                    let msg_raw = part.msg_raw.as_ref().cloned().unwrap_or_default();
                    txt_raw = format!("{}\n\n{}", subject, msg_raw);
                }
                if is_system_message != SystemMessage::Unknown {
                    part.param.set_int(Param::Cmd, is_system_message as i32);
                }

                let ephemeral_timestamp = if in_fresh {
                    0
                } else {
                    match ephemeral_timer {
                        EphemeralTimer::Disabled => 0,
                        EphemeralTimer::Enabled { duration } => {
                            rcvd_timestamp + i64::from(duration)
                        }
                    }
                };

                stmt.execute(paramsv![
                    rfc724_mid,
                    server_folder,
                    server_uid as i32,
                    chat_id,
                    from_id as i32,
                    to_id as i32,
                    sort_timestamp,
                    sent_timestamp,
                    rcvd_timestamp,
                    part.typ,
                    state,
                    is_dc_message,
                    part.msg,
                    // txt_raw might contain invalid utf8
                    txt_raw,
                    part.param.to_string(),
                    part.bytes as isize,
                    is_hidden,
                    if save_mime_headers || mime_modified {
                        mime_headers.clone()
                    } else {
                        None
                    },
                    mime_in_reply_to,
                    mime_references,
                    mime_modified,
                    part.error.take().unwrap_or_default(),
                    ephemeral_timer,
                    ephemeral_timestamp
                ])?;

                drop(stmt);
                ids.push(MsgId::new(crate::sql::get_rowid(
                    &mut conn,
                    "msgs",
                    "rfc724_mid",
                    &rfc724_mid,
                )?));
            }
            Ok((parts, ids, is_hidden))
        })
        .await?;

    if let Some(id) = ids.iter().last() {
        *insert_msg_id = *id;
    }

    if !is_hidden {
        chat_id.unarchive(context).await?;
    }

    *hidden = is_hidden;
    created_db_entries.extend(ids.iter().map(|id| (chat_id, *id)));
    mime_parser.parts = new_parts;

    info!(
        context,
        "Message has {} parts and is assigned to chat #{}.", icnt, chat_id,
    );

    // new outgoing message from another device marks the chat as noticed.
    if !incoming && !*hidden && !chat_id.is_special() {
        chat::marknoticed_chat_if_older_than(context, chat_id, sort_timestamp).await?;
    }

    // check event to send
    if chat_id.is_trash() || *hidden {
        *create_event_to_send = None;
    } else if incoming && state == MessageState::InFresh {
        if Blocked::Not != chat_id_blocked {
            *create_event_to_send = Some(CreateEvent::MsgsChanged);
        } else {
            *create_event_to_send = Some(CreateEvent::IncomingMsg);
        }
    }

    async fn update_last_subject(
        context: &Context,
        chat_id: ChatId,
        mime_parser: &MimeMessage,
    ) -> Result<()> {
        let mut chat = Chat::load_from_db(context, chat_id).await?;
        chat.param.set(
            Param::LastSubject,
            mime_parser
                .get_subject()
                .ok_or_else(|| format_err!("No subject in email"))?,
        );
        chat.update_param(context).await?;
        Ok(())
    }
    if !is_mdn {
        update_last_subject(context, chat_id, mime_parser)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    context,
                    "Could not update LastSubject of chat: {}",
                    e.to_string()
                )
            });
    }

    Ok(())
}

async fn save_locations(
    context: &Context,
    mime_parser: &MimeMessage,
    chat_id: ChatId,
    from_id: u32,
    insert_msg_id: MsgId,
    hidden: bool,
) {
    if chat_id.is_special() {
        return;
    }
    let mut location_id_written = false;
    let mut send_event = false;

    if mime_parser.message_kml.is_some() {
        let locations = &mime_parser.message_kml.as_ref().unwrap().locations;
        let newest_location_id = location::save(context, chat_id, from_id, locations, true)
            .await
            .unwrap_or_default();
        if 0 != newest_location_id
            && !hidden
            && location::set_msg_location_id(context, insert_msg_id, newest_location_id)
                .await
                .is_ok()
        {
            location_id_written = true;
            send_event = true;
        }
    }

    if mime_parser.location_kml.is_some() {
        if let Some(ref addr) = mime_parser.location_kml.as_ref().unwrap().addr {
            if let Ok(contact) = Contact::get_by_id(context, from_id).await {
                if contact.get_addr().to_lowercase() == addr.to_lowercase() {
                    let locations = &mime_parser.location_kml.as_ref().unwrap().locations;
                    let newest_location_id =
                        location::save(context, chat_id, from_id, locations, false)
                            .await
                            .unwrap_or_default();
                    if newest_location_id != 0 && !hidden && !location_id_written {
                        if let Err(err) = location::set_msg_location_id(
                            context,
                            insert_msg_id,
                            newest_location_id,
                        )
                        .await
                        {
                            error!(context, "Failed to set msg_location_id: {:?}", err);
                        }
                    }
                    send_event = true;
                }
            }
        }
    }
    if send_event {
        context.emit_event(EventType::LocationChanged(Some(from_id)));
    }
}

async fn calc_sort_timestamp(
    context: &Context,
    message_timestamp: i64,
    chat_id: ChatId,
    is_fresh_msg: bool,
) -> i64 {
    let mut sort_timestamp = message_timestamp;

    // get newest non fresh message for this chat
    // update sort_timestamp if less than that
    if is_fresh_msg {
        let last_msg_time: Option<i64> = context
            .sql
            .query_get_value(
                context,
                "SELECT MAX(timestamp) FROM msgs WHERE chat_id=? AND state>?",
                paramsv![chat_id, MessageState::InFresh],
            )
            .await;

        if let Some(last_msg_time) = last_msg_time {
            if last_msg_time > sort_timestamp {
                sort_timestamp = last_msg_time;
            }
        }
    }

    if sort_timestamp >= dc_smeared_time(context).await {
        sort_timestamp = dc_create_smeared_timestamp(context).await;
    }

    sort_timestamp
}

/// This function tries to extract the group-id from the message and returns the
/// corresponding chat_id. If the chat does not exist, it is created.
/// If the message contains groups commands (name, profile image, changed members),
/// they are executed as well.
///
/// If no group-id could be extracted, message is assigned to the same chat as the
/// parent message.
///
/// If there is no parent message in the database, and there are more than two members,
/// a new ad-hoc group is created.
///
/// On success the function returns the found/created (chat_id, chat_blocked) tuple.
#[allow(non_snake_case, clippy::cognitive_complexity)]
async fn create_or_lookup_group(
    context: &Context,
    mime_parser: &mut MimeMessage,
    allow_creation: bool,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &ContactIds,
) -> Result<(ChatId, Blocked)> {
    let mut chat_id_blocked = Blocked::Not;
    let mut recreate_member_list = false;
    let mut send_EVENT_CHAT_MODIFIED = false;
    let mut X_MrAddToGrp = None;
    let mut X_MrGrpNameChanged = false;
    let mut better_msg: String = From::from("");

    if mime_parser.is_system_message == SystemMessage::LocationStreamingEnabled {
        better_msg = stock_str::msg_location_enabled_by(context, from_id).await;
        set_better_msg(mime_parser, &better_msg);
    }

    let grpid = if let Some(grpid) = try_getting_grpid(mime_parser) {
        grpid
    } else {
        let mut member_ids: Vec<u32> = to_ids.iter().copied().collect();
        if !member_ids.contains(&from_id) {
            member_ids.push(from_id);
        }
        if !member_ids.contains(&DC_CONTACT_ID_SELF) {
            member_ids.push(DC_CONTACT_ID_SELF);
        }

        // Try to assign message to the same group as the parent message.
        //
        // We don't do this for chat messages to ensure private replies to group messages, which
        // have In-Reply-To and quote but no Chat-Group-ID header, are assigned to 1:1 chat.
        // Chat messages should always include explicit group ID in group messages.
        if mime_parser.get(HeaderDef::ChatVersion).is_none() {
            if let Some(parent) = get_parent_message(context, mime_parser).await? {
                let chat = Chat::load_from_db(context, parent.chat_id).await?;

                // Check that destination chat is a group chat.
                // Otherwise, it could be a reply to an undecipherable
                // group message that we previously assigned to a 1:1 chat.
                if chat.typ == Chattype::Group {
                    // Return immediately without attempting to execute group commands,
                    // as this message does not contain an explicit group-id header.
                    return Ok((chat.id, chat.blocked));
                }
            }
        }

        if !allow_creation {
            info!(context, "creating ad-hoc group prevented from caller");
            return Ok((ChatId::new(0), Blocked::Not));
        }

        return create_adhoc_group(context, mime_parser, create_blocked, &member_ids)
            .await
            .map(|chat_id| {
                chat_id
                    .map(|chat_id| (chat_id, create_blocked))
                    .unwrap_or((ChatId::new(0), Blocked::Not))
            })
            .map_err(|err| {
                info!(context, "could not create adhoc-group: {:?}", err);
                err
            });
    };

    // now we have a grpid that is non-empty
    // but we might not know about this group

    let grpname = mime_parser.get(HeaderDef::ChatGroupName).cloned();
    let mut removed_id = None;

    if let Some(removed_addr) = mime_parser.get(HeaderDef::ChatGroupMemberRemoved).cloned() {
        removed_id = Contact::lookup_id_by_addr(context, &removed_addr, Origin::Unknown).await?;
        match removed_id {
            Some(contact_id) => {
                mime_parser.is_system_message = SystemMessage::MemberRemovedFromGroup;
                better_msg = if contact_id == from_id {
                    stock_str::msg_group_left(context, from_id).await
                } else {
                    stock_str::msg_del_member(context, &removed_addr, from_id).await
                };
            }
            None => warn!(context, "removed {:?} has no contact_id", removed_addr),
        }
    } else {
        let field = mime_parser.get(HeaderDef::ChatGroupMemberAdded).cloned();
        if let Some(added_member) = field {
            mime_parser.is_system_message = SystemMessage::MemberAddedToGroup;
            better_msg = stock_str::msg_add_member(context, &added_member, from_id).await;
            X_MrAddToGrp = Some(added_member);
        } else if let Some(old_name) = mime_parser.get(HeaderDef::ChatGroupNameChanged) {
            X_MrGrpNameChanged = true;
            better_msg = stock_str::msg_grp_name(
                context,
                old_name,
                if let Some(ref name) = grpname {
                    name
                } else {
                    ""
                },
                from_id as u32,
            )
            .await;
            mime_parser.is_system_message = SystemMessage::GroupNameChanged;
        } else if let Some(value) = mime_parser.get(HeaderDef::ChatContent) {
            if value == "group-avatar-changed" {
                if let Some(avatar_action) = &mime_parser.group_avatar {
                    // this is just an explicit message containing the group-avatar,
                    // apart from that, the group-avatar is send along with various other messages
                    mime_parser.is_system_message = SystemMessage::GroupImageChanged;
                    better_msg = match avatar_action {
                        AvatarAction::Delete => {
                            stock_str::msg_grp_img_deleted(context, from_id).await
                        }
                        AvatarAction::Change(_) => {
                            stock_str::msg_grp_img_changed(context, from_id).await
                        }
                    };
                }
            }
        }
    }
    set_better_msg(mime_parser, &better_msg);

    // check, if we have a chat with this group ID
    let (mut chat_id, _, _blocked) = chat::get_chat_id_by_grpid(context, &grpid)
        .await
        .unwrap_or((ChatId::new(0), false, Blocked::Not));
    if !chat_id.is_unset() && !chat::is_contact_in_chat(context, chat_id, from_id as u32).await {
        // The From-address is not part of this group.
        // It could be a new user or a DSN from a mailer-daemon.
        // in any case we do not want to recreate the member list
        // but still show the message as part of the chat.
        // After all, the sender has a reference/in-reply-to that
        // points to this chat.
        let s = stock_str::unknown_sender_for_chat(context).await;
        mime_parser.repl_msg_by_error(s);
    }

    // check if the group does not exist but should be created
    let group_explicitly_left = chat::is_group_explicitly_left(context, &grpid)
        .await
        .unwrap_or_default();
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .await
        .unwrap_or_default();

    if chat_id.is_unset()
            && !mime_parser.is_mailinglist_message()
            && !grpid.is_empty()
            && grpname.is_some()
            // otherwise, a pending "quit" message may pop up
            && removed_id.is_none()
            // re-create explicitly left groups only if ourself is re-added
            && (!group_explicitly_left
                || X_MrAddToGrp.is_some() && addr_cmp(&self_addr, X_MrAddToGrp.as_ref().unwrap()))
    {
        // group does not exist but should be created
        let create_protected = if mime_parser.get(HeaderDef::ChatVerified).is_some() {
            if let Err(err) =
                check_verified_properties(context, mime_parser, from_id as u32, to_ids).await
            {
                warn!(context, "verification problem: {}", err);
                let s = format!("{}. See 'Info' for more details", err);
                mime_parser.repl_msg_by_error(&s);
            }
            ProtectionStatus::Protected
        } else {
            ProtectionStatus::Unprotected
        };

        if !allow_creation {
            info!(context, "creating group forbidden by caller");
            return Ok((ChatId::new(0), Blocked::Not));
        }

        chat_id = create_multiuser_record(
            context,
            Chattype::Group,
            &grpid,
            grpname.as_ref().unwrap(),
            create_blocked,
            create_protected,
        )
        .await?;
        chat_id_blocked = create_blocked;
        recreate_member_list = true;

        // once, we have protected-chats explained in UI, we can uncomment the following lines.
        // ("verified groups" did not add a message anyway)
        //
        //if create_protected == ProtectionStatus::Protected {
        // set from_id=0 as it is not clear that the sender of this random group message
        // actually really has enabled chat-protection at some point.
        //chat_id
        //    .add_protection_msg(context, ProtectionStatus::Protected, false, 0)
        //    .await?;
        //}
    }

    // again, check chat_id
    if chat_id.is_special() {
        if mime_parser.decrypting_failed {
            // It is possible that the message was sent to a valid,
            // yet unknown group, which was rejected because
            // Chat-Group-Name, which is in the encrypted part, was
            // not found. We can't create a properly named group in
            // this case, so assign error message to 1:1 chat with the
            // sender instead.
            return Ok((ChatId::new(0), Blocked::Not));
        } else {
            // The message was decrypted successfully, but contains a late "quit" or otherwise
            // unwanted message.
            info!(context, "message belongs to unwanted group (TRASH)");
            return Ok((ChatId::new(DC_CHAT_ID_TRASH), chat_id_blocked));
        }
    }

    // We have a valid chat_id > DC_CHAT_ID_LAST_SPECIAL.
    //
    // However, it's possible that we got a non-DC message
    // and the user hit "reply" instead of "reply-all".
    // We heuristically detect this case and show
    // a placeholder-system-message to warn about this
    // and refer to "message-info" to see the message.
    // This is similar to how we show messages arriving
    // in verified chat using an un-verified key or cleartext.

    // XXX insert code in a different PR :)

    // execute group commands
    if X_MrAddToGrp.is_some() {
        recreate_member_list = true;
    } else if X_MrGrpNameChanged {
        if let Some(ref grpname) = grpname {
            if grpname.len() < 200 {
                info!(context, "updating grpname for chat {}", chat_id);
                if context
                    .sql
                    .execute(
                        "UPDATE chats SET name=? WHERE id=?;",
                        paramsv![grpname.to_string(), chat_id],
                    )
                    .await
                    .is_ok()
                {
                    context.emit_event(EventType::ChatModified(chat_id));
                }
            }
        }
    } else if mime_parser.is_system_message == SystemMessage::ChatProtectionEnabled {
        recreate_member_list = true;
    }

    if let Some(avatar_action) = &mime_parser.group_avatar {
        info!(context, "group-avatar change for {}", chat_id);
        if let Ok(mut chat) = Chat::load_from_db(context, chat_id).await {
            match avatar_action {
                AvatarAction::Change(profile_image) => {
                    chat.param.set(Param::ProfileImage, profile_image);
                }
                AvatarAction::Delete => {
                    chat.param.remove(Param::ProfileImage);
                }
            };
            chat.update_param(context).await?;
            send_EVENT_CHAT_MODIFIED = true;
        }
    }

    // add members to group/check members
    if recreate_member_list {
        if !chat::is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF).await {
            // Members could have been removed while we were
            // absent. We can't use existing member list and need to
            // start from scratch.
            context
                .sql
                .execute(
                    "DELETE FROM chats_contacts WHERE chat_id=?;",
                    paramsv![chat_id],
                )
                .await
                .ok();

            chat::add_to_chat_contacts_table(context, chat_id, DC_CONTACT_ID_SELF).await;
        }
        if from_id > DC_CONTACT_ID_LAST_SPECIAL
            && !Contact::addr_equals_contact(context, &self_addr, from_id as u32).await
            && !chat::is_contact_in_chat(context, chat_id, from_id).await
        {
            chat::add_to_chat_contacts_table(context, chat_id, from_id as u32).await;
        }
        for &to_id in to_ids.iter() {
            info!(context, "adding to={:?} to chat id={}", to_id, chat_id);
            if !Contact::addr_equals_contact(context, &self_addr, to_id).await
                && !chat::is_contact_in_chat(context, chat_id, to_id).await
            {
                chat::add_to_chat_contacts_table(context, chat_id, to_id).await;
            }
        }
        send_EVENT_CHAT_MODIFIED = true;
    } else if let Some(contact_id) = removed_id {
        chat::remove_from_chat_contacts_table(context, chat_id, contact_id).await;
        send_EVENT_CHAT_MODIFIED = true;
    }

    if send_EVENT_CHAT_MODIFIED {
        context.emit_event(EventType::ChatModified(chat_id));
    }
    Ok((chat_id, chat_id_blocked))
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
) -> (ChatId, Blocked) {
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

    if let Ok((chat_id, _, blocked)) = chat::get_chat_id_by_grpid(context, &listid).await {
        return (chat_id, blocked);
    }

    // for mailchimp lists, the name in `ListId` is just a long number.
    // a usable name for these lists is in the `From` header
    // and we can detect these lists by a unique `ListId`-suffix.
    if listid.ends_with(".list-id.mcsv.net") {
        if let Some(from) = mime_parser.from.first() {
            if let Some(display_name) = &from.display_name {
                name = display_name.clone();
            }
        }
    }

    // if we have an additional name square brackets in the subject, we prefer that
    // (as that part is much more visible, we assume, that names is shorter and comes more to the point,
    // than the sometimes longer part from ListId)
    let subject = mime_parser.get_subject().unwrap_or_default();
    static SUBJECT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^.{0,5}\[(.*.)\]").unwrap());
    if let Some(cap) = SUBJECT.captures(&subject) {
        name = cap[1].to_string();
    }

    if name.is_empty() {
        name = listid.clone();
    }

    if allow_creation {
        // list does not exist but should be created
        match create_multiuser_record(
            context,
            Chattype::Mailinglist,
            &listid,
            &name,
            Blocked::Deaddrop,
            ProtectionStatus::Unprotected,
        )
        .await
        {
            Ok(chat_id) => {
                chat::add_to_chat_contacts_table(context, chat_id, DC_CONTACT_ID_SELF).await;
                (chat_id, Blocked::Deaddrop)
            }
            Err(e) => {
                warn!(
                    context,
                    "Failed to create mailinglist '{}' for grpid={}: {}",
                    &name,
                    &listid,
                    e.to_string()
                );
                (ChatId::new(0), Blocked::Deaddrop)
            }
        }
    } else {
        info!(context, "creating list forbidden by caller");
        (ChatId::new(0), Blocked::Not)
    }
}

fn try_getting_grpid(mime_parser: &MimeMessage) -> Option<String> {
    if let Some(optional_field) = mime_parser.get(HeaderDef::ChatGroupId) {
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
    let header = mime_parser.get(headerdef)?;
    let parts = header
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    parts.filter_map(dc_extract_grpid_from_rfc724_mid).next()
}

/// Creates ad-hoc group and returns chat ID on success.
async fn create_adhoc_group(
    context: &Context,
    mime_parser: &MimeMessage,
    create_blocked: Blocked,
    member_ids: &[u32],
) -> Result<Option<ChatId>> {
    if mime_parser.is_mailinglist_message() {
        info!(
            context,
            "not creating ad-hoc group for mailing list message"
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
            "not creating ad-hoc group for message that cannot be decrypted"
        );
        return Ok(None);
    }

    if member_ids.len() < 3 {
        info!(context, "not creating ad-hoc group: too few contacts");
        return Ok(None);
    }

    // Create a new ad-hoc group.
    let grpid = create_adhoc_grp_id(context, member_ids).await;

    // use subject as initial chat name
    let grpname = mime_parser
        .get_subject()
        .unwrap_or_else(|| "Unnamed group".to_string());

    let new_chat_id: ChatId = create_multiuser_record(
        context,
        Chattype::Group,
        &grpid,
        grpname,
        create_blocked,
        ProtectionStatus::Unprotected,
    )
    .await?;
    for &member_id in member_ids.iter() {
        chat::add_to_chat_contacts_table(context, new_chat_id, member_id).await;
    }

    context.emit_event(EventType::ChatModified(new_chat_id));

    Ok(Some(new_chat_id))
}

async fn create_multiuser_record(
    context: &Context,
    chattype: Chattype,
    grpid: impl AsRef<str>,
    grpname: impl AsRef<str>,
    create_blocked: Blocked,
    create_protected: ProtectionStatus,
) -> Result<ChatId> {
    context.sql.execute(
        "INSERT INTO chats (type, name, grpid, blocked, created_timestamp, protected) VALUES(?, ?, ?, ?, ?, ?);",
        paramsv![
            chattype,
            grpname.as_ref(),
            grpid.as_ref(),
            create_blocked,
            time(),
            create_protected,
        ],
    ).await?;

    let row_id = context
        .sql
        .get_rowid(context, "chats", "grpid", grpid.as_ref())
        .await?;

    let chat_id = ChatId::new(row_id);
    info!(
        context,
        "Created group/mailinglist '{}' grpid={} as {}",
        grpname.as_ref(),
        grpid.as_ref(),
        chat_id
    );
    Ok(chat_id)
}

/// Creates ad-hoc group ID.
///
/// Algorithm:
/// - sort normalized, lowercased, e-mail addresses alphabetically
/// - put all e-mail addresses into a single string, separate the address by a single comma
/// - sha-256 this string (without possibly terminating null-characters)
/// - encode the first 64 bits of the sha-256 output as lowercase hex (results in 16 characters from the set [0-9a-f])
///
/// This ensures that different Delta Chat clients generate the same group ID unless some of them
/// are hidden in BCC. This group ID is sent by DC in the messages sent to this chat,
/// so having the same ID prevents group split.
async fn create_adhoc_grp_id(context: &Context, member_ids: &[u32]) -> String {
    let member_ids_str = join(member_ids.iter().map(|x| x.to_string()), ",");
    let member_cs = context
        .get_config(Config::ConfiguredAddr)
        .await
        .unwrap_or_else(|| "no-self".to_string())
        .to_lowercase();

    let members = context
        .sql
        .query_map(
            format!(
                "SELECT addr FROM contacts WHERE id IN({}) AND id!=1", // 1=DC_CONTACT_ID_SELF
                member_ids_str
            ),
            paramsv![],
            |row| row.get::<_, String>(0),
            |rows| {
                let mut addrs = rows.collect::<std::result::Result<Vec<_>, _>>()?;
                addrs.sort();
                let mut acc = member_cs.clone();
                for addr in &addrs {
                    acc += ",";
                    acc += &addr.to_lowercase();
                }
                Ok(acc)
            },
        )
        .await
        .unwrap_or(member_cs);

    hex_hash(&members)
}

#[allow(clippy::indexing_slicing)]
fn hex_hash(s: impl AsRef<str>) -> String {
    let bytes = s.as_ref().as_bytes();
    let result = Sha256::digest(bytes);
    hex::encode(&result[..8])
}

async fn check_verified_properties(
    context: &Context,
    mimeparser: &MimeMessage,
    from_id: u32,
    to_ids: &ContactIds,
) -> Result<()> {
    let contact = Contact::load_from_db(context, from_id).await?;

    ensure!(mimeparser.was_encrypted(), "This message is not encrypted.");

    if mimeparser.get(HeaderDef::ChatVerified).is_none() {
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
    if from_id != DC_CONTACT_ID_SELF {
        let peerstate = Peerstate::from_addr(context, contact.get_addr()).await?;

        if peerstate.is_none()
            || contact.is_verified_ex(context, peerstate.as_ref()).await
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
    let mut to_ids = to_ids.clone();
    to_ids.remove(&DC_CONTACT_ID_SELF);

    if to_ids.is_empty() {
        return Ok(());
    }
    let to_ids_str = join(to_ids.iter().map(|x| x.to_string()), ",");

    let rows = context
        .sql
        .query_map(
            format!(
                "SELECT c.addr, LENGTH(ps.verified_key_fingerprint)  FROM contacts c  \
             LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.id IN({}) ",
                to_ids_str
            ),
            paramsv![],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1).unwrap_or(0))),
            |rows| {
                rows.collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            },
        )
        .await?;

    for (to_addr, _is_verified) in rows.into_iter() {
        info!(
            context,
            "check_verified_properties: {:?} self={:?}",
            to_addr,
            context.is_self_addr(&to_addr).await
        );
        let mut is_verified = _is_verified != 0;
        let peerstate = Peerstate::from_addr(context, &to_addr).await?;

        // mark gossiped keys (if any) as verified
        if mimeparser.gossipped_addr.contains(&to_addr) {
            if let Some(mut peerstate) = peerstate {
                // if we're here, we know the gossip key is verified:
                // - use the gossip-key as verified-key if there is no verified-key
                // - OR if the verified-key does not match public-key or gossip-key
                //   (otherwise a verified key can _only_ be updated through QR scan which might be annoying,
                //   see https://github.com/nextleap-project/countermitm/issues/46 for a discussion about this point)
                if !is_verified
                    || peerstate.verified_key_fingerprint != peerstate.public_key_fingerprint
                        && peerstate.verified_key_fingerprint != peerstate.gossip_key_fingerprint
                {
                    info!(context, "{} has verified {}.", contact.get_addr(), to_addr,);
                    let fp = peerstate.gossip_key_fingerprint.clone();
                    if let Some(fp) = fp {
                        peerstate.set_verified(
                            PeerstateKeyType::GossipKey,
                            &fp,
                            PeerstateVerifiedStatus::BidirectVerified,
                        );
                        peerstate.save_to_db(&context.sql, false).await?;
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

fn set_better_msg(mime_parser: &mut MimeMessage, better_msg: impl AsRef<str>) {
    let msg = better_msg.as_ref();
    if !msg.is_empty() {
        if let Some(part) = mime_parser.parts.get_mut(0) {
            if part.typ == Viewtype::Text {
                part.msg = msg.to_string();
            }
        }
    }
}

/// Given a list of Message-IDs, returns the latest message found in the database.
async fn get_rfc724_mid_in_list(context: &Context, mid_list: &str) -> Result<Option<Message>> {
    if mid_list.is_empty() {
        return Ok(None);
    }

    if let Ok(ids) = parse_message_ids(mid_list) {
        for id in ids.iter().rev() {
            if let Some((_, _, msg_id)) = rfc724_mid_exists(context, id).await? {
                return Ok(Some(Message::load_from_db(context, msg_id).await?));
            }
        }
    }

    Ok(None)
}

/// Returns the last message referenced from References: header found in the database.
///
/// If none found, tries In-Reply-To: as a fallback for classic MUAs that don't set the
/// References: header.
async fn get_parent_message(
    context: &Context,
    mime_parser: &MimeMessage,
) -> Result<Option<Message>> {
    if let Some(field) = mime_parser.get(HeaderDef::References) {
        if let Some(msg) = get_rfc724_mid_in_list(context, field).await? {
            return Ok(Some(msg));
        }
    }

    if let Some(field) = mime_parser.get(HeaderDef::InReplyTo) {
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

/// * param `prevent_rename`: if true, the display_name of this contact will not be changed. Useful for
/// mailing lists: In some mailing lists, many users write from the same address but with different
/// display names. We don't want the display name to change everytime the user gets a new email from
/// a mailing list.
async fn dc_add_or_lookup_contacts_by_address_list(
    context: &Context,
    address_list: &[SingleInfo],
    origin: Origin,
    prevent_rename: bool,
) -> Result<ContactIds> {
    let mut contact_ids = ContactIds::new();
    for info in address_list.iter() {
        let display_name = if prevent_rename {
            Some("")
        } else {
            info.display_name.as_deref()
        };
        contact_ids.insert(
            add_or_lookup_contact_by_addr(context, display_name, &info.addr, origin).await?,
        );
    }

    Ok(contact_ids)
}

/// Add contacts to database on receiving messages.
async fn add_or_lookup_contact_by_addr(
    context: &Context,
    display_name: Option<impl AsRef<str>>,
    addr: &str,
    origin: Origin,
) -> Result<u32> {
    if context.is_self_addr(addr).await? {
        return Ok(DC_CONTACT_ID_SELF);
    }
    let display_name_normalized = display_name.map(normalize_name).unwrap_or_default();

    let (row_id, _modified) =
        Contact::add_or_lookup(context, display_name_normalized, addr, origin).await?;
    ensure!(row_id > 0, "could not add contact: {:?}", addr);

    Ok(row_id)
}

fn dc_create_incoming_rfc724_mid(
    message_timestamp: i64,
    contact_id_from: u32,
    contact_ids_to: &ContactIds,
) -> Option<String> {
    /* create a deterministic rfc724_mid from input such that
    repeatedly calling it with the same input results in the same Message-id */

    let largest_id_to = contact_ids_to.iter().max().copied().unwrap_or_default();
    let result = format!(
        "{}-{}-{}@stub",
        message_timestamp, contact_id_from, largest_id_to
    );
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::chat::{ChatItem, ChatVisibility};
    use crate::chatlist::Chatlist;
    use crate::constants::{DC_CHAT_ID_DEADDROP, DC_CONTACT_ID_INFO, DC_GCL_NO_SPECIALS};
    use crate::message::ContactRequestDecision::*;
    use crate::message::{ContactRequestDecision, Message};
    use crate::test_utils::{get_chat_msg, TestContext};

    #[test]
    fn test_hex_hash() {
        let data = "hello world";

        let res = hex_hash(data);
        assert_eq!(res, "b94d27b9934d3e08");
    }

    #[async_std::test]
    async fn test_grpid_simple() {
        let context = TestContext::new().await;
        let raw = b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    From: hello\n\
                    Subject: outer-subject\n\
                    In-Reply-To: <lqkjwelq123@123123>\n\
                    References: <Gr.HcxyMARjyJy.9-uvzWPTLtV@nauta.cu>\n\
                    \n\
                    hello\x00";
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        assert_eq!(extract_grpid(&mimeparser, HeaderDef::InReplyTo), None);
        let grpid = Some("HcxyMARjyJy");
        assert_eq!(extract_grpid(&mimeparser, HeaderDef::References), grpid);
    }

    #[async_std::test]
    async fn test_grpid_from_multiple() {
        let context = TestContext::new().await;
        let raw = b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    From: hello\n\
                    Subject: outer-subject\n\
                    In-Reply-To: <Gr.HcxyMARjyJy.9-qweqwe@asd.net>\n\
                    References: <qweqweqwe>, <Gr.HcxyMARjyJy.9-uvzWPTLtV@nau.ca>\n\
                    \n\
                    hello\x00";
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..])
            .await
            .unwrap();
        let grpid = Some("HcxyMARjyJy");
        assert_eq!(extract_grpid(&mimeparser, HeaderDef::InReplyTo), grpid);
        assert_eq!(extract_grpid(&mimeparser, HeaderDef::References), grpid);
    }

    #[test]
    fn test_dc_create_incoming_rfc724_mid() {
        let mut members = ContactIds::new();
        assert_eq!(
            dc_create_incoming_rfc724_mid(123, 45, &members),
            Some("123-45-0@stub".into())
        );
        members.insert(7);
        members.insert(3);
        assert_eq!(
            dc_create_incoming_rfc724_mid(123, 45, &members),
            Some("123-45-7@stub".into())
        );
        members.insert(9);
        assert_eq!(
            dc_create_incoming_rfc724_mid(123, 45, &members),
            Some("123-45-9@stub".into())
        );
    }

    static MSGRMSG: &[u8] =
        b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    From: Bob <bob@example.com>\n\
                    To: alice@example.com\n\
                    Chat-Version: 1.0\n\
                    Subject: Chat: hello\n\
                    Message-ID: <Mr.1111@example.com>\n\
                    Date: Sun, 22 Mar 2020 22:37:55 +0000\n\
                    \n\
                    hello\n";

    static ONETOONE_NOREPLY_MAIL: &[u8] =
        b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    From: Bob <bob@example.com>\n\
                    To: alice@example.com\n\
                    Subject: Chat: hello\n\
                    Message-ID: <2222@example.com>\n\
                    Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                    \n\
                    hello\n";

    static GRP_MAIL: &[u8] =
        b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    From: bob@example.com\n\
                    To: alice@example.com, claire@example.com\n\
                    Subject: group with Alice, Bob and Claire\n\
                    Message-ID: <3333@example.com>\n\
                    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                    \n\
                    hello\n";

    #[async_std::test]
    async fn test_adhoc_group_show_chats_only() {
        let t = TestContext::new_alice().await;
        assert_eq!(t.get_config_int(Config::ShowEmails).await, 0);

        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);

        dc_receive_imf(&t, MSGRMSG, "INBOX", 1, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);

        dc_receive_imf(&t, ONETOONE_NOREPLY_MAIL, "INBOX", 1, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);

        dc_receive_imf(&t, GRP_MAIL, "INBOX", 1, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
    }

    #[async_std::test]
    async fn test_adhoc_group_show_accepted_contact_unknown() {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("1")).await.unwrap();
        dc_receive_imf(&t, GRP_MAIL, "INBOX", 1, false)
            .await
            .unwrap();

        // adhoc-group with unknown contacts with show_emails=accepted is ignored for unknown contacts
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);
    }

    #[async_std::test]
    async fn test_adhoc_group_show_accepted_contact_known() {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("1")).await.unwrap();
        Contact::create(&t, "Bob", "bob@example.com").await.unwrap();
        dc_receive_imf(&t, GRP_MAIL, "INBOX", 1, false)
            .await
            .unwrap();

        // adhoc-group with known contacts with show_emails=accepted is still ignored for known contacts
        // (and existent chat is required)
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);
    }

    #[async_std::test]
    async fn test_adhoc_group_show_accepted_contact_accepted() {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("1")).await.unwrap();

        // accept Bob by accepting a delta-message from Bob
        dc_receive_imf(&t, MSGRMSG, "INBOX", 1, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        assert!(chats.get_chat_id(0).is_deaddrop());
        let chat_id = chat::create_by_msg_id(&t, chats.get_msg_id(0).unwrap())
            .await
            .unwrap();
        assert!(!chat_id.is_special());
        let chat = chat::Chat::load_from_db(&t, chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Single);
        assert_eq!(chat.name, "Bob");
        assert_eq!(chat::get_chat_contacts(&t, chat_id).await.len(), 1);
        assert_eq!(chat::get_chat_msgs(&t, chat_id, 0, None).await.len(), 1);

        // receive a non-delta-message from Bob, shows up because of the show_emails setting
        dc_receive_imf(&t, ONETOONE_NOREPLY_MAIL, "INBOX", 2, false)
            .await
            .unwrap();
        assert_eq!(chat::get_chat_msgs(&t, chat_id, 0, None).await.len(), 2);

        // let Bob create an adhoc-group by a non-delta-message, shows up because of the show_emails setting
        dc_receive_imf(&t, GRP_MAIL, "INBOX", 3, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 2);
        let chat_id = chat::create_by_msg_id(&t, chats.get_msg_id(0).unwrap())
            .await
            .unwrap();
        let chat = chat::Chat::load_from_db(&t, chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Group);
        assert_eq!(chat.name, "group with Alice, Bob and Claire");
        assert_eq!(chat::get_chat_contacts(&t, chat_id).await.len(), 3);
    }

    #[async_std::test]
    async fn test_adhoc_group_show_all() {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("2")).await.unwrap();
        dc_receive_imf(&t, GRP_MAIL, "INBOX", 1, false)
            .await
            .unwrap();

        // adhoc-group with unknown contacts with show_emails=all will show up in the deaddrop
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        assert!(chats.get_chat_id(0).is_deaddrop());
        let chat_id = chat::create_by_msg_id(&t, chats.get_msg_id(0).unwrap())
            .await
            .unwrap();
        let chat = chat::Chat::load_from_db(&t, chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Group);
        assert_eq!(chat.name, "group with Alice, Bob and Claire");
        assert_eq!(chat::get_chat_contacts(&t, chat_id).await.len(), 3);
    }

    #[async_std::test]
    async fn test_read_receipt_and_unarchive() {
        // create alice's account
        let t = TestContext::new_alice().await;

        let bob_id = Contact::create(&t, "bob", "bob@example.com").await.unwrap();
        let one2one_id = chat::create_by_contact_id(&t, bob_id).await.unwrap();
        one2one_id
            .set_visibility(&t, ChatVisibility::Archived)
            .await
            .unwrap();
        let one2one = Chat::load_from_db(&t, one2one_id).await.unwrap();
        assert!(one2one.get_visibility() == ChatVisibility::Archived);

        // create a group with bob, archive group
        let group_id = chat::create_group_chat(&t, ProtectionStatus::Unprotected, "foo")
            .await
            .unwrap();
        chat::add_contact_to_chat(&t, group_id, bob_id).await;
        assert_eq!(chat::get_chat_msgs(&t, group_id, 0, None).await.len(), 0);
        group_id
            .set_visibility(&t, ChatVisibility::Archived)
            .await
            .unwrap();
        let group = Chat::load_from_db(&t, group_id).await.unwrap();
        assert!(group.get_visibility() == ChatVisibility::Archived);

        // everything archived, chatlist should be empty
        assert_eq!(
            Chatlist::try_load(&t, DC_GCL_NO_SPECIALS, None, None)
                .await
                .unwrap()
                .len(),
            0
        );

        // send a message to group with bob
        dc_receive_imf(
            &t,
            format!(
                "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.com\n\
                 To: bob@example.com\n\
                 Subject: foo\n\
                 Message-ID: <Gr.{}.12345678901@example.com>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: {}\n\
                 Chat-Group-Name: foo\n\
                 Chat-Disposition-Notification-To: alice@example.com\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
                group.grpid, group.grpid
            )
            .as_bytes(),
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();
        let msg = get_chat_msg(&t, group_id, 0, 1).await;
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert_eq!(msg.text.unwrap(), "hello");
        assert_eq!(msg.state, MessageState::OutDelivered);
        let group = Chat::load_from_db(&t, group_id).await.unwrap();
        assert!(group.get_visibility() == ChatVisibility::Normal);

        // bob sends a read receipt to the group
        dc_receive_imf(
            &t,
            format!(
                "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
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
                 Original-Message-ID: <Gr.{}.12345678901@example.com>\n\
                 Disposition: manual-action/MDN-sent-automatically; displayed\n\
                 \n\
                 \n\
                 --SNIPP--",
                group.grpid
            )
            .as_bytes(),
            "INBOX",
            1,
            false,
        )
        .await.unwrap();
        assert_eq!(chat::get_chat_msgs(&t, group_id, 0, None).await.len(), 1);
        let msg = message::Message::load_from_db(&t, msg.id).await.unwrap();
        assert_eq!(msg.state, MessageState::OutMdnRcvd);

        // check, the read-receipt has not unarchived the one2one
        assert_eq!(
            Chatlist::try_load(&t, DC_GCL_NO_SPECIALS, None, None)
                .await
                .unwrap()
                .len(),
            1
        );
        let one2one = Chat::load_from_db(&t, one2one_id).await.unwrap();
        assert!(one2one.get_visibility() == ChatVisibility::Archived);
    }

    #[async_std::test]
    async fn test_no_from() {
        // if there is no from given, from_id stays 0 which is just fine. These messages
        // are very rare, however, we have to add them to the database (they go to the
        // "deaddrop" chat) to avoid a re-download from the server. See also [**]

        let t = TestContext::new_alice().await;
        let context = &t;

        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert!(chats.get_msg_id(0).is_err());

        dc_receive_imf(
            context,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 To: bob@example.com\n\
                 Subject: foo\n\
                 Message-ID: <3924@example.com>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();

        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        // Check that the message was added to the database:
        assert!(chats.get_msg_id(0).is_ok());
    }

    #[async_std::test]
    async fn test_escaped_from() {
        let t = TestContext::new_alice().await;
        let contact_id = Contact::create(&t, "foobar", "foobar@example.com")
            .await
            .unwrap();
        let chat_id = chat::create_by_contact_id(&t, contact_id).await.unwrap();
        dc_receive_imf(
            &t,
            b"From: =?UTF-8?B?0JjQvNGPLCDQpNCw0LzQuNC70LjRjw==?= <foobar@example.com>\n\
                 To: alice@example.com\n\
                 Subject: foo\n\
                 Message-ID: <asdklfjjaweofi@example.com>\n\
                 Chat-Version: 1.0\n\
                 Chat-Disposition-Notification-To: =?UTF-8?B?0JjQvNGPLCDQpNCw0LzQuNC70LjRjw==?= <foobar@example.com>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        ).await.unwrap();
        assert_eq!(
            Contact::load_from_db(&t, contact_id)
                .await
                .unwrap()
                .get_authname(),
            "Имя, Фамилия",
        );
        let msg = get_chat_msg(&t, chat_id, 0, 1).await;
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert_eq!(msg.text.unwrap(), "hello");
        assert_eq!(msg.param.get_int(Param::WantsMdn).unwrap(), 1);
    }

    #[async_std::test]
    async fn test_escaped_recipients() {
        let t = TestContext::new_alice().await;
        Contact::create(&t, "foobar", "foobar@example.com")
            .await
            .unwrap();

        let carl_contact_id =
            Contact::add_or_lookup(&t, "Carl", "carl@host.tld", Origin::IncomingUnknownFrom)
                .await
                .unwrap()
                .0;

        dc_receive_imf(
            &t,
            b"From: Foobar <foobar@example.com>\n\
                 To: =?UTF-8?B?0JjQvNGPLCDQpNCw0LzQuNC70LjRjw==?= alice@example.com\n\
                 Cc: =?utf-8?q?=3Ch2=3E?= <carl@host.tld>\n\
                 Subject: foo\n\
                 Message-ID: <asdklfjjaweofi@example.com>\n\
                 Chat-Version: 1.0\n\
                 Chat-Disposition-Notification-To: <foobar@example.com>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();
        let contact = Contact::load_from_db(&t, carl_contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "h2");

        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        let msg = Message::load_from_db(&t, chats.get_msg_id(0).unwrap())
            .await
            .unwrap();
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert_eq!(msg.text.unwrap(), "hello");
        assert_eq!(msg.param.get_int(Param::WantsMdn).unwrap(), 1);
    }

    #[async_std::test]
    async fn test_cc_to_contact() {
        let t = TestContext::new_alice().await;
        Contact::create(&t, "foobar", "foobar@example.com")
            .await
            .unwrap();

        let carl_contact_id =
            Contact::add_or_lookup(&t, "garabage", "carl@host.tld", Origin::IncomingUnknownFrom)
                .await
                .unwrap()
                .0;

        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: Foobar <foobar@example.com>\n\
                 To: alice@example.com\n\
                 Cc: Carl <carl@host.tld>\n\
                 Subject: foo\n\
                 Message-ID: <asdklfjjaweofi@example.com>\n\
                 Chat-Version: 1.0\n\
                 Chat-Disposition-Notification-To: <foobar@example.com>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();
        let contact = Contact::load_from_db(&t, carl_contact_id).await.unwrap();
        assert_eq!(contact.get_name(), "");
        assert_eq!(contact.get_display_name(), "Carl");
    }

    #[async_std::test]
    async fn test_parse_ndn_tiscali() {
        test_parse_ndn(
            "alice@tiscali.it",
            "shenauithz@testrun.org",
            "Mr.un2NYERi1RM.lbQ5F9q-QyJ@tiscali.it",
            include_bytes!("../test-data/message/tiscali_ndn.eml"),
            None,
        )
        .await;
    }

    #[async_std::test]
    async fn test_parse_ndn_testrun() {
        test_parse_ndn(
            "alice@testrun.org",
            "hcksocnsofoejx@five.chat",
            "Mr.A7pTA5IgrUA.q4bP41vAJOp@testrun.org",
            include_bytes!("../test-data/message/testrun_ndn.eml"),
            Some("Undelivered Mail Returned to Sender – This is the mail system at host hq5.merlinux.eu.\n\nI\'m sorry to have to inform you that your message could not\nbe delivered to one or more recipients. It\'s attached below.\n\nFor further assistance, please send mail to postmaster.\n\nIf you do so, please include this problem report. You can\ndelete your own text from the attached returned message.\n\n                   The mail system\n\n<hcksocnsofoejx@five.chat>: host mail.five.chat[195.62.125.103] said: 550 5.1.1\n    <hcksocnsofoejx@five.chat>: Recipient address rejected: User unknown in\n    virtual mailbox table (in reply to RCPT TO command)"),
        )
        .await;
    }

    #[async_std::test]
    async fn test_parse_ndn_yahoo() {
        test_parse_ndn(
            "alice@yahoo.com",
            "haeclirth.sinoenrat@yahoo.com",
            "1680295672.3657931.1591783872936@mail.yahoo.com",
            include_bytes!("../test-data/message/yahoo_ndn.eml"),
            Some("Failure Notice – Sorry, we were unable to deliver your message to the following address.\n\n<haeclirth.sinoenrat@yahoo.com>:\n554: delivery error: dd Not a valid recipient - atlas117.free.mail.ne1.yahoo.com [...]"),
        )
        .await;
    }

    #[async_std::test]
    async fn test_parse_ndn_gmail() {
        test_parse_ndn(
            "alice@gmail.com",
            "assidhfaaspocwaeofi@gmail.com",
            "CABXKi8zruXJc_6e4Dr087H5wE7sLp+u250o0N2q5DdjF_r-8wg@mail.gmail.com",
            include_bytes!("../test-data/message/gmail_ndn.eml"),
            Some("Delivery Status Notification (Failure) – ** Die Adresse wurde nicht gefunden **\n\nIhre Nachricht wurde nicht an assidhfaaspocwaeofi@gmail.com zugestellt, weil die Adresse nicht gefunden wurde oder keine E-Mails empfangen kann.\n\nHier erfahren Sie mehr: https://support.google.com/mail/?p=NoSuchUser\n\nAntwort:\n\n550 5.1.1 The email account that you tried to reach does not exist. Please try double-checking the recipient\'s email address for typos or unnecessary spaces. Learn more at https://support.google.com/mail/?p=NoSuchUser i18sor6261697wrs.38 - gsmtp"),
        )
        .await;
    }

    #[async_std::test]
    async fn test_parse_ndn_gmx() {
        test_parse_ndn(
            "alice@gmx.com",
            "snaerituhaeirns@gmail.com",
            "9c9c2a32-056b-3592-c372-d7e8f0bd4bc2@gmx.de",
            include_bytes!("../test-data/message/gmx_ndn.eml"),
            Some("Mail delivery failed: returning message to sender – This message was created automatically by mail delivery software.\n\nA message that you sent could not be delivered to one or more of\nits recipients. This is a permanent error. The following address(es)\nfailed:\n\nsnaerituhaeirns@gmail.com:\nSMTP error from remote server for RCPT TO command, host: gmail-smtp-in.l.google.com (66.102.1.27) reason: 550-5.1.1 The email account that you tried to reach does not exist. Please\n try\n550-5.1.1 double-checking the recipient\'s email address for typos or\n550-5.1.1 unnecessary spaces. Learn more at\n550 5.1.1  https://support.google.com/mail/?p=NoSuchUser f6si2517766wmc.21\n9 - gsmtp [...]"),
        )
        .await;
    }

    #[async_std::test]
    async fn test_parse_ndn_posteo() {
        test_parse_ndn(
            "alice@posteo.org",
            "hanerthaertidiuea@gmx.de",
            "04422840-f884-3e37-5778-8192fe22d8e1@posteo.de",
            include_bytes!("../test-data/message/posteo_ndn.eml"),
            Some("Undelivered Mail Returned to Sender – This is the mail system at host mout01.posteo.de.\n\nI\'m sorry to have to inform you that your message could not\nbe delivered to one or more recipients. It\'s attached below.\n\nFor further assistance, please send mail to postmaster.\n\nIf you do so, please include this problem report. You can\ndelete your own text from the attached returned message.\n\n                   The mail system\n\n<hanerthaertidiuea@gmx.de>: host mx01.emig.gmx.net[212.227.17.5] said: 550\n    Requested action not taken: mailbox unavailable (in reply to RCPT TO\n    command)"),
        )
        .await;
    }

    // ndn = Non Delivery Notification
    async fn test_parse_ndn(
        self_addr: &str,
        foreign_addr: &str,
        rfc724_mid_outgoing: &str,
        raw_ndn: &[u8],
        error_msg: Option<&str>,
    ) {
        let t = TestContext::new().await;
        t.configure_addr(self_addr).await;

        dc_receive_imf(
            &t,
            format!(
                "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                From: {}\n\
                To: {}\n\
                Subject: foo\n\
                Message-ID: <{}>\n\
                Chat-Version: 1.0\n\
                Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                \n\
                hello\n",
                self_addr, foreign_addr, rfc724_mid_outgoing
            )
            .as_bytes(),
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();

        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        let msg_id = chats.get_msg_id(0).unwrap();

        // Check that the ndn would be downloaded:
        let headers = mailparse::parse_mail(raw_ndn).unwrap().headers;
        assert!(
            crate::imap::prefetch_should_download(&t, &headers, ShowEmails::Off)
                .await
                .unwrap()
        );

        dc_receive_imf(&t, raw_ndn, "INBOX", 1, false)
            .await
            .unwrap();
        let msg = Message::load_from_db(&t, msg_id).await.unwrap();

        assert_eq!(msg.state, MessageState::OutFailed);

        assert_eq!(msg.error(), error_msg.map(|error| error.to_string()));
    }

    #[async_std::test]
    async fn test_parse_ndn_group_msg() {
        let t = TestContext::new().await;
        t.configure_addr("alice@gmail.com").await;

        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@gmail.com\n\
                 To: bob@example.com, assidhfaaspocwaeofi@gmail.com\n\
                 Subject: foo\n\
                 Message-ID: <CADWx9Cs32Wa7Gy-gM0bvbq54P_FEHe7UcsAV=yW7sVVW=fiMYQ@mail.gmail.com>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: abcde\n\
                 Chat-Group-Name: foo\n\
                 Chat-Disposition-Notification-To: alice@example.com\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();

        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        let msg_id = chats.get_msg_id(0).unwrap();

        let raw = include_bytes!("../test-data/message/gmail_ndn_group.eml");
        dc_receive_imf(&t, raw, "INBOX", 1, false).await.unwrap();

        let msg = Message::load_from_db(&t, msg_id).await.unwrap();

        assert_eq!(msg.state, MessageState::OutFailed);

        let msgs = chat::get_chat_msgs(&t, msg.chat_id, 0, None).await;
        let msg_id = if let ChatItem::Message { msg_id } = msgs.last().unwrap() {
            msg_id
        } else {
            panic!("Wrong item type");
        };
        let last_msg = Message::load_from_db(&t, *msg_id).await.unwrap();

        assert_eq!(
            last_msg.text,
            Some(stock_str::failed_sending_to(&t, "assidhfaaspocwaeofi@gmail.com").await,)
        );
        assert_eq!(last_msg.from_id, DC_CONTACT_ID_INFO);
    }

    async fn load_imf_email(context: &Context, imf_raw: &[u8]) -> Message {
        context
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();
        dc_receive_imf(context, imf_raw, "INBOX", 0, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(context, 0, None, None).await.unwrap();
        let msg_id = chats.get_msg_id(0).unwrap();
        Message::load_from_db(context, msg_id).await.unwrap()
    }

    #[async_std::test]
    async fn test_html_only_mail() {
        let t = TestContext::new_alice().await;
        let msg = load_imf_email(&t, include_bytes!("../test-data/message/wrong-html.eml")).await;
        assert_eq!(msg.text.unwrap(), "   Guten Abend,   \n\n   Lots of text   \n\n   text with Umlaut ä...   \n\n   MfG    [...]");
    }

    static GH_MAILINGLIST: &[u8] =
        b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
    From: Max Mustermann <notifications@github.com>\n\
    To: deltachat/deltachat-core-rust <deltachat-core-rust@noreply.github.com>\n\
    Subject: Let's put some [brackets here that] have nothing to do with the topic\n\
    Message-ID: <3333@example.org>\n\
    List-ID: deltachat/deltachat-core-rust <deltachat-core-rust.deltachat.github.com>\n\
    Precedence: list\n\
    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
    \n\
    hello\n";

    static GH_MAILINGLIST2: &[u8] =
        b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
    From: Github <notifications@github.com>\n\
    To: deltachat/deltachat-core-rust <deltachat-core-rust@noreply.github.com>\n\
    Subject: [deltachat/deltachat-core-rust] PR run failed\n\
    Message-ID: <3334@example.org>\n\
    List-ID: deltachat/deltachat-core-rust <deltachat-core-rust.deltachat.github.com>\n\
    Precedence: list\n\
    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
    \n\
    hello back\n";

    #[async_std::test]
    async fn test_github_mailing_list() {
        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();

        dc_receive_imf(&t.ctx, GH_MAILINGLIST, "INBOX", 1, false)
            .await
            .unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);

        let chat_id = chat::create_by_msg_id(&t.ctx, chats.get_msg_id(0).unwrap())
            .await
            .unwrap();
        let chat = chat::Chat::load_from_db(&t.ctx, chat_id).await.unwrap();

        assert!(chat.is_mailing_list());
        assert_eq!(chat.can_send(), false);
        assert_eq!(chat.name, "deltachat/deltachat-core-rust");
        assert_eq!(chat::get_chat_contacts(&t.ctx, chat_id).await.len(), 1);

        dc_receive_imf(&t.ctx, GH_MAILINGLIST2, "INBOX", 1, false)
            .await
            .unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        let contacts = Contact::get_all(&t.ctx, 0, None as Option<String>)
            .await
            .unwrap();
        assert_eq!(contacts.len(), 0); // mailing list recipients and senders do not count as "known contacts"

        let msg1 = get_chat_msg(&t, chat_id, 0, 2).await;
        let contact1 = Contact::load_from_db(&t.ctx, msg1.from_id).await.unwrap();
        assert_eq!(contact1.get_addr(), "notifications@github.com");
        assert_eq!(contact1.get_display_name(), "notifications@github.com"); // Make sure this is not "Max Mustermann" or somethinng

        let msg2 = get_chat_msg(&t, chat_id, 1, 2).await;
        let contact2 = Contact::load_from_db(&t.ctx, msg2.from_id).await.unwrap();
        assert_eq!(contact2.get_addr(), "notifications@github.com");

        assert_eq!(msg1.get_override_sender_name().unwrap(), "Max Mustermann");
        assert_eq!(msg2.get_override_sender_name().unwrap(), "Github");
    }

    static DC_MAILINGLIST: &[u8] = b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
    From: Bob <bob@posteo.org>\n\
    To: delta-dev@codespeak.net\n\
    Subject: Re: [delta-dev] What's up?\n\
    Message-ID: <38942@posteo.org>\n\
    List-ID: \"discussions about and around https://delta.chat developments\" <delta.codespeak.net>\n\
    Precedence: list\n\
    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
    \n\
    body\n";

    static DC_MAILINGLIST2: &[u8] = b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
    From: Charlie <charlie@posteo.org>\n\
    To: delta-dev@codespeak.net\n\
    Subject: Re: [delta-dev] DC is nice!\n\
    Message-ID: <38943@posteo.org>\n\
    List-ID: \"discussions about and around https://delta.chat developments\" <delta.codespeak.net>\n\
    Precedence: list\n\
    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
    \n\
    body 4\n";

    #[async_std::test]
    async fn test_classic_mailing_list() {
        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();
        dc_receive_imf(&t.ctx, DC_MAILINGLIST, "INBOX", 1, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        let chat_id = chat::create_by_msg_id(&t.ctx, chats.get_msg_id(0).unwrap())
            .await
            .unwrap();
        let chat = Chat::load_from_db(&t.ctx, chat_id).await.unwrap();
        assert_eq!(chat.name, "delta-dev");

        let msg = get_chat_msg(&t, chat_id, 0, 1).await;
        let contact1 = Contact::load_from_db(&t.ctx, msg.from_id).await.unwrap();
        assert_eq!(contact1.get_addr(), "bob@posteo.org");
    }

    #[async_std::test]
    async fn test_mailing_list_decide_block() {
        let deaddrop = ChatId::new(DC_CHAT_ID_DEADDROP);
        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();

        dc_receive_imf(&t.ctx, DC_MAILINGLIST, "INBOX", 1, false)
            .await
            .unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats.get_chat_id(0), deaddrop); // Test that the message is shown in the deaddrop

        let msg = get_chat_msg(&t, deaddrop, 0, 1).await;

        // Answer "Block" on the contact request
        message::decide_on_contact_request(&t.ctx, msg.get_id(), Block).await;

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0); // Test that the message disappeared
        let msgs = chat::get_chat_msgs(&t.ctx, deaddrop, 0, None).await;
        assert_eq!(msgs.len(), 0);

        dc_receive_imf(&t.ctx, DC_MAILINGLIST2, "INBOX", 1, false)
            .await
            .unwrap();

        // Test that the mailing list stays disappeared
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0); // Test that the message is not shown
        let msgs = chat::get_chat_msgs(&t.ctx, deaddrop, 0, None).await;
        assert_eq!(msgs.len(), 0);
    }

    #[async_std::test]
    async fn test_mailing_list_decide_block_then_unblock() {
        let deaddrop = ChatId::new(DC_CHAT_ID_DEADDROP);
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("2")).await.unwrap();

        dc_receive_imf(&t, DC_MAILINGLIST, "INBOX", 1000, false)
            .await
            .unwrap();
        let blocked = Contact::get_all_blocked(&t).await.unwrap();
        assert_eq!(blocked.len(), 0);

        // Answer "Block" on the contact request,
        // this should add one blocked contact and deaddrop should be empty again
        let msg = get_chat_msg(&t, deaddrop, 0, 1).await;
        message::decide_on_contact_request(&t, msg.get_id(), Block).await;
        let blocked = Contact::get_all_blocked(&t).await.unwrap();
        assert_eq!(blocked.len(), 1);
        let msgs = chat::get_chat_msgs(&t, deaddrop, 0, None).await;
        assert_eq!(msgs.len(), 0);

        // Unblock contact and check if the next message arrives in real chat
        Contact::unblock(&t, *blocked.first().unwrap()).await;
        let blocked = Contact::get_all_blocked(&t).await.unwrap();
        assert_eq!(blocked.len(), 0);

        dc_receive_imf(&t.ctx, DC_MAILINGLIST2, "INBOX", 1001, false)
            .await
            .unwrap();
        let msg = t.get_last_msg().await;
        assert_ne!(msg.chat_id, deaddrop);
        let msgs = chat::get_chat_msgs(&t, msg.chat_id, 0, None).await;
        assert_eq!(msgs.len(), 2);
        let msgs = chat::get_chat_msgs(&t, deaddrop, 0, None).await;
        assert_eq!(msgs.len(), 0);
    }

    #[async_std::test]
    async fn test_mailing_list_decide_not_now() {
        let deaddrop = ChatId::new(DC_CHAT_ID_DEADDROP);
        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();

        dc_receive_imf(&t.ctx, DC_MAILINGLIST, "INBOX", 1, false)
            .await
            .unwrap();

        let msg = get_chat_msg(&t, deaddrop, 0, 1).await;

        // Answer "Not now" on the contact request
        message::decide_on_contact_request(&t.ctx, msg.get_id(), NotNow).await;

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0); // Test that the message disappeared
        let msgs = chat::get_chat_msgs(&t.ctx, deaddrop, 0, None).await;
        assert_eq!(msgs.len(), 1); // ...but is still shown in the deaddrop

        dc_receive_imf(&t.ctx, DC_MAILINGLIST2, "INBOX", 1, false)
            .await
            .unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1); // Test that the new mailing list message is shown again
        let msgs = chat::get_chat_msgs(&t.ctx, deaddrop, 0, None).await;
        assert_eq!(msgs.len(), 2);
    }

    #[async_std::test]
    async fn test_mailing_list_decide_accept() {
        let deaddrop = ChatId::new(DC_CHAT_ID_DEADDROP);
        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();

        dc_receive_imf(&t.ctx, DC_MAILINGLIST, "INBOX", 1, false)
            .await
            .unwrap();

        let msg = get_chat_msg(&t, deaddrop, 0, 1).await;

        // Answer "Start chat" on the contact request
        message::decide_on_contact_request(&t.ctx, msg.get_id(), StartChat).await;

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1); // Test that the message is shown
        let chat_id = chats.get_chat_id(0);
        assert_ne!(chat_id, deaddrop);

        dc_receive_imf(&t.ctx, DC_MAILINGLIST2, "INBOX", 1, false)
            .await
            .unwrap();

        let msgs = chat::get_chat_msgs(&t.ctx, chat_id, 0, None).await;
        assert_eq!(msgs.len(), 2);
    }

    #[async_std::test]
    async fn test_majordomo_mailing_list() {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("2")).await.unwrap();

        // test mailing lists not having a `ListId:`-header
        dc_receive_imf(
            &t,
            b"From: Foo Bar <foo@bar.org>\n\
    To: deltachat/deltachat-core-rust <deltachat-core-rust@noreply.github.com>\n\
    Subject: [ola] just a subject\n\
    Message-ID: <3333@example.org>\n\
    Sender: My list <mylist@bar.org>\n\
    Precedence: list\n\
    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
    \n\
    hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();
        let msg = t.get_last_msg().await;
        let chat_id =
            message::decide_on_contact_request(&t, msg.id, ContactRequestDecision::StartChat)
                .await
                .unwrap();
        let chat = Chat::load_from_db(&t, chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Mailinglist);
        assert_eq!(chat.grpid, "mylist@bar.org");
        assert_eq!(chat.name, "ola");
        assert_eq!(chat::get_chat_msgs(&t, chat.id, 0, None).await.len(), 1);

        // receive another message with no sender name but the same address,
        // make sure this lands in the same chat
        dc_receive_imf(
            &t,
            b"From: Nu Bar <nu@bar.org>\n\
    To: deltachat/deltachat-core-rust <deltachat-core-rust@noreply.github.com>\n\
    Subject: [ola] Re: just a subject\n\
    Message-ID: <4444@example.org>\n\
    Sender: mylist@bar.org\n\
    Precedence: list\n\
    Date: Sun, 22 Mar 2020 23:37:57 +0000\n\
    \n\
    hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();
        assert_eq!(chat::get_chat_msgs(&t, chat.id, 0, None).await.len(), 2);
    }

    #[async_std::test]
    async fn test_mailchimp_mailing_list() {
        let t = TestContext::new_alice().await;
        t.set_config(Config::ShowEmails, Some("2")).await.unwrap();

        dc_receive_imf(
            &t,
            b"To: alice <alice@example.org>\n\
            Subject: =?utf-8?Q?How=20early=20megacities=20emerged=20from=20Cambodia=E2=80=99s=20jungles?=\n\
            From: =?utf-8?Q?Atlas=20Obscura?= <info@atlasobscura.com>\n\
            List-ID: 399fc0402f1b154b67965632emc list <399fc0402f1b154b67965632e.100761.list-id.mcsv.net>\n\
            Message-ID: <555@example.org>\n\
            Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
            \n\
            hello\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();
        let msg = t.get_last_msg().await;
        let chat = Chat::load_from_db(&t, msg.chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Mailinglist);
        assert_eq!(chat.blocked, Blocked::Deaddrop);
        assert_eq!(
            chat.grpid,
            "399fc0402f1b154b67965632e.100761.list-id.mcsv.net"
        );
        assert_eq!(chat.name, "Atlas Obscura");
    }

    #[async_std::test]
    async fn test_dont_show_tokens_in_contacts_list() {
        check_dont_show_in_contacts_list(
            "reply+OGHVYCLVBEGATYBICAXBIRQATABUOTUCERABERAHNO@reply.github.com",
        )
        .await;
    }

    #[async_std::test]
    async fn test_dont_show_noreply_in_contacts_list() {
        check_dont_show_in_contacts_list("noreply@github.com").await;
    }

    async fn check_dont_show_in_contacts_list(addr: &str) {
        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ShowEmails, Some("2"))
            .await
            .unwrap();
        dc_receive_imf(
            &t,
            format!(
                "Subject: Re: [deltachat/deltachat-core-rust] DC is the best repo on GitHub!
To: {}
References: <deltachat/deltachat-core-rust/pull/1625@github.com>
 <deltachat/deltachat-core-rust/pull/1625/c644661857@github.com>
From: alice@example.com
Message-ID: <d2717387-0ba7-7b60-9b09-fd89a76ea8a0@gmx.de>
Date: Tue, 16 Jun 2020 12:04:20 +0200
MIME-Version: 1.0
Content-Type: text/plain; charset=utf-8
Content-Transfer-Encoding: 7bit

YEAAAAAA!.
",
                addr
            )
            .as_bytes(),
            "Sent",
            1,
            false,
        )
        .await
        .unwrap();
        let contacts = Contact::get_all(&t, 0, None as Option<&str>).await.unwrap();
        assert!(contacts.is_empty()); // The contact should not have been added to the db
    }

    #[async_std::test]
    async fn test_pdf_filename_simple() {
        let t = TestContext::new_alice().await;
        let msg = load_imf_email(
            &t,
            include_bytes!("../test-data/message/pdf_filename_simple.eml"),
        )
        .await;
        assert_eq!(msg.viewtype, Viewtype::File);
        assert_eq!(msg.text.unwrap(), "mail body");
        assert_eq!(msg.param.get(Param::File).unwrap(), "$BLOBDIR/simple.pdf");
    }

    #[async_std::test]
    async fn test_pdf_filename_continuation() {
        // test filenames split across multiple header lines, see rfc 2231
        let t = TestContext::new_alice().await;
        let msg = load_imf_email(
            &t,
            include_bytes!("../test-data/message/pdf_filename_continuation.eml"),
        )
        .await;
        assert_eq!(msg.viewtype, Viewtype::File);
        assert_eq!(msg.text.unwrap(), "mail body");
        assert_eq!(
            msg.param.get(Param::File).unwrap(),
            "$BLOBDIR/test pdf äöüß.pdf"
        );
    }

    /// Test that classical MUA messages are assigned to group chats based on the `In-Reply-To`
    /// header.
    #[async_std::test]
    async fn test_in_reply_to() {
        let t = TestContext::new().await;
        t.configure_addr("bob@example.com").await;

        // Receive message from Alice about group "foo".
        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.org\n\
                 To: bob@example.com, charlie@example.net\n\
                 Subject: foo\n\
                 Message-ID: <message@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: foo\n\
                 Chat-Group-Name: foo\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello foo\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();

        // Receive reply from Charlie without group ID but with In-Reply-To header.
        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: charlie@example.net\n\
                 To: alice@example.org, bob@example.com\n\
                 Subject: Re: foo\n\
                 Message-ID: <message@example.net>\n\
                 In-Reply-To: <message@example.org>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 reply foo\n",
            "INBOX",
            2,
            false,
        )
        .await
        .unwrap();

        let msg = t.get_last_msg().await;
        assert_eq!(msg.get_text().unwrap(), "reply foo");

        // Load the first message from the same chat.
        let msgs = chat::get_chat_msgs(&t, msg.chat_id, 0, None).await;
        let msg_id = if let ChatItem::Message { msg_id } = msgs.first().unwrap() {
            msg_id
        } else {
            panic!("Wrong item type");
        };

        let reply_msg = Message::load_from_db(&t, *msg_id).await.unwrap();
        assert_eq!(reply_msg.get_text().unwrap(), "hello foo");

        // Check that reply got into the same chat as the original message.
        assert_eq!(msg.chat_id, reply_msg.chat_id);

        // Make sure we looked at real chat ID and do not just
        // test that both messages got into deaddrop.
        assert!(!msg.chat_id.is_special());
    }

    /// Test that classical MUA messages are assigned to group chats
    /// based on the `In-Reply-To` header for two-member groups.
    #[async_std::test]
    async fn test_in_reply_to_two_member_group() {
        let t = TestContext::new().await;
        t.configure_addr("bob@example.com").await;

        // Receive message from Alice about group "foo".
        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.org\n\
                 To: bob@example.com\n\
                 Subject: foo\n\
                 Message-ID: <message@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: foo\n\
                 Chat-Group-Name: foo\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello foo\n",
            "INBOX",
            1,
            false,
        )
        .await
        .unwrap();

        // Receive a classic MUA reply from Alice.
        // It is assigned to the group chat.
        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.org\n\
                 To: bob@example.com\n\
                 Subject: Re: foo\n\
                 Message-ID: <reply@example.org>\n\
                 In-Reply-To: <message@example.org>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 classic reply\n",
            "INBOX",
            2,
            false,
        )
        .await
        .unwrap();

        // Ensure message is assigned to group chat.
        let msg = t.get_last_msg().await;
        let chat = Chat::load_from_db(&t, msg.chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Group);
        assert_eq!(msg.get_text().unwrap(), "classic reply");

        // Receive a Delta Chat reply from Alice.
        // It is assigned to group chat, because it has a group ID.
        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.org\n\
                 To: bob@example.com\n\
                 Subject: Re: foo\n\
                 Message-ID: <chatreply@example.org>\n\
                 In-Reply-To: <message@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: foo\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 chat reply\n",
            "INBOX",
            3,
            false,
        )
        .await
        .unwrap();

        // Ensure message is assigned to group chat.
        let msg = t.get_last_msg().await;
        let chat = Chat::load_from_db(&t, msg.chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Group);
        assert_eq!(msg.get_text().unwrap(), "chat reply");

        // Receive a private Delta Chat reply from Alice.
        // It is assigned to 1:1 chat, because it has no group ID,
        // which means it was created using "reply privately" feature.
        // Normally it contains a quote, but it should not matter.
        dc_receive_imf(
            &t,
            b"Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                 From: alice@example.org\n\
                 To: bob@example.com\n\
                 Subject: Re: foo\n\
                 Message-ID: <chatprivatereply@example.org>\n\
                 In-Reply-To: <message@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 private reply\n",
            "INBOX",
            4,
            false,
        )
        .await
        .unwrap();

        // Ensure message is assigned to a 1:1 chat.
        let msg = t.get_last_msg().await;
        let chat = Chat::load_from_db(&t, msg.chat_id).await.unwrap();
        assert_eq!(chat.typ, Chattype::Single);
        assert_eq!(msg.get_text().unwrap(), "private reply");
    }
}

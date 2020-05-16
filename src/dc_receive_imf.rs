use itertools::join;
use sha2::{Digest, Sha256};

use num_traits::FromPrimitive;

use mailparse::SingleInfo;

use crate::chat::{self, Chat, ChatId};
use crate::config::Config;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::{bail, ensure, Result};
use crate::events::Event;
use crate::headerdef::HeaderDef;
use crate::job::*;
use crate::message::{self, MessageState, MessengerMessage, MsgId};
use crate::mimeparser::*;
use crate::param::*;
use crate::peerstate::*;
use crate::securejoin::{self, handle_securejoin_handshake, observe_securejoin_on_other_device};
use crate::sql;
use crate::stock::StockMessage;
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
pub fn dc_receive_imf(
    context: &Context,
    imf_raw: &[u8],
    server_folder: impl AsRef<str>,
    server_uid: u32,
    seen: bool,
) -> Result<()> {
    info!(
        context,
        "Receiving message {}/{}...",
        if !server_folder.as_ref().is_empty() {
            server_folder.as_ref()
        } else {
            "?"
        },
        server_uid,
    );

    if std::env::var(crate::DCC_MIME_DEBUG).unwrap_or_default() == "2" {
        info!(context, "dc_receive_imf: incoming message mime-body:");
        println!("{}", String::from_utf8_lossy(imf_raw));
    }

    let mut mime_parser = match MimeMessage::from_bytes(context, imf_raw) {
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
                    CreateEvent::MsgsChanged => Event::MsgsChanged { msg_id, chat_id },
                    CreateEvent::IncomingMsg => Event::IncomingMsg { msg_id, chat_id },
                };
                context.call_cb(event);
            }
        }
    };

    if let Some(value) = mime_parser.get(HeaderDef::Date) {
        // is not yet checked against bad times! we do this later if we have the database information.
        sent_timestamp = mailparse::dateparse(value).unwrap_or_default();
    }

    // get From: (it can be an address list!) and check if it is known (for known From:'s we add
    // the other To:/Cc: in the 3rd pass)
    // or if From: is equal to SELF (in this case, it is any outgoing messages,
    // we do not check Return-Path any more as this is unreliable, see
    // https://github.com/deltachat/deltachat-core/issues/150)
    let (from_id, from_id_blocked, incoming_origin) =
        from_field_to_contact_id(context, &mime_parser.from)?;

    let incoming = from_id != DC_CONTACT_ID_SELF;

    let mut to_ids = ContactIds::new();

    to_ids.extend(&dc_add_or_lookup_contacts_by_address_list(
        context,
        &mime_parser.recipients,
        if !incoming {
            Origin::OutgoingTo
        } else if incoming_origin.is_known() {
            Origin::IncomingTo
        } else {
            Origin::IncomingUnknownTo
        },
    )?);

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
            from_id_blocked,
            &mut hidden,
            &mut chat_id,
            seen,
            &mut needs_delete_job,
            &mut insert_msg_id,
            &mut created_db_entries,
            &mut create_event_to_send,
        ) {
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
        );
    }

    if let Some(avatar_action) = &mime_parser.user_avatar {
        match contact::set_profile_image(&context, from_id, avatar_action) {
            Ok(()) => {
                context.call_cb(Event::ChatModified(chat_id));
            }
            Err(err) => {
                warn!(context, "reveive_imf cannot update profile image: {}", err);
            }
        };
    }

    // Get user-configured server deletion
    let delete_server_after = context.get_config_delete_server_after();

    if !created_db_entries.is_empty() {
        if needs_delete_job || delete_server_after == Some(0) {
            for db_entry in &created_db_entries {
                job_add(
                    context,
                    Action::DeleteMsgOnImap,
                    db_entry.1.to_u32() as i32,
                    Params::new(),
                    0,
                );
            }
        } else {
            // Move message if we don't delete it immediately.
            context.do_heuristics_moves(server_folder.as_ref(), insert_msg_id);
        }
    }

    // if we delete we don't need to try moving messages
    if needs_delete_job && !created_db_entries.is_empty() {
        job_add(
            context,
            Action::DeleteMsgOnImap,
            created_db_entries[0].1.to_u32() as i32,
            Params::new(),
            0,
        );
    } else {
        context.do_heuristics_moves(server_folder.as_ref(), insert_msg_id);
    }

    info!(
        context,
        "received message {} has Message-Id: {}", server_uid, rfc724_mid
    );

    cleanup(context, &create_event_to_send, created_db_entries);

    mime_parser.handle_reports(context, from_id, sent_timestamp);

    Ok(())
}

/// Converts "From" field to contact id.
///
/// Also returns whether it is blocked or not and its origin.
pub fn from_field_to_contact_id(
    context: &Context,
    from_address_list: &[SingleInfo],
) -> Result<(u32, bool, Origin)> {
    let from_ids = dc_add_or_lookup_contacts_by_address_list(
        context,
        from_address_list,
        Origin::IncomingUnknownFrom,
    )?;

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
        if let Ok(contact) = Contact::load_from_db(context, from_id) {
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
fn add_parts(
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
    from_id_blocked: bool,
    hidden: &mut bool,
    chat_id: &mut ChatId,
    seen: bool,
    needs_delete_job: &mut bool,
    insert_msg_id: &mut MsgId,
    created_db_entries: &mut Vec<(ChatId, MsgId)>,
    create_event_to_send: &mut Option<CreateEvent>,
) -> Result<()> {
    let mut state: MessageState;
    let mut chat_id_blocked = Blocked::Not;
    let mut sort_timestamp = 0;
    let mut rcvd_timestamp = 0;
    let mut mime_in_reply_to = String::new();
    let mut mime_references = String::new();
    let mut incoming_origin = incoming_origin;

    // check, if the mail is already in our database - if so, just update the folder/uid
    // (if the mail was moved around) and finish. (we may get a mail twice eg. if it is
    // moved between folders. make sure, this check is done eg. before securejoin-processing) */
    if let Some((old_server_folder, old_server_uid, _)) =
        message::rfc724_mid_exists(context, &rfc724_mid)?
    {
        if old_server_folder != server_folder.as_ref() || old_server_uid != server_uid {
            message::update_server_uid(context, &rfc724_mid, server_folder.as_ref(), server_uid);
        }

        warn!(context, "Message already in DB");
        return Ok(());
    }

    let mut msgrmsg = if mime_parser.has_chat_version() {
        MessengerMessage::Yes
    } else if is_reply_to_messenger_message(context, mime_parser) {
        MessengerMessage::Reply
    } else {
        MessengerMessage::No
    };
    // incoming non-chat messages may be discarded
    let mut allow_creation = true;
    let show_emails =
        ShowEmails::from_i32(context.get_config_int(Config::ShowEmails)).unwrap_or_default();
    if mime_parser.is_system_message != SystemMessage::AutocryptSetupMessage
        && msgrmsg == MessengerMessage::No
    {
        // this message is a classic email not a chat-message nor a reply to one
        match show_emails {
            ShowEmails::Off => {
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
        state = if seen {
            MessageState::InSeen
        } else {
            MessageState::InFresh
        };
        to_id = DC_CONTACT_ID_SELF;

        // handshake may mark contacts as verified and must be processed before chats are created
        if mime_parser.get(HeaderDef::SecureJoin).is_some() {
            msgrmsg = MessengerMessage::Yes; // avoid discarding by show_emails setting
            *chat_id = ChatId::new(0);
            allow_creation = true;
            match handle_securejoin_handshake(context, mime_parser, from_id) {
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
                    context.bob.write().unwrap().status = 0; // secure-join failed
                    context.stop_ongoing();
                    warn!(context, "Error in Secure-Join message handling: {}", err);
                    return Ok(());
                }
            }
        }

        let (test_normal_chat_id, test_normal_chat_id_blocked) =
            chat::lookup_by_contact_id(context, from_id).unwrap_or_default();

        // get the chat_id - a chat_id here is no indicator that the chat is displayed in the normal list,
        // it might also be blocked and displayed in the deaddrop as a result
        if chat_id.is_unset() {
            // try to create a group
            // (groups appear automatically only if the _sender_ is known, see core issue #54)

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
            )?;
            *chat_id = new_chat_id;
            chat_id_blocked = new_chat_id_blocked;
            if !chat_id.is_unset()
                && chat_id_blocked != Blocked::Not
                && create_blocked == Blocked::Not
            {
                new_chat_id.unblock(context);
                chat_id_blocked = Blocked::Not;
            }
        }

        if chat_id.is_unset() {
            // check if the message belongs to a mailing list
            if mime_parser.is_mailinglist_message() {
                *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
                info!(context, "Message belongs to a mailing list and is ignored.",);
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
                        .unwrap_or_default();
                *chat_id = id;
                chat_id_blocked = bl;
            }
            if !chat_id.is_unset() && Blocked::Not != chat_id_blocked {
                if Blocked::Not == create_blocked {
                    chat_id.unblock(context);
                    chat_id_blocked = Blocked::Not;
                } else if is_reply_to_known_message(context, mime_parser) {
                    // we do not want any chat to be created implicitly.  Because of the origin-scale-up,
                    // the contact requests will pop up and this should be just fine.
                    Contact::scaleup_origin_by_id(context, from_id, Origin::IncomingReplyTo);
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
        }

        // if the chat_id is blocked,
        // for unknown senders and non-delta-messages set the state to NOTICED
        // to not result in a chatlist-contact-request (this would require the state FRESH)
        if Blocked::Not != chat_id_blocked
            && state == MessageState::InFresh
            && !incoming_origin.is_known()
            && msgrmsg == MessengerMessage::No
            && show_emails != ShowEmails::All
        {
            state = MessageState::InNoticed;
        }
    } else {
        // Outgoing

        // the mail is on the IMAP server, probably it is also delivered.
        // We cannot recreate other states (read, error).
        state = MessageState::OutDelivered;
        to_id = to_ids.get_index(0).cloned().unwrap_or_default();

        // handshake may mark contacts as verified and must be processed before chats are created
        if mime_parser.get(HeaderDef::SecureJoin).is_some() {
            msgrmsg = MessengerMessage::Yes; // avoid discarding by show_emails setting
            *chat_id = ChatId::new(0);
            allow_creation = true;
            match observe_securejoin_on_other_device(context, mime_parser, to_id) {
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

        if !to_ids.is_empty() {
            if chat_id.is_unset() {
                let (new_chat_id, new_chat_id_blocked) = create_or_lookup_group(
                    context,
                    &mut mime_parser,
                    allow_creation,
                    Blocked::Not,
                    from_id,
                    to_ids,
                )?;
                *chat_id = new_chat_id;
                chat_id_blocked = new_chat_id_blocked;
                // automatically unblock chat when the user sends a message
                if !chat_id.is_unset() && chat_id_blocked != Blocked::Not {
                    new_chat_id.unblock(context);
                    chat_id_blocked = Blocked::Not;
                }
            }
            if chat_id.is_unset() && allow_creation {
                let create_blocked = if MessengerMessage::No != msgrmsg
                    && !Contact::is_blocked_load(context, to_id)
                {
                    Blocked::Not
                } else {
                    Blocked::Deaddrop
                };
                let (id, bl) = chat::create_or_lookup_by_contact_id(context, to_id, create_blocked)
                    .unwrap_or_default();
                *chat_id = id;
                chat_id_blocked = bl;

                if !chat_id.is_unset()
                    && Blocked::Not != chat_id_blocked
                    && Blocked::Not == create_blocked
                {
                    chat_id.unblock(context);
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
                    .unwrap_or_default();
            *chat_id = id;
            chat_id_blocked = bl;

            if !chat_id.is_unset() && Blocked::Not != chat_id_blocked {
                chat_id.unblock(context);
                chat_id_blocked = Blocked::Not;
            }
        }
        if chat_id.is_unset() {
            *chat_id = ChatId::new(DC_CHAT_ID_TRASH);
        }
    }

    // Extract autodelete timer from the message.
    let timer = if let Some(value) = mime_parser.get(HeaderDef::AutodeleteTimer) {
        match value.parse::<u32>() {
            Ok(timer) => timer,
            Err(err) => {
                warn!(
                    context,
                    "can't parse autodelete timer \"{}\": {}", value, err
                );
                0
            }
        }
    } else {
        0
    };

    // Apply autodelete timer changes to the chat.
    if chat::get_autodelete_timer(context, *chat_id) != timer {
        match chat::inner_set_autodelete_timer(context, *chat_id, timer) {
            Ok(()) => {
                let stock_str = context.stock_system_msg(
                    StockMessage::MsgAutodeleteTimerChanged,
                    timer.to_string(),
                    "",
                    from_id,
                );
                chat::add_info_msg(context, *chat_id, stock_str);
                context.call_cb(Event::ChatAutodeleteTimerModified {
                    chat_id: *chat_id,
                    timer,
                });
            }
            Err(err) => {
                warn!(
                    context,
                    "failed to modify timer for chat {}: {}", chat_id, err
                );
            }
        }
    }

    // correct message_timestamp, it should not be used before,
    // however, we cannot do this earlier as we need from_id to be set
    calc_timestamps(
        context,
        *chat_id,
        from_id,
        *sent_timestamp,
        !seen,
        &mut sort_timestamp,
        sent_timestamp,
        &mut rcvd_timestamp,
    );

    // unarchive chat
    chat_id.unarchive(context)?;

    // if the mime-headers should be saved, find out its size
    // (the mime-header ends with an empty line)
    let save_mime_headers = context.get_config_bool(Config::SaveMimeHeaders);
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

    let mut txt_raw = None;

    context.sql.prepare(
        "INSERT INTO msgs \
         (rfc724_mid, server_folder, server_uid, chat_id, from_id, to_id, timestamp, \
         timestamp_sent, timestamp_rcvd, type, state, msgrmsg,  txt, txt_raw, param, \
         bytes, hidden, mime_headers,  mime_in_reply_to, mime_references, autodelete_timer) \
         VALUES (?,?,?,?,?,?, ?,?,?,?,?,?, ?,?,?,?,?,?, ?,?,?);",
        |mut stmt, conn| {
            let subject = mime_parser.get_subject().unwrap_or_default();

            for part in mime_parser.parts.iter_mut() {
                let is_mdn = !mime_parser.reports.is_empty();

                let is_location_kml = mime_parser.location_kml.is_some()
                    && icnt == 1
                    && (part.msg == "-location-" || part.msg.is_empty());

                if is_mdn || is_location_kml {
                    *hidden = true;
                    if state == MessageState::InFresh {
                        state = MessageState::InNoticed;
                    }
                }

                if part.typ == Viewtype::Text {
                    let msg_raw = part.msg_raw.as_ref().cloned().unwrap_or_default();
                    txt_raw = Some(format!("{}\n\n{}", subject, msg_raw));
                }
                if mime_parser.is_system_message != SystemMessage::Unknown {
                    part.param
                        .set_int(Param::Cmd, mime_parser.is_system_message as i32);
                }

                stmt.execute(params![
                    rfc724_mid,
                    server_folder.as_ref(),
                    server_uid as i32,
                    *chat_id,
                    from_id as i32,
                    to_id as i32,
                    sort_timestamp,
                    *sent_timestamp,
                    rcvd_timestamp,
                    part.typ,
                    state,
                    msgrmsg,
                    &part.msg,
                    // txt_raw might contain invalid utf8
                    txt_raw.unwrap_or_default(),
                    part.param.to_string(),
                    part.bytes as isize,
                    *hidden,
                    if save_mime_headers {
                        Some(String::from_utf8_lossy(imf_raw))
                    } else {
                        None
                    },
                    mime_in_reply_to,
                    mime_references,
                    timer
                ])?;

                txt_raw = None;
                let row_id =
                    sql::get_rowid_with_conn(context, conn, "msgs", "rfc724_mid", &rfc724_mid);
                *insert_msg_id = MsgId::new(row_id);
                created_db_entries.push((*chat_id, *insert_msg_id));
            }
            Ok(())
        },
    )?;

    info!(
        context,
        "Message has {} parts and is assigned to chat #{}.", icnt, *chat_id,
    );

    // check event to send
    if chat_id.is_trash() || *hidden {
        *create_event_to_send = None;
    } else if incoming && state == MessageState::InFresh {
        if from_id_blocked {
            *create_event_to_send = None;
        } else if Blocked::Not != chat_id_blocked {
            *create_event_to_send = Some(CreateEvent::MsgsChanged);
        } else {
            *create_event_to_send = Some(CreateEvent::IncomingMsg);
        }
    }

    Ok(())
}

fn save_locations(
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
        let newest_location_id =
            location::save(context, chat_id, from_id, locations, true).unwrap_or_default();
        if 0 != newest_location_id
            && !hidden
            && location::set_msg_location_id(context, insert_msg_id, newest_location_id).is_ok()
        {
            location_id_written = true;
            send_event = true;
        }
    }

    if mime_parser.location_kml.is_some() {
        if let Some(ref addr) = mime_parser.location_kml.as_ref().unwrap().addr {
            if let Ok(contact) = Contact::get_by_id(context, from_id) {
                if contact.get_addr().to_lowercase() == addr.to_lowercase() {
                    let locations = &mime_parser.location_kml.as_ref().unwrap().locations;
                    let newest_location_id =
                        location::save(context, chat_id, from_id, locations, false)
                            .unwrap_or_default();
                    if newest_location_id != 0 && !hidden && !location_id_written {
                        if let Err(err) = location::set_msg_location_id(
                            context,
                            insert_msg_id,
                            newest_location_id,
                        ) {
                            error!(context, "Failed to set msg_location_id: {:?}", err);
                        }
                    }
                    send_event = true;
                }
            }
        }
    }
    if send_event {
        context.call_cb(Event::LocationChanged(Some(from_id)));
    }
}

#[allow(clippy::too_many_arguments)]
fn calc_timestamps(
    context: &Context,
    chat_id: ChatId,
    from_id: u32,
    message_timestamp: i64,
    is_fresh_msg: bool,
    sort_timestamp: &mut i64,
    sent_timestamp: &mut i64,
    rcvd_timestamp: &mut i64,
) {
    *rcvd_timestamp = time();
    *sent_timestamp = message_timestamp;
    if *sent_timestamp > *rcvd_timestamp {
        *sent_timestamp = *rcvd_timestamp
    }
    *sort_timestamp = message_timestamp;
    if is_fresh_msg {
        let last_msg_time: Option<i64> = context.sql.query_get_value(
            context,
            "SELECT MAX(timestamp) FROM msgs WHERE chat_id=? and from_id!=? AND timestamp>=?",
            params![chat_id, from_id as i32, *sort_timestamp],
        );
        if let Some(last_msg_time) = last_msg_time {
            if last_msg_time > 0 && *sort_timestamp <= last_msg_time {
                *sort_timestamp = last_msg_time + 1;
            }
        }
    }
    if *sort_timestamp >= dc_smeared_time(context) {
        *sort_timestamp = dc_create_smeared_timestamp(context);
    }
}

/// This function tries extracts the group-id from the message and returns the
/// corresponding chat_id. If the chat_id is not existent, it is created.
/// If the message contains groups commands (name, profile image, changed members),
/// they are executed as well.
///
/// if no group-id could be extracted from the message, create_or_lookup_adhoc_group() is called
/// which tries to create or find out the chat_id by:
/// - is there a group with the same recipients? if so, use this (if there are multiple, use the most recent one)
/// - create an ad-hoc group based on the recipient list
///
/// on success the function returns the found/created (chat_id, chat_blocked) tuple .
#[allow(non_snake_case, clippy::cognitive_complexity)]
fn create_or_lookup_group(
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
        better_msg =
            context.stock_system_msg(StockMessage::MsgLocationEnabled, "", "", from_id as u32);
        set_better_msg(mime_parser, &better_msg);
    }

    let mut grpid = "".to_string();
    if let Some(optional_field) = mime_parser.get(HeaderDef::ChatGroupId) {
        grpid = optional_field.clone();
    }

    if grpid.is_empty() {
        if let Some(value) = mime_parser.get(HeaderDef::MessageId) {
            if let Some(extracted_grpid) = dc_extract_grpid_from_rfc724_mid(&value) {
                grpid = extracted_grpid.to_string();
            }
        }
        if grpid.is_empty() {
            if let Some(extracted_grpid) = extract_grpid(mime_parser, HeaderDef::InReplyTo) {
                grpid = extracted_grpid.to_string();
            } else if let Some(extracted_grpid) = extract_grpid(mime_parser, HeaderDef::References)
            {
                grpid = extracted_grpid.to_string();
            } else {
                return create_or_lookup_adhoc_group(
                    context,
                    mime_parser,
                    allow_creation,
                    create_blocked,
                    from_id,
                    to_ids,
                )
                .map_err(|err| {
                    info!(context, "could not create adhoc-group: {:?}", err);
                    err
                });
            }
        }
    }
    // now we have a grpid that is non-empty
    // but we might not know about this group

    let grpname = mime_parser.get(HeaderDef::ChatGroupName).cloned();
    let mut removed_id = 0;

    if let Some(removed_addr) = mime_parser.get(HeaderDef::ChatGroupMemberRemoved).cloned() {
        removed_id = Contact::lookup_id_by_addr(context, &removed_addr, Origin::Unknown);
        if removed_id == 0 {
            warn!(context, "removed {:?} has no contact_id", removed_addr);
        } else {
            mime_parser.is_system_message = SystemMessage::MemberRemovedFromGroup;
            better_msg = context.stock_system_msg(
                if removed_id == from_id as u32 {
                    StockMessage::MsgGroupLeft
                } else {
                    StockMessage::MsgDelMember
                },
                &removed_addr,
                "",
                from_id as u32,
            );
        }
    } else {
        let field = mime_parser.get(HeaderDef::ChatGroupMemberAdded).cloned();
        if let Some(optional_field) = field {
            mime_parser.is_system_message = SystemMessage::MemberAddedToGroup;
            better_msg = context.stock_system_msg(
                StockMessage::MsgAddMember,
                &optional_field,
                "",
                from_id as u32,
            );
            X_MrAddToGrp = Some(optional_field);
        } else {
            let field = mime_parser.get(HeaderDef::ChatGroupNameChanged);
            if let Some(field) = field {
                X_MrGrpNameChanged = true;
                better_msg = context.stock_system_msg(
                    StockMessage::MsgGrpName,
                    field,
                    if let Some(ref name) = grpname {
                        name
                    } else {
                        ""
                    },
                    from_id as u32,
                );

                mime_parser.is_system_message = SystemMessage::GroupNameChanged;
            } else if let Some(value) = mime_parser.get(HeaderDef::ChatContent) {
                if value == "group-avatar-changed" {
                    if let Some(avatar_action) = &mime_parser.group_avatar {
                        // this is just an explicit message containing the group-avatar,
                        // apart from that, the group-avatar is send along with various other messages
                        mime_parser.is_system_message = SystemMessage::GroupImageChanged;
                        better_msg = context.stock_system_msg(
                            match avatar_action {
                                AvatarAction::Delete => StockMessage::MsgGrpImgDeleted,
                                AvatarAction::Change(_) => StockMessage::MsgGrpImgChanged,
                            },
                            "",
                            "",
                            from_id as u32,
                        )
                    }
                }
            }
        }
    }
    set_better_msg(mime_parser, &better_msg);

    // check, if we have a chat with this group ID
    let (mut chat_id, chat_id_verified, _blocked) = chat::get_chat_id_by_grpid(context, &grpid)
        .unwrap_or((ChatId::new(0), false, Blocked::Not));
    if !chat_id.is_error() {
        if chat_id_verified {
            if let Err(err) =
                check_verified_properties(context, mime_parser, from_id as u32, to_ids)
            {
                warn!(context, "verification problem: {}", err);
                let s = format!("{}. See 'Info' for more details", err);
                mime_parser.repl_msg_by_error(s);
            }
        }
        if !chat::is_contact_in_chat(context, chat_id, from_id as u32) {
            // The From-address is not part of this group.
            // It could be a new user or a DSN from a mailer-daemon.
            // in any case we do not want to recreate the member list
            // but still show the message as part of the chat.
            // After all, the sender has a reference/in-reply-to that
            // points to this chat.
            let s = context.stock_str(StockMessage::UnknownSenderForChat);
            mime_parser.repl_msg_by_error(s.to_string());
        }
    }

    // check if the group does not exist but should be created
    let group_explicitly_left = chat::is_group_explicitly_left(context, &grpid).unwrap_or_default();
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_default();

    if chat_id.is_error()
            && !mime_parser.is_mailinglist_message()
            && !grpid.is_empty()
            && grpname.is_some()
            // otherwise, a pending "quit" message may pop up
            && removed_id == 0
            // re-create explicitly left groups only if ourself is re-added
            && (!group_explicitly_left
                || X_MrAddToGrp.is_some() && addr_cmp(&self_addr, X_MrAddToGrp.as_ref().unwrap()))
    {
        // group does not exist but should be created
        let create_verified = if mime_parser.get(HeaderDef::ChatVerified).is_some() {
            if let Err(err) =
                check_verified_properties(context, mime_parser, from_id as u32, to_ids)
            {
                warn!(context, "verification problem: {}", err);
                let s = format!("{}. See 'Info' for more details", err);
                mime_parser.repl_msg_by_error(&s);
            }
            VerifiedStatus::Verified
        } else {
            VerifiedStatus::Unverified
        };

        if !allow_creation {
            info!(context, "creating group forbidden by caller");
            return Ok((ChatId::new(0), Blocked::Not));
        }

        chat_id = create_group_record(
            context,
            &grpid,
            grpname.as_ref().unwrap(),
            create_blocked,
            create_verified,
        );
        chat_id_blocked = create_blocked;
        recreate_member_list = true;
    }

    // again, check chat_id
    if chat_id.is_special() {
        return if group_explicitly_left {
            Ok((ChatId::new(DC_CHAT_ID_TRASH), chat_id_blocked))
        } else {
            create_or_lookup_adhoc_group(
                context,
                mime_parser,
                allow_creation,
                create_blocked,
                from_id,
                to_ids,
            )
            .map_err(|err| {
                warn!(context, "failed to create ad-hoc group: {:?}", err);
                err
            })
        };
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
                if sql::execute(
                    context,
                    &context.sql,
                    "UPDATE chats SET name=? WHERE id=?;",
                    params![grpname, chat_id],
                )
                .is_ok()
                {
                    context.call_cb(Event::ChatModified(chat_id));
                }
            }
        }
    }
    if let Some(avatar_action) = &mime_parser.group_avatar {
        info!(context, "group-avatar change for {}", chat_id);
        if let Ok(mut chat) = Chat::load_from_db(context, chat_id) {
            match avatar_action {
                AvatarAction::Change(profile_image) => {
                    chat.param.set(Param::ProfileImage, profile_image);
                }
                AvatarAction::Delete => {
                    chat.param.remove(Param::ProfileImage);
                }
            };
            chat.update_param(context)?;
            send_EVENT_CHAT_MODIFIED = true;
        }
    }

    // add members to group/check members
    if recreate_member_list {
        if !chat::is_contact_in_chat(context, chat_id, DC_CONTACT_ID_SELF) {
            // Members could have been removed while we were
            // absent. We can't use existing member list and need to
            // start from scratch.
            sql::execute(
                context,
                &context.sql,
                "DELETE FROM chats_contacts WHERE chat_id=?;",
                params![chat_id],
            )
            .ok();

            chat::add_to_chat_contacts_table(context, chat_id, DC_CONTACT_ID_SELF);
        }
        if from_id > DC_CONTACT_ID_LAST_SPECIAL
            && !Contact::addr_equals_contact(context, &self_addr, from_id as u32)
            && !chat::is_contact_in_chat(context, chat_id, from_id)
        {
            chat::add_to_chat_contacts_table(context, chat_id, from_id as u32);
        }
        for &to_id in to_ids.iter() {
            info!(context, "adding to={:?} to chat id={}", to_id, chat_id);
            if !Contact::addr_equals_contact(context, &self_addr, to_id)
                && !chat::is_contact_in_chat(context, chat_id, to_id)
            {
                chat::add_to_chat_contacts_table(context, chat_id, to_id);
            }
        }
        send_EVENT_CHAT_MODIFIED = true;
    } else if removed_id > 0 {
        chat::remove_from_chat_contacts_table(context, chat_id, removed_id);
        send_EVENT_CHAT_MODIFIED = true;
    }

    if send_EVENT_CHAT_MODIFIED {
        context.call_cb(Event::ChatModified(chat_id));
    }
    Ok((chat_id, chat_id_blocked))
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

/// Handle groups for received messages, return chat_id/Blocked status on success
fn create_or_lookup_adhoc_group(
    context: &Context,
    mime_parser: &MimeMessage,
    allow_creation: bool,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &ContactIds,
) -> Result<(ChatId, Blocked)> {
    // if we're here, no grpid was found, check if there is an existing
    // ad-hoc group matching the to-list or if we should and can create one
    // (we do not want to heuristically look at the likely mangled Subject)

    if mime_parser.is_mailinglist_message() {
        // XXX we could parse List-* headers and actually create and
        // manage a mailing list group, eventually
        info!(
            context,
            "not creating ad-hoc group for mailing list message"
        );
        return Ok((ChatId::new(0), Blocked::Not));
    }

    let mut member_ids: Vec<u32> = to_ids.iter().copied().collect();
    if !member_ids.contains(&from_id) {
        member_ids.push(from_id);
    }
    if !member_ids.contains(&DC_CONTACT_ID_SELF) {
        member_ids.push(DC_CONTACT_ID_SELF);
    }

    if member_ids.len() < 3 {
        info!(context, "not creating ad-hoc group: too few contacts");
        return Ok((ChatId::new(0), Blocked::Not));
    }

    let chat_ids = search_chat_ids_by_contact_ids(context, &member_ids)?;
    if !chat_ids.is_empty() {
        let chat_ids_str = join(chat_ids.iter().map(|x| x.to_string()), ",");
        let res = context.sql.query_row(
            format!(
                "SELECT c.id,
                        c.blocked
                   FROM chats c
                   LEFT JOIN msgs m
                          ON m.chat_id=c.id
                  WHERE c.id IN({})
                  ORDER BY m.timestamp DESC,
                           m.id DESC
                  LIMIT 1;",
                chat_ids_str
            ),
            params![],
            |row| {
                Ok((
                    row.get::<_, ChatId>(0)?,
                    row.get::<_, Option<Blocked>>(1)?.unwrap_or_default(),
                ))
            },
        );

        if let Ok((id, id_blocked)) = res {
            /* success, chat found */
            return Ok((id, id_blocked));
        }
    }

    if !allow_creation {
        info!(context, "creating ad-hoc group prevented from caller");
        return Ok((ChatId::new(0), Blocked::Not));
    }

    // we do not check if the message is a reply to another group, this may result in
    // chats with unclear member list. instead we create a new group in the following lines ...

    // create a new ad-hoc group
    // - there is no need to check if this group exists; otherwise we would have caught it above
    let grpid = create_adhoc_grp_id(context, &member_ids);
    if grpid.is_empty() {
        warn!(
            context,
            "failed to create ad-hoc grpid for {:?}", member_ids
        );
        return Ok((ChatId::new(0), Blocked::Not));
    }
    // use subject as initial chat name
    let grpname = mime_parser
        .get_subject()
        .unwrap_or_else(|| "Unnamed group".to_string());

    // create group record
    let new_chat_id: ChatId = create_group_record(
        context,
        &grpid,
        grpname,
        create_blocked,
        VerifiedStatus::Unverified,
    );
    for &member_id in &member_ids {
        chat::add_to_chat_contacts_table(context, new_chat_id, member_id);
    }

    context.call_cb(Event::ChatModified(new_chat_id));

    Ok((new_chat_id, create_blocked))
}

fn create_group_record(
    context: &Context,
    grpid: impl AsRef<str>,
    grpname: impl AsRef<str>,
    create_blocked: Blocked,
    create_verified: VerifiedStatus,
) -> ChatId {
    if sql::execute(
        context,
        &context.sql,
        "INSERT INTO chats (type, name, grpid, blocked, created_timestamp) VALUES(?, ?, ?, ?, ?);",
        params![
            if VerifiedStatus::Unverified != create_verified {
                Chattype::VerifiedGroup
            } else {
                Chattype::Group
            },
            grpname.as_ref(),
            grpid.as_ref(),
            create_blocked,
            time(),
        ],
    )
    .is_err()
    {
        warn!(
            context,
            "Failed to create group '{}' for grpid={}",
            grpname.as_ref(),
            grpid.as_ref()
        );
        return ChatId::new(0);
    }
    let row_id = sql::get_rowid(context, &context.sql, "chats", "grpid", grpid.as_ref());
    let chat_id = ChatId::new(row_id);
    info!(
        context,
        "Created group '{}' grpid={} as {}",
        grpname.as_ref(),
        grpid.as_ref(),
        chat_id
    );
    chat_id
}

fn create_adhoc_grp_id(context: &Context, member_ids: &[u32]) -> String {
    /* algorithm:
    - sort normalized, lowercased, e-mail addresses alphabetically
    - put all e-mail addresses into a single string, separate the address by a single comma
    - sha-256 this string (without possibly terminating null-characters)
    - encode the first 64 bits of the sha-256 output as lowercase hex (results in 16 characters from the set [0-9a-f])
     */
    let member_ids_str = join(member_ids.iter().map(|x| x.to_string()), ",");
    let member_cs = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_else(|| "no-self".to_string())
        .to_lowercase();

    let members = context
        .sql
        .query_map(
            format!(
                "SELECT addr FROM contacts WHERE id IN({}) AND id!=1", // 1=DC_CONTACT_ID_SELF
                member_ids_str
            ),
            params![],
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
        .unwrap_or_else(|_| member_cs);

    hex_hash(&members)
}

fn hex_hash(s: impl AsRef<str>) -> String {
    let bytes = s.as_ref().as_bytes();
    let result = Sha256::digest(bytes);
    hex::encode(&result[..8])
}

fn search_chat_ids_by_contact_ids(
    context: &Context,
    unsorted_contact_ids: &[u32],
) -> Result<Vec<ChatId>> {
    /* searches chat_id's by the given contact IDs, may return zero, one or more chat_id's */
    let mut contact_ids = Vec::with_capacity(23);
    let mut chat_ids = Vec::with_capacity(23);

    /* copy array, remove duplicates and SELF, sort by ID */
    if !unsorted_contact_ids.is_empty() {
        for &curr_id in unsorted_contact_ids {
            if curr_id != 1 && !contact_ids.contains(&curr_id) {
                contact_ids.push(curr_id);
            }
        }
        if !contact_ids.is_empty() {
            contact_ids.sort();
            let contact_ids_str = join(contact_ids.iter().map(|x| x.to_string()), ",");
            context.sql.query_map(
                format!(
                    "SELECT DISTINCT cc.chat_id, cc.contact_id
                       FROM chats_contacts cc
                       LEFT JOIN chats c ON c.id=cc.chat_id
                      WHERE cc.chat_id IN(SELECT chat_id FROM chats_contacts WHERE contact_id IN({}))
                        AND c.type=120
                        AND cc.contact_id!=1
                      ORDER BY cc.chat_id, cc.contact_id;", // 1=DC_CONTACT_ID_SELF
                    contact_ids_str
                ),
                params![],
                |row| Ok((row.get::<_, ChatId>(0)?, row.get::<_, u32>(1)?)),
                |rows| {
                    let mut last_chat_id = ChatId::new(0);
                    let mut matches = 0;
                    let mut mismatches = 0;

                    for row in rows {
                        let (chat_id, contact_id) = row?;
                        if chat_id != last_chat_id {
                            if matches == contact_ids.len() && mismatches == 0 {
                                chat_ids.push(last_chat_id);
                            }
                            last_chat_id = chat_id;
                            matches = 0;
                            mismatches = 0;
                        }
                        if matches < contact_ids.len() && contact_id == contact_ids[matches] {
                            matches += 1;
                        } else {
                            mismatches += 1;
                        }
                    }

                    if matches == contact_ids.len() && mismatches == 0 {
                        chat_ids.push(last_chat_id);
                    }
                Ok(())
                }
            )?;
        }
    }

    Ok(chat_ids)
}

fn check_verified_properties(
    context: &Context,
    mimeparser: &MimeMessage,
    from_id: u32,
    to_ids: &ContactIds,
) -> Result<()> {
    let contact = Contact::load_from_db(context, from_id)?;

    ensure!(mimeparser.was_encrypted(), "This message is not encrypted.");

    // ensure, the contact is verified
    // and the message is signed with a verified key of the sender.
    // this check is skipped for SELF as there is no proper SELF-peerstate
    // and results in group-splits otherwise.
    if from_id != DC_CONTACT_ID_SELF {
        let peerstate = Peerstate::from_addr(context, &context.sql, contact.get_addr());

        if peerstate.is_none()
            || contact.is_verified_ex(context, peerstate.as_ref())
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

    let rows = context.sql.query_map(
        format!(
            "SELECT c.addr, LENGTH(ps.verified_key_fingerprint)  FROM contacts c  \
             LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.id IN({}) ",
            to_ids_str
        ),
        params![],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1).unwrap_or(0))),
        |rows| {
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        },
    )?;

    for (to_addr, _is_verified) in rows.into_iter() {
        info!(
            context,
            "check_verified_properties: {:?} self={:?}",
            to_addr,
            context.is_self_addr(&to_addr)
        );
        let mut is_verified = _is_verified != 0;
        let peerstate = Peerstate::from_addr(context, &context.sql, &to_addr);

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
                        peerstate.save_to_db(&context.sql, false)?;
                        is_verified = true;
                    }
                }
            }
        }
        if !is_verified {
            bail!(
                "{} is not a member of this verified group",
                to_addr.to_string()
            );
        }
    }
    Ok(())
}

fn set_better_msg(mime_parser: &mut MimeMessage, better_msg: impl AsRef<str>) {
    let msg = better_msg.as_ref();
    if !msg.is_empty() && !mime_parser.parts.is_empty() {
        let part = &mut mime_parser.parts[0];
        if part.typ == Viewtype::Text {
            part.msg = msg.to_string();
        }
    }
}

fn is_reply_to_known_message(context: &Context, mime_parser: &MimeMessage) -> bool {
    /* check if the message is a reply to a known message; the replies are identified by the Message-ID from
    `In-Reply-To`/`References:` (to support non-Delta-Clients) */

    if let Some(field) = mime_parser.get(HeaderDef::InReplyTo) {
        if is_known_rfc724_mid_in_list(context, &field) {
            return true;
        }
    }

    if let Some(field) = mime_parser.get(HeaderDef::References) {
        if is_known_rfc724_mid_in_list(context, &field) {
            return true;
        }
    }

    false
}

fn is_known_rfc724_mid_in_list(context: &Context, mid_list: &str) -> bool {
    if mid_list.is_empty() {
        return false;
    }

    if let Ok(ids) = parse_message_ids(mid_list) {
        for id in ids.iter() {
            if is_known_rfc724_mid(context, id) {
                return true;
            }
        }
    }

    false
}

/// Check if a message is a reply to a known message (messenger or non-messenger).
fn is_known_rfc724_mid(context: &Context, rfc724_mid: &str) -> bool {
    let rfc724_mid = rfc724_mid.trim_start_matches('<').trim_end_matches('>');
    context
        .sql
        .exists(
            "SELECT m.id FROM msgs m  \
             LEFT JOIN chats c ON m.chat_id=c.id  \
             WHERE m.rfc724_mid=?  \
             AND m.chat_id>9 AND c.blocked=0;",
            params![rfc724_mid],
        )
        .unwrap_or_default()
}

/// Checks if the message defined by mime_parser references a message send by us from Delta Chat.
/// This is similar to is_reply_to_known_message() but
/// - checks also if any of the referenced IDs are send by a messenger
/// - it is okay, if the referenced messages are moved to trash here
/// - no check for the Chat-* headers (function is only called if it is no messenger message itself)
fn is_reply_to_messenger_message(context: &Context, mime_parser: &MimeMessage) -> bool {
    if let Some(value) = mime_parser.get(HeaderDef::InReplyTo) {
        if is_msgrmsg_rfc724_mid_in_list(context, &value) {
            return true;
        }
    }

    if let Some(value) = mime_parser.get(HeaderDef::References) {
        if is_msgrmsg_rfc724_mid_in_list(context, &value) {
            return true;
        }
    }

    false
}

pub(crate) fn is_msgrmsg_rfc724_mid_in_list(context: &Context, mid_list: &str) -> bool {
    if let Ok(ids) = parse_message_ids(mid_list) {
        for id in ids.iter() {
            if is_msgrmsg_rfc724_mid(context, id) {
                return true;
            }
        }
    }
    false
}

/// Check if a message is a reply to any messenger message.
fn is_msgrmsg_rfc724_mid(context: &Context, rfc724_mid: &str) -> bool {
    let rfc724_mid = rfc724_mid.trim_start_matches('<').trim_end_matches('>');
    context
        .sql
        .exists(
            "SELECT id FROM msgs  WHERE rfc724_mid=?  AND msgrmsg!=0  AND chat_id>9;",
            params![rfc724_mid],
        )
        .unwrap_or_default()
}

fn dc_add_or_lookup_contacts_by_address_list(
    context: &Context,
    address_list: &[SingleInfo],
    origin: Origin,
) -> Result<ContactIds> {
    let mut contact_ids = ContactIds::new();
    for info in address_list.iter() {
        contact_ids.insert(add_or_lookup_contact_by_addr(
            context,
            &info.display_name,
            &info.addr,
            origin,
        )?);
    }

    Ok(contact_ids)
}

/// Add contacts to database on receiving messages.
fn add_or_lookup_contact_by_addr(
    context: &Context,
    display_name: &Option<String>,
    addr: &str,
    origin: Origin,
) -> Result<u32> {
    if context.is_self_addr(addr)? {
        return Ok(DC_CONTACT_ID_SELF);
    }
    let display_name_normalized = display_name
        .as_ref()
        .map(normalize_name)
        .unwrap_or_default();

    let (row_id, _modified) =
        Contact::add_or_lookup(context, display_name_normalized, addr, origin)?;
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
    use crate::chat::ChatVisibility;
    use crate::chatlist::Chatlist;
    use crate::message::Message;
    use crate::test_utils::{dummy_context, TestContext};

    #[test]
    fn test_hex_hash() {
        let data = "hello world";

        let res = hex_hash(data);
        assert_eq!(res, "b94d27b9934d3e08");
    }

    #[test]
    fn test_grpid_simple() {
        let context = dummy_context();
        let raw = b"From: hello\n\
                    Subject: outer-subject\n\
                    In-Reply-To: <lqkjwelq123@123123>\n\
                    References: <Gr.HcxyMARjyJy.9-uvzWPTLtV@nauta.cu>\n\
                    \n\
                    hello\x00";
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..]).unwrap();
        assert_eq!(extract_grpid(&mimeparser, HeaderDef::InReplyTo), None);
        let grpid = Some("HcxyMARjyJy");
        assert_eq!(extract_grpid(&mimeparser, HeaderDef::References), grpid);
    }

    #[test]
    fn test_grpid_from_multiple() {
        let context = dummy_context();
        let raw = b"From: hello\n\
                    Subject: outer-subject\n\
                    In-Reply-To: <Gr.HcxyMARjyJy.9-qweqwe@asd.net>\n\
                    References: <qweqweqwe>, <Gr.HcxyMARjyJy.9-uvzWPTLtV@nau.ca>\n\
                    \n\
                    hello\x00";
        let mimeparser = MimeMessage::from_bytes(&context.ctx, &raw[..]).unwrap();
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

    #[test]
    fn test_is_known_rfc724_mid() {
        let t = dummy_context();
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("first message".to_string());
        let msg_id = chat::add_device_msg(&t.ctx, None, Some(&mut msg)).unwrap();
        let msg = Message::load_from_db(&t.ctx, msg_id).unwrap();

        // Message-IDs may or may not be surrounded by angle brackets
        assert!(is_known_rfc724_mid(
            &t.ctx,
            format!("<{}>", msg.rfc724_mid).as_str()
        ));
        assert!(is_known_rfc724_mid(&t.ctx, &msg.rfc724_mid));
        assert!(!is_known_rfc724_mid(&t.ctx, "nonexistant@message.id"));
    }

    #[test]
    fn test_is_msgrmsg_rfc724_mid() {
        let t = dummy_context();
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some("first message".to_string());
        let msg_id = chat::add_device_msg(&t.ctx, None, Some(&mut msg)).unwrap();
        let msg = Message::load_from_db(&t.ctx, msg_id).unwrap();

        // Message-IDs may or may not be surrounded by angle brackets
        assert!(is_msgrmsg_rfc724_mid(
            &t.ctx,
            format!("<{}>", msg.rfc724_mid).as_str()
        ));
        assert!(is_msgrmsg_rfc724_mid(&t.ctx, &msg.rfc724_mid));
        assert!(!is_msgrmsg_rfc724_mid(&t.ctx, "nonexistant@message.id"));
    }

    fn configured_offline_context() -> TestContext {
        let t = dummy_context();
        t.ctx
            .set_config(Config::Addr, Some("alice@example.org"))
            .unwrap();
        t.ctx
            .set_config(Config::ConfiguredAddr, Some("alice@example.org"))
            .unwrap();
        t.ctx.set_config(Config::Configured, Some("1")).unwrap();
        t
    }

    static MSGRMSG: &[u8] = b"From: Bob <bob@example.org>\n\
                    To: alice@example.org\n\
                    Chat-Version: 1.0\n\
                    Subject: Chat: hello\n\
                    Message-ID: <Mr.1111@example.org>\n\
                    Date: Sun, 22 Mar 2020 22:37:55 +0000\n\
                    \n\
                    hello\n";

    static ONETOONE_NOREPLY_MAIL: &[u8] = b"From: Bob <bob@example.org>\n\
                    To: alice@example.org\n\
                    Subject: Chat: hello\n\
                    Message-ID: <2222@example.org>\n\
                    Date: Sun, 22 Mar 2020 22:37:56 +0000\n\
                    \n\
                    hello\n";

    static GRP_MAIL: &[u8] = b"From: bob@example.org\n\
                    To: alice@example.org, claire@example.org\n\
                    Subject: group with Alice, Bob and Claire\n\
                    Message-ID: <3333@example.org>\n\
                    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                    \n\
                    hello\n";

    #[test]
    fn test_adhoc_group_show_chats_only() {
        let t = configured_offline_context();
        assert_eq!(t.ctx.get_config_int(Config::ShowEmails), 0);

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 0);

        dc_receive_imf(&t.ctx, MSGRMSG, "INBOX", 1, false).unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 1);

        dc_receive_imf(&t.ctx, ONETOONE_NOREPLY_MAIL, "INBOX", 1, false).unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 1);

        dc_receive_imf(&t.ctx, GRP_MAIL, "INBOX", 1, false).unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 1);
    }

    #[test]
    fn test_adhoc_group_show_accepted_contact_unknown() {
        let t = configured_offline_context();
        t.ctx.set_config(Config::ShowEmails, Some("1")).unwrap();
        dc_receive_imf(&t.ctx, GRP_MAIL, "INBOX", 1, false).unwrap();

        // adhoc-group with unknown contacts with show_emails=accepted is ignored for unknown contacts
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 0);
    }

    #[test]
    fn test_adhoc_group_show_accepted_contact_known() {
        let t = configured_offline_context();
        t.ctx.set_config(Config::ShowEmails, Some("1")).unwrap();
        Contact::create(&t.ctx, "Bob", "bob@example.org").unwrap();
        dc_receive_imf(&t.ctx, GRP_MAIL, "INBOX", 1, false).unwrap();

        // adhoc-group with known contacts with show_emails=accepted is still ignored for known contacts
        // (and existent chat is required)
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 0);
    }

    #[test]
    fn test_adhoc_group_show_accepted_contact_accepted() {
        let t = configured_offline_context();
        t.ctx.set_config(Config::ShowEmails, Some("1")).unwrap();

        // accept Bob by accepting a delta-message from Bob
        dc_receive_imf(&t.ctx, MSGRMSG, "INBOX", 1, false).unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 1);
        assert!(chats.get_chat_id(0).is_deaddrop());
        let chat_id = chat::create_by_msg_id(&t.ctx, chats.get_msg_id(0).unwrap()).unwrap();
        assert!(!chat_id.is_special());
        let chat = chat::Chat::load_from_db(&t.ctx, chat_id).unwrap();
        assert_eq!(chat.typ, Chattype::Single);
        assert_eq!(chat.name, "Bob");
        assert_eq!(chat::get_chat_contacts(&t.ctx, chat_id).len(), 1);
        assert_eq!(chat::get_chat_msgs(&t.ctx, chat_id, 0, None).len(), 1);

        // receive a non-delta-message from Bob, shows up because of the show_emails setting
        dc_receive_imf(&t.ctx, ONETOONE_NOREPLY_MAIL, "INBOX", 2, false).unwrap();
        assert_eq!(chat::get_chat_msgs(&t.ctx, chat_id, 0, None).len(), 2);

        // let Bob create an adhoc-group by a non-delta-message, shows up because of the show_emails setting
        dc_receive_imf(&t.ctx, GRP_MAIL, "INBOX", 3, false).unwrap();
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 2);
        let chat_id = chat::create_by_msg_id(&t.ctx, chats.get_msg_id(0).unwrap()).unwrap();
        let chat = chat::Chat::load_from_db(&t.ctx, chat_id).unwrap();
        assert_eq!(chat.typ, Chattype::Group);
        assert_eq!(chat.name, "group with Alice, Bob and Claire");
        assert_eq!(chat::get_chat_contacts(&t.ctx, chat_id).len(), 3);
    }

    #[test]
    fn test_adhoc_group_show_all() {
        let t = configured_offline_context();
        t.ctx.set_config(Config::ShowEmails, Some("2")).unwrap();
        dc_receive_imf(&t.ctx, GRP_MAIL, "INBOX", 1, false).unwrap();

        // adhoc-group with unknown contacts with show_emails=all will show up in the deaddrop
        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert_eq!(chats.len(), 1);
        assert!(chats.get_chat_id(0).is_deaddrop());
        let chat_id = chat::create_by_msg_id(&t.ctx, chats.get_msg_id(0).unwrap()).unwrap();
        let chat = chat::Chat::load_from_db(&t.ctx, chat_id).unwrap();
        assert_eq!(chat.typ, Chattype::Group);
        assert_eq!(chat.name, "group with Alice, Bob and Claire");
        assert_eq!(chat::get_chat_contacts(&t.ctx, chat_id).len(), 3);
    }

    #[test]
    fn test_read_receipt_and_unarchive() {
        // create alice's account
        let t = configured_offline_context();

        // create one-to-one with bob, archive one-to-one
        let bob_id = Contact::create(&t.ctx, "bob", "bob@exampel.org").unwrap();
        let one2one_id = chat::create_by_contact_id(&t.ctx, bob_id).unwrap();
        one2one_id
            .set_visibility(&t.ctx, ChatVisibility::Archived)
            .unwrap();
        let one2one = Chat::load_from_db(&t.ctx, one2one_id).unwrap();
        assert!(one2one.get_visibility() == ChatVisibility::Archived);

        // create a group with bob, archive group
        let group_id = chat::create_group_chat(&t.ctx, VerifiedStatus::Unverified, "foo").unwrap();
        chat::add_contact_to_chat(&t.ctx, group_id, bob_id);
        assert_eq!(chat::get_chat_msgs(&t.ctx, group_id, 0, None).len(), 0);
        group_id
            .set_visibility(&t.ctx, ChatVisibility::Archived)
            .unwrap();
        let group = Chat::load_from_db(&t.ctx, group_id).unwrap();
        assert!(group.get_visibility() == ChatVisibility::Archived);

        // everything archived, chatlist should be empty
        assert_eq!(
            Chatlist::try_load(&t.ctx, DC_GCL_NO_SPECIALS, None, None)
                .unwrap()
                .len(),
            0
        );

        // send a message to group with bob
        dc_receive_imf(
            &t.ctx,
            format!(
                "From: alice@example.org\n\
                 To: bob@example.org\n\
                 Subject: foo\n\
                 Message-ID: <Gr.{}.12345678901@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Group-ID: {}\n\
                 Chat-Group-Name: foo\n\
                 Chat-Disposition-Notification-To: alice@example.org\n\
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
        .unwrap();
        let msgs = chat::get_chat_msgs(&t.ctx, group_id, 0, None);
        assert_eq!(msgs.len(), 1);
        let msg_id = msgs.first().unwrap();
        let msg = message::Message::load_from_db(&t.ctx, msg_id.clone()).unwrap();
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert_eq!(msg.text.unwrap(), "hello");
        assert_eq!(msg.state, MessageState::OutDelivered);
        let group = Chat::load_from_db(&t.ctx, group_id).unwrap();
        assert!(group.get_visibility() == ChatVisibility::Normal);

        // bob sends a read receipt to the group
        dc_receive_imf(
            &t.ctx,
            format!(
                "From: bob@example.org\n\
                 To: alice@example.org\n\
                 Subject: message opened\n\
                 Date: Sun, 22 Mar 2020 23:37:57 +0000\n\
                 Chat-Version: 1.0\n\
                 Message-ID: <Mr.12345678902@example.org>\n\
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
                 Original-Recipient: rfc822;bob@example.org\n\
                 Final-Recipient: rfc822;bob@example.org\n\
                 Original-Message-ID: <Gr.{}.12345678901@example.org>\n\
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
        .unwrap();
        assert_eq!(chat::get_chat_msgs(&t.ctx, group_id, 0, None).len(), 1);
        let msg = message::Message::load_from_db(&t.ctx, msg_id.clone()).unwrap();
        assert_eq!(msg.state, MessageState::OutMdnRcvd);

        // check, the read-receipt has not unarchived the one2one
        assert_eq!(
            Chatlist::try_load(&t.ctx, DC_GCL_NO_SPECIALS, None, None)
                .unwrap()
                .len(),
            1
        );
        let one2one = Chat::load_from_db(&t.ctx, one2one_id).unwrap();
        assert!(one2one.get_visibility() == ChatVisibility::Archived);
    }

    #[test]
    fn test_no_from() {
        // if there is no from given, from_id stays 0 which is just fine. These messages
        // are very rare, however, we have to add them to the database (they go to the
        // "deaddrop" chat) to avoid a re-download from the server. See also [**]

        let t = configured_offline_context();
        let context = &t.ctx;

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        assert!(chats.get_msg_id(0).is_err());

        dc_receive_imf(
            context,
            b"To: bob@example.org\n\
                 Subject: foo\n\
                 Message-ID: <3924@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        )
        .unwrap();

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        // Check that the message was added to the database:
        assert!(chats.get_msg_id(0).is_ok());
    }

    #[test]
    fn test_escaped_from() {
        let t = configured_offline_context();
        let contact_id = Contact::create(&t.ctx, "foobar", "foobar@example.com").unwrap();
        let chat_id = chat::create_by_contact_id(&t.ctx, contact_id).unwrap();
        dc_receive_imf(
            &t.ctx,
            b"From: =?UTF-8?B?0JjQvNGPLCDQpNCw0LzQuNC70LjRjw==?= <foobar@example.com>\n\
                 To: alice@example.org\n\
                 Subject: foo\n\
                 Message-ID: <asdklfjjaweofi@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Disposition-Notification-To: =?UTF-8?B?0JjQvNGPLCDQpNCw0LzQuNC70LjRjw==?= <foobar@example.com>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        ).unwrap();
        assert_eq!(
            Contact::load_from_db(&t.ctx, contact_id)
                .unwrap()
                .get_authname(),
            "Фамилия Имя", // The name was "Имя, Фамилия" and ("lastname, firstname") and should be swapped to "firstname, lastname"
        );
        let msgs = chat::get_chat_msgs(&t.ctx, chat_id, 0, None);
        assert_eq!(msgs.len(), 1);
        let msg_id = msgs.first().unwrap();
        let msg = message::Message::load_from_db(&t.ctx, msg_id.clone()).unwrap();
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert_eq!(msg.text.unwrap(), "hello");
        assert_eq!(msg.param.get_int(Param::WantsMdn).unwrap(), 1);
    }

    #[test]
    fn test_escaped_recipients() {
        let t = configured_offline_context();
        Contact::create(&t.ctx, "foobar", "foobar@example.com").unwrap();

        let carl_contact_id =
            Contact::add_or_lookup(&t.ctx, "Carl", "carl@host.tld", Origin::IncomingUnknownFrom)
                .unwrap()
                .0;

        dc_receive_imf(
            &t.ctx,
            b"From: Foobar <foobar@example.com>\n\
                 To: =?UTF-8?B?0JjQvNGPLCDQpNCw0LzQuNC70LjRjw==?= alice@example.org\n\
                 Cc: =?utf-8?q?=3Ch2=3E?= <carl@host.tld>\n\
                 Subject: foo\n\
                 Message-ID: <asdklfjjaweofi@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Disposition-Notification-To: <foobar@example.com>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        )
        .unwrap();
        assert_eq!(
            Contact::load_from_db(&t.ctx, carl_contact_id)
                .unwrap()
                .get_name(),
            "h2"
        );

        let chats = Chatlist::try_load(&t.ctx, 0, None, None).unwrap();
        let msg = Message::load_from_db(&t.ctx, chats.get_msg_id(0).unwrap()).unwrap();
        assert_eq!(msg.is_dc_message, MessengerMessage::Yes);
        assert_eq!(msg.text.unwrap(), "hello");
        assert_eq!(msg.param.get_int(Param::WantsMdn).unwrap(), 1);
    }

    #[test]
    fn test_cc_to_contact() {
        let t = configured_offline_context();
        Contact::create(&t.ctx, "foobar", "foobar@example.com").unwrap();

        let carl_contact_id = Contact::add_or_lookup(
            &t.ctx,
            "garabage",
            "carl@host.tld",
            Origin::IncomingUnknownFrom,
        )
        .unwrap()
        .0;

        dc_receive_imf(
            &t.ctx,
            b"From: Foobar <foobar@example.com>\n\
                 To: alice@example.org\n\
                 Cc: Carl <carl@host.tld>\n\
                 Subject: foo\n\
                 Message-ID: <asdklfjjaweofi@example.org>\n\
                 Chat-Version: 1.0\n\
                 Chat-Disposition-Notification-To: <foobar@example.com>\n\
                 Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                 \n\
                 hello\n",
            "INBOX",
            1,
            false,
        )
        .unwrap();
        assert_eq!(
            Contact::load_from_db(&t.ctx, carl_contact_id)
                .unwrap()
                .get_name(),
            "Carl"
        );
    }
}

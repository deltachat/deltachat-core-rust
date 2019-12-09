use itertools::join;
use sha2::{Digest, Sha256};

use num_traits::FromPrimitive;

use crate::blob::BlobObject;
use crate::chat::{self, Chat};
use crate::config::Config;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::Result;
use crate::events::Event;
use crate::job::*;
use crate::location;
use crate::message::{self, MessageState, MsgId};
use crate::mimeparser::*;
use crate::param::*;
use crate::peerstate::*;
use crate::securejoin::handle_securejoin_handshake;
use crate::sql;
use crate::stock::StockMessage;

#[derive(Debug, PartialEq, Eq)]
enum CreateEvent {
    MsgsChanged,
    IncomingMsg,
}

/// Receive a message and add it to the database.
pub fn dc_receive_imf(
    context: &Context,
    imf_raw: &[u8],
    server_folder: impl AsRef<str>,
    server_uid: u32,
    flags: u32,
) {
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

    let mime_parser = MimeParser::from_bytes(context, imf_raw);
    let mut mime_parser = if let Err(err) = mime_parser {
        warn!(context, "dc_receive_imf parse error: {}", err);
        return;
    } else {
        mime_parser.unwrap()
    };

    if mime_parser.header.is_empty() {
        // Error - even adding an empty record won't help as we do not know the message ID
        warn!(context, "No header.");
        return;
    }

    // the function returns the number of created messages in the database
    let mut incoming = true;
    let mut incoming_origin = Origin::Unknown;
    let mut to_self = false;
    let mut from_id = 0u32;
    let mut from_id_blocked = 0;
    let mut to_id = 0u32;
    let mut chat_id = 0;
    let mut hidden = false;

    let mut needs_delete_job = false;
    let mut insert_msg_id = MsgId::new_unset();

    let mut sent_timestamp = 0;
    let mut created_db_entries = Vec::new();
    let mut create_event_to_send = Some(CreateEvent::MsgsChanged);
    let mut rr_event_to_send = Vec::new();

    let mut to_ids = Vec::with_capacity(16);

    // helper method to handle early exit and memory cleanup
    let cleanup = |context: &Context,
                   create_event_to_send: &Option<CreateEvent>,
                   created_db_entries: &Vec<(usize, MsgId)>,
                   rr_event_to_send: &Vec<(u32, MsgId)>| {
        if let Some(create_event_to_send) = create_event_to_send {
            for (chat_id, msg_id) in created_db_entries {
                let event = match create_event_to_send {
                    CreateEvent::MsgsChanged => Event::MsgsChanged {
                        msg_id: *msg_id,
                        chat_id: *chat_id as u32,
                    },
                    CreateEvent::IncomingMsg => Event::IncomingMsg {
                        msg_id: *msg_id,
                        chat_id: *chat_id as u32,
                    },
                };
                context.call_cb(event);
            }
        }
        for (chat_id, msg_id) in rr_event_to_send {
            context.call_cb(Event::MsgRead {
                chat_id: *chat_id,
                msg_id: *msg_id,
            });
        }
    };

    if let Some(value) = mime_parser.lookup_field("Date") {
        // is not yet checked against bad times! we do this later if we have the database information.
        sent_timestamp = mailparse::dateparse(value).unwrap_or_default();
    }

    // get From: and check if it is known (for known From:'s we add the other To:/Cc: in the 3rd pass)
    // or if From: is equal to SELF (in this case, it is any outgoing messages,
    // we do not check Return-Path any more as this is unreliable, see issue #150
    if let Some(field_from) = mime_parser.lookup_field("From") {
        let mut check_self = false;
        let mut from_list = Vec::with_capacity(16);
        dc_add_or_lookup_contacts_by_address_list(
            context,
            &field_from,
            Origin::IncomingUnknownFrom,
            &mut from_list,
            &mut check_self,
        );
        if check_self {
            incoming = false;
            if mime_parser.sender_equals_recipient() {
                from_id = DC_CONTACT_ID_SELF;
            }
        } else if !from_list.is_empty() {
            // if there is no from given, from_id stays 0 which is just fine. These messages
            // are very rare, however, we have to add them to the database (they go to the
            // "deaddrop" chat) to avoid a re-download from the server. See also [**]
            from_id = from_list[0];
            incoming_origin = Contact::get_origin_by_id(context, from_id, &mut from_id_blocked)
        }
    }

    // Make sure, to_ids starts with the first To:-address (Cc: is added in the loop below pass)
    if let Some(field) = mime_parser.lookup_field("To") {
        dc_add_or_lookup_contacts_by_address_list(
            context,
            &field,
            if !incoming {
                Origin::OutgoingTo
            } else if incoming_origin.is_verified() {
                Origin::IncomingTo
            } else {
                Origin::IncomingUnknownTo
            },
            &mut to_ids,
            &mut to_self,
        );
    }

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
                    error!(context, "can not create incoming rfc724_mid");
                    cleanup(
                        context,
                        &create_event_to_send,
                        &created_db_entries,
                        &rr_event_to_send,
                    );
                    return;
                }
            }
        }
    };
    if mime_parser.get_last_nonmeta().is_some() {
        if let Err(err) = add_parts(
            context,
            &mut mime_parser,
            imf_raw,
            incoming,
            &mut incoming_origin,
            server_folder.as_ref(),
            server_uid,
            &mut to_ids,
            &rfc724_mid,
            &mut sent_timestamp,
            &mut from_id,
            from_id_blocked,
            &mut hidden,
            &mut chat_id,
            &mut to_id,
            flags,
            &mut needs_delete_job,
            to_self,
            &mut insert_msg_id,
            &mut created_db_entries,
            &mut create_event_to_send,
        ) {
            warn!(context, "add_parts error: {:?}", err);

            cleanup(
                context,
                &create_event_to_send,
                &created_db_entries,
                &rr_event_to_send,
            );
            return;
        }
    } else {
        // there are no non-meta data in message, do some basic calculations so that the varaiables
        // are correct in the further processing
        if sent_timestamp > time() {
            sent_timestamp = time()
        }
    }

    mime_parser.handle_reports(
        from_id,
        sent_timestamp,
        &mut rr_event_to_send,
        &server_folder,
        server_uid,
    );

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

    cleanup(
        context,
        &create_event_to_send,
        &created_db_entries,
        &rr_event_to_send,
    );
}

fn add_parts(
    context: &Context,
    mut mime_parser: &mut MimeParser,
    imf_raw: &[u8],
    incoming: bool,
    incoming_origin: &mut Origin,
    server_folder: impl AsRef<str>,
    server_uid: u32,
    to_ids: &mut Vec<u32>,
    rfc724_mid: &str,
    sent_timestamp: &mut i64,
    from_id: &mut u32,
    from_id_blocked: i32,
    hidden: &mut bool,
    chat_id: &mut u32,
    to_id: &mut u32,
    flags: u32,
    needs_delete_job: &mut bool,
    to_self: bool,
    insert_msg_id: &mut MsgId,
    created_db_entries: &mut Vec<(usize, MsgId)>,
    create_event_to_send: &mut Option<CreateEvent>,
) -> Result<()> {
    let mut state: MessageState;
    let mut msgrmsg: i32;
    let mut chat_id_blocked = Blocked::Not;
    let mut sort_timestamp = 0;
    let mut rcvd_timestamp = 0;
    let mut mime_in_reply_to = String::new();
    let mut mime_references = String::new();

    // collect the rest information, CC: is added to the to-list, BCC: is ignored
    // (we should not add BCC to groups as this would split groups. We could add them as "known contacts",
    // however, the benefit is very small and this may leak data that is expected to be hidden)
    if let Some(fld_cc) = mime_parser.lookup_field("Cc") {
        dc_add_or_lookup_contacts_by_address_list(
            context,
            fld_cc,
            if !incoming {
                Origin::OutgoingCc
            } else if incoming_origin.is_verified() {
                Origin::IncomingCc
            } else {
                Origin::IncomingUnknownCc
            },
            to_ids,
            &mut false,
        );
    }

    // check, if the mail is already in our database - if so, just update the folder/uid
    // (if the mail was moved around) and finish. (we may get a mail twice eg. if it is
    // moved between folders. make sure, this check is done eg. before securejoin-processing) */
    if let Ok((old_server_folder, old_server_uid, _)) =
        message::rfc724_mid_exists(context, &rfc724_mid)
    {
        if old_server_folder != server_folder.as_ref() || old_server_uid != server_uid {
            message::update_server_uid(context, &rfc724_mid, server_folder.as_ref(), server_uid);
        }

        bail!("Message already in DB");
    }

    // 1 or 0 for yes/no
    msgrmsg = mime_parser.has_chat_version() as _;
    if msgrmsg == 0 && is_reply_to_messenger_message(context, mime_parser) {
        // 2=no, but is reply to messenger message
        msgrmsg = 2;
    }
    // incoming non-chat messages may be discarded;
    // maybe this can be optimized later, by checking the state before the message body is downloaded
    let mut allow_creation = true;
    let show_emails =
        ShowEmails::from_i32(context.get_config_int(Config::ShowEmails)).unwrap_or_default();
    if mime_parser.is_system_message != SystemMessage::AutocryptSetupMessage && msgrmsg == 0 {
        // this message is a classic email not a chat-message nor a reply to one
        if show_emails == ShowEmails::Off {
            *chat_id = DC_CHAT_ID_TRASH;
            allow_creation = false
        } else if show_emails == ShowEmails::AcceptedContacts {
            allow_creation = false
        }
    }

    // check if the message introduces a new chat:
    // - outgoing messages introduce a chat with the first to: address if they are sent by a messenger
    // - incoming messages introduce a chat only for known contacts if they are sent by a messenger
    // (of course, the user can add other chats manually later)
    if incoming {
        state = if 0 != flags & DC_IMAP_SEEN {
            MessageState::InSeen
        } else {
            MessageState::InFresh
        };
        *to_id = DC_CONTACT_ID_SELF;
        let mut needs_stop_ongoing_process = false;

        // handshake messages must be processed _before_ chats are created
        // (eg. contacs may be marked as verified)
        if mime_parser.lookup_field("Secure-Join").is_some() {
            // avoid discarding by show_emails setting
            msgrmsg = 1;
            *chat_id = 0;
            allow_creation = true;
            match handle_securejoin_handshake(context, mime_parser, *from_id) {
                Ok(ret) => {
                    if ret.hide_this_msg {
                        *hidden = true;
                        *needs_delete_job = ret.delete_this_msg;
                        state = MessageState::InSeen;
                    }
                    if let Some(status) = ret.bob_securejoin_success {
                        context.bob.write().unwrap().status = status as i32;
                    }
                    needs_stop_ongoing_process = ret.stop_ongoing_process;
                }
                Err(err) => {
                    warn!(
                        context,
                        "Unexpected messaged passed to Secure-Join handshake protocol: {}", err
                    );
                }
            }
        }

        let (test_normal_chat_id, test_normal_chat_id_blocked) =
            chat::lookup_by_contact_id(context, *from_id).unwrap_or_default();

        // get the chat_id - a chat_id here is no indicator that the chat is displayed in the normal list,
        // it might also be blocked and displayed in the deaddrop as a result
        if *chat_id == 0 {
            // try to create a group
            // (groups appear automatically only if the _sender_ is known, see core issue #54)

            let create_blocked =
                if 0 != test_normal_chat_id && test_normal_chat_id_blocked == Blocked::Not {
                    Blocked::Not
                } else {
                    Blocked::Deaddrop
                };

            let (new_chat_id, new_chat_id_blocked) = create_or_lookup_group(
                context,
                &mut mime_parser,
                allow_creation,
                create_blocked,
                *from_id,
                to_ids,
            )?;
            *chat_id = new_chat_id;
            chat_id_blocked = new_chat_id_blocked;
            if *chat_id != 0 && chat_id_blocked != Blocked::Not && create_blocked == Blocked::Not {
                chat::unblock(context, new_chat_id);
                chat_id_blocked = Blocked::Not;
            }
        }

        if *chat_id == 0 {
            // check if the message belongs to a mailing list
            if mime_parser.is_mailinglist_message() {
                *chat_id = DC_CHAT_ID_TRASH;
                info!(context, "Message belongs to a mailing list and is ignored.",);
            }
        }

        if *chat_id == 0 {
            // try to create a normal chat
            let create_blocked = if *from_id == *to_id {
                Blocked::Not
            } else {
                Blocked::Deaddrop
            };

            if 0 != test_normal_chat_id {
                *chat_id = test_normal_chat_id;
                chat_id_blocked = test_normal_chat_id_blocked;
            } else if allow_creation {
                let (id, bl) =
                    chat::create_or_lookup_by_contact_id(context, *from_id, create_blocked)
                        .unwrap_or_default();
                *chat_id = id;
                chat_id_blocked = bl;
            }
            if 0 != *chat_id && Blocked::Not != chat_id_blocked {
                if Blocked::Not == create_blocked {
                    chat::unblock(context, *chat_id);
                    chat_id_blocked = Blocked::Not;
                } else if is_reply_to_known_message(context, mime_parser) {
                    //  we do not want any chat to be created implicitly.  Because of the origin-scale-up,
                    // the contact requests will pop up and this should be just fine.
                    Contact::scaleup_origin_by_id(context, *from_id, Origin::IncomingReplyTo);
                    info!(
                        context,
                        "Message is a reply to a known message, mark sender as known.",
                    );
                    if !incoming_origin.is_verified() {
                        *incoming_origin = Origin::IncomingReplyTo;
                    }
                }
            }
        }
        if *chat_id == 0 {
            // maybe from_id is null or sth. else is suspicious, move message to trash
            *chat_id = DC_CHAT_ID_TRASH;
        }

        // if the chat_id is blocked,
        // for unknown senders and non-delta-messages set the state to NOTICED
        // to not result in a chatlist-contact-request (this would require the state FRESH)
        if Blocked::Not != chat_id_blocked
            && state == MessageState::InFresh
            && !incoming_origin.is_verified()
            && msgrmsg == 0
            && show_emails != ShowEmails::All
        {
            state = MessageState::InNoticed;
        }

        if needs_stop_ongoing_process {
            // The Secure-Join protocol finished and the group
            // creation handling is done.  Stopping the ongoing
            // process will let dc_join_securejoin() return.
            context.stop_ongoing();
        }
    } else {
        // Outgoing

        // the mail is on the IMAP server, probably it is also delivered.
        // We cannot recreate other states (read, error).
        state = MessageState::OutDelivered;
        *from_id = DC_CONTACT_ID_SELF;
        if !to_ids.is_empty() {
            *to_id = to_ids[0];
            if *chat_id == 0 {
                let (new_chat_id, new_chat_id_blocked) = create_or_lookup_group(
                    context,
                    &mut mime_parser,
                    allow_creation,
                    Blocked::Not,
                    *from_id,
                    to_ids,
                )?;
                *chat_id = new_chat_id;
                chat_id_blocked = new_chat_id_blocked;
                // automatically unblock chat when the user sends a message
                if *chat_id != 0 && chat_id_blocked != Blocked::Not {
                    chat::unblock(context, new_chat_id);
                    chat_id_blocked = Blocked::Not;
                }
            }
            if *chat_id == 0 && allow_creation {
                let create_blocked = if 0 != msgrmsg && !Contact::is_blocked_load(context, *to_id) {
                    Blocked::Not
                } else {
                    Blocked::Deaddrop
                };
                let (id, bl) =
                    chat::create_or_lookup_by_contact_id(context, *to_id, create_blocked)
                        .unwrap_or_default();
                *chat_id = id;
                chat_id_blocked = bl;

                if 0 != *chat_id
                    && Blocked::Not != chat_id_blocked
                    && Blocked::Not == create_blocked
                {
                    chat::unblock(context, *chat_id);
                    chat_id_blocked = Blocked::Not;
                }
            }
        }
        if *chat_id == 0 && to_ids.is_empty() && to_self {
            // from_id==to_id==DC_CONTACT_ID_SELF - this is a self-sent messages,
            // maybe an Autocrypt Setup Messag
            let (id, bl) =
                chat::create_or_lookup_by_contact_id(context, DC_CONTACT_ID_SELF, Blocked::Not)
                    .unwrap_or_default();
            *chat_id = id;
            chat_id_blocked = bl;

            if 0 != *chat_id && Blocked::Not != chat_id_blocked {
                chat::unblock(context, *chat_id);
                chat_id_blocked = Blocked::Not;
            }
        }
        if *chat_id == 0 {
            *chat_id = DC_CHAT_ID_TRASH;
        }
    }
    // correct message_timestamp, it should not be used before,
    // however, we cannot do this earlier as we need from_id to be set
    calc_timestamps(
        context,
        *chat_id,
        *from_id,
        *sent_timestamp,
        0 == flags & DC_IMAP_SEEN,
        &mut sort_timestamp,
        sent_timestamp,
        &mut rcvd_timestamp,
    );

    // unarchive chat
    chat::unarchive(context, *chat_id)?;

    // if the mime-headers should be saved, find out its size
    // (the mime-header ends with an empty line)
    let save_mime_headers = context.get_config_bool(Config::SaveMimeHeaders);
    if let Some(raw) = mime_parser.lookup_field("In-Reply-To") {
        mime_in_reply_to = raw.clone();
    }

    if let Some(raw) = mime_parser.lookup_field("References") {
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
         bytes, hidden, mime_headers,  mime_in_reply_to, mime_references) \
         VALUES (?,?,?,?,?,?, ?,?,?,?,?,?, ?,?,?,?,?,?, ?,?);",
        |mut stmt, conn| {
            let subject = mime_parser.get_subject().unwrap_or_default();

            for part in mime_parser.parts.iter_mut() {
                if part.is_meta {
                    continue;
                }

                if mime_parser.location_kml.is_some()
                    && icnt == 1
                    && (part.msg == "-location-" || part.msg.is_empty())
                {
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
                    *chat_id as i32,
                    *from_id as i32,
                    *to_id as i32,
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
                ])?;

                txt_raw = None;
                let row_id =
                    sql::get_rowid_with_conn(context, conn, "msgs", "rfc724_mid", &rfc724_mid);
                *insert_msg_id = MsgId::new(row_id);
                created_db_entries.push((*chat_id as usize, *insert_msg_id));
            }
            Ok(())
        },
    )?;

    info!(
        context,
        "Message has {} parts and is assigned to chat #{}.", icnt, *chat_id,
    );

    // check event to send
    if *chat_id == DC_CHAT_ID_TRASH {
        *create_event_to_send = None;
    } else if incoming && state == MessageState::InFresh {
        if 0 != from_id_blocked {
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
    mime_parser: &MimeParser,
    chat_id: u32,
    from_id: u32,
    insert_msg_id: MsgId,
    hidden: bool,
) {
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL {
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

fn calc_timestamps(
    context: &Context,
    chat_id: u32,
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
            params![chat_id as i32, from_id as i32, *sort_timestamp],
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
#[allow(non_snake_case)]
fn create_or_lookup_group(
    context: &Context,
    mime_parser: &mut MimeParser,
    allow_creation: bool,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &[u32],
) -> Result<(u32, Blocked)> {
    let mut chat_id_blocked = Blocked::Not;
    let mut grpname = None;
    let to_ids_cnt = to_ids.len();
    let mut recreate_member_list = false;
    let mut send_EVENT_CHAT_MODIFIED = false;
    let mut X_MrRemoveFromGrp = None;
    let mut X_MrAddToGrp = None;
    let mut X_MrGrpNameChanged = false;
    let mut X_MrGrpImageChanged = "".to_string();
    let mut better_msg: String = From::from("");

    if mime_parser.is_system_message == SystemMessage::LocationStreamingEnabled {
        better_msg =
            context.stock_system_msg(StockMessage::MsgLocationEnabled, "", "", from_id as u32)
    }
    set_better_msg(mime_parser, &better_msg);

    let mut grpid = "".to_string();
    if let Some(optional_field) = mime_parser.lookup_field("Chat-Group-ID") {
        grpid = optional_field.clone();
    }

    if grpid.is_empty() {
        if let Some(value) = mime_parser.lookup_field("Message-ID") {
            if let Some(extracted_grpid) = dc_extract_grpid_from_rfc724_mid(&value) {
                grpid = extracted_grpid.to_string();
            }
        }
        if grpid.is_empty() {
            if let Some(extracted_grpid) = get_grpid_from_list(mime_parser, "In-Reply-To") {
                grpid = extracted_grpid;
            } else if let Some(extracted_grpid) = get_grpid_from_list(mime_parser, "References") {
                grpid = extracted_grpid;
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

    if let Some(optional_field) = mime_parser.lookup_field("Chat-Group-Name").cloned() {
        grpname = Some(optional_field);
    }
    let field = mime_parser
        .lookup_field("Chat-Group-Member-Removed")
        .cloned();
    if let Some(optional_field) = field {
        X_MrRemoveFromGrp = Some(optional_field);
        mime_parser.is_system_message = SystemMessage::MemberRemovedFromGroup;
        let left_group = Contact::lookup_id_by_addr(context, X_MrRemoveFromGrp.as_ref().unwrap())
            == from_id as u32;
        better_msg = context.stock_system_msg(
            if left_group {
                StockMessage::MsgGroupLeft
            } else {
                StockMessage::MsgDelMember
            },
            X_MrRemoveFromGrp.as_ref().unwrap(),
            "",
            from_id as u32,
        )
    } else {
        let field = mime_parser.lookup_field("Chat-Group-Member-Added").cloned();
        if let Some(optional_field) = field {
            X_MrAddToGrp = Some(optional_field);
            mime_parser.is_system_message = SystemMessage::MemberAddedToGroup;
            if let Some(optional_field) = mime_parser.lookup_field("Chat-Group-Image").cloned() {
                X_MrGrpImageChanged = optional_field;
            }
            better_msg = context.stock_system_msg(
                StockMessage::MsgAddMember,
                X_MrAddToGrp.as_ref().unwrap(),
                "",
                from_id as u32,
            )
        } else {
            let field = mime_parser.lookup_field("Chat-Group-Name-Changed");
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
            } else if let Some(optional_field) =
                mime_parser.lookup_field("Chat-Group-Image").cloned()
            {
                // fld_value is a pointer somewhere into mime_parser, must not be freed
                X_MrGrpImageChanged = optional_field;
                mime_parser.is_system_message = SystemMessage::GroupImageChanged;
                better_msg = context.stock_system_msg(
                    if X_MrGrpImageChanged == "0" {
                        StockMessage::MsgGrpImgDeleted
                    } else {
                        StockMessage::MsgGrpImgChanged
                    },
                    "",
                    "",
                    from_id as u32,
                )
            }
        }
    }
    set_better_msg(mime_parser, &better_msg);

    // check, if we have a chat with this group ID
    let (mut chat_id, chat_id_verified, _blocked) = chat::get_chat_id_by_grpid(context, &grpid);
    if chat_id != 0 {
        if chat_id_verified {
            if let Err(err) =
                check_verified_properties(context, mime_parser, from_id as u32, to_ids)
            {
                warn!(context, "verification problem: {}", err);
                let s = format!("{}. See 'Info' for more details", err);
                mime_parser.repl_msg_by_error(s);
            }
        }
        // check if the sender is a member of the existing group -
        // if not, we'll recreate the group list
        if !chat::is_contact_in_chat(context, chat_id, from_id as u32) {
            recreate_member_list = true;
        }
    }

    // check if the group does not exist but should be created
    let group_explicitly_left = chat::is_group_explicitly_left(context, &grpid).unwrap_or_default();
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_default();

    if chat_id == 0
            && !mime_parser.is_mailinglist_message()
            && !grpid.is_empty()
            && grpname.is_some()
            // otherwise, a pending "quit" message may pop up
            && X_MrRemoveFromGrp.is_none()
            // re-create explicitly left groups only if ourself is re-added
            && (!group_explicitly_left
                || X_MrAddToGrp.is_some() && addr_cmp(&self_addr, X_MrAddToGrp.as_ref().unwrap()))
    {
        // group does not exist but should be created
        let create_verified = if mime_parser.lookup_field("Chat-Verified").is_some() {
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
            return Ok((0, Blocked::Not));
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
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL {
        return if group_explicitly_left {
            Ok((DC_CHAT_ID_TRASH, chat_id_blocked))
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

    // execute group commands
    if X_MrAddToGrp.is_some() || X_MrRemoveFromGrp.is_some() {
        recreate_member_list = true;
    } else if X_MrGrpNameChanged {
        if let Some(ref grpname) = grpname {
            if grpname.len() < 200 {
                info!(context, "updating grpname for chat {}", chat_id);
                if sql::execute(
                    context,
                    &context.sql,
                    "UPDATE chats SET name=? WHERE id=?;",
                    params![grpname, chat_id as i32],
                )
                .is_ok()
                {
                    context.call_cb(Event::ChatModified(chat_id));
                }
            }
        }
    }
    if !X_MrGrpImageChanged.is_empty() {
        info!(
            context,
            "grp-image-change {} chat {}", X_MrGrpImageChanged, chat_id
        );
        let mut changed = false;
        let mut grpimage: Option<BlobObject> = None;
        if X_MrGrpImageChanged == "0" {
            changed = true;
        } else {
            for part in &mut mime_parser.parts {
                if part.typ == Viewtype::Image {
                    grpimage = part
                        .param
                        .get_blob(Param::File, context, true)
                        .unwrap_or(None);
                    info!(context, "found image {:?}", grpimage);
                    changed = true;
                }
            }
        }
        if changed {
            info!(
                context,
                "New group image set to '{}'.",
                grpimage
                    .as_ref()
                    .map(|blob| blob.as_name().to_string())
                    .unwrap_or_default()
            );
            if let Ok(mut chat) = Chat::load_from_db(context, chat_id) {
                match grpimage {
                    Some(blob) => chat.param.set(Param::ProfileImage, blob.as_name()),
                    None => chat.param.remove(Param::ProfileImage),
                };
                chat.update_param(context)?;
                send_EVENT_CHAT_MODIFIED = true;
            }
        }
    }

    // add members to group/check members
    // for recreation: we should add a timestamp
    if recreate_member_list {
        // TODO: the member list should only be recreated if the corresponding message is newer
        // than the one that is responsible for the current member list, see
        // https://github.com/deltachat/deltachat-core/issues/127

        let skip = X_MrRemoveFromGrp.as_ref();
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM chats_contacts WHERE chat_id=?;",
            params![chat_id as i32],
        )
        .ok();
        if skip.is_none() || !addr_cmp(&self_addr, skip.unwrap()) {
            chat::add_to_chat_contacts_table(context, chat_id, DC_CONTACT_ID_SELF);
        }
        if from_id > DC_CHAT_ID_LAST_SPECIAL
            && !Contact::addr_equals_contact(context, &self_addr, from_id as u32)
            && (skip.is_none()
                || !Contact::addr_equals_contact(context, skip.unwrap(), from_id as u32))
        {
            chat::add_to_chat_contacts_table(context, chat_id, from_id as u32);
        }
        for &to_id in to_ids.iter() {
            if !Contact::addr_equals_contact(context, &self_addr, to_id)
                && (skip.is_none() || !Contact::addr_equals_contact(context, skip.unwrap(), to_id))
            {
                chat::add_to_chat_contacts_table(context, chat_id, to_id);
            }
        }
        send_EVENT_CHAT_MODIFIED = true;
        chat::reset_gossiped_timestamp(context, chat_id);
    }

    if send_EVENT_CHAT_MODIFIED {
        context.call_cb(Event::ChatModified(chat_id));
    }

    // check the number of receivers -
    // the only critical situation is if the user hits "Reply" instead
    // of "Reply all" in a non-messenger-client */
    if to_ids_cnt == 1
        && !mime_parser.has_chat_version()
        && chat::get_chat_contact_cnt(context, chat_id) > 3
    {
        // to_ids_cnt==1 may be "From: A, To: B, SELF" as SELF is not counted in to_ids_cnt.
        // So everything up to 3 is no error.
        create_or_lookup_adhoc_group(
            context,
            mime_parser,
            allow_creation,
            create_blocked,
            from_id,
            to_ids,
        )
        .map_err(|err| {
            warn!(context, "could not create ad-hoc group: {:?}", err);
            err
        })?;
    }
    Ok((chat_id, chat_id_blocked))
}

/// try extract a grpid from a message-id list header value
fn get_grpid_from_list(mime_parser: &MimeParser, header_key: &str) -> Option<String> {
    if let Some(value) = mime_parser.lookup_field(header_key) {
        for part in value.split(',').map(str::trim) {
            if !part.is_empty() {
                if let Some(extracted_grpid) = dc_extract_grpid_from_rfc724_mid(part) {
                    return Some(extracted_grpid.to_string());
                }
            }
        }
    }
    None
}

/// Handle groups for received messages, return chat_id/Blocked status on success
fn create_or_lookup_adhoc_group(
    context: &Context,
    mime_parser: &MimeParser,
    allow_creation: bool,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &[u32],
) -> Result<(u32, Blocked)> {
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
        return Ok((0, Blocked::Not));
    }

    let mut member_ids = to_ids.to_vec();
    if !member_ids.contains(&from_id) {
        member_ids.push(from_id);
    }
    if !member_ids.contains(&DC_CONTACT_ID_SELF) {
        member_ids.push(DC_CONTACT_ID_SELF);
    }

    if member_ids.len() < 3 {
        info!(context, "not creating ad-hoc group: too few contacts");
        return Ok((0, Blocked::Not));
    }

    let chat_ids = search_chat_ids_by_contact_ids(context, &member_ids)?;
    if !chat_ids.is_empty() {
        let chat_ids_str = join(chat_ids.iter().map(|x| x.to_string()), ",");
        let res = context.sql.query_row(
            format!(
                "SELECT c.id, c.blocked  FROM chats c  \
                 LEFT JOIN msgs m ON m.chat_id=c.id  WHERE c.id IN({})  ORDER BY m.timestamp DESC, m.id DESC  LIMIT 1;",
                chat_ids_str
            ),
            params![],
            |row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, Option<Blocked>>(1)?.unwrap_or_default()))
            }
        );

        if let Ok((id, id_blocked)) = res {
            /* success, chat found */
            return Ok((id as u32, id_blocked));
        }
    }

    if !allow_creation {
        info!(context, "creating ad-hoc group prevented from caller");
        return Ok((0, Blocked::Not));
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
        return Ok((0, Blocked::Not));
    }
    // use subject as initial chat name
    let grpname = mime_parser.get_subject().unwrap_or_else(|| {
        context.stock_string_repl_int(StockMessage::Member, member_ids.len() as i32)
    });

    // create group record
    let new_chat_id = create_group_record(
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
) -> u32 {
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
        return 0;
    }

    sql::get_rowid(context, &context.sql, "chats", "grpid", grpid.as_ref())
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

#[allow(non_snake_case)]
fn search_chat_ids_by_contact_ids(
    context: &Context,
    unsorted_contact_ids: &[u32],
) -> Result<Vec<u32>> {
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
                    "SELECT DISTINCT cc.chat_id, cc.contact_id \
                       FROM chats_contacts cc \
                       LEFT JOIN chats c ON c.id=cc.chat_id \
                       WHERE cc.chat_id IN(SELECT chat_id FROM chats_contacts WHERE contact_id IN({})) \
                         AND c.type=120 \
                         AND cc.contact_id!=1 \
                       ORDER BY cc.chat_id, cc.contact_id;", // 1=DC_CONTACT_ID_SELF
                    contact_ids_str
                ),
                params![],
                |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)),
                |rows| {
                    let mut last_chat_id = 0;
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
    mimeparser: &MimeParser,
    from_id: u32,
    to_ids: &[u32],
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

    let to_ids_str = join(to_ids.iter().map(|x| x.to_string()), ",");

    let rows = context.sql.query_map(
        format!(
            "SELECT c.addr, LENGTH(ps.verified_key_fingerprint)  FROM contacts c  \
             LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.id IN({}) ",
            to_ids_str,
        ),
        params![],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1).unwrap_or(0))),
        |rows| {
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        },
    )?;

    for (to_addr, _is_verified) in rows.into_iter() {
        let mut is_verified = _is_verified != 0;
        let mut peerstate = Peerstate::from_addr(context, &context.sql, &to_addr);

        // mark gossiped keys (if any) as verified
        if mimeparser.gossipped_addr.contains(&to_addr) && peerstate.is_some() {
            let peerstate = peerstate.as_mut().unwrap();

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
        if !is_verified {
            bail!(
                "{} is not a member of this verified group",
                to_addr.to_string()
            );
        }
    }
    Ok(())
}

fn set_better_msg(mime_parser: &mut MimeParser, better_msg: impl AsRef<str>) {
    let msg = better_msg.as_ref();
    if !msg.is_empty() && !mime_parser.parts.is_empty() {
        let part = &mut mime_parser.parts[0];
        if part.typ == Viewtype::Text {
            part.msg = msg.to_string();
        }
    }
}

fn is_reply_to_known_message(context: &Context, mime_parser: &MimeParser) -> bool {
    /* check if the message is a reply to a known message; the replies are identified by the Message-ID from
    `In-Reply-To`/`References:` (to support non-Delta-Clients) */

    if let Some(field) = mime_parser.lookup_field("In-Reply-To") {
        if is_known_rfc724_mid_in_list(context, &field) {
            return true;
        }
    }

    if let Some(field) = mime_parser.lookup_field("References") {
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

    if let Ok(ids) = mailparse::addrparse(mid_list) {
        for id in ids.iter() {
            if is_known_rfc724_mid(context, id) {
                return true;
            }
        }
    }

    false
}

/// Check if a message is a reply to a known message (messenger or non-messenger).
fn is_known_rfc724_mid(context: &Context, rfc724_mid: &mailparse::MailAddr) -> bool {
    let addr = extract_single_from_addr(rfc724_mid);
    context
        .sql
        .exists(
            "SELECT m.id FROM msgs m  \
             LEFT JOIN chats c ON m.chat_id=c.id  \
             WHERE m.rfc724_mid=?  \
             AND m.chat_id>9 AND c.blocked=0;",
            params![addr],
        )
        .unwrap_or_default()
}

/// Checks if the message defined by mime_parser references a message send by us from Delta Chat.
/// This is similar to is_reply_to_known_message() but
/// - checks also if any of the referenced IDs are send by a messenger
/// - it is okay, if the referenced messages are moved to trash here
/// - no check for the Chat-* headers (function is only called if it is no messenger message itself)
fn is_reply_to_messenger_message(context: &Context, mime_parser: &MimeParser) -> bool {
    if let Some(value) = mime_parser.lookup_field("In-Reply-To") {
        if is_msgrmsg_rfc724_mid_in_list(context, &value) {
            return true;
        }
    }

    if let Some(value) = mime_parser.lookup_field("References") {
        if is_msgrmsg_rfc724_mid_in_list(context, &value) {
            return true;
        }
    }

    false
}

fn is_msgrmsg_rfc724_mid_in_list(context: &Context, mid_list: &str) -> bool {
    if let Ok(ids) = mailparse::addrparse(mid_list) {
        for id in ids.iter() {
            if is_msgrmsg_rfc724_mid(context, id) {
                return true;
            }
        }
    }
    false
}

fn extract_single_from_addr(addr: &mailparse::MailAddr) -> &String {
    match addr {
        mailparse::MailAddr::Group(infos) => &infos.addrs[0].addr,
        mailparse::MailAddr::Single(info) => &info.addr,
    }
}

/// Check if a message is a reply to any messenger message.
fn is_msgrmsg_rfc724_mid(context: &Context, rfc724_mid: &mailparse::MailAddr) -> bool {
    let addr = extract_single_from_addr(rfc724_mid);
    context
        .sql
        .exists(
            "SELECT id FROM msgs  WHERE rfc724_mid=?  AND msgrmsg!=0  AND chat_id>9;",
            params![addr],
        )
        .unwrap_or_default()
}

fn dc_add_or_lookup_contacts_by_address_list(
    context: &Context,
    addr_list_raw: &str,
    origin: Origin,
    ids: &mut Vec<u32>,
    check_self: &mut bool,
) {
    let addrs = mailparse::addrparse(addr_list_raw);
    if addrs.is_err() {
        return;
    }
    for addr in addrs.unwrap().iter() {
        match addr {
            mailparse::MailAddr::Single(info) => {
                add_or_lookup_contact_by_addr(
                    context,
                    &info.display_name,
                    &info.addr,
                    origin,
                    ids,
                    check_self,
                );
            }
            mailparse::MailAddr::Group(infos) => {
                for info in &infos.addrs {
                    add_or_lookup_contact_by_addr(
                        context,
                        &info.display_name,
                        &info.addr,
                        origin,
                        ids,
                        check_self,
                    );
                }
            }
        }
    }
}

/// Add contacts to database on receiving messages.
fn add_or_lookup_contact_by_addr(
    context: &Context,
    display_name: &Option<String>,
    addr: &str,
    origin: Origin,
    ids: &mut Vec<u32>,
    check_self: &mut bool,
) {
    // is addr_spec equal to SELF?
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_default();

    if addr_cmp(self_addr, addr) {
        *check_self = true;
    }

    if *check_self {
        return;
    }

    // add addr_spec if missing, update otherwise
    let display_name_normalized = display_name
        .as_ref()
        .map(normalize_name)
        .unwrap_or_default();

    // can be NULL
    let row_id = Contact::add_or_lookup(context, display_name_normalized, addr, origin)
        .map(|(id, _)| id)
        .unwrap_or_default();

    if 0 != row_id && !ids.contains(&row_id) {
        ids.push(row_id);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::dummy_context;

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
        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();
        assert_eq!(get_grpid_from_list(&mimeparser, "In-Reply-To"), None);
        let grpid = Some("HcxyMARjyJy".to_string());
        assert_eq!(get_grpid_from_list(&mimeparser, "References"), grpid);
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
        let mimeparser = MimeParser::from_bytes(&context.ctx, &raw[..]).unwrap();
        let grpid = Some("HcxyMARjyJy".to_string());
        assert_eq!(get_grpid_from_list(&mimeparser, "In-Reply-To"), grpid);
        assert_eq!(get_grpid_from_list(&mimeparser, "References"), grpid);
    }
}

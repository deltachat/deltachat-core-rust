use std::ptr;

use itertools::join;
use libc::strcmp;
use mmime::clist::*;
use mmime::mailimf::types::*;
use mmime::mailmime::content::*;
use mmime::mailmime::types::*;
use mmime::mailmime::*;
use mmime::other::*;
use sha2::{Digest, Sha256};

use num_traits::FromPrimitive;

use crate::blob::BlobObject;
use crate::chat::{self, Chat};
use crate::config::Config;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_mimeparser::*;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::error::Result;
use crate::events::Event;
use crate::job::*;
use crate::location;
use crate::message::{self, MessageState, MsgId};
use crate::param::*;
use crate::peerstate::*;
use crate::securejoin::handle_securejoin_handshake;
use crate::sql;
use crate::stock::StockMessage;
use crate::wrapmime;

#[derive(Debug, PartialEq, Eq)]
enum CreateEvent {
    MsgsChanged,
    IncomingMsg,
}

/// Receive a message and add it to the database.
pub unsafe fn dc_receive_imf(
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

    // Parse the imf to mailimf_message. normally, this is done by mailimf_message_parse(),
    // however, as we also need the MIME data,
    // we use mailmime_parse() through dc_mimeparser (both call mailimf_struct_multiple_parse()
    // somewhen, I did not found out anything that speaks against this approach yet)

    let mut mime_parser = MimeParser::new(context);
    if let Err(err) = mime_parser.parse(imf_raw) {
        warn!(context, "dc_receive_imf parse error: {}", err);
    };

    if mime_parser.header.is_empty() {
        // Error - even adding an empty record won't help as we do not know the message ID
        warn!(context, "No header.");
        return;
    }

    // the function returns the number of created messages in the database
    let mut incoming = 1;
    let mut incoming_origin = Origin::Unknown;
    let mut to_self = 0;
    let mut from_id = 0u32;
    let mut from_id_blocked = 0;
    let mut to_id = 0u32;
    let mut chat_id = 0;
    let mut hidden = 0;

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

    if let Some(field) = mime_parser.lookup_field_typ("Date", MAILIMF_FIELD_ORIG_DATE) {
        let orig_date = (*field).fld_data.fld_orig_date;
        if !orig_date.is_null() {
            // is not yet checked against bad times! we do this later if we have the database information.
            sent_timestamp = dc_timestamp_from_date((*orig_date).dt_date_time)
        }
    }

    // get From: and check if it is known (for known From:'s we add the other To:/Cc: in the 3rd pass)
    // or if From: is equal to SELF (in this case, it is any outgoing messages,
    // we do not check Return-Path any more as this is unreliable, see issue #150
    if let Some(field) = mime_parser.lookup_field_typ("From", MAILIMF_FIELD_FROM) {
        let fld_from = (*field).fld_data.fld_from;
        if !fld_from.is_null() {
            let mut check_self = 0;
            let mut from_list = Vec::with_capacity(16);
            dc_add_or_lookup_contacts_by_mailbox_list(
                context,
                (*fld_from).frm_mb_list,
                Origin::IncomingUnknownFrom,
                &mut from_list,
                &mut check_self,
            );
            if 0 != check_self {
                incoming = 0;
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
    }

    // Make sure, to_ids starts with the first To:-address (Cc: is added in the loop below pass)
    if let Some(field) = mime_parser.lookup_field_typ("To", MAILIMF_FIELD_TO) {
        let fld_to = (*field).fld_data.fld_to;
        if !fld_to.is_null() {
            dc_add_or_lookup_contacts_by_address_list(
                context,
                (*fld_to).to_addr_list,
                if 0 == incoming {
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
    }

    // Add parts

    let rfc724_mid = match mime_parser.get_rfc724_mid() {
        Some(x) => x,
        None => {
            // missing Message-IDs may come if the mail was set from this account with another
            // client that relies in the SMTP server to generate one.
            // true eg. for the Webmailer used in all-inkl-KAS
            match dc_create_incoming_rfc724_mid(sent_timestamp, from_id, &to_ids) {
                Some(x) => x.to_string(),
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
            warn!(context, "{}", err);

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

    if !mime_parser.reports.is_empty() {
        handle_reports(
            context,
            &mime_parser,
            from_id,
            sent_timestamp,
            &mut rr_event_to_send,
            &server_folder,
            server_uid,
        );
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

unsafe fn add_parts(
    context: &Context,
    mut mime_parser: &mut MimeParser,
    imf_raw: &[u8],
    incoming: i32,
    incoming_origin: &mut Origin,
    server_folder: impl AsRef<str>,
    server_uid: u32,
    to_ids: &mut Vec<u32>,
    rfc724_mid: &str,
    sent_timestamp: &mut i64,
    from_id: &mut u32,
    from_id_blocked: i32,
    hidden: &mut libc::c_int,
    chat_id: &mut u32,
    to_id: &mut u32,
    flags: u32,
    needs_delete_job: &mut bool,
    to_self: i32,
    insert_msg_id: &mut MsgId,
    created_db_entries: &mut Vec<(usize, MsgId)>,
    create_event_to_send: &mut Option<CreateEvent>,
) -> Result<()> {
    let mut state: MessageState;
    let mut msgrmsg: libc::c_int;
    let mut chat_id_blocked = Blocked::Not;
    let mut sort_timestamp = 0;
    let mut rcvd_timestamp = 0;
    let mut mime_in_reply_to = String::new();
    let mut mime_references = String::new();

    // collect the rest information, CC: is added to the to-list, BCC: is ignored
    // (we should not add BCC to groups as this would split groups. We could add them as "known contacts",
    // however, the benefit is very small and this may leak data that is expected to be hidden)
    if let Some(field) = mime_parser.lookup_field_typ("Cc", MAILIMF_FIELD_CC) {
        let fld_cc = (*field).fld_data.fld_cc;
        if !fld_cc.is_null() {
            dc_add_or_lookup_contacts_by_address_list(
                context,
                (*fld_cc).cc_addr_list,
                if 0 == incoming {
                    Origin::OutgoingCc
                } else if incoming_origin.is_verified() {
                    Origin::IncomingCc
                } else {
                    Origin::IncomingUnknownCc
                },
                to_ids,
                std::ptr::null_mut(),
            );
        }
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
    msgrmsg = mime_parser.is_send_by_messenger as _;
    if msgrmsg == 0 && 0 != dc_is_reply_to_messenger_message(context, mime_parser) {
        // 2=no, but is reply to messenger message
        msgrmsg = 2;
    }
    // incoming non-chat messages may be discarded;
    // maybe this can be optimized later, by checking the state before the message body is downloaded
    let mut allow_creation = 1;
    let show_emails =
        ShowEmails::from_i32(context.get_config_int(Config::ShowEmails)).unwrap_or_default();
    if mime_parser.is_system_message != SystemMessage::AutocryptSetupMessage && msgrmsg == 0 {
        // this message is a classic email not a chat-message nor a reply to one
        if show_emails == ShowEmails::Off {
            *chat_id = DC_CHAT_ID_TRASH;
            allow_creation = 0
        } else if show_emails == ShowEmails::AcceptedContacts {
            allow_creation = 0
        }
    }

    // check if the message introduces a new chat:
    // - outgoing messages introduce a chat with the first to: address if they are sent by a messenger
    // - incoming messages introduce a chat only for known contacts if they are sent by a messenger
    // (of course, the user can add other chats manually later)
    if 0 != incoming {
        state = if 0 != flags & DC_IMAP_SEEN {
            MessageState::InSeen
        } else {
            MessageState::InFresh
        };
        *to_id = DC_CONTACT_ID_SELF;
        // handshake messages must be processed _before_ chats are created
        // (eg. contacs may be marked as verified)
        if mime_parser.lookup_field("Secure-Join").is_some() {
            // avoid discarding by show_emails setting
            msgrmsg = 1;
            *chat_id = 0;
            allow_creation = 1;
            let handshake = handle_securejoin_handshake(context, mime_parser, *from_id);
            if 0 != handshake & DC_HANDSHAKE_STOP_NORMAL_PROCESSING {
                *hidden = 1;
                *needs_delete_job = 0 != handshake & DC_HANDSHAKE_ADD_DELETE_JOB;
                state = MessageState::InSeen;
            }
        }

        let (test_normal_chat_id, test_normal_chat_id_blocked) =
            chat::lookup_by_contact_id(context, *from_id).unwrap_or_default();

        // get the chat_id - a chat_id here is no indicator that the chat is displayed in the normal list,
        // it might also be blocked and displayed in the deaddrop as a result
        if *chat_id == 0 {
            // try to create a group
            // (groups appear automatically only if the _sender_ is known, see core issue #54)

            let create_blocked = if 0 != test_normal_chat_id
                && test_normal_chat_id_blocked == Blocked::Not
                || incoming_origin.is_start_new_chat()
            {
                Blocked::Not
            } else {
                Blocked::Deaddrop
            };

            create_or_lookup_group(
                context,
                &mut mime_parser,
                allow_creation,
                create_blocked,
                *from_id,
                to_ids,
                chat_id,
                &mut chat_id_blocked,
            )?;
            if 0 != *chat_id && Blocked::Not != chat_id_blocked && create_blocked == Blocked::Not {
                chat::unblock(context, *chat_id);
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
            let create_blocked = if incoming_origin.is_start_new_chat() || *from_id == *to_id {
                Blocked::Not
            } else {
                Blocked::Deaddrop
            };

            if 0 != test_normal_chat_id {
                *chat_id = test_normal_chat_id;
                chat_id_blocked = test_normal_chat_id_blocked;
            } else if 0 != allow_creation {
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
                } else if 0 != dc_is_reply_to_known_message(context, mime_parser) {
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
    } else {
        // Outgoing

        // the mail is on the IMAP server, probably it is also delivered.
        // We cannot recreate other states (read, error).
        state = MessageState::OutDelivered;
        *from_id = DC_CONTACT_ID_SELF;
        if !to_ids.is_empty() {
            *to_id = to_ids[0];
            if *chat_id == 0 {
                create_or_lookup_group(
                    context,
                    &mut mime_parser,
                    allow_creation,
                    Blocked::Not,
                    *from_id,
                    to_ids,
                    chat_id,
                    &mut chat_id_blocked,
                )?;
                if 0 != *chat_id && Blocked::Not != chat_id_blocked {
                    chat::unblock(context, *chat_id);
                    chat_id_blocked = Blocked::Not;
                }
            }
            if *chat_id == 0 && 0 != allow_creation {
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
        if *chat_id == 0 {
            if to_ids.is_empty() && 0 != to_self {
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
        if 0 != flags & DC_IMAP_SEEN { 0 } else { 1 },
        &mut sort_timestamp,
        sent_timestamp,
        &mut rcvd_timestamp,
    );

    // unarchive chat
    chat::unarchive(context, *chat_id)?;

    // if the mime-headers should be saved, find out its size
    // (the mime-header ends with an empty line)
    let save_mime_headers = context.get_config_bool(Config::SaveMimeHeaders);
    if let Some(field) = mime_parser.lookup_field_typ("In-Reply-To", MAILIMF_FIELD_IN_REPLY_TO) {
        let fld_in_reply_to = (*field).fld_data.fld_in_reply_to;
        if !fld_in_reply_to.is_null() {
            mime_in_reply_to = dc_str_from_clist((*(*field).fld_data.fld_in_reply_to).mid_list, " ")
        }
    }

    if let Some(field) = mime_parser.lookup_field_typ("References", MAILIMF_FIELD_REFERENCES) {
        let fld_references = (*field).fld_data.fld_references;
        if !fld_references.is_null() {
            mime_references = dc_str_from_clist((*(*field).fld_data.fld_references).mid_list, " ")
        }
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
            for i in 0..icnt {
                let part = &mut mime_parser.parts[i];
                if part.is_meta {
                    continue;
                }

                if let Some(ref msg) = part.msg {
                    if mime_parser.location_kml.is_some()
                        && icnt == 1
                        && (msg == "-location-" || msg.is_empty())
                    {
                        *hidden = 1;
                        if state == MessageState::InFresh {
                            state = MessageState::InNoticed;
                        }
                    }
                }
                if part.typ == Viewtype::Text {
                    let msg_raw = part.msg_raw.as_ref().cloned().unwrap_or_default();
                    let subject = mime_parser
                        .subject
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or("".into());
                    txt_raw = Some(format!("{}\n\n{}", subject, msg_raw));
                }
                if mime_parser.is_system_message != SystemMessage::Unknown {
                    part.param
                        .set_int(Param::Cmd, mime_parser.is_system_message as i32);
                }

                stmt.execute(params![
                    rfc724_mid,
                    server_folder.as_ref(),
                    server_uid as libc::c_int,
                    *chat_id as libc::c_int,
                    *from_id as libc::c_int,
                    *to_id as libc::c_int,
                    sort_timestamp,
                    *sent_timestamp,
                    rcvd_timestamp,
                    part.typ,
                    state,
                    msgrmsg,
                    part.msg.as_ref().map_or("", String::as_str),
                    // txt_raw might contain invalid utf8
                    txt_raw.unwrap_or_default(),
                    part.param.to_string(),
                    part.bytes,
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
    } else if 0 != incoming && state == MessageState::InFresh {
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

// Handle reports (mainly MDNs)
unsafe fn handle_reports(
    context: &Context,
    mime_parser: &MimeParser,
    from_id: u32,
    sent_timestamp: i64,
    rr_event_to_send: &mut Vec<(u32, MsgId)>,
    server_folder: impl AsRef<str>,
    server_uid: u32,
) {
    let mdns_enabled = context.get_config_bool(Config::MdnsEnabled);

    for report_root in &mime_parser.reports {
        let report_root = *report_root;
        let mut mdn_consumed = 0;
        let report_type = mailmime_find_ct_parameter(report_root, "report-type");

        if report_root.is_null() || report_type.is_null() || (*report_type).pa_value.is_null() {
            continue;
        }

        // the first part is for humans, the second for machines
        if strcmp(
            (*report_type).pa_value,
            b"disposition-notification\x00" as *const u8 as *const libc::c_char,
        ) == 0
            && (*(*report_root).mm_data.mm_multipart.mm_mp_list).count >= 2
        {
            // to get a clear functionality, do not show incoming MDNs if the options is disabled
            if mdns_enabled {
                let report_data = (if !if !(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                    .first
                    .is_null()
                {
                    (*(*(*report_root).mm_data.mm_multipart.mm_mp_list).first).next
                } else {
                    ptr::null_mut()
                }
                .is_null()
                {
                    (*if !(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                        .first
                        .is_null()
                    {
                        (*(*(*report_root).mm_data.mm_multipart.mm_mp_list).first).next
                    } else {
                        ptr::null_mut()
                    })
                    .data
                } else {
                    ptr::null_mut()
                }) as *mut Mailmime;

                if !report_data.is_null()
                    && (*(*(*report_data).mm_content_type).ct_type).tp_type
                        == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int
                    && (*(*(*(*report_data).mm_content_type).ct_type)
                        .tp_data
                        .tp_composite_type)
                        .ct_type
                        == MAILMIME_COMPOSITE_TYPE_MESSAGE as libc::c_int
                    && strcmp(
                        (*(*report_data).mm_content_type).ct_subtype,
                        b"disposition-notification\x00" as *const u8 as *const libc::c_char,
                    ) == 0
                {
                    if let Ok(report_body) = wrapmime::mailmime_transfer_decode(report_data) {
                        let mut report_parsed = std::ptr::null_mut();
                        let mut dummy = 0;

                        if mailmime_parse(
                            report_body.as_ptr() as *const _,
                            report_body.len(),
                            &mut dummy,
                            &mut report_parsed,
                        ) == MAIL_NO_ERROR as libc::c_int
                            && !report_parsed.is_null()
                        {
                            let report_fields =
                                wrapmime::mailmime_find_mailimf_fields(report_parsed);
                            if !report_fields.is_null() {
                                let of_disposition = wrapmime::mailimf_find_optional_field(
                                    report_fields,
                                    b"Disposition\x00" as *const u8 as *const libc::c_char,
                                );
                                let of_org_msgid = wrapmime::mailimf_find_optional_field(
                                    report_fields,
                                    b"Original-Message-ID\x00" as *const u8 as *const libc::c_char,
                                );
                                if !of_disposition.is_null()
                                    && !(*of_disposition).fld_value.is_null()
                                    && !of_org_msgid.is_null()
                                    && !(*of_org_msgid).fld_value.is_null()
                                {
                                    if let Ok(rfc724_mid) =
                                        wrapmime::parse_message_id(std::slice::from_raw_parts(
                                            (*of_org_msgid).fld_value as *const u8,
                                            libc::strlen((*of_org_msgid).fld_value),
                                        ))
                                    {
                                        if let Some((chat_id, msg_id)) = message::mdn_from_ext(
                                            context,
                                            from_id,
                                            &rfc724_mid,
                                            sent_timestamp,
                                        ) {
                                            rr_event_to_send.push((chat_id, msg_id));
                                            mdn_consumed = 1;
                                        }
                                    }
                                }
                            }
                            mailmime_free(report_parsed);
                        }
                    }
                }
            }

            if mime_parser.is_send_by_messenger || 0 != mdn_consumed {
                let mut param = Params::new();
                param.set(Param::ServerFolder, server_folder.as_ref());
                param.set_int(Param::ServerUid, server_uid as i32);
                if mime_parser.is_send_by_messenger && context.get_config_bool(Config::MvboxMove) {
                    param.set_int(Param::AlsoMove, 1);
                }
                job_add(context, Action::MarkseenMdnOnImap, 0, param, 0);
            }
        }
    }
}

fn save_locations(
    context: &Context,
    mime_parser: &MimeParser,
    chat_id: u32,
    from_id: u32,
    insert_msg_id: MsgId,
    hidden: i32,
) {
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL {
        return ();
    }
    let mut location_id_written = false;
    let mut send_event = false;

    if mime_parser.message_kml.is_some() {
        let locations = &mime_parser.message_kml.as_ref().unwrap().locations;
        let newest_location_id =
            location::save(context, chat_id, from_id, locations, true).unwrap_or_default();
        if 0 != newest_location_id && 0 == hidden {
            if location::set_msg_location_id(context, insert_msg_id, newest_location_id).is_ok() {
                location_id_written = true;
                send_event = true;
            }
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
                    if newest_location_id != 0 && hidden == 0 && !location_id_written {
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

unsafe fn calc_timestamps(
    context: &Context,
    chat_id: u32,
    from_id: u32,
    message_timestamp: i64,
    is_fresh_msg: libc::c_int,
    sort_timestamp: *mut i64,
    sent_timestamp: *mut i64,
    rcvd_timestamp: *mut i64,
) {
    *rcvd_timestamp = time();
    *sent_timestamp = message_timestamp;
    if *sent_timestamp > *rcvd_timestamp {
        *sent_timestamp = *rcvd_timestamp
    }
    *sort_timestamp = message_timestamp;
    if 0 != is_fresh_msg {
        let last_msg_time: Option<i64> = context.sql.query_get_value(
            context,
            "SELECT MAX(timestamp) FROM msgs WHERE chat_id=? and from_id!=? AND timestamp>=?",
            params![chat_id as i32, from_id as i32, *sort_timestamp],
        );
        if let Some(last_msg_time) = last_msg_time {
            if last_msg_time > 0 {
                if *sort_timestamp <= last_msg_time {
                    *sort_timestamp = last_msg_time + 1;
                }
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
/// So when the function returns, the caller has the group id matching the current state of the group.
#[allow(non_snake_case)]
unsafe fn create_or_lookup_group(
    context: &Context,
    mime_parser: &mut MimeParser,
    allow_creation: libc::c_int,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &mut Vec<u32>,
    ret_chat_id: *mut u32,
    ret_chat_id_blocked: &mut Blocked,
) -> Result<()> {
    let group_explicitly_left: bool;
    let mut chat_id = 0;
    let mut chat_id_blocked = Blocked::Not;
    let mut grpid = "".to_string();
    let mut grpname = None;
    let to_ids_cnt = to_ids.len();
    let mut recreate_member_list = 0;
    let mut send_EVENT_CHAT_MODIFIED = 0;
    let mut X_MrRemoveFromGrp = None;
    let mut X_MrAddToGrp = None;
    let mut X_MrGrpNameChanged = 0;
    let mut X_MrGrpImageChanged = "".to_string();
    let mut better_msg: String = From::from("");

    let cleanup = |ret_chat_id: *mut u32,
                   ret_chat_id_blocked: &mut Blocked,
                   chat_id: u32,
                   chat_id_blocked: Blocked| {
        if !ret_chat_id.is_null() {
            *ret_chat_id = chat_id;
        }
        *ret_chat_id_blocked = if 0 != chat_id {
            chat_id_blocked
        } else {
            Blocked::Not
        };
    };

    if mime_parser.is_system_message == SystemMessage::LocationStreamingEnabled {
        better_msg =
            context.stock_system_msg(StockMessage::MsgLocationEnabled, "", "", from_id as u32)
    }
    set_better_msg(mime_parser, &better_msg);

    if let Some(optional_field) = mime_parser.lookup_optional_field("Chat-Group-ID") {
        grpid = optional_field;
    }

    if grpid.is_empty() {
        if let Some(field) = mime_parser.lookup_field_typ("Message-ID", MAILIMF_FIELD_MESSAGE_ID) {
            let fld_message_id = (*field).fld_data.fld_message_id;
            if !fld_message_id.is_null() {
                if let Some(extracted_grpid) =
                    dc_extract_grpid_from_rfc724_mid(&to_string_lossy((*fld_message_id).mid_value))
                {
                    grpid = extracted_grpid.to_string();
                } else {
                    grpid = "".to_string();
                }
            }
        }
        if grpid.is_empty() {
            if let Some(field) =
                mime_parser.lookup_field_typ("In-Reply-To", MAILIMF_FIELD_IN_REPLY_TO)
            {
                let fld_in_reply_to = (*field).fld_data.fld_in_reply_to;
                if !fld_in_reply_to.is_null() {
                    grpid = to_string_lossy(dc_extract_grpid_from_rfc724_mid_list(
                        (*fld_in_reply_to).mid_list,
                    ));
                }
            }
            if grpid.is_empty() {
                if let Some(field) =
                    mime_parser.lookup_field_typ("References", MAILIMF_FIELD_REFERENCES)
                {
                    let fld_references = (*field).fld_data.fld_references;
                    if !fld_references.is_null() {
                        grpid = to_string_lossy(dc_extract_grpid_from_rfc724_mid_list(
                            (*fld_references).mid_list,
                        ));
                    }
                }

                if grpid.is_empty() {
                    create_or_lookup_adhoc_group(
                        context,
                        mime_parser,
                        allow_creation,
                        create_blocked,
                        from_id,
                        to_ids,
                        &mut chat_id,
                        &mut chat_id_blocked,
                    )?;
                    cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
                    return Ok(());
                }
            }
        }
    }

    if let Some(optional_field) = mime_parser.lookup_optional_field("Chat-Group-Name") {
        grpname = Some(dc_decode_header_words(&optional_field));
    }
    if let Some(optional_field) = mime_parser.lookup_optional_field("Chat-Group-Member-Removed") {
        X_MrRemoveFromGrp = Some(optional_field);
        mime_parser.is_system_message = SystemMessage::MemberRemovedFromGroup;
        let left_group = (Contact::lookup_id_by_addr(context, X_MrRemoveFromGrp.as_ref().unwrap())
            == from_id as u32) as libc::c_int;
        better_msg = context.stock_system_msg(
            if 0 != left_group {
                StockMessage::MsgGroupLeft
            } else {
                StockMessage::MsgDelMember
            },
            X_MrRemoveFromGrp.as_ref().unwrap(),
            "",
            from_id as u32,
        )
    } else {
        if let Some(optional_field) = mime_parser.lookup_optional_field("Chat-Group-Member-Added") {
            X_MrAddToGrp = Some(optional_field);
            mime_parser.is_system_message = SystemMessage::MemberAddedToGroup;
            if let Some(optional_field) = mime_parser.lookup_optional_field("Chat-Group-Image") {
                X_MrGrpImageChanged = optional_field;
            }
            better_msg = context.stock_system_msg(
                StockMessage::MsgAddMember,
                X_MrAddToGrp.as_ref().unwrap(),
                "",
                from_id as u32,
            )
        } else {
            if let Some(optional_field) =
                mime_parser.lookup_optional_field("Chat-Group-Name-Changed")
            {
                X_MrGrpNameChanged = 1;
                mime_parser.is_system_message = SystemMessage::GroupNameChanged;
                better_msg = context.stock_system_msg(
                    StockMessage::MsgGrpName,
                    &optional_field,
                    if let Some(ref name) = grpname {
                        name
                    } else {
                        ""
                    },
                    from_id as u32,
                )
            } else {
                if let Some(optional_field) = mime_parser.lookup_optional_field("Chat-Group-Image")
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
    }

    // check if the sender is a member of the existing group -
    // if not, we'll recreate the group list
    if chat_id != 0 && !chat::is_contact_in_chat(context, chat_id, from_id as u32) {
        recreate_member_list = 1;
    }

    // check if the group does not exist but should be created
    group_explicitly_left = chat::is_group_explicitly_left(context, &grpid).unwrap_or_default();

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
        let mut create_verified = VerifiedStatus::Unverified;
        if mime_parser.lookup_field("Chat-Verified").is_some() {
            create_verified = VerifiedStatus::Verified;

            if let Err(err) =
                check_verified_properties(context, mime_parser, from_id as u32, to_ids)
            {
                warn!(context, "verification problem: {}", err);
                let s = format!("{}. See 'Info' for more details", err);
                mime_parser.repl_msg_by_error(&s);
            }
        }
        if 0 == allow_creation {
            cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
            return Ok(());
        }
        chat_id = create_group_record(
            context,
            &grpid,
            grpname.as_ref().unwrap(),
            create_blocked,
            create_verified,
        );
        chat_id_blocked = create_blocked;
        recreate_member_list = 1;
    }

    // again, check chat_id
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL {
        chat_id = 0;
        if group_explicitly_left {
            chat_id = DC_CHAT_ID_TRASH;
        } else {
            create_or_lookup_adhoc_group(
                context,
                mime_parser,
                allow_creation,
                create_blocked,
                from_id,
                to_ids,
                &mut chat_id,
                &mut chat_id_blocked,
            )?;
        }
        cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
        return Ok(());
    }

    // execute group commands
    if X_MrAddToGrp.is_some() || X_MrRemoveFromGrp.is_some() {
        recreate_member_list = 1;
    } else if 0 != X_MrGrpNameChanged {
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
                send_EVENT_CHAT_MODIFIED = 1;
            }
        }
    }

    // add members to group/check members
    // for recreation: we should add a timestamp
    if 0 != recreate_member_list {
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
        if from_id > DC_CHAT_ID_LAST_SPECIAL {
            if !Contact::addr_equals_contact(context, &self_addr, from_id as u32)
                && (skip.is_none()
                    || !Contact::addr_equals_contact(context, skip.unwrap(), from_id as u32))
            {
                chat::add_to_chat_contacts_table(context, chat_id, from_id as u32);
            }
        }
        for &to_id in to_ids.iter() {
            if !Contact::addr_equals_contact(context, &self_addr, to_id)
                && (skip.is_none() || !Contact::addr_equals_contact(context, skip.unwrap(), to_id))
            {
                chat::add_to_chat_contacts_table(context, chat_id, to_id);
            }
        }
        send_EVENT_CHAT_MODIFIED = 1;
        chat::reset_gossiped_timestamp(context, chat_id);
    }

    if 0 != send_EVENT_CHAT_MODIFIED {
        context.call_cb(Event::ChatModified(chat_id));
    }

    // check the number of receivers -
    // the only critical situation is if the user hits "Reply" instead of "Reply all" in a non-messenger-client */
    if to_ids_cnt == 1 && !mime_parser.is_send_by_messenger {
        let is_contact_cnt = chat::get_chat_contact_cnt(context, chat_id);
        if is_contact_cnt > 3 {
            // to_ids_cnt==1 may be "From: A, To: B, SELF" as SELF is not counted in to_ids_cnt.
            // So everything up to 3 is no error.
            chat_id = 0;
            create_or_lookup_adhoc_group(
                context,
                mime_parser,
                allow_creation,
                create_blocked,
                from_id,
                to_ids,
                &mut chat_id,
                &mut chat_id_blocked,
            )?;
        }
    }

    cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
    return Ok(());
}

/// Handle groups for received messages
unsafe fn create_or_lookup_adhoc_group(
    context: &Context,
    mime_parser: &MimeParser,
    allow_creation: libc::c_int,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &mut Vec<u32>,
    ret_chat_id: *mut u32,
    ret_chat_id_blocked: &mut Blocked,
) -> Result<()> {
    // if we're here, no grpid was found, check there is an existing ad-hoc
    // group matching the to-list or if we can create one
    let mut chat_id = 0;
    let mut chat_id_blocked = Blocked::Not;

    let cleanup = |ret_chat_id: *mut u32,
                   ret_chat_id_blocked: &mut Blocked,
                   chat_id: u32,
                   chat_id_blocked: Blocked| {
        if !ret_chat_id.is_null() {
            *ret_chat_id = chat_id;
        }
        *ret_chat_id_blocked = chat_id_blocked;
    };

    // build member list from the given ids
    if to_ids.is_empty() || mime_parser.is_mailinglist_message() {
        // too few contacts or a mailinglist
        cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
        return Ok(());
    }

    let mut member_ids = to_ids.clone();
    if !member_ids.contains(&from_id) {
        member_ids.push(from_id);
    }
    if !member_ids.contains(&DC_CONTACT_ID_SELF) {
        member_ids.push(DC_CONTACT_ID_SELF);
    }
    if member_ids.len() < 3 {
        // too few contacts given
        cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
        return Ok(());
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
            chat_id = id as u32;
            chat_id_blocked = id_blocked;
            /* success, chat found */
            cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
            return Ok(());
        }
    }

    if 0 == allow_creation {
        cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
        return Ok(());
    }
    // we do not check if the message is a reply to another group, this may result in
    // chats with unclear member list. instead we create a new group in the following lines ...

    // create a new ad-hoc group
    // - there is no need to check if this group exists; otherwise we would have caught it above
    let grpid = create_adhoc_grp_id(context, &member_ids);
    if grpid.is_empty() {
        cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
        return Ok(());
    }

    // use subject as initial chat name
    let grpname = if let Some(subject) = mime_parser.subject.as_ref().filter(|s| !s.is_empty()) {
        subject.to_string()
    } else {
        context.stock_string_repl_int(StockMessage::Member, member_ids.len() as libc::c_int)
    };

    // create group record
    chat_id = create_group_record(
        context,
        &grpid,
        grpname,
        create_blocked,
        VerifiedStatus::Unverified,
    );
    chat_id_blocked = create_blocked;
    for &member_id in &member_ids {
        chat::add_to_chat_contacts_table(context, chat_id, member_id);
    }

    context.call_cb(Event::ChatModified(chat_id));

    cleanup(ret_chat_id, ret_chat_id_blocked, chat_id, chat_id_blocked);
    return Ok(());
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
        "INSERT INTO chats (type, name, grpid, blocked) VALUES(?, ?, ?, ?);",
        params![
            if VerifiedStatus::Unverified != create_verified {
                Chattype::VerifiedGroup
            } else {
                Chattype::Group
            },
            grpname.as_ref(),
            grpid.as_ref(),
            create_blocked,
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
    unsorted_contact_ids: &Vec<u32>,
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

    ensure!(mimeparser.encrypted, "This message is not encrypted.");

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
                        DC_PS_GOSSIP_KEY,
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
    if msg.len() > 0 && !mime_parser.parts.is_empty() {
        let part = &mut mime_parser.parts[0];
        if part.typ == Viewtype::Text {
            part.msg = Some(msg.to_string());
        }
    };
}

unsafe fn dc_is_reply_to_known_message(context: &Context, mime_parser: &MimeParser) -> libc::c_int {
    /* check if the message is a reply to a known message; the replies are identified by the Message-ID from
    `In-Reply-To`/`References:` (to support non-Delta-Clients) */

    if let Some(field) = mime_parser.lookup_field("In-Reply-To") {
        if (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
            let fld_in_reply_to = (*field).fld_data.fld_in_reply_to;
            if !fld_in_reply_to.is_null() {
                if is_known_rfc724_mid_in_list(
                    context,
                    (*(*field).fld_data.fld_in_reply_to).mid_list,
                ) {
                    return 1;
                }
            }
        }
    }

    if let Some(field) = mime_parser.lookup_field("References") {
        if (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int {
            let fld_references = (*field).fld_data.fld_references;
            if !fld_references.is_null()
                && is_known_rfc724_mid_in_list(
                    context,
                    (*(*field).fld_data.fld_references).mid_list,
                )
            {
                return 1;
            }
        }
    }

    0
}

unsafe fn is_known_rfc724_mid_in_list(context: &Context, mid_list: *const clist) -> bool {
    if mid_list.is_null() {
        return false;
    }

    for data in &*mid_list {
        if is_known_rfc724_mid(context, data.cast()) != 0 {
            return true;
        }
    }

    false
}

/// Check if a message is a reply to a known message (messenger or non-messenger).
fn is_known_rfc724_mid(context: &Context, rfc724_mid: *const libc::c_char) -> libc::c_int {
    if rfc724_mid.is_null() {
        return 0;
    }
    context
        .sql
        .exists(
            "SELECT m.id FROM msgs m  \
             LEFT JOIN chats c ON m.chat_id=c.id  \
             WHERE m.rfc724_mid=?  \
             AND m.chat_id>9 AND c.blocked=0;",
            params![to_string_lossy(rfc724_mid)],
        )
        .unwrap_or_default() as libc::c_int
}

unsafe fn dc_is_reply_to_messenger_message(
    context: &Context,
    mime_parser: &MimeParser,
) -> libc::c_int {
    /* function checks, if the message defined by mime_parser references a message send by us from Delta Chat.
    This is similar to is_reply_to_known_message() but
    - checks also if any of the referenced IDs are send by a messenger
    - it is okay, if the referenced messages are moved to trash here
    - no check for the Chat-* headers (function is only called if it is no messenger message itself) */

    if let Some(field) = mime_parser.lookup_field("In-Reply-To") {
        if (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
            let fld_in_reply_to = (*field).fld_data.fld_in_reply_to;
            if !fld_in_reply_to.is_null() {
                if 0 != is_msgrmsg_rfc724_mid_in_list(
                    context,
                    (*(*field).fld_data.fld_in_reply_to).mid_list,
                ) {
                    return 1;
                }
            }
        }
    }

    if let Some(field) = mime_parser.lookup_field("References") {
        if (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int {
            let fld_references: *mut mailimf_references = (*field).fld_data.fld_references;
            if !fld_references.is_null() {
                if 0 != is_msgrmsg_rfc724_mid_in_list(
                    context,
                    (*(*field).fld_data.fld_references).mid_list,
                ) {
                    return 1;
                }
            }
        }
    }

    0
}

unsafe fn is_msgrmsg_rfc724_mid_in_list(context: &Context, mid_list: *const clist) -> libc::c_int {
    if !mid_list.is_null() {
        let mut cur: *mut clistiter = (*mid_list).first;
        while !cur.is_null() {
            if 0 != is_msgrmsg_rfc724_mid(
                context,
                &to_string_lossy((*cur).data as *const libc::c_char),
            ) {
                return 1;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                ptr::null_mut()
            }
        }
    }
    0
}

/// Check if a message is a reply to any messenger message.
fn is_msgrmsg_rfc724_mid(context: &Context, rfc724_mid: &str) -> libc::c_int {
    if rfc724_mid.is_empty() {
        return 0;
    }
    context
        .sql
        .exists(
            "SELECT id FROM msgs  WHERE rfc724_mid=?  AND msgrmsg!=0  AND chat_id>9;",
            params![rfc724_mid],
        )
        .unwrap_or_default() as libc::c_int
}

unsafe fn dc_add_or_lookup_contacts_by_address_list(
    context: &Context,
    adr_list: *const mailimf_address_list,
    origin: Origin,
    ids: &mut Vec<u32>,
    check_self: *mut libc::c_int,
) {
    if adr_list.is_null() {
        return;
    }
    let mut cur: *mut clistiter = (*(*adr_list).ad_list).first;
    while !cur.is_null() {
        let adr: *mut mailimf_address = (if !cur.is_null() {
            (*cur).data
        } else {
            ptr::null_mut()
        }) as *mut mailimf_address;
        if !adr.is_null() {
            if (*adr).ad_type == MAILIMF_ADDRESS_MAILBOX as libc::c_int {
                let mb: *mut mailimf_mailbox = (*adr).ad_data.ad_mailbox;
                if !mb.is_null() {
                    add_or_lookup_contact_by_addr(
                        context,
                        (*mb).mb_display_name,
                        (*mb).mb_addr_spec,
                        origin,
                        ids,
                        check_self,
                    );
                }
            } else if (*adr).ad_type == MAILIMF_ADDRESS_GROUP as libc::c_int {
                let group: *mut mailimf_group = (*adr).ad_data.ad_group;
                if !group.is_null() && !(*group).grp_mb_list.is_null() {
                    dc_add_or_lookup_contacts_by_mailbox_list(
                        context,
                        (*group).grp_mb_list,
                        origin,
                        ids,
                        check_self,
                    );
                }
            }
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            ptr::null_mut()
        }
    }
}

unsafe fn dc_add_or_lookup_contacts_by_mailbox_list(
    context: &Context,
    mb_list: *const mailimf_mailbox_list,
    origin: Origin,
    ids: &mut Vec<u32>,
    check_self: *mut libc::c_int,
) {
    if mb_list.is_null() {
        return;
    }
    let mut cur: *mut clistiter = (*(*mb_list).mb_list).first;
    while !cur.is_null() {
        let mb: *mut mailimf_mailbox = (if !cur.is_null() {
            (*cur).data
        } else {
            ptr::null_mut()
        }) as *mut mailimf_mailbox;
        if !mb.is_null() {
            add_or_lookup_contact_by_addr(
                context,
                (*mb).mb_display_name,
                (*mb).mb_addr_spec,
                origin,
                ids,
                check_self,
            );
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            ptr::null_mut()
        }
    }
}

/// Add contacts to database on receiving messages.
unsafe fn add_or_lookup_contact_by_addr(
    context: &Context,
    display_name_enc: *const libc::c_char,
    addr_spec: *const libc::c_char,
    origin: Origin,
    ids: &mut Vec<u32>,
    mut check_self: *mut libc::c_int,
) {
    /* is addr_spec equal to SELF? */
    let mut dummy: libc::c_int = 0;
    if check_self.is_null() {
        check_self = &mut dummy
    }
    if addr_spec.is_null() {
        return;
    }
    *check_self = 0;
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_default();

    if addr_cmp(self_addr, to_string_lossy(addr_spec)) {
        *check_self = 1;
    }

    if 0 != *check_self {
        return;
    }
    /* add addr_spec if missing, update otherwise */
    let mut display_name_dec = "".to_string();
    if !display_name_enc.is_null() {
        let tmp = dc_decode_header_words(&to_string_lossy(display_name_enc));
        display_name_dec = normalize_name(&tmp);
    }
    /*can be NULL*/
    let row_id = Contact::add_or_lookup(
        context,
        display_name_dec,
        to_string_lossy(addr_spec),
        origin,
    )
    .map(|(id, _)| id)
    .unwrap_or_default();
    if 0 != row_id && !ids.contains(&row_id) {
        ids.push(row_id);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_hash() {
        let data = "hello world";

        let res = hex_hash(data);
        assert_eq!(res, "b94d27b9934d3e08");
    }
}

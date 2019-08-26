use std::ptr;

use itertools::join;
use mmime::mailimf::*;
use mmime::mailimf_types::*;
use mmime::mailmime::*;
use mmime::mailmime_content::*;
use mmime::mailmime_types::*;
use mmime::mmapstring::*;
use mmime::other::*;
use sha2::{Digest, Sha256};

use crate::chat::{self, Chat};
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_mimeparser::*;
use crate::dc_move::*;
use crate::dc_securejoin::*;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::error::Result;
use crate::job::*;
use crate::location::*;
use crate::message::*;
use crate::param::*;
use crate::peerstate::*;
use crate::sql;
use crate::stock::StockMessage;
use crate::types::*;
use crate::x::*;

/// Receive a message and add it to the database.
pub unsafe fn dc_receive_imf(
    context: &Context,
    imf_raw_not_terminated: *const libc::c_char,
    imf_raw_bytes: size_t,
    server_folder: impl AsRef<str>,
    server_uid: u32,
    flags: u32,
) {
    info!(
        context,
        0,
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

    let mut mime_parser = dc_mimeparser_new(context);
    dc_mimeparser_parse(&mut mime_parser, imf_raw_not_terminated, imf_raw_bytes);

    if mime_parser.header.is_empty() {
        // Error - even adding an empty record won't help as we do not know the message ID
        info!(context, 0, "No header.");
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

    let mut add_delete_job: libc::c_int = 0;
    let mut insert_msg_id = 0;

    // Message-ID from the header
    let rfc724_mid = std::ptr::null_mut();
    let mut sent_timestamp = 0;
    let mut created_db_entries = Vec::new();
    let mut create_event_to_send = Some(Event::MSGS_CHANGED);
    let mut rr_event_to_send = Vec::new();

    let mut to_ids = Vec::with_capacity(16);

    // helper method to handle early exit and memory cleanup
    let cleanup = |context: &Context,
                   rfc724_mid: *mut libc::c_char,
                   create_event_to_send: &Option<Event>,
                   created_db_entries: &Vec<(usize, usize)>,
                   rr_event_to_send: &Vec<(u32, u32)>| {
        free(rfc724_mid.cast());

        if let Some(create_event_to_send) = create_event_to_send {
            for (msg_id, insert_id) in created_db_entries {
                context.call_cb(*create_event_to_send, *msg_id, *insert_id);
            }
        }
        for (chat_id, msg_id) in rr_event_to_send {
            context.call_cb(Event::MSG_READ, *chat_id as uintptr_t, *msg_id as uintptr_t);
        }
    };

    if let Some(field) = lookup_field(&mime_parser, "Date", MAILIMF_FIELD_ORIG_DATE) {
        let orig_date = (*field).fld_data.fld_orig_date;
        if !orig_date.is_null() {
            // is not yet checked against bad times! we do this later if we have the database information.
            sent_timestamp = dc_timestamp_from_date((*orig_date).dt_date_time)
        }
    }

    // get From: and check if it is known (for known From:'s we add the other To:/Cc: in the 3rd pass)
    // or if From: is equal to SELF (in this case, it is any outgoing messages,
    // we do not check Return-Path any more as this is unreliable, see issue #150
    if let Some(field) = lookup_field(&mime_parser, "From", MAILIMF_FIELD_FROM) {
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
                if 0 != dc_mimeparser_sender_equals_recipient(&mime_parser) {
                    from_id = DC_CONTACT_ID_SELF as u32;
                }
            } else if from_list.len() >= 1 {
                // if there is no from given, from_id stays 0 which is just fine. These messages
                // are very rare, however, we have to add them to the database (they go to the
                // "deaddrop" chat) to avoid a re-download from the server. See also [**]
                from_id = from_list[0];
                incoming_origin = Contact::get_origin_by_id(context, from_id, &mut from_id_blocked)
            }
        }
    }

    // Make sure, to_ids starts with the first To:-address (Cc: is added in the loop below pass)
    if let Some(field) = lookup_field(&mime_parser, "To", MAILIMF_FIELD_TO) {
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
    if dc_mimeparser_get_last_nonmeta(&mut mime_parser).is_some() {
        if let Err(err) = add_parts(
            context,
            &mut mime_parser,
            imf_raw_not_terminated,
            imf_raw_bytes,
            incoming,
            &mut incoming_origin,
            server_folder.as_ref(),
            server_uid,
            &mut to_ids,
            rfc724_mid,
            &mut sent_timestamp,
            &mut from_id,
            from_id_blocked,
            &mut hidden,
            &mut chat_id,
            &mut to_id,
            flags,
            &mut add_delete_job,
            to_self,
            &mut insert_msg_id,
            &mut created_db_entries,
            &mut create_event_to_send,
        ) {
            info!(context, 0, "{}", err);

            cleanup(
                context,
                rfc724_mid,
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
            server_folder,
            server_uid,
        );
    }

    if !mime_parser.message_kml.is_none() && chat_id > DC_CHAT_ID_LAST_SPECIAL as u32 {
        save_locations(
            context,
            &mime_parser,
            chat_id,
            from_id,
            insert_msg_id,
            hidden,
        );
    }

    if 0 != add_delete_job && !created_db_entries.is_empty() {
        job_add(
            context,
            Action::DeleteMsgOnImap,
            created_db_entries[0].1 as i32,
            Params::new(),
            0,
        );
    }

    info!(
        context,
        0,
        "received message {} has Message-Id: {}",
        server_uid,
        to_string(rfc724_mid)
    );

    cleanup(
        context,
        rfc724_mid,
        &create_event_to_send,
        &created_db_entries,
        &rr_event_to_send,
    );
}

unsafe fn add_parts(
    context: &Context,
    mut mime_parser: &mut dc_mimeparser_t,
    imf_raw_not_terminated: *const libc::c_char,
    imf_raw_bytes: size_t,
    incoming: i32,
    incoming_origin: &mut Origin,
    server_folder: impl AsRef<str>,
    server_uid: u32,
    to_ids: &mut Vec<u32>,
    mut rfc724_mid: *mut libc::c_char,
    sent_timestamp: &mut i64,
    from_id: &mut u32,
    from_id_blocked: i32,
    hidden: &mut libc::c_int,
    chat_id: &mut u32,
    to_id: &mut u32,
    flags: u32,
    add_delete_job: &mut libc::c_int,
    to_self: i32,
    insert_msg_id: &mut u32,
    created_db_entries: &mut Vec<(usize, usize)>,
    create_event_to_send: &mut Option<Event>,
) -> Result<()> {
    let mut state: MessageState;
    let mut msgrmsg: libc::c_int;
    let mut chat_id_blocked = Blocked::Not;
    let mut sort_timestamp = 0;
    let mut rcvd_timestamp = 0;
    let mut mime_in_reply_to = std::ptr::null_mut();
    let mut mime_references = std::ptr::null_mut();
    let mut txt_raw = std::ptr::null_mut();

    let cleanup = |mime_in_reply_to: *mut libc::c_char,
                   mime_references: *mut libc::c_char,
                   txt_raw: *mut libc::c_char| {
        free(mime_in_reply_to.cast());
        free(mime_references.cast());
        free(txt_raw.cast());
    };

    // collect the rest information, CC: is added to the to-list, BCC: is ignored
    // (we should not add BCC to groups as this would split groups. We could add them as "known contacts",
    // however, the benefit is very small and this may leak data that is expected to be hidden)
    if let Some(field) = lookup_field(mime_parser, "Cc", MAILIMF_FIELD_CC) {
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

    // get Message-ID; if the header is lacking one, generate one based on fields that do never
    // change. (missing Message-IDs may come if the mail was set from this account with another
    // client that relies in the SMTP server to generate one.
    // true eg. for the Webmailer used in all-inkl-KAS)
    if let Some(field) = lookup_field(mime_parser, "Message-ID", MAILIMF_FIELD_MESSAGE_ID) {
        let fld_message_id = (*field).fld_data.fld_message_id;
        if !fld_message_id.is_null() {
            rfc724_mid = dc_strdup((*fld_message_id).mid_value)
        }
    }

    if rfc724_mid.is_null() {
        rfc724_mid = dc_create_incoming_rfc724_mid(*sent_timestamp, *from_id, to_ids);
        if rfc724_mid.is_null() {
            cleanup(mime_in_reply_to, mime_references, txt_raw);
            bail!("Cannot create Message-ID");
        }
    }

    // check, if the mail is already in our database - if so, just update the folder/uid
    // (if the mail was moved around) and finish. (we may get a mail twice eg. if it is
    // moved between folders. make sure, this check is done eg. before securejoin-processing) */
    let mut old_server_folder = std::ptr::null_mut();
    let mut old_server_uid = 0;

    if 0 != dc_rfc724_mid_exists(
        context,
        rfc724_mid,
        &mut old_server_folder,
        &mut old_server_uid,
    ) {
        if as_str(old_server_folder) != server_folder.as_ref() || old_server_uid != server_uid {
            dc_update_server_uid(context, rfc724_mid, server_folder.as_ref(), server_uid);
        }

        free(old_server_folder.cast());
        cleanup(mime_in_reply_to, mime_references, txt_raw);
        bail!("Message already in DB");
    }

    // 1 or 0 for yes/no
    msgrmsg = mime_parser.is_send_by_messenger;
    if msgrmsg == 0 && 0 != dc_is_reply_to_messenger_message(context, mime_parser) {
        // 2=no, but is reply to messenger message
        msgrmsg = 2;
    }
    // incoming non-chat messages may be discarded;
    // maybe this can be optimized later, by checking the state before the message body is downloaded
    let mut allow_creation = 1;
    if mime_parser.is_system_message != DC_CMD_AUTOCRYPT_SETUP_MESSAGE && msgrmsg == 0 {
        let show_emails = context
            .sql
            .get_config_int(context, "show_emails")
            .unwrap_or_default();
        if show_emails == 0 {
            *chat_id = 3;
            allow_creation = 0
        } else if show_emails == 1 {
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
        *to_id = 1;
        // handshake messages must be processed _before_ chats are created
        // (eg. contacs may be marked as verified)
        if !dc_mimeparser_lookup_field(mime_parser, "Secure-Join").is_null() {
            // avoid discarding by show_emails setting
            msgrmsg = 1;
            *chat_id = 0;
            allow_creation = 1;
            let handshake = dc_handle_securejoin_handshake(context, mime_parser, *from_id);
            if 0 != handshake & DC_HANDSHAKE_STOP_NORMAL_PROCESSING {
                *hidden = 1;
                *add_delete_job = handshake & DC_HANDSHAKE_ADD_DELETE_JOB;
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
            );
            if 0 != *chat_id && Blocked::Not != chat_id_blocked && create_blocked == Blocked::Not {
                chat::unblock(context, *chat_id);
                chat_id_blocked = Blocked::Not;
            }
        }

        if *chat_id == 0 {
            // check if the message belongs to a mailing list
            if 0 != dc_mimeparser_is_mailinglist_message(mime_parser) {
                *chat_id = 3;
                info!(
                    context,
                    0, "Message belongs to a mailing list and is ignored.",
                );
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
                        0, "Message is a reply to a known message, mark sender as known.",
                    );
                    if !incoming_origin.is_verified() {
                        *incoming_origin = Origin::IncomingReplyTo;
                    }
                }
            }
        }
        if *chat_id == 0 {
            // maybe from_id is null or sth. else is suspicious, move message to trash
            *chat_id = DC_CHAT_ID_TRASH as u32;
        }

        // if the chat_id is blocked,
        // for unknown senders and non-delta messages set the state to NOTICED
        // to not result in a contact request (this would require the state FRESH)
        if Blocked::Not != chat_id_blocked && state == MessageState::InFresh {
            if !incoming_origin.is_verified() && msgrmsg == 0 {
                state = MessageState::InNoticed;
            }
        }
    } else {
        // Outgoing

        // the mail is on the IMAP server, probably it is also delivered.
        // We cannot recreate other states (read, error).
        state = MessageState::OutDelivered;
        *from_id = DC_CONTACT_ID_SELF as u32;
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
                );
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
                let (id, bl) = chat::create_or_lookup_by_contact_id(context, 1, Blocked::Not)
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
            *chat_id = DC_CHAT_ID_TRASH as u32;
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
    chat::unarchive(context, *chat_id).unwrap();

    // if the mime-headers should be saved, find out its size
    // (the mime-header ends with an empty line)
    let save_mime_headers = context
        .sql
        .get_config_int(context, "save_mime_headers")
        .unwrap_or_default();
    if let Some(field) = lookup_field(mime_parser, "In-Reply-To", MAILIMF_FIELD_IN_REPLY_TO) {
        let fld_in_reply_to = (*field).fld_data.fld_in_reply_to;
        if !fld_in_reply_to.is_null() {
            mime_in_reply_to = dc_str_from_clist(
                (*(*field).fld_data.fld_in_reply_to).mid_list,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
    }

    if let Some(field) = lookup_field(mime_parser, "References", MAILIMF_FIELD_REFERENCES) {
        let fld_references = (*field).fld_data.fld_references;
        if !fld_references.is_null() {
            mime_references = dc_str_from_clist(
                (*(*field).fld_data.fld_references).mid_list,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
    }

    // fine, so far.  now, split the message into simple parts usable as "short messages"
    // and add them to the database (mails sent by other messenger clients should result
    // into only one message; mails sent by other clients may result in several messages
    // (eg. one per attachment))
    let icnt = mime_parser.parts.len();
    let is_ok = context
        .sql
        .prepare(
            "INSERT INTO msgs \
             (rfc724_mid, server_folder, server_uid, chat_id, from_id, to_id, timestamp, \
             timestamp_sent, timestamp_rcvd, type, state, msgrmsg,  txt, txt_raw, param, \
             bytes, hidden, mime_headers,  mime_in_reply_to, mime_references) \
             VALUES (?,?,?,?,?,?, ?,?,?,?,?,?, ?,?,?,?,?,?, ?,?);",
            |mut stmt, conn| {
                for i in 0..icnt {
                    let part = &mut mime_parser.parts[i];
                    if part.is_meta != 0 {
                        continue;
                    }

                    if !mime_parser.location_kml.is_none()
                        && icnt == 1
                        && !part.msg.is_null()
                        && (strcmp(
                            part.msg,
                            b"-location-\x00" as *const u8 as *const libc::c_char,
                        ) == 0
                            || *part.msg.offset(0) as libc::c_int == 0)
                    {
                        *hidden = 1;
                        if state == MessageState::InFresh {
                            state = MessageState::InNoticed;
                        }
                    }
                    if part.type_0 == Viewtype::Text as i32 {
                        txt_raw = dc_mprintf(
                            b"%s\n\n%s\x00" as *const u8 as *const libc::c_char,
                            if !mime_parser.subject.is_null() {
                                mime_parser.subject
                            } else {
                                b"\x00" as *const u8 as *const libc::c_char
                            },
                            part.msg_raw,
                        )
                    }
                    if 0 != mime_parser.is_system_message {
                        part.param
                            .set_int(Param::Cmd, mime_parser.is_system_message);
                    }

                    stmt.execute(params![
                        as_str(rfc724_mid),
                        server_folder.as_ref(),
                        server_uid as libc::c_int,
                        *chat_id as libc::c_int,
                        *from_id as libc::c_int,
                        *to_id as libc::c_int,
                        sort_timestamp,
                        *sent_timestamp,
                        rcvd_timestamp,
                        part.type_0,
                        state,
                        msgrmsg,
                        if !part.msg.is_null() {
                            as_str(part.msg)
                        } else {
                            ""
                        },
                        // txt_raw might contain invalid utf8
                        if !txt_raw.is_null() {
                            to_string_lossy(txt_raw)
                        } else {
                            String::new()
                        },
                        part.param.to_string(),
                        part.bytes,
                        *hidden,
                        if 0 != save_mime_headers {
                            let body_string = std::str::from_utf8(std::slice::from_raw_parts(
                                imf_raw_not_terminated as *const u8,
                                imf_raw_bytes,
                            ))
                            .unwrap();

                            Some(body_string)
                        } else {
                            None
                        },
                        to_string(mime_in_reply_to),
                        to_string(mime_references),
                    ])?;

                    free(txt_raw as *mut libc::c_void);
                    txt_raw = 0 as *mut libc::c_char;
                    *insert_msg_id = sql::get_rowid_with_conn(
                        context,
                        conn,
                        "msgs",
                        "rfc724_mid",
                        as_str(rfc724_mid),
                    );
                    created_db_entries.push((*chat_id as usize, *insert_msg_id as usize));
                }
                Ok(())
            },
        )
        .is_ok();

    if !is_ok {
        // i/o error - there is nothing more we can do - in other cases, we try to write at least an empty record
        cleanup(mime_in_reply_to, mime_references, txt_raw);
        bail!("Cannot write DB.");
    }

    info!(
        context,
        0, "Message has {} parts and is assigned to chat #{}.", icnt, *chat_id,
    );

    // check event to send
    if *chat_id == DC_CHAT_ID_TRASH as u32 {
        *create_event_to_send = None;
    } else if 0 != incoming && state == MessageState::InFresh {
        if 0 != from_id_blocked {
            *create_event_to_send = None;
        } else if Blocked::Not != chat_id_blocked {
            *create_event_to_send = Some(Event::MSGS_CHANGED);
        } else {
            *create_event_to_send = Some(Event::INCOMING_MSG);
        }
    }

    dc_do_heuristics_moves(context, server_folder.as_ref(), *insert_msg_id);
    cleanup(mime_in_reply_to, mime_references, txt_raw);

    Ok(())
}

/// Lookup a mime field given its name and type.
unsafe fn lookup_field(
    parser: &dc_mimeparser_t,
    name: &str,
    typ: u32,
) -> Option<*const mailimf_field> {
    let field = dc_mimeparser_lookup_field(parser, name);
    if !field.is_null() && (*field).fld_type == typ as libc::c_int {
        Some(field)
    } else {
        None
    }
}

// Handle reports (mainly MDNs)
unsafe fn handle_reports(
    context: &Context,
    mime_parser: &dc_mimeparser_t,
    from_id: u32,
    sent_timestamp: i64,
    rr_event_to_send: &mut Vec<(u32, u32)>,
    server_folder: impl AsRef<str>,
    server_uid: u32,
) {
    let mdns_enabled = context
        .sql
        .get_config_int(context, "mdns_enabled")
        .unwrap_or_else(|| DC_MDNS_DEFAULT_ENABLED);

    for report_root in &mime_parser.reports {
        let report_root = *report_root;
        let mut mdn_consumed = 0;
        let report_type = mailmime_find_ct_parameter(
            report_root,
            b"report-type\x00" as *const u8 as *const libc::c_char,
        );

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
            if 0 != mdns_enabled {
                let report_data = (if !if !(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                    .first
                    .is_null()
                {
                    (*(*(*report_root).mm_data.mm_multipart.mm_mp_list).first).next
                } else {
                    0 as *mut clistcell
                }
                .is_null()
                {
                    (*if !(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                        .first
                        .is_null()
                    {
                        (*(*(*report_root).mm_data.mm_multipart.mm_mp_list).first).next
                    } else {
                        0 as *mut clistcell
                    })
                    .data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime;

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
                    let mut report_body = std::ptr::null();
                    let mut report_body_bytes = 0;
                    let mut to_mmap_string_unref = std::ptr::null_mut();

                    if 0 != mailmime_transfer_decode(
                        report_data,
                        &mut report_body,
                        &mut report_body_bytes,
                        &mut to_mmap_string_unref,
                    ) {
                        let mut report_parsed = std::ptr::null_mut();
                        let mut dummy = 0;

                        if mailmime_parse(
                            report_body,
                            report_body_bytes,
                            &mut dummy,
                            &mut report_parsed,
                        ) == MAIL_NO_ERROR as libc::c_int
                            && !report_parsed.is_null()
                        {
                            let report_fields = mailmime_find_mailimf_fields(report_parsed);
                            if !report_fields.is_null() {
                                let of_disposition = mailimf_find_optional_field(
                                    report_fields,
                                    b"Disposition\x00" as *const u8 as *const libc::c_char,
                                );
                                let of_org_msgid = mailimf_find_optional_field(
                                    report_fields,
                                    b"Original-Message-ID\x00" as *const u8 as *const libc::c_char,
                                );
                                if !of_disposition.is_null()
                                    && !(*of_disposition).fld_value.is_null()
                                    && !of_org_msgid.is_null()
                                    && !(*of_org_msgid).fld_value.is_null()
                                {
                                    let mut rfc724_mid_0 = std::ptr::null_mut();
                                    dummy = 0;

                                    if mailimf_msg_id_parse(
                                        (*of_org_msgid).fld_value,
                                        strlen((*of_org_msgid).fld_value),
                                        &mut dummy,
                                        &mut rfc724_mid_0,
                                    ) == MAIL_NO_ERROR as libc::c_int
                                        && !rfc724_mid_0.is_null()
                                    {
                                        let mut chat_id_0 = 0;
                                        let mut msg_id = 0;

                                        if 0 != dc_mdn_from_ext(
                                            context,
                                            from_id,
                                            rfc724_mid_0,
                                            sent_timestamp,
                                            &mut chat_id_0,
                                            &mut msg_id,
                                        ) {
                                            rr_event_to_send.push((chat_id_0, msg_id));
                                        }
                                        mdn_consumed = (msg_id != 0) as libc::c_int;
                                        free(rfc724_mid_0.cast());
                                    }
                                }
                            }
                            mailmime_free(report_parsed);
                        }
                        if !to_mmap_string_unref.is_null() {
                            mmap_string_unref(to_mmap_string_unref);
                        }
                    }
                }
            }

            if 0 != mime_parser.is_send_by_messenger || 0 != mdn_consumed {
                let mut param = Params::new();
                param.set(Param::ServerFolder, server_folder.as_ref());
                param.set_int(Param::ServerUid, server_uid as i32);
                if 0 != mime_parser.is_send_by_messenger
                    && 0 != context
                        .sql
                        .get_config_int(context, "mvbox_move")
                        .unwrap_or_else(|| 1)
                {
                    param.set_int(Param::AlsoMove, 1);
                }
                job_add(context, Action::MarkseenMdnOnImap, 0, param, 0);
            }
        }
    }
}

unsafe fn save_locations(
    context: &Context,
    mime_parser: &dc_mimeparser_t,
    chat_id: u32,
    from_id: u32,
    insert_msg_id: u32,
    hidden: i32,
) {
    let mut location_id_written = false;
    let mut send_event = false;

    if !mime_parser.message_kml.is_none() && chat_id > DC_CHAT_ID_LAST_SPECIAL as libc::c_uint {
        let newest_location_id: uint32_t = dc_save_locations(
            context,
            chat_id,
            from_id,
            &mime_parser.message_kml.as_ref().unwrap().locations,
            1,
        );
        if 0 != newest_location_id && 0 == hidden {
            dc_set_msg_location_id(context, insert_msg_id, newest_location_id);
            location_id_written = true;
            send_event = true;
        }
    }

    if !mime_parser.location_kml.is_none() && chat_id > DC_CHAT_ID_LAST_SPECIAL as libc::c_uint {
        if let Some(ref addr) = mime_parser.location_kml.as_ref().unwrap().addr {
            if let Ok(contact) = Contact::get_by_id(context, from_id) {
                if !contact.get_addr().is_empty()
                    && contact.get_addr().to_lowercase() == addr.to_lowercase()
                {
                    let newest_location_id = dc_save_locations(
                        context,
                        chat_id,
                        from_id,
                        &mime_parser.location_kml.as_ref().unwrap().locations,
                        0,
                    );
                    if newest_location_id != 0 && hidden == 0 && !location_id_written {
                        dc_set_msg_location_id(context, insert_msg_id, newest_location_id);
                    }
                    send_event = true;
                }
            }
        }
    }
    if send_event {
        context.call_cb(
            Event::LOCATION_CHANGED,
            from_id as uintptr_t,
            0 as uintptr_t,
        );
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
        let last_msg_time: Option<i64> = context.sql.query_row_col(
            context,
            "SELECT MAX(timestamp) FROM msgs WHERE chat_id=? and from_id!=? AND timestamp>=?",
            params![chat_id as i32, from_id as i32, *sort_timestamp],
            0,
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
    mime_parser: &mut dc_mimeparser_t,
    allow_creation: libc::c_int,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &mut Vec<u32>,
    ret_chat_id: *mut uint32_t,
    ret_chat_id_blocked: &mut Blocked,
) {
    let group_explicitly_left: bool;
    let mut chat_id = 0;
    let mut chat_id_blocked = Blocked::Not;
    let mut chat_id_verified = 0;
    let mut grpid = "".to_string();
    let mut grpname = std::ptr::null_mut();
    let to_ids_cnt = to_ids.len();
    let mut recreate_member_list = 0;
    let mut send_EVENT_CHAT_MODIFIED = 0;
    // pointer somewhere into mime_parser, must not be freed
    let mut X_MrRemoveFromGrp = std::ptr::null_mut();
    // pointer somewhere into mime_parser, must not be freed
    let mut X_MrAddToGrp = std::ptr::null_mut();
    let mut X_MrGrpNameChanged = 0;
    let mut X_MrGrpImageChanged = std::ptr::null();
    let mut better_msg: String = From::from("");
    let mut failure_reason = std::ptr::null_mut();

    let cleanup = |grpname: *mut libc::c_char,
                   failure_reason: *mut libc::c_char,
                   ret_chat_id: *mut uint32_t,
                   ret_chat_id_blocked: &mut Blocked,
                   chat_id: u32,
                   chat_id_blocked: Blocked| {
        free(grpname.cast());
        free(failure_reason.cast());

        if !ret_chat_id.is_null() {
            *ret_chat_id = chat_id;
        }
        *ret_chat_id_blocked = if 0 != chat_id {
            chat_id_blocked
        } else {
            Blocked::Not
        };
    };

    if mime_parser.is_system_message == DC_CMD_LOCATION_STREAMING_ENABLED {
        better_msg =
            context.stock_system_msg(StockMessage::MsgLocationEnabled, "", "", from_id as u32)
    }
    set_better_msg(mime_parser, &better_msg);

    // search the grpid in the header
    let optional_field = dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Group-ID");
    if !optional_field.is_null() {
        grpid = to_string((*optional_field).fld_value)
    }
    if grpid.is_empty() {
        if let Some(field) = lookup_field(mime_parser, "Message-ID", MAILIMF_FIELD_MESSAGE_ID) {
            let fld_message_id = (*field).fld_data.fld_message_id;
            if !fld_message_id.is_null() {
                if let Some(extracted_grpid) =
                    dc_extract_grpid_from_rfc724_mid(as_str((*fld_message_id).mid_value))
                {
                    grpid = extracted_grpid.to_string();
                } else {
                    grpid = "".to_string();
                }
            }
        }
        if grpid.is_empty() {
            if let Some(field) = lookup_field(mime_parser, "In-Reply-To", MAILIMF_FIELD_IN_REPLY_TO)
            {
                let fld_in_reply_to = (*field).fld_data.fld_in_reply_to;
                if !fld_in_reply_to.is_null() {
                    grpid = to_string(dc_extract_grpid_from_rfc724_mid_list(
                        (*fld_in_reply_to).mid_list,
                    ));
                }
            }
            if grpid.is_empty() {
                if let Some(field) =
                    lookup_field(mime_parser, "References", MAILIMF_FIELD_REFERENCES)
                {
                    let fld_references = (*field).fld_data.fld_references;
                    if !fld_references.is_null() {
                        grpid = to_string(dc_extract_grpid_from_rfc724_mid_list(
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
                    );
                    cleanup(
                        grpname,
                        failure_reason,
                        ret_chat_id,
                        ret_chat_id_blocked,
                        chat_id,
                        chat_id_blocked,
                    );
                    return;
                }
            }
        }
    }

    let optional_field = dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Group-Name");
    if !optional_field.is_null() {
        grpname = dc_decode_header_words((*optional_field).fld_value)
    }
    let optional_field =
        dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Group-Member-Removed");
    if !optional_field.is_null() {
        X_MrRemoveFromGrp = (*optional_field).fld_value;
        mime_parser.is_system_message = DC_CMD_MEMBER_REMOVED_FROM_GROUP;
        let left_group = (Contact::lookup_id_by_addr(context, as_str(X_MrRemoveFromGrp))
            == from_id as u32) as libc::c_int;
        better_msg = context.stock_system_msg(
            if 0 != left_group {
                StockMessage::MsgGroupLeft
            } else {
                StockMessage::MsgDelMember
            },
            as_str(X_MrRemoveFromGrp),
            "",
            from_id as u32,
        )
    } else {
        let optional_field =
            dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Group-Member-Added");
        if !optional_field.is_null() {
            X_MrAddToGrp = (*optional_field).fld_value;
            mime_parser.is_system_message = DC_CMD_MEMBER_ADDED_TO_GROUP;
            let optional_field =
                dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Group-Image");
            if !optional_field.is_null() {
                X_MrGrpImageChanged = (*optional_field).fld_value
            }
            better_msg = context.stock_system_msg(
                StockMessage::MsgAddMember,
                as_str(X_MrAddToGrp),
                "",
                from_id as u32,
            )
        } else {
            let optional_field =
                dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Group-Name-Changed");
            if !optional_field.is_null() {
                X_MrGrpNameChanged = 1;
                mime_parser.is_system_message = DC_CMD_GROUPNAME_CHANGED;
                better_msg = context.stock_system_msg(
                    StockMessage::MsgGrpName,
                    as_str((*optional_field).fld_value),
                    as_str(grpname),
                    from_id as u32,
                )
            } else {
                let optional_field =
                    dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Group-Image");
                if !optional_field.is_null() {
                    X_MrGrpImageChanged = (*optional_field).fld_value;
                    mime_parser.is_system_message = DC_CMD_GROUPIMAGE_CHANGED;
                    better_msg = context.stock_system_msg(
                        if strcmp(
                            X_MrGrpImageChanged,
                            b"0\x00" as *const u8 as *const libc::c_char,
                        ) == 0
                        {
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
    chat_id = chat::get_chat_id_by_grpid(
        context,
        &grpid,
        Some(&mut chat_id_blocked),
        &mut chat_id_verified,
    );
    if chat_id != 0 {
        if 0 != chat_id_verified
            && 0 == check_verified_properties(
                context,
                mime_parser,
                from_id as uint32_t,
                to_ids,
                &mut failure_reason,
            )
        {
            dc_mimeparser_repl_msg_by_error(mime_parser, failure_reason);
        }
    }

    // check if the sender is a member of the existing group -
    // if not, we'll recreate the group list
    if chat_id != 0 && 0 == chat::is_contact_in_chat(context, chat_id, from_id as u32) {
        recreate_member_list = 1;
    }

    // check if the group does not exist but should be created
    group_explicitly_left = chat::is_group_explicitly_left(context, &grpid).unwrap_or_default();

    let self_addr = context
        .sql
        .get_config(context, "configured_addr")
        .unwrap_or_default();
    if chat_id == 0
            && 0 == dc_mimeparser_is_mailinglist_message(mime_parser)
            && !grpid.is_empty()
            && !grpname.is_null()
            // otherwise, a pending "quit" message may pop up
            && X_MrRemoveFromGrp.is_null()
            // re-create explicitly left groups only if ourself is re-added
            && (!group_explicitly_left
                || !X_MrAddToGrp.is_null() && addr_cmp(&self_addr, as_str(X_MrAddToGrp)))
    {
        let mut create_verified = VerifiedStatus::Unverified;
        if !dc_mimeparser_lookup_field(mime_parser, "Chat-Verified").is_null() {
            create_verified = VerifiedStatus::Verified;
            if 0 == check_verified_properties(
                context,
                mime_parser,
                from_id as uint32_t,
                to_ids,
                &mut failure_reason,
            ) {
                dc_mimeparser_repl_msg_by_error(mime_parser, failure_reason);
            }
        }
        if 0 == allow_creation {
            cleanup(
                grpname,
                failure_reason,
                ret_chat_id,
                ret_chat_id_blocked,
                chat_id,
                chat_id_blocked,
            );
            return;
        }
        chat_id = create_group_record(context, &grpid, grpname, create_blocked, create_verified);
        chat_id_blocked = create_blocked;
        recreate_member_list = 1;
    }

    // again, check chat_id
    if chat_id <= DC_CHAT_ID_LAST_SPECIAL as u32 {
        chat_id = 0;
        if group_explicitly_left {
            chat_id = DC_CHAT_ID_TRASH as u32;
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
            );
        }
        cleanup(
            grpname,
            failure_reason,
            ret_chat_id,
            ret_chat_id_blocked,
            chat_id,
            chat_id_blocked,
        );
        return;
    }

    // execute group commands
    if !X_MrAddToGrp.is_null() || !X_MrRemoveFromGrp.is_null() {
        recreate_member_list = 1;
    } else if 0 != X_MrGrpNameChanged && !grpname.is_null() && strlen(grpname) < 200 {
        if sql::execute(
            context,
            &context.sql,
            "UPDATE chats SET name=? WHERE id=?;",
            params![as_str(grpname), chat_id as i32],
        )
        .is_ok()
        {
            context.call_cb(Event::CHAT_MODIFIED, chat_id as uintptr_t, 0);
        }
    }
    if !X_MrGrpImageChanged.is_null() {
        let mut ok = 0;
        let mut grpimage = 0 as *mut libc::c_char;
        if strcmp(
            X_MrGrpImageChanged,
            b"0\x00" as *const u8 as *const libc::c_char,
        ) == 0
        {
            ok = 1
        } else {
            for part in &mut mime_parser.parts {
                if part.type_0 == 20 {
                    grpimage = part
                        .param
                        .get(Param::File)
                        .map(|s| s.strdup())
                        .unwrap_or_else(|| std::ptr::null_mut());
                    ok = 1
                }
            }
        }
        if 0 != ok {
            info!(
                context,
                0,
                "New group image set to {}.",
                if !grpimage.is_null() {
                    "DELETED".to_string()
                } else {
                    to_string(grpimage)
                },
            );
            if let Ok(mut chat) = Chat::load_from_db(context, chat_id) {
                if grpimage.is_null() {
                    chat.param.remove(Param::ProfileImage);
                } else {
                    chat.param.set(Param::ProfileImage, as_str(grpimage));
                }
                chat.update_param().unwrap();
                send_EVENT_CHAT_MODIFIED = 1;
            }

            free(grpimage as *mut libc::c_void);
        }
    }

    // add members to group/check members
    // for recreation: we should add a timestamp
    if 0 != recreate_member_list {
        // TODO: the member list should only be recreated if the corresponding message is newer
        // than the one that is responsible for the current member list, see
        // https://github.com/deltachat/deltachat-core/issues/127

        let skip = if !X_MrRemoveFromGrp.is_null() {
            X_MrRemoveFromGrp
        } else {
            0 as *mut libc::c_char
        };
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM chats_contacts WHERE chat_id=?;",
            params![chat_id as i32],
        )
        .ok();
        if skip.is_null() || !addr_cmp(&self_addr, as_str(skip)) {
            chat::add_to_chat_contacts_table(context, chat_id, DC_CONTACT_ID_SELF as u32);
        }
        if from_id > DC_CHAT_ID_LAST_SPECIAL as u32 {
            if !Contact::addr_equals_contact(context, &self_addr, from_id as u32)
                && (skip.is_null()
                    || !Contact::addr_equals_contact(context, to_string(skip), from_id as u32))
            {
                chat::add_to_chat_contacts_table(context, chat_id, from_id as u32);
            }
        }
        for &to_id in to_ids.iter() {
            if !Contact::addr_equals_contact(context, &self_addr, to_id)
                && (skip.is_null()
                    || !Contact::addr_equals_contact(context, to_string(skip), to_id))
            {
                chat::add_to_chat_contacts_table(context, chat_id, to_id);
            }
        }
        send_EVENT_CHAT_MODIFIED = 1;
        chat::reset_gossiped_timestamp(context, chat_id);
    }

    if 0 != send_EVENT_CHAT_MODIFIED {
        context.call_cb(Event::CHAT_MODIFIED, chat_id as uintptr_t, 0 as uintptr_t);
    }

    // check the number of receivers -
    // the only critical situation is if the user hits "Reply" instead of "Reply all" in a non-messenger-client */
    if to_ids_cnt == 1 && mime_parser.is_send_by_messenger == 0 {
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
            );
        }
    }

    cleanup(
        grpname,
        failure_reason,
        ret_chat_id,
        ret_chat_id_blocked,
        chat_id,
        chat_id_blocked,
    );
}

/// Handle groups for received messages
unsafe fn create_or_lookup_adhoc_group(
    context: &Context,
    mime_parser: &dc_mimeparser_t,
    allow_creation: libc::c_int,
    create_blocked: Blocked,
    from_id: u32,
    to_ids: &mut Vec<u32>,
    ret_chat_id: *mut uint32_t,
    ret_chat_id_blocked: &mut Blocked,
) {
    // if we're here, no grpid was found, check there is an existing ad-hoc
    // group matching the to-list or if we can create one
    let mut chat_id = 0;
    let mut chat_id_blocked = Blocked::Not;
    let mut grpname = 0 as *mut libc::c_char;

    let cleanup = |grpname: *mut libc::c_char,
                   ret_chat_id: *mut uint32_t,
                   ret_chat_id_blocked: &mut Blocked,
                   chat_id: u32,
                   chat_id_blocked: Blocked| {
        free(grpname as *mut libc::c_void);

        if !ret_chat_id.is_null() {
            *ret_chat_id = chat_id;
        }
        *ret_chat_id_blocked = chat_id_blocked;
    };

    // build member list from the given ids
    if to_ids.is_empty() || 0 != dc_mimeparser_is_mailinglist_message(mime_parser) {
        // too few contacts or a mailinglist
        cleanup(
            grpname,
            ret_chat_id,
            ret_chat_id_blocked,
            chat_id,
            chat_id_blocked,
        );
        return;
    }

    let mut member_ids = to_ids.clone();
    if !member_ids.contains(&from_id) {
        member_ids.push(from_id);
    }
    if !member_ids.contains(&1) {
        member_ids.push(1);
    }
    if member_ids.len() < 3 {
        // too few contacts given
        cleanup(
            grpname,
            ret_chat_id,
            ret_chat_id_blocked,
            chat_id,
            chat_id_blocked,
        );
        return;
    }

    let chat_ids = search_chat_ids_by_contact_ids(context, &member_ids);
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
            cleanup(
                grpname,
                ret_chat_id,
                ret_chat_id_blocked,
                chat_id,
                chat_id_blocked,
            );
            return;
        }
    }

    if 0 == allow_creation {
        cleanup(
            grpname,
            ret_chat_id,
            ret_chat_id_blocked,
            chat_id,
            chat_id_blocked,
        );
        return;
    }
    // we do not check if the message is a reply to another group, this may result in
    // chats with unclear member list. instead we create a new group in the following lines ...

    // create a new ad-hoc group
    // - there is no need to check if this group exists; otherwise we would have caught it above
    let grpid = create_adhoc_grp_id(context, &member_ids);
    if grpid.is_empty() {
        cleanup(
            grpname,
            ret_chat_id,
            ret_chat_id_blocked,
            chat_id,
            chat_id_blocked,
        );
        return;
    }

    // use subject as initial chat name
    if !mime_parser.subject.is_null() && 0 != *mime_parser.subject.offset(0isize) as libc::c_int {
        grpname = dc_strdup(mime_parser.subject)
    } else {
        grpname = context
            .stock_string_repl_int(StockMessage::Member, member_ids.len() as libc::c_int)
            .strdup();
    }

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

    context.call_cb(Event::CHAT_MODIFIED, chat_id as uintptr_t, 0 as uintptr_t);

    cleanup(
        grpname,
        ret_chat_id,
        ret_chat_id_blocked,
        chat_id,
        chat_id_blocked,
    );
}

fn create_group_record(
    context: &Context,
    grpid: impl AsRef<str>,
    grpname: *const libc::c_char,
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
            as_str(grpname),
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

fn create_adhoc_grp_id(context: &Context, member_ids: &Vec<u32>) -> String {
    /* algorithm:
    - sort normalized, lowercased, e-mail addresses alphabetically
    - put all e-mail addresses into a single string, separate the address by a single comma
    - sha-256 this string (without possibly terminating null-characters)
    - encode the first 64 bits of the sha-256 output as lowercase hex (results in 16 characters from the set [0-9a-f])
     */
    let member_ids_str = join(member_ids.iter().map(|x| x.to_string()), ",");
    let member_cs = context
        .sql
        .get_config(context, "configured_addr")
        .unwrap_or_else(|| "no-self".to_string())
        .to_lowercase();

    let members = context
        .sql
        .query_map(
            format!(
                "SELECT addr FROM contacts WHERE id IN({}) AND id!=1",
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
unsafe fn search_chat_ids_by_contact_ids(
    context: &Context,
    unsorted_contact_ids: &Vec<u32>,
) -> Vec<u32> {
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
                    "SELECT DISTINCT cc.chat_id, cc.contact_id  FROM chats_contacts cc  LEFT JOIN chats c ON c.id=cc.chat_id  WHERE cc.chat_id IN(SELECT chat_id FROM chats_contacts WHERE contact_id IN({}))   AND c.type=120   AND cc.contact_id!=1 ORDER BY cc.chat_id, cc.contact_id;",
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
                        if chat_id as u32 != last_chat_id {
                            if matches == contact_ids.len() && mismatches == 0 {
                                chat_ids.push(last_chat_id);
                            }
                            last_chat_id = chat_id as u32;
                            matches = 0;
                            mismatches = 0;
                        }
                        if contact_id == contact_ids[matches] {
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
            ).unwrap(); // TODO: better error handling
        }
    }

    chat_ids
}

unsafe fn check_verified_properties(
    context: &Context,
    mimeparser: &dc_mimeparser_t,
    from_id: u32,
    to_ids: &Vec<u32>,
    failure_reason: *mut *mut libc::c_char,
) -> libc::c_int {
    let verify_fail = |reason: String| {
        *failure_reason = format!("{}. See \"Info\" for details.", reason).strdup();
        warn!(context, 0, "{}", reason);
    };

    let contact = match Contact::load_from_db(context, from_id) {
        Ok(contact) => contact,
        Err(_err) => {
            verify_fail("Internal Error; cannot load contact".into());
            return 0;
        }
    };

    if 0 == mimeparser.e2ee_helper.encrypted {
        verify_fail("This message is not encrypted".into());
        return 0;
    }

    // ensure, the contact is verified
    // and the message is signed with a verified key of the sender.
    // this check is skipped for SELF as there is no proper SELF-peerstate
    // and results in group-splits otherwise.
    if from_id != 1 {
        let peerstate = Peerstate::from_addr(context, &context.sql, contact.get_addr());

        if peerstate.is_none()
            || contact.is_verified_ex(peerstate.as_ref()) != VerifiedStatus::BidirectVerified
        {
            verify_fail("The sender of this message is not verified.".into());
            return 0;
        }

        if let Some(peerstate) = peerstate {
            if !peerstate.has_verified_key(&mimeparser.e2ee_helper.signatures) {
                verify_fail("The message was sent with non-verified encryption.".into());
                return 0;
            }
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
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
        |rows| {
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        },
    );

    if rows.is_err() {
        return 0;
    }
    for (to_addr, mut is_verified) in rows.unwrap().into_iter() {
        let mut peerstate = Peerstate::from_addr(context, &context.sql, &to_addr);
        if mimeparser.e2ee_helper.gossipped_addr.contains(&to_addr) && peerstate.is_some() {
            let peerstate = peerstate.as_mut().unwrap();

            // if we're here, we know the gossip key is verified:
            // - use the gossip-key as verified-key if there is no verified-key
            // - OR if the verified-key does not match public-key or gossip-key
            //   (otherwise a verified key can _only_ be updated through QR scan which might be annoying,
            //   see https://github.com/nextleap-project/countermitm/issues/46 for a discussion about this point)
            if 0 == is_verified
                || peerstate.verified_key_fingerprint != peerstate.public_key_fingerprint
                    && peerstate.verified_key_fingerprint != peerstate.gossip_key_fingerprint
            {
                info!(
                    context,
                    0,
                    "{} has verfied {}.",
                    contact.get_addr(),
                    to_addr,
                );
                let fp = peerstate.gossip_key_fingerprint.clone();
                if let Some(fp) = fp {
                    peerstate.set_verified(0, &fp, 2);
                    peerstate.save_to_db(&context.sql, false);
                    is_verified = 1;
                }
            }
        }
        if 0 == is_verified {
            verify_fail(format!(
                "{} is not a member of this verified group",
                to_addr
            ));
            return 0;
        }
    }

    1
}

unsafe fn set_better_msg<T: AsRef<str>>(mime_parser: &mut dc_mimeparser_t, better_msg: T) {
    let msg = better_msg.as_ref();
    if msg.len() > 0 && !mime_parser.parts.is_empty() {
        let part = &mut mime_parser.parts[0];
        if (*part).type_0 == 10 {
            free(part.msg as *mut libc::c_void);
            part.msg = msg.strdup();
        }
    };
}

unsafe fn dc_is_reply_to_known_message(
    context: &Context,
    mime_parser: &dc_mimeparser_t,
) -> libc::c_int {
    /* check if the message is a reply to a known message; the replies are identified by the Message-ID from
    `In-Reply-To`/`References:` (to support non-Delta-Clients) or from `Chat-Predecessor:` (Delta clients, see comment in dc_chat.c) */
    let optional_field = dc_mimeparser_lookup_optional_field(mime_parser, "Chat-Predecessor");
    if !optional_field.is_null() {
        if 0 != is_known_rfc724_mid(context, (*optional_field).fld_value) {
            return 1;
        }
    }
    let field = dc_mimeparser_lookup_field(mime_parser, "In-Reply-To");
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
        let fld_in_reply_to: *mut mailimf_in_reply_to = (*field).fld_data.fld_in_reply_to;
        if !fld_in_reply_to.is_null() {
            if 0 != is_known_rfc724_mid_in_list(
                context,
                (*(*field).fld_data.fld_in_reply_to).mid_list,
            ) {
                return 1;
            }
        }
    }
    let field = dc_mimeparser_lookup_field(mime_parser, "References");
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int {
        let fld_references: *mut mailimf_references = (*field).fld_data.fld_references;
        if !fld_references.is_null() {
            if 0 != is_known_rfc724_mid_in_list(
                context,
                (*(*field).fld_data.fld_references).mid_list,
            ) {
                return 1;
            }
        }
    }
    0
}

unsafe fn is_known_rfc724_mid_in_list(context: &Context, mid_list: *const clist) -> libc::c_int {
    if !mid_list.is_null() {
        let mut cur: *mut clistiter;
        cur = (*mid_list).first;
        while !cur.is_null() {
            if 0 != is_known_rfc724_mid(
                context,
                (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char,
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
            params![as_str(rfc724_mid)],
        )
        .unwrap_or_default() as libc::c_int
}

unsafe fn dc_is_reply_to_messenger_message(
    context: &Context,
    mime_parser: &dc_mimeparser_t,
) -> libc::c_int {
    /* function checks, if the message defined by mime_parser references a message send by us from Delta Chat.
    This is similar to is_reply_to_known_message() but
    - checks also if any of the referenced IDs are send by a messenger
    - it is okay, if the referenced messages are moved to trash here
    - no check for the Chat-* headers (function is only called if it is no messenger message itself) */
    let field = dc_mimeparser_lookup_field(mime_parser, "In-Reply-To");
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
        let fld_in_reply_to: *mut mailimf_in_reply_to = (*field).fld_data.fld_in_reply_to;
        if !fld_in_reply_to.is_null() {
            if 0 != is_msgrmsg_rfc724_mid_in_list(
                context,
                (*(*field).fld_data.fld_in_reply_to).mid_list,
            ) {
                return 1;
            }
        }
    }
    let field = dc_mimeparser_lookup_field(mime_parser, "References");
    if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int {
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
    0
}

unsafe fn is_msgrmsg_rfc724_mid_in_list(context: &Context, mid_list: *const clist) -> libc::c_int {
    if !mid_list.is_null() {
        let mut cur: *mut clistiter = (*mid_list).first;
        while !cur.is_null() {
            if 0 != is_msgrmsg_rfc724_mid(
                context,
                (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char,
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
fn is_msgrmsg_rfc724_mid(context: &Context, rfc724_mid: *const libc::c_char) -> libc::c_int {
    if rfc724_mid.is_null() {
        return 0;
    }
    context
        .sql
        .exists(
            "SELECT id FROM msgs  WHERE rfc724_mid=?  AND msgrmsg!=0  AND chat_id>9;",
            params![as_str(rfc724_mid)],
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
            0 as *mut libc::c_void
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
            0 as *mut clistcell
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
            0 as *mut libc::c_void
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
            0 as *mut clistcell
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
        .sql
        .get_config(context, "configured_addr")
        .unwrap_or_default();

    if addr_cmp(self_addr, as_str(addr_spec)) {
        *check_self = 1;
    }

    if 0 != *check_self {
        return;
    }
    /* add addr_spec if missing, update otherwise */
    let mut display_name_dec = "".to_string();
    if !display_name_enc.is_null() {
        let tmp = as_str(dc_decode_header_words(display_name_enc));
        display_name_dec = normalize_name(&tmp);
    }
    /*can be NULL*/
    let row_id = Contact::add_or_lookup(context, display_name_dec, as_str(addr_spec), origin)
        .map(|(id, _)| id)
        .unwrap_or_default();
    if 0 != row_id {
        if !ids.contains(&row_id) {
            ids.push(row_id);
        }
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

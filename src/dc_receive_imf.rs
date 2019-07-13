use mmime::mailimf::*;
use mmime::mailimf_types::*;
use mmime::mailmime::*;
use mmime::mailmime_content::*;
use mmime::mailmime_types::*;
use mmime::mmapstring::*;
use mmime::other::*;
use sha2::{Digest, Sha256};

use crate::constants::*;
use crate::context::Context;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_job::*;
use crate::dc_location::*;
use crate::dc_mimeparser::*;
use crate::dc_move::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_securejoin::*;
use crate::dc_stock::*;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::peerstate::*;
use crate::sql;
use crate::types::*;
use crate::x::*;

pub unsafe fn dc_receive_imf(
    context: &Context,
    imf_raw_not_terminated: *const libc::c_char,
    imf_raw_bytes: size_t,
    server_folder: impl AsRef<str>,
    server_uid: uint32_t,
    flags: uint32_t,
) {
    let mut current_block: u64;
    /* the function returns the number of created messages in the database */
    let mut incoming: libc::c_int = 1;
    let mut incoming_origin: libc::c_int = 0;
    let mut to_self: libc::c_int = 0;
    let mut from_id: uint32_t = 0 as uint32_t;
    let mut from_id_blocked: libc::c_int = 0;
    let mut to_id: uint32_t = 0 as uint32_t;
    let mut chat_id: uint32_t = 0 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0;
    let mut state: libc::c_int;
    let mut hidden: libc::c_int = 0;
    let mut msgrmsg: libc::c_int;
    let mut add_delete_job: libc::c_int = 0;
    let mut insert_msg_id: uint32_t = 0 as uint32_t;

    let mut i: size_t;
    let mut icnt: size_t;
    /* Message-ID from the header */
    let mut rfc724_mid = 0 as *mut libc::c_char;
    let mut sort_timestamp = 0;
    let mut sent_timestamp = 0;
    let mut rcvd_timestamp = 0;
    let mut field: *const mailimf_field;
    let mut mime_in_reply_to = 0 as *mut libc::c_char;
    let mut mime_references = 0 as *mut libc::c_char;
    let mut created_db_entries = Vec::new();
    let mut create_event_to_send = Some(Event::MSGS_CHANGED);
    let mut rr_event_to_send = Vec::new();
    let mut txt_raw = 0 as *mut libc::c_char;

    // XXX converting the below "to_ids" to a Vec quickly leads to lots of changes
    // so we keep it as a dc_array for now
    let to_ids = dc_array_new(16);
    assert!(!to_ids.is_null());

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

    let mut mime_parser = dc_mimeparser_new(context);
    dc_mimeparser_parse(&mut mime_parser, imf_raw_not_terminated, imf_raw_bytes);
    if mime_parser.header.is_empty() {
        info!(context, 0, "No header.",);
    } else {
        /* Error - even adding an empty record won't help as we do not know the message ID */
        field = dc_mimeparser_lookup_field(&mut mime_parser, "Date");
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_ORIG_DATE as libc::c_int {
            let orig_date: *mut mailimf_orig_date = (*field).fld_data.fld_orig_date;
            if !orig_date.is_null() {
                sent_timestamp = dc_timestamp_from_date((*orig_date).dt_date_time)
            }
        }
        field = dc_mimeparser_lookup_field(&mime_parser, "From");
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_FROM as libc::c_int {
            let fld_from: *mut mailimf_from = (*field).fld_data.fld_from;
            if !fld_from.is_null() {
                let mut check_self: libc::c_int = 0;
                let from_list: *mut dc_array_t = dc_array_new(16 as size_t);
                dc_add_or_lookup_contacts_by_mailbox_list(
                    context,
                    (*fld_from).frm_mb_list,
                    0x10,
                    from_list,
                    &mut check_self,
                );
                if 0 != check_self {
                    incoming = 0;
                    if 0 != dc_mimeparser_sender_equals_recipient(&mime_parser) {
                        from_id = 1 as uint32_t
                    }
                } else if dc_array_get_cnt(from_list) >= 1 {
                    from_id = dc_array_get_id(from_list, 0 as size_t);
                    incoming_origin = dc_get_contact_origin(context, from_id, &mut from_id_blocked)
                }
                dc_array_unref(from_list);
            }
        }
        field = dc_mimeparser_lookup_field(&mime_parser, "To");
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_TO as libc::c_int {
            let fld_to: *mut mailimf_to = (*field).fld_data.fld_to;
            if !fld_to.is_null() {
                dc_add_or_lookup_contacts_by_address_list(
                    context,
                    (*fld_to).to_addr_list,
                    if 0 == incoming {
                        0x4000
                    } else if incoming_origin >= 0x100 {
                        0x400
                    } else {
                        0x40
                    },
                    to_ids,
                    &mut to_self,
                );
            }
        }
        if !dc_mimeparser_get_last_nonmeta(&mime_parser).is_null() {
            field = dc_mimeparser_lookup_field(&mime_parser, "Cc");
            if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_CC as libc::c_int {
                let fld_cc: *mut mailimf_cc = (*field).fld_data.fld_cc;
                if !fld_cc.is_null() {
                    dc_add_or_lookup_contacts_by_address_list(
                        context,
                        (*fld_cc).cc_addr_list,
                        if 0 == incoming {
                            0x2000
                        } else if incoming_origin >= 0x100 {
                            0x200
                        } else {
                            0x20
                        },
                        to_ids,
                        0 as *mut libc::c_int,
                    );
                }
            }
            field = dc_mimeparser_lookup_field(&mime_parser, "Message-ID");
            if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_MESSAGE_ID as libc::c_int {
                let fld_message_id: *mut mailimf_message_id = (*field).fld_data.fld_message_id;
                if !fld_message_id.is_null() {
                    rfc724_mid = dc_strdup((*fld_message_id).mid_value)
                }
            }
            if rfc724_mid.is_null() {
                rfc724_mid = dc_create_incoming_rfc724_mid(sent_timestamp, from_id, to_ids);
                if rfc724_mid.is_null() {
                    info!(context, 0, "Cannot create Message-ID.",);
                    current_block = 16282941964262048061;
                } else {
                    current_block = 777662472977924419;
                }
            } else {
                current_block = 777662472977924419;
            }
            match current_block {
                16282941964262048061 => {}
                _ => {
                    /* check, if the mail is already in our database - if so, just update the folder/uid (if the mail was moved around) and finish.
                    (we may get a mail twice eg. if it is moved between folders. make sure, this check is done eg. before securejoin-processing) */
                    let mut old_server_folder: *mut libc::c_char = 0 as *mut libc::c_char;
                    let mut old_server_uid: uint32_t = 0 as uint32_t;
                    if 0 != dc_rfc724_mid_exists(
                        context,
                        rfc724_mid,
                        &mut old_server_folder,
                        &mut old_server_uid,
                    ) {
                        if as_str(old_server_folder) != server_folder.as_ref()
                            || old_server_uid != server_uid
                        {
                            dc_update_server_uid(
                                context,
                                rfc724_mid,
                                server_folder.as_ref(),
                                server_uid,
                            );
                        }
                        free(old_server_folder as *mut libc::c_void);
                        info!(context, 0, "Message already in DB.");
                        current_block = 16282941964262048061;
                    } else {
                        msgrmsg = mime_parser.is_send_by_messenger;
                        if msgrmsg == 0
                            && 0 != dc_is_reply_to_messenger_message(context, &mime_parser)
                        {
                            msgrmsg = 2
                        }
                        /* incoming non-chat messages may be discarded;
                        maybe this can be optimized later,
                        by checking the state before the message body is downloaded */
                        let mut allow_creation: libc::c_int = 1;
                        if msgrmsg == 0 {
                            let show_emails: libc::c_int =
                                sql::get_config_int(context, &context.sql, "show_emails", 0);
                            if show_emails == 0 {
                                chat_id = 3 as uint32_t;
                                allow_creation = 0
                            } else if show_emails == 1 {
                                allow_creation = 0
                            }
                        }
                        if 0 != incoming {
                            state = if 0 != flags & 0x1 { 16 } else { 10 };
                            to_id = 1 as uint32_t;
                            if !dc_mimeparser_lookup_field(&mime_parser, "Secure-Join").is_null() {
                                msgrmsg = 1;
                                chat_id = 0 as uint32_t;
                                allow_creation = 1;
                                let handshake: libc::c_int =
                                    dc_handle_securejoin_handshake(context, &mime_parser, from_id);
                                if 0 != handshake & 0x2 {
                                    hidden = 1;
                                    add_delete_job = handshake & 0x4;
                                    state = 16
                                }
                            }
                            let mut test_normal_chat_id: uint32_t = 0 as uint32_t;
                            let mut test_normal_chat_id_blocked: libc::c_int = 0;
                            dc_lookup_real_nchat_by_contact_id(
                                context,
                                from_id,
                                &mut test_normal_chat_id,
                                &mut test_normal_chat_id_blocked,
                            );
                            if chat_id == 0 as libc::c_uint {
                                let create_blocked: libc::c_int = if 0 != test_normal_chat_id
                                    && test_normal_chat_id_blocked == 0
                                    || incoming_origin >= 0x7fffffff
                                {
                                    0
                                } else {
                                    2
                                };
                                create_or_lookup_group(
                                    context,
                                    &mut mime_parser,
                                    allow_creation,
                                    create_blocked,
                                    from_id as int32_t,
                                    to_ids,
                                    &mut chat_id,
                                    &mut chat_id_blocked,
                                );
                                if 0 != chat_id && 0 != chat_id_blocked && 0 == create_blocked {
                                    dc_unblock_chat(context, chat_id);
                                    chat_id_blocked = 0
                                }
                            }
                            if chat_id == 0 as libc::c_uint {
                                if 0 != dc_mimeparser_is_mailinglist_message(&mime_parser) {
                                    chat_id = 3 as uint32_t;
                                    info!(
                                        context,
                                        0, "Message belongs to a mailing list and is ignored.",
                                    );
                                }
                            }
                            if chat_id == 0 as libc::c_uint {
                                let create_blocked_0: libc::c_int =
                                    if incoming_origin >= 0x7fffffff || from_id == to_id {
                                        0
                                    } else {
                                        2
                                    };
                                if 0 != test_normal_chat_id {
                                    chat_id = test_normal_chat_id;
                                    chat_id_blocked = test_normal_chat_id_blocked
                                } else if 0 != allow_creation {
                                    dc_create_or_lookup_nchat_by_contact_id(
                                        context,
                                        from_id,
                                        create_blocked_0,
                                        &mut chat_id,
                                        &mut chat_id_blocked,
                                    );
                                }
                                if 0 != chat_id && 0 != chat_id_blocked {
                                    if 0 == create_blocked_0 {
                                        dc_unblock_chat(context, chat_id);
                                        chat_id_blocked = 0
                                    } else if 0
                                        != dc_is_reply_to_known_message(context, &mime_parser)
                                    {
                                        dc_scaleup_contact_origin(context, from_id, 0x100);
                                        info!(
                                            context,
                                            0,
                                            "Message is a reply to a known message, mark sender as known.",
                                        );
                                        incoming_origin = if incoming_origin > 0x100 {
                                            incoming_origin
                                        } else {
                                            0x100
                                        }
                                    }
                                }
                            }
                            if chat_id == 0 as libc::c_uint {
                                chat_id = 3 as uint32_t
                            }
                            if 0 != chat_id_blocked && state == 10 {
                                if incoming_origin < 0x100 && msgrmsg == 0 {
                                    state = 13
                                }
                            }
                        } else {
                            state = 26;
                            from_id = 1 as uint32_t;
                            if dc_array_get_cnt(to_ids) >= 1 {
                                to_id = dc_array_get_id(to_ids, 0 as size_t);
                                if chat_id == 0 as libc::c_uint {
                                    create_or_lookup_group(
                                        context,
                                        &mut mime_parser,
                                        allow_creation,
                                        0,
                                        from_id as int32_t,
                                        to_ids,
                                        &mut chat_id,
                                        &mut chat_id_blocked,
                                    );
                                    if 0 != chat_id && 0 != chat_id_blocked {
                                        dc_unblock_chat(context, chat_id);
                                        chat_id_blocked = 0
                                    }
                                }
                                if chat_id == 0 as libc::c_uint && 0 != allow_creation {
                                    let create_blocked_1: libc::c_int =
                                        if 0 != msgrmsg && !dc_is_contact_blocked(context, to_id) {
                                            0
                                        } else {
                                            2
                                        };
                                    dc_create_or_lookup_nchat_by_contact_id(
                                        context,
                                        to_id,
                                        create_blocked_1,
                                        &mut chat_id,
                                        &mut chat_id_blocked,
                                    );
                                    if 0 != chat_id && 0 != chat_id_blocked && 0 == create_blocked_1
                                    {
                                        dc_unblock_chat(context, chat_id);
                                        chat_id_blocked = 0
                                    }
                                }
                            }
                            if chat_id == 0 as libc::c_uint {
                                if dc_array_get_cnt(to_ids) == 0 && 0 != to_self {
                                    dc_create_or_lookup_nchat_by_contact_id(
                                        context,
                                        1 as uint32_t,
                                        0,
                                        &mut chat_id,
                                        &mut chat_id_blocked,
                                    );
                                    if 0 != chat_id && 0 != chat_id_blocked {
                                        dc_unblock_chat(context, chat_id);
                                        chat_id_blocked = 0
                                    }
                                }
                            }
                            if chat_id == 0 as libc::c_uint {
                                chat_id = 3 as uint32_t
                            }
                        }
                        calc_timestamps(
                            context,
                            chat_id,
                            from_id,
                            sent_timestamp,
                            if 0 != flags & 0x1 { 0 } else { 1 },
                            &mut sort_timestamp,
                            &mut sent_timestamp,
                            &mut rcvd_timestamp,
                        );
                        dc_unarchive_chat(context, chat_id);
                        // if the mime-headers should be saved, find out its size
                        // (the mime-header ends with an empty line)
                        let save_mime_headers =
                            sql::get_config_int(context, &context.sql, "save_mime_headers", 0);
                        field = dc_mimeparser_lookup_field(&mime_parser, "In-Reply-To");
                        if !field.is_null()
                            && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int
                        {
                            let fld_in_reply_to: *mut mailimf_in_reply_to =
                                (*field).fld_data.fld_in_reply_to;
                            if !fld_in_reply_to.is_null() {
                                mime_in_reply_to = dc_str_from_clist(
                                    (*(*field).fld_data.fld_in_reply_to).mid_list,
                                    b" \x00" as *const u8 as *const libc::c_char,
                                )
                            }
                        }
                        field = dc_mimeparser_lookup_field(&mime_parser, "References");
                        if !field.is_null()
                            && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int
                        {
                            let fld_references: *mut mailimf_references =
                                (*field).fld_data.fld_references;
                            if !fld_references.is_null() {
                                mime_references = dc_str_from_clist(
                                    (*(*field).fld_data.fld_references).mid_list,
                                    b" \x00" as *const u8 as *const libc::c_char,
                                )
                            }
                        }
                        icnt = carray_count(mime_parser.parts) as size_t;

                        context.sql.prepare(
                            "INSERT INTO msgs \
                             (rfc724_mid, server_folder, server_uid, chat_id, from_id, to_id, timestamp, \
                             timestamp_sent, timestamp_rcvd, type, state, msgrmsg,  txt, txt_raw, param, \
                             bytes, hidden, mime_headers,  mime_in_reply_to, mime_references) \
                             VALUES (?,?,?,?,?,?, ?,?,?,?,?,?, ?,?,?,?,?,?, ?,?);",
                            |mut stmt| {
                                let mut i = 0;
                                loop {
                                    if !(i < icnt) {
                                        current_block = 2756754640271984560;
                                        break;
                                    }
                                    let part = carray_get(mime_parser.parts, i as libc::c_uint) as *mut dc_mimepart_t;
                                    if !(0 != (*part).is_meta) {
                                        if !mime_parser.location_kml.is_null()
                                            && icnt == 1
                                            && !(*part).msg.is_null()
                                            && (strcmp(
                                                (*part).msg,
                                                b"-location-\x00" as *const u8 as *const libc::c_char,
                                            ) == 0
                                                || *(*part).msg.offset(0isize) as libc::c_int == 0)
                                        {
                                            hidden = 1;
                                            if state == 10 {
                                                state = 13
                                            }
                                        }
                                        if (*part).type_0 == 10 {
                                            txt_raw = dc_mprintf(
                                                b"%s\n\n%s\x00" as *const u8 as *const libc::c_char,
                                                if !mime_parser.subject.is_null() {
                                                    mime_parser.subject
                                                } else {
                                                    b"\x00" as *const u8 as *const libc::c_char
                                                },
                                                (*part).msg_raw,
                                            )
                                        }
                                        if 0 != mime_parser.is_system_message {
                                            dc_param_set_int(
                                                (*part).param,
                                                'S' as i32,
                                                mime_parser.is_system_message,
                                            );
                                        }

                                        let res = stmt.execute(params![
                                            as_str(rfc724_mid),
                                            server_folder.as_ref(),
                                            server_uid as libc::c_int,
                                            chat_id as libc::c_int,
                                            from_id as libc::c_int,
                                            to_id as libc::c_int,
                                            sort_timestamp,
                                            sent_timestamp,
                                            rcvd_timestamp,
                                            (*part).type_0,
                                            state,
                                            msgrmsg,
                                            if !(*part).msg.is_null() {
                                                as_str((*part).msg)
                                            } else {
                                                ""
                                            },
                                            if !txt_raw.is_null() {
                                                as_str(txt_raw)
                                            } else {
                                                ""
                                            },
                                            as_str((*(*part).param).packed),
                                            (*part).bytes,
                                            hidden,
                                            if 0 != save_mime_headers {
                                                Some(to_string(imf_raw_not_terminated))
                                            } else {
                                                None
                                            },
                                            to_string(mime_in_reply_to),
                                            to_string(mime_references),
                                        ]);

                                        if res.is_err() {
                                            info!(context, 0, "Cannot write DB.",);
                                            /* i/o error - there is nothing more we can do - in other cases, we try to write at least an empty record */
                                            current_block = 16282941964262048061;
                                            break;
                                        } else {
                                            free(txt_raw as *mut libc::c_void);
                                            txt_raw = 0 as *mut libc::c_char;
                                            insert_msg_id = sql::get_rowid(
                                                context,
                                                &context.sql,
                                                "msgs",
                                                "rfc724_mid",
                                                as_str(rfc724_mid),
                                            );
                                            created_db_entries.push((chat_id as usize, insert_msg_id as usize));
                                        }
                                    }
                                    i = i.wrapping_add(1)
                                }
                                Ok(())
                            }
                        ).unwrap(); // TODO: better error handling
                        match current_block {
                            16282941964262048061 => {}
                            _ => {
                                info!(
                                    context,
                                    0,
                                    "Message has {} parts and is assigned to chat #{}.",
                                    icnt,
                                    chat_id,
                                );
                                if chat_id == 3 as libc::c_uint {
                                    create_event_to_send = None;
                                } else if 0 != incoming && state == 10 {
                                    if 0 != from_id_blocked {
                                        create_event_to_send = None;
                                    } else if 0 != chat_id_blocked {
                                        create_event_to_send = Some(Event::MSGS_CHANGED);
                                    } else {
                                        create_event_to_send = Some(Event::INCOMING_MSG);
                                    }
                                }
                                dc_do_heuristics_moves(
                                    context,
                                    server_folder.as_ref(),
                                    insert_msg_id,
                                );
                                current_block = 18330534242458572360;
                            }
                        }
                    }
                }
            }
        } else {
            if sent_timestamp > time() {
                sent_timestamp = time()
            }
            current_block = 18330534242458572360;
        }
        match current_block {
            16282941964262048061 => {}
            _ => {
                if carray_count(mime_parser.reports) > 0 as libc::c_uint {
                    let mdns_enabled =
                        sql::get_config_int(context, &context.sql, "mdns_enabled", 1);
                    icnt = carray_count(mime_parser.reports) as size_t;
                    i = 0 as size_t;
                    while i < icnt {
                        let mut mdn_consumed: libc::c_int = 0;
                        let report_root: *mut mailmime =
                            carray_get(mime_parser.reports, i as libc::c_uint) as *mut mailmime;
                        let report_type: *mut mailmime_parameter = mailmime_find_ct_parameter(
                            report_root,
                            b"report-type\x00" as *const u8 as *const libc::c_char,
                        );
                        if !(report_root.is_null()
                            || report_type.is_null()
                            || (*report_type).pa_value.is_null())
                        {
                            if strcmp(
                                (*report_type).pa_value,
                                b"disposition-notification\x00" as *const u8 as *const libc::c_char,
                            ) == 0
                                && (*(*report_root).mm_data.mm_multipart.mm_mp_list).count >= 2
                            {
                                if 0 != mdns_enabled {
                                    let report_data: *mut mailmime =
                                        (if !if !(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                                            .first
                                            .is_null()
                                        {
                                            (*(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                                                .first)
                                                .next
                                        } else {
                                            0 as *mut clistcell
                                        }
                                        .is_null()
                                        {
                                            (*if !(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                                                .first
                                                .is_null()
                                            {
                                                (*(*(*report_root).mm_data.mm_multipart.mm_mp_list)
                                                    .first)
                                                    .next
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
                                            b"disposition-notification\x00" as *const u8
                                                as *const libc::c_char,
                                        ) == 0
                                    {
                                        let mut report_body: *const libc::c_char =
                                            0 as *const libc::c_char;
                                        let mut report_body_bytes: size_t = 0 as size_t;
                                        let mut to_mmap_string_unref: *mut libc::c_char =
                                            0 as *mut libc::c_char;
                                        if 0 != mailmime_transfer_decode(
                                            report_data,
                                            &mut report_body,
                                            &mut report_body_bytes,
                                            &mut to_mmap_string_unref,
                                        ) {
                                            let mut report_parsed: *mut mailmime =
                                                0 as *mut mailmime;
                                            let mut dummy: size_t = 0 as size_t;
                                            if mailmime_parse(
                                                report_body,
                                                report_body_bytes,
                                                &mut dummy,
                                                &mut report_parsed,
                                            ) == MAIL_NO_ERROR as libc::c_int
                                                && !report_parsed.is_null()
                                            {
                                                let report_fields: *mut mailimf_fields =
                                                    mailmime_find_mailimf_fields(report_parsed);
                                                if !report_fields.is_null() {
                                                    let  of_disposition:
                                                    *mut mailimf_optional_field =
                                                        mailimf_find_optional_field(report_fields,
                                                                                    b"Disposition\x00"
                                                                                    as
                                                                                    *const u8
                                                                                    as
                                                                                    *const libc::c_char);
                                                    let of_org_msgid: *mut mailimf_optional_field =
                                                        mailimf_find_optional_field(
                                                            report_fields,
                                                            b"Original-Message-ID\x00" as *const u8
                                                                as *const libc::c_char,
                                                        );
                                                    if !of_disposition.is_null()
                                                        && !(*of_disposition).fld_value.is_null()
                                                        && !of_org_msgid.is_null()
                                                        && !(*of_org_msgid).fld_value.is_null()
                                                    {
                                                        let mut rfc724_mid_0: *mut libc::c_char =
                                                            0 as *mut libc::c_char;
                                                        dummy = 0 as size_t;
                                                        if mailimf_msg_id_parse(
                                                            (*of_org_msgid).fld_value,
                                                            strlen((*of_org_msgid).fld_value),
                                                            &mut dummy,
                                                            &mut rfc724_mid_0,
                                                        ) == MAIL_NO_ERROR as libc::c_int
                                                            && !rfc724_mid_0.is_null()
                                                        {
                                                            let mut chat_id_0: uint32_t =
                                                                0 as uint32_t;
                                                            let mut msg_id: uint32_t =
                                                                0 as uint32_t;
                                                            if 0 != dc_mdn_from_ext(
                                                                context,
                                                                from_id,
                                                                rfc724_mid_0,
                                                                sent_timestamp,
                                                                &mut chat_id_0,
                                                                &mut msg_id,
                                                            ) {
                                                                rr_event_to_send
                                                                    .push((chat_id_0, 0));
                                                                rr_event_to_send.push((msg_id, 0));
                                                            }
                                                            mdn_consumed = (msg_id
                                                                != 0 as libc::c_uint)
                                                                as libc::c_int;
                                                            free(rfc724_mid_0 as *mut libc::c_void);
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
                                    let param = dc_param_new();
                                    dc_param_set(
                                        param,
                                        'Z' as i32,
                                        to_cstring(server_folder.as_ref()).as_ptr(),
                                    );
                                    dc_param_set_int(param, 'z' as i32, server_uid as i32);
                                    if 0 != mime_parser.is_send_by_messenger
                                        && 0 != sql::get_config_int(
                                            context,
                                            &context.sql,
                                            "mvbox_move",
                                            1,
                                        )
                                    {
                                        dc_param_set_int(param, 'M' as i32, 1);
                                    }
                                    dc_job_add(context, 120, 0, (*param).packed, 0);
                                    dc_param_unref(param);
                                }
                            }
                        }
                        i = i.wrapping_add(1)
                    }
                }
                if !mime_parser.message_kml.is_null() && chat_id > 9 as libc::c_uint {
                    let mut location_id_written = false;
                    let mut send_event = false;

                    if !mime_parser.message_kml.is_null()
                        && chat_id > DC_CHAT_ID_LAST_SPECIAL as libc::c_uint
                    {
                        let newest_location_id: uint32_t = dc_save_locations(
                            context,
                            chat_id,
                            from_id,
                            (*mime_parser.message_kml).locations,
                            1,
                        );
                        if 0 != newest_location_id && 0 == hidden {
                            dc_set_msg_location_id(context, insert_msg_id, newest_location_id);
                            location_id_written = true;
                            send_event = true;
                        }
                    }

                    if !mime_parser.location_kml.is_null()
                        && chat_id > DC_CHAT_ID_LAST_SPECIAL as libc::c_uint
                    {
                        let contact = dc_get_contact(context, from_id);
                        if !(*mime_parser.location_kml).addr.is_null()
                            && !contact.is_null()
                            && !(*contact).addr.is_null()
                            && strcasecmp((*contact).addr, (*mime_parser.location_kml).addr) == 0
                        {
                            let newest_location_id = dc_save_locations(
                                context,
                                chat_id,
                                from_id,
                                (*mime_parser.location_kml).locations,
                                0,
                            );
                            if newest_location_id != 0 && hidden == 0 && !location_id_written {
                                dc_set_msg_location_id(context, insert_msg_id, newest_location_id);
                            }
                            send_event = true;
                        }
                        dc_contact_unref(contact);
                    }
                    if send_event {
                        context.call_cb(
                            Event::LOCATION_CHANGED,
                            from_id as uintptr_t,
                            0 as uintptr_t,
                        );
                    }
                }

                if 0 != add_delete_job && !created_db_entries.is_empty() {
                    dc_job_add(
                        context,
                        DC_JOB_DELETE_MSG_ON_IMAP,
                        created_db_entries[0].1 as i32,
                        0 as *const libc::c_char,
                        0,
                    );
                }
            }
        }
    }

    free(rfc724_mid as *mut libc::c_void);
    free(mime_in_reply_to as *mut libc::c_void);
    free(mime_references as *mut libc::c_void);
    dc_array_unref(to_ids);

    if let Some(create_event_to_send) = create_event_to_send {
        for (msg_id, insert_id) in &created_db_entries {
            context.call_cb(create_event_to_send, *msg_id, *insert_id);
        }
    }
    for (chat_id, msg_id) in &rr_event_to_send {
        context.call_cb(Event::MSG_READ, *chat_id as uintptr_t, *msg_id as uintptr_t);
    }

    free(txt_raw as *mut libc::c_void);
}

/* ******************************************************************************
 * Misc. Tools
 ******************************************************************************/
unsafe fn calc_timestamps(
    context: &Context,
    chat_id: uint32_t,
    from_id: uint32_t,
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
        let last_msg_time: Option<i64> = sql::query_row(
            context,
            &context.sql,
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

/* the function tries extracts the group-id from the message and returns the
corresponding chat_id.  If the chat_id is not existant, it is created.
If the message contains groups commands (name, profile image, changed members),
they are executed as well.

if no group-id could be extracted from the message, create_or_lookup_adhoc_group() is called
which tries to create or find out the chat_id by:
- is there a group with the same recipients? if so, use this (if there are multiple, use the most recent one)
- create an ad-hoc group based on the recipient list

So when the function returns, the caller has the group id matching the current
state of the group. */
unsafe fn create_or_lookup_group(
    context: &Context,
    mime_parser: &mut dc_mimeparser_t,
    allow_creation: libc::c_int,
    create_blocked: libc::c_int,
    from_id: int32_t,
    to_ids: *const dc_array_t,
    ret_chat_id: *mut uint32_t,
    ret_chat_id_blocked: *mut libc::c_int,
) {
    let group_explicitly_left: libc::c_int;
    let mut current_block: u64;
    let mut chat_id: uint32_t = 0 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0;
    let mut chat_id_verified: libc::c_int = 0;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpname: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut i: libc::c_int;
    let to_ids_cnt: libc::c_int = dc_array_get_cnt(to_ids) as libc::c_int;
    let mut recreate_member_list: libc::c_int = 0;
    let mut send_EVENT_CHAT_MODIFIED: libc::c_int = 0;
    /* pointer somewhere into mime_parser, must not be freed */
    let mut X_MrRemoveFromGrp: *mut libc::c_char = 0 as *mut libc::c_char;
    /* pointer somewhere into mime_parser, must not be freed */
    let mut X_MrAddToGrp: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut X_MrGrpNameChanged: libc::c_int = 0;
    let mut X_MrGrpImageChanged: *const libc::c_char = 0 as *const libc::c_char;
    let mut better_msg: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut failure_reason: *mut libc::c_char = 0 as *mut libc::c_char;
    if mime_parser.is_system_message == 8 {
        better_msg = dc_stock_system_msg(
            context,
            64,
            0 as *const libc::c_char,
            0 as *const libc::c_char,
            from_id as uint32_t,
        )
    }
    set_better_msg(mime_parser, &mut better_msg);
    /* search the grpid in the header */
    let mut field: *mut mailimf_field;
    let mut optional_field: *mut mailimf_optional_field;
    optional_field = dc_mimeparser_lookup_optional_field(
        mime_parser,
        b"Chat-Group-ID\x00" as *const u8 as *const libc::c_char,
    );
    if !optional_field.is_null() {
        grpid = dc_strdup((*optional_field).fld_value)
    }
    if grpid.is_null() {
        field = dc_mimeparser_lookup_field(mime_parser, "Message-ID");
        if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_MESSAGE_ID as libc::c_int {
            let fld_message_id: *mut mailimf_message_id = (*field).fld_data.fld_message_id;
            if !fld_message_id.is_null() {
                grpid = dc_extract_grpid_from_rfc724_mid((*fld_message_id).mid_value)
            }
        }
        if grpid.is_null() {
            field = dc_mimeparser_lookup_field(mime_parser, "In-Reply-To");
            if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_IN_REPLY_TO as libc::c_int {
                let fld_in_reply_to: *mut mailimf_in_reply_to = (*field).fld_data.fld_in_reply_to;
                if !fld_in_reply_to.is_null() {
                    grpid = dc_extract_grpid_from_rfc724_mid_list((*fld_in_reply_to).mid_list)
                }
            }
            if grpid.is_null() {
                field = dc_mimeparser_lookup_field(mime_parser, "References");
                if !field.is_null() && (*field).fld_type == MAILIMF_FIELD_REFERENCES as libc::c_int
                {
                    let fld_references: *mut mailimf_references = (*field).fld_data.fld_references;
                    if !fld_references.is_null() {
                        grpid = dc_extract_grpid_from_rfc724_mid_list((*fld_references).mid_list)
                    }
                }
                if grpid.is_null() {
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
                    current_block = 281803052766328415;
                } else {
                    current_block = 18435049525520518667;
                }
            } else {
                current_block = 18435049525520518667;
            }
        } else {
            current_block = 18435049525520518667;
        }
    } else {
        current_block = 18435049525520518667;
    }
    match current_block {
        18435049525520518667 => {
            optional_field = dc_mimeparser_lookup_optional_field(
                mime_parser,
                b"Chat-Group-Name\x00" as *const u8 as *const libc::c_char,
            );
            if !optional_field.is_null() {
                grpname = dc_decode_header_words((*optional_field).fld_value)
            }
            optional_field = dc_mimeparser_lookup_optional_field(
                mime_parser,
                b"Chat-Group-Member-Removed\x00" as *const u8 as *const libc::c_char,
            );
            if !optional_field.is_null() {
                X_MrRemoveFromGrp = (*optional_field).fld_value;
                mime_parser.is_system_message = 5;
                let left_group: libc::c_int =
                    (dc_lookup_contact_id_by_addr(context, X_MrRemoveFromGrp)
                        == from_id as libc::c_uint) as libc::c_int;
                better_msg = dc_stock_system_msg(
                    context,
                    if 0 != left_group { 19 } else { 18 },
                    X_MrRemoveFromGrp,
                    0 as *const libc::c_char,
                    from_id as uint32_t,
                )
            } else {
                optional_field = dc_mimeparser_lookup_optional_field(
                    mime_parser,
                    b"Chat-Group-Member-Added\x00" as *const u8 as *const libc::c_char,
                );
                if !optional_field.is_null() {
                    X_MrAddToGrp = (*optional_field).fld_value;
                    mime_parser.is_system_message = 4;
                    optional_field = dc_mimeparser_lookup_optional_field(
                        mime_parser,
                        b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char,
                    );
                    if !optional_field.is_null() {
                        X_MrGrpImageChanged = (*optional_field).fld_value
                    }
                    better_msg = dc_stock_system_msg(
                        context,
                        17,
                        X_MrAddToGrp,
                        0 as *const libc::c_char,
                        from_id as uint32_t,
                    )
                } else {
                    optional_field = dc_mimeparser_lookup_optional_field(
                        mime_parser,
                        b"Chat-Group-Name-Changed\x00" as *const u8 as *const libc::c_char,
                    );
                    if !optional_field.is_null() {
                        X_MrGrpNameChanged = 1;
                        mime_parser.is_system_message = 2;
                        better_msg = dc_stock_system_msg(
                            context,
                            15,
                            (*optional_field).fld_value,
                            grpname,
                            from_id as uint32_t,
                        )
                    } else {
                        optional_field = dc_mimeparser_lookup_optional_field(
                            mime_parser,
                            b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char,
                        );
                        if !optional_field.is_null() {
                            X_MrGrpImageChanged = (*optional_field).fld_value;
                            mime_parser.is_system_message = 3;
                            better_msg = dc_stock_system_msg(
                                context,
                                if strcmp(
                                    X_MrGrpImageChanged,
                                    b"0\x00" as *const u8 as *const libc::c_char,
                                ) == 0
                                {
                                    33
                                } else {
                                    16
                                },
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                                from_id as uint32_t,
                            )
                        }
                    }
                }
            }
            set_better_msg(mime_parser, &mut better_msg);
            chat_id = dc_get_chat_id_by_grpid(
                context,
                grpid,
                &mut chat_id_blocked,
                &mut chat_id_verified,
            );
            if chat_id != 0 as libc::c_uint {
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
            if chat_id != 0 as libc::c_uint
                && 0 == dc_is_contact_in_chat(context, chat_id, from_id as uint32_t)
            {
                recreate_member_list = 1
            }
            /* check if the group does not exist but should be created */
            group_explicitly_left = dc_is_group_explicitly_left(context, grpid);
            let self_addr = sql::get_config(context, &context.sql, "configured_addr", Some(""))
                .unwrap_or_default();
            if chat_id == 0 as libc::c_uint
                && 0 == dc_mimeparser_is_mailinglist_message(mime_parser)
                && !grpid.is_null()
                && !grpname.is_null()
                && X_MrRemoveFromGrp.is_null()
                && (0 == group_explicitly_left
                    || !X_MrAddToGrp.is_null() && dc_addr_cmp(&self_addr, as_str(X_MrAddToGrp)))
            {
                /*otherwise, a pending "quit" message may pop up*/
                /*re-create explicitly left groups only if ourself is re-added*/
                let mut create_verified: libc::c_int = 0;
                if !dc_mimeparser_lookup_field(mime_parser, "Chat-Verified").is_null() {
                    create_verified = 1;
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
                    current_block = 281803052766328415;
                } else {
                    chat_id = create_group_record(
                        context,
                        grpid,
                        grpname,
                        create_blocked,
                        create_verified,
                    );
                    chat_id_blocked = create_blocked;
                    recreate_member_list = 1;
                    current_block = 200744462051969938;
                }
            } else {
                current_block = 200744462051969938;
            }
            match current_block {
                281803052766328415 => {}
                _ => {
                    /* again, check chat_id */
                    if chat_id <= 9 as libc::c_uint {
                        chat_id = 0 as uint32_t;
                        if 0 != group_explicitly_left {
                            chat_id = 3 as uint32_t
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
                    } else {
                        if !X_MrAddToGrp.is_null() || !X_MrRemoveFromGrp.is_null() {
                            recreate_member_list = 1
                        } else if 0 != X_MrGrpNameChanged
                            && !grpname.is_null()
                            && strlen(grpname) < 200
                        {
                            if sql::execute(
                                context,
                                &context.sql,
                                "UPDATE chats SET name=? WHERE id=?;",
                                params![as_str(grpname), chat_id as i32],
                            ) {
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
                                let mut i_0: libc::c_int = 0;
                                while (i_0 as libc::c_uint) < carray_count(mime_parser.parts) {
                                    let part: *mut dc_mimepart_t =
                                        carray_get(mime_parser.parts, i_0 as libc::c_uint)
                                            as *mut dc_mimepart_t;
                                    if (*part).type_0 == 20 {
                                        grpimage = dc_param_get(
                                            (*part).param,
                                            'f' as i32,
                                            0 as *const libc::c_char,
                                        );
                                        ok = 1
                                    }
                                    i_0 += 1
                                }
                            }
                            if 0 != ok {
                                let chat: *mut Chat = dc_chat_new(context);
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
                                dc_chat_load_from_db(chat, chat_id);
                                dc_param_set((*chat).param, 'i' as i32, grpimage);
                                dc_chat_update_param(chat);
                                dc_chat_unref(chat);
                                free(grpimage as *mut libc::c_void);
                                send_EVENT_CHAT_MODIFIED = 1
                            }
                        }
                        if 0 != recreate_member_list {
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
                            );
                            if skip.is_null() || !dc_addr_cmp(&self_addr, as_str(skip)) {
                                dc_add_to_chat_contacts_table(context, chat_id, 1);
                            }
                            if from_id > 9 {
                                if !dc_addr_equals_contact(context, &self_addr, from_id as u32)
                                    && (skip.is_null()
                                        || !dc_addr_equals_contact(
                                            context,
                                            to_string(skip),
                                            from_id as u32,
                                        ))
                                {
                                    dc_add_to_chat_contacts_table(
                                        context,
                                        chat_id,
                                        from_id as uint32_t,
                                    );
                                }
                            }
                            i = 0;
                            while i < to_ids_cnt {
                                let to_id = dc_array_get_id(to_ids, i as size_t);
                                if !dc_addr_equals_contact(context, &self_addr, to_id)
                                    && (skip.is_null()
                                        || !dc_addr_equals_contact(context, to_string(skip), to_id))
                                {
                                    dc_add_to_chat_contacts_table(context, chat_id, to_id);
                                }
                                i += 1
                            }
                            send_EVENT_CHAT_MODIFIED = 1;
                            dc_reset_gossiped_timestamp(context, chat_id);
                        }
                        if 0 != send_EVENT_CHAT_MODIFIED {
                            context.call_cb(
                                Event::CHAT_MODIFIED,
                                chat_id as uintptr_t,
                                0 as uintptr_t,
                            );
                        }
                        /* check the number of receivers -
                        the only critical situation is if the user hits "Reply" instead of "Reply all" in a non-messenger-client */
                        if to_ids_cnt == 1 && mime_parser.is_send_by_messenger == 0 {
                            let is_contact_cnt: libc::c_int =
                                dc_get_chat_contact_cnt(context, chat_id);
                            if is_contact_cnt > 3 {
                                /* to_ids_cnt==1 may be "From: A, To: B, SELF" as SELF is not counted in to_ids_cnt. So everything up to 3 is no error. */
                                chat_id = 0 as uint32_t;
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
                    }
                }
            }
        }
        _ => {}
    }
    free(grpid as *mut libc::c_void);
    free(grpname as *mut libc::c_void);

    free(better_msg as *mut libc::c_void);
    free(failure_reason as *mut libc::c_void);
    if !ret_chat_id.is_null() {
        *ret_chat_id = chat_id
    }
    if !ret_chat_id_blocked.is_null() {
        *ret_chat_id_blocked = if 0 != chat_id { chat_id_blocked } else { 0 }
    };
}
/* ******************************************************************************
 * Handle groups for received messages
 ******************************************************************************/
unsafe fn create_or_lookup_adhoc_group(
    context: &Context,
    mime_parser: &dc_mimeparser_t,
    allow_creation: libc::c_int,
    create_blocked: libc::c_int,
    from_id: int32_t,
    to_ids: *const dc_array_t,
    ret_chat_id: *mut uint32_t,
    ret_chat_id_blocked: *mut libc::c_int,
) {
    let current_block: u64;
    /* if we're here, no grpid was found, check there is an existing ad-hoc
    group matching the to-list or if we can create one */
    let mut member_ids: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut chat_id: uint32_t = 0 as uint32_t;
    let mut chat_id_blocked = 0;
    let mut i;
    let mut chat_ids: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut chat_ids_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpname: *mut libc::c_char = 0 as *mut libc::c_char;
    /* build member list from the given ids */
    if !(dc_array_get_cnt(to_ids) == 0 || 0 != dc_mimeparser_is_mailinglist_message(mime_parser)) {
        /* too few contacts or a mailinglist */
        member_ids = dc_array_duplicate(to_ids);
        if !dc_array_search_id(member_ids, from_id as uint32_t, 0 as *mut size_t) {
            dc_array_add_id(member_ids, from_id as uint32_t);
        }
        if !dc_array_search_id(member_ids, 1 as uint32_t, 0 as *mut size_t) {
            dc_array_add_id(member_ids, 1 as uint32_t);
        }
        if !(dc_array_get_cnt(member_ids) < 3) {
            /* too few contacts given */
            chat_ids = search_chat_ids_by_contact_ids(context, member_ids);
            if dc_array_get_cnt(chat_ids) > 0 {
                chat_ids_str = dc_array_get_string(chat_ids, b",\x00" as *const u8 as *const _);
                let res = context.sql.query_row(
                    format!(
                        "SELECT c.id, c.blocked  FROM chats c  \
                         LEFT JOIN msgs m ON m.chat_id=c.id  WHERE c.id IN({})  ORDER BY m.timestamp DESC, m.id DESC  LIMIT 1;",
                        as_str(chat_ids_str),
                    ),
                    params![],
                    |row| {
                        Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?))
                    }
                );

                if let Ok((id, id_blocked)) = res {
                    chat_id = id as u32;
                    chat_id_blocked = id_blocked;
                    /* success, chat found */
                    current_block = 11334989263469503965;
                } else {
                    current_block = 11194104282611034094;
                }
            } else {
                current_block = 11194104282611034094;
            }
            match current_block {
                11334989263469503965 => {}
                _ => {
                    if !(0 == allow_creation) {
                        /* we do not check if the message is a reply to another group, this may result in
                        chats with unclear member list. instead we create a new group in the following lines ... */
                        /* create a new ad-hoc group
                        - there is no need to check if this group exists; otherwise we would have catched it above */
                        grpid = create_adhoc_grp_id(context, member_ids);
                        if !grpid.is_null() {
                            if !mime_parser.subject.is_null()
                                && 0 != *mime_parser.subject.offset(0isize) as libc::c_int
                            {
                                grpname = dc_strdup(mime_parser.subject)
                            } else {
                                grpname = dc_stock_str_repl_int(
                                    context,
                                    4,
                                    dc_array_get_cnt(member_ids) as libc::c_int,
                                )
                            }
                            chat_id =
                                create_group_record(context, grpid, grpname, create_blocked, 0);
                            chat_id_blocked = create_blocked;
                            i = 0;
                            while i < dc_array_get_cnt(member_ids) {
                                dc_add_to_chat_contacts_table(
                                    context,
                                    chat_id,
                                    dc_array_get_id(member_ids, i as size_t),
                                );
                                i += 1
                            }
                            context.call_cb(
                                Event::CHAT_MODIFIED,
                                chat_id as uintptr_t,
                                0 as uintptr_t,
                            );
                        }
                    }
                }
            }
        }
    }
    dc_array_unref(member_ids);
    dc_array_unref(chat_ids);
    free(chat_ids_str as *mut libc::c_void);
    free(grpid as *mut libc::c_void);
    free(grpname as *mut libc::c_void);

    if !ret_chat_id.is_null() {
        *ret_chat_id = chat_id
    }
    if !ret_chat_id_blocked.is_null() {
        *ret_chat_id_blocked = chat_id_blocked
    };
}

fn create_group_record(
    context: &Context,
    grpid: *const libc::c_char,
    grpname: *const libc::c_char,
    create_blocked: libc::c_int,
    create_verified: libc::c_int,
) -> u32 {
    if !sql::execute(
        context,
        &context.sql,
        "INSERT INTO chats (type, name, grpid, blocked) VALUES(?, ?, ?, ?);",
        params![
            if 0 != create_verified { 130 } else { 120 },
            as_str(grpname),
            as_str(grpid),
            create_blocked,
        ],
    ) {
        return 0;
    }

    sql::get_rowid(context, &context.sql, "chats", "grpid", as_str(grpid))
}

unsafe fn create_adhoc_grp_id(context: &Context, member_ids: *mut dc_array_t) -> *mut libc::c_char {
    /* algorithm:
    - sort normalized, lowercased, e-mail addresses alphabetically
    - put all e-mail addresses into a single string, separate the addresss by a single comma
    - sha-256 this string (without possibly terminating null-characters)
    - encode the first 64 bits of the sha-256 output as lowercase hex (results in 16 characters from the set [0-9a-f])
     */
    let member_ids_str = dc_array_get_string(member_ids, b",\x00" as *const u8 as *const _);
    let member_cs = sql::get_config(context, &context.sql, "configured_addr", Some("no-self"))
        .unwrap()
        .to_lowercase();

    let members = context
        .sql
        .query_map(
            format!(
                "SELECT addr FROM contacts WHERE id IN({}) AND id!=1",
                as_str(member_ids_str)
            ),
            params![],
            |row| row.get::<_, String>(0),
            |rows| {
                let mut addrs = rows.collect::<Result<Vec<_>, _>>()?;
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
    free(member_ids_str as *mut libc::c_void);

    hex_hash(&members) as *mut _
}

fn hex_hash(s: impl AsRef<str>) -> *const libc::c_char {
    let bytes = s.as_ref().as_bytes();
    let result = Sha256::digest(bytes);
    let result_hex = hex::encode(&result[..8]);
    let result_cstring = to_cstring(result_hex);

    unsafe { strdup(result_cstring.as_ptr()) }
}

unsafe fn search_chat_ids_by_contact_ids(
    context: &Context,
    unsorted_contact_ids: *const dc_array_t,
) -> *mut dc_array_t {
    /* searches chat_id's by the given contact IDs, may return zero, one or more chat_id's */
    let contact_ids: *mut dc_array_t = dc_array_new(23 as size_t);
    let mut contact_ids_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let chat_ids: *mut dc_array_t = dc_array_new(23 as size_t);

    /* copy array, remove duplicates and SELF, sort by ID */
    let mut i: libc::c_int;
    let iCnt: libc::c_int = dc_array_get_cnt(unsorted_contact_ids) as libc::c_int;
    if !(iCnt <= 0) {
        i = 0;
        while i < iCnt {
            let curr_id: uint32_t = dc_array_get_id(unsorted_contact_ids, i as size_t);
            if curr_id != 1 as libc::c_uint
                && !dc_array_search_id(contact_ids, curr_id, 0 as *mut size_t)
            {
                dc_array_add_id(contact_ids, curr_id);
            }
            i += 1
        }
        if !(dc_array_get_cnt(contact_ids) == 0) {
            dc_array_sort_ids(contact_ids);
            contact_ids_str =
                dc_array_get_string(contact_ids, b",\x00" as *const u8 as *const libc::c_char);

            context.sql.query_map(
                format!(
                    "SELECT DISTINCT cc.chat_id, cc.contact_id  FROM chats_contacts cc  LEFT JOIN chats c ON c.id=cc.chat_id  WHERE cc.chat_id IN(SELECT chat_id FROM chats_contacts WHERE contact_id IN({}))   AND c.type=120   AND cc.contact_id!=1 ORDER BY cc.chat_id, cc.contact_id;",
                    as_str(contact_ids_str)
                ),
                params![],
                |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?)),
                |rows| {
                    let mut last_chat_id = 0;
                    let mut matches = 0;
                    let mut mismatches = 0;

                    for row in rows {
                        let (chat_id, contact_id) = row?;
                        if chat_id as u32 != last_chat_id {
                            if matches == dc_array_get_cnt(contact_ids) && mismatches == 0 {
                                dc_array_add_id(chat_ids, last_chat_id);
                            }
                            last_chat_id = chat_id as u32;
                            matches = 0;
                            mismatches = 0;
                        }
                        if contact_id as u32 == dc_array_get_id(contact_ids, matches as size_t) {
                            matches += 1;
                        } else {
                            mismatches += 1;
                        }
                    }

                    if matches == dc_array_get_cnt(contact_ids) && mismatches == 0 {
                        dc_array_add_id(chat_ids, last_chat_id);
                    }
                Ok(())
                }
            ).unwrap(); // TODO: better error handling
        }
    }
    free(contact_ids_str as *mut libc::c_void);
    dc_array_unref(contact_ids);

    chat_ids
}

unsafe fn check_verified_properties(
    context: &Context,
    mimeparser: &dc_mimeparser_t,
    from_id: uint32_t,
    to_ids: *const dc_array_t,
    failure_reason: *mut *mut libc::c_char,
) -> libc::c_int {
    let contact = dc_contact_new(context);

    let verify_fail = |reason: String| {
        *failure_reason =
            strdup(to_cstring(format!("{}. See \"Info\" for details.", reason)).as_ptr());
        warn!(context, 0, "{}", reason);
    };

    let cleanup = || {
        dc_contact_unref(contact);
    };

    if !dc_contact_load_from_db(contact, &context.sql, from_id) {
        verify_fail("Internal Error; cannot load contact".into());
        cleanup();
        return 0;
    }

    if 0 == mimeparser.e2ee_helper.encrypted {
        verify_fail("This message is not encrypted".into());
        cleanup();
        return 0;
    }

    // ensure, the contact is verified
    // and the message is signed with a verified key of the sender.
    // this check is skipped for SELF as there is no proper SELF-peerstate
    // and results in group-splits otherwise.
    if from_id != 1 {
        let peerstate = Peerstate::from_addr(context, &context.sql, as_str((*contact).addr));

        if peerstate.is_none() || dc_contact_is_verified_ex(contact, peerstate.as_ref()) != 2 {
            verify_fail("The sender of this message is not verified.".into());
            cleanup();
            return 0;
        }

        if let Some(peerstate) = peerstate {
            if !peerstate.has_verified_key(&mimeparser.e2ee_helper.signatures) {
                verify_fail("The message was sent with non-verified encryption.".into());
                cleanup();
                return 0;
            }
        }
    }

    let to_ids_str_c = dc_array_get_string(to_ids, b",\x00" as *const u8 as *const libc::c_char);
    let to_ids_str = to_string(to_ids_str_c);
    free(to_ids_str_c as *mut libc::c_void);

    let ok = context
        .sql
        .query_map(
            format!(
                "SELECT c.addr, LENGTH(ps.verified_key_fingerprint)  FROM contacts c  \
                 LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE c.id IN({}) ",
                &to_ids_str,
            ),
            params![],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
            |rows| {
                for row in rows {
                    let (to_addr, mut is_verified) = row?;
                    let mut peerstate = Peerstate::from_addr(context, &context.sql, &to_addr);
                    if mimeparser.e2ee_helper.gossipped_addr.contains(&to_addr)
                        && peerstate.is_some()
                    {
                        let peerstate = peerstate.as_mut().unwrap();

                        // if we're here, we know the gossip key is verified:
                        // - use the gossip-key as verified-key if there is no verified-key
                        // - OR if the verified-key does not match public-key or gossip-key
                        //   (otherwise a verified key can _only_ be updated through QR scan which might be annoying,
                        //   see https://github.com/nextleap-project/countermitm/issues/46 for a discussion about this point)
                        if 0 == is_verified
                            || peerstate.verified_key_fingerprint
                                != peerstate.public_key_fingerprint
                                && peerstate.verified_key_fingerprint
                                    != peerstate.gossip_key_fingerprint
                        {
                            info!(
                                context,
                                0,
                                "{} has verfied {}.",
                                as_str((*contact).addr),
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
                        cleanup();
                        return Err(failure::format_err!("not a valid memember").into());
                    }
                }
                Ok(())
            },
        )
        .is_ok(); // TODO: Better default

    cleanup();

    ok as libc::c_int
}

unsafe fn set_better_msg(mime_parser: &dc_mimeparser_t, better_msg: *mut *mut libc::c_char) {
    if !(*better_msg).is_null() && carray_count((*mime_parser).parts) > 0 as libc::c_uint {
        let mut part: *mut dc_mimepart_t =
            carray_get(mime_parser.parts, 0 as libc::c_uint) as *mut dc_mimepart_t;
        if (*part).type_0 == 10 {
            free((*part).msg as *mut libc::c_void);
            (*part).msg = *better_msg;
            *better_msg = 0 as *mut libc::c_char
        }
    };
}
unsafe fn dc_is_reply_to_known_message(
    context: &Context,
    mime_parser: &dc_mimeparser_t,
) -> libc::c_int {
    /* check if the message is a reply to a known message; the replies are identified by the Message-ID from
    `In-Reply-To`/`References:` (to support non-Delta-Clients) or from `Chat-Predecessor:` (Delta clients, see comment in dc_chat.c) */
    let optional_field = dc_mimeparser_lookup_optional_field(
        mime_parser,
        b"Chat-Predecessor\x00" as *const u8 as *const libc::c_char,
    );
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
    return 0;
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
                0 as *mut clistcell
            }
        }
    }
    return 0;
}

/* ******************************************************************************
 * Check if a message is a reply to a known message (messenger or non-messenger)
 ******************************************************************************/

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
    return 0;
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
                0 as *mut clistcell
            }
        }
    }
    return 0;
}
/* ******************************************************************************
 * Check if a message is a reply to any messenger message
 ******************************************************************************/
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
    origin: libc::c_int,
    ids: *mut dc_array_t,
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
    origin: libc::c_int,
    ids: *mut dc_array_t,
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
/* ******************************************************************************
 * Add contacts to database on receiving messages
 ******************************************************************************/
unsafe fn add_or_lookup_contact_by_addr(
    context: &Context,
    display_name_enc: *const libc::c_char,
    addr_spec: *const libc::c_char,
    origin: libc::c_int,
    ids: *mut dc_array_t,
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
    let self_addr = sql::get_config(context, &context.sql, "configured_addr", Some("")).unwrap();

    if dc_addr_cmp(self_addr, as_str(addr_spec)) {
        *check_self = 1;
    }

    if 0 != *check_self {
        return;
    }
    /* add addr_spec if missing, update otherwise */
    let mut display_name_dec = 0 as *mut libc::c_char;
    if !display_name_enc.is_null() {
        display_name_dec = dc_decode_header_words(display_name_enc);
        dc_normalize_name(display_name_dec);
    }
    /*can be NULL*/
    let row_id = dc_add_or_lookup_contact(
        context,
        display_name_dec,
        addr_spec,
        origin,
        0 as *mut libc::c_int,
    );
    free(display_name_dec as *mut libc::c_void);
    if 0 != row_id {
        if !dc_array_search_id(ids, row_id, 0 as *mut size_t) {
            dc_array_add_id(ids, row_id);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_hash() {
        let data = "hello world";

        let res_c = hex_hash(data);
        let res = to_string(res_c);
        assert_eq!(res, "b94d27b9934d3e08");
    }
}

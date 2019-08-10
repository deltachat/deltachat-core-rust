use std::ffi::CString;

use mmime::mailimf_types::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::aheader::EncryptPreference;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_configure::*;
use crate::dc_e2ee::*;
use crate::dc_lot::*;
use crate::dc_mimeparser::*;
use crate::dc_msg::*;
use crate::dc_qr::*;
use crate::dc_strencode::*;
use crate::dc_token::*;
use crate::dc_tools::*;
use crate::key::*;
use crate::param::*;
use crate::peerstate::*;
use crate::stock::StockMessage;
use crate::types::*;
use crate::x::*;

pub unsafe fn dc_get_securejoin_qr(
    context: &Context,
    group_chat_id: uint32_t,
) -> *mut libc::c_char {
    /* =========================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    ========================================================= */

    let mut fingerprint = 0 as *mut libc::c_char;
    let mut invitenumber: *mut libc::c_char;
    let mut auth: *mut libc::c_char;
    let mut chat = 0 as *mut Chat;
    let mut group_name = 0 as *mut libc::c_char;
    let mut group_name_urlencoded = 0 as *mut libc::c_char;
    let mut qr: Option<String> = None;

    dc_ensure_secret_key_exists(context).ok();
    invitenumber = dc_token_lookup(context, DC_TOKEN_INVITENUMBER, group_chat_id);
    if invitenumber.is_null() {
        invitenumber = dc_create_id().strdup();
        dc_token_save(context, DC_TOKEN_INVITENUMBER, group_chat_id, invitenumber);
    }
    auth = dc_token_lookup(context, DC_TOKEN_AUTH, group_chat_id);
    if auth.is_null() {
        auth = dc_create_id().strdup();
        dc_token_save(context, DC_TOKEN_AUTH, group_chat_id, auth);
    }
    let self_addr = context.sql.get_config(context, "configured_addr");

    let cleanup = |fingerprint, chat, group_name, group_name_urlencoded| {
        free(fingerprint as *mut libc::c_void);
        free(invitenumber as *mut libc::c_void);
        free(auth as *mut libc::c_void);
        dc_chat_unref(chat);
        free(group_name as *mut libc::c_void);
        free(group_name_urlencoded as *mut libc::c_void);

        if let Some(qr) = qr {
            qr.strdup()
        } else {
            std::ptr::null_mut()
        }
    };

    if self_addr.is_none() {
        error!(context, 0, "Not configured, cannot generate QR code.",);
        return cleanup(fingerprint, chat, group_name, group_name_urlencoded);
    }

    let self_addr = self_addr.unwrap();
    let self_name = context
        .sql
        .get_config(context, "displayname")
        .unwrap_or_default();

    fingerprint = get_self_fingerprint(context);

    if fingerprint.is_null() {
        return cleanup(fingerprint, chat, group_name, group_name_urlencoded);
    }

    let self_addr_urlencoded = utf8_percent_encode(&self_addr, NON_ALPHANUMERIC).to_string();
    let self_name_urlencoded = utf8_percent_encode(&self_name, NON_ALPHANUMERIC).to_string();

    qr = if 0 != group_chat_id {
        chat = dc_get_chat(context, group_chat_id);
        if chat.is_null() {
            error!(
                context,
                0, "Cannot get QR-code for chat-id {}", group_chat_id,
            );
            return cleanup(fingerprint, chat, group_name, group_name_urlencoded);
        }

        group_name = dc_chat_get_name(chat);
        group_name_urlencoded = dc_urlencode(group_name);

        Some(format!(
            "OPENPGP4FPR:{}#a={}&g={}&x={}&i={}&s={}",
            as_str(fingerprint),
            self_addr_urlencoded,
            as_str(group_name_urlencoded),
            as_str((*chat).grpid),
            as_str(invitenumber),
            as_str(auth),
        ))
    } else {
        Some(format!(
            "OPENPGP4FPR:{}#a={}&n={}&i={}&s={}",
            as_str(fingerprint),
            self_addr_urlencoded,
            self_name_urlencoded,
            as_str(invitenumber),
            as_str(auth),
        ))
    };

    info!(context, 0, "Generated QR code: {}", qr.as_ref().unwrap());

    cleanup(fingerprint, chat, group_name, group_name_urlencoded)
}

fn get_self_fingerprint(context: &Context) -> *mut libc::c_char {
    if let Some(self_addr) = context.sql.get_config(context, "configured_addr") {
        if let Some(key) = Key::from_self_public(context, self_addr, &context.sql) {
            return key.fingerprint_c();
        }
    }

    std::ptr::null_mut()
}

pub unsafe fn dc_join_securejoin(context: &Context, qr: *const libc::c_char) -> uint32_t {
    /* ==========================================================
    ====             Bob - the joiner's side             =====
    ====   Step 2 in "Setup verified contact" protocol   =====
    ========================================================== */
    let mut ret_chat_id: libc::c_int = 0i32;
    let ongoing_allocated: libc::c_int;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;
    let mut join_vg: libc::c_int = 0i32;
    let mut qr_scan: *mut dc_lot_t = 0 as *mut dc_lot_t;
    info!(context, 0, "Requesting secure-join ...",);
    dc_ensure_secret_key_exists(context).ok();
    ongoing_allocated = dc_alloc_ongoing(context);
    if !(ongoing_allocated == 0i32) {
        qr_scan = dc_check_qr(context, qr);
        if qr_scan.is_null() || (*qr_scan).state != 200i32 && (*qr_scan).state != 202i32 {
            error!(context, 0, "Unknown QR code.",);
        } else {
            contact_chat_id = dc_create_chat_by_contact_id(context, (*qr_scan).id);
            if contact_chat_id == 0i32 as libc::c_uint {
                error!(context, 0, "Unknown contact.",);
            } else if !(context
                .running_state
                .clone()
                .read()
                .unwrap()
                .shall_stop_ongoing)
            {
                join_vg = ((*qr_scan).state == 202i32) as libc::c_int;
                {
                    let bob_a = context.bob.clone();
                    let mut bob = bob_a.write().unwrap();
                    bob.status = 0;
                    bob.qr_scan = qr_scan;
                }
                if 0 != fingerprint_equals_sender(context, (*qr_scan).fingerprint, contact_chat_id)
                {
                    info!(context, 0, "Taking protocol shortcut.");
                    context.bob.clone().write().unwrap().expects = 6;
                    context.call_cb(
                        Event::SECUREJOIN_JOINER_PROGRESS,
                        chat_id_2_contact_id(context, contact_chat_id) as uintptr_t,
                        400i32 as uintptr_t,
                    );
                    let own_fingerprint: *mut libc::c_char = get_self_fingerprint(context);
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char
                        },
                        (*qr_scan).auth,
                        own_fingerprint,
                        if 0 != join_vg {
                            (*qr_scan).text2
                        } else {
                            0 as *mut libc::c_char
                        },
                    );
                    free(own_fingerprint as *mut libc::c_void);
                } else {
                    context.bob.clone().write().unwrap().expects = 2;
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-request\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-request\x00" as *const u8 as *const libc::c_char
                        },
                        (*qr_scan).invitenumber,
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                }

                // Bob -> Alice
                while !(context
                    .running_state
                    .clone()
                    .read()
                    .unwrap()
                    .shall_stop_ongoing)
                {
                    std::thread::sleep(std::time::Duration::new(0, 3_000_000));
                }
            }
        }
    }
    let bob_a = context.bob.clone();
    let mut bob = bob_a.write().unwrap();

    bob.expects = 0;
    if bob.status == 1 {
        if 0 != join_vg {
            ret_chat_id = dc_get_chat_id_by_grpid(
                context,
                (*qr_scan).text2,
                0 as *mut libc::c_int,
                0 as *mut libc::c_int,
            ) as libc::c_int
        } else {
            ret_chat_id = contact_chat_id as libc::c_int
        }
    }
    bob.qr_scan = std::ptr::null_mut();

    dc_lot_unref(qr_scan);
    if 0 != ongoing_allocated {
        dc_free_ongoing(context);
    }
    ret_chat_id as uint32_t
}

unsafe fn send_handshake_msg(
    context: &Context,
    contact_chat_id: uint32_t,
    step: *const libc::c_char,
    param2: *const libc::c_char,
    fingerprint: *const libc::c_char,
    grpid: *const libc::c_char,
) {
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    (*msg).type_0 = Viewtype::Text;
    (*msg).text = Some(format!("Secure-Join: {}", to_string(step)));
    (*msg).hidden = 1;
    (*msg).param.set_int(Param::Cmd, 7);
    if step.is_null() {
        (*msg).param.remove(Param::Arg);
    } else {
        (*msg).param.set(Param::Arg, as_str(step));
    }
    if !param2.is_null() {
        (*msg).param.set(Param::Arg2, as_str(param2));
    }
    if !fingerprint.is_null() {
        (*msg).param.set(Param::Arg3, as_str(fingerprint));
    }
    if !grpid.is_null() {
        (*msg).param.set(Param::Arg4, as_str(grpid));
    }
    if strcmp(step, b"vg-request\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(step, b"vc-request\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        (*msg).param.set_int(
            Param::ForcePlaintext,
            ForcePlaintext::AddAutocryptHeader as i32,
        );
    } else {
        (*msg).param.set_int(Param::GuranteeE2ee, 1);
    }
    dc_send_msg(context, contact_chat_id, msg);
    dc_msg_unref(msg);
}

unsafe fn chat_id_2_contact_id(context: &Context, contact_chat_id: uint32_t) -> uint32_t {
    let mut contact_id: uint32_t = 0i32 as uint32_t;
    let contacts: *mut dc_array_t = dc_get_chat_contacts(context, contact_chat_id);
    if !(dc_array_get_cnt(contacts) != 1) {
        contact_id = dc_array_get_id(contacts, 0i32 as size_t)
    }
    dc_array_unref(contacts);

    contact_id
}

unsafe fn fingerprint_equals_sender(
    context: &Context,
    fingerprint: *const libc::c_char,
    contact_chat_id: uint32_t,
) -> libc::c_int {
    if fingerprint.is_null() {
        return 0;
    }
    let mut fingerprint_equal: libc::c_int = 0i32;
    let contacts = dc_get_chat_contacts(context, contact_chat_id);

    if !(dc_array_get_cnt(contacts) != 1) {
        if let Ok(contact) = Contact::load_from_db(context, dc_array_get_id(contacts, 0)) {
            if let Some(peerstate) = Peerstate::from_addr(context, &context.sql, contact.get_addr())
            {
                let fingerprint_normalized = dc_normalize_fingerprint(as_str(fingerprint));
                if peerstate.public_key_fingerprint.is_some()
                    && &fingerprint_normalized == peerstate.public_key_fingerprint.as_ref().unwrap()
                {
                    fingerprint_equal = 1;
                }
            }
        } else {
            return 0;
        }
    }
    dc_array_unref(contacts);

    fingerprint_equal
}

/* library private: secure-join */
pub unsafe fn dc_handle_securejoin_handshake(
    context: &Context,
    mimeparser: &dc_mimeparser_t,
    contact_id: uint32_t,
) -> libc::c_int {
    let mut current_block: u64;
    let step: *const libc::c_char;
    let join_vg: libc::c_int;
    let mut scanned_fingerprint_of_alice: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut auth: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut own_fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;
    let mut contact_chat_id_blocked: libc::c_int = 0i32;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: libc::c_int = 0i32;

    if !(contact_id <= 9i32 as libc::c_uint) {
        step = lookup_field(mimeparser, "Secure-Join");
        if !step.is_null() {
            info!(
                context,
                0,
                ">>>>>>>>>>>>>>>>>>>>>>>>> secure-join message \'{}\' received",
                as_str(step),
            );
            join_vg = (strncmp(step, b"vg-\x00" as *const u8 as *const libc::c_char, 3) == 0)
                as libc::c_int;
            dc_create_or_lookup_nchat_by_contact_id(
                context,
                contact_id,
                0i32,
                &mut contact_chat_id,
                &mut contact_chat_id_blocked,
            );
            if 0 != contact_chat_id_blocked {
                dc_unblock_chat(context, contact_chat_id);
            }
            ret = 0x2i32;
            if strcmp(step, b"vg-request\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(step, b"vc-request\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                /* =========================================================
                ====             Alice - the inviter side            ====
                ====   Step 3 in "Setup verified contact" protocol   ====
                ========================================================= */
                // this message may be unencrypted (Bob, the joinder and the sender, might not have Alice's key yet)
                // it just ensures, we have Bobs key now. If we do _not_ have the key because eg. MitM has removed it,
                // send_message() will fail with the error "End-to-end-encryption unavailable unexpectedly.", so, there is no additional check needed here.
                // verify that the `Secure-Join-Invitenumber:`-header matches invitenumber written to the QR code
                let invitenumber: *const libc::c_char;
                invitenumber = lookup_field(mimeparser, "Secure-Join-Invitenumber");
                if invitenumber.is_null() {
                    warn!(context, 0, "Secure-join denied (invitenumber missing).",);
                    current_block = 4378276786830486580;
                } else if !dc_token_exists(context, DC_TOKEN_INVITENUMBER, invitenumber) {
                    warn!(context, 0, "Secure-join denied (bad invitenumber).",);
                    current_block = 4378276786830486580;
                } else {
                    info!(context, 0, "Secure-join requested.",);

                    context.call_cb(
                        Event::SECUREJOIN_INVITER_PROGRESS,
                        contact_id as uintptr_t,
                        300i32 as uintptr_t,
                    );
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-auth-required\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-auth-required\x00" as *const u8 as *const libc::c_char
                        },
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                    current_block = 10256747982273457880;
                }
            } else if strcmp(
                step,
                b"vg-auth-required\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-auth-required\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                let cond = {
                    let bob_a = context.bob.clone();
                    let bob = bob_a.read().unwrap();
                    let scan = bob.qr_scan;
                    scan.is_null() || bob.expects != 2 || 0 != join_vg && (*scan).state != 202
                };

                if cond {
                    warn!(context, 0, "auth-required message out of sync.",);
                    // no error, just aborted somehow or a mail from another handshake
                    current_block = 4378276786830486580;
                } else {
                    {
                        let scan = context.bob.clone().read().unwrap().qr_scan;
                        scanned_fingerprint_of_alice = dc_strdup((*scan).fingerprint);
                        auth = dc_strdup((*scan).auth);
                        if 0 != join_vg {
                            grpid = dc_strdup((*scan).text2)
                        }
                    }
                    if 0 == encrypted_and_signed(mimeparser, scanned_fingerprint_of_alice) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            if 0 != mimeparser.e2ee_helper.encrypted {
                                b"No valid signature.\x00" as *const u8 as *const libc::c_char
                            } else {
                                b"Not encrypted.\x00" as *const u8 as *const libc::c_char
                            },
                        );
                        end_bobs_joining(context, 0i32);
                        current_block = 4378276786830486580;
                    } else if 0
                        == fingerprint_equals_sender(
                            context,
                            scanned_fingerprint_of_alice,
                            contact_chat_id,
                        )
                    {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Fingerprint mismatch on joiner-side.\x00" as *const u8
                                as *const libc::c_char,
                        );
                        end_bobs_joining(context, 0i32);
                        current_block = 4378276786830486580;
                    } else {
                        info!(context, 0, "Fingerprint verified.",);
                        own_fingerprint = get_self_fingerprint(context);
                        context.call_cb(
                            Event::SECUREJOIN_JOINER_PROGRESS,
                            contact_id as uintptr_t,
                            400i32 as uintptr_t,
                        );
                        context.bob.clone().write().unwrap().expects = 6;

                        send_handshake_msg(
                            context,
                            contact_chat_id,
                            if 0 != join_vg {
                                b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char
                            } else {
                                b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char
                            },
                            auth,
                            own_fingerprint,
                            grpid,
                        );
                        current_block = 10256747982273457880;
                    }
                }
            } else if strcmp(
                step,
                b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                /* ============================================================
                ====              Alice - the inviter side              ====
                ====   Steps 5+6 in "Setup verified contact" protocol   ====
                ====  Step 6 in "Out-of-band verified groups" protocol  ====
                ============================================================ */
                // verify that Secure-Join-Fingerprint:-header matches the fingerprint of Bob
                let fingerprint: *const libc::c_char;
                fingerprint = lookup_field(mimeparser, "Secure-Join-Fingerprint");
                if fingerprint.is_null() {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint not provided.\x00" as *const u8 as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if 0 == encrypted_and_signed(mimeparser, fingerprint) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Auth not encrypted.\x00" as *const u8 as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if 0 == fingerprint_equals_sender(context, fingerprint, contact_chat_id) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    info!(context, 0, "Fingerprint verified.",);
                    // verify that the `Secure-Join-Auth:`-header matches the secret written to the QR code
                    let auth_0: *const libc::c_char;
                    auth_0 = lookup_field(mimeparser, "Secure-Join-Auth");
                    if auth_0.is_null() {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Auth not provided.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else if !dc_token_exists(context, DC_TOKEN_AUTH, auth_0) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Auth invalid.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else if 0 == mark_peer_as_verified(context, fingerprint) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                                as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else {
                        Contact::scaleup_origin_by_id(
                            context,
                            contact_id,
                            Origin::SecurejoinInvited,
                        );
                        info!(context, 0, "Auth verified.",);
                        secure_connection_established(context, contact_chat_id);
                        context.call_cb(
                            Event::CONTACTS_CHANGED,
                            contact_id as uintptr_t,
                            0i32 as uintptr_t,
                        );
                        context.call_cb(
                            Event::SECUREJOIN_INVITER_PROGRESS,
                            contact_id as uintptr_t,
                            600i32 as uintptr_t,
                        );
                        if 0 != join_vg {
                            grpid = dc_strdup(lookup_field(mimeparser, "Secure-Join-Group"));
                            let group_chat_id: uint32_t = dc_get_chat_id_by_grpid(
                                context,
                                grpid,
                                0 as *mut libc::c_int,
                                0 as *mut libc::c_int,
                            );
                            if group_chat_id == 0i32 as libc::c_uint {
                                error!(context, 0, "Chat {} not found.", as_str(grpid),);
                                current_block = 4378276786830486580;
                            } else {
                                dc_add_contact_to_chat_ex(
                                    context,
                                    group_chat_id,
                                    contact_id,
                                    0x1i32,
                                );
                                current_block = 10256747982273457880;
                            }
                        } else {
                            send_handshake_msg(
                                context,
                                contact_chat_id,
                                b"vc-contact-confirm\x00" as *const u8 as *const libc::c_char,
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                            );
                            context.call_cb(
                                Event::SECUREJOIN_INVITER_PROGRESS,
                                contact_id as uintptr_t,
                                1000i32 as uintptr_t,
                            );
                            current_block = 10256747982273457880;
                        }
                    }
                }
            } else if strcmp(
                step,
                b"vg-member-added\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-contact-confirm\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                if 0 != join_vg {
                    ret = 0x1i32
                }
                if context.bob.clone().read().unwrap().expects != 6 {
                    info!(context, 0, "Message belongs to a different handshake.",);
                    current_block = 4378276786830486580;
                } else {
                    let cond = {
                        let scan = context.bob.clone().read().unwrap().qr_scan;
                        scan.is_null() || 0 != join_vg && (*scan).state != 202
                    };
                    if cond {
                        warn!(
                            context,
                            0, "Message out of sync or belongs to a different handshake.",
                        );
                        current_block = 4378276786830486580;
                    } else {
                        {
                            let scan = context.bob.clone().read().unwrap().qr_scan;
                            scanned_fingerprint_of_alice = dc_strdup((*scan).fingerprint);
                            if 0 != join_vg {
                                grpid = dc_strdup((*scan).text2)
                            }
                        }
                        let mut vg_expect_encrypted: libc::c_int = 1i32;
                        if 0 != join_vg {
                            let mut is_verified_group: libc::c_int = 0i32;
                            dc_get_chat_id_by_grpid(
                                context,
                                grpid,
                                0 as *mut libc::c_int,
                                &mut is_verified_group,
                            );
                            if 0 == is_verified_group {
                                vg_expect_encrypted = 0i32
                            }
                        }
                        if 0 != vg_expect_encrypted {
                            if 0 == encrypted_and_signed(mimeparser, scanned_fingerprint_of_alice) {
                                could_not_establish_secure_connection(
                                    context,
                                    contact_chat_id,
                                    b"Contact confirm message not encrypted.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                end_bobs_joining(context, 0i32);
                                current_block = 4378276786830486580;
                            } else {
                                current_block = 5195798230510548452;
                            }
                        } else {
                            current_block = 5195798230510548452;
                        }
                        match current_block {
                            4378276786830486580 => {}
                            _ => {
                                if 0 == mark_peer_as_verified(context, scanned_fingerprint_of_alice)
                                {
                                    could_not_establish_secure_connection(
                                        context,
                                        contact_chat_id,
                                        b"Fingerprint mismatch on joiner-side.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    current_block = 4378276786830486580;
                                } else {
                                    Contact::scaleup_origin_by_id(
                                        context,
                                        contact_id,
                                        Origin::SecurejoinJoined,
                                    );
                                    context.call_cb(
                                        Event::CONTACTS_CHANGED,
                                        0i32 as uintptr_t,
                                        0i32 as uintptr_t,
                                    );
                                    if 0 != join_vg {
                                        if !addr_equals_self(
                                            context,
                                            as_str(lookup_field(
                                                mimeparser,
                                                "Chat-Group-Member-Added",
                                            )),
                                        ) {
                                            info!(
                                                context,
                                                0,
                                                "Message belongs to a different handshake (scaled up contact anyway to allow creation of group)."
                                            );
                                            current_block = 4378276786830486580;
                                        } else {
                                            current_block = 9180031981464905198;
                                        }
                                    } else {
                                        current_block = 9180031981464905198;
                                    }
                                    match current_block {
                                        4378276786830486580 => {}
                                        _ => {
                                            secure_connection_established(context, contact_chat_id);
                                            context.bob.clone().write().unwrap().expects = 0;
                                            if 0 != join_vg {
                                                send_handshake_msg(
                                                    context,
                                                    contact_chat_id,
                                                    b"vg-member-added-received\x00" as *const u8
                                                        as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                );
                                            }
                                            end_bobs_joining(context, 1i32);
                                            current_block = 10256747982273457880;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if strcmp(
                step,
                b"vg-member-added-received\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
            {
                /* ============================================================
                ====              Alice - the inviter side              ====
                ====  Step 8 in "Out-of-band verified groups" protocol  ====
                ============================================================ */
                if let Ok(contact) = Contact::get_by_id(context, contact_id) {
                    if contact.is_verified() == VerifiedStatus::Unverified {
                        warn!(context, 0, "vg-member-added-received invalid.",);
                        current_block = 4378276786830486580;
                    } else {
                        context.call_cb(
                            Event::SECUREJOIN_INVITER_PROGRESS,
                            contact_id as uintptr_t,
                            800i32 as uintptr_t,
                        );
                        context.call_cb(
                            Event::SECUREJOIN_INVITER_PROGRESS,
                            contact_id as uintptr_t,
                            1000i32 as uintptr_t,
                        );
                        current_block = 10256747982273457880;
                    }
                } else {
                    warn!(context, 0, "vg-member-added-received invalid.",);
                    current_block = 4378276786830486580;
                }
            } else {
                current_block = 10256747982273457880;
            }
            match current_block {
                4378276786830486580 => {}
                _ => {
                    if 0 != ret & 0x2i32 {
                        ret |= 0x4i32
                    }
                }
            }
        }
    }

    free(scanned_fingerprint_of_alice as *mut libc::c_void);
    free(auth as *mut libc::c_void);
    free(own_fingerprint as *mut libc::c_void);
    free(grpid as *mut libc::c_void);

    ret
}

unsafe fn end_bobs_joining(context: &Context, status: libc::c_int) {
    context.bob.clone().write().unwrap().status = status;
    dc_stop_ongoing_process(context);
}

unsafe fn secure_connection_established(context: &Context, contact_chat_id: uint32_t) {
    let contact_id: uint32_t = chat_id_2_contact_id(context, contact_chat_id);
    let contact = Contact::get_by_id(context, contact_id);
    let addr = if let Ok(ref contact) = contact {
        contact.get_addr()
    } else {
        "?"
    };
    let msg =
        CString::new(context.stock_string_repl_str(StockMessage::ContactVerified, addr)).unwrap();
    dc_add_device_msg(context, contact_chat_id, msg.as_ptr());
    context.call_cb(
        Event::CHAT_MODIFIED,
        contact_chat_id as uintptr_t,
        0i32 as uintptr_t,
    );
}

unsafe fn lookup_field(mimeparser: &dc_mimeparser_t, key: &str) -> *const libc::c_char {
    let mut value: *const libc::c_char = 0 as *const libc::c_char;
    let field: *mut mailimf_field = dc_mimeparser_lookup_field(mimeparser, key);
    if field.is_null()
        || (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
        || (*field).fld_data.fld_optional_field.is_null()
        || {
            value = (*(*field).fld_data.fld_optional_field).fld_value;
            value.is_null()
        }
    {
        return 0 as *const libc::c_char;
    }

    value
}

unsafe fn could_not_establish_secure_connection(
    context: &Context,
    contact_chat_id: uint32_t,
    details: *const libc::c_char,
) {
    let contact_id: uint32_t = chat_id_2_contact_id(context, contact_chat_id);
    let contact = Contact::get_by_id(context, contact_id);
    let msg = context.stock_string_repl_str(
        StockMessage::ContactNotVerified,
        if let Ok(ref contact) = contact {
            contact.get_addr()
        } else {
            "?"
        },
    );
    let msg_c = CString::new(msg.as_str()).unwrap();
    dc_add_device_msg(context, contact_chat_id, msg_c.as_ptr());
    error!(context, 0, "{} ({})", msg, as_str(details));
}

unsafe fn mark_peer_as_verified(
    context: &Context,
    fingerprint: *const libc::c_char,
) -> libc::c_int {
    let mut success = 0;

    if let Some(ref mut peerstate) =
        Peerstate::from_fingerprint(context, &context.sql, as_str(fingerprint))
    {
        if peerstate.set_verified(1, as_str(fingerprint), 2) {
            peerstate.prefer_encrypt = EncryptPreference::Mutual;
            peerstate.to_save = Some(ToSave::All);
            peerstate.save_to_db(&context.sql, false);
            success = 1;
        }
    }

    success
}

/* ******************************************************************************
 * Tools: Misc.
 ******************************************************************************/

// TODO should return bool
unsafe fn encrypted_and_signed(
    mimeparser: &dc_mimeparser_t,
    expected_fingerprint: *const libc::c_char,
) -> libc::c_int {
    if 0 == mimeparser.e2ee_helper.encrypted {
        warn!(mimeparser.context, 0, "Message not encrypted.",);
        return 0i32;
    }
    if mimeparser.e2ee_helper.signatures.len() <= 0 {
        warn!(mimeparser.context, 0, "Message not signed.",);
        return 0i32;
    }
    if expected_fingerprint.is_null() {
        warn!(mimeparser.context, 0, "Fingerprint for comparison missing.",);
        return 0i32;
    }
    if !mimeparser
        .e2ee_helper
        .signatures
        .contains(as_str(expected_fingerprint))
    {
        warn!(
            mimeparser.context,
            0,
            "Message does not match expected fingerprint {}.",
            as_str(expected_fingerprint),
        );
        return 0;
    }

    1
}

pub unsafe fn dc_handle_degrade_event(context: &Context, peerstate: &Peerstate) {
    let mut contact_chat_id = 0;

    // - we do not issue an warning for DC_DE_ENCRYPTION_PAUSED as this is quite normal
    // - currently, we do not issue an extra warning for DC_DE_VERIFICATION_LOST - this always comes
    //   together with DC_DE_FINGERPRINT_CHANGED which is logged, the idea is not to bother
    //   with things they cannot fix, so the user is just kicked from the verified group
    //   (and he will know this and can fix this)
    if Some(DegradeEvent::FingerprintChanged) == peerstate.degrade_event {
        let contact_id: i32 = context
            .sql
            .query_row_col(
                context,
                "SELECT id FROM contacts WHERE addr=?;",
                params![&peerstate.addr],
                0,
            )
            .unwrap_or_default();
        if contact_id > 0 {
            dc_create_or_lookup_nchat_by_contact_id(
                context,
                contact_id as u32,
                2,
                &mut contact_chat_id,
                0 as *mut libc::c_int,
            );
            let peeraddr: &str = match peerstate.addr {
                Some(ref addr) => &addr,
                None => "",
            };
            let msg = CString::new(
                context.stock_string_repl_str(StockMessage::ContactSetupChanged, peeraddr),
            )
            .unwrap();
            dc_add_device_msg(context, contact_chat_id, msg.as_ptr());
            context.call_cb(
                Event::CHAT_MODIFIED,
                contact_chat_id as uintptr_t,
                0 as uintptr_t,
            );
        }
    }
}

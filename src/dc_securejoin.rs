use mmime::mailimf_types::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::ptr;

use crate::aheader::EncryptPreference;
use crate::chat::{self, Chat};
use crate::configure::*;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_mimeparser::*;
use crate::dc_token::*;
use crate::dc_tools::*;
use crate::e2ee::*;
use crate::key::*;
use crate::lot::LotState;
use crate::message::*;
use crate::param::*;
use crate::peerstate::*;
use crate::qr::check_qr;
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
    let cleanup = |fingerprint, invitenumber, auth, qr: Option<String>| {
        free(fingerprint as *mut libc::c_void);
        free(invitenumber as *mut libc::c_void);
        free(auth as *mut libc::c_void);

        if let Some(qr) = qr {
            qr.strdup()
        } else {
            "".strdup()
        }
    };

    let mut fingerprint = ptr::null_mut();
    let mut qr: Option<String> = None;

    ensure_secret_key_exists(context).ok();
    let invitenumber = dc_token_lookup(context, DC_TOKEN_INVITENUMBER, group_chat_id)
        .unwrap_or_else(|| {
            let invitenumber_s = dc_create_id();
            dc_token_save(
                context,
                DC_TOKEN_INVITENUMBER,
                group_chat_id,
                &invitenumber_s,
            );
            invitenumber_s
        });
    let auth = dc_token_lookup(context, DC_TOKEN_AUTH, group_chat_id).unwrap_or_else(|| {
        let auth_s = dc_create_id();
        dc_token_save(context, DC_TOKEN_AUTH, group_chat_id, &auth_s);
        auth_s
    });
    let self_addr = context.sql.get_config(context, "configured_addr");

    if self_addr.is_none() {
        error!(context, 0, "Not configured, cannot generate QR code.",);
        return cleanup(fingerprint, invitenumber, auth, qr);
    }

    let self_addr = self_addr.unwrap();
    let self_name = context
        .sql
        .get_config(context, "displayname")
        .unwrap_or_default();

    fingerprint = get_self_fingerprint(context);

    if fingerprint.is_null() {
        return cleanup(fingerprint, invitenumber, auth, qr);
    }

    let self_addr_urlencoded = utf8_percent_encode(&self_addr, NON_ALPHANUMERIC).to_string();
    let self_name_urlencoded = utf8_percent_encode(&self_name, NON_ALPHANUMERIC).to_string();

    qr = if 0 != group_chat_id {
        if let Ok(chat) = Chat::load_from_db(context, group_chat_id) {
            let group_name = chat.get_name();
            let group_name_urlencoded =
                utf8_percent_encode(&group_name, NON_ALPHANUMERIC).to_string();

            Some(format!(
                "OPENPGP4FPR:{}#a={}&g={}&x={}&i={}&s={}",
                as_str(fingerprint),
                self_addr_urlencoded,
                &group_name_urlencoded,
                &chat.grpid,
                &invitenumber,
                &auth,
            ))
        } else {
            error!(
                context,
                0, "Cannot get QR-code for chat-id {}", group_chat_id,
            );
            return cleanup(fingerprint, invitenumber, auth, qr);
        }
    } else {
        Some(format!(
            "OPENPGP4FPR:{}#a={}&n={}&i={}&s={}",
            as_str(fingerprint),
            self_addr_urlencoded,
            self_name_urlencoded,
            &invitenumber,
            &auth,
        ))
    };

    info!(context, 0, "Generated QR code: {}", qr.as_ref().unwrap());

    return cleanup(fingerprint, invitenumber, auth, qr);
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

    info!(context, 0, "Requesting secure-join ...",);
    ensure_secret_key_exists(context).ok();
    ongoing_allocated = dc_alloc_ongoing(context);

    if !(ongoing_allocated == 0i32) {
        let qr_scan = check_qr(context, as_str(qr));
        if qr_scan.state != LotState::QrAskVerifyContact
            && qr_scan.state != LotState::QrAskVerifyGroup
        {
            error!(context, 0, "Unknown QR code.",);
        } else {
            contact_chat_id = chat::create_by_contact_id(context, qr_scan.id).unwrap_or_default();
            if contact_chat_id == 0i32 as libc::c_uint {
                error!(context, 0, "Unknown contact.",);
            } else if !(context
                .running_state
                .clone()
                .read()
                .unwrap()
                .shall_stop_ongoing)
            {
                join_vg = (qr_scan.get_state() == LotState::QrAskVerifyGroup) as libc::c_int;
                {
                    let mut bob = context.bob.write().unwrap();
                    bob.status = 0;
                    bob.qr_scan = Some(qr_scan);
                }
                if 0 != fingerprint_equals_sender(
                    context,
                    context
                        .bob
                        .read()
                        .unwrap()
                        .qr_scan
                        .as_ref()
                        .unwrap()
                        .fingerprint
                        .as_ref()
                        .unwrap(),
                    contact_chat_id,
                ) {
                    info!(context, 0, "Taking protocol shortcut.");
                    context.bob.write().unwrap().expects = 6;
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
                        context
                            .bob
                            .read()
                            .unwrap()
                            .qr_scan
                            .as_ref()
                            .unwrap()
                            .auth
                            .as_ref()
                            .unwrap()
                            .to_string(),
                        own_fingerprint,
                        if 0 != join_vg {
                            context
                                .bob
                                .read()
                                .unwrap()
                                .qr_scan
                                .as_ref()
                                .unwrap()
                                .text2
                                .as_ref()
                                .unwrap()
                                .to_string()
                        } else {
                            "".to_string()
                        },
                    );
                    free(own_fingerprint as *mut libc::c_void);
                } else {
                    context.bob.write().unwrap().expects = 2;
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-request\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-request\x00" as *const u8 as *const libc::c_char
                        },
                        context
                            .bob
                            .read()
                            .unwrap()
                            .qr_scan
                            .as_ref()
                            .unwrap()
                            .invitenumber
                            .as_ref()
                            .unwrap(),
                        ptr::null(),
                        "",
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
    let mut bob = context.bob.write().unwrap();
    bob.expects = 0;
    if bob.status == 1 {
        if 0 != join_vg {
            ret_chat_id = chat::get_chat_id_by_grpid(
                context,
                bob.qr_scan.as_ref().unwrap().text2.as_ref().unwrap(),
                None,
                ptr::null_mut(),
            ) as libc::c_int
        } else {
            ret_chat_id = contact_chat_id as libc::c_int
        }
    }

    bob.qr_scan = None;

    if 0 != ongoing_allocated {
        dc_free_ongoing(context);
    }
    ret_chat_id as uint32_t
}

unsafe fn send_handshake_msg(
    context: &Context,
    contact_chat_id: uint32_t,
    step: *const libc::c_char,
    param2: impl AsRef<str>,
    fingerprint: *const libc::c_char,
    grpid: impl AsRef<str>,
) {
    let mut msg = dc_msg_new_untyped(context);
    msg.type_0 = Viewtype::Text;
    msg.text = Some(format!("Secure-Join: {}", to_string(step)));
    msg.hidden = true;
    msg.param.set_int(Param::Cmd, 7);
    if step.is_null() {
        msg.param.remove(Param::Arg);
    } else {
        msg.param.set(Param::Arg, as_str(step));
    }
    if !param2.as_ref().is_empty() {
        msg.param.set(Param::Arg2, param2);
    }
    if !fingerprint.is_null() {
        msg.param.set(Param::Arg3, as_str(fingerprint));
    }
    if !grpid.as_ref().is_empty() {
        msg.param.set(Param::Arg4, grpid.as_ref());
    }
    if strcmp(step, b"vg-request\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(step, b"vc-request\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        msg.param.set_int(
            Param::ForcePlaintext,
            ForcePlaintext::AddAutocryptHeader as i32,
        );
    } else {
        msg.param.set_int(Param::GuranteeE2ee, 1);
    }
    // TODO. handle cleanup on error
    chat::send_msg(context, contact_chat_id, &mut msg).unwrap();
}

unsafe fn chat_id_2_contact_id(context: &Context, contact_chat_id: uint32_t) -> uint32_t {
    let contacts = chat::get_chat_contacts(context, contact_chat_id);
    if contacts.len() == 1 {
        contacts[0]
    } else {
        0
    }
}

unsafe fn fingerprint_equals_sender(
    context: &Context,
    fingerprint: impl AsRef<str>,
    contact_chat_id: u32,
) -> libc::c_int {
    let mut fingerprint_equal = 0;
    let contacts = chat::get_chat_contacts(context, contact_chat_id);

    if contacts.len() == 1 {
        if let Ok(contact) = Contact::load_from_db(context, contacts[0]) {
            if let Some(peerstate) = Peerstate::from_addr(context, &context.sql, contact.get_addr())
            {
                let fingerprint_normalized = dc_normalize_fingerprint(fingerprint.as_ref());
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

    fingerprint_equal
}

/* library private: secure-join */
pub unsafe fn dc_handle_securejoin_handshake(
    context: &Context,
    mimeparser: &dc_mimeparser_t,
    contact_id: uint32_t,
) -> libc::c_int {
    let mut ok_to_continue: bool;
    let step: *const libc::c_char;
    let join_vg: libc::c_int;
    let mut own_fingerprint: *mut libc::c_char = ptr::null_mut();
    let contact_chat_id: u32;
    let contact_chat_id_blocked: Blocked;
    let mut grpid = "".to_string();
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
            let (id, bl) = chat::create_or_lookup_by_contact_id(context, contact_id, Blocked::Not)
                .unwrap_or_default();
            contact_chat_id = id;
            contact_chat_id_blocked = bl;

            if Blocked::Not != contact_chat_id_blocked {
                chat::unblock(context, contact_chat_id);
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
                    ok_to_continue = false;
                } else if !dc_token_exists(context, DC_TOKEN_INVITENUMBER, as_str(invitenumber)) {
                    warn!(context, 0, "Secure-join denied (bad invitenumber).",);
                    ok_to_continue = false;
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
                        "",
                        ptr::null(),
                        "",
                    );
                    ok_to_continue = true;
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
                    let bob = context.bob.read().unwrap();
                    let scan = bob.qr_scan.as_ref();
                    scan.is_none()
                        || bob.expects != 2
                        || 0 != join_vg && scan.unwrap().state != LotState::QrAskVerifyGroup
                };

                if cond {
                    warn!(context, 0, "auth-required message out of sync.",);
                    // no error, just aborted somehow or a mail from another handshake
                    ok_to_continue = false;
                } else {
                    let scanned_fingerprint_of_alice = context
                        .bob
                        .read()
                        .unwrap()
                        .qr_scan
                        .as_ref()
                        .unwrap()
                        .fingerprint
                        .as_ref()
                        .unwrap()
                        .to_string();

                    let auth = context
                        .bob
                        .read()
                        .unwrap()
                        .qr_scan
                        .as_ref()
                        .unwrap()
                        .auth
                        .as_ref()
                        .unwrap()
                        .to_string();
                    if 0 != join_vg {
                        grpid = context
                            .bob
                            .read()
                            .unwrap()
                            .qr_scan
                            .as_ref()
                            .unwrap()
                            .text2
                            .as_ref()
                            .unwrap()
                            .to_string();
                    }

                    if !encrypted_and_signed(mimeparser, &scanned_fingerprint_of_alice) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            if mimeparser.e2ee_helper.encrypted {
                                b"No valid signature.\x00" as *const u8 as *const libc::c_char
                            } else {
                                b"Not encrypted.\x00" as *const u8 as *const libc::c_char
                            },
                        );
                        end_bobs_joining(context, 0i32);
                        ok_to_continue = false;
                    } else if 0
                        == fingerprint_equals_sender(
                            context,
                            &scanned_fingerprint_of_alice,
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
                        ok_to_continue = false;
                    } else {
                        info!(context, 0, "Fingerprint verified.",);
                        own_fingerprint = get_self_fingerprint(context);
                        context.call_cb(
                            Event::SECUREJOIN_JOINER_PROGRESS,
                            contact_id as uintptr_t,
                            400i32 as uintptr_t,
                        );
                        context.bob.write().unwrap().expects = 6;

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
                        ok_to_continue = true;
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
                let fingerprint = lookup_field(mimeparser, "Secure-Join-Fingerprint");
                if fingerprint.is_null() {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint not provided.\x00" as *const u8 as *const libc::c_char,
                    );
                    ok_to_continue = false;
                } else if !encrypted_and_signed(mimeparser, as_str(fingerprint)) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Auth not encrypted.\x00" as *const u8 as *const libc::c_char,
                    );
                    ok_to_continue = false;
                } else if 0
                    == fingerprint_equals_sender(context, as_str(fingerprint), contact_chat_id)
                {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    ok_to_continue = false;
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
                        ok_to_continue = false;
                    } else if !dc_token_exists(context, DC_TOKEN_AUTH, as_str(auth_0)) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Auth invalid.\x00" as *const u8 as *const libc::c_char,
                        );
                        ok_to_continue = false;
                    } else if 0 == mark_peer_as_verified(context, as_str(fingerprint)) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                                as *const libc::c_char,
                        );
                        ok_to_continue = false;
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
                            grpid = to_string(lookup_field(mimeparser, "Secure-Join-Group"));
                            let group_chat_id: uint32_t =
                                chat::get_chat_id_by_grpid(context, &grpid, None, ptr::null_mut());
                            if group_chat_id == 0i32 as libc::c_uint {
                                error!(context, 0, "Chat {} not found.", &grpid);
                                ok_to_continue = false;
                            } else {
                                chat::add_contact_to_chat_ex(
                                    context,
                                    group_chat_id,
                                    contact_id,
                                    0x1i32,
                                );
                                ok_to_continue = true;
                            }
                        } else {
                            send_handshake_msg(
                                context,
                                contact_chat_id,
                                b"vc-contact-confirm\x00" as *const u8 as *const libc::c_char,
                                "",
                                ptr::null(),
                                "",
                            );
                            context.call_cb(
                                Event::SECUREJOIN_INVITER_PROGRESS,
                                contact_id as uintptr_t,
                                1000i32 as uintptr_t,
                            );
                            ok_to_continue = true;
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
                if context.bob.read().unwrap().expects != 6 {
                    info!(context, 0, "Message belongs to a different handshake.",);
                    ok_to_continue = false;
                } else {
                    let cond = {
                        let bob = context.bob.read().unwrap();
                        let scan = bob.qr_scan.as_ref();
                        scan.is_none()
                            || 0 != join_vg && scan.unwrap().state != LotState::QrAskVerifyGroup
                    };
                    if cond {
                        warn!(
                            context,
                            0, "Message out of sync or belongs to a different handshake.",
                        );
                        ok_to_continue = false;
                    } else {
                        let scanned_fingerprint_of_alice = context
                            .bob
                            .read()
                            .unwrap()
                            .qr_scan
                            .as_ref()
                            .unwrap()
                            .fingerprint
                            .as_ref()
                            .unwrap()
                            .to_string();

                        if 0 != join_vg {
                            grpid = context
                                .bob
                                .read()
                                .unwrap()
                                .qr_scan
                                .as_ref()
                                .unwrap()
                                .text2
                                .as_ref()
                                .unwrap()
                                .to_string();
                        }

                        let mut vg_expect_encrypted: libc::c_int = 1i32;
                        if 0 != join_vg {
                            let mut is_verified_group: libc::c_int = 0i32;
                            chat::get_chat_id_by_grpid(
                                context,
                                grpid,
                                None,
                                &mut is_verified_group,
                            );
                            if 0 == is_verified_group {
                                vg_expect_encrypted = 0i32
                            }
                        }
                        if 0 != vg_expect_encrypted {
                            if !encrypted_and_signed(mimeparser, &scanned_fingerprint_of_alice) {
                                could_not_establish_secure_connection(
                                    context,
                                    contact_chat_id,
                                    b"Contact confirm message not encrypted.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                end_bobs_joining(context, 0i32);
                                ok_to_continue = false;
                            } else {
                                ok_to_continue = true;
                            }
                        } else {
                            ok_to_continue = true;
                        }
                        if ok_to_continue {
                            if 0 == mark_peer_as_verified(context, &scanned_fingerprint_of_alice) {
                                could_not_establish_secure_connection(
                                    context,
                                    contact_chat_id,
                                    b"Fingerprint mismatch on joiner-side.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                ok_to_continue = false;
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
                                        as_str(lookup_field(mimeparser, "Chat-Group-Member-Added")),
                                    ) {
                                        info!(
                                                context,
                                                0,
                                                "Message belongs to a different handshake (scaled up contact anyway to allow creation of group)."
                                            );
                                        ok_to_continue = false;
                                    } else {
                                        ok_to_continue = true;
                                    }
                                } else {
                                    ok_to_continue = true;
                                }
                                if ok_to_continue {
                                    secure_connection_established(context, contact_chat_id);
                                    context.bob.write().unwrap().expects = 0;
                                    if 0 != join_vg {
                                        send_handshake_msg(
                                            context,
                                            contact_chat_id,
                                            b"vg-member-added-received\x00" as *const u8
                                                as *const libc::c_char,
                                            "",
                                            ptr::null(),
                                            "",
                                        );
                                    }
                                    end_bobs_joining(context, 1i32);
                                    ok_to_continue = true;
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
                        ok_to_continue = false;
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
                        ok_to_continue = true;
                    }
                } else {
                    warn!(context, 0, "vg-member-added-received invalid.",);
                    ok_to_continue = false;
                }
            } else {
                ok_to_continue = true;
            }
            if ok_to_continue {
                if 0 != ret & 0x2i32 {
                    ret |= 0x4i32
                }
            }
        }
    }

    free(own_fingerprint as *mut libc::c_void);

    ret
}

unsafe fn end_bobs_joining(context: &Context, status: libc::c_int) {
    context.bob.write().unwrap().status = status;
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
    let msg = context.stock_string_repl_str(StockMessage::ContactVerified, addr);
    chat::add_device_msg(context, contact_chat_id, msg);
    context.call_cb(
        Event::CHAT_MODIFIED,
        contact_chat_id as uintptr_t,
        0i32 as uintptr_t,
    );
}

unsafe fn lookup_field(mimeparser: &dc_mimeparser_t, key: &str) -> *const libc::c_char {
    let mut value: *const libc::c_char = ptr::null();
    let field: *mut mailimf_field = dc_mimeparser_lookup_field(mimeparser, key);
    if field.is_null()
        || (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
        || (*field).fld_data.fld_optional_field.is_null()
        || {
            value = (*(*field).fld_data.fld_optional_field).fld_value;
            value.is_null()
        }
    {
        return ptr::null();
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

    chat::add_device_msg(context, contact_chat_id, &msg);
    error!(context, 0, "{} ({})", &msg, as_str(details));
}

unsafe fn mark_peer_as_verified(context: &Context, fingerprint: impl AsRef<str>) -> libc::c_int {
    let mut success = 0;

    if let Some(ref mut peerstate) =
        Peerstate::from_fingerprint(context, &context.sql, fingerprint.as_ref())
    {
        if peerstate.set_verified(1, fingerprint.as_ref(), 2) {
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

unsafe fn encrypted_and_signed(
    mimeparser: &dc_mimeparser_t,
    expected_fingerprint: impl AsRef<str>,
) -> bool {
    if !mimeparser.e2ee_helper.encrypted {
        warn!(mimeparser.context, 0, "Message not encrypted.",);
        return false;
    }
    if mimeparser.e2ee_helper.signatures.len() <= 0 {
        warn!(mimeparser.context, 0, "Message not signed.",);
        return false;
    }
    if expected_fingerprint.as_ref().is_empty() {
        warn!(mimeparser.context, 0, "Fingerprint for comparison missing.",);
        return false;
    }
    if !mimeparser
        .e2ee_helper
        .signatures
        .contains(expected_fingerprint.as_ref())
    {
        warn!(
            mimeparser.context,
            0,
            "Message does not match expected fingerprint {}.",
            expected_fingerprint.as_ref(),
        );
        return false;
    }

    true
}

pub unsafe fn dc_handle_degrade_event(context: &Context, peerstate: &Peerstate) {
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
            let (contact_chat_id, _) =
                chat::create_or_lookup_by_contact_id(context, contact_id as u32, Blocked::Deaddrop)
                    .unwrap_or_default();

            let peeraddr: &str = match peerstate.addr {
                Some(ref addr) => &addr,
                None => "",
            };
            let msg = context.stock_string_repl_str(StockMessage::ContactSetupChanged, peeraddr);

            chat::add_device_msg(context, contact_chat_id, msg);
            context.call_cb(
                Event::CHAT_MODIFIED,
                contact_chat_id as uintptr_t,
                0 as uintptr_t,
            );
        }
    }
}

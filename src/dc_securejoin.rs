use mmime::mailimf_types::*;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
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
use crate::error::Error;
use crate::key::*;
use crate::lot::LotState;
use crate::message::*;
use crate::param::*;
use crate::peerstate::*;
use crate::qr::check_qr;
use crate::stock::StockMessage;
use crate::types::*;

pub const NON_ALPHANUMERIC_WITHOUT_DOT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'.');

pub unsafe fn dc_get_securejoin_qr(context: &Context, group_chat_id: uint32_t) -> Option<String> {
    /* =========================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    ========================================================= */

    let fingerprint: String;

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
        return None;
    }

    let self_addr = self_addr.unwrap();
    let self_name = context
        .sql
        .get_config(context, "displayname")
        .unwrap_or_default();

    fingerprint = match get_self_fingerprint(context) {
        Some(fp) => fp,
        None => {
            return None;
        }
    };

    let self_addr_urlencoded =
        utf8_percent_encode(&self_addr, NON_ALPHANUMERIC_WITHOUT_DOT).to_string();
    let self_name_urlencoded =
        utf8_percent_encode(&self_name, NON_ALPHANUMERIC_WITHOUT_DOT).to_string();

    let qr = if 0 != group_chat_id {
        if let Ok(chat) = Chat::load_from_db(context, group_chat_id) {
            let group_name = chat.get_name();
            let group_name_urlencoded =
                utf8_percent_encode(&group_name, NON_ALPHANUMERIC).to_string();

            Some(format!(
                "OPENPGP4FPR:{}#a={}&g={}&x={}&i={}&s={}",
                fingerprint,
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
            return None;
        }
    } else {
        Some(format!(
            "OPENPGP4FPR:{}#a={}&n={}&i={}&s={}",
            fingerprint, self_addr_urlencoded, self_name_urlencoded, &invitenumber, &auth,
        ))
    };

    info!(context, 0, "Generated QR code: {}", qr.as_ref().unwrap());

    qr
}

fn get_self_fingerprint(context: &Context) -> Option<String> {
    if let Some(self_addr) = context.sql.get_config(context, "configured_addr") {
        if let Some(key) = Key::from_self_public(context, self_addr, &context.sql) {
            return Some(key.fingerprint());
        }
    }
    None
}

pub unsafe fn dc_join_securejoin(context: &Context, qr: &str) -> uint32_t {
    /* ==========================================================
    ====             Bob - the joiner's side             =====
    ====   Step 2 in "Setup verified contact" protocol   =====
    ========================================================== */
    let ongoing_allocated: libc::c_int;
    let mut contact_chat_id: uint32_t = 0;
    let mut join_vg: bool = false;

    info!(context, 0, "Requesting secure-join ...",);
    ensure_secret_key_exists(context).ok();
    ongoing_allocated = dc_alloc_ongoing(context);

    if ongoing_allocated != 0 {
        let qr_scan = check_qr(context, &qr);
        if qr_scan.state != LotState::QrAskVerifyContact
            && qr_scan.state != LotState::QrAskVerifyGroup
        {
            error!(context, 0, "Unknown QR code.",);
        } else {
            contact_chat_id = chat::create_by_contact_id(context, qr_scan.id).unwrap_or_default();
            if contact_chat_id == 0 {
                error!(context, 0, "Unknown contact.",);
            } else if !(context
                .running_state
                .clone()
                .read()
                .unwrap()
                .shall_stop_ongoing)
            {
                join_vg = qr_scan.get_state() == LotState::QrAskVerifyGroup;
                {
                    let mut bob = context.bob.write().unwrap();
                    bob.status = 0;
                    bob.qr_scan = Some(qr_scan);
                }
                if fingerprint_equals_sender(
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
                    let own_fingerprint = get_self_fingerprint(context).unwrap();
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if join_vg {
                            "vg-request-with-auth"
                        } else {
                            "vc-request-with-auth"
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
                        Some(own_fingerprint),
                        if join_vg {
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
                } else {
                    context.bob.write().unwrap().expects = 2;
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if join_vg { "vg-request" } else { "vc-request" },
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
                        None,
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
    let ret_chat_id = if bob.status == 1 {
        if join_vg {
            chat::get_chat_id_by_grpid(
                context,
                bob.qr_scan.as_ref().unwrap().text2.as_ref().unwrap(),
            )
            .0
        } else {
            contact_chat_id
        }
    } else {
        0
    };
    bob.qr_scan = None;

    if 0 != ongoing_allocated {
        dc_free_ongoing(context);
    }
    ret_chat_id as uint32_t
}

unsafe fn send_handshake_msg(
    context: &Context,
    contact_chat_id: uint32_t,
    step: &str,
    param2: impl AsRef<str>,
    fingerprint: Option<String>,
    grpid: impl AsRef<str>,
) {
    let mut msg = dc_msg_new_untyped(context);
    msg.type_0 = Viewtype::Text;
    msg.text = Some(format!("Secure-Join: {}", step));
    msg.hidden = true;
    msg.param.set_int(Param::Cmd, 7);
    if step.is_empty() {
        msg.param.remove(Param::Arg);
    } else {
        msg.param.set(Param::Arg, step);
    }
    if !param2.as_ref().is_empty() {
        msg.param.set(Param::Arg2, param2);
    }
    if let Some(fp) = fingerprint {
        msg.param.set(Param::Arg3, fp);
    }
    if !grpid.as_ref().is_empty() {
        msg.param.set(Param::Arg4, grpid.as_ref());
    }
    if step == "vg-request" || step == "vc-request" {
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

fn chat_id_2_contact_id(context: &Context, contact_chat_id: uint32_t) -> uint32_t {
    let contacts = chat::get_chat_contacts(context, contact_chat_id);
    if contacts.len() == 1 {
        contacts[0]
    } else {
        0
    }
}

fn fingerprint_equals_sender(
    context: &Context,
    fingerprint: impl AsRef<str>,
    contact_chat_id: u32,
) -> bool {
    let contacts = chat::get_chat_contacts(context, contact_chat_id);

    if contacts.len() == 1 {
        if let Ok(contact) = Contact::load_from_db(context, contacts[0]) {
            if let Some(peerstate) = Peerstate::from_addr(context, &context.sql, contact.get_addr())
            {
                let fingerprint_normalized = dc_normalize_fingerprint(fingerprint.as_ref());
                if peerstate.public_key_fingerprint.is_some()
                    && &fingerprint_normalized == peerstate.public_key_fingerprint.as_ref().unwrap()
                {
                    return true;
                }
            }
        }
    }
    false
}

/* library private: secure-join */
pub unsafe fn dc_handle_securejoin_handshake(
    context: &Context,
    mimeparser: &dc_mimeparser_t,
    contact_id: uint32_t,
) -> libc::c_int {
    let mut ok_to_continue: bool;
    let step: String;
    let join_vg: bool;
    let own_fingerprint: String;
    let contact_chat_id: u32;
    let contact_chat_id_blocked: Blocked;
    let mut grpid = "".to_string();
    let mut ret: libc::c_int = 0i32;

    if !(contact_id <= DC_CONTACT_ID_LAST_SPECIAL) {
        step = lookup_field(mimeparser, "Secure-Join");
        if !step.is_empty() {
            info!(
                context,
                0, ">>>>>>>>>>>>>>>>>>>>>>>>> secure-join message \'{}\' received", step,
            );
            join_vg = step.starts_with("vg-");
            let (id, bl) = chat::create_or_lookup_by_contact_id(context, contact_id, Blocked::Not)
                .unwrap_or_default();
            contact_chat_id = id;
            contact_chat_id_blocked = bl;

            if Blocked::Not != contact_chat_id_blocked {
                chat::unblock(context, contact_chat_id);
            }
            ret = 0x2i32;
            if step == "vg-request" || step == "vc-request" {
                /* =========================================================
                ====             Alice - the inviter side            ====
                ====   Step 3 in "Setup verified contact" protocol   ====
                ========================================================= */
                // this message may be unencrypted (Bob, the joinder and the sender, might not have Alice's key yet)
                // it just ensures, we have Bobs key now. If we do _not_ have the key because eg. MitM has removed it,
                // send_message() will fail with the error "End-to-end-encryption unavailable unexpectedly.", so, there is no additional check needed here.
                // verify that the `Secure-Join-Invitenumber:`-header matches invitenumber written to the QR code
                let invitenumber = lookup_field(mimeparser, "Secure-Join-Invitenumber");
                if invitenumber == "" {
                    warn!(context, 0, "Secure-join denied (invitenumber missing).",);
                    ok_to_continue = false;
                } else if !dc_token_exists(context, DC_TOKEN_INVITENUMBER, &invitenumber) {
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
                        if join_vg {
                            "vg-auth-required"
                        } else {
                            "vc-auth-required"
                        },
                        "",
                        None,
                        "",
                    );
                    ok_to_continue = true;
                }
            } else if step == "vg-auth-required" || step == "vc-auth-required" {
                let cond = {
                    let bob = context.bob.read().unwrap();
                    let scan = bob.qr_scan.as_ref();
                    scan.is_none()
                        || bob.expects != 2
                        || join_vg && scan.unwrap().state != LotState::QrAskVerifyGroup
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
                    if join_vg {
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
                                "No valid signature."
                            } else {
                                "Not encrypted."
                            },
                        );
                        end_bobs_joining(context, 0i32);
                        ok_to_continue = false;
                    } else if !fingerprint_equals_sender(
                        context,
                        &scanned_fingerprint_of_alice,
                        contact_chat_id,
                    ) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            "Fingerprint mismatch on joiner-side.",
                        );
                        end_bobs_joining(context, 0i32);
                        ok_to_continue = false;
                    } else {
                        info!(context, 0, "Fingerprint verified.",);
                        own_fingerprint = get_self_fingerprint(context).unwrap();
                        context.call_cb(
                            Event::SECUREJOIN_JOINER_PROGRESS,
                            contact_id as uintptr_t,
                            400i32 as uintptr_t,
                        );
                        context.bob.write().unwrap().expects = 6;

                        send_handshake_msg(
                            context,
                            contact_chat_id,
                            if join_vg {
                                "vg-request-with-auth"
                            } else {
                                "vc-request-with-auth"
                            },
                            auth,
                            Some(own_fingerprint),
                            grpid,
                        );
                        ok_to_continue = true;
                    }
                }
            } else if step == "vg-request-with-auth" || step == "vc-request-with-auth" {
                /* ============================================================
                ====              Alice - the inviter side              ====
                ====   Steps 5+6 in "Setup verified contact" protocol   ====
                ====  Step 6 in "Out-of-band verified groups" protocol  ====
                ============================================================ */
                // verify that Secure-Join-Fingerprint:-header matches the fingerprint of Bob
                let fingerprint = lookup_field(mimeparser, "Secure-Join-Fingerprint");
                if fingerprint.is_empty() {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Fingerprint not provided.",
                    );
                    ok_to_continue = false;
                } else if !encrypted_and_signed(mimeparser, &fingerprint) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Auth not encrypted.",
                    );
                    ok_to_continue = false;
                } else if !fingerprint_equals_sender(context, &fingerprint, contact_chat_id) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Fingerprint mismatch on inviter-side.",
                    );
                    ok_to_continue = false;
                } else {
                    info!(context, 0, "Fingerprint verified.",);
                    // verify that the `Secure-Join-Auth:`-header matches the secret written to the QR code
                    let auth_0 = lookup_field(mimeparser, "Secure-Join-Auth");
                    if auth_0.is_empty() {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            "Auth not provided.",
                        );
                        ok_to_continue = false;
                    } else if !dc_token_exists(context, DC_TOKEN_AUTH, &auth_0) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            "Auth invalid.",
                        );
                        ok_to_continue = false;
                    } else if mark_peer_as_verified(context, fingerprint).is_err() {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            "Fingerprint mismatch on inviter-side.",
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
                        if join_vg {
                            grpid = lookup_field(mimeparser, "Secure-Join-Group");
                            let (group_chat_id, _, _) = chat::get_chat_id_by_grpid(context, &grpid);
                            if group_chat_id == 0 {
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
                                "vc-contact-confirm",
                                "",
                                None,
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
            } else if step == "vg-member-added" || step == "vc-contact-confirm" {
                if join_vg {
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
                            || join_vg && scan.unwrap().state != LotState::QrAskVerifyGroup
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

                        if join_vg {
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

                        let vg_expect_encrypted = if join_vg {
                            let (_, is_verified_group, _) =
                                chat::get_chat_id_by_grpid(context, grpid);
                            // when joining a non-verified group
                            // the vg-member-added message may be unencrypted
                            // when not all group members have keys or prefer encryption.
                            // So only expect encryption if this is a verified group 
                            is_verified_group
                        } else {
                            true
                        };
                        if vg_expect_encrypted {
                            if !encrypted_and_signed(mimeparser, &scanned_fingerprint_of_alice) {
                                could_not_establish_secure_connection(
                                    context,
                                    contact_chat_id,
                                    "Contact confirm message not encrypted.",
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
                            if mark_peer_as_verified(context, &scanned_fingerprint_of_alice)
                                .is_err()
                            {
                                could_not_establish_secure_connection(
                                    context,
                                    contact_chat_id,
                                    "Fingerprint mismatch on joiner-side.",
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
                                if join_vg {
                                    if !addr_equals_self(
                                        context,
                                        lookup_field(mimeparser, "Chat-Group-Member-Added"),
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
                                    if join_vg {
                                        send_handshake_msg(
                                            context,
                                            contact_chat_id,
                                            "vg-member-added-received",
                                            "",
                                            None,
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
            } else if step == "vg-member-added-received" {
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

unsafe fn lookup_field(mimeparser: &dc_mimeparser_t, key: &str) -> String {
    let field: *mut mailimf_field = dc_mimeparser_lookup_field(mimeparser, key);
    let mut value: *const libc::c_char = ptr::null();
    if field.is_null()
        || (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
        || (*field).fld_data.fld_optional_field.is_null()
        || {
            value = (*(*field).fld_data.fld_optional_field).fld_value;
            value.is_null()
        }
    {
        return String::from("");
    }
    as_str(value).to_string()
}

fn could_not_establish_secure_connection(
    context: &Context,
    contact_chat_id: uint32_t,
    details: &str,
) {
    let contact_id = chat_id_2_contact_id(context, contact_chat_id);
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
    error!(context, 0, "{} ({})", &msg, details);
}

fn mark_peer_as_verified(context: &Context, fingerprint: impl AsRef<str>) -> Result<(), Error> {
    if let Some(ref mut peerstate) =
        Peerstate::from_fingerprint(context, &context.sql, fingerprint.as_ref())
    {
        if peerstate.set_verified(1, fingerprint.as_ref(), 2) {
            peerstate.prefer_encrypt = EncryptPreference::Mutual;
            peerstate.to_save = Some(ToSave::All);
            peerstate.save_to_db(&context.sql, false);
            return Ok(());
        }
    }
    bail!(
        "could not mark peer as verified for fingerprint {}",
        fingerprint.as_ref()
    );
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

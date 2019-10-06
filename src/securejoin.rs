use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::aheader::EncryptPreference;
use crate::chat::{self, Chat};
use crate::config::*;
use crate::constants::*;
use crate::contact::*;
use crate::context::Context;
use crate::dc_mimeparser::*;
use crate::e2ee::*;
use crate::error::Error;
use crate::events::Event;
use crate::key::*;
use crate::lot::LotState;
use crate::message::Message;
use crate::param::*;
use crate::peerstate::*;
use crate::qr::check_qr;
use crate::stock::StockMessage;
use crate::token;

pub const NON_ALPHANUMERIC_WITHOUT_DOT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'.');

macro_rules! joiner_progress {
    ($context:tt, $contact_id:expr, $progress:expr) => {
        assert!(
            $progress >= 0 && $progress <= 1000,
            "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
        );
        $context.call_cb($crate::events::Event::SecurejoinJoinerProgress {
            contact_id: $contact_id,
            progress: $progress,
        });
    };
}

macro_rules! inviter_progress {
    ($context:tt, $contact_id:expr, $progress:expr) => {
        assert!(
            $progress >= 0 && $progress <= 1000,
            "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
        );
        $context.call_cb($crate::events::Event::SecurejoinInviterProgress {
            contact_id: $contact_id,
            progress: $progress,
        });
    };
}

macro_rules! get_qr_attr {
    ($context:tt, $attr:ident) => {
        $context
            .bob
            .read()
            .unwrap()
            .qr_scan
            .as_ref()
            .unwrap()
            .$attr
            .as_ref()
            .unwrap()
    };
}

pub fn dc_get_securejoin_qr(context: &Context, group_chat_id: u32) -> Option<String> {
    /* =========================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    ========================================================= */

    let fingerprint: String;

    ensure_secret_key_exists(context).ok();
    let invitenumber = token::lookup_or_new(context, token::Namespace::InviteNumber, group_chat_id);
    let auth = token::lookup_or_new(context, token::Namespace::Auth, group_chat_id);
    let self_addr = match context.get_config(Config::ConfiguredAddr) {
        Some(addr) => addr,
        None => {
            error!(context, "Not configured, cannot generate QR code.",);
            return None;
        }
    };

    let self_name = context.get_config(Config::Displayname).unwrap_or_default();

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
            error!(context, "Cannot get QR-code for chat-id {}", group_chat_id,);
            return None;
        }
    } else {
        Some(format!(
            "OPENPGP4FPR:{}#a={}&n={}&i={}&s={}",
            fingerprint, self_addr_urlencoded, self_name_urlencoded, &invitenumber, &auth,
        ))
    };

    info!(context, "Generated QR code: {}", qr.as_ref().unwrap());

    qr
}

fn get_self_fingerprint(context: &Context) -> Option<String> {
    if let Some(self_addr) = context.get_config(Config::ConfiguredAddr) {
        if let Some(key) = Key::from_self_public(context, self_addr, &context.sql) {
            return Some(key.fingerprint());
        }
    }
    None
}

pub fn dc_join_securejoin(context: &Context, qr: &str) -> u32 {
    let cleanup =
        |context: &Context, contact_chat_id: u32, ongoing_allocated: bool, join_vg: bool| {
            let mut bob = context.bob.write().unwrap();
            bob.expects = 0;
            let ret_chat_id = if bob.status == DC_BOB_SUCCESS {
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

            if ongoing_allocated {
                context.free_ongoing();
            }
            ret_chat_id as u32
        };
    /* ==========================================================
    ====             Bob - the joiner's side             =====
    ====   Step 2 in "Setup verified contact" protocol   =====
    ========================================================== */
    let mut contact_chat_id: u32 = 0;
    let mut join_vg: bool = false;

    info!(context, "Requesting secure-join ...",);
    ensure_secret_key_exists(context).ok();
    if !context.alloc_ongoing() {
        return cleanup(&context, contact_chat_id, false, join_vg);
    }
    let qr_scan = check_qr(context, &qr);
    if qr_scan.state != LotState::QrAskVerifyContact && qr_scan.state != LotState::QrAskVerifyGroup
    {
        error!(context, "Unknown QR code.",);
        return cleanup(&context, contact_chat_id, true, join_vg);
    }
    contact_chat_id = chat::create_by_contact_id(context, qr_scan.id).unwrap_or_default();
    if contact_chat_id == 0 {
        error!(context, "Unknown contact.",);
        return cleanup(&context, contact_chat_id, true, join_vg);
    }
    if context.shall_stop_ongoing() {
        return cleanup(&context, contact_chat_id, true, join_vg);
    }
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
        info!(context, "Taking protocol shortcut.");
        context.bob.write().unwrap().expects = DC_VC_CONTACT_CONFIRM;
        joiner_progress!(context, chat_id_2_contact_id(context, contact_chat_id), 400);
        let own_fingerprint = get_self_fingerprint(context).unwrap_or_default();
        send_handshake_msg(
            context,
            contact_chat_id,
            if join_vg {
                "vg-request-with-auth"
            } else {
                "vc-request-with-auth"
            },
            get_qr_attr!(context, auth).to_string(),
            Some(own_fingerprint),
            if join_vg {
                get_qr_attr!(context, text2).to_string()
            } else {
                "".to_string()
            },
        );
    } else {
        context.bob.write().unwrap().expects = DC_VC_AUTH_REQUIRED;
        send_handshake_msg(
            context,
            contact_chat_id,
            if join_vg { "vg-request" } else { "vc-request" },
            get_qr_attr!(context, invitenumber),
            None,
            "",
        );
    }

    // Bob -> Alice
    while !context.shall_stop_ongoing() {
        std::thread::sleep(std::time::Duration::new(0, 3_000_000));
    }
    cleanup(&context, contact_chat_id, true, join_vg)
}

fn send_handshake_msg(
    context: &Context,
    contact_chat_id: u32,
    step: &str,
    param2: impl AsRef<str>,
    fingerprint: Option<String>,
    grpid: impl AsRef<str>,
) {
    let mut msg = Message::default();
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
    chat::send_msg(context, contact_chat_id, &mut msg).unwrap_or_default();
}

fn chat_id_2_contact_id(context: &Context, contact_chat_id: u32) -> u32 {
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
pub fn handle_securejoin_handshake(
    context: &Context,
    mimeparser: &MimeParser,
    contact_id: u32,
) -> libc::c_int {
    let own_fingerprint: String;

    if contact_id <= DC_CONTACT_ID_LAST_SPECIAL {
        return 0;
    }
    let step = match mimeparser.lookup_optional_field("Secure-Join") {
        Some(s) => s,
        None => {
            return 0;
        }
    };
    info!(
        context,
        ">>>>>>>>>>>>>>>>>>>>>>>>> secure-join message \'{}\' received", step,
    );
    let (contact_chat_id, contact_chat_id_blocked) =
        chat::create_or_lookup_by_contact_id(context, contact_id, Blocked::Not).unwrap_or_default();

    if contact_chat_id_blocked != Blocked::Not {
        chat::unblock(context, contact_chat_id);
    }
    let mut ret: libc::c_int = DC_HANDSHAKE_STOP_NORMAL_PROCESSING;
    let join_vg = step.starts_with("vg-");

    match step.as_str() {
        "vg-request" | "vc-request" => {
            /* =========================================================
            ====             Alice - the inviter side            ====
            ====   Step 3 in "Setup verified contact" protocol   ====
            ========================================================= */
            // this message may be unencrypted (Bob, the joinder and the sender, might not have Alice's key yet)
            // it just ensures, we have Bobs key now. If we do _not_ have the key because eg. MitM has removed it,
            // send_message() will fail with the error "End-to-end-encryption unavailable unexpectedly.", so, there is no additional check needed here.
            // verify that the `Secure-Join-Invitenumber:`-header matches invitenumber written to the QR code
            let invitenumber = match mimeparser.lookup_optional_field("Secure-Join-Invitenumber") {
                Some(n) => n,
                None => {
                    warn!(context, "Secure-join denied (invitenumber missing).",);
                    return ret;
                }
            };
            if !token::exists(context, token::Namespace::InviteNumber, &invitenumber) {
                warn!(context, "Secure-join denied (bad invitenumber).",);
                return ret;
            }
            info!(context, "Secure-join requested.",);

            inviter_progress!(context, contact_id, 300);
            send_handshake_msg(
                context,
                contact_chat_id,
                &format!("{}-auth-required", &step[..2]),
                "",
                None,
                "",
            );
        }
        "vg-auth-required" | "vc-auth-required" => {
            let cond = {
                let bob = context.bob.read().unwrap();
                let scan = bob.qr_scan.as_ref();
                scan.is_none()
                    || bob.expects != DC_VC_AUTH_REQUIRED
                    || join_vg && scan.unwrap().state != LotState::QrAskVerifyGroup
            };

            if cond {
                warn!(context, "auth-required message out of sync.",);
                // no error, just aborted somehow or a mail from another handshake
                return ret;
            }
            let scanned_fingerprint_of_alice = get_qr_attr!(context, fingerprint).to_string();
            let auth = get_qr_attr!(context, auth).to_string();

            if !encrypted_and_signed(mimeparser, &scanned_fingerprint_of_alice) {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    if mimeparser.encrypted {
                        "No valid signature."
                    } else {
                        "Not encrypted."
                    },
                );
                end_bobs_joining(context, DC_BOB_ERROR);
                return ret;
            }
            if !fingerprint_equals_sender(context, &scanned_fingerprint_of_alice, contact_chat_id) {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on joiner-side.",
                );
                end_bobs_joining(context, DC_BOB_ERROR);
                return ret;
            }
            info!(context, "Fingerprint verified.",);
            own_fingerprint = get_self_fingerprint(context).unwrap();
            joiner_progress!(context, contact_id, 400);
            context.bob.write().unwrap().expects = DC_VC_CONTACT_CONFIRM;

            send_handshake_msg(
                context,
                contact_chat_id,
                &format!("{}-request-with-auth", &step[..2]),
                auth,
                Some(own_fingerprint),
                if join_vg {
                    get_qr_attr!(context, text2).to_string()
                } else {
                    "".to_string()
                },
            );
        }
        "vg-request-with-auth" | "vc-request-with-auth" => {
            /* ============================================================
            ====              Alice - the inviter side              ====
            ====   Steps 5+6 in "Setup verified contact" protocol   ====
            ====  Step 6 in "Out-of-band verified groups" protocol  ====
            ============================================================ */
            // verify that Secure-Join-Fingerprint:-header matches the fingerprint of Bob
            let fingerprint = match mimeparser.lookup_optional_field("Secure-Join-Fingerprint") {
                Some(fp) => fp,
                None => {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Fingerprint not provided.",
                    );
                    return ret;
                }
            };
            if !encrypted_and_signed(mimeparser, &fingerprint) {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Auth not encrypted.",
                );
                return ret;
            }
            if !fingerprint_equals_sender(context, &fingerprint, contact_chat_id) {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on inviter-side.",
                );
                return ret;
            }
            info!(context, "Fingerprint verified.",);
            // verify that the `Secure-Join-Auth:`-header matches the secret written to the QR code
            let auth_0 = match mimeparser.lookup_optional_field("Secure-Join-Auth") {
                Some(auth) => auth,
                None => {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Auth not provided.",
                    );
                    return ret;
                }
            };
            if !token::exists(context, token::Namespace::Auth, &auth_0) {
                could_not_establish_secure_connection(context, contact_chat_id, "Auth invalid.");
                return ret;
            }
            if mark_peer_as_verified(context, fingerprint).is_err() {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on inviter-side.",
                );
                return ret;
            }
            Contact::scaleup_origin_by_id(context, contact_id, Origin::SecurejoinInvited);
            info!(context, "Auth verified.",);
            secure_connection_established(context, contact_chat_id);
            emit_event!(context, Event::ContactsChanged(Some(contact_id)));
            inviter_progress!(context, contact_id, 600);
            if join_vg {
                let field_grpid = mimeparser
                    .lookup_optional_field("Secure-Join-Group")
                    .unwrap_or_default();
                let (group_chat_id, _, _) = chat::get_chat_id_by_grpid(context, &field_grpid);
                if group_chat_id == 0 {
                    error!(context, "Chat {} not found.", &field_grpid);
                    return ret;
                } else {
                    if let Err(err) =
                        chat::add_contact_to_chat_ex(context, group_chat_id, contact_id, true)
                    {
                        error!(context, "failed to add contact: {}", err);
                    }
                }
            } else {
                send_handshake_msg(context, contact_chat_id, "vc-contact-confirm", "", None, "");
                inviter_progress!(context, contact_id, 1000);
            }
        }
        "vg-member-added" | "vc-contact-confirm" => {
            if join_vg {
                ret = DC_HANDSHAKE_CONTINUE_NORMAL_PROCESSING;
            }
            if context.bob.read().unwrap().expects != DC_VC_CONTACT_CONFIRM {
                info!(context, "Message belongs to a different handshake.",);
                return ret;
            }
            let cond = {
                let bob = context.bob.read().unwrap();
                let scan = bob.qr_scan.as_ref();
                scan.is_none() || join_vg && scan.unwrap().state != LotState::QrAskVerifyGroup
            };
            if cond {
                warn!(
                    context,
                    "Message out of sync or belongs to a different handshake.",
                );
                return ret;
            }
            let scanned_fingerprint_of_alice = get_qr_attr!(context, fingerprint).to_string();

            let vg_expect_encrypted = if join_vg {
                let group_id = get_qr_attr!(context, text2).to_string();
                let (_, is_verified_group, _) = chat::get_chat_id_by_grpid(context, group_id);
                // when joining a non-verified group
                // the vg-member-added message may be unencrypted
                // when not all group members have keys or prefer encryption.
                // So only expect encryption if this is a verified group
                is_verified_group
            } else {
                // setup contact is always encrypted
                true
            };
            if vg_expect_encrypted
                && !encrypted_and_signed(mimeparser, &scanned_fingerprint_of_alice)
            {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Contact confirm message not encrypted.",
                );
                end_bobs_joining(context, DC_BOB_ERROR);
                return ret;
            }

            if mark_peer_as_verified(context, &scanned_fingerprint_of_alice).is_err() {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on joiner-side.",
                );
                return ret;
            }
            Contact::scaleup_origin_by_id(context, contact_id, Origin::SecurejoinJoined);
            emit_event!(context, Event::ContactsChanged(None));
            let cg_member_added = mimeparser
                .lookup_optional_field("Chat-Group-Member-Added")
                .unwrap_or_default();
            if join_vg && !addr_equals_self(context, cg_member_added) {
                info!(context, "Message belongs to a different handshake (scaled up contact anyway to allow creation of group).");
                return ret;
            }
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
            end_bobs_joining(context, DC_BOB_SUCCESS);
        }
        "vg-member-added-received" => {
            /* ============================================================
            ====              Alice - the inviter side              ====
            ====  Step 8 in "Out-of-band verified groups" protocol  ====
            ============================================================ */
            if let Ok(contact) = Contact::get_by_id(context, contact_id) {
                if contact.is_verified(context) == VerifiedStatus::Unverified {
                    warn!(context, "vg-member-added-received invalid.",);
                    return ret;
                }
                inviter_progress!(context, contact_id, 800);
                inviter_progress!(context, contact_id, 1000);
            } else {
                warn!(context, "vg-member-added-received invalid.",);
                return ret;
            }
        }
        _ => {
            warn!(context, "invalid step: {}", step);
        }
    }
    if ret == DC_HANDSHAKE_STOP_NORMAL_PROCESSING {
        ret |= DC_HANDSHAKE_ADD_DELETE_JOB;
    }
    ret
}

fn end_bobs_joining(context: &Context, status: libc::c_int) {
    context.bob.write().unwrap().status = status;
    context.stop_ongoing();
}

fn secure_connection_established(context: &Context, contact_chat_id: u32) {
    let contact_id: u32 = chat_id_2_contact_id(context, contact_chat_id);
    let contact = Contact::get_by_id(context, contact_id);
    let addr = if let Ok(ref contact) = contact {
        contact.get_addr()
    } else {
        "?"
    };
    let msg = context.stock_string_repl_str(StockMessage::ContactVerified, addr);
    chat::add_device_msg(context, contact_chat_id, msg);
    emit_event!(context, Event::ChatModified(contact_chat_id));
}

fn could_not_establish_secure_connection(context: &Context, contact_chat_id: u32, details: &str) {
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
    error!(context, "{} ({})", &msg, details);
}

fn mark_peer_as_verified(context: &Context, fingerprint: impl AsRef<str>) -> Result<(), Error> {
    if let Some(ref mut peerstate) =
        Peerstate::from_fingerprint(context, &context.sql, fingerprint.as_ref())
    {
        if peerstate.set_verified(1, fingerprint.as_ref(), 2) {
            peerstate.prefer_encrypt = EncryptPreference::Mutual;
            peerstate.to_save = Some(ToSave::All);
            peerstate
                .save_to_db(&context.sql, false)
                .unwrap_or_default();
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

fn encrypted_and_signed(mimeparser: &MimeParser, expected_fingerprint: impl AsRef<str>) -> bool {
    if !mimeparser.encrypted {
        warn!(mimeparser.context, "Message not encrypted.",);
        false
    } else if mimeparser.signatures.is_empty() {
        warn!(mimeparser.context, "Message not signed.",);
        false
    } else if expected_fingerprint.as_ref().is_empty() {
        warn!(mimeparser.context, "Fingerprint for comparison missing.",);
        false
    } else if !mimeparser
        .signatures
        .contains(expected_fingerprint.as_ref())
    {
        warn!(
            mimeparser.context,
            "Message does not match expected fingerprint {}.",
            expected_fingerprint.as_ref(),
        );
        false
    } else {
        true
    }
}

pub fn handle_degrade_event(context: &Context, peerstate: &Peerstate) -> Result<(), Error> {
    // - we do not issue an warning for DC_DE_ENCRYPTION_PAUSED as this is quite normal
    // - currently, we do not issue an extra warning for DC_DE_VERIFICATION_LOST - this always comes
    //   together with DC_DE_FINGERPRINT_CHANGED which is logged, the idea is not to bother
    //   with things they cannot fix, so the user is just kicked from the verified group
    //   (and he will know this and can fix this)
    if Some(DegradeEvent::FingerprintChanged) == peerstate.degrade_event {
        let contact_id: i32 = match context.sql.query_get_value(
            context,
            "SELECT id FROM contacts WHERE addr=?;",
            params![&peerstate.addr],
        ) {
            None => bail!(
                "contact with peerstate.addr {:?} not found",
                &peerstate.addr
            ),
            Some(contact_id) => contact_id,
        };
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
            emit_event!(context, Event::ChatModified(contact_chat_id));
        }
    }
    Ok(())
}

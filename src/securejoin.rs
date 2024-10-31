//! Implementation of [SecureJoin protocols](https://securejoin.delta.chat/).

use anyhow::{ensure, Context as _, Error, Result};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::aheader::EncryptPreference;
use crate::chat::{self, get_chat_id_by_grpid, Chat, ChatId, ChatIdBlocked, ProtectionStatus};
use crate::chatlist_events;
use crate::config::Config;
use crate::constants::{Blocked, Chattype, NON_ALPHANUMERIC_WITHOUT_DOT};
use crate::contact::{Contact, ContactId, Origin};
use crate::context::Context;
use crate::e2ee::ensure_secret_key_exists;
use crate::events::EventType;
use crate::headerdef::HeaderDef;
use crate::key::{load_self_public_key, DcKey, Fingerprint};
use crate::message::{Message, Viewtype};
use crate::mimeparser::{MimeMessage, SystemMessage};
use crate::param::Param;
use crate::peerstate::Peerstate;
use crate::qr::check_qr;
use crate::securejoin::bob::JoinerProgress;
use crate::stock_str;
use crate::sync::Sync::*;
use crate::token;
use crate::tools::time;

mod bob;
mod bobstate;
mod qrinvite;

pub(crate) use bobstate::BobState;
use qrinvite::QrInvite;

use crate::token::Namespace;

fn inviter_progress(context: &Context, contact_id: ContactId, progress: usize) {
    debug_assert!(
        progress <= 1000,
        "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
    );
    context.emit_event(EventType::SecurejoinInviterProgress {
        contact_id,
        progress,
    });
}

/// Generates a Secure Join QR code.
///
/// With `group` set to `None` this generates a setup-contact QR code, with `group` set to a
/// [`ChatId`] generates a join-group QR code for the given chat.
pub async fn get_securejoin_qr(context: &Context, group: Option<ChatId>) -> Result<String> {
    /*=======================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    =======================================================*/

    ensure_secret_key_exists(context).await.ok();

    let chat = match group {
        Some(id) => {
            let chat = Chat::load_from_db(context, id).await?;
            ensure!(
                chat.typ == Chattype::Group,
                "Can't generate SecureJoin QR code for 1:1 chat {id}"
            );
            ensure!(
                !chat.grpid.is_empty(),
                "Can't generate SecureJoin QR code for ad-hoc group {id}"
            );
            Some(chat)
        }
        None => None,
    };
    let grpid = chat.as_ref().map(|c| c.grpid.as_str());
    let sync_token = token::lookup(context, Namespace::InviteNumber, grpid)
        .await?
        .is_none();
    // invitenumber will be used to allow starting the handshake,
    // auth will be used to verify the fingerprint
    let invitenumber = token::lookup_or_new(context, Namespace::InviteNumber, grpid).await?;
    let auth = token::lookup_or_new(context, Namespace::Auth, grpid).await?;
    let self_addr = context.get_primary_self_addr().await?;
    let self_name = context
        .get_config(Config::Displayname)
        .await?
        .unwrap_or_default();

    let fingerprint = get_self_fingerprint(context).await?;

    let self_addr_urlencoded =
        utf8_percent_encode(&self_addr, NON_ALPHANUMERIC_WITHOUT_DOT).to_string();
    let self_name_urlencoded =
        utf8_percent_encode(&self_name, NON_ALPHANUMERIC_WITHOUT_DOT).to_string();

    let qr = if let Some(chat) = chat {
        // parameters used: a=g=x=i=s=
        let group_name = chat.get_name();
        let group_name_urlencoded = utf8_percent_encode(group_name, NON_ALPHANUMERIC).to_string();
        if sync_token {
            context
                .sync_qr_code_tokens(Some(chat.grpid.as_str()))
                .await?;
            context.scheduler.interrupt_inbox().await;
        }
        format!(
            "OPENPGP4FPR:{}#a={}&g={}&x={}&i={}&s={}",
            fingerprint.hex(),
            self_addr_urlencoded,
            &group_name_urlencoded,
            &chat.grpid,
            &invitenumber,
            &auth,
        )
    } else {
        // parameters used: a=n=i=s=
        if sync_token {
            context.sync_qr_code_tokens(None).await?;
            context.scheduler.interrupt_inbox().await;
        }
        format!(
            "OPENPGP4FPR:{}#a={}&n={}&i={}&s={}",
            fingerprint.hex(),
            self_addr_urlencoded,
            self_name_urlencoded,
            &invitenumber,
            &auth,
        )
    };

    info!(context, "Generated QR code.");
    Ok(qr)
}

async fn get_self_fingerprint(context: &Context) -> Result<Fingerprint> {
    let key = load_self_public_key(context)
        .await
        .context("Failed to load key")?;
    Ok(key.fingerprint())
}

/// Take a scanned QR-code and do the setup-contact/join-group/invite handshake.
///
/// This is the start of the process for the joiner.  See the module and ffi documentation
/// for more details.
///
/// The function returns immediately and the handshake will run in background.
pub async fn join_securejoin(context: &Context, qr: &str) -> Result<ChatId> {
    securejoin(context, qr).await.map_err(|err| {
        warn!(context, "Fatal joiner error: {:#}", err);
        // The user just scanned this QR code so has context on what failed.
        error!(context, "QR process failed");
        err
    })
}

async fn securejoin(context: &Context, qr: &str) -> Result<ChatId> {
    /*========================================================
    ====             Bob - the joiner's side             =====
    ====   Step 2 in "Setup verified contact" protocol   =====
    ========================================================*/

    info!(context, "Requesting secure-join ...",);
    let qr_scan = check_qr(context, qr).await?;

    let invite = QrInvite::try_from(qr_scan)?;

    bob::start_protocol(context, invite).await
}

/// Send handshake message from Alice's device;
/// Bob's handshake messages are sent in `BobState::send_handshake_message()`.
async fn send_alice_handshake_msg(
    context: &Context,
    contact_id: ContactId,
    step: &str,
) -> Result<()> {
    let mut msg = Message {
        viewtype: Viewtype::Text,
        text: format!("Secure-Join: {step}"),
        hidden: true,
        ..Default::default()
    };
    msg.param.set_cmd(SystemMessage::SecurejoinMessage);
    msg.param.set(Param::Arg, step);
    msg.param.set_int(Param::GuaranteeE2ee, 1);
    chat::send_msg(
        context,
        ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Yes)
            .await?
            .id,
        &mut msg,
    )
    .await?;
    Ok(())
}

/// Get an unblocked chat that can be used for info messages.
async fn info_chat_id(context: &Context, contact_id: ContactId) -> Result<ChatId> {
    let chat_id_blocked = ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Not).await?;
    Ok(chat_id_blocked.id)
}

/// Checks fingerprint and marks the contact as forward verified
/// if fingerprint matches.
async fn verify_sender_by_fingerprint(
    context: &Context,
    fingerprint: &Fingerprint,
    contact_id: ContactId,
) -> Result<bool> {
    let contact = Contact::get_by_id(context, contact_id).await?;
    let peerstate = match Peerstate::from_addr(context, contact.get_addr()).await {
        Ok(peerstate) => peerstate,
        Err(err) => {
            warn!(
                context,
                "Failed to sender peerstate for {}: {}",
                contact.get_addr(),
                err
            );
            return Ok(false);
        }
    };

    if let Some(mut peerstate) = peerstate {
        if peerstate
            .public_key_fingerprint
            .as_ref()
            .filter(|&fp| fp == fingerprint)
            .is_some()
        {
            if let Some(public_key) = &peerstate.public_key {
                let verifier = contact.get_addr().to_owned();
                peerstate.set_verified(public_key.clone(), fingerprint.clone(), verifier)?;
                peerstate.prefer_encrypt = EncryptPreference::Mutual;
                peerstate.save_to_db(&context.sql).await?;
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// What to do with a Secure-Join handshake message after it was handled.
///
/// This status is returned to [`receive_imf`] which will use it to decide what to do
/// next with this incoming setup-contact/secure-join handshake message.
///
/// [`receive_imf`]: crate::receive_imf::receive_imf
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HandshakeMessage {
    /// The message has been fully handled and should be removed/delete.
    ///
    /// This removes the message both locally and on the IMAP server.
    Done,
    /// The message should be ignored/hidden, but not removed/deleted.
    ///
    /// This leaves it on the IMAP server.  It means other devices on this account can
    /// receive and potentially process this message as well.  This is useful for example
    /// when the other device is running the protocol and has the relevant QR-code
    /// information while this device does not have the joiner state ([`BobState`]).
    Ignore,
    /// The message should be further processed by incoming message handling.
    ///
    /// This may for example result in a group being created if it is a message which added
    /// us to a group (a `vg-member-added` message).
    Propagate,
}

/// Handle incoming secure-join handshake.
///
/// This function will update the securejoin state in the database as the protocol
/// progresses.
///
/// A message which results in [`Err`] will be hidden from the user but not deleted, it may
/// be a valid message for something else we are not aware off.  E.g. it could be part of a
/// handshake performed by another DC app on the same account.
///
/// When `handle_securejoin_handshake()` is called, the message is not yet filed in the
/// database; this is done by `receive_imf()` later on as needed.
#[allow(clippy::indexing_slicing)]
pub(crate) async fn handle_securejoin_handshake(
    context: &Context,
    mime_message: &MimeMessage,
    contact_id: ContactId,
) -> Result<HandshakeMessage> {
    if contact_id.is_special() {
        return Err(Error::msg("Can not be called with special contact ID"));
    }
    let step = mime_message
        .get_header(HeaderDef::SecureJoin)
        .context("Not a Secure-Join message")?;

    info!(context, "Received secure-join message {step:?}.");

    let join_vg = step.starts_with("vg-");

    if !matches!(step, "vg-request" | "vc-request") {
        let mut self_found = false;
        let self_fingerprint = load_self_public_key(context).await?.fingerprint();
        for (addr, key) in &mime_message.gossiped_keys {
            if key.fingerprint() == self_fingerprint && context.is_self_addr(addr).await? {
                self_found = true;
                break;
            }
        }
        if !self_found {
            // This message isn't intended for us. Possibly the peer doesn't own the key which the
            // message is signed with but forwarded someone's message to us.
            warn!(context, "Step {step}: No self addr+pubkey gossip found.");
            return Ok(HandshakeMessage::Ignore);
        }
    }

    match step {
        "vg-request" | "vc-request" => {
            /*=======================================================
            ====             Alice - the inviter side            ====
            ====   Step 3 in "Setup verified contact" protocol   ====
            =======================================================*/

            // this message may be unencrypted (Bob, the joiner and the sender, might not have Alice's key yet)
            // it just ensures, we have Bobs key now. If we do _not_ have the key because eg. MitM has removed it,
            // send_message() will fail with the error "End-to-end-encryption unavailable unexpectedly.", so, there is no additional check needed here.
            // verify that the `Secure-Join-Invitenumber:`-header matches invitenumber written to the QR code
            let invitenumber = match mime_message.get_header(HeaderDef::SecureJoinInvitenumber) {
                Some(n) => n,
                None => {
                    warn!(context, "Secure-join denied (invitenumber missing)");
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if !token::exists(context, token::Namespace::InviteNumber, invitenumber).await? {
                warn!(context, "Secure-join denied (bad invitenumber).");
                return Ok(HandshakeMessage::Ignore);
            }

            inviter_progress(context, contact_id, 300);

            // for setup-contact, make Alice's one-to-one chat with Bob visible
            // (secure-join-information are shown in the group chat)
            if !join_vg {
                ChatId::create_for_contact(context, contact_id).await?;
            }

            // Alice -> Bob
            send_alice_handshake_msg(
                context,
                contact_id,
                &format!("{}-auth-required", &step[..2]),
            )
            .await
            .context("failed sending auth-required handshake message")?;
            Ok(HandshakeMessage::Done)
        }
        "vg-auth-required" | "vc-auth-required" => {
            /*========================================================
            ====             Bob - the joiner's side             =====
            ====   Step 4 in "Setup verified contact" protocol   =====
            ========================================================*/
            bob::handle_auth_required(context, mime_message).await
        }
        "vg-request-with-auth" | "vc-request-with-auth" => {
            /*==========================================================
            ====              Alice - the inviter side              ====
            ====   Steps 5+6 in "Setup verified contact" protocol   ====
            ====  Step 6 in "Out-of-band verified groups" protocol  ====
            ==========================================================*/

            // verify that Secure-Join-Fingerprint:-header matches the fingerprint of Bob
            let Some(fp) = mime_message.get_header(HeaderDef::SecureJoinFingerprint) else {
                warn!(
                    context,
                    "Ignoring {step} message because fingerprint is not provided."
                );
                return Ok(HandshakeMessage::Ignore);
            };
            let fingerprint: Fingerprint = fp.parse()?;
            if !encrypted_and_signed(context, mime_message, &fingerprint) {
                warn!(
                    context,
                    "Ignoring {step} message because the message is not encrypted."
                );
                return Ok(HandshakeMessage::Ignore);
            }
            if !verify_sender_by_fingerprint(context, &fingerprint, contact_id).await? {
                warn!(
                    context,
                    "Ignoring {step} message because of fingerprint mismatch."
                );
                return Ok(HandshakeMessage::Ignore);
            }
            info!(context, "Fingerprint verified.",);
            // verify that the `Secure-Join-Auth:`-header matches the secret written to the QR code
            let Some(auth) = mime_message.get_header(HeaderDef::SecureJoinAuth) else {
                warn!(
                    context,
                    "Ignoring {step} message because of missing auth code."
                );
                return Ok(HandshakeMessage::Ignore);
            };
            let Some(grpid) = token::auth_foreign_key(context, auth).await? else {
                warn!(
                    context,
                    "Ignoring {step} message because of invalid auth code."
                );
                return Ok(HandshakeMessage::Ignore);
            };
            let group_chat_id = match grpid.as_str() {
                "" => None,
                id => {
                    let Some((chat_id, ..)) = get_chat_id_by_grpid(context, id).await? else {
                        warn!(context, "Ignoring {step} message: unknown grpid {id}.",);
                        return Ok(HandshakeMessage::Ignore);
                    };
                    Some(chat_id)
                }
            };

            let contact_addr = Contact::get_by_id(context, contact_id)
                .await?
                .get_addr()
                .to_owned();
            let backward_verified = true;
            let fingerprint_found = mark_peer_as_verified(
                context,
                fingerprint.clone(),
                contact_addr,
                backward_verified,
            )
            .await?;
            if !fingerprint_found {
                warn!(
                    context,
                    "Ignoring {step} message because of the failure to find matching peerstate."
                );
                return Ok(HandshakeMessage::Ignore);
            }
            contact_id.regossip_keys(context).await?;
            ContactId::scaleup_origin(context, &[contact_id], Origin::SecurejoinInvited).await?;
            info!(context, "Auth verified.",);
            context.emit_event(EventType::ContactsChanged(Some(contact_id)));
            inviter_progress(context, contact_id, 600);
            if let Some(group_chat_id) = group_chat_id {
                // Join group.
                secure_connection_established(
                    context,
                    contact_id,
                    group_chat_id,
                    mime_message.timestamp_sent,
                )
                .await?;
                chat::add_contact_to_chat_ex(context, Nosync, group_chat_id, contact_id, true)
                    .await?;
                inviter_progress(context, contact_id, 800);
                inviter_progress(context, contact_id, 1000);
            } else {
                // Setup verified contact.
                secure_connection_established(
                    context,
                    contact_id,
                    info_chat_id(context, contact_id).await?,
                    mime_message.timestamp_sent,
                )
                .await?;
                send_alice_handshake_msg(context, contact_id, "vc-contact-confirm")
                    .await
                    .context("failed sending vc-contact-confirm message")?;

                inviter_progress(context, contact_id, 1000);
            }
            Ok(HandshakeMessage::Ignore) // "Done" would delete the message and break multi-device (the key from Autocrypt-header is needed)
        }
        /*=======================================================
        ====             Bob - the joiner's side             ====
        ====   Step 7 in "Setup verified contact" protocol   ====
        =======================================================*/
        "vc-contact-confirm" => {
            if let Some(mut bobstate) = BobState::from_db(&context.sql).await? {
                if !bobstate.is_msg_expected(context, step) {
                    warn!(context, "Unexpected vc-contact-confirm.");
                    return Ok(HandshakeMessage::Ignore);
                }

                bobstate.step_contact_confirm(context).await?;
                bobstate.emit_progress(context, JoinerProgress::Succeeded);
            }
            Ok(HandshakeMessage::Ignore)
        }
        "vg-member-added" => {
            let Some(member_added) = mime_message.get_header(HeaderDef::ChatGroupMemberAdded)
            else {
                warn!(
                    context,
                    "vg-member-added without Chat-Group-Member-Added header."
                );
                return Ok(HandshakeMessage::Propagate);
            };
            if !context.is_self_addr(member_added).await? {
                info!(
                    context,
                    "Member {member_added} added by unrelated SecureJoin process."
                );
                return Ok(HandshakeMessage::Propagate);
            }
            if let Some(mut bobstate) = BobState::from_db(&context.sql).await? {
                if !bobstate.is_msg_expected(context, step) {
                    warn!(context, "Unexpected vg-member-added.");
                    return Ok(HandshakeMessage::Propagate);
                }

                bobstate.step_contact_confirm(context).await?;
                bobstate.emit_progress(context, JoinerProgress::Succeeded);
            }
            Ok(HandshakeMessage::Propagate)
        }

        "vg-member-added-received" | "vc-contact-confirm-received" => {
            // Deprecated steps, delete them immediately.
            Ok(HandshakeMessage::Done)
        }
        _ => {
            warn!(context, "invalid step: {}", step);
            Ok(HandshakeMessage::Ignore)
        }
    }
}

/// Observe self-sent Securejoin message.
///
/// In a multi-device-setup, there may be other devices that "see" the handshake messages.
/// If we see self-sent messages encrypted+signed correctly with our key,
/// we can make some conclusions of it.
///
/// If we see self-sent {vc,vg}-request-with-auth,
/// we know that we are Bob (joiner-observer)
/// that just marked peer (Alice) as forward-verified
/// either after receiving {vc,vg}-auth-required
/// or immediately after scanning the QR-code
/// if the key was already known.
///
/// If we see self-sent vc-contact-confirm or vg-member-added message,
/// we know that we are Alice (inviter-observer)
/// that just marked peer (Bob) as forward (and backward)-verified
/// in response to correct vc-request-with-auth message.
///
/// In both cases we can mark the peer as forward-verified.
pub(crate) async fn observe_securejoin_on_other_device(
    context: &Context,
    mime_message: &MimeMessage,
    contact_id: ContactId,
) -> Result<HandshakeMessage> {
    if contact_id.is_special() {
        return Err(Error::msg("Can not be called with special contact ID"));
    }
    let step = mime_message
        .get_header(HeaderDef::SecureJoin)
        .context("Not a Secure-Join message")?;
    info!(context, "Observing secure-join message {step:?}.");

    if !matches!(
        step,
        "vg-request-with-auth" | "vc-request-with-auth" | "vg-member-added" | "vc-contact-confirm"
    ) {
        return Ok(HandshakeMessage::Ignore);
    };

    if !encrypted_and_signed(context, mime_message, &get_self_fingerprint(context).await?) {
        could_not_establish_secure_connection(
            context,
            contact_id,
            info_chat_id(context, contact_id).await?,
            "Message not encrypted correctly.",
        )
        .await?;
        return Ok(HandshakeMessage::Ignore);
    }

    let addr = Contact::get_by_id(context, contact_id)
        .await?
        .get_addr()
        .to_lowercase();

    let Some(key) = mime_message.gossiped_keys.get(&addr) else {
        could_not_establish_secure_connection(
            context,
            contact_id,
            info_chat_id(context, contact_id).await?,
            &format!(
                "No gossip header for '{}' at step {}, please update Delta Chat on all \
                        your devices.",
                &addr, step,
            ),
        )
        .await?;
        return Ok(HandshakeMessage::Ignore);
    };

    let Some(mut peerstate) = Peerstate::from_addr(context, &addr).await? else {
        could_not_establish_secure_connection(
            context,
            contact_id,
            info_chat_id(context, contact_id).await?,
            &format!("No peerstate in db for '{}' at step {}", &addr, step),
        )
        .await?;
        return Ok(HandshakeMessage::Ignore);
    };

    let Some(fingerprint) = peerstate.gossip_key_fingerprint.clone() else {
        could_not_establish_secure_connection(
            context,
            contact_id,
            info_chat_id(context, contact_id).await?,
            &format!(
                "No gossip key fingerprint in db for '{}' at step {}",
                &addr, step,
            ),
        )
        .await?;
        return Ok(HandshakeMessage::Ignore);
    };
    peerstate.set_verified(key.clone(), fingerprint, addr)?;
    if matches!(step, "vg-member-added" | "vc-contact-confirm") {
        peerstate.backward_verified_key_id =
            Some(context.get_config_i64(Config::KeyId).await?).filter(|&id| id > 0);
    }
    peerstate.prefer_encrypt = EncryptPreference::Mutual;
    peerstate.save_to_db(&context.sql).await?;

    ChatId::set_protection_for_contact(context, contact_id, mime_message.timestamp_sent).await?;

    if step == "vg-member-added" {
        inviter_progress(context, contact_id, 800);
    }
    if step == "vg-member-added" || step == "vc-contact-confirm" {
        inviter_progress(context, contact_id, 1000);
    }

    if step == "vg-request-with-auth" || step == "vc-request-with-auth" {
        // This actually reflects what happens on the first device (which does the secure
        // join) and causes a subsequent "vg-member-added" message to create an unblocked
        // verified group.
        ChatId::create_for_contact_with_blocked(context, contact_id, Blocked::Not).await?;
    }

    if step == "vg-member-added" {
        Ok(HandshakeMessage::Propagate)
    } else {
        Ok(HandshakeMessage::Ignore)
    }
}

async fn secure_connection_established(
    context: &Context,
    contact_id: ContactId,
    chat_id: ChatId,
    timestamp: i64,
) -> Result<()> {
    let private_chat_id = ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Yes)
        .await?
        .id;
    private_chat_id
        .set_protection(
            context,
            ProtectionStatus::Protected,
            timestamp,
            Some(contact_id),
        )
        .await?;
    context.emit_event(EventType::ChatModified(chat_id));
    chatlist_events::emit_chatlist_item_changed(context, chat_id);
    Ok(())
}

async fn could_not_establish_secure_connection(
    context: &Context,
    contact_id: ContactId,
    chat_id: ChatId,
    details: &str,
) -> Result<()> {
    let contact = Contact::get_by_id(context, contact_id).await?;
    let mut msg = stock_str::contact_not_verified(context, &contact).await;
    msg += " (";
    msg += details;
    msg += ")";
    chat::add_info_msg(context, chat_id, &msg, time()).await?;
    warn!(
        context,
        "StockMessage::ContactNotVerified posted to 1:1 chat ({})", details
    );
    Ok(())
}

/// Tries to mark peer with provided key fingerprint as verified.
///
/// Returns true if such key was found, false otherwise.
async fn mark_peer_as_verified(
    context: &Context,
    fingerprint: Fingerprint,
    verifier: String,
    backward_verified: bool,
) -> Result<bool> {
    let Some(ref mut peerstate) = Peerstate::from_fingerprint(context, &fingerprint).await? else {
        return Ok(false);
    };
    let Some(ref public_key) = peerstate.public_key else {
        return Ok(false);
    };
    peerstate.set_verified(public_key.clone(), fingerprint, verifier)?;
    peerstate.prefer_encrypt = EncryptPreference::Mutual;
    if backward_verified {
        peerstate.backward_verified_key_id =
            Some(context.get_config_i64(Config::KeyId).await?).filter(|&id| id > 0);
    }
    peerstate.save_to_db(&context.sql).await?;
    Ok(true)
}

/* ******************************************************************************
 * Tools: Misc.
 ******************************************************************************/

fn encrypted_and_signed(
    context: &Context,
    mimeparser: &MimeMessage,
    expected_fingerprint: &Fingerprint,
) -> bool {
    if !mimeparser.was_encrypted() {
        warn!(context, "Message not encrypted.",);
        false
    } else if !mimeparser.signatures.contains(expected_fingerprint) {
        warn!(
            context,
            "Message does not match expected fingerprint {}.", expected_fingerprint,
        );
        false
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use deltachat_contact_tools::{ContactAddress, EmailAddress};

    use super::*;
    use crate::chat::{remove_contact_from_chat, CantSendReason};
    use crate::chatlist::Chatlist;
    use crate::constants::{self, Chattype};
    use crate::imex::{imex, ImexMode};
    use crate::receive_imf::receive_imf;
    use crate::stock_str::{self, chat_protection_enabled};
    use crate::test_utils::get_chat_msg;
    use crate::test_utils::{TestContext, TestContextManager};
    use crate::tools::SystemTime;
    use std::collections::HashSet;
    use std::time::Duration;

    #[derive(PartialEq)]
    enum SetupContactCase {
        Normal,
        CheckProtectionTimestamp,
        WrongAliceGossip,
        SecurejoinWaitTimeout,
        AliceIsBot,
        AliceHasName,
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact() {
        test_setup_contact_ex(SetupContactCase::Normal).await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_protection_timestamp() {
        test_setup_contact_ex(SetupContactCase::CheckProtectionTimestamp).await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_wrong_alice_gossip() {
        test_setup_contact_ex(SetupContactCase::WrongAliceGossip).await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_wait_timeout() {
        test_setup_contact_ex(SetupContactCase::SecurejoinWaitTimeout).await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_alice_is_bot() {
        test_setup_contact_ex(SetupContactCase::AliceIsBot).await
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_alice_has_name() {
        test_setup_contact_ex(SetupContactCase::AliceHasName).await
    }

    async fn test_setup_contact_ex(case: SetupContactCase) {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let alice_addr = &alice.get_config(Config::Addr).await.unwrap().unwrap();
        if case == SetupContactCase::AliceHasName {
            alice
                .set_config(Config::Displayname, Some("Alice"))
                .await
                .unwrap();
        }
        let bob = tcm.bob().await;
        bob.set_config(Config::Displayname, Some("Bob Examplenet"))
            .await
            .unwrap();
        let alice_auto_submitted_hdr;
        match case {
            SetupContactCase::AliceIsBot => {
                alice.set_config_bool(Config::Bot, true).await.unwrap();
                alice_auto_submitted_hdr = "Auto-Submitted: auto-generated";
            }
            _ => alice_auto_submitted_hdr = "Auto-Submitted: auto-replied",
        };
        for t in [&alice, &bob] {
            t.set_config_bool(Config::VerifiedOneOnOneChats, true)
                .await
                .unwrap();
        }

        assert_eq!(
            Chatlist::try_load(&alice, 0, None, None)
                .await
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            Chatlist::try_load(&bob, 0, None, None).await.unwrap().len(),
            0
        );

        // Step 1: Generate QR-code, ChatId(0) indicates setup-contact
        let qr = get_securejoin_qr(&alice.ctx, None).await.unwrap();
        // We want Bob to learn Alice's name from their messages, not from the QR code.
        alice
            .set_config(Config::Displayname, Some("Alice Exampleorg"))
            .await
            .unwrap();

        // Step 2: Bob scans QR-code, sends vc-request
        join_securejoin(&bob.ctx, &qr).await.unwrap();
        assert_eq!(
            Chatlist::try_load(&bob, 0, None, None).await.unwrap().len(),
            1
        );
        let contact_alice_id = Contact::lookup_id_by_addr(&bob.ctx, alice_addr, Origin::Unknown)
            .await
            .expect("Error looking up contact")
            .expect("Contact not found");
        let sent = bob.pop_sent_msg().await;
        assert!(!sent.payload.contains("Bob Examplenet"));
        assert_eq!(sent.recipient(), EmailAddress::new(alice_addr).unwrap());
        let msg = alice.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        assert_eq!(msg.get_header(HeaderDef::SecureJoin).unwrap(), "vc-request");
        assert!(msg.get_header(HeaderDef::SecureJoinInvitenumber).is_some());
        assert!(msg.get_header(HeaderDef::AutoSubmitted).is_none());

        // Step 3: Alice receives vc-request, sends vc-auth-required
        alice.recv_msg_trash(&sent).await;
        assert_eq!(
            Chatlist::try_load(&alice, 0, None, None)
                .await
                .unwrap()
                .len(),
            1
        );

        let sent = alice.pop_sent_msg().await;
        assert!(sent.payload.contains(alice_auto_submitted_hdr));
        assert!(!sent.payload.contains("Alice Exampleorg"));
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vc-auth-required"
        );
        let bob_chat = bob.create_chat(&alice).await;
        assert_eq!(bob_chat.can_send(&bob).await.unwrap(), false);
        assert_eq!(
            bob_chat.why_cant_send(&bob).await.unwrap(),
            Some(CantSendReason::SecurejoinWait)
        );
        if case == SetupContactCase::SecurejoinWaitTimeout {
            SystemTime::shift(Duration::from_secs(constants::SECUREJOIN_WAIT_TIMEOUT));
            assert_eq!(bob_chat.can_send(&bob).await.unwrap(), true);
        }

        // Step 4: Bob receives vc-auth-required, sends vc-request-with-auth
        bob.recv_msg_trash(&sent).await;
        let bob_chat = bob.create_chat(&alice).await;
        assert_eq!(bob_chat.can_send(&bob).await.unwrap(), true);

        // Check Bob emitted the JoinerProgress event.
        let event = bob
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::SecurejoinJoinerProgress { .. }))
            .await;
        match event {
            EventType::SecurejoinJoinerProgress {
                contact_id,
                progress,
            } => {
                let alice_contact_id =
                    Contact::lookup_id_by_addr(&bob.ctx, alice_addr, Origin::Unknown)
                        .await
                        .expect("Error looking up contact")
                        .expect("Contact not found");
                assert_eq!(contact_id, alice_contact_id);
                assert_eq!(progress, 400);
            }
            _ => unreachable!(),
        }

        // Check Bob sent the right message.
        let sent = bob.pop_sent_msg().await;
        assert!(sent.payload.contains("Auto-Submitted: auto-replied"));
        assert!(!sent.payload.contains("Bob Examplenet"));
        let mut msg = alice.parse_msg(&sent).await;
        let vc_request_with_auth_ts_sent = msg
            .get_header(HeaderDef::Date)
            .and_then(|value| mailparse::dateparse(value).ok())
            .unwrap();
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vc-request-with-auth"
        );
        assert!(msg.get_header(HeaderDef::SecureJoinAuth).is_some());
        let bob_fp = load_self_public_key(&bob.ctx).await.unwrap().fingerprint();
        assert_eq!(
            *msg.get_header(HeaderDef::SecureJoinFingerprint).unwrap(),
            bob_fp.hex()
        );

        if case == SetupContactCase::WrongAliceGossip {
            let wrong_pubkey = load_self_public_key(&bob).await.unwrap();
            let alice_pubkey = msg
                .gossiped_keys
                .insert(alice_addr.to_string(), wrong_pubkey)
                .unwrap();
            let contact_bob = alice.add_or_lookup_contact(&bob).await;
            let handshake_msg = handle_securejoin_handshake(&alice, &msg, contact_bob.id)
                .await
                .unwrap();
            assert_eq!(handshake_msg, HandshakeMessage::Ignore);
            assert_eq!(contact_bob.is_verified(&alice.ctx).await.unwrap(), false);

            msg.gossiped_keys
                .insert(alice_addr.to_string(), alice_pubkey)
                .unwrap();
            let handshake_msg = handle_securejoin_handshake(&alice, &msg, contact_bob.id)
                .await
                .unwrap();
            assert_eq!(handshake_msg, HandshakeMessage::Ignore);
            assert!(contact_bob.is_verified(&alice.ctx).await.unwrap());
            return;
        }

        // Alice should not yet have Bob verified
        let contact_bob_id =
            Contact::lookup_id_by_addr(&alice.ctx, "bob@example.net", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_bob = Contact::get_by_id(&alice.ctx, contact_bob_id)
            .await
            .unwrap();
        assert_eq!(contact_bob.is_verified(&alice.ctx).await.unwrap(), false);
        assert_eq!(contact_bob.get_authname(), "");

        if case == SetupContactCase::CheckProtectionTimestamp {
            SystemTime::shift(Duration::from_secs(3600));
        }

        // Step 5+6: Alice receives vc-request-with-auth, sends vc-contact-confirm
        alice.recv_msg_trash(&sent).await;
        assert_eq!(contact_bob.is_verified(&alice.ctx).await.unwrap(), true);
        let contact_bob = Contact::get_by_id(&alice.ctx, contact_bob_id)
            .await
            .unwrap();
        assert_eq!(contact_bob.get_authname(), "Bob Examplenet");
        assert!(contact_bob.get_name().is_empty());
        assert_eq!(contact_bob.is_bot(), false);

        // exactly one one-to-one chat should be visible for both now
        // (check this before calling alice.create_chat() explicitly below)
        assert_eq!(
            Chatlist::try_load(&alice, 0, None, None)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            Chatlist::try_load(&bob, 0, None, None).await.unwrap().len(),
            1
        );

        // Check Alice got the verified message in her 1:1 chat.
        {
            let chat = alice.create_chat(&bob).await;
            let msg = get_chat_msg(&alice, chat.get_id(), 0, 1).await;
            assert!(msg.is_info());
            let expected_text = chat_protection_enabled(&alice).await;
            assert_eq!(msg.get_text(), expected_text);
            if case == SetupContactCase::CheckProtectionTimestamp {
                assert_eq!(msg.timestamp_sort, vc_request_with_auth_ts_sent + 1);
            }
        }

        // Make sure Alice hasn't yet sent their name to Bob.
        let contact_alice = Contact::get_by_id(&bob.ctx, contact_alice_id)
            .await
            .unwrap();
        match case {
            SetupContactCase::AliceHasName => assert_eq!(contact_alice.get_authname(), "Alice"),
            _ => assert_eq!(contact_alice.get_authname(), ""),
        };

        // Check Alice sent the right message to Bob.
        let sent = alice.pop_sent_msg().await;
        assert!(sent.payload.contains(alice_auto_submitted_hdr));
        assert!(!sent.payload.contains("Alice Exampleorg"));
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vc-contact-confirm"
        );

        // Bob should not yet have Alice verified
        assert_eq!(contact_alice.is_verified(&bob.ctx).await.unwrap(), false);

        // Step 7: Bob receives vc-contact-confirm
        bob.recv_msg_trash(&sent).await;
        assert_eq!(contact_alice.is_verified(&bob.ctx).await.unwrap(), true);
        let contact_alice = Contact::get_by_id(&bob.ctx, contact_alice_id)
            .await
            .unwrap();
        assert_eq!(contact_alice.get_authname(), "Alice Exampleorg");
        assert!(contact_alice.get_name().is_empty());
        assert_eq!(contact_alice.is_bot(), case == SetupContactCase::AliceIsBot);

        if case != SetupContactCase::SecurejoinWaitTimeout {
            // Later we check that the timeout message isn't added to the already protected chat.
            SystemTime::shift(Duration::from_secs(constants::SECUREJOIN_WAIT_TIMEOUT + 1));
            assert_eq!(
                bob_chat
                    .check_securejoin_wait(&bob, constants::SECUREJOIN_WAIT_TIMEOUT)
                    .await
                    .unwrap(),
                0
            );
        }

        // Check Bob got expected info messages in his 1:1 chat.
        let msg_cnt: usize = match case {
            SetupContactCase::SecurejoinWaitTimeout => 3,
            _ => 2,
        };
        let mut i = 0..msg_cnt;
        let msg = get_chat_msg(&bob, bob_chat.get_id(), i.next().unwrap(), msg_cnt).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_text(), stock_str::securejoin_wait(&bob).await);
        if case == SetupContactCase::SecurejoinWaitTimeout {
            let msg = get_chat_msg(&bob, bob_chat.get_id(), i.next().unwrap(), msg_cnt).await;
            assert!(msg.is_info());
            assert_eq!(
                msg.get_text(),
                stock_str::securejoin_wait_timeout(&bob).await
            );
        }
        let msg = get_chat_msg(&bob, bob_chat.get_id(), i.next().unwrap(), msg_cnt).await;
        assert!(msg.is_info());
        assert_eq!(msg.get_text(), chat_protection_enabled(&bob).await);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_bad_qr() {
        let bob = TestContext::new_bob().await;
        let ret = join_securejoin(&bob.ctx, "not a qr code").await;
        assert!(ret.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_bob_knows_alice() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        // Ensure Bob knows Alice_FP
        let alice_pubkey = load_self_public_key(&alice.ctx).await?;
        let peerstate = Peerstate {
            addr: "alice@example.org".into(),
            last_seen: 10,
            last_seen_autocrypt: 10,
            prefer_encrypt: EncryptPreference::Mutual,
            public_key: Some(alice_pubkey.clone()),
            public_key_fingerprint: Some(alice_pubkey.fingerprint()),
            gossip_key: Some(alice_pubkey.clone()),
            gossip_timestamp: 10,
            gossip_key_fingerprint: Some(alice_pubkey.fingerprint()),
            verified_key: None,
            verified_key_fingerprint: None,
            verifier: None,
            secondary_verified_key: None,
            secondary_verified_key_fingerprint: None,
            secondary_verifier: None,
            backward_verified_key_id: None,
            fingerprint_changed: false,
        };
        peerstate.save_to_db(&bob.ctx.sql).await?;

        // Step 1: Generate QR-code, ChatId(0) indicates setup-contact
        let qr = get_securejoin_qr(&alice.ctx, None).await?;

        // Step 2+4: Bob scans QR-code, sends vc-request-with-auth, skipping vc-request
        join_securejoin(&bob.ctx, &qr).await.unwrap();

        // Check Bob emitted the JoinerProgress event.
        let event = bob
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::SecurejoinJoinerProgress { .. }))
            .await;
        match event {
            EventType::SecurejoinJoinerProgress {
                contact_id,
                progress,
            } => {
                let alice_contact_id =
                    Contact::lookup_id_by_addr(&bob.ctx, "alice@example.org", Origin::Unknown)
                        .await
                        .expect("Error looking up contact")
                        .expect("Contact not found");
                assert_eq!(contact_id, alice_contact_id);
                assert_eq!(progress, 400);
            }
            _ => unreachable!(),
        }

        // Check Bob sent the right handshake message.
        let sent = bob.pop_sent_msg().await;
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vc-request-with-auth"
        );
        assert!(msg.get_header(HeaderDef::SecureJoinAuth).is_some());
        let bob_fp = load_self_public_key(&bob.ctx).await?.fingerprint();
        assert_eq!(
            *msg.get_header(HeaderDef::SecureJoinFingerprint).unwrap(),
            bob_fp.hex()
        );

        // Alice should not yet have Bob verified
        let (contact_bob_id, _modified) = Contact::add_or_lookup(
            &alice.ctx,
            "",
            &ContactAddress::new("bob@example.net")?,
            Origin::ManuallyCreated,
        )
        .await?;
        let contact_bob = Contact::get_by_id(&alice.ctx, contact_bob_id).await?;
        assert_eq!(contact_bob.is_verified(&alice.ctx).await?, false);

        // Step 5+6: Alice receives vc-request-with-auth, sends vc-contact-confirm
        alice.recv_msg_trash(&sent).await;
        assert_eq!(contact_bob.is_verified(&alice.ctx).await?, true);

        let sent = alice.pop_sent_msg().await;
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vc-contact-confirm"
        );

        // Bob should not yet have Alice verified
        let contact_alice_id =
            Contact::lookup_id_by_addr(&bob.ctx, "alice@example.org", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_alice = Contact::get_by_id(&bob.ctx, contact_alice_id).await?;
        assert_eq!(contact_bob.is_verified(&bob.ctx).await?, false);

        // Step 7: Bob receives vc-contact-confirm
        bob.recv_msg_trash(&sent).await;
        assert_eq!(contact_alice.is_verified(&bob.ctx).await?, true);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_setup_contact_concurrent_calls() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        // do a scan that is not working as claire is never responding
        let qr_stale = "OPENPGP4FPR:1234567890123456789012345678901234567890#a=claire%40foo.de&n=&i=12345678901&s=23456789012";
        let claire_id = join_securejoin(&bob, qr_stale).await?;
        let chat = Chat::load_from_db(&bob, claire_id).await?;
        assert!(!claire_id.is_special());
        assert_eq!(chat.typ, Chattype::Single);
        assert!(bob.pop_sent_msg().await.payload().contains("claire@foo.de"));

        // subsequent scans shall abort existing ones or run concurrently -
        // but they must not fail as otherwise the whole qr scanning becomes unusable until restart.
        let qr = get_securejoin_qr(&alice, None).await?;
        let alice_id = join_securejoin(&bob, &qr).await?;
        let chat = Chat::load_from_db(&bob, alice_id).await?;
        assert!(!alice_id.is_special());
        assert_eq!(chat.typ, Chattype::Single);
        assert_ne!(claire_id, alice_id);
        assert!(bob
            .pop_sent_msg()
            .await
            .payload()
            .contains("alice@example.org"));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_secure_join() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        // We start with empty chatlists.
        assert_eq!(Chatlist::try_load(&alice, 0, None, None).await?.len(), 0);
        assert_eq!(Chatlist::try_load(&bob, 0, None, None).await?.len(), 0);

        let alice_chatid =
            chat::create_group_chat(&alice.ctx, ProtectionStatus::Protected, "the chat").await?;

        // Step 1: Generate QR-code, secure-join implied by chatid
        let qr = get_securejoin_qr(&alice.ctx, Some(alice_chatid))
            .await
            .unwrap();

        // Step 2: Bob scans QR-code, sends vg-request
        let bob_chatid = join_securejoin(&bob.ctx, &qr).await?;
        assert_eq!(Chatlist::try_load(&bob, 0, None, None).await?.len(), 1);

        let sent = bob.pop_sent_msg().await;
        assert_eq!(
            sent.recipient(),
            EmailAddress::new("alice@example.org").unwrap()
        );
        let msg = alice.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        assert_eq!(msg.get_header(HeaderDef::SecureJoin).unwrap(), "vg-request");
        assert!(msg.get_header(HeaderDef::SecureJoinInvitenumber).is_some());
        assert!(msg.get_header(HeaderDef::AutoSubmitted).is_none());

        // Old Delta Chat core sent `Secure-Join-Group` header in `vg-request`,
        // but it was only used by Alice in `vg-request-with-auth`.
        // New Delta Chat versions do not use `Secure-Join-Group` header at all
        // and it is deprecated.
        // Now `Secure-Join-Group` header
        // is only sent in `vg-request-with-auth` for compatibility.
        assert!(msg.get_header(HeaderDef::SecureJoinGroup).is_none());

        // Step 3: Alice receives vg-request, sends vg-auth-required
        alice.recv_msg_trash(&sent).await;

        let sent = alice.pop_sent_msg().await;
        assert!(sent.payload.contains("Auto-Submitted: auto-replied"));
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vg-auth-required"
        );

        // Step 4: Bob receives vg-auth-required, sends vg-request-with-auth
        bob.recv_msg_trash(&sent).await;
        let sent = bob.pop_sent_msg().await;

        // Check Bob emitted the JoinerProgress event.
        let event = bob
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::SecurejoinJoinerProgress { .. }))
            .await;
        match event {
            EventType::SecurejoinJoinerProgress {
                contact_id,
                progress,
            } => {
                let alice_contact_id =
                    Contact::lookup_id_by_addr(&bob.ctx, "alice@example.org", Origin::Unknown)
                        .await
                        .expect("Error looking up contact")
                        .expect("Contact not found");
                assert_eq!(contact_id, alice_contact_id);
                assert_eq!(progress, 400);
            }
            _ => unreachable!(),
        }

        // Check Bob sent the right handshake message.
        assert!(sent.payload.contains("Auto-Submitted: auto-replied"));
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vg-request-with-auth"
        );
        assert!(msg.get_header(HeaderDef::SecureJoinAuth).is_some());
        let bob_fp = load_self_public_key(&bob.ctx).await?.fingerprint();
        assert_eq!(
            *msg.get_header(HeaderDef::SecureJoinFingerprint).unwrap(),
            bob_fp.hex()
        );

        // Alice should not yet have Bob verified
        let contact_bob_id =
            Contact::lookup_id_by_addr(&alice.ctx, "bob@example.net", Origin::Unknown)
                .await?
                .expect("Contact not found");
        let contact_bob = Contact::get_by_id(&alice.ctx, contact_bob_id).await?;
        assert_eq!(contact_bob.is_verified(&alice.ctx).await?, false);

        // Step 5+6: Alice receives vg-request-with-auth, sends vg-member-added
        alice.recv_msg_trash(&sent).await;
        assert_eq!(contact_bob.is_verified(&alice.ctx).await?, true);

        let sent = alice.pop_sent_msg().await;
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get_header(HeaderDef::SecureJoin).unwrap(),
            "vg-member-added"
        );
        // Formally this message is auto-submitted, but as the member addition is a result of an
        // explicit user action, the Auto-Submitted header shouldn't be present. Otherwise it would
        // be strange to have it in "member-added" messages of verified groups only.
        assert!(msg.get_header(HeaderDef::AutoSubmitted).is_none());

        {
            // Now Alice's chat with Bob should still be hidden, the verified message should
            // appear in the group chat.

            let chat = alice.get_chat(&bob).await;
            assert_eq!(
                chat.blocked,
                Blocked::Yes,
                "Alice's 1:1 chat with Bob is not hidden"
            );
            // There should be 3 messages in the chat:
            // - The ChatProtectionEnabled message
            // - You added member bob@example.net
            let msg = get_chat_msg(&alice, alice_chatid, 0, 2).await;
            assert!(msg.is_info());
            let expected_text = chat_protection_enabled(&alice).await;
            assert_eq!(msg.get_text(), expected_text);
        }

        // Bob should not yet have Alice verified
        let contact_alice_id =
            Contact::lookup_id_by_addr(&bob.ctx, "alice@example.org", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_alice = Contact::get_by_id(&bob.ctx, contact_alice_id).await?;
        assert_eq!(contact_bob.is_verified(&bob.ctx).await?, false);

        // Step 7: Bob receives vg-member-added
        bob.recv_msg(&sent).await;
        {
            // Bob has Alice verified, message shows up in the group chat.
            assert_eq!(contact_alice.is_verified(&bob.ctx).await?, true);
            let chat = bob.get_chat(&alice).await;
            assert_eq!(
                chat.blocked,
                Blocked::Yes,
                "Bob's 1:1 chat with Alice is not hidden"
            );
            for item in chat::get_chat_msgs(&bob.ctx, bob_chatid).await.unwrap() {
                if let chat::ChatItem::Message { msg_id } = item {
                    let msg = Message::load_from_db(&bob.ctx, msg_id).await.unwrap();
                    let text = msg.get_text();
                    println!("msg {msg_id} text: {text}");
                }
            }
        }

        let bob_chat = Chat::load_from_db(&bob.ctx, bob_chatid).await?;
        assert!(bob_chat.is_protected());
        assert!(bob_chat.typ == Chattype::Group);

        // On this "happy path", Alice and Bob get only a group-chat where all information are added to.
        // The one-to-one chats are used internally for the hidden handshake messages,
        // however, should not be visible in the UIs.
        assert_eq!(Chatlist::try_load(&alice, 0, None, None).await?.len(), 1);
        assert_eq!(Chatlist::try_load(&bob, 0, None, None).await?.len(), 1);

        // If Bob then sends a direct message to alice, however, the one-to-one with Alice should appear.
        let bobs_chat_with_alice = bob.create_chat(&alice).await;
        let sent = bob.send_text(bobs_chat_with_alice.id, "Hello").await;
        alice.recv_msg(&sent).await;
        assert_eq!(Chatlist::try_load(&alice, 0, None, None).await?.len(), 2);
        assert_eq!(Chatlist::try_load(&bob, 0, None, None).await?.len(), 2);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_adhoc_group_no_qr() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let mime = br#"Subject: First thread
Message-ID: first@example.org
To: Alice <alice@example.org>, Bob <bob@example.net>
From: Claire <claire@example.org>
Content-Type: text/plain; charset=utf-8; format=flowed; delsp=no

First thread."#;

        receive_imf(&alice, mime, false).await?;
        let msg = alice.get_last_msg().await;
        let chat_id = msg.chat_id;

        assert!(get_securejoin_qr(&alice, Some(chat_id)).await.is_err());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_unknown_sender() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        tcm.execute_securejoin(&alice, &bob).await;

        let alice_chat_id = alice
            .create_group_with_members(ProtectionStatus::Protected, "Group with Bob", &[&bob])
            .await;

        let sent = alice.send_text(alice_chat_id, "Hi!").await;
        let bob_chat_id = bob.recv_msg(&sent).await.chat_id;

        let sent = bob.send_text(bob_chat_id, "Hi hi!").await;

        let alice_bob_contact_id = Contact::create(&alice, "Bob", "bob@example.net").await?;
        remove_contact_from_chat(&alice, alice_chat_id, alice_bob_contact_id).await?;
        alice.pop_sent_msg().await;

        // The message from Bob is delivered late, Bob is already removed.
        let msg = alice.recv_msg(&sent).await;
        assert_eq!(msg.text, "Hi hi!");
        assert_eq!(msg.error.unwrap(), "Unknown sender for this chat.");

        Ok(())
    }

    /// Tests that Bob gets Alice as verified
    /// if `vc-contact-confirm` is lost but Alice then sends
    /// a message to Bob in a verified 1:1 chat with a `Chat-Verified` header.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lost_contact_confirm() {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;
        for t in [&alice, &bob] {
            t.set_config_bool(Config::VerifiedOneOnOneChats, true)
                .await
                .unwrap();
        }

        let qr = get_securejoin_qr(&alice.ctx, None).await.unwrap();
        join_securejoin(&bob.ctx, &qr).await.unwrap();

        // vc-request
        let sent = bob.pop_sent_msg().await;
        alice.recv_msg_trash(&sent).await;

        // vc-auth-required
        let sent = alice.pop_sent_msg().await;
        bob.recv_msg_trash(&sent).await;

        // vc-request-with-auth
        let sent = bob.pop_sent_msg().await;
        alice.recv_msg_trash(&sent).await;

        // Alice has Bob verified now.
        let contact_bob_id =
            Contact::lookup_id_by_addr(&alice.ctx, "bob@example.net", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_bob = Contact::get_by_id(&alice.ctx, contact_bob_id)
            .await
            .unwrap();
        assert_eq!(contact_bob.is_verified(&alice.ctx).await.unwrap(), true);

        // Alice sends vc-contact-confirm, but it gets lost.
        let _sent_vc_contact_confirm = alice.pop_sent_msg().await;

        // Bob should not yet have Alice verified
        let contact_alice_id =
            Contact::lookup_id_by_addr(&bob, "alice@example.org", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_alice = Contact::get_by_id(&bob, contact_alice_id).await.unwrap();
        assert_eq!(contact_alice.is_verified(&bob).await.unwrap(), false);

        // Alice sends a text message to Bob.
        let received_hello = tcm.send_recv(&alice, &bob, "Hello!").await;
        let chat_id = received_hello.chat_id;
        let chat = Chat::load_from_db(&bob, chat_id).await.unwrap();
        assert_eq!(chat.is_protected(), true);

        // Received text message in a verified 1:1 chat results in backward verification
        // and Bob now marks alice as verified.
        let contact_alice = Contact::get_by_id(&bob, contact_alice_id).await.unwrap();
        assert_eq!(contact_alice.is_verified(&bob).await.unwrap(), true);
    }

    /// An unencrypted message with already known Autocrypt key, but sent from another address,
    /// means that it's rather a new contact sharing the same key than the existing one changed its
    /// address, otherwise it would already have our key to encrypt.
    ///
    /// This is a regression test for a bug where DC wrongly executed AEAP in this case.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_shared_bobs_key() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let bob = &tcm.bob().await;
        let bob_addr = &bob.get_config(Config::Addr).await?.unwrap();

        tcm.execute_securejoin(bob, alice).await;

        let export_dir = tempfile::tempdir().unwrap();
        imex(bob, ImexMode::ExportSelfKeys, export_dir.path(), None).await?;
        let bob2 = &TestContext::new().await;
        let bob2_addr = "bob2@example.net";
        bob2.configure_addr(bob2_addr).await;
        imex(bob2, ImexMode::ImportSelfKeys, export_dir.path(), None).await?;

        tcm.execute_securejoin(bob2, alice).await;

        let bob3 = &TestContext::new().await;
        let bob3_addr = "bob3@example.net";
        bob3.configure_addr(bob3_addr).await;
        imex(bob3, ImexMode::ImportSelfKeys, export_dir.path(), None).await?;
        tcm.send_recv(bob3, alice, "hi Alice!").await;
        let msg = tcm.send_recv(alice, bob3, "hi Bob3!").await;
        assert!(msg.get_showpadlock());

        let mut bob_ids = HashSet::new();
        bob_ids.insert(
            Contact::lookup_id_by_addr(alice, bob_addr, Origin::Unknown)
                .await?
                .unwrap(),
        );
        bob_ids.insert(
            Contact::lookup_id_by_addr(alice, bob2_addr, Origin::Unknown)
                .await?
                .unwrap(),
        );
        bob_ids.insert(
            Contact::lookup_id_by_addr(alice, bob3_addr, Origin::Unknown)
                .await?
                .unwrap(),
        );
        assert_eq!(bob_ids.len(), 3);
        Ok(())
    }
}

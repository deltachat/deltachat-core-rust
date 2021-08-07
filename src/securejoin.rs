//! Verified contact protocol implementation as [specified by countermitm project](https://countermitm.readthedocs.io/en/stable/new.html#setup-contact-protocol)

use std::convert::TryFrom;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context as _, Error, Result};
use async_std::channel::Receiver;
use async_std::sync::Mutex;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::aheader::EncryptPreference;
use crate::chat::{self, Chat, ChatId, ChatIdBlocked};
use crate::config::Config;
use crate::constants::{Blocked, Viewtype, DC_CONTACT_ID_LAST_SPECIAL};
use crate::contact::{Contact, Origin, VerifiedStatus};
use crate::context::Context;
use crate::dc_tools::time;
use crate::e2ee::ensure_secret_key_exists;
use crate::events::EventType;
use crate::headerdef::HeaderDef;
use crate::key::{DcKey, Fingerprint, SignedPublicKey};
use crate::message::Message;
use crate::mimeparser::{MimeMessage, SystemMessage};
use crate::param::Param;
use crate::peerstate::{Peerstate, PeerstateKeyType, PeerstateVerifiedStatus, ToSave};
use crate::qr::check_qr;
use crate::stock_str;
use crate::token;

mod bobstate;
mod qrinvite;

use bobstate::{BobHandshakeStage, BobState, BobStateHandle};
use qrinvite::{QrError, QrInvite};

pub const NON_ALPHANUMERIC_WITHOUT_DOT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'.');

macro_rules! joiner_progress {
    ($context:tt, $contact_id:expr, $progress:expr) => {
        assert!(
            $progress >= 0 && $progress <= 1000,
            "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
        );
        $context.emit_event($crate::events::EventType::SecurejoinJoinerProgress {
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
        $context.emit_event($crate::events::EventType::SecurejoinInviterProgress {
            contact_id: $contact_id,
            progress: $progress,
        });
    };
}

/// State for setup-contact/secure-join protocol joiner's side, aka Bob's side.
///
/// The setup-contact protocol needs to carry state for both the inviter (Alice) and the
/// joiner/invitee (Bob).  For Alice this state is minimal and in the `tokens` table in the
/// database.  For Bob this state is only carried live on the [`Context`] in this struct.
#[derive(Debug, Default)]
pub(crate) struct Bob {
    inner: Mutex<Option<BobState>>,
}

/// Return value for [`Bob::start_protocol`].
///
/// This indicates which protocol variant was started and provides the required information
/// about it.
enum StartedProtocolVariant {
    /// The setup-contact protocol, to verify a contact.
    SetupContact,
    /// The secure-join protocol, to join a group.
    SecureJoin {
        ongoing_receiver: Receiver<()>,
        group_id: String,
    },
}

impl Bob {
    /// Starts the securejoin protocol with the QR `invite`.
    ///
    /// This will try to start the securejoin protocol for the given QR `invite`.  If it
    /// succeeded the protocol state will be tracked in `self`.
    ///
    /// This function takes care of starting the "ongoing" mechanism if required and
    /// handling errors while starting the protocol.
    ///
    /// # Returns
    ///
    /// If the started protocol is joining a group the returned struct contains information
    /// about the group and ongoing process.
    async fn start_protocol(
        &self,
        context: &Context,
        invite: QrInvite,
    ) -> Result<StartedProtocolVariant, JoinError> {
        let mut guard = self.inner.lock().await;
        if guard.is_some() {
            return Err(JoinError::AlreadyRunning);
        }
        let variant = match invite {
            QrInvite::Group { ref grpid, .. } => {
                let receiver = context
                    .alloc_ongoing()
                    .await
                    .map_err(|_| JoinError::OngoingRunning)?;
                StartedProtocolVariant::SecureJoin {
                    ongoing_receiver: receiver,
                    group_id: grpid.clone(),
                }
            }
            _ => StartedProtocolVariant::SetupContact,
        };
        match BobState::start_protocol(context, invite).await {
            Ok((state, stage)) => {
                if matches!(stage, BobHandshakeStage::RequestWithAuthSent) {
                    joiner_progress!(context, state.invite().contact_id(), 400);
                }
                *guard = Some(state);
                Ok(variant)
            }
            Err(err) => {
                if let StartedProtocolVariant::SecureJoin { .. } = variant {
                    context.free_ongoing().await;
                }
                Err(err)
            }
        }
    }

    /// Returns a handle to the [`BobState`] of the handshake.
    ///
    /// If there currently isn't a handshake running this will return `None`.  Otherwise
    /// this will return a handle to the current [`BobState`].  This handle allows
    /// processing an incoming message and allows terminating the handshake.
    ///
    /// The handle contains an exclusive lock, which is held until the handle is dropped.
    /// This guarantees all state and state changes are correct and allows safely
    /// terminating the handshake without worrying about concurrency.
    async fn state(&self, context: &Context) -> Option<BobStateHandle<'_>> {
        let guard = self.inner.lock().await;
        let ret = BobStateHandle::from_guard(guard);
        if ret.is_none() {
            info!(context, "No active BobState found for securejoin handshake");
        }
        ret
    }
}

/// Generates a Secure Join QR code.
///
/// With `group` set to `None` this generates a setup-contact QR code, with `group` set to a
/// [`ChatId`] generates a join-group QR code for the given chat.
pub async fn dc_get_securejoin_qr(context: &Context, group: Option<ChatId>) -> Option<String> {
    /*=======================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    =======================================================*/

    ensure_secret_key_exists(context).await.ok();

    // invitenumber will be used to allow starting the handshake,
    // auth will be used to verify the fingerprint
    let invitenumber = token::lookup_or_new(context, token::Namespace::InviteNumber, group).await;
    let auth = token::lookup_or_new(context, token::Namespace::Auth, group).await;
    let self_addr = match context.get_config(Config::ConfiguredAddr).await {
        Ok(Some(addr)) => addr,
        Ok(None) => {
            error!(context, "Not configured, cannot generate QR code.");
            return None;
        }
        Err(err) => {
            error!(
                context,
                "Unable to retrieve configuration, cannot generate QR code: {:?}", err
            );
            return None;
        }
    };

    let self_name = context
        .get_config(Config::Displayname)
        .await
        .ok()?
        .unwrap_or_default();

    let fingerprint: Fingerprint = match get_self_fingerprint(context).await {
        Some(fp) => fp,
        None => {
            return None;
        }
    };

    let self_addr_urlencoded =
        utf8_percent_encode(&self_addr, NON_ALPHANUMERIC_WITHOUT_DOT).to_string();
    let self_name_urlencoded =
        utf8_percent_encode(&self_name, NON_ALPHANUMERIC_WITHOUT_DOT).to_string();

    let qr = if let Some(group) = group {
        // parameters used: a=g=x=i=s=
        if let Ok(chat) = Chat::load_from_db(context, group).await {
            let group_name = chat.get_name();
            let group_name_urlencoded =
                utf8_percent_encode(group_name, NON_ALPHANUMERIC).to_string();

            Some(format!(
                "OPENPGP4FPR:{}#a={}&g={}&x={}&i={}&s={}",
                fingerprint.hex(),
                self_addr_urlencoded,
                &group_name_urlencoded,
                &chat.grpid,
                &invitenumber,
                &auth,
            ))
        } else {
            error!(context, "Cannot get QR-code for chat-id {}", group,);
            return None;
        }
    } else {
        // parameters used: a=n=i=s=
        Some(format!(
            "OPENPGP4FPR:{}#a={}&n={}&i={}&s={}",
            fingerprint.hex(),
            self_addr_urlencoded,
            self_name_urlencoded,
            &invitenumber,
            &auth,
        ))
    };

    info!(context, "Generated QR code: {}", qr.as_ref().unwrap());

    qr
}

async fn get_self_fingerprint(context: &Context) -> Option<Fingerprint> {
    match SignedPublicKey::load_self(context).await {
        Ok(key) => Some(key.fingerprint()),
        Err(_) => {
            warn!(context, "get_self_fingerprint(): failed to load key");
            None
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error("Unknown QR-code: {0}")]
    QrCode(#[from] QrError),
    #[error("A setup-contact/secure-join protocol is already running")]
    AlreadyRunning,
    #[error("An \"ongoing\" process is already running")]
    OngoingRunning,
    #[error("Failed to send handshake message: {0}")]
    SendMessage(#[from] SendMsgError),
    // Note that this can currently only occur if there is a bug in the QR/Lot code as this
    // is supposed to create a contact for us.
    #[error("Unknown contact (this is a bug): {0}")]
    UnknownContact(#[source] anyhow::Error),
    // Note that this can only occur if we failed to create the chat correctly.
    #[error("Ongoing sender dropped (this is a bug)")]
    OngoingSenderDropped,
    #[error("Other")]
    Other(#[from] anyhow::Error),
}

/// Take a scanned QR-code and do the setup-contact/join-group/invite handshake.
///
/// This is the start of the process for the joiner.  See the module and ffi documentation
/// for more details.
///
/// When joining a group this will start an "ongoing" process and will block until the
/// process is completed, the [`ChatId`] for the new group is not known any sooner.  When
/// verifying a contact this returns immediately.
pub async fn dc_join_securejoin(context: &Context, qr: &str) -> Result<ChatId, JoinError> {
    securejoin(context, qr).await.map_err(|err| {
        warn!(context, "Fatal joiner error: {:#}", err);
        // This is a modal operation, the user has context on what failed.
        error!(context, "QR process failed");
        err
    })
}

async fn securejoin(context: &Context, qr: &str) -> Result<ChatId, JoinError> {
    /*========================================================
    ====             Bob - the joiner's side             =====
    ====   Step 2 in "Setup verified contact" protocol   =====
    ========================================================*/

    info!(context, "Requesting secure-join ...",);
    let qr_scan = check_qr(context, qr).await;

    let invite = QrInvite::try_from(qr_scan)?;

    match context.bob.start_protocol(context, invite.clone()).await? {
        StartedProtocolVariant::SetupContact => {
            // for a one-to-one-chat, the chat is already known, return the chat-id,
            // the verification runs in background
            let chat_id = ChatId::create_for_contact(context, invite.contact_id())
                .await
                .map_err(JoinError::UnknownContact)?;
            Ok(chat_id)
        }
        StartedProtocolVariant::SecureJoin {
            ongoing_receiver,
            group_id,
        } => {
            // for a group-join, wait until the protocol is finished and the group is created
            ongoing_receiver
                .recv()
                .await
                .map_err(|_| JoinError::OngoingSenderDropped)?;

            // handle_securejoin_handshake() calls Context::stop_ongoing before the group
            // chat is created (it is created after handle_securejoin_handshake() returns by
            // dc_receive_imf()).  As a hack we just wait a bit for it to appear.

            // If the protocol is aborted by Bob, this timeout will also happen.
            let start = Instant::now();
            let chatid = loop {
                {
                    match chat::get_chat_id_by_grpid(context, &group_id).await? {
                        Some((chatid, _is_protected, _blocked)) => break chatid,
                        None => {
                            if start.elapsed() > Duration::from_secs(7) {
                                context.free_ongoing().await;
                                return Err(JoinError::Other(anyhow!(
                                    "Ongoing sender dropped (this is a bug)"
                                )));
                            }
                        }
                    }
                }
                async_std::task::sleep(Duration::from_millis(50)).await;
            };
            context.free_ongoing().await;
            Ok(chatid)
        }
    }
}

/// Error when failing to send a protocol handshake message.
///
/// Wrapping the [anyhow::Error] means we can "impl From" more easily on errors from this
/// function.
#[derive(Debug, thiserror::Error)]
#[error("Failed sending handshake message")]
pub struct SendMsgError(#[from] anyhow::Error);

async fn send_handshake_msg(
    context: &Context,
    contact_chat_id: ChatId,
    step: &str,
    param2: &str,
    fingerprint: Option<Fingerprint>,
    grpid: &str,
) -> Result<(), SendMsgError> {
    let mut msg = Message {
        viewtype: Viewtype::Text,
        text: Some(format!("Secure-Join: {}", step)),
        hidden: true,
        ..Default::default()
    };
    msg.param.set_cmd(SystemMessage::SecurejoinMessage);
    if step.is_empty() {
        msg.param.remove(Param::Arg);
    } else {
        msg.param.set(Param::Arg, step);
    }
    if !param2.is_empty() {
        msg.param.set(Param::Arg2, param2);
    }
    if let Some(fp) = fingerprint {
        msg.param.set(Param::Arg3, fp.hex());
    }
    if !grpid.is_empty() {
        msg.param.set(Param::Arg4, grpid);
    }
    if step == "vg-request" || step == "vc-request" {
        msg.param.set_int(Param::ForcePlaintext, 1);
    } else {
        msg.param.set_int(Param::GuaranteeE2ee, 1);
    }

    chat::send_msg(context, contact_chat_id, &mut msg).await?;
    Ok(())
}

async fn chat_id_2_contact_id(context: &Context, contact_chat_id: ChatId) -> Result<u32, Error> {
    if let [contact_id] = chat::get_chat_contacts(context, contact_chat_id).await?[..] {
        Ok(contact_id)
    } else {
        Ok(0)
    }
}

async fn fingerprint_equals_sender(
    context: &Context,
    fingerprint: &Fingerprint,
    contact_chat_id: ChatId,
) -> Result<bool, Error> {
    if let [contact_id] = chat::get_chat_contacts(context, contact_chat_id).await?[..] {
        if let Ok(contact) = Contact::load_from_db(context, contact_id).await {
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

            if let Some(peerstate) = peerstate {
                if peerstate.public_key_fingerprint.is_some()
                    && fingerprint == peerstate.public_key_fingerprint.as_ref().unwrap()
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// What to do with a Secure-Join handshake message after it was handled.
///
/// This status is returned to [`dc_receive_imf`] which will use it to decide what to do
/// next with this incoming setup-contact/secure-join handshake message.
///
/// [`dc_receive_imf`]: crate::dc_receive_imf::dc_receive_imf
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
/// This function will update the securejoin state in [`InnerContext::bob`] and also
/// terminate the ongoing process using [`Context::stop_ongoing`] as required by the
/// protocol.
///
/// A message which results in [`Err`] will be hidden from the user but not deleted, it may
/// be a valid message for something else we are not aware off.  E.g. it could be part of a
/// handshake performed by another DC app on the same account.
///
/// When `handle_securejoin_handshake()` is called, the message is not yet filed in the
/// database; this is done by `receive_imf()` later on as needed.
///
/// [`InnerContext::bob`]: crate::context::InnerContext::bob
#[allow(clippy::indexing_slicing)]
pub(crate) async fn handle_securejoin_handshake(
    context: &Context,
    mime_message: &MimeMessage,
    contact_id: u32,
) -> Result<HandshakeMessage> {
    if contact_id <= DC_CONTACT_ID_LAST_SPECIAL {
        return Err(Error::msg("Can not be called with special contact ID"));
    }
    let step = mime_message
        .get(HeaderDef::SecureJoin)
        .context("Not a Secure-Join message")?;

    info!(
        context,
        ">>>>>>>>>>>>>>>>>>>>>>>>> secure-join message \'{}\' received", step,
    );

    let contact_chat_id = {
        let chat = ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Not)
            .await
            .with_context(|| {
                format!(
                    "Failed to look up or create chat for contact {}",
                    contact_id
                )
            })?;
        if chat.blocked != Blocked::Not {
            chat.id.unblock(context).await?;
        }
        chat.id
    };

    let join_vg = step.starts_with("vg-");

    match step.as_str() {
        "vg-request" | "vc-request" => {
            /*=======================================================
            ====             Alice - the inviter side            ====
            ====   Step 3 in "Setup verified contact" protocol   ====
            =======================================================*/

            // this message may be unencrypted (Bob, the joiner and the sender, might not have Alice's key yet)
            // it just ensures, we have Bobs key now. If we do _not_ have the key because eg. MitM has removed it,
            // send_message() will fail with the error "End-to-end-encryption unavailable unexpectedly.", so, there is no additional check needed here.
            // verify that the `Secure-Join-Invitenumber:`-header matches invitenumber written to the QR code
            let invitenumber = match mime_message.get(HeaderDef::SecureJoinInvitenumber) {
                Some(n) => n,
                None => {
                    warn!(context, "Secure-join denied (invitenumber missing)");
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if !token::exists(context, token::Namespace::InviteNumber, invitenumber).await {
                warn!(context, "Secure-join denied (bad invitenumber).");
                return Ok(HandshakeMessage::Ignore);
            }
            info!(context, "Secure-join requested.",);

            inviter_progress!(context, contact_id, 300);

            // Alice -> Bob
            send_handshake_msg(
                context,
                contact_chat_id,
                &format!("{}-auth-required", &step[..2]),
                "",
                None,
                "",
            )
            .await?;
            Ok(HandshakeMessage::Done)
        }
        "vg-auth-required" | "vc-auth-required" => {
            /*========================================================
            ====             Bob - the joiner's side             =====
            ====   Step 4 in "Setup verified contact" protocol   =====
            ========================================================*/
            match context.bob.state(context).await {
                Some(mut bobstate) => match bobstate.handle_message(context, mime_message).await {
                    Some(BobHandshakeStage::Terminated(why)) => {
                        could_not_establish_secure_connection(context, bobstate.chat_id(), why)
                            .await?;
                        Ok(HandshakeMessage::Done)
                    }
                    Some(_stage) => {
                        joiner_progress!(context, bobstate.invite().contact_id(), 400);
                        Ok(HandshakeMessage::Done)
                    }
                    None => Ok(HandshakeMessage::Ignore),
                },
                None => Ok(HandshakeMessage::Ignore),
            }
        }
        "vg-request-with-auth" | "vc-request-with-auth" => {
            /*==========================================================
            ====              Alice - the inviter side              ====
            ====   Steps 5+6 in "Setup verified contact" protocol   ====
            ====  Step 6 in "Out-of-band verified groups" protocol  ====
            ==========================================================*/

            // verify that Secure-Join-Fingerprint:-header matches the fingerprint of Bob
            let fingerprint: Fingerprint = match mime_message.get(HeaderDef::SecureJoinFingerprint)
            {
                Some(fp) => fp.parse()?,
                None => {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Fingerprint not provided.",
                    )
                    .await?;
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if !encrypted_and_signed(context, mime_message, Some(&fingerprint)) {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Auth not encrypted.",
                )
                .await?;
                return Ok(HandshakeMessage::Ignore);
            }
            if !fingerprint_equals_sender(context, &fingerprint, contact_chat_id).await? {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on inviter-side.",
                )
                .await?;
                return Ok(HandshakeMessage::Ignore);
            }
            info!(context, "Fingerprint verified.",);
            // verify that the `Secure-Join-Auth:`-header matches the secret written to the QR code
            let auth_0 = match mime_message.get(HeaderDef::SecureJoinAuth) {
                Some(auth) => auth,
                None => {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Auth not provided.",
                    )
                    .await?;
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if !token::exists(context, token::Namespace::Auth, auth_0).await {
                could_not_establish_secure_connection(context, contact_chat_id, "Auth invalid.")
                    .await?;
                return Ok(HandshakeMessage::Ignore);
            }
            if mark_peer_as_verified(context, &fingerprint).await.is_err() {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on inviter-side.",
                )
                .await?;
                return Ok(HandshakeMessage::Ignore);
            }
            Contact::scaleup_origin_by_id(context, contact_id, Origin::SecurejoinInvited).await;
            info!(context, "Auth verified.",);
            secure_connection_established(context, contact_chat_id).await?;
            emit_event!(context, EventType::ContactsChanged(Some(contact_id)));
            inviter_progress!(context, contact_id, 600);
            if join_vg {
                // the vg-member-added message is special:
                // this is a normal Chat-Group-Member-Added message
                // with an additional Secure-Join header
                let field_grpid = match mime_message.get(HeaderDef::SecureJoinGroup) {
                    Some(s) => s.as_str(),
                    None => {
                        warn!(context, "Missing Secure-Join-Group header");
                        return Ok(HandshakeMessage::Ignore);
                    }
                };
                match chat::get_chat_id_by_grpid(context, field_grpid).await? {
                    Some((group_chat_id, _, _)) => {
                        if let Err(err) =
                            chat::add_contact_to_chat_ex(context, group_chat_id, contact_id, true)
                                .await
                        {
                            error!(context, "failed to add contact: {}", err);
                        }
                    }
                    None => bail!("Chat {} not found", &field_grpid),
                }
            } else {
                // Alice -> Bob
                send_handshake_msg(
                    context,
                    contact_chat_id,
                    "vc-contact-confirm",
                    "",
                    Some(fingerprint),
                    "",
                )
                .await?;

                inviter_progress!(context, contact_id, 1000);
            }
            Ok(HandshakeMessage::Ignore) // "Done" would delete the message and break multi-device (the key from Autocrypt-header is needed)
        }
        "vg-member-added" | "vc-contact-confirm" => {
            /*=======================================================
            ====             Bob - the joiner's side             ====
            ====   Step 7 in "Setup verified contact" protocol   ====
            =======================================================*/
            info!(context, "matched vc-contact-confirm step");
            let retval = if join_vg {
                HandshakeMessage::Propagate
            } else {
                HandshakeMessage::Ignore
            };
            match context.bob.state(context).await {
                Some(mut bobstate) => match bobstate.handle_message(context, mime_message).await {
                    Some(BobHandshakeStage::Terminated(why)) => {
                        could_not_establish_secure_connection(context, bobstate.chat_id(), why)
                            .await?;
                        Ok(HandshakeMessage::Done)
                    }
                    Some(BobHandshakeStage::Completed) => {
                        // Can only be BobHandshakeStage::Completed
                        secure_connection_established(context, bobstate.chat_id()).await?;
                        Ok(retval)
                    }
                    Some(_) => {
                        warn!(
                            context,
                            "Impossible state returned from handling handshake message"
                        );
                        Ok(retval)
                    }
                    None => Ok(retval),
                },
                None => Ok(retval),
            }
        }
        "vg-member-added-received" | "vc-contact-confirm-received" => {
            /*==========================================================
            ====              Alice - the inviter side              ====
            ====  Step 8 in "Out-of-band verified groups" protocol  ====
            ==========================================================*/

            if let Ok(contact) = Contact::get_by_id(context, contact_id).await {
                if contact.is_verified(context).await == VerifiedStatus::Unverified {
                    warn!(context, "{} invalid.", step);
                    return Ok(HandshakeMessage::Ignore);
                }
                if join_vg {
                    // Responsible for showing "$Bob securely joined $group" message
                    inviter_progress!(context, contact_id, 800);
                    inviter_progress!(context, contact_id, 1000);
                    let field_grpid = mime_message
                        .get(HeaderDef::SecureJoinGroup)
                        .map(|s| s.as_str())
                        .unwrap_or_else(|| "");
                    if let Err(err) = chat::get_chat_id_by_grpid(context, &field_grpid).await {
                        warn!(context, "Failed to lookup chat_id from grpid: {}", err);
                        return Err(
                            err.context(format!("Chat for group {} not found", &field_grpid))
                        );
                    }
                }
                Ok(HandshakeMessage::Ignore) // "Done" deletes the message and breaks multi-device
            } else {
                warn!(context, "{} invalid.", step);
                Ok(HandshakeMessage::Ignore)
            }
        }
        _ => {
            warn!(context, "invalid step: {}", step);
            Ok(HandshakeMessage::Ignore)
        }
    }
}

/// observe_securejoin_on_other_device() must be called when a self-sent securejoin message is seen.
///
/// in a multi-device-setup, there may be other devices that "see" the handshake messages.
/// if the seen messages seen are self-sent messages encrypted+signed correctly with our key,
/// we can make some conclusions of it:
///
/// - if we see the self-sent-message vg-member-added/vc-contact-confirm,
///   we know that we're an inviter-observer.
///   the inviting device has marked a peer as verified on vg-request-with-auth/vc-request-with-auth
///   before sending vg-member-added/vc-contact-confirm - so, if we observe vg-member-added/vc-contact-confirm,
///   we can mark the peer as verified as well.
///
/// - if we see the self-sent-message vg-member-added-received
///   we know that we're an joiner-observer.
///   the joining device has marked the peer as verified on vg-member-added/vc-contact-confirm
///   before sending vg-member-added-received - so, if we observe vg-member-added-received,
///   we can mark the peer as verified as well.
pub(crate) async fn observe_securejoin_on_other_device(
    context: &Context,
    mime_message: &MimeMessage,
    contact_id: u32,
) -> Result<HandshakeMessage> {
    if contact_id <= DC_CONTACT_ID_LAST_SPECIAL {
        return Err(Error::msg("Can not be called with special contact ID"));
    }
    let step = mime_message
        .get(HeaderDef::SecureJoin)
        .context("Not a Secure-Join message")?;
    info!(context, "observing secure-join message \'{}\'", step);

    let contact_chat_id = {
        let chat = ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Not)
            .await
            .with_context(|| {
                format!(
                    "Failed to look up or create chat for contact {}",
                    contact_id
                )
            })?;
        if chat.blocked != Blocked::Not {
            chat.id.unblock(context).await?;
        }
        chat.id
    };

    match step.as_str() {
        "vg-member-added"
        | "vc-contact-confirm"
        | "vg-member-added-received"
        | "vc-contact-confirm-received" => {
            if !encrypted_and_signed(
                context,
                mime_message,
                get_self_fingerprint(context).await.as_ref(),
            ) {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Message not encrypted correctly.",
                )
                .await?;
                return Ok(HandshakeMessage::Ignore);
            }
            let fingerprint: Fingerprint = match mime_message.get(HeaderDef::SecureJoinFingerprint)
            {
                Some(fp) => fp.parse()?,
                None => {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        "Fingerprint not provided, please update Delta Chat on all your devices.",
                    )
                    .await?;
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if mark_peer_as_verified(context, &fingerprint).await.is_err() {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    format!("Fingerprint mismatch on observing {}.", step).as_ref(),
                )
                .await?;
                return Ok(HandshakeMessage::Ignore);
            }
            Ok(if step.as_str() == "vg-member-added" {
                HandshakeMessage::Propagate
            } else {
                HandshakeMessage::Ignore
            })
        }
        _ => Ok(HandshakeMessage::Ignore),
    }
}

async fn secure_connection_established(
    context: &Context,
    contact_chat_id: ChatId,
) -> Result<(), Error> {
    let contact_id = chat_id_2_contact_id(context, contact_chat_id).await?;
    let contact = Contact::get_by_id(context, contact_id).await;

    let addr = if let Ok(ref contact) = contact {
        contact.get_addr()
    } else {
        "?"
    };
    let msg = stock_str::contact_verified(context, addr).await;
    chat::add_info_msg(context, contact_chat_id, msg, time()).await;
    emit_event!(context, EventType::ChatModified(contact_chat_id));
    info!(context, "StockMessage::ContactVerified posted to 1:1 chat");

    Ok(())
}

async fn could_not_establish_secure_connection(
    context: &Context,
    contact_chat_id: ChatId,
    details: &str,
) -> Result<(), Error> {
    let contact_id = chat_id_2_contact_id(context, contact_chat_id).await?;
    let contact = Contact::get_by_id(context, contact_id).await;
    let msg = stock_str::contact_not_verified(
        context,
        if let Ok(ref contact) = contact {
            contact.get_addr()
        } else {
            "?"
        },
    )
    .await;

    chat::add_info_msg(context, contact_chat_id, &msg, time()).await;
    error!(
        context,
        "StockMessage::ContactNotVerified posted to 1:1 chat ({})", details
    );

    Ok(())
}

async fn mark_peer_as_verified(context: &Context, fingerprint: &Fingerprint) -> Result<(), Error> {
    if let Some(ref mut peerstate) =
        Peerstate::from_fingerprint(context, &context.sql, fingerprint).await?
    {
        if peerstate.set_verified(
            PeerstateKeyType::PublicKey,
            fingerprint,
            PeerstateVerifiedStatus::BidirectVerified,
        ) {
            peerstate.prefer_encrypt = EncryptPreference::Mutual;
            peerstate.to_save = Some(ToSave::All);
            peerstate
                .save_to_db(&context.sql, false)
                .await
                .unwrap_or_default();
            return Ok(());
        }
    }
    bail!(
        "could not mark peer as verified for fingerprint {}",
        fingerprint.hex()
    );
}

/* ******************************************************************************
 * Tools: Misc.
 ******************************************************************************/

fn encrypted_and_signed(
    context: &Context,
    mimeparser: &MimeMessage,
    expected_fingerprint: Option<&Fingerprint>,
) -> bool {
    if !mimeparser.was_encrypted() {
        warn!(context, "Message not encrypted.",);
        false
    } else if let Some(expected_fingerprint) = expected_fingerprint {
        if !mimeparser.signatures.contains(expected_fingerprint) {
            warn!(
                context,
                "Message does not match expected fingerprint {}.", expected_fingerprint,
            );
            false
        } else {
            true
        }
    } else {
        warn!(context, "Fingerprint for comparison missing.");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_std::prelude::*;

    use crate::chat;
    use crate::chat::ProtectionStatus;
    use crate::events::Event;
    use crate::peerstate::Peerstate;
    use crate::test_utils::TestContext;

    #[async_std::test]
    async fn test_setup_contact() {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Setup JoinerProgress sinks.
        let (joiner_progress_tx, joiner_progress_rx) = async_std::channel::bounded(100);
        bob.add_event_sink(move |event: Event| {
            let joiner_progress_tx = joiner_progress_tx.clone();
            async move {
                if let EventType::SecurejoinJoinerProgress { .. } = event.typ {
                    joiner_progress_tx.try_send(event).unwrap();
                }
            }
        })
        .await;

        // Step 1: Generate QR-code, ChatId(0) indicates setup-contact
        let qr = dc_get_securejoin_qr(&alice.ctx, None).await.unwrap();

        // Step 2: Bob scans QR-code, sends vc-request
        dc_join_securejoin(&bob.ctx, &qr).await.unwrap();

        let sent = bob.pop_sent_msg().await;
        assert!(!bob.ctx.has_ongoing().await);
        assert_eq!(sent.recipient(), "alice@example.com".parse().unwrap());
        let msg = alice.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        assert_eq!(msg.get(HeaderDef::SecureJoin).unwrap(), "vc-request");
        assert!(msg.get(HeaderDef::SecureJoinInvitenumber).is_some());

        // Step 3: Alice receives vc-request, sends vc-auth-required
        alice.recv_msg(&sent).await;

        let sent = alice.pop_sent_msg().await;
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(msg.get(HeaderDef::SecureJoin).unwrap(), "vc-auth-required");

        // Step 4: Bob receives vc-auth-required, sends vc-request-with-auth
        bob.recv_msg(&sent).await;

        // Check Bob emitted the JoinerProgress event.
        {
            let evt = joiner_progress_rx
                .recv()
                .timeout(Duration::from_secs(10))
                .await
                .expect("timeout waiting for JoinerProgress event")
                .expect("missing JoinerProgress event");
            match evt.typ {
                EventType::SecurejoinJoinerProgress {
                    contact_id,
                    progress,
                } => {
                    let alice_contact_id =
                        Contact::lookup_id_by_addr(&bob.ctx, "alice@example.com", Origin::Unknown)
                            .await
                            .expect("Error looking up contact")
                            .expect("Contact not found");
                    assert_eq!(contact_id, alice_contact_id);
                    assert_eq!(progress, 400);
                }
                _ => panic!("Wrong event type"),
            }
        }

        // Check Bob sent the right message.
        let sent = bob.pop_sent_msg().await;
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vc-request-with-auth"
        );
        assert!(msg.get(HeaderDef::SecureJoinAuth).is_some());
        let bob_fp = SignedPublicKey::load_self(&bob.ctx)
            .await
            .unwrap()
            .fingerprint();
        assert_eq!(
            *msg.get(HeaderDef::SecureJoinFingerprint).unwrap(),
            bob_fp.hex()
        );

        // Alice should not yet have Bob verified
        let contact_bob_id =
            Contact::lookup_id_by_addr(&alice.ctx, "bob@example.net", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_bob = Contact::load_from_db(&alice.ctx, contact_bob_id)
            .await
            .unwrap();
        assert_eq!(
            contact_bob.is_verified(&alice.ctx).await,
            VerifiedStatus::Unverified
        );

        // Step 5+6: Alice receives vc-request-with-auth, sends vc-contact-confirm
        alice.recv_msg(&sent).await;
        assert_eq!(
            contact_bob.is_verified(&alice.ctx).await,
            VerifiedStatus::BidirectVerified
        );

        // Check Alice got the verified message in her 1:1 chat.
        {
            let chat = alice.create_chat(&bob).await;
            let msg_id = chat::get_chat_msgs(&alice.ctx, chat.get_id(), 0x1, None)
                .await
                .unwrap()
                .into_iter()
                .filter_map(|item| match item {
                    chat::ChatItem::Message { msg_id } => Some(msg_id),
                    _ => None,
                })
                .max()
                .expect("No messages in Alice's 1:1 chat");
            let msg = Message::load_from_db(&alice.ctx, msg_id).await.unwrap();
            assert!(msg.is_info());
            let text = msg.get_text().unwrap();
            assert!(text.contains("bob@example.net verified"));
        }

        // Check Alice sent the right message to Bob.
        let sent = alice.pop_sent_msg().await;
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vc-contact-confirm"
        );

        // Bob should not yet have Alice verified
        let contact_alice_id =
            Contact::lookup_id_by_addr(&bob.ctx, "alice@example.com", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_alice = Contact::load_from_db(&bob.ctx, contact_alice_id)
            .await
            .unwrap();
        assert_eq!(
            contact_bob.is_verified(&bob.ctx).await,
            VerifiedStatus::Unverified
        );

        // Step 7: Bob receives vc-contact-confirm, sends vc-contact-confirm-received
        bob.recv_msg(&sent).await;
        assert_eq!(
            contact_alice.is_verified(&bob.ctx).await,
            VerifiedStatus::BidirectVerified
        );

        // Check Bob got the verified message in his 1:1 chat.
        {
            let chat = bob.create_chat(&alice).await;
            let msg_id = chat::get_chat_msgs(&bob.ctx, chat.get_id(), 0x1, None)
                .await
                .unwrap()
                .into_iter()
                .filter_map(|item| match item {
                    chat::ChatItem::Message { msg_id } => Some(msg_id),
                    _ => None,
                })
                .max()
                .expect("No messages in Bob's 1:1 chat");
            let msg = Message::load_from_db(&bob.ctx, msg_id).await.unwrap();
            assert!(msg.is_info());
            let text = msg.get_text().unwrap();
            assert!(text.contains("alice@example.com verified"));
        }

        // Check Bob sent the final message
        let sent = bob.pop_sent_msg().await;
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vc-contact-confirm-received"
        );
    }

    #[async_std::test]
    async fn test_setup_contact_bad_qr() {
        let bob = TestContext::new_bob().await;
        let ret = dc_join_securejoin(&bob.ctx, "not a qr code").await;
        assert!(matches!(ret, Err(JoinError::QrCode(_))));
    }

    #[async_std::test]
    async fn test_setup_contact_bob_knows_alice() {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Setup JoinerProgress sinks.
        let (joiner_progress_tx, joiner_progress_rx) = async_std::channel::bounded(100);
        bob.add_event_sink(move |event: Event| {
            let joiner_progress_tx = joiner_progress_tx.clone();
            async move {
                if let EventType::SecurejoinJoinerProgress { .. } = event.typ {
                    joiner_progress_tx.try_send(event).unwrap();
                }
            }
        })
        .await;

        // Ensure Bob knows Alice_FP
        let alice_pubkey = SignedPublicKey::load_self(&alice.ctx).await.unwrap();
        let peerstate = Peerstate {
            addr: "alice@example.com".into(),
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
            to_save: Some(ToSave::All),
            fingerprint_changed: false,
        };
        peerstate.save_to_db(&bob.ctx.sql, true).await.unwrap();

        // Step 1: Generate QR-code, ChatId(0) indicates setup-contact
        let qr = dc_get_securejoin_qr(&alice.ctx, None).await.unwrap();

        // Step 2+4: Bob scans QR-code, sends vc-request-with-auth, skipping vc-request
        dc_join_securejoin(&bob.ctx, &qr).await.unwrap();

        // Check Bob emitted the JoinerProgress event.
        {
            let evt = joiner_progress_rx
                .recv()
                .timeout(Duration::from_secs(10))
                .await
                .expect("timeout waiting for JoinerProgress event")
                .expect("missing JoinerProgress event");
            match evt.typ {
                EventType::SecurejoinJoinerProgress {
                    contact_id,
                    progress,
                } => {
                    let alice_contact_id =
                        Contact::lookup_id_by_addr(&bob.ctx, "alice@example.com", Origin::Unknown)
                            .await
                            .expect("Error looking up contact")
                            .expect("Contact not found");
                    assert_eq!(contact_id, alice_contact_id);
                    assert_eq!(progress, 400);
                }
                _ => panic!("Wrong event type"),
            }
        }
        assert!(!bob.ctx.has_ongoing().await);

        // Check Bob sent the right handshake message.
        let sent = bob.pop_sent_msg().await;
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vc-request-with-auth"
        );
        assert!(msg.get(HeaderDef::SecureJoinAuth).is_some());
        let bob_fp = SignedPublicKey::load_self(&bob.ctx)
            .await
            .unwrap()
            .fingerprint();
        assert_eq!(
            *msg.get(HeaderDef::SecureJoinFingerprint).unwrap(),
            bob_fp.hex()
        );

        // Alice should not yet have Bob verified
        let (contact_bob_id, _modified) = Contact::add_or_lookup(
            &alice.ctx,
            "Bob",
            "bob@example.net",
            Origin::ManuallyCreated,
        )
        .await
        .unwrap();
        let contact_bob = Contact::load_from_db(&alice.ctx, contact_bob_id)
            .await
            .unwrap();
        assert_eq!(
            contact_bob.is_verified(&alice.ctx).await,
            VerifiedStatus::Unverified
        );

        // Step 5+6: Alice receives vc-request-with-auth, sends vc-contact-confirm
        alice.recv_msg(&sent).await;
        assert_eq!(
            contact_bob.is_verified(&alice.ctx).await,
            VerifiedStatus::BidirectVerified
        );

        let sent = alice.pop_sent_msg().await;
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vc-contact-confirm"
        );

        // Bob should not yet have Alice verified
        let contact_alice_id =
            Contact::lookup_id_by_addr(&bob.ctx, "alice@example.com", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_alice = Contact::load_from_db(&bob.ctx, contact_alice_id)
            .await
            .unwrap();
        assert_eq!(
            contact_bob.is_verified(&bob.ctx).await,
            VerifiedStatus::Unverified
        );

        // Step 7: Bob receives vc-contact-confirm, sends vc-contact-confirm-received
        bob.recv_msg(&sent).await;
        assert_eq!(
            contact_alice.is_verified(&bob.ctx).await,
            VerifiedStatus::BidirectVerified
        );

        let sent = bob.pop_sent_msg().await;
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vc-contact-confirm-received"
        );
    }

    #[async_std::test]
    async fn test_secure_join() {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Setup JoinerProgress sinks.
        let (joiner_progress_tx, joiner_progress_rx) = async_std::channel::bounded(100);
        bob.add_event_sink(move |event: Event| {
            let joiner_progress_tx = joiner_progress_tx.clone();
            async move {
                if let EventType::SecurejoinJoinerProgress { .. } = event.typ {
                    joiner_progress_tx.try_send(event).unwrap();
                }
            }
        })
        .await;

        let chatid = chat::create_group_chat(&alice.ctx, ProtectionStatus::Protected, "the chat")
            .await
            .unwrap();

        // Step 1: Generate QR-code, secure-join implied by chatid
        let qr = dc_get_securejoin_qr(&alice.ctx, Some(chatid))
            .await
            .unwrap();

        // Step 2: Bob scans QR-code, sends vg-request; blocks on ongoing process
        let joiner = {
            let qr = qr.clone();
            let ctx = bob.ctx.clone();
            async_std::task::spawn(async move { dc_join_securejoin(&ctx, &qr).await.unwrap() })
        };

        let sent = bob.pop_sent_msg().await;
        assert!(bob.ctx.has_ongoing().await);
        assert_eq!(sent.recipient(), "alice@example.com".parse().unwrap());
        let msg = alice.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        assert_eq!(msg.get(HeaderDef::SecureJoin).unwrap(), "vg-request");
        assert!(msg.get(HeaderDef::SecureJoinInvitenumber).is_some());

        // Step 3: Alice receives vg-request, sends vg-auth-required
        alice.recv_msg(&sent).await;

        let sent = alice.pop_sent_msg().await;
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(msg.get(HeaderDef::SecureJoin).unwrap(), "vg-auth-required");

        // Step 4: Bob receives vg-auth-required, sends vg-request-with-auth
        bob.recv_msg(&sent).await;
        let sent = bob.pop_sent_msg().await;

        // Check Bob emitted the JoinerProgress event.
        {
            let evt = joiner_progress_rx
                .recv()
                .timeout(Duration::from_secs(10))
                .await
                .expect("timeout waiting for JoinerProgress event")
                .expect("missing JoinerProgress event");
            match evt.typ {
                EventType::SecurejoinJoinerProgress {
                    contact_id,
                    progress,
                } => {
                    let alice_contact_id =
                        Contact::lookup_id_by_addr(&bob.ctx, "alice@example.com", Origin::Unknown)
                            .await
                            .expect("Error looking up contact")
                            .expect("Contact not found");
                    assert_eq!(contact_id, alice_contact_id);
                    assert_eq!(progress, 400);
                }
                _ => panic!("Wrong event type"),
            }
        }

        // Check Bob sent the right handshake message.
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vg-request-with-auth"
        );
        assert!(msg.get(HeaderDef::SecureJoinAuth).is_some());
        let bob_fp = SignedPublicKey::load_self(&bob.ctx)
            .await
            .unwrap()
            .fingerprint();
        assert_eq!(
            *msg.get(HeaderDef::SecureJoinFingerprint).unwrap(),
            bob_fp.hex()
        );

        // Alice should not yet have Bob verified
        let contact_bob_id =
            Contact::lookup_id_by_addr(&alice.ctx, "bob@example.net", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_bob = Contact::load_from_db(&alice.ctx, contact_bob_id)
            .await
            .unwrap();
        assert_eq!(
            contact_bob.is_verified(&alice.ctx).await,
            VerifiedStatus::Unverified
        );

        // Step 5+6: Alice receives vg-request-with-auth, sends vg-member-added
        alice.recv_msg(&sent).await;
        assert_eq!(
            contact_bob.is_verified(&alice.ctx).await,
            VerifiedStatus::BidirectVerified
        );

        let sent = alice.pop_sent_msg().await;
        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(msg.get(HeaderDef::SecureJoin).unwrap(), "vg-member-added");

        // Bob should not yet have Alice verified
        let contact_alice_id =
            Contact::lookup_id_by_addr(&bob.ctx, "alice@example.com", Origin::Unknown)
                .await
                .expect("Error looking up contact")
                .expect("Contact not found");
        let contact_alice = Contact::load_from_db(&bob.ctx, contact_alice_id)
            .await
            .unwrap();
        assert_eq!(
            contact_bob.is_verified(&bob.ctx).await,
            VerifiedStatus::Unverified
        );

        // Step 7: Bob receives vg-member-added, sends vg-member-added-received
        bob.recv_msg(&sent).await;
        assert_eq!(
            contact_alice.is_verified(&bob.ctx).await,
            VerifiedStatus::BidirectVerified
        );

        let sent = bob.pop_sent_msg().await;
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        assert_eq!(
            msg.get(HeaderDef::SecureJoin).unwrap(),
            "vg-member-added-received"
        );

        let bob_chatid = joiner.await;
        let bob_chat = Chat::load_from_db(&bob.ctx, bob_chatid).await.unwrap();
        assert!(bob_chat.is_protected());
        assert!(!bob.ctx.has_ongoing().await)
    }
}

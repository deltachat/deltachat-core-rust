//! Verified contact protocol implementation as [specified by countermitm project](https://countermitm.readthedocs.io/en/stable/new.html#setup-contact-protocol)

use std::convert::TryFrom;
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Error, Result};
use async_std::sync::{Mutex, MutexGuard};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::aheader::EncryptPreference;
use crate::chat::{self, Chat, ChatId};
use crate::config::Config;
use crate::constants::{Blocked, Viewtype, DC_CONTACT_ID_LAST_SPECIAL};
use crate::contact::{Contact, Origin, VerifiedStatus};
use crate::context::Context;
use crate::e2ee::ensure_secret_key_exists;
use crate::events::EventType;
use crate::headerdef::HeaderDef;
use crate::key::{self, DcKey, Fingerprint, FingerprintError, SignedPublicKey};
use crate::lot::{Lot, LotState};
use crate::message::Message;
use crate::mimeparser::{MimeMessage, SystemMessage};
use crate::param::Param;
use crate::peerstate::{Peerstate, PeerstateKeyType, PeerstateVerifiedStatus, ToSave};
use crate::qr::check_qr;
use crate::sql;
use crate::stock::StockMessage;
use crate::token;

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
/// database.  For Bob this state is only carried live on the [Context] in this struct.
#[derive(Debug, Default)]
pub(crate) struct Bob {
    inner: Mutex<Option<BobState>>,
}

impl Bob {
    /// Starts the securejoin protocol with the QR `invite`.
    ///
    /// This will try to start the securejoin protocol for the given QR `invite`.  If it
    /// succeeded the protocol state will be tracked in `self`.
    ///
    /// This function takes care of starting the "ongoing" mechanism if required and
    /// handling errors while starting the protocol.
    async fn start_protocol(&self, context: &Context, invite: QrInvite) -> Result<(), JoinError> {
        let mut guard = self.inner.lock().await;
        if guard.is_some() {
            return Err(JoinError::AlreadyRunning);
        }
        let mut did_alloc_ongoing = false;
        if let QrInvite::Group { .. } = invite {
            if context.alloc_ongoing().await.is_err() {
                return Err(JoinError::OngoingRunning);
            }
            did_alloc_ongoing = true;
        }
        match BobState::start_protocol(context, invite).await {
            Ok((state, stage)) => {
                if matches!(stage, BobHandshakeStage::RequestWithAuthSent) {
                    joiner_progress!(context, state.invite.contact_id(), 400);
                }
                *guard = Some(state);
                Ok(())
            }
            Err(err) => {
                if did_alloc_ongoing {
                    context.stop_ongoing().await;
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

/// A handle to work with the [`BobState`] of Bob's securejoin protocol.
///
/// This handle can only be created for when an underlying [`BobState`] exists.  It keeps
/// open a lock which guarantees unique access to the state and this struct must be dropped
/// to return the lock.
struct BobStateHandle<'a> {
    guard: MutexGuard<'a, Option<BobState>>,
    clear_state_on_drop: bool,
}

impl<'a> BobStateHandle<'a> {
    /// Creates a new instance, upholding the guarantee that [`BobState`] must exist.
    fn from_guard(guard: MutexGuard<'a, Option<BobState>>) -> Option<Self> {
        match *guard {
            Some(_) => Some(Self {
                guard,
                clear_state_on_drop: false,
            }),
            None => None,
        }
    }

    /// Returns the [`ChatId`] of the 1:1 chat with the inviter (Alice).
    pub fn chat_id(&self) -> Result<ChatId> {
        match *self.guard {
            Some(ref bobstate) => Ok(bobstate.chat_id),
            None => Err(Error::msg("Invalid BobStateHandle state")),
        }
    }

    /// Returns a reference to the [`QrInvite`] of the joiner process.
    pub fn invite(&self) -> Result<&QrInvite> {
        match *self.guard {
            Some(ref bobstate) => Ok(&bobstate.invite),
            None => Err(Error::msg("Invalid BobStateHandle state")),
        }
    }

    /// Handles the given message for the securejoin handshake for Bob.
    ///
    /// This proxies to [`BobState::handle_message`] and makes sure to clear the state when
    /// the protocol state is terminal.  It returns `Some` if the message successfully
    /// advanced the state of the protocol state machine, `None` otherwise.
    pub async fn handle_message(
        &mut self,
        context: &Context,
        mime_message: &MimeMessage,
    ) -> Option<BobHandshakeStage> {
        info!(context, "Handling securejoin message for BobStateHandle");
        match *self.guard {
            Some(ref mut bobstate) => match bobstate.handle_message(context, mime_message).await {
                Ok(Some(stage)) => {
                    if matches!(stage,
                                BobHandshakeStage::Completed
                                | BobHandshakeStage::Terminated(_))
                    {
                        self.finish_protocol(context).await;
                    }
                    Some(stage)
                }
                Ok(None) => None,
                Err(_) => {
                    self.finish_protocol(context).await;
                    None
                }
            },
            None => None,
        }
    }

    /// Marks the bob handshake as finished.
    ///
    /// This will clear the state on [`Context::bob`] once this handle is dropped, allowing
    /// a new handshake to be started from [`Bob`].
    ///
    /// Note that the state is only cleared on Drop since otherwise the invariant that the
    /// state is always cosistent is violated.  However the "ongoing" prococess is released
    /// here a little bit earlier as this requires access to the Context, which we do not
    /// have on Drop (Drop can not run asynchronous code).
    async fn finish_protocol(&mut self, context: &Context) {
        info!(context, "Finishing securejoin handshake protocol for Bob");
        self.clear_state_on_drop = true;
        if let Some(ref bobstate) = *self.guard {
            if let QrInvite::Group { .. } = bobstate.invite {
                context.stop_ongoing().await;
            }
        }
    }
}

impl<'a> Drop for BobStateHandle<'a> {
    fn drop(&mut self) {
        if self.clear_state_on_drop {
            self.guard.take();
        }
    }
}

/// The securejoin state kept in-memory while Bob is joining.
///
/// This is currently stored in [`Bob`] which is stored on the [`Context`], thus Bob can
/// only run one securejoin joiner protocol at a time.
///
/// This purposefully has nothing optional, the state is always fully valid.  See
/// [`Bob::state`] to get access to this state.
///
/// # Conducing the securejoin handshake
///
/// The methods on this struct allow you to interact with the state and thus conduct the
/// securejoin handshake for Bob.  The methods **only concern themselves** with the protocol
/// state and explicitly avoid doing performing any user interactions required by
/// securejoin.  This simplifies the concerns and logic required in both the callers and in
/// the state management.  The return values can be used to understand what user
/// interactions need to happen.
#[derive(Debug)]
struct BobState {
    /// The QR Invite code.
    invite: QrInvite,
    /// The next expected message from Alice.
    next: SecureJoinStep,
    /// The [ChatId] of the 1:1 chat with Alice, matching [QrInvite::contact].
    chat_id: ChatId,
}

impl BobState {
    /// Starts the securejoin protocol and creates a new [`BobState`].
    ///
    /// # Bob - the joiner's side
    /// ## Step 2 in the "Setup Contact protocol", section 2.1 of countermitm 0.10.0
    async fn start_protocol(
        context: &Context,
        invite: QrInvite,
    ) -> Result<(Self, BobHandshakeStage), JoinError> {
        let chat_id = chat::create_by_contact_id(context, invite.contact_id())
            .await
            .map_err(JoinError::UnknownContact)?;
        if fingerprint_equals_sender(context, invite.fingerprint(), chat_id).await {
            // The scanned fingerprint matches Alice's key, we can proceed to step 4b.
            info!(context, "Taking securejoin protocol shortcut");
            let state = Self {
                invite,
                next: SecureJoinStep::ContactConfirm,
                chat_id,
            };
            state
                .send_handshake_message(context, BobHandshakeMsg::RequestWithAuth)
                .await?;
            Ok((state, BobHandshakeStage::RequestWithAuthSent))
        } else {
            let state = Self {
                invite,
                next: SecureJoinStep::AuthRequired,
                chat_id,
            };
            state
                .send_handshake_message(context, BobHandshakeMsg::Request)
                .await?;
            Ok((state, BobHandshakeStage::RequestSent))
        }
    }

    /// Handles the given message for the securejoin handshake for Bob.
    ///
    /// If the message was not used for this handshake `None` is returned, otherwise the new
    /// stage is returned.  Once [`BobHandshakeStage::Completed`] or
    /// [`BobHandshakeStage::Terminated`] are reached this [`BobState`] should be destroyed,
    /// further calling it will just result in the messages being unused by this handshake.
    ///
    /// # Errors
    ///
    /// Under normal operation this should never return an error, regardless of what kind of
    /// message it is called with.  Any errors therefore should be treated as fatal internal
    /// errors and this entire [`BobState`] should be thrown away as the state machine can
    /// no longer be considered consistent.
    async fn handle_message(
        &mut self,
        context: &Context,
        mime_message: &MimeMessage,
    ) -> Result<Option<BobHandshakeStage>> {
        let step = match mime_message.get(HeaderDef::SecureJoin) {
            Some(step) => step,
            None => {
                warn!(
                    context,
                    "Message has no Secure-Join header: {}",
                    mime_message.get_rfc724_mid().unwrap_or_default()
                );
                return Ok(None);
            }
        };
        if !self.is_msg_expected(context, step.as_str()) {
            info!(context, "{} message out of sync for BobState", step);
            return Ok(None);
        }
        match step.as_str() {
            "vg-auth-required" | "vc-auth-required" => {
                self.step_auth_required(context, mime_message).await
            }
            "vg-member-added" | "vc-contact-confirm" => {
                self.step_contact_confirm(context, mime_message).await
            }
            _ => {
                warn!(context, "Invalid step for BobState: {}", step);
                Ok(None)
            }
        }
    }

    /// Returns `true` if the message is expected according to the protocol.
    fn is_msg_expected(&self, context: &Context, step: &str) -> bool {
        let variant_matches = match self.invite {
            QrInvite::Contact { .. } => step.starts_with("vc-"),
            QrInvite::Group { .. } => step.starts_with("vg-"),
        };
        let step_matches = self.next.matches(context, step);
        variant_matches && step_matches
    }

    /// Handles a *vc-auth-required* or *vg-auth-required* message.
    ///
    /// # Bob - the joiner's side
    /// ## Step 4 in the "Setup Contact protocol", section 2.1 of countermitm 0.10.0
    async fn step_auth_required(
        &mut self,
        context: &Context,
        mime_message: &MimeMessage,
    ) -> Result<Option<BobHandshakeStage>> {
        info!(
            context,
            "Bob Step 4 - handling vc-auth-require/vg-auth-required message"
        );
        if !encrypted_and_signed(context, mime_message, Some(self.invite.fingerprint())) {
            let reason = if mime_message.was_encrypted() {
                "Valid signature missing"
            } else {
                "Required encryption missing"
            };
            self.next = SecureJoinStep::Terminated;
            return Ok(Some(BobHandshakeStage::Terminated(reason)));
        }
        if !fingerprint_equals_sender(context, self.invite.fingerprint(), self.chat_id).await {
            self.next = SecureJoinStep::Terminated;
            return Ok(Some(BobHandshakeStage::Terminated("Fingerprint mismatch")));
        }
        info!(context, "Fingerprint verified.",);
        self.next = SecureJoinStep::ContactConfirm;
        self.send_handshake_message(context, BobHandshakeMsg::RequestWithAuth)
            .await?;
        Ok(Some(BobHandshakeStage::RequestWithAuthSent))
    }

    /// Handles a *vc-contact-confirm* or *vg-member-added* message.
    ///
    /// # Bob - the joiner's side
    /// ## Step 7 in the "Setup Contact protocol", section 2.1 of countermitm 0.10.0
    ///
    /// This deviates from the protocol by also sending a confirmation message in response
    /// to the *vc-contact-confirm* message.  This has no specific value to the protocol and
    /// is only done out of symmerty with *vg-member-added* handling.
    async fn step_contact_confirm(
        &mut self,
        context: &Context,
        mime_message: &MimeMessage,
    ) -> Result<Option<BobHandshakeStage>> {
        info!(
            context,
            "Bob Step 7 - handling vc-contact-confirm/vg-member-added message"
        );
        let vg_expect_encrypted = match self.invite {
            QrInvite::Contact { .. } => {
                // setup-contact is always encrypted
                true
            }
            QrInvite::Group { ref grpid, .. } => {
                // This is buggy, is_verified_group will always be
                // false since the group is created by receive_imf for
                // the very handshake message we're handling now.  But
                // only after we have returned.  It does not impact
                // the security invariants of secure-join however.
                let (_, is_verified_group, _) = chat::get_chat_id_by_grpid(context, grpid)
                    .await
                    .unwrap_or((ChatId::new(0), false, Blocked::Not));
                // when joining a non-verified group
                // the vg-member-added message may be unencrypted
                // when not all group members have keys or prefer encryption.
                // So only expect encryption if this is a verified group
                is_verified_group
            }
        };
        if vg_expect_encrypted
            && !encrypted_and_signed(context, mime_message, Some(self.invite.fingerprint()))
        {
            self.next = SecureJoinStep::Terminated;
            return Ok(Some(BobHandshakeStage::Terminated(
                "Contact confirm message not encrypted",
            )));
        }
        mark_peer_as_verified(context, self.invite.fingerprint()).await?;
        Contact::scaleup_origin_by_id(context, self.invite.contact_id(), Origin::SecurejoinJoined)
            .await;
        emit_event!(context, EventType::ContactsChanged(None));

        if let QrInvite::Group { .. } = self.invite {
            let member_added = mime_message
                .get(HeaderDef::ChatGroupMemberAdded)
                .map(|s| s.as_str())
                .ok_or_else(|| Error::msg("Missing Chat-Group-Member-Added header"))?;
            if !context.is_self_addr(member_added).await? {
                info!(context, "Message belongs to a different handshake (scaled up contact anyway to allow creation of group).");
                return Ok(None);
            }
        }

        self.send_handshake_message(context, BobHandshakeMsg::ContactConfirmReceived)
            .await
            .map_err(|_| {
                warn!(
                    context,
                    "Failed to send vc-contact-confirm-received/vg-member-added-received"
                );
            })
            // This is not an error affecting the protocol outcome.
            .ok();

        self.next = SecureJoinStep::Completed;
        Ok(Some(BobHandshakeStage::Completed))
    }

    /// Sends the requested handshake message to Alice.
    ///
    /// This takes care of adding the required headers for the step.
    async fn send_handshake_message(
        &self,
        context: &Context,
        step: BobHandshakeMsg,
    ) -> Result<(), SendMsgError> {
        let mut msg = Message::default();
        msg.viewtype = Viewtype::Text;
        msg.text = Some(step.body_text(&self.invite));
        msg.hidden = true;
        msg.param.set_cmd(SystemMessage::SecurejoinMessage);

        // Sends the step in Secure-Join header.
        msg.param
            .set(Param::Arg, step.securejoin_header(&self.invite));

        match step {
            BobHandshakeMsg::Request => {
                // Sends the Secure-Join-Invitenumber header in mimefactory.rs.
                msg.param.set(Param::Arg2, self.invite.invitenumber());
                msg.param.set_int(Param::ForcePlaintext, 1);
            }
            BobHandshakeMsg::RequestWithAuth => {
                // Sends the Secure-Join-Auth header in mimefactory.rs.
                msg.param.set(Param::Arg2, self.invite.authcode());
                msg.param.set_int(Param::GuaranteeE2ee, 1);
            }
            BobHandshakeMsg::ContactConfirmReceived => {
                msg.param.set_int(Param::GuaranteeE2ee, 1);
            }
        };

        // Sends our own fingerprint in the Secure-Join-Fingerprint header.
        let bob_fp = SignedPublicKey::load_self(context).await?.fingerprint();
        msg.param.set(Param::Arg3, bob_fp.hex());

        // Sends the grpid in the Secure-Join-Group header.
        if let QrInvite::Group { ref grpid, .. } = self.invite {
            msg.param.set(Param::Arg4, grpid);
        }

        chat::send_msg(context, self.chat_id, &mut msg).await?;
        Ok(())
    }
}

/// The stage of the [`BobState`] securejoin handshake protocol state machine.
///
/// This does not concern itself with user interactions, only represents what happened to
/// the protocol state machine from handling this message.
#[derive(Clone, Copy, Debug, Display)]
enum BobHandshakeStage {
    /// Step 2 completed: (vc|vg)-request message sent.
    ///
    /// Note that this is only ever returned by [`BobState::start_protocol`] and never by
    /// [`BobState::handle_message`].
    RequestSent,
    /// Step 4 completed: (vc|vg)-request-with-auth message sent.
    RequestWithAuthSent,
    /// The protocol completed successfully.
    Completed,
    /// The protocol prematurely terminated with given reason.
    Terminated(&'static str),
}

/// Identifies the SecureJoin handshake messages Bob can send.
enum BobHandshakeMsg {
    /// vc-request or vg-request
    Request,
    /// vc-request-with-auth or vg-request-with-auth
    RequestWithAuth,
    /// vc-contact-confirm-received or vg-member-added-received
    ContactConfirmReceived,
}

impl BobHandshakeMsg {
    /// Returns the text to send in the body of the handshake message.
    ///
    /// This text has no significance to the protocol, but would be visible if users see
    /// this email message directly, e.g. when accessing their email without using
    /// DeltaChat.
    fn body_text(&self, invite: &QrInvite) -> String {
        format!("Secure-Join: {}", self.securejoin_header(invite))
    }

    /// Returns the `Secure-Join` header value.
    ///
    /// This identifies the step this message is sending information about.  Most protocol
    /// steps include additional information into other headers, see
    /// [`BobState::send_handshake_message`] for these.
    fn securejoin_header(&self, invite: &QrInvite) -> &'static str {
        match self {
            Self::Request => match invite {
                QrInvite::Contact { .. } => "vc-request",
                QrInvite::Group { .. } => "vg-request",
            },
            Self::RequestWithAuth => match invite {
                QrInvite::Contact { .. } => "vc-request-with-auth",
                QrInvite::Group { .. } => "vg-request-with-auth",
            },
            Self::ContactConfirmReceived => match invite {
                QrInvite::Contact { .. } => "vc-contact-confirm-received",
                QrInvite::Group { .. } => "vg-member-added-received",
            },
        }
    }
}

/// Represents the data from a QR-code scan.
///
/// There are methods to conveniently access fields present in both variants.
#[derive(Debug, Clone)]
enum QrInvite {
    Contact {
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
    Group {
        contact_id: u32,
        fingerprint: Fingerprint,
        name: String,
        grpid: String,
        invitenumber: String,
        authcode: String,
    },
}

impl QrInvite {
    /// The contact ID of the inviter.
    ///
    /// The actual QR-code contains a URL-encoded email address, but upon scanning this is
    /// currently translated to a contact ID.
    fn contact_id(&self) -> u32 {
        match self {
            Self::Contact { contact_id, .. } | Self::Group { contact_id, .. } => *contact_id,
        }
    }

    /// The fingerprint of the inviter.
    fn fingerprint(&self) -> &Fingerprint {
        match self {
            Self::Contact { fingerprint, .. } | Self::Group { fingerprint, .. } => &fingerprint,
        }
    }

    /// The `INVITENUMBER` of the setup-contact/secure-join protocol.
    fn invitenumber(&self) -> &str {
        match self {
            Self::Contact { invitenumber, .. } | Self::Group { invitenumber, .. } => &invitenumber,
        }
    }

    /// The `AUTH` code of the setup-contact/secure-join protocol.
    fn authcode(&self) -> &str {
        match self {
            Self::Contact { authcode, .. } | Self::Group { authcode, .. } => &authcode,
        }
    }
}

impl TryFrom<Lot> for QrInvite {
    type Error = QrError;

    fn try_from(lot: Lot) -> Result<Self, Self::Error> {
        if lot.state != LotState::QrAskVerifyContact && lot.state != LotState::QrAskVerifyGroup {
            return Err(QrError::UnsupportedProtocol);
        }
        let fingerprint = lot.fingerprint.ok_or(QrError::MissingFingerprint)?;
        let invitenumber = lot.invitenumber.ok_or(QrError::MissingInviteNumber)?;
        let authcode = lot.auth.ok_or(QrError::MissingAuthCode)?;
        match lot.state {
            LotState::QrAskVerifyContact => Ok(QrInvite::Contact {
                contact_id: lot.id,
                fingerprint,
                invitenumber,
                authcode,
            }),
            LotState::QrAskVerifyGroup => Ok(QrInvite::Group {
                contact_id: lot.id,
                fingerprint,
                name: lot.text1.ok_or(QrError::MissingGroupName)?,
                grpid: lot.text2.ok_or(QrError::MissingGroupId)?,
                invitenumber,
                authcode,
            }),
            _ => Err(QrError::UnsupportedProtocol),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QrError {
    #[error("Unsupported protocol in QR-code")]
    UnsupportedProtocol,
    #[error("Failed to read fingerprint")]
    InvalidFingerprint(#[from] FingerprintError),
    #[error("Missing fingerprint")]
    MissingFingerprint,
    #[error("Missing invitenumber")]
    MissingInviteNumber,
    #[error("Missing auth code")]
    MissingAuthCode,
    #[error("Missing group name")]
    MissingGroupName,
    #[error("Missing group id")]
    MissingGroupId,
}

/// The next message expected by [`BobState`] in the setup-contact/secure-join protocol.
#[derive(Debug, PartialEq)]
enum SecureJoinStep {
    /// Expecting the auth-required message.
    ///
    /// This corresponds to the `vc-auth-required` or `vg-auth-required` message of step 3d.
    AuthRequired,
    /// Expecting the contact-confirm message.
    ///
    /// This corresponds to the `vc-contact-confirm` or `vg-member-added` message of step
    /// 6b.
    ContactConfirm,
    /// The protocol terminated because of an error.
    ///
    /// The securejoin protocol terminated, this exists to ensure [`BobState`] can detect
    /// when it earlier signalled that is should be terminated.  It is an error to call with
    /// this state.
    Terminated,
    /// The protocol completed.
    ///
    /// This exists to ensure [`BobState`] can detect when it earlier signalled that it is
    /// complete.  It is an error to call with this state.
    Completed,
}

impl SecureJoinStep {
    /// Compares the legacy string representation of a step to a [`SecureJoinStep`] variant.
    fn matches(&self, context: &Context, step: &str) -> bool {
        match self {
            Self::AuthRequired => step == "vc-auth-required" || step == "vg-auth-required",
            Self::ContactConfirm => step == "vc-contact-confirm" || step == "vg-member-added",
            SecureJoinStep::Terminated => {
                warn!(context, "Terminated state for next securejoin step");
                false
            }
            SecureJoinStep::Completed => {
                warn!(context, "Complted state for next securejoin step");
                false
            }
        }
    }
}

pub async fn dc_get_securejoin_qr(context: &Context, group_chat_id: ChatId) -> Option<String> {
    /*=======================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    =======================================================*/

    ensure_secret_key_exists(context).await.ok();

    // invitenumber will be used to allow starting the handshake,
    // auth will be used to verify the fingerprint
    let invitenumber =
        token::lookup_or_new(context, token::Namespace::InviteNumber, group_chat_id).await;
    let auth = token::lookup_or_new(context, token::Namespace::Auth, group_chat_id).await;
    let self_addr = match context.get_config(Config::ConfiguredAddr).await {
        Some(addr) => addr,
        None => {
            error!(context, "Not configured, cannot generate QR code.",);
            return None;
        }
    };

    let self_name = context
        .get_config(Config::Displayname)
        .await
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

    let qr = if !group_chat_id.is_unset() {
        // parameters used: a=g=x=i=s=
        if let Ok(chat) = Chat::load_from_db(context, group_chat_id).await {
            let group_name = chat.get_name();
            let group_name_urlencoded =
                utf8_percent_encode(&group_name, NON_ALPHANUMERIC).to_string();

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
            error!(context, "Cannot get QR-code for chat-id {}", group_chat_id,);
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
    #[error("Unknown QR-code")]
    QrCode(#[from] QrError),
    #[error("A setup-contact/secure-join protocol is already running")]
    AlreadyRunning,
    #[error("An \"ongoing\" process is already running")]
    OngoingRunning,
    #[error("Failed to send handshake message")]
    SendMessage(#[from] SendMsgError),
    // Note that this can currently only occur if there is a bug in the QR/Lot code as this
    // is supposed to create a contact for us.
    #[error("Unknown contact (this is a bug)")]
    UnknownContact(#[source] anyhow::Error),
    // Note that this can only occur if we failed to create the chat correctly.
    #[error("No Chat found for group (this is a bug)")]
    MissingChat(#[source] sql::Error),
}

/// Take a scanned QR-code and do the setup-contact/join-group/invite handshake.
///
/// This is the start of the process for the joiner.  See the module and ffi documentation
/// for more details.
///
/// When **joining a group** this will start an "ongoing" process and will block until the
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
    let qr_scan = check_qr(context, &qr).await;
    let invite = QrInvite::try_from(qr_scan)?;

    context.bob.start_protocol(context, invite.clone()).await?;

    match invite {
        QrInvite::Contact { .. } => {
            // for a one-to-one-chat, the chat is already known, return the chat-id,
            // the verification runs in background
            let chat_id = chat::create_by_contact_id(context, invite.contact_id())
                .await
                .map_err(JoinError::UnknownContact)?;
            Ok(chat_id)
        }
        QrInvite::Group { ref grpid, .. } => {
            // for a group-join, wait until the secure-join is done and the group is created
            while !context.shall_stop_ongoing().await {
                async_std::task::sleep(Duration::from_millis(50)).await;
            }

            // handle_securejoin_handshake() calls Context::stop_ongoing before the group
            // chat is created (it is created after handle_securejoin_handshake() returns by
            // dc_receive_imf()).  As a hack we just wait a bit for it to appear.

            // If the protocol is aborted by Bob, this timeout will also happen.
            let start = Instant::now();
            let chatid = loop {
                {
                    match chat::get_chat_id_by_grpid(context, grpid).await {
                        Ok((chatid, _is_protected, _blocked)) => break chatid,
                        Err(err) => {
                            if start.elapsed() > Duration::from_secs(7) {
                                return Err(JoinError::MissingChat(err));
                            }
                        }
                    }
                }
                async_std::task::sleep(Duration::from_millis(50)).await;
            };
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

impl From<key::Error> for SendMsgError {
    fn from(source: key::Error) -> Self {
        Self(anyhow::Error::new(source))
    }
}

async fn send_handshake_msg(
    context: &Context,
    contact_chat_id: ChatId,
    step: &str,
    param2: impl AsRef<str>,
    fingerprint: Option<Fingerprint>,
    grpid: impl AsRef<str>,
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
    if !param2.as_ref().is_empty() {
        msg.param.set(Param::Arg2, param2);
    }
    if let Some(fp) = fingerprint {
        msg.param.set(Param::Arg3, fp.hex());
    }
    if !grpid.as_ref().is_empty() {
        msg.param.set(Param::Arg4, grpid.as_ref());
    }
    if step == "vg-request" || step == "vc-request" {
        msg.param.set_int(Param::ForcePlaintext, 1);
    } else {
        msg.param.set_int(Param::GuaranteeE2ee, 1);
    }

    chat::send_msg(context, contact_chat_id, &mut msg).await?;
    Ok(())
}

async fn chat_id_2_contact_id(context: &Context, contact_chat_id: ChatId) -> u32 {
    if let [contact_id] = chat::get_chat_contacts(context, contact_chat_id).await[..] {
        contact_id
    } else {
        0
    }
}

async fn fingerprint_equals_sender(
    context: &Context,
    fingerprint: &Fingerprint,
    contact_chat_id: ChatId,
) -> bool {
    if let [contact_id] = chat::get_chat_contacts(context, contact_chat_id).await[..] {
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
                    return false;
                }
            };

            if let Some(peerstate) = peerstate {
                if peerstate.public_key_fingerprint.is_some()
                    && fingerprint == peerstate.public_key_fingerprint.as_ref().unwrap()
                {
                    return true;
                }
            }
        }
    }
    false
}

/// What to do with a Secure-Join handshake message after it was handled.
///
/// This status is returned to [`dc_receive_imf`] which will use it to decide what to do
/// next with this incoming setup-contact/secure-join handshake message.
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
        let (chat_id, blocked) =
            chat::create_or_lookup_by_contact_id(context, contact_id, Blocked::Not)
                .await
                .with_context(|| {
                    format!(
                        "Failed to look up or create chat for contact {}",
                        contact_id
                    )
                })?;
        if blocked != Blocked::Not {
            chat_id.unblock(context).await;
        }
        chat_id
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
            if !token::exists(context, token::Namespace::InviteNumber, &invitenumber).await {
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
                        could_not_establish_secure_connection(context, bobstate.chat_id()?, why)
                            .await;
                        Ok(HandshakeMessage::Done)
                    }
                    Some(_stage) => {
                        joiner_progress!(context, bobstate.invite()?.contact_id(), 400);
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
                    .await;
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if !encrypted_and_signed(context, mime_message, Some(&fingerprint)) {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Auth not encrypted.",
                )
                .await;
                return Ok(HandshakeMessage::Ignore);
            }
            if !fingerprint_equals_sender(context, &fingerprint, contact_chat_id).await {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on inviter-side.",
                )
                .await;
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
                    .await;
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if !token::exists(context, token::Namespace::Auth, &auth_0).await {
                could_not_establish_secure_connection(context, contact_chat_id, "Auth invalid.")
                    .await;
                return Ok(HandshakeMessage::Ignore);
            }
            if mark_peer_as_verified(context, &fingerprint).await.is_err() {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    "Fingerprint mismatch on inviter-side.",
                )
                .await;
                return Ok(HandshakeMessage::Ignore);
            }
            Contact::scaleup_origin_by_id(context, contact_id, Origin::SecurejoinInvited).await;
            info!(context, "Auth verified.",);
            secure_connection_established(context, contact_chat_id).await;
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
                match chat::get_chat_id_by_grpid(context, field_grpid).await {
                    Ok((group_chat_id, _, _)) => {
                        if let Err(err) =
                            chat::add_contact_to_chat_ex(context, group_chat_id, contact_id, true)
                                .await
                        {
                            error!(context, "failed to add contact: {}", err);
                        }
                    }
                    Err(err) => {
                        error!(context, "Chat {} not found: {}", &field_grpid, err);
                        return Err(Error::new(err)
                            .context(format!("Chat for group {} not found", &field_grpid)));
                    }
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
                        could_not_establish_secure_connection(context, bobstate.chat_id()?, why)
                            .await;
                        Ok(HandshakeMessage::Done)
                    }
                    Some(_stage) => {
                        // Can only be BobHandshakeStage::Completed
                        secure_connection_established(context, bobstate.chat_id()?).await;
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
                        return Err(Error::new(err)
                            .context(format!("Chat for group {} not found", &field_grpid)));
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
        let (chat_id, blocked) =
            chat::create_or_lookup_by_contact_id(context, contact_id, Blocked::Not)
                .await
                .with_context(|| {
                    format!(
                        "Failed to look up or create chat for contact {}",
                        contact_id
                    )
                })?;
        if blocked != Blocked::Not {
            chat_id.unblock(context).await;
        }
        chat_id
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
                .await;
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
                    .await;
                    return Ok(HandshakeMessage::Ignore);
                }
            };
            if mark_peer_as_verified(context, &fingerprint).await.is_err() {
                could_not_establish_secure_connection(
                    context,
                    contact_chat_id,
                    format!("Fingerprint mismatch on observing {}.", step).as_ref(),
                )
                .await;
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

async fn secure_connection_established(context: &Context, contact_chat_id: ChatId) {
    let contact_id: u32 = chat_id_2_contact_id(context, contact_chat_id).await;
    let contact = Contact::get_by_id(context, contact_id).await;

    let addr = if let Ok(ref contact) = contact {
        contact.get_addr()
    } else {
        "?"
    };
    let msg = context
        .stock_string_repl_str(StockMessage::ContactVerified, addr)
        .await;
    chat::add_info_msg(context, contact_chat_id, &msg).await;
    emit_event!(context, EventType::ChatModified(contact_chat_id));
    info!(context, "{}", msg);
}

async fn could_not_establish_secure_connection(
    context: &Context,
    contact_chat_id: ChatId,
    details: &str,
) {
    let contact_id = chat_id_2_contact_id(context, contact_chat_id).await;
    let contact = Contact::get_by_id(context, contact_id).await;
    let msg = context
        .stock_string_repl_str(
            StockMessage::ContactNotVerified,
            if let Ok(ref contact) = contact {
                contact.get_addr()
            } else {
                "?"
            },
        )
        .await;

    chat::add_info_msg(context, contact_chat_id, &msg).await;
    error!(context, "{} ({})", &msg, details);
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
    use crate::test_utils::{LogSink, TestContext};

    #[async_std::test]
    async fn test_setup_contact() {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Setup JoinerProgress sinks.
        let (joiner_progress_tx, joiner_progress_rx) = async_std::sync::channel(100);
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
        let qr = dc_get_securejoin_qr(&alice.ctx, ChatId::new(0))
            .await
            .unwrap();

        // Step 2: Bob scans QR-code, sends vc-request
        dc_join_securejoin(&bob.ctx, &qr).await.unwrap();

        let sent = bob.pop_sent_msg().await;
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
        let (joiner_progress_tx, joiner_progress_rx) = async_std::sync::channel(100);
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
            context: &bob.ctx,
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
        let qr = dc_get_securejoin_qr(&alice.ctx, ChatId::new(0))
            .await
            .unwrap();

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
        let (joiner_progress_tx, joiner_progress_rx) = async_std::sync::channel(100);
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
        let qr = dc_get_securejoin_qr(&alice.ctx, chatid).await.unwrap();

        // Step 2: Bob scans QR-code, sends vg-request; blocks on ongoing process
        let joiner = {
            let qr = qr.clone();
            let ctx = bob.ctx.clone();
            async_std::task::spawn(async move { dc_join_securejoin(&ctx, &qr).await.unwrap() })
        };

        let sent = bob.pop_sent_msg().await;
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
    }
}

//! Secure-Join protocol state machine for Bob, the joiner-side.
//!
//! This module contains the state machine to run the Secure-Join handshake for Bob and does
//! not do any user interaction required by the protocol.  Instead the state machine
//! provides all the information to its driver so it can perform the correct interactions.
//!
//! The [`BobState`] is only directly used to initially create it when starting the
//! protocol.  Afterwards it must be stored in a mutex and the [`BobStateHandle`] should be
//! used to work with the state.

use anyhow::{Error, Result};
use async_std::sync::MutexGuard;

use crate::chat::{self, ChatId};
use crate::constants::Viewtype;
use crate::contact::{Contact, Origin};
use crate::context::Context;
use crate::events::EventType;
use crate::headerdef::HeaderDef;
use crate::key::{DcKey, SignedPublicKey};
use crate::message::Message;
use crate::mimeparser::{MimeMessage, SystemMessage};
use crate::param::Param;

use super::qrinvite::QrInvite;
use super::{
    encrypted_and_signed, fingerprint_equals_sender, mark_peer_as_verified, JoinError, SendMsgError,
};

/// The stage of the [`BobState`] securejoin handshake protocol state machine.
///
/// This does not concern itself with user interactions, only represents what happened to
/// the protocol state machine from handling this message.
#[derive(Clone, Copy, Debug, Display)]
pub enum BobHandshakeStage {
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

/// A handle to work with the [`BobState`] of Bob's securejoin protocol.
///
/// This handle can only be created for when an underlying [`BobState`] exists.  It keeps
/// open a lock which guarantees unique access to the state and this struct must be dropped
/// to return the lock.
pub struct BobStateHandle<'a> {
    guard: MutexGuard<'a, Option<BobState>>,
    bobstate: BobState,
    clear_state_on_drop: bool,
}

impl<'a> BobStateHandle<'a> {
    /// Creates a new instance, upholding the guarantee that [`BobState`] must exist.
    pub fn from_guard(mut guard: MutexGuard<'a, Option<BobState>>) -> Option<Self> {
        guard.take().map(|bobstate| Self {
            guard,
            bobstate,
            clear_state_on_drop: false,
        })
    }

    /// Returns the [`ChatId`] of the 1:1 chat with the inviter (Alice).
    pub fn chat_id(&self) -> ChatId {
        self.bobstate.chat_id
    }

    /// Returns a reference to the [`QrInvite`] of the joiner process.
    pub fn invite(&self) -> &QrInvite {
        &self.bobstate.invite
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
        match self.bobstate.handle_message(context, mime_message).await {
            Ok(Some(stage)) => {
                if matches!(
                    stage,
                    BobHandshakeStage::Completed | BobHandshakeStage::Terminated(_)
                ) {
                    self.finish_protocol(context).await;
                }
                Some(stage)
            }
            Ok(None) => None,
            Err(err) => {
                warn!(
                    context,
                    "Error handling handshake message, aborting handshake: {}", err
                );
                self.finish_protocol(context).await;
                None
            }
        }
    }

    /// Marks the bob handshake as finished.
    ///
    /// This will clear the state on [`InnerContext::bob`] once this handle is dropped,
    /// allowing a new handshake to be started from [`Bob`].
    ///
    /// Note that the state is only cleared on Drop since otherwise the invariant that the
    /// state is always consistent is violated.  However the "ongoing" process is released
    /// here a little bit earlier as this requires access to the Context, which we do not
    /// have on Drop (Drop can not run asynchronous code).  Stopping the "ongoing" process
    /// will release [`securejoin`](super::securejoin) which in turn will finally free the
    /// ongoing process using [`Context::free_ongoing`].
    ///
    /// [`InnerContext::bob`]: crate::context::InnerContext::bob
    /// [`Bob`]: super::Bob
    async fn finish_protocol(&mut self, context: &Context) {
        info!(context, "Finishing securejoin handshake protocol for Bob");
        self.clear_state_on_drop = true;
        if let QrInvite::Group { .. } = self.bobstate.invite {
            context.stop_ongoing().await;
        }
    }
}

impl<'a> Drop for BobStateHandle<'a> {
    fn drop(&mut self) {
        if self.clear_state_on_drop {
            // The Option should already be empty because we take it out in the ctor,
            // however the typesystem doesn't guarantee this so do it again anyway.
            self.guard.take();
        } else {
            // Make sure to put back the BobState into the Option of the Mutex, it was taken
            // out by the constructor.
            self.guard.replace(self.bobstate.clone());
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
/// # Conducting the securejoin handshake
///
/// The methods on this struct allow you to interact with the state and thus conduct the
/// securejoin handshake for Bob.  The methods only concern themselves with the protocol
/// state and explicitly avoid performing any user interactions required by securejoin.
/// This simplifies the concerns and logic required in both the callers and in the state
/// management.  The return values can be used to understand what user interactions need to
/// happen.
///
/// [`Bob`]: super::Bob
/// [`Bob::state`]: super::Bob::state
#[derive(Debug, Clone)]
pub struct BobState {
    /// The QR Invite code.
    invite: QrInvite,
    /// The next expected message from Alice.
    next: SecureJoinStep,
    /// The [`ChatId`] of the 1:1 chat with Alice, matching [`QrInvite::contact_id`].
    chat_id: ChatId,
}

impl BobState {
    /// Starts the securejoin protocol and creates a new [`BobState`].
    ///
    /// # Bob - the joiner's side
    /// ## Step 2 in the "Setup Contact protocol", section 2.1 of countermitm 0.10.0
    pub async fn start_protocol(
        context: &Context,
        invite: QrInvite,
    ) -> Result<(Self, BobHandshakeStage), JoinError> {
        let chat_id = ChatId::create_for_contact(context, invite.contact_id())
            .await
            .map_err(JoinError::UnknownContact)?;
        if fingerprint_equals_sender(context, invite.fingerprint(), chat_id).await? {
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

    /// Returns the [`QrInvite`] used to create this [`BobState`].
    pub fn invite(&self) -> &QrInvite {
        &self.invite
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
        let step = match mime_message.get_header(HeaderDef::SecureJoin) {
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
        if !fingerprint_equals_sender(context, self.invite.fingerprint(), self.chat_id).await? {
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

                let is_verified_group = chat::get_chat_id_by_grpid(context, grpid)
                    .await?
                    .map_or(false, |(_chat_id, is_protected, _blocked)| is_protected);
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
            .await?;
        context.emit_event(EventType::ContactsChanged(None));

        if let QrInvite::Group { .. } = self.invite {
            let member_added = mime_message
                .get_header(HeaderDef::ChatGroupMemberAdded)
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
        let mut msg = Message {
            viewtype: Viewtype::Text,
            text: Some(step.body_text(&self.invite)),
            hidden: true,
            ..Default::default()
        };
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

/// The next message expected by [`BobState`] in the setup-contact/secure-join protocol.
#[derive(Debug, Clone, PartialEq)]
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

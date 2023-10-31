//! Secure-Join protocol state machine for Bob, the joiner-side.
//!
//! This module contains the state machine to run the Secure-Join handshake for Bob and does
//! not do any user interaction required by the protocol.  Instead the state machine
//! provides all the information to its driver so it can perform the correct interactions.
//!
//! The [`BobState`] is only directly used to initially create it when starting the
//! protocol.

use anyhow::{Error, Result};
use rusqlite::Connection;

use super::qrinvite::QrInvite;
use super::{encrypted_and_signed, fingerprint_equals_sender, mark_peer_as_verified};
use crate::chat::{self, ChatId};
use crate::contact::{Contact, Origin};
use crate::context::Context;
use crate::events::EventType;
use crate::headerdef::HeaderDef;
use crate::key::{load_self_public_key, DcKey};
use crate::message::{Message, Viewtype};
use crate::mimeparser::{MimeMessage, SystemMessage};
use crate::param::Param;
use crate::sql::Sql;

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

/// The securejoin state kept while Bob is joining.
///
/// This is stored in the database and loaded from there using [`BobState::from_db`].  To
/// create a new one use [`BobState::start_protocol`].
///
/// This purposefully has nothing optional, the state is always fully valid.  However once a
/// terminal state is reached in [`BobState::next`] the entry in the database will already
/// have been deleted.
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
    /// Database primary key.
    id: i64,
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
    /// The `chat_id` needs to be the ID of the 1:1 chat with Alice, this chat will be used
    /// to exchange the SecureJoin handshake messages as well as for showing error messages.
    ///
    /// # Bob - the joiner's side
    /// ## Step 2 in the "Setup Contact protocol", section 2.1 of countermitm 0.10.0
    ///
    /// This currently aborts any other securejoin process if any did not yet complete.  The
    /// ChatIds of the relevant 1:1 chat of any aborted handshakes are returned so that you
    /// can report the aboreted handshake in the chat.  (Yes, there can only ever be one
    /// ChatId in that Vec, the database doesn't care though.)
    pub async fn start_protocol(
        context: &Context,
        invite: QrInvite,
        chat_id: ChatId,
    ) -> Result<(Self, BobHandshakeStage, Vec<Self>)> {
        let (stage, next) =
            if fingerprint_equals_sender(context, invite.fingerprint(), invite.contact_id()).await?
            {
                // The scanned fingerprint matches Alice's key, we can proceed to step 4b.
                info!(context, "Taking securejoin protocol shortcut");
                send_handshake_message(context, &invite, chat_id, BobHandshakeMsg::RequestWithAuth)
                    .await?;
                (
                    BobHandshakeStage::RequestWithAuthSent,
                    SecureJoinStep::ContactConfirm,
                )
            } else {
                send_handshake_message(context, &invite, chat_id, BobHandshakeMsg::Request).await?;
                (BobHandshakeStage::RequestSent, SecureJoinStep::AuthRequired)
            };
        let (id, aborted_states) =
            Self::insert_new_db_entry(context, next, invite.clone(), chat_id).await?;
        let state = Self {
            id,
            invite,
            next,
            chat_id,
        };
        Ok((state, stage, aborted_states))
    }

    /// Inserts a new entry in the bobstate table, deleting all previous entries.
    ///
    /// Returns the ID of the newly inserted entry and all the aborted states.
    async fn insert_new_db_entry(
        context: &Context,
        next: SecureJoinStep,
        invite: QrInvite,
        chat_id: ChatId,
    ) -> Result<(i64, Vec<Self>)> {
        context
            .sql
            .transaction(move |transaction| {
                // We need to start a write transaction right away, so that we have the
                // database locked and no one else can write to this table while we read the
                // rows that we will delete.  So start with a dummy UPDATE.
                transaction.execute(
                    r#"UPDATE bobstate SET next_step=?;"#,
                    (SecureJoinStep::Terminated,),
                )?;
                let mut stmt = transaction.prepare("SELECT id FROM bobstate;")?;
                let mut aborted = Vec::new();
                for id in stmt.query_map((), |row| row.get::<_, i64>(0))? {
                    let id = id?;
                    let state = BobState::from_db_id(transaction, id)?;
                    aborted.push(state);
                }

                // Finally delete everything and insert new row.
                transaction.execute("DELETE FROM bobstate;", ())?;
                transaction.execute(
                    "INSERT INTO bobstate (invite, next_step, chat_id) VALUES (?, ?, ?);",
                    (invite, next, chat_id),
                )?;
                let id = transaction.last_insert_rowid();
                Ok((id, aborted))
            })
            .await
    }

    /// Load [`BobState`] from the database.
    pub async fn from_db(sql: &Sql) -> Result<Option<Self>> {
        // Because of how Self::start_protocol() updates the database we are currently
        // guaranteed to only have one row.
        sql.query_row_optional(
            "SELECT id, invite, next_step, chat_id FROM bobstate;",
            (),
            |row| {
                let s = BobState {
                    id: row.get(0)?,
                    invite: row.get(1)?,
                    next: row.get(2)?,
                    chat_id: row.get(3)?,
                };
                Ok(s)
            },
        )
        .await
    }

    fn from_db_id(connection: &Connection, id: i64) -> rusqlite::Result<Self> {
        connection.query_row(
            "SELECT invite, next_step, chat_id FROM bobstate WHERE id=?;",
            (id,),
            |row| {
                let s = BobState {
                    id,
                    invite: row.get(0)?,
                    next: row.get(1)?,
                    chat_id: row.get(2)?,
                };
                Ok(s)
            },
        )
    }

    /// Returns the [`QrInvite`] used to create this [`BobState`].
    pub fn invite(&self) -> &QrInvite {
        &self.invite
    }

    /// Returns the [`ChatId`] of the 1:1 chat with the inviter (Alice).
    pub fn alice_chat(&self) -> ChatId {
        self.chat_id
    }

    /// Updates the [`BobState::next`] field in memory and the database.
    ///
    /// If the next state is a terminal state it will remove this [`BobState`] from the
    /// database.
    ///
    /// If a user scanned a new QR code after this [`BobState`] was loaded this update will
    /// fail currently because starting a new joiner process currently kills any previously
    /// running processes.  This is a limitation which will go away in the future.
    async fn update_next(&mut self, sql: &Sql, next: SecureJoinStep) -> Result<()> {
        // TODO: write test verifying how this would fail.
        match next {
            SecureJoinStep::AuthRequired | SecureJoinStep::ContactConfirm => {
                sql.execute(
                    "UPDATE bobstate SET next_step=? WHERE id=?;",
                    (next, self.id),
                )
                .await?;
            }
            SecureJoinStep::Terminated | SecureJoinStep::Completed => {
                sql.execute("DELETE FROM bobstate WHERE id=?;", (self.id,))
                    .await?;
            }
        }
        self.next = next;
        Ok(())
    }

    /// Handles the given message for the securejoin handshake for Bob.
    ///
    /// If the message was not used for this handshake `None` is returned, otherwise the new
    /// stage is returned.  Once [`BobHandshakeStage::Completed`] or
    /// [`BobHandshakeStage::Terminated`] are reached this [`BobState`] should be destroyed,
    /// further calling it will just result in the messages being unused by this handshake.
    pub(crate) async fn handle_message(
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
            self.update_next(&context.sql, SecureJoinStep::Terminated)
                .await?;
            return Ok(Some(BobHandshakeStage::Terminated(reason)));
        }
        if !fingerprint_equals_sender(context, self.invite.fingerprint(), self.invite.contact_id())
            .await?
        {
            self.update_next(&context.sql, SecureJoinStep::Terminated)
                .await?;
            return Ok(Some(BobHandshakeStage::Terminated("Fingerprint mismatch")));
        }
        info!(context, "Fingerprint verified.",);
        self.update_next(&context.sql, SecureJoinStep::ContactConfirm)
            .await?;
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
    /// is only done out of symmetry with *vg-member-added* handling.
    async fn step_contact_confirm(
        &mut self,
        context: &Context,
        mime_message: &MimeMessage,
    ) -> Result<Option<BobHandshakeStage>> {
        info!(
            context,
            "Bob Step 7 - handling vc-contact-confirm/vg-member-added message"
        );
        mark_peer_as_verified(
            context,
            self.invite.fingerprint().clone(),
            mime_message.from.addr.to_string(),
        )
        .await?;
        Contact::scaleup_origin_by_id(context, self.invite.contact_id(), Origin::SecurejoinJoined)
            .await?;
        context.emit_event(EventType::ContactsChanged(None));

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

        self.update_next(&context.sql, SecureJoinStep::Completed)
            .await?;
        Ok(Some(BobHandshakeStage::Completed))
    }

    /// Sends the requested handshake message to Alice.
    ///
    /// This takes care of adding the required headers for the step.
    async fn send_handshake_message(&self, context: &Context, step: BobHandshakeMsg) -> Result<()> {
        send_handshake_message(context, &self.invite, self.chat_id, step).await
    }
}

/// Sends the requested handshake message to Alice.
///
/// Same as [`BobState::send_handshake_message`] but this variation allows us to send this
/// message before we create the state in [`BobState::start_protocol`].
async fn send_handshake_message(
    context: &Context,
    invite: &QrInvite,
    chat_id: ChatId,
    step: BobHandshakeMsg,
) -> Result<()> {
    let mut msg = Message {
        viewtype: Viewtype::Text,
        text: step.body_text(invite),
        hidden: true,
        ..Default::default()
    };
    msg.param.set_cmd(SystemMessage::SecurejoinMessage);

    // Sends the step in Secure-Join header.
    msg.param.set(Param::Arg, step.securejoin_header(invite));

    match step {
        BobHandshakeMsg::Request => {
            // Sends the Secure-Join-Invitenumber header in mimefactory.rs.
            msg.param.set(Param::Arg2, invite.invitenumber());
            msg.force_plaintext();
        }
        BobHandshakeMsg::RequestWithAuth => {
            // Sends the Secure-Join-Auth header in mimefactory.rs.
            msg.param.set(Param::Arg2, invite.authcode());
            msg.param.set_int(Param::GuaranteeE2ee, 1);
        }
        BobHandshakeMsg::ContactConfirmReceived => {
            msg.param.set_int(Param::GuaranteeE2ee, 1);
        }
    };

    // Sends our own fingerprint in the Secure-Join-Fingerprint header.
    let bob_fp = load_self_public_key(context).await?.fingerprint();
    msg.param.set(Param::Arg3, bob_fp.hex());

    // Sends the grpid in the Secure-Join-Group header.
    if let QrInvite::Group { ref grpid, .. } = invite {
        msg.param.set(Param::Arg4, grpid);
    }

    chat::send_msg(context, chat_id, &mut msg).await?;
    Ok(())
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureJoinStep {
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
                warn!(context, "Completed state for next securejoin step");
                false
            }
        }
    }
}

impl rusqlite::types::ToSql for SecureJoinStep {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let num = match &self {
            SecureJoinStep::AuthRequired => 0,
            SecureJoinStep::ContactConfirm => 1,
            SecureJoinStep::Terminated => 2,
            SecureJoinStep::Completed => 3,
        };
        let val = rusqlite::types::Value::Integer(num);
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

impl rusqlite::types::FromSql for SecureJoinStep {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).and_then(|val| match val {
            0 => Ok(SecureJoinStep::AuthRequired),
            1 => Ok(SecureJoinStep::ContactConfirm),
            2 => Ok(SecureJoinStep::Terminated),
            3 => Ok(SecureJoinStep::Completed),
            _ => Err(rusqlite::types::FromSqlError::OutOfRange(val)),
        })
    }
}

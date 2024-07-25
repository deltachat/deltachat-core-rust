//! Bob's side of SecureJoin handling.
//!
//! This are some helper functions around [`BobState`] which augment the state changes with
//! the required user interactions.

use anyhow::{Context as _, Result};

use super::bobstate::{BobHandshakeStage, BobState};
use super::qrinvite::QrInvite;
use super::HandshakeMessage;
use crate::chat::{is_contact_in_chat, ChatId, ProtectionStatus};
use crate::constants::{self, Blocked, Chattype};
use crate::contact::Contact;
use crate::context::Context;
use crate::events::EventType;
use crate::mimeparser::{MimeMessage, SystemMessage};
use crate::sync::Sync::*;
use crate::tools::{create_smeared_timestamp, time};
use crate::{chat, stock_str};

/// Starts the securejoin protocol with the QR `invite`.
///
/// This will try to start the securejoin protocol for the given QR `invite`.  If it
/// succeeded the protocol state will be tracked in `self`.
///
/// This function takes care of handling multiple concurrent joins and handling errors while
/// starting the protocol.
///
/// # Returns
///
/// The [`ChatId`] of the created chat is returned, for a SetupContact QR this is the 1:1
/// chat with Alice, for a SecureJoin QR this is the group chat.
pub(super) async fn start_protocol(context: &Context, invite: QrInvite) -> Result<ChatId> {
    // A 1:1 chat is needed to send messages to Alice.  When joining a group this chat is
    // hidden, if a user starts sending messages in it it will be unhidden in
    // receive_imf.
    let hidden = match invite {
        QrInvite::Contact { .. } => Blocked::Not,
        QrInvite::Group { .. } => Blocked::Yes,
    };
    let chat_id = ChatId::create_for_contact_with_blocked(context, invite.contact_id(), hidden)
        .await
        .with_context(|| format!("can't create chat for contact {}", invite.contact_id()))?;

    // Now start the protocol and initialise the state
    let (state, stage, aborted_states) =
        BobState::start_protocol(context, invite.clone(), chat_id).await?;
    for state in aborted_states {
        error!(context, "Aborting previously unfinished QR Join process.");
        state.notify_aborted(context, "New QR code scanned").await?;
        state.emit_progress(context, JoinerProgress::Error);
    }
    if matches!(stage, BobHandshakeStage::RequestWithAuthSent) {
        state.emit_progress(context, JoinerProgress::RequestWithAuthSent);
    }
    match invite {
        QrInvite::Group { .. } => {
            // For a secure-join we need to create the group and add the contact.  The group will
            // only become usable once the protocol is finished.
            let group_chat_id = state.joining_chat_id(context).await?;
            if !is_contact_in_chat(context, group_chat_id, invite.contact_id()).await? {
                chat::add_to_chat_contacts_table(context, group_chat_id, &[invite.contact_id()])
                    .await?;
            }
            let msg = stock_str::secure_join_started(context, invite.contact_id()).await;
            chat::add_info_msg(context, group_chat_id, &msg, time()).await?;
            Ok(group_chat_id)
        }
        QrInvite::Contact { .. } => {
            // For setup-contact the BobState already ensured the 1:1 chat exists because it
            // uses it to send the handshake messages.
            let chat_id = state.alice_chat();
            // Calculate the sort timestamp before checking the chat protection status so that if we
            // race with its change, we don't add our message below the protection message.
            let sort_to_bottom = true;
            let (received, incoming) = (false, false);
            let ts_sort = chat_id
                .calc_sort_timestamp(context, 0, sort_to_bottom, received, incoming)
                .await?;
            if chat_id.is_protected(context).await? == ProtectionStatus::Unprotected {
                let ts_start = time();
                chat::add_info_msg_with_cmd(
                    context,
                    chat_id,
                    &stock_str::securejoin_wait(context).await,
                    SystemMessage::SecurejoinWait,
                    ts_sort,
                    Some(ts_start),
                    None,
                    None,
                )
                .await?;
                chat_id.spawn_securejoin_wait(context, constants::SECUREJOIN_WAIT_TIMEOUT);
            }
            Ok(chat_id)
        }
    }
}

/// Handles `vc-auth-required` and `vg-auth-required` handshake messages.
///
/// # Bob - the joiner's side
/// ## Step 4 in the "Setup Contact protocol"
pub(super) async fn handle_auth_required(
    context: &Context,
    message: &MimeMessage,
) -> Result<HandshakeMessage> {
    let Some(mut bobstate) = BobState::from_db(&context.sql).await? else {
        return Ok(HandshakeMessage::Ignore);
    };

    match bobstate.handle_auth_required(context, message).await? {
        Some(BobHandshakeStage::Terminated(why)) => {
            bobstate.notify_aborted(context, why).await?;
            Ok(HandshakeMessage::Done)
        }
        Some(_stage) => {
            if bobstate.is_join_group() {
                // The message reads "Alice replied, waiting to be added to the group…",
                // so only show it on secure-join and not on setup-contact.
                let contact_id = bobstate.invite().contact_id();
                let msg = stock_str::secure_join_replies(context, contact_id).await;
                let chat_id = bobstate.joining_chat_id(context).await?;
                chat::add_info_msg(context, chat_id, &msg, time()).await?;
            }
            bobstate
                .set_peer_verified(context, message.timestamp_sent)
                .await?;
            bobstate.emit_progress(context, JoinerProgress::RequestWithAuthSent);
            Ok(HandshakeMessage::Done)
        }
        None => Ok(HandshakeMessage::Ignore),
    }
}

/// Private implementations for user interactions about this [`BobState`].
impl BobState {
    fn is_join_group(&self) -> bool {
        match self.invite() {
            QrInvite::Contact { .. } => false,
            QrInvite::Group { .. } => true,
        }
    }

    pub(crate) fn emit_progress(&self, context: &Context, progress: JoinerProgress) {
        let contact_id = self.invite().contact_id();
        context.emit_event(EventType::SecurejoinJoinerProgress {
            contact_id,
            progress: progress.into(),
        });
    }

    /// Returns the [`ChatId`] of the chat being joined.
    ///
    /// This is the chat in which you want to notify the user as well.
    ///
    /// When joining a group this is the [`ChatId`] of the group chat, when verifying a
    /// contact this is the [`ChatId`] of the 1:1 chat.  The 1:1 chat is assumed to exist
    /// because a [`BobState`] can not exist without, the group chat will be created if it
    /// does not yet exist.
    async fn joining_chat_id(&self, context: &Context) -> Result<ChatId> {
        match self.invite() {
            QrInvite::Contact { .. } => Ok(self.alice_chat()),
            QrInvite::Group {
                ref grpid,
                ref name,
                ..
            } => {
                let group_chat_id = match chat::get_chat_id_by_grpid(context, grpid).await? {
                    Some((chat_id, _protected, _blocked)) => {
                        chat_id.unblock_ex(context, Nosync).await?;
                        chat_id
                    }
                    None => {
                        ChatId::create_multiuser_record(
                            context,
                            Chattype::Group,
                            grpid,
                            name,
                            Blocked::Not,
                            ProtectionStatus::Unprotected, // protection is added later as needed
                            None,
                            create_smeared_timestamp(context),
                        )
                        .await?
                    }
                };
                Ok(group_chat_id)
            }
        }
    }

    /// Notifies the user that the SecureJoin was aborted.
    ///
    /// This creates an info message in the chat being joined.
    async fn notify_aborted(&self, context: &Context, why: &str) -> Result<()> {
        let contact = Contact::get_by_id(context, self.invite().contact_id()).await?;
        let mut msg = stock_str::contact_not_verified(context, &contact).await;
        msg += " (";
        msg += why;
        msg += ")";
        let chat_id = self.joining_chat_id(context).await?;
        chat::add_info_msg(context, chat_id, &msg, time()).await?;
        warn!(
            context,
            "StockMessage::ContactNotVerified posted to joining chat ({})", why
        );
        Ok(())
    }

    /// Turns 1:1 chat with SecureJoin peer into protected chat.
    pub(crate) async fn set_peer_verified(&self, context: &Context, timestamp: i64) -> Result<()> {
        let contact = Contact::get_by_id(context, self.invite().contact_id()).await?;
        self.alice_chat()
            .set_protection(
                context,
                ProtectionStatus::Protected,
                timestamp,
                Some(contact.id),
            )
            .await?;
        Ok(())
    }
}

/// Progress updates for [`EventType::SecurejoinJoinerProgress`].
///
/// This has an `From<JoinerProgress> for usize` impl yielding numbers between 0 and a 1000
/// which can be shown as a progress bar.
pub(crate) enum JoinerProgress {
    /// An error occurred.
    Error,
    /// vg-vc-request-with-auth sent.
    ///
    /// Typically shows as "alice@addr verified, introducing myself."
    RequestWithAuthSent,
    /// Completed securejoin.
    Succeeded,
}

impl From<JoinerProgress> for usize {
    fn from(progress: JoinerProgress) -> Self {
        match progress {
            JoinerProgress::Error => 0,
            JoinerProgress::RequestWithAuthSent => 400,
            JoinerProgress::Succeeded => 1000,
        }
    }
}

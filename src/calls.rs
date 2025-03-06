//! # Handle calls.

use crate::chat::{send_msg, Chat, ChatId};
use crate::constants::Chattype;
use crate::context::Context;
use crate::events::EventType;
use crate::message::{Message, MsgId, Viewtype};
use crate::mimeparser::{MimeMessage, SystemMessage};
use anyhow::{anyhow, ensure, Result};

impl Context {
    /// Start an outgoing call.
    pub async fn place_outgoing_call(&self, chat_id: ChatId) -> Result<MsgId> {
        let chat = Chat::load_from_db(self, chat_id).await?;
        ensure!(chat.typ == Chattype::Single && !chat.is_self_talk());

        let mut msg = Message {
            viewtype: Viewtype::Text,
            text: "Calling...".into(),
            ..Default::default()
        };
        msg.param.set_cmd(SystemMessage::OutgoingCall);
        msg.id = send_msg(self, chat_id, &mut msg).await?;
        Ok(msg.id)
    }

    /// Accept an incoming call.
    /// This implicitly accepts the contact request, if not yet done.
    pub async fn accept_incoming_call(&self, msg_id: MsgId) -> Result<()> {
        let call = Message::load_from_db(self, msg_id).await?;
        ensure!(call.get_info_type() == SystemMessage::IncomingCall);

        let chat = Chat::load_from_db(self, call.chat_id).await?;
        if chat.is_contact_request() {
            chat.id.accept(self).await?;
        }

        let mut msg = Message {
            viewtype: Viewtype::Text,
            text: "Call accepted".into(),
            ..Default::default()
        };
        msg.param.set_cmd(SystemMessage::CallAccepted);
        msg.set_quote(self, Some(&call)).await?;
        msg.id = send_msg(self, call.chat_id, &mut msg).await?;
        self.emit_event(EventType::IncomingCallAccepted { msg_id });
        Ok(())
    }

    /// End an call.
    /// This function may be called for both, incoming and outgoing calls.
    /// All participant devices get informed about the ended call.
    pub async fn end_call(&self, msg_id: MsgId) -> Result<()> {
        let call = Message::load_from_db(self, msg_id).await?;

        let mut msg = Message {
            viewtype: Viewtype::Text,
            text: "Call ended".into(),
            ..Default::default()
        };
        msg.param.set_cmd(SystemMessage::CallEnded);
        msg.set_quote(self, Some(&call)).await?;
        msg.id = send_msg(self, call.chat_id, &mut msg).await?;
        self.emit_event(EventType::CallEnded { msg_id });
        Ok(())
    }

    /// The the parent call message.
    /// The given ID is either the call message itself or a child.
    async fn load_call_msg(&self, msg_id: MsgId) -> Result<Message> {
        let msg = Message::load_from_db(self, msg_id).await?;
        if msg.get_info_type() == SystemMessage::CallAccepted
            || msg.get_info_type() == SystemMessage::CallEnded
        {
            if let Some(parent) = msg.parent(self).await? {
                return Ok(parent);
            } else {
                return Err(anyhow!("Call parent missing"));
            }
        } else {
            Ok(msg)
        }
    }

    pub(crate) async fn handle_call_msg(
        &self,
        mime_message: &MimeMessage,
        msg_id: MsgId,
    ) -> Result<()> {
        let call = self.load_call_msg(msg_id).await?;
        let incoming_call = call.get_info_type() == SystemMessage::IncomingCall;

        match mime_message.is_system_message {
            SystemMessage::IncomingCall => {
                if incoming_call {
                    self.emit_event(EventType::IncomingCall { msg_id });
                }
            }
            SystemMessage::CallAccepted => {
                if incoming_call {
                    self.emit_event(EventType::IncomingCallAccepted { msg_id });
                } else {
                    self.emit_event(EventType::OutgoingCallAccepted { msg_id });
                }
            }
            SystemMessage::CallEnded => {
                self.emit_event(EventType::CallEnded { msg_id });
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContextManager;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accept_call_callee_ends() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = &tcm.alice().await;
        let alice2 = &tcm.alice().await;
        let bob = &tcm.bob().await;
        let bob2 = &tcm.bob().await;

        // Alice creates a chat with Bob and places an outgoing call there.
        // Alice's other device sees the same message as an outgoing call.
        let alice_chat = alice.create_chat(bob).await;
        let test_msg_id = alice.place_outgoing_call(alice_chat.id).await?;
        let sent1 = alice.pop_sent_msg().await;
        let alice_call = Message::load_from_db(alice, sent1.sender_msg_id).await?;
        assert_eq!(sent1.sender_msg_id, test_msg_id);
        assert!(alice_call.is_info());
        assert_eq!(alice_call.get_info_type(), SystemMessage::OutgoingCall);

        let alice2_call = alice2.recv_msg(&sent1).await;
        assert!(alice2_call.is_info());
        assert_eq!(alice2_call.get_info_type(), SystemMessage::OutgoingCall);

        // Bob receives the message referring to the call on two devices;
        // it is an incoming call from the view of Bob
        let bob_call = bob.recv_msg(&sent1).await;
        bob.evtracker
            .get_matching(|evt| matches!(evt, EventType::IncomingCall { .. }))
            .await;
        assert!(bob_call.is_info());
        assert_eq!(bob_call.get_info_type(), SystemMessage::IncomingCall);

        let bob2_call = bob2.recv_msg(&sent1).await;
        assert!(bob2_call.is_info());
        assert_eq!(bob2_call.get_info_type(), SystemMessage::IncomingCall);

        // Bob accepts the incoming call, this does not add an additional message to the chat
        bob.accept_incoming_call(bob_call.id).await?;
        bob.evtracker
            .get_matching(|evt| matches!(evt, EventType::IncomingCallAccepted { .. }))
            .await;
        let sent2 = bob.pop_sent_msg().await;

        bob2.recv_msg(&sent2).await;
        bob2.evtracker
            .get_matching(|evt| matches!(evt, EventType::IncomingCallAccepted { .. }))
            .await;

        // Alice receives the acceptance message
        alice.recv_msg(&sent2).await;
        alice
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::OutgoingCallAccepted { .. }))
            .await;

        alice2.recv_msg(&sent2).await;
        alice2
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::OutgoingCallAccepted { .. }))
            .await;

        // Bob has accepted the call and also ends it
        bob.end_call(bob_call.id).await?;
        bob.evtracker
            .get_matching(|evt| matches!(evt, EventType::CallEnded { .. }))
            .await;
        let sent3 = bob.pop_sent_msg().await;

        bob2.recv_msg(&sent3).await;
        bob2.evtracker
            .get_matching(|evt| matches!(evt, EventType::CallEnded { .. }))
            .await;

        // Alice receives the ending message
        alice.recv_msg(&sent3).await;
        alice
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::CallEnded { .. }))
            .await;

        alice2.recv_msg(&sent3).await;
        alice2
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::CallEnded { .. }))
            .await;

        Ok(())
    }
}

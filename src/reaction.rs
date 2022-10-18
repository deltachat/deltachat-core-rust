//! # Reactions.
//!
//! Reactions are short messages consisting of emojis sent in reply to
//! messages. Unlike normal messages which are added to the end of the chat,
//! reactions are supposed to be displayed near the original messages.
//!
//! RFC 9078 specifies how reactions are transmitted in MIME messages.
//!
//! Reaction update semantics is not well-defined in RFC 9078, so
//! Delta Chat uses the same semantics as in
//! [XEP-0444](https://xmpp.org/extensions/xep-0444.html) section
//! "3.2 Updating reactions to a message". Received reactions override
//! all previously received reactions from the same user and it is
//! possible to remove all reactions by sending an empty string as a reaction,
//! even though RFC 9078 requires at least one emoji to be sent.

use std::collections::BTreeMap;
use std::fmt;

use anyhow::Result;

use crate::chat::{send_msg, ChatId};
use crate::contact::ContactId;
use crate::context::Context;
use crate::events::EventType;
use crate::message::{rfc724_mid_exists, Message, MsgId, Viewtype};

/// A single reaction consisting of multiple emoji sequences.
///
/// It is guaranteed to have all emojis sorted and deduplicated inside.
#[derive(Debug, Default, Clone)]
pub struct Reaction {
    /// Canonical represntation of reaction as a string of space-separated emojis.
    reaction: String,
}

// We implement From<&str> instead of std::str::FromStr, because
// FromStr requires error type and reaction parsing never returns an
// error.
impl From<&str> for Reaction {
    /// Parses a string containing a reaction.
    ///
    /// Reaction string is separated by spaces or tabs (`WSP` in ABNF),
    /// but this function accepts any ASCII whitespace, so even a CRLF at
    /// the end of string is acceptable.
    ///
    /// Any short enough string is accepted as a reaction to avoid the
    /// complexity of validating emoji sequences ase required by RFC
    /// 9078. On the sender side UI is responsible to provide only
    /// valid emoji sequences via reaction picker. On the receiver
    /// side, abuse of the possibility to use arbitrary strings as
    /// reactions is not different from other kinds of spam attacks
    /// such as sending large numbers of large messages, and should be
    /// dealt with the same way, e.g. by blocking the user.
    fn from(reaction: &str) -> Self {
        let mut emojis: Vec<&str> = reaction
            .split_ascii_whitespace()
            .filter(|&emoji| emoji.len() < 30)
            .collect();
        emojis.sort();
        emojis.dedup();
        let reaction = emojis.join(" ");
        Self { reaction }
    }
}

impl Reaction {
    /// Returns true if reaction contains no emojis.
    pub fn is_empty(&self) -> bool {
        self.reaction.is_empty()
    }

    /// Returns a vector of emojis composing a reaction.
    pub fn emojis(&self) -> Vec<&str> {
        self.reaction.split(' ').collect()
    }

    /// Returns space-separated string of emojis
    pub fn as_str(&self) -> &str {
        &self.reaction
    }

    /// Appends emojis from another reaction to this reaction.
    pub fn add(&self, other: Self) -> Self {
        let mut emojis: Vec<&str> = self.emojis();
        emojis.append(&mut other.emojis());
        emojis.sort();
        emojis.dedup();
        let reaction = emojis.join(" ");
        Self { reaction }
    }
}

/// Structure representing all reactions to a particular message.
#[derive(Debug)]
pub struct Reactions {
    /// Map from a contact to its reaction to message.
    reactions: BTreeMap<ContactId, Reaction>,
}

impl Reactions {
    /// Returns vector of contacts that reacted to the message.
    pub fn contacts(&self) -> Vec<ContactId> {
        self.reactions.keys().copied().collect()
    }

    /// Returns reaction of a given contact to message.
    ///
    /// If contact did not react to message or removed the reaction,
    /// this method returns an empty reaction.
    pub fn get(&self, contact_id: ContactId) -> Reaction {
        self.reactions.get(&contact_id).cloned().unwrap_or_default()
    }

    /// Returns true if the message has no reactions.
    pub fn is_empty(&self) -> bool {
        self.reactions.is_empty()
    }
}

impl fmt::Display for Reactions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut emoji_frequencies: BTreeMap<String, usize> = BTreeMap::new();
        for reaction in self.reactions.values() {
            for emoji in reaction.emojis() {
                emoji_frequencies
                    .entry(emoji.to_string())
                    .and_modify(|x| *x += 1)
                    .or_insert(1);
            }
        }
        let mut first = true;
        for (emoji, frequency) in emoji_frequencies {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "{}{}", emoji, frequency)?;
        }
        Ok(())
    }
}

async fn set_msg_id_reaction(
    context: &Context,
    msg_id: MsgId,
    chat_id: ChatId,
    contact_id: ContactId,
    reaction: Reaction,
) -> Result<()> {
    if reaction.is_empty() {
        // Simply remove the record instead of setting it to empty string.
        context
            .sql
            .execute(
                "DELETE FROM reactions
                 WHERE msg_id = ?1
                 AND contact_id = ?2",
                paramsv![msg_id, contact_id],
            )
            .await?;
    } else {
        context
            .sql
            .execute(
                "INSERT INTO reactions (msg_id, contact_id, reaction)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(msg_id, contact_id)
                 DO UPDATE SET reaction=excluded.reaction",
                paramsv![msg_id, contact_id, reaction.as_str()],
            )
            .await?;
    }

    context.emit_event(EventType::ReactionsChanged {
        chat_id,
        msg_id,
        contact_id,
    });
    Ok(())
}

/// Sends a reaction to message `msg_id`, overriding previously sent reactions.
///
/// `reaction` is a string consisting of space-separated emoji. Use
/// empty string to retract a reaction.
pub async fn send_reaction(context: &Context, msg_id: MsgId, reaction: &str) -> Result<MsgId> {
    let msg = Message::load_from_db(context, msg_id).await?;
    let chat_id = msg.chat_id;

    let reaction: Reaction = reaction.into();
    let mut reaction_msg = Message::new(Viewtype::Reaction);
    reaction_msg.text = Some(reaction.as_str().to_string());

    set_msg_id_reaction(context, msg_id, msg.chat_id, ContactId::SELF, reaction).await?;
    send_msg(context, chat_id, &mut reaction_msg).await
}

/// Adds given reaction to message `msg_id` and sends an update.
///
/// This can be used to implement advanced clients that allow reacting
/// with multiple emojis. For a simple messenger UI, you probably want
/// to use [`send_reaction()`] instead so reacing with a new emoji
/// removes previous emoji at the same time.
pub async fn add_reaction(context: &Context, msg_id: MsgId, reaction: &str) -> Result<MsgId> {
    let self_reaction = get_self_reaction(context, msg_id).await?;
    let reaction = self_reaction.add(Reaction::from(reaction));
    send_reaction(context, msg_id, reaction.as_str()).await
}

/// Updates reaction of `contact_id` on the message with `in_reply_to`
/// Message-ID. If no such message is found in the database, reaction
/// is ignored.
///
/// `reaction` is a space-separated string of emojis. It can be empty
/// if contact wants to remove all reactions.
pub(crate) async fn set_msg_reaction(
    context: &Context,
    in_reply_to: &str,
    chat_id: ChatId,
    contact_id: ContactId,
    reaction: Reaction,
) -> Result<()> {
    if let Some(msg_id) = rfc724_mid_exists(context, in_reply_to).await? {
        set_msg_id_reaction(context, msg_id, chat_id, contact_id, reaction).await
    } else {
        info!(
            context,
            "Can't assign reaction to unknown message with Message-ID {}", in_reply_to
        );
        Ok(())
    }
}

/// Get our own reaction for a given message.
async fn get_self_reaction(context: &Context, msg_id: MsgId) -> Result<Reaction> {
    let reaction_str: Option<String> = context
        .sql
        .query_get_value(
            "SELECT reaction
             FROM reactions
             WHERE msg_id=? AND contact_id=?",
            paramsv![msg_id, ContactId::SELF],
        )
        .await?;
    Ok(reaction_str
        .as_deref()
        .map(Reaction::from)
        .unwrap_or_default())
}

/// Returns a structure containing all reactions to the message.
pub async fn get_msg_reactions(context: &Context, msg_id: MsgId) -> Result<Reactions> {
    let reactions = context
        .sql
        .query_map(
            "SELECT contact_id, reaction FROM reactions WHERE msg_id=?",
            paramsv![msg_id],
            |row| {
                let contact_id: ContactId = row.get(0)?;
                let reaction: String = row.get(1)?;
                Ok((contact_id, reaction))
            },
            |rows| {
                let mut reactions = Vec::new();
                for row in rows {
                    let (contact_id, reaction) = row?;
                    reactions.push((contact_id, Reaction::from(reaction.as_str())));
                }
                Ok(reactions)
            },
        )
        .await?
        .into_iter()
        .collect();
    Ok(Reactions { reactions })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::Config;
    use crate::constants::DC_CHAT_ID_TRASH;
    use crate::contact::{Contact, Origin};
    use crate::message::MessageState;
    use crate::receive_imf::receive_imf;
    use crate::test_utils::TestContext;

    #[test]
    fn test_parse_reaction() {
        // Check that basic set of emojis from RFC 9078 is supported.
        assert_eq!(Reaction::from("👍").emojis(), vec!["👍"]);
        assert_eq!(Reaction::from("👎").emojis(), vec!["👎"]);
        assert_eq!(Reaction::from("😀").emojis(), vec!["😀"]);
        assert_eq!(Reaction::from("☹").emojis(), vec!["☹"]);
        assert_eq!(Reaction::from("😢").emojis(), vec!["😢"]);

        // Empty string can be used to remove all reactions.
        assert!(Reaction::from("").is_empty());

        // Short strings can be used as emojis, could be used to add
        // support for custom emojis via emoji shortcodes.
        assert_eq!(Reaction::from(":deltacat:").emojis(), vec![":deltacat:"]);

        // Check that long strings are not valid emojis.
        assert!(
            Reaction::from(":foobarbazquuxaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa:").is_empty()
        );

        // Multiple reactions separated by spaces or tabs are supported.
        assert_eq!(Reaction::from("👍 ❤").emojis(), vec!["❤", "👍"]);
        assert_eq!(Reaction::from("👍\t❤").emojis(), vec!["❤", "👍"]);

        // Invalid emojis are removed, but valid emojis are retained.
        assert_eq!(
            Reaction::from("👍\t:foo: ❤").emojis(),
            vec![":foo:", "❤", "👍"]
        );
        assert_eq!(Reaction::from("👍\t:foo: ❤").as_str(), ":foo: ❤ 👍");

        // Duplicates are removed.
        assert_eq!(Reaction::from("👍 👍").emojis(), vec!["👍"]);
    }

    #[test]
    fn test_add_reaction() {
        let reaction1 = Reaction::from("👍 😀");
        let reaction2 = Reaction::from("❤");
        let reaction_sum = reaction1.add(reaction2);

        assert_eq!(reaction_sum.emojis(), vec!["❤", "👍", "😀"]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_receive_reaction() -> Result<()> {
        let alice = TestContext::new_alice().await;
        alice.set_config(Config::ShowEmails, Some("2")).await?;

        // Alice receives BCC-self copy of a message sent to Bob.
        receive_imf(
            &alice,
            "To: bob@example.net\n\
From: alice@example.org\n\
Date: Today, 29 February 2021 00:00:00 -800\n\
Message-ID: 12345@example.org\n\
Subject: Meeting\n\
\n\
Can we chat at 1pm pacific, today?"
                .as_bytes(),
            false,
        )
        .await?;
        let msg = alice.get_last_msg().await;
        assert_eq!(msg.state, MessageState::OutDelivered);
        let reactions = get_msg_reactions(&alice, msg.id).await?;
        let contacts = reactions.contacts();
        assert_eq!(contacts.len(), 0);

        let bob_id = Contact::add_or_lookup(&alice, "", "bob@example.net", Origin::ManuallyCreated)
            .await?
            .0;
        let bob_reaction = reactions.get(bob_id);
        assert!(bob_reaction.is_empty()); // Bob has not reacted to message yet.

        // Alice receives reaction to her message from Bob.
        receive_imf(
            &alice,
            "To: alice@example.org\n\
From: bob@example.net\n\
Date: Today, 29 February 2021 00:00:10 -800\n\
Message-ID: 56789@example.net\n\
In-Reply-To: 12345@example.org\n\
Subject: Meeting\n\
Mime-Version: 1.0 (1.0)\n\
Content-Type: text/plain; charset=utf-8\n\
Content-Disposition: reaction\n\
\n\
\u{1F44D}"
                .as_bytes(),
            false,
        )
        .await?;

        let reactions = get_msg_reactions(&alice, msg.id).await?;
        assert_eq!(reactions.to_string(), "👍1");

        let contacts = reactions.contacts();
        assert_eq!(contacts.len(), 1);

        assert_eq!(contacts.get(0), Some(&bob_id));
        let bob_reaction = reactions.get(bob_id);
        assert_eq!(bob_reaction.is_empty(), false);
        assert_eq!(bob_reaction.emojis(), vec!["👍"]);
        assert_eq!(bob_reaction.as_str(), "👍");

        Ok(())
    }

    async fn expect_reactions_changed_event(
        t: &TestContext,
        expected_chat_id: ChatId,
        expected_msg_id: MsgId,
        expected_contact_id: ContactId,
    ) -> Result<()> {
        let event = t
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::ReactionsChanged { .. }))
            .await;
        match event {
            EventType::ReactionsChanged {
                chat_id,
                msg_id,
                contact_id,
            } => {
                assert_eq!(chat_id, expected_chat_id);
                assert_eq!(msg_id, expected_msg_id);
                assert_eq!(contact_id, expected_contact_id);
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send_reaction() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let chat_alice = alice.create_chat(&bob).await;
        let alice_msg = alice.send_text(chat_alice.id, "Hi!").await;
        let bob_msg = bob.recv_msg(&alice_msg).await;

        bob_msg.chat_id.accept(&bob).await?;

        send_reaction(&bob, bob_msg.id, "👍").await.unwrap();
        expect_reactions_changed_event(&bob, bob_msg.chat_id, bob_msg.id, ContactId::SELF).await?;

        let bob_reaction_msg = bob.pop_sent_msg().await;
        let alice_reaction_msg = alice.recv_msg_opt(&bob_reaction_msg).await.unwrap();
        assert_eq!(alice_reaction_msg.chat_id, DC_CHAT_ID_TRASH);

        let reactions = get_msg_reactions(&alice, alice_msg.sender_msg_id).await?;
        assert_eq!(reactions.to_string(), "👍1");
        let contacts = reactions.contacts();
        assert_eq!(contacts.len(), 1);
        let bob_id = contacts.get(0).unwrap();
        let bob_reaction = reactions.get(*bob_id);
        assert_eq!(bob_reaction.is_empty(), false);
        assert_eq!(bob_reaction.emojis(), vec!["👍"]);
        assert_eq!(bob_reaction.as_str(), "👍");
        expect_reactions_changed_event(&alice, chat_alice.id, alice_msg.sender_msg_id, *bob_id)
            .await?;

        // Alice reacts to own message.
        send_reaction(&alice, alice_msg.sender_msg_id, "👍 😀")
            .await
            .unwrap();
        let reactions = get_msg_reactions(&alice, alice_msg.sender_msg_id).await?;
        assert_eq!(reactions.to_string(), "👍2 😀1");

        Ok(())
    }
}

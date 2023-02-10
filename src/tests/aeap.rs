#![allow(clippy::indexing_slicing)]

use anyhow::Result;

use crate::chat;
use crate::chat::ChatId;
use crate::contact;
use crate::contact::Contact;
use crate::contact::ContactId;
use crate::message::Message;
use crate::peerstate;
use crate::peerstate::Peerstate;
use crate::receive_imf::receive_imf;
use crate::stock_str;
use crate::test_utils::TestContext;
use crate::test_utils::TestContextManager;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_change_primary_self_addr() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;

    tcm.send_recv_accept(&alice, &bob, "Hi").await;
    let bob_alice_chat = bob.create_chat(&alice).await;

    tcm.change_addr(&alice, "alice@someotherdomain.xyz").await;

    tcm.section("Bob sends a message to Alice, encrypting to her previous key");
    let sent = bob.send_text(bob_alice_chat.id, "hi back").await;

    // Alice set up message forwarding so that she still receives
    // the message with her new address
    let alice_msg = alice.recv_msg(&sent).await;
    assert_eq!(alice_msg.text, Some("hi back".to_string()));
    assert_eq!(alice_msg.get_showpadlock(), true);
    let alice_bob_chat = alice.create_chat(&bob).await;
    assert_eq!(alice_msg.chat_id, alice_bob_chat.id);

    tcm.section("Bob sends a message to Alice without In-Reply-To");
    // Even if Bob sends a message to Alice without In-Reply-To,
    // it's still assigned to the 1:1 chat with Bob and not to
    // a group (without secondary addresses, an ad-hoc group
    // would be created)
    receive_imf(
        &alice,
        b"From: bob@example.net
To: alice@example.org
Chat-Version: 1.0
Message-ID: <456@example.com>

Message w/out In-Reply-To
",
        false,
    )
    .await?;

    let alice_msg = alice.get_last_msg().await;

    assert_eq!(
        alice_msg.text,
        Some("Message w/out In-Reply-To".to_string())
    );
    assert_eq!(alice_msg.get_showpadlock(), false);
    assert_eq!(alice_msg.chat_id, alice_bob_chat.id);

    Ok(())
}

enum ChatForTransition {
    OneToOne,
    GroupChat,
    VerifiedGroup,
}
use ChatForTransition::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_0() {
    check_aeap_transition(OneToOne, false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_1() {
    check_aeap_transition(GroupChat, false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_0_verified() {
    check_aeap_transition(OneToOne, true, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_1_verified() {
    check_aeap_transition(GroupChat, true, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_2_verified() {
    check_aeap_transition(VerifiedGroup, true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_0_bob_knew_new_addr() {
    check_aeap_transition(OneToOne, false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_1_bob_knew_new_addr() {
    check_aeap_transition(GroupChat, false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_0_verified_bob_knew_new_addr() {
    check_aeap_transition(OneToOne, true, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_1_verified_bob_knew_new_addr() {
    check_aeap_transition(GroupChat, true, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_transition_2_verified_bob_knew_new_addr() {
    check_aeap_transition(VerifiedGroup, true, true).await;
}

/// Happy path test for AEAP in various configurations.
/// - `chat_for_transition`: Which chat the transition message should be sent in
/// - `verified`: Whether Alice and Bob verified each other
/// - `bob_knew_new_addr`: Whether Bob already had a chat with Alice's new address
async fn check_aeap_transition(
    chat_for_transition: ChatForTransition,
    verified: bool,
    bob_knew_new_addr: bool,
) {
    // Alice's new address is "fiona@example.net" so that we can test
    // the case where Bob already had contact with Alice's new address
    const ALICE_NEW_ADDR: &str = "fiona@example.net";

    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;

    if bob_knew_new_addr {
        let fiona = tcm.fiona().await;

        tcm.send_recv_accept(&fiona, &bob, "Hi").await;
        tcm.send_recv_accept(&bob, &fiona, "Hi back").await;
    }

    tcm.send_recv_accept(&alice, &bob, "Hi").await;
    tcm.send_recv_accept(&bob, &alice, "Hi back").await;

    if verified {
        mark_as_verified(&alice, &bob).await;
        mark_as_verified(&bob, &alice).await;
    }

    let mut groups = vec![
        chat::create_group_chat(&bob, chat::ProtectionStatus::Unprotected, "Group 0")
            .await
            .unwrap(),
        chat::create_group_chat(&bob, chat::ProtectionStatus::Unprotected, "Group 1")
            .await
            .unwrap(),
    ];
    if verified {
        groups.push(
            chat::create_group_chat(&bob, chat::ProtectionStatus::Protected, "Group 2")
                .await
                .unwrap(),
        );
        groups.push(
            chat::create_group_chat(&bob, chat::ProtectionStatus::Protected, "Group 3")
                .await
                .unwrap(),
        );
    }

    let old_contact = Contact::create(&bob, "Alice", "alice@example.org")
        .await
        .unwrap();
    for group in &groups {
        chat::add_contact_to_chat(&bob, *group, old_contact)
            .await
            .unwrap();
    }

    // Already add the new contact to one of the groups.
    // We can then later check that the contact isn't in the group twice.
    let already_new_contact = Contact::create(&bob, "Alice", "fiona@example.net")
        .await
        .unwrap();
    if verified {
        chat::add_contact_to_chat(&bob, groups[2], already_new_contact)
            .await
            .unwrap();
    }

    // groups 0 and 2 stay unpromoted (i.e. local
    // on Bob's device, Alice doesn't know about them)
    tcm.section("Promoting group 1");
    let sent = bob.send_text(groups[1], "group created").await;
    let group1_alice = alice.recv_msg(&sent).await.chat_id;

    let mut group3_alice = None;
    if verified {
        tcm.section("Promoting group 3");
        let sent = bob.send_text(groups[3], "group created").await;
        group3_alice = Some(alice.recv_msg(&sent).await.chat_id);
    }

    tcm.change_addr(&alice, ALICE_NEW_ADDR).await;

    tcm.section("Alice sends another message to Bob, this time from her new addr");
    // No matter which chat Alice sends to, the transition should be done in all groups
    let chat_to_send = match chat_for_transition {
        OneToOne => alice.create_chat(&bob).await.id,
        GroupChat => group1_alice,
        VerifiedGroup => group3_alice.expect("No verified group"),
    };
    let sent = alice
        .send_text(chat_to_send, "Hello from my new addr!")
        .await;
    let recvd = bob.recv_msg(&sent).await;
    let sent_timestamp = recvd.timestamp_sent;
    assert_eq!(recvd.text.unwrap(), "Hello from my new addr!");

    tcm.section("Check that the AEAP transition worked");
    check_that_transition_worked(
        &groups[2..],
        &alice,
        "alice@example.org",
        ALICE_NEW_ADDR,
        "Alice",
        &bob,
    )
    .await;
    check_no_transition_done(&groups[0..2], "alice@example.org", &bob).await;

    // Assert that the autocrypt header is also applied to the peerstate
    // if the address changed
    let bob_alice_peerstate = Peerstate::from_addr(&bob, ALICE_NEW_ADDR)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bob_alice_peerstate.last_seen, sent_timestamp);
    assert_eq!(bob_alice_peerstate.last_seen_autocrypt, sent_timestamp);

    tcm.section("Test switching back");
    tcm.change_addr(&alice, "alice@example.org").await;
    let sent = alice
        .send_text(chat_to_send, "Hello from my old addr!")
        .await;
    let recvd = bob.recv_msg(&sent).await;
    assert_eq!(recvd.text.unwrap(), "Hello from my old addr!");

    check_that_transition_worked(
        &groups[2..],
        &alice,
        // Note that "alice@example.org" and ALICE_NEW_ADDR are switched now:
        ALICE_NEW_ADDR,
        "alice@example.org",
        "Alice",
        &bob,
    )
    .await;
}

async fn check_that_transition_worked(
    groups: &[ChatId],
    alice: &TestContext,
    old_alice_addr: &str,
    new_alice_addr: &str,
    name: &str,
    bob: &TestContext,
) {
    let new_contact = Contact::lookup_id_by_addr(bob, new_alice_addr, contact::Origin::Unknown)
        .await
        .unwrap()
        .unwrap();

    for group in groups {
        let members = chat::get_chat_contacts(bob, *group).await.unwrap();
        // In all the groups, exactly Bob and Alice's new number are members.
        // (and Alice's new number isn't in there twice)
        assert_eq!(
            members.len(),
            2,
            "Group {} has members {:?}, but should have members {:?} and {:?}",
            group,
            &members,
            new_contact,
            ContactId::SELF
        );
        assert!(members.contains(&new_contact));
        assert!(members.contains(&ContactId::SELF));

        let info_msg = get_last_info_msg(bob, *group).await.unwrap();
        let expected_text =
            stock_str::aeap_addr_changed(bob, name, old_alice_addr, new_alice_addr).await;
        assert_eq!(info_msg.text.unwrap(), expected_text);
        assert_eq!(info_msg.from_id, ContactId::INFO);

        let msg = format!("Sending to group {group}");
        let sent = bob.send_text(*group, &msg).await;
        let recvd = alice.recv_msg(&sent).await;
        assert_eq!(recvd.text.unwrap(), msg);
    }
}

async fn check_no_transition_done(groups: &[ChatId], old_alice_addr: &str, bob: &TestContext) {
    let old_contact = Contact::lookup_id_by_addr(bob, old_alice_addr, contact::Origin::Unknown)
        .await
        .unwrap()
        .unwrap();

    for group in groups {
        let members = chat::get_chat_contacts(bob, *group).await.unwrap();
        // In all the groups, exactly Bob and Alice's _old_ number are members.
        assert_eq!(
            members.len(),
            2,
            "Group {} has members {:?}, but should have members {:?} and {:?}",
            group,
            &members,
            old_contact,
            ContactId::SELF
        );
        assert!(members.contains(&old_contact));
        assert!(members.contains(&ContactId::SELF));

        let last_info_msg = get_last_info_msg(bob, *group).await;
        assert!(
            last_info_msg.is_none(),
            "{last_info_msg:?} shouldn't be there (or it's an unrelated info msg)"
        );
    }
}

async fn mark_as_verified(this: &TestContext, other: &TestContext) {
    let other_addr = other.get_primary_self_addr().await.unwrap();
    let mut peerstate = peerstate::Peerstate::from_addr(this, &other_addr)
        .await
        .unwrap()
        .unwrap();

    peerstate.verified_key = peerstate.public_key.clone();
    peerstate.verified_key_fingerprint = peerstate.public_key_fingerprint.clone();

    peerstate.save_to_db(&this.sql).await.unwrap();
}

async fn get_last_info_msg(t: &TestContext, chat_id: ChatId) -> Option<Message> {
    let msgs = chat::get_chat_msgs_ex(
        &t.ctx,
        chat_id,
        chat::MessageListOptions {
            info_only: true,
            add_daymarker: false,
        },
    )
    .await
    .unwrap();
    let msg_id = if let chat::ChatItem::Message { msg_id } = msgs.last()? {
        msg_id
    } else {
        return None;
    };
    Some(Message::load_from_db(&t.ctx, *msg_id).await.unwrap())
}

/// Test that an attacker - here Fiona - can't replay a message sent by Alice
/// to make Bob think that there was a transition to Fiona's address.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_aeap_replay_attack() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;

    tcm.send_recv_accept(&alice, &bob, "Hi").await;
    tcm.send_recv_accept(&bob, &alice, "Hi back").await;

    let group =
        chat::create_group_chat(&bob, chat::ProtectionStatus::Unprotected, "Group 0").await?;

    let bob_alice_contact = Contact::create(&bob, "Alice", "alice@example.org").await?;
    chat::add_contact_to_chat(&bob, group, bob_alice_contact).await?;

    // Alice sends a message which Bob doesn't receive or something
    // A real attack would rather re-use a message that was sent to a group
    // and replace the Message-Id or so.
    let chat = alice.create_chat(&bob).await;
    let sent = alice.send_text(chat.id, "whoop whoop").await;

    // Fiona gets the message, replaces the From addr...
    let sent = sent
        .payload()
        .replace("From: <alice@example.org>", "From: <fiona@example.net>")
        .replace("addr=alice@example.org;", "addr=fiona@example.net;");
    sent.find("From: <fiona@example.net>").unwrap(); // Assert that it worked
    sent.find("addr=fiona@example.net;").unwrap(); // Assert that it worked

    tcm.section("Fiona replaced the From addr and forwards the message to Bob");
    receive_imf(&bob, sent.as_bytes(), false).await?.unwrap();

    // Check that no transition was done
    assert!(chat::is_contact_in_chat(&bob, group, bob_alice_contact).await?);
    let bob_fiona_contact = Contact::create(&bob, "", "fiona@example.net").await?;
    assert!(!chat::is_contact_in_chat(&bob, group, bob_fiona_contact).await?);

    Ok(())
}

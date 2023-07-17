use anyhow::Result;
use pretty_assertions::assert_eq;

use crate::chat::{Chat, ProtectionStatus};
use crate::config::Config;
use crate::contact::VerifiedStatus;
use crate::contact::{Contact, Origin};
use crate::message::{Message, Viewtype};
use crate::mimefactory::MimeFactory;
use crate::mimeparser::SystemMessage;
use crate::receive_imf::receive_imf;
use crate::stock_str;
use crate::test_utils::{get_chat_msg, mark_as_verified, TestContext, TestContextManager};
use crate::{e2ee, message};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_verified_oneonone_chat_broken_by_classical() {
    check_verified_oneonone_chat(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_verified_oneonone_chat_broken_by_device_change() {
    check_verified_oneonone_chat(false).await;
}

async fn check_verified_oneonone_chat(broken_by_classical_email: bool) {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    tcm.execute_securejoin(&alice, &bob).await;

    assert_verified(&alice, &bob, ProtectionStatus::Protected).await;
    assert_verified(&bob, &alice, ProtectionStatus::Protected).await;

    if broken_by_classical_email {
        tcm.section("Bob uses a classical MUA to send a message to Alice");
        receive_imf(
            &alice,
            b"Subject: Re: Message from alice\r\n\
          From: <bob@example.net>\r\n\
          To: <alice@example.org>\r\n\
          Date: Mon, 12 Dec 2022 14:33:39 +0000\r\n\
          Message-ID: <abcd@example.net>\r\n\
          \r\n\
          Heyho!\r\n",
            false,
        )
        .await
        .unwrap()
        .unwrap();
    } else {
        tcm.section("Bob sets up another Delta Chat device");
        let bob2 = TestContext::new().await;
        enable_verified_oneonone_chats(&[&bob2]).await;
        bob2.set_name("bob2");
        bob2.configure_addr("bob@example.net").await;

        tcm.send_recv(&bob2, &alice, "Using another device now")
            .await;
    }

    // Bob's contact is still verified, but the chat isn't marked as protected anymore
    assert_verified(&alice, &bob, ProtectionStatus::ProtectionBroken).await;

    tcm.section("Bob sends another message from DC");
    tcm.send_recv(&bob, &alice, "Using DC again").await;

    let contact = alice.add_or_lookup_contact(&bob).await;
    assert_eq!(
        contact.is_verified(&alice.ctx).await.unwrap(),
        VerifiedStatus::BidirectVerified
    );

    // Bob's chat is marked as verified again
    assert_verified(&alice, &bob, ProtectionStatus::Protected).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create_verified_oneonone_chat() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    let fiona = tcm.fiona().await;
    enable_verified_oneonone_chats(&[&alice, &bob, &fiona]).await;

    tcm.execute_securejoin(&alice, &bob).await;
    tcm.execute_securejoin(&bob, &fiona).await;
    assert_verified(&alice, &bob, ProtectionStatus::Protected).await;
    assert_verified(&bob, &alice, ProtectionStatus::Protected).await;
    assert_verified(&bob, &fiona, ProtectionStatus::Protected).await;
    assert_verified(&fiona, &bob, ProtectionStatus::Protected).await;

    let group_id = bob
        .create_group_with_members(
            ProtectionStatus::Protected,
            "Group with everyone",
            &[&alice, &fiona],
        )
        .await;
    assert_eq!(
        get_chat_msg(&bob, group_id, 0, 1).await.get_info_type(),
        SystemMessage::ChatProtectionEnabled
    );

    {
        let sent = bob.send_text(group_id, "Heyho").await;
        alice.recv_msg(&sent).await;

        let msg = fiona.recv_msg(&sent).await;
        assert_eq!(
            get_chat_msg(&fiona, msg.chat_id, 0, 2)
                .await
                .get_info_type(),
            SystemMessage::ChatProtectionEnabled
        );
    }

    // Alice and Fiona should now be verified because of gossip
    let alice_fiona_contact = alice.add_or_lookup_contact(&fiona).await;
    assert_eq!(
        alice_fiona_contact.is_verified(&alice).await.unwrap(),
        VerifiedStatus::BidirectVerified
    );

    // As soon as Alice creates a chat with Fiona, it should directly be protected
    {
        let chat = alice.create_chat(&fiona).await;
        assert!(chat.is_protected());

        let msg = alice.get_last_msg().await;
        let expected_text = stock_str::chat_protection_enabled(&alice).await;
        assert_eq!(msg.text, expected_text);
    }

    // Fiona should also see the chat as protected
    {
        let rcvd = tcm.send_recv(&alice, &fiona, "Hi Fiona").await;
        let alice_fiona_id = rcvd.chat_id;
        let chat = Chat::load_from_db(&fiona, alice_fiona_id).await?;
        assert!(chat.is_protected());

        let msg0 = get_chat_msg(&fiona, chat.id, 0, 2).await;
        let expected_text = stock_str::chat_protection_enabled(&fiona).await;
        assert_eq!(msg0.text, expected_text);
    }

    tcm.section("Fiona reinstalls DC");
    drop(fiona);

    let fiona_new = tcm.unconfigured().await;
    enable_verified_oneonone_chats(&[&fiona_new]).await;
    fiona_new.configure_addr("fiona@example.net").await;
    e2ee::ensure_secret_key_exists(&fiona_new).await?;

    tcm.send_recv(&fiona_new, &alice, "I have a new device")
        .await;

    // The chat should be and stay unprotected
    {
        let chat = alice.get_chat(&fiona_new).await.unwrap();
        assert!(!chat.is_protected());
        assert!(chat.is_protection_broken());

        // After recreating the chat, it should still be unprotected
        chat.id.delete(&alice).await?;

        let chat = alice.create_chat(&fiona_new).await;
        assert!(!chat.is_protected());
        assert!(!chat.is_protection_broken());
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create_unverified_oneonone_chat() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    // A chat with an unknown contact should be created unprotected
    let chat = alice.create_chat(&bob).await;
    assert!(!chat.is_protected());
    assert!(!chat.is_protection_broken());

    receive_imf(
        &alice,
        b"From: Bob <bob@example.net>\n\
          To: alice@example.org\n\
          Message-ID: <1234-2@example.org>\n\
          \n\
          hello\n",
        false,
    )
    .await?;

    chat.id.delete(&alice).await.unwrap();
    // Now Bob is a known contact, new chats should still be created unprotected
    let chat = alice.create_chat(&bob).await;
    assert!(!chat.is_protected());
    assert!(!chat.is_protection_broken());

    tcm.send_recv(&bob, &alice, "hi").await;
    chat.id.delete(&alice).await.unwrap();
    // Now we have a public key, new chats should still be created unprotected
    let chat = alice.create_chat(&bob).await;
    assert!(!chat.is_protected());
    assert!(!chat.is_protection_broken());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_degrade_verified_oneonone_chat() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    mark_as_verified(&alice, &bob).await;

    let alice_chat = alice.create_chat(&bob).await;
    assert!(alice_chat.is_protected());

    receive_imf(
        &alice,
        b"From: Bob <bob@example.net>\n\
          To: alice@example.org\n\
          Message-ID: <1234-2@example.org>\n\
          \n\
          hello\n",
        false,
    )
    .await?;

    let contact_id = Contact::lookup_id_by_addr(&alice, "bob@example.net", Origin::Hidden)
        .await?
        .unwrap();

    let msg0 = get_chat_msg(&alice, alice_chat.id, 0, 3).await;
    let enabled = stock_str::chat_protection_enabled(&alice).await;
    assert_eq!(msg0.text, enabled);
    assert_eq!(msg0.param.get_cmd(), SystemMessage::ChatProtectionEnabled);

    let msg1 = get_chat_msg(&alice, alice_chat.id, 1, 3).await;
    let disabled = stock_str::chat_protection_disabled(&alice, contact_id).await;
    assert_eq!(msg1.text, disabled);
    assert_eq!(msg1.param.get_cmd(), SystemMessage::ChatProtectionDisabled);

    let msg2 = get_chat_msg(&alice, alice_chat.id, 2, 3).await;
    assert_eq!(msg2.text, "hello".to_string());
    assert!(!msg2.is_system_message());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_verified_oneonone_chat_enable_disable() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    // Alice & Bob verify each other
    mark_as_verified(&alice, &bob).await;
    mark_as_verified(&bob, &alice).await;

    let chat = alice.create_chat(&bob).await;
    assert!(chat.is_protected());

    for alice_accepts_breakage in [true, false] {
        // Bob uses Thunderbird to send a message
        receive_imf(
            &alice,
            format!(
                "From: Bob <bob@example.net>\n\
              To: alice@example.org\n\
              Message-ID: <1234-2{alice_accepts_breakage}@example.org>\n\
              \n\
              Message from Thunderbird\n"
            )
            .as_bytes(),
            false,
        )
        .await?;

        let chat = alice.get_chat(&bob).await.unwrap();
        assert!(!chat.is_protected());
        assert!(chat.is_protection_broken());

        if alice_accepts_breakage {
            tcm.section("Alice clicks 'Accept' on the input-bar-dialog");
            chat.id.accept(&alice).await?;
            let chat = alice.get_chat(&bob).await.unwrap();
            assert!(!chat.is_protected());
            assert!(!chat.is_protection_broken());
        }

        // Bob sends a message from DC again
        tcm.send_recv(&bob, &alice, "Hello from DC").await;
        let chat = alice.get_chat(&bob).await.unwrap();
        assert!(chat.is_protected());
        assert!(!chat.is_protection_broken());
    }

    alice
        .golden_test_chat(chat.id, "test_verified_oneonone_chat_enable_disable")
        .await;

    Ok(())
}

/// Messages with old timestamps are difficult for verified chats:
/// - They must not be sorted over a protection-changed info message.
///   That's what `test_old_message_2` tests
/// - If they change the protection, then they must not be sorted over existing other messages,
///   because then the protection-changed info message would also be above these existing messages.
///   That's what `test_old_message_3` tests.
///
/// `test_old_message_1` tests the case where both the old and the new message
/// change verification
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_old_message_1() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    mark_as_verified(&alice, &bob).await;

    let chat = alice.create_chat(&bob).await; // This creates a protection-changed info message
    assert!(chat.is_protected());

    // This creates protection-changed info message #2;
    // even though the date is old, info message and email must be sorted below the original info message.
    receive_imf(
        &alice,
        b"From: Bob <bob@example.net>\n\
          To: alice@example.org\n\
          Message-ID: <1234-2-3@example.org>\n\
          Date: Sat, 07 Dec 2019 19:00:27 +0000\n\
          \n\
          Message from Thunderbird\n",
        true,
    )
    .await?;

    alice.golden_test_chat(chat.id, "test_old_message_1").await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_old_message_2() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    mark_as_verified(&alice, &bob).await;

    // This creates protection-changed info message #1:
    let chat = alice.create_chat(&bob).await;
    assert!(chat.is_protected());
    let protection_msg = alice.get_last_msg().await;
    assert_eq!(
        protection_msg.param.get_cmd(),
        SystemMessage::ChatProtectionEnabled
    );

    // This creates protection-changed info message #2.
    let first_email = receive_imf(
        &alice,
        b"From: Bob <bob@example.net>\n\
          To: alice@example.org\n\
          Message-ID: <1234-2-3@example.org>\n\
          Date: Sun, 08 Dec 2019 19:00:27 +0000\n\
          \n\
          Somewhat old message\n",
        false,
    )
    .await?
    .unwrap();

    // Both messages will get the same timestamp as the protection-changed
    // message, so this one will be sorted under the previous one
    // even though it has an older timestamp.
    let second_email = receive_imf(
        &alice,
        b"From: Bob <bob@example.net>\n\
          To: alice@example.org\n\
          Message-ID: <2319-2-3@example.org>\n\
          Date: Sat, 07 Dec 2019 19:00:27 +0000\n\
          \n\
          Even older message, that must NOT be shown before the info message\n",
        true,
    )
    .await?
    .unwrap();

    assert_eq!(first_email.sort_timestamp, second_email.sort_timestamp);
    assert_eq!(first_email.sort_timestamp, protection_msg.timestamp_sort);

    alice.golden_test_chat(chat.id, "test_old_message_2").await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_old_message_3() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    mark_as_verified(&alice, &bob).await;
    mark_as_verified(&bob, &alice).await;

    tcm.send_recv_accept(&bob, &alice, "Heyho from my verified device!")
        .await;

    // This unverified message must not be sorted over the message sent in the previous line:
    receive_imf(
        &alice,
        b"From: Bob <bob@example.net>\n\
          To: alice@example.org\n\
          Message-ID: <1234-2-3@example.org>\n\
          Date: Sat, 07 Dec 2019 19:00:27 +0000\n\
          \n\
          Old, unverified message\n",
        true,
    )
    .await?;

    alice
        .golden_test_chat(alice.get_chat(&bob).await.unwrap().id, "test_old_message_3")
        .await;

    Ok(())
}

/// Alice is offline for some time.
/// When she comes online, first her inbox is synced and then her sentbox.
/// This test tests that the messages are still in the right order.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_old_message_4() -> Result<()> {
    let alice = TestContext::new_alice().await;
    let msg_incoming = receive_imf(
        &alice,
        b"From: Bob <bob@example.net>\n\
          To: alice@example.org\n\
          Message-ID: <1234-2-3@example.org>\n\
          Date: Sun, 08 Dec 2019 19:00:27 +0000\n\
          \n\
          Thanks, Alice!\n",
        true,
    )
    .await?
    .unwrap();

    let msg_sent = receive_imf(
        &alice,
        b"From: alice@example.org\n\
          To: Bob <bob@example.net>\n\
          Message-ID: <1234-2-4@example.org>\n\
          Date: Sat, 07 Dec 2019 19:00:27 +0000\n\
          \n\
          Happy birthday, Bob!\n",
        true,
    )
    .await?
    .unwrap();

    // The "Happy birthday" message should be shown first, and then the "Thanks" message
    assert!(msg_sent.sort_timestamp < msg_incoming.sort_timestamp);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_mdn_doesnt_disable_verification() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;
    bob.set_config_bool(Config::MdnsEnabled, true).await?;

    // Alice & Bob verify each other
    mark_as_verified(&alice, &bob).await;
    mark_as_verified(&bob, &alice).await;

    let rcvd = tcm.send_recv_accept(&alice, &bob, "Heyho").await;
    message::markseen_msgs(&bob, vec![rcvd.id]).await?;

    let mimefactory = MimeFactory::from_mdn(&bob, &rcvd, vec![]).await?;
    let rendered_msg = mimefactory.render(&bob).await?;
    let body = rendered_msg.message;
    receive_imf(&alice, body.as_bytes(), false).await.unwrap();

    assert_verified(&alice, &bob, ProtectionStatus::Protected).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_outgoing_mua_msg() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    mark_as_verified(&alice, &bob).await;
    mark_as_verified(&bob, &alice).await;

    tcm.send_recv_accept(&bob, &alice, "Heyho from DC").await;
    assert_verified(&alice, &bob, ProtectionStatus::Protected).await;

    let sent = receive_imf(
        &alice,
        b"From: alice@example.org\n\
          To: bob@example.net\n\
          \n\
          One classical MUA message",
        false,
    )
    .await?
    .unwrap();
    tcm.send_recv(&alice, &bob, "Sending with DC again").await;

    alice
        .golden_test_chat(sent.chat_id, "test_outgoing_mua_msg")
        .await;

    Ok(())
}

/// If Bob answers unencrypted from another address with a classical MUA,
/// the message is under some circumstances still assigned to the original
/// chat (see lookup_chat_by_reply()); this is meant to make aliases
/// work nicely.
/// However, if the original chat is verified, the unencrypted message
/// must NOT be assigned to it (it would be replaced by an error
/// message in the verified chat, so, this would just be a usability issue,
/// not a security issue).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reply() -> Result<()> {
    for verified in [false, true] {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;
        enable_verified_oneonone_chats(&[&alice, &bob]).await;

        if verified {
            mark_as_verified(&alice, &bob).await;
            mark_as_verified(&bob, &alice).await;
        }

        tcm.send_recv_accept(&bob, &alice, "Heyho from DC").await;
        let encrypted_msg = tcm.send_recv(&alice, &bob, "Heyho back").await;

        let unencrypted_msg = receive_imf(
            &alice,
            format!(
                "From: bob@someotherdomain.org\n\
                 To: some-alias-forwarding-to-alice@example.org\n\
                 In-Reply-To: {}\n\
                 \n\
                 Weird reply",
                encrypted_msg.rfc724_mid
            )
            .as_bytes(),
            false,
        )
        .await?
        .unwrap();

        let unencrypted_msg = Message::load_from_db(&alice, unencrypted_msg.msg_ids[0]).await?;
        assert_eq!(unencrypted_msg.text, "Weird reply");

        if verified {
            assert_ne!(unencrypted_msg.chat_id, encrypted_msg.chat_id);
        } else {
            assert_eq!(unencrypted_msg.chat_id, encrypted_msg.chat_id);
        }
    }

    Ok(())
}

/// Regression test for the following bug:
///
/// - Scan your chat partner's QR Code
/// - They change devices
/// - They send you a message
/// - Without accepting the encryption downgrade, scan your chat partner's QR Code again
///
/// -> The re-verification fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_break_protection_then_verify_again() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice, &bob]).await;

    // Cave: Bob can't write a message to Alice here.
    // If he did, alice would increase his peerstate's last_seen timestamp.
    // Then, after Bob reinstalls DC, alice's `if message_time > last_seen*`
    // checks would return false (there are many checks of this form in peerstate.rs).
    // Therefore, during the securejoin, Alice wouldn't accept the new key
    // and reject the securejoin.

    mark_as_verified(&alice, &bob).await;
    mark_as_verified(&bob, &alice).await;

    alice.create_chat(&bob).await;
    assert_verified(&alice, &bob, ProtectionStatus::Protected).await;

    tcm.section("Bob reinstalls DC");
    drop(bob);
    let bob_new = tcm.unconfigured().await;
    enable_verified_oneonone_chats(&[&bob_new]).await;
    bob_new.configure_addr("bob@example.net").await;
    e2ee::ensure_secret_key_exists(&bob_new).await?;

    tcm.send_recv(&bob_new, &alice, "I have a new device").await;

    let contact = alice.add_or_lookup_contact(&bob_new).await;
    assert_eq!(
        contact.is_verified(&alice).await.unwrap(),
        // Bob sent a message with a new key, so he most likely doesn't have
        // the old key anymore. This means that Alice's device should show
        // him as unverified:
        VerifiedStatus::Unverified
    );
    let chat = alice.get_chat(&bob_new).await.unwrap();
    assert_eq!(chat.is_protected(), false);
    assert_eq!(chat.is_protection_broken(), true);

    {
        let alice_bob_chat = alice.get_chat(&bob_new).await.unwrap();
        assert!(!alice_bob_chat.can_send(&alice).await?);

        // Alice's UI should still be able to save a draft, which Alice started to type right when she got Bob's message:
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text("Draftttt".to_string());
        alice_bob_chat.id.set_draft(&alice, Some(&mut msg)).await?;
        assert_eq!(
            alice_bob_chat.id.get_draft(&alice).await?.unwrap().text,
            "Draftttt"
        );
    }

    tcm.execute_securejoin(&alice, &bob_new).await;
    assert_verified(&alice, &bob_new, ProtectionStatus::Protected).await;

    Ok(())
}

/// Regression test:
/// - Verify a contact
/// - The contact stops using DC and sends a message from a classical MUA instead
/// - Delete the 1:1 chat
/// - Create a 1:1 chat
/// - Check that the created chat is not marked as protected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create_oneonone_chat_with_former_verified_contact() -> Result<()> {
    let mut tcm = TestContextManager::new();
    let alice = tcm.alice().await;
    let bob = tcm.bob().await;
    enable_verified_oneonone_chats(&[&alice]).await;

    mark_as_verified(&alice, &bob).await;

    receive_imf(
        &alice,
        b"Subject: Message from bob\r\n\
          From: <bob@example.net>\r\n\
          To: <alice@example.org>\r\n\
          Date: Mon, 12 Dec 2022 14:33:39 +0000\r\n\
          Message-ID: <abcd@example.net>\r\n\
          \r\n\
          Heyho!\r\n",
        false,
    )
    .await
    .unwrap()
    .unwrap();

    alice.create_chat(&bob).await;

    assert_verified(&alice, &bob, ProtectionStatus::Unprotected).await;

    Ok(())
}

// ============== Helper Functions ==============

async fn assert_verified(this: &TestContext, other: &TestContext, protected: ProtectionStatus) {
    let contact = this.add_or_lookup_contact(other).await;
    assert_eq!(
        contact.is_verified(this).await.unwrap(),
        VerifiedStatus::BidirectVerified
    );

    let chat = this.get_chat(other).await.unwrap();
    let (expect_protected, expect_broken) = match protected {
        ProtectionStatus::Unprotected => (false, false),
        ProtectionStatus::Protected => (true, false),
        ProtectionStatus::ProtectionBroken => (false, true),
    };
    assert_eq!(chat.is_protected(), expect_protected);
    assert_eq!(chat.is_protection_broken(), expect_broken);
}

async fn enable_verified_oneonone_chats(test_contexts: &[&TestContext]) {
    for t in test_contexts {
        t.set_config_bool(Config::VerifiedOneOnOneChats, true)
            .await
            .unwrap()
    }
}

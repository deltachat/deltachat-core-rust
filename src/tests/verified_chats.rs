use anyhow::Result;
use pretty_assertions::assert_eq;

use crate::chat::{Chat, ProtectionStatus};
use crate::contact::VerifiedStatus;
use crate::contact::{Contact, Origin};
use crate::e2ee;
use crate::mimeparser::SystemMessage;
use crate::receive_imf::receive_imf;
use crate::stock_str;
use crate::test_utils::{get_chat_msg, mark_as_verified, TestContext, TestContextManager};

// TODO read receipts shoudn't be able to change the verification status
// TODO when testing with Marine, I had multiple installations of DC. This somehow broke things.

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
            get_chat_msg(&fiona, msg.chat_id, 0, 2).await.get_info_type(),
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
        let expected_text =
            stock_str::chat_verification_enabled(&alice, alice_fiona_contact.id).await;
        assert_eq!(msg.text.unwrap(), expected_text);
    }

    // Fiona should also see the chat as protected
    {
        let rcvd = tcm.send_recv(&alice, &fiona, "Hi Fiona").await;
        let alice_fiona_id = rcvd.chat_id;
        let chat = Chat::load_from_db(&fiona, alice_fiona_id).await?;
        assert!(chat.is_protected());

        let msg0 = get_chat_msg(&fiona, chat.id, 0, 2).await;
        let contact_id = Contact::lookup_id_by_addr(&fiona, "alice@example.org", Origin::Hidden)
            .await?
            .unwrap();
        let expected_text = stock_str::chat_verification_enabled(&fiona, contact_id).await;
        assert_eq!(msg0.text.unwrap(), expected_text);
    }

    tcm.section("Fiona reinstalls DC");
    drop(fiona);

    let fiona_new = tcm.unconfigured().await;
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
    let enabled = stock_str::chat_verification_enabled(&alice, contact_id).await;
    assert_eq!(msg0.text, enabled);
    assert_eq!(msg0.param.get_cmd(), SystemMessage::ChatProtectionEnabled);

    let msg1 = get_chat_msg(&alice, alice_chat.id, 1, 3).await;
    let disabled = stock_str::chat_verification_disabled(&alice, contact_id).await;
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

import logging

from deltachat_rpc_client import Chat, SpecialContactId


def test_qr_setup_contact(acfactory) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    qr_code, _svg = alice.get_qr_code()
    bob.secure_join(qr_code)

    alice.wait_for_securejoin_inviter_success()

    # Test that Alice verified Bob's profile.
    alice_contact_bob = alice.get_contact_by_addr(bob.get_config("addr"))
    alice_contact_bob_snapshot = alice_contact_bob.get_snapshot()
    assert alice_contact_bob_snapshot.is_verified

    bob.wait_for_securejoin_joiner_success()

    # Test that Bob verified Alice's profile.
    bob_contact_alice = bob.get_contact_by_addr(alice.get_config("addr"))
    bob_contact_alice_snapshot = bob_contact_alice.get_snapshot()
    assert bob_contact_alice_snapshot.is_verified


def test_qr_securejoin(acfactory):
    alice, bob = acfactory.get_online_accounts(2)

    logging.info("Alice creates a verified group")
    alice_chat = alice.create_group("Verified group", protect=True)

    logging.info("Bob joins verified group")
    qr_code, _svg = alice_chat.get_qr_code()
    bob.secure_join(qr_code)

    alice.wait_for_securejoin_inviter_success()

    # Test that Alice verified Bob's profile.
    alice_contact_bob = alice.get_contact_by_addr(bob.get_config("addr"))
    alice_contact_bob_snapshot = alice_contact_bob.get_snapshot()
    assert alice_contact_bob_snapshot.is_verified

    bob.wait_for_securejoin_joiner_success()

    # Test that Bob verified Alice's profile.
    bob_contact_alice = bob.get_contact_by_addr(alice.get_config("addr"))
    bob_contact_alice_snapshot = bob_contact_alice.get_snapshot()
    assert bob_contact_alice_snapshot.is_verified


def test_qr_securejoin_contact_request(acfactory) -> None:
    """Alice invites Bob to a group when Bob's chat with Alice is in a contact request mode."""
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()
    alice_chat_bob.send_text("Hello!")

    snapshot = bob.get_message_by_id(bob.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Hello!"
    bob_chat_alice = snapshot.chat
    assert bob_chat_alice.get_basic_snapshot().is_contact_request

    alice_chat = alice.create_group("Verified group", protect=True)
    logging.info("Bob joins verified group")
    qr_code, _svg = alice_chat.get_qr_code()
    bob.secure_join(qr_code)
    while True:
        event = bob.wait_for_event()
        if event["kind"] == "SecurejoinJoinerProgress" and event["progress"] == 1000:
            break

    # Chat stays being a contact request.
    assert bob_chat_alice.get_basic_snapshot().is_contact_request


def test_qr_readreceipt(acfactory) -> None:
    alice, bob, charlie = acfactory.get_online_accounts(3)

    logging.info("Bob and Charlie setup contact with Alice")
    qr_code, _svg = alice.get_qr_code()

    bob.secure_join(qr_code)
    charlie.secure_join(qr_code)

    for joiner in [bob, charlie]:
        joiner.wait_for_securejoin_joiner_success()

    logging.info("Alice creates a verified group")
    group = alice.create_group("Group", protect=True)

    bob_addr = bob.get_config("addr")
    charlie_addr = charlie.get_config("addr")

    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_contact_charlie = alice.create_contact(charlie_addr, "Charlie")

    group.add_contact(alice_contact_bob)
    group.add_contact(alice_contact_charlie)

    # Promote a group.
    group.send_message(text="Hello")

    logging.info("Bob and Charlie receive a group")

    bob_msg_id = bob.wait_for_incoming_msg_event().msg_id
    bob_message = bob.get_message_by_id(bob_msg_id)
    bob_snapshot = bob_message.get_snapshot()
    assert bob_snapshot.text == "Hello"

    # Charlie receives the same "Hello" message as Bob.
    charlie.wait_for_incoming_msg_event()

    logging.info("Bob sends a message to the group")

    bob_out_message = bob_snapshot.chat.send_message(text="Hi from Bob!")

    charlie_msg_id = charlie.wait_for_incoming_msg_event().msg_id
    charlie_message = charlie.get_message_by_id(charlie_msg_id)
    charlie_snapshot = charlie_message.get_snapshot()
    assert charlie_snapshot.text == "Hi from Bob!"

    bob_contact_charlie = bob.create_contact(charlie_addr, "Charlie")
    assert not bob.get_chat_by_contact(bob_contact_charlie)

    logging.info("Charlie reads Bob's message")
    charlie_message.mark_seen()

    while True:
        event = bob.wait_for_event()
        if event["kind"] == "MsgRead" and event["msg_id"] == bob_out_message.id:
            break

    # Receiving a read receipt from Charlie
    # should not unblock hidden chat with Charlie for Bob.
    assert not bob.get_chat_by_contact(bob_contact_charlie)


def test_verified_group_recovery(acfactory) -> None:
    """Tests verified group recovery by reverifying a member and sending a message in a group."""
    ac1, ac2, ac3 = acfactory.get_online_accounts(3)

    logging.info("ac1 creates verified group")
    chat = ac1.create_group("Verified group", protect=True)
    assert chat.get_basic_snapshot().is_protected

    logging.info("ac2 joins verified group")
    qr_code, _svg = chat.get_qr_code()
    ac2.secure_join(qr_code)
    ac1.wait_for_securejoin_inviter_success()

    # ac1 has ac2 directly verified.
    ac1_contact_ac2 = ac1.get_contact_by_addr(ac2.get_config("addr"))
    assert ac1_contact_ac2.get_snapshot().verifier_id == SpecialContactId.SELF

    logging.info("ac3 joins verified group")
    ac3_chat = ac3.secure_join(qr_code)
    ac1.wait_for_securejoin_inviter_success()

    logging.info("ac2 logs in on a new device")
    ac2 = acfactory.resetup_account(ac2)

    logging.info("ac2 reverifies with ac3")
    qr_code, _svg = ac3.get_qr_code()
    ac2.secure_join(qr_code)

    ac3.wait_for_securejoin_inviter_success()

    logging.info("ac3 sends a message to the group")
    assert len(ac3_chat.get_contacts()) == 3
    ac3_chat.send_text("Hi!")

    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Hi!"

    msg_id = ac2.wait_for_incoming_msg_event().msg_id
    message = ac2.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.text == "Hi!"

    # ac1 contact is verified for ac2 because ac3 gossiped ac1 key in the "Hi!" message.
    ac1_contact = ac2.get_contact_by_addr(ac1.get_config("addr"))
    assert ac1_contact.get_snapshot().is_verified

    # ac2 can write messages to the group.
    snapshot.chat.send_text("Works again!")

    snapshot = ac3.get_message_by_id(ac3.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Works again!"

    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Works again!"

    ac1_chat_messages = snapshot.chat.get_messages()
    ac2_addr = ac2.get_config("addr")
    assert ac1_chat_messages[-2].get_snapshot().text == f"Changed setup for {ac2_addr}"

    # ac2 is now verified by ac3 for ac1
    ac1_contact_ac3 = ac1.get_contact_by_addr(ac3.get_config("addr"))
    assert ac1_contact_ac2.get_snapshot().verifier_id == ac1_contact_ac3.id


def test_verified_group_member_added_recovery(acfactory) -> None:
    """Tests verified group recovery by reverifiying than removing and adding a member back."""
    ac1, ac2, ac3 = acfactory.get_online_accounts(3)

    logging.info("ac1 creates verified group")
    chat = ac1.create_group("Verified group", protect=True)
    assert chat.get_basic_snapshot().is_protected

    logging.info("ac2 joins verified group")
    qr_code, _svg = chat.get_qr_code()
    ac2.secure_join(qr_code)
    ac1.wait_for_securejoin_inviter_success()

    # ac1 has ac2 directly verified.
    ac1_contact_ac2 = ac1.get_contact_by_addr(ac2.get_config("addr"))
    assert ac1_contact_ac2.get_snapshot().verifier_id == SpecialContactId.SELF

    logging.info("ac3 joins verified group")
    ac3_chat = ac3.secure_join(qr_code)
    ac1.wait_for_securejoin_inviter_success()

    logging.info("ac2 logs in on a new device")
    ac2 = acfactory.resetup_account(ac2)

    logging.info("ac2 reverifies with ac3")
    qr_code, _svg = ac3.get_qr_code()
    ac2.secure_join(qr_code)

    ac3.wait_for_securejoin_inviter_success()

    logging.info("ac3 sends a message to the group")
    assert len(ac3_chat.get_contacts()) == 3
    ac3_chat.send_text("Hi!")

    msg_id = ac2.wait_for_incoming_msg_event().msg_id
    message = ac2.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    logging.info("Received message %s", snapshot.text)
    assert snapshot.text == "Hi!"

    ac1.wait_for_incoming_msg_event()  # Hi!

    ac3_contact_ac2 = ac3.get_contact_by_addr(ac2.get_config("addr"))
    ac3_chat.remove_contact(ac3_contact_ac2)
    ac3_chat.add_contact(ac3_contact_ac2)

    msg_id = ac2.wait_for_incoming_msg_event().msg_id
    message = ac2.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert "removed" in snapshot.text

    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert "removed" in snapshot.text

    event = ac2.wait_for_incoming_msg_event()
    msg_id = event.msg_id
    chat_id = event.chat_id
    message = ac2.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    logging.info("ac2 got event message: %s", snapshot.text)
    assert "added" in snapshot.text

    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert "added" in snapshot.text

    chat = Chat(ac2, chat_id)
    chat.send_text("Works again!")

    msg_id = ac3.wait_for_incoming_msg_event().msg_id
    message = ac3.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.text == "Works again!"

    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Works again!"

    ac1_contact_ac2 = ac1.get_contact_by_addr(ac2.get_config("addr"))
    ac1_contact_ac2_snapshot = ac1_contact_ac2.get_snapshot()
    assert ac1_contact_ac2_snapshot.is_verified
    assert ac1_contact_ac2_snapshot.verifier_id == ac1.get_contact_by_addr(ac3.get_config("addr")).id

    # ac2 is now verified by ac3 for ac1
    ac1_contact_ac3 = ac1.get_contact_by_addr(ac3.get_config("addr"))
    assert ac1_contact_ac2.get_snapshot().verifier_id == ac1_contact_ac3.id

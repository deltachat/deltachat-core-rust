import logging

from deltachat_rpc_client import Chat, SpecialContactId


def test_qr_setup_contact(acfactory) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    qr_code, _svg = alice.get_qr_code()
    bob.secure_join(qr_code)

    while True:
        event = alice.wait_for_event()
        if event["kind"] == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

    # Test that Alice verified Bob's profile.
    alice_contact_bob = alice.get_contact_by_addr(bob.get_config("addr"))
    alice_contact_bob_snapshot = alice_contact_bob.get_snapshot()
    assert alice_contact_bob_snapshot.is_verified

    while True:
        event = bob.wait_for_event()
        if event["kind"] == "SecurejoinJoinerProgress" and event["progress"] == 1000:
            break

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
    while True:
        event = alice.wait_for_event()
        if event.kind == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

    # Test that Alice verified Bob's profile.
    alice_contact_bob = alice.get_contact_by_addr(bob.get_config("addr"))
    alice_contact_bob_snapshot = alice_contact_bob.get_snapshot()
    assert alice_contact_bob_snapshot.is_verified

    while True:
        event = bob.wait_for_event()
        if event["kind"] == "SecurejoinJoinerProgress" and event["progress"] == 1000:
            break

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


def test_verified_group_recovery(acfactory, rpc) -> None:
    ac1, ac2, ac3 = acfactory.get_online_accounts(3)

    logging.info("ac1 creates verified group")
    chat = ac1.create_group("Verified group", protect=True)
    assert chat.get_basic_snapshot().is_protected

    logging.info("ac2 joins verified group")
    qr_code, _svg = chat.get_qr_code()
    ac2.secure_join(qr_code)
    while True:
        event = ac1.wait_for_event()
        if event.kind == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

    # ac1 has ac2 directly verified.
    ac1_contact_ac2 = ac1.get_contact_by_addr(ac2.get_config("addr"))
    assert ac1_contact_ac2.get_snapshot().verifier_id == SpecialContactId.SELF

    logging.info("ac3 joins verified group")
    ac3_chat = ac3.secure_join(qr_code)
    while True:
        event = ac1.wait_for_event()
        if event.kind == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

    logging.info("ac2 logs in on a new device")
    ac2 = acfactory.resetup_account(ac2)

    logging.info("ac2 reverifies with ac3")
    qr_code, _svg = ac3.get_qr_code()
    ac2.secure_join(qr_code)

    while True:
        event = ac3.wait_for_event()
        if event.kind == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

    logging.info("ac3 sends a message to the group")
    assert len(ac3_chat.get_contacts()) == 3
    ac3_chat.send_text("Hi!")

    msg_id = ac2.wait_for_incoming_msg_event().msg_id
    message = ac2.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    logging.info("Received message %s", snapshot.text)
    assert snapshot.text == "Hi!"

    # ac1 contact cannot be verified by ac2 because ac3 did not gossip ac1 key in the "Hi!" message.
    ac1_contact = ac2.get_contact_by_addr(ac1.get_config("addr"))
    assert not ac1_contact.get_snapshot().is_verified

    ac3_contact_id_ac1 = rpc.lookup_contact_id_by_addr(ac3.id, ac1.get_config("addr"))
    ac3_chat.remove_contact(ac3_contact_id_ac1)
    ac3_chat.add_contact(ac3_contact_id_ac1)

    msg_id = ac2.wait_for_incoming_msg_event().msg_id
    message = ac2.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    logging.info("ac2 got event message: %s", snapshot.text)
    assert "removed" in snapshot.text

    event = ac2.wait_for_incoming_msg_event()
    msg_id = event.msg_id
    chat_id = event.chat_id
    message = ac2.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    logging.info("ac2 got event message: %s", snapshot.text)
    assert "added" in snapshot.text

    assert ac1_contact.get_snapshot().is_verified

    chat = Chat(ac2, chat_id)
    chat.send_text("Works again!")

    msg_id = ac3.wait_for_incoming_msg_event().msg_id
    message = ac3.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.text == "Works again!"

    ac1.wait_for_incoming_msg_event()  # Hi!
    ac1.wait_for_incoming_msg_event()  # Member removed
    ac1.wait_for_incoming_msg_event()  # Member added
    snapshot = ac1.get_message_by_id(ac1.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Works again!"

    # ac2 is now verified by ac3 for ac1
    ac1_contact_ac3 = ac1.get_contact_by_addr(ac3.get_config("addr"))
    assert ac1_contact_ac2.get_snapshot().verifier_id == ac1_contact_ac3.id

    ac1_chat_messages = snapshot.chat.get_messages()
    ac2_addr = ac2.get_config("addr")
    assert ac1_chat_messages[-2].get_snapshot().text == f"Changed setup for {ac2_addr}"


def test_verified_group_member_added_recovery(acfactory) -> None:
    ac1, ac2, ac3 = acfactory.get_online_accounts(3)

    logging.info("ac1 creates verified group")
    chat = ac1.create_group("Verified group", protect=True)
    assert chat.get_basic_snapshot().is_protected

    logging.info("ac2 joins verified group")
    qr_code, _svg = chat.get_qr_code()
    ac2.secure_join(qr_code)
    while True:
        event = ac1.wait_for_event()
        if event.kind == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

    # ac1 has ac2 directly verified.
    ac1_contact_ac2 = ac1.get_contact_by_addr(ac2.get_config("addr"))
    assert ac1_contact_ac2.get_snapshot().verifier_id == SpecialContactId.SELF

    logging.info("ac3 joins verified group")
    ac3_chat = ac3.secure_join(qr_code)
    while True:
        event = ac1.wait_for_event()
        if event.kind == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

    logging.info("ac2 logs in on a new device")
    ac2 = acfactory.resetup_account(ac2)

    logging.info("ac2 reverifies with ac3")
    qr_code, _svg = ac3.get_qr_code()
    ac2.secure_join(qr_code)

    while True:
        event = ac3.wait_for_event()
        if event.kind == "SecurejoinInviterProgress" and event["progress"] == 1000:
            break

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

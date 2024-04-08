import base64
import os

from deltachat_rpc_client import Account, EventType, const


def wait_for_chatlist_order_and_specific_item(account, chat_id):
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_CHANGED:
            break
        if event.kind == EventType.CHATLIST_ITEM_CHANGED and event.chat_id == chat_id:
            break
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_CHANGED:
            break
        if event.kind == EventType.CHATLIST_ITEM_CHANGED and event.chat_id == chat_id:
            break


def wait_for_chatlist_specific_item(account, chat_id):
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_ITEM_CHANGED and event.chat_id == chat_id:
            break


def wait_for_chatlist_order(account):
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_CHANGED:
            break


def test_delivery_status(acfactory) -> None:
    """
    Test change status on chatlistitem when status changes (delivered, read)
    """
    # explicit type Annotations are needed for vscode
    alice: Account
    bob: Account
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()

    alice.clear_all_events()
    bob.stop_io()
    alice.stop_io()
    alice_chat_bob.send_text("hi")
    wait_for_chatlist_order_and_specific_item(alice, chat_id=alice_chat_bob.id)

    alice.clear_all_events()
    alice.start_io()
    wait_for_chatlist_specific_item(alice, chat_id=alice_chat_bob.id)

    bob.clear_all_events()
    bob.start_io()

    while True:
        event = bob.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            msg = bob.get_message_by_id(event.msg_id)
            bob._rpc.accept_chat(bob.id, msg.get_snapshot().chat_id)
            bob.mark_seen_messages([msg])
            break

    chat_item = alice._rpc.get_chatlist_items_by_entries(alice.id, [alice_chat_bob.id])[str(alice_chat_bob.id)]
    assert chat_item["summaryStatus"] == const.MessageState.OUT_DELIVERED

    alice.clear_all_events()

    while True:
        event = alice.wait_for_event()
        if event.kind == EventType.MSG_READ:
            break

    wait_for_chatlist_specific_item(alice, chat_id=alice_chat_bob.id)
    chat_item = alice._rpc.get_chatlist_items_by_entries(alice.id, [alice_chat_bob.id])[str(alice_chat_bob.id)]
    assert chat_item["summaryStatus"] == const.MessageState.OUT_MDN_RCVD


def test_delivery_status_failed(acfactory) -> None:
    """
    Test change status on chatlistitem when status changes failed
    """
    # explicit type Annotations are needed for vscode
    alice: Account
    (alice,) = acfactory.get_online_accounts(1)

    invalid_contact = alice.create_contact("example@example.com", "invalid address")
    invalid_chat = alice.get_chat_by_id(alice._rpc.create_chat_by_contact_id(alice.id, invalid_contact.id))

    alice.clear_all_events()

    failing_message = invalid_chat.send_text("test")

    wait_for_chatlist_order_and_specific_item(alice, invalid_chat.id)

    assert failing_message.get_snapshot().state == const.MessageState.OUT_PENDING

    while True:
        event = alice.wait_for_event()
        if event.kind == EventType.MSG_FAILED:
            break

    wait_for_chatlist_specific_item(alice, invalid_chat.id)

    assert failing_message.get_snapshot().state == const.MessageState.OUT_FAILED


def test_download_on_demand(acfactory) -> None:
    """
    Test if download on demand emits chatlist update events.
    This is only needed for last message in chat, but finding that out is too expensive, so it's always emitted
    """
    # explicit type Annotations are needed for vscode
    alice: Account
    bob: Account
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()
    alice_chat_bob.send_text("hi")

    alice.set_config("download_limit", "1")

    while True:
        event = bob.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            msg = bob.get_message_by_id(event.msg_id)
            chat_id = msg.get_snapshot().chat_id
            bob._rpc.accept_chat(bob.id, msg.get_snapshot().chat_id)
            bob.get_chat_by_id(chat_id).send_message(
                "Hello World, this message is bigger than 5 bytes",
                html=base64.b64encode(os.urandom(300000)).decode("utf-8"),
            )
            break

    while True:
        event = alice.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            msg_id = event.msg_id
            break

    assert alice.get_message_by_id(msg_id).get_snapshot().download_state == const.DownloadState.AVAILABLE

    alice.clear_all_events()
    chat_id = alice.get_message_by_id(msg_id).get_snapshot().chat_id
    alice._rpc.download_full_message(alice.id, msg_id)

    wait_for_chatlist_specific_item(alice, chat_id)


def get_multi_account_test_setup(acfactory) -> [Account, Account, Account]:
    # explicit type Annotations are needed for vscode
    alice: Account
    bob: Account
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()
    alice_chat_bob.send_text("hi")

    while True:
        event = bob.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            break

    alice_second_device: Account = acfactory.get_unconfigured_account()

    alice._rpc.provide_backup.future(alice.id)
    backup_code = alice._rpc.get_backup_qr(alice.id)
    alice_second_device._rpc.get_backup(alice_second_device.id, backup_code)
    alice_second_device.start_io()
    alice.clear_all_events()
    alice_second_device.clear_all_events()
    bob.clear_all_events()
    return [alice, alice_second_device, bob, alice_chat_bob]


def test_imap_sync_seen_msgs(acfactory) -> None:
    """
    Test that chatlist changed events are emitted for the second device
    when the message is marked as read on the first device
    """
    alice, alice_second_device, bob, alice_chat_bob = get_multi_account_test_setup(acfactory)

    alice_chat_bob.send_text("hello")
    while True:
        event = bob.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            msg = bob.get_message_by_id(event.msg_id)
            bob_chat_id = msg.get_snapshot().chat_id
            bob._rpc.accept_chat(bob.id, bob_chat_id)
            break

    alice.clear_all_events()
    alice_second_device.clear_all_events()
    bob.get_chat_by_id(bob_chat_id).send_text("hello")

    # make sure alice_second_device already received the message
    while True:
        event = alice_second_device.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            break

    while True:
        event = alice.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            msg = alice.get_message_by_id(event.msg_id)
            alice_second_device.clear_all_events()
            alice.mark_seen_messages([msg])
            break

    wait_for_chatlist_specific_item(bob, bob_chat_id)
    wait_for_chatlist_specific_item(alice, alice_chat_bob.id)


def test_multidevice_sync_chat(acfactory) -> None:
    """
    Test multidevice sync: syncing chat visibility and muting across multiple devices
    """
    alice, alice_second_device, bob, alice_chat_bob = get_multi_account_test_setup(acfactory)

    alice_chat_bob.archive()
    wait_for_chatlist_specific_item(alice_second_device, alice_chat_bob.id)
    assert alice_second_device.get_chat_by_id(alice_chat_bob.id).get_basic_snapshot().archived

    alice_second_device.clear_all_events()
    alice_chat_bob.pin()
    wait_for_chatlist_specific_item(alice_second_device, alice_chat_bob.id)

    alice_second_device.clear_all_events()
    alice_chat_bob.mute()
    wait_for_chatlist_specific_item(alice_second_device, alice_chat_bob.id)
    assert alice_second_device.get_chat_by_id(alice_chat_bob.id).get_basic_snapshot().is_muted

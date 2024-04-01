import logging
import threading
import time

import pytest
from deltachat_rpc_client import Chat, SpecialContactId, EventType, Account, const


def wait_for_chatlist_order_and_specific_item(account, chat_id):
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_CHANGED:
            break
        if event.kind == EventType.CHATLIST_ITEM_CHANGED:
            if event.chat_id == chat_id:
                break
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_CHANGED:
            break
        if event.kind == EventType.CHATLIST_ITEM_CHANGED:
            if event.chat_id == chat_id:
                break

def wait_for_chatlist_specific_item(account, chat_id):
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_ITEM_CHANGED:
            if event.chat_id == chat_id:
                break

def wait_for_chatlist_order(account):
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CHATLIST_CHANGED:
            break


def test_delivery_status(acfactory, tmp_path) -> None:
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
    msg_id = alice_chat_bob.send_text("hi")
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

def test_delivery_status_failed(acfactory, tmp_path) -> None:
    """
    Test change status on chatlistitem when status changes failed
    """
    # explicit type Annotations are needed for vscode
    alice: Account
    alice, = acfactory.get_online_accounts(1)

    invalid_contact = alice.create_contact("example@example.com", "invalid address")
    invalid_chat = alice.get_chat_by_id(alice._rpc.create_chat_by_contact_id(alice.id,invalid_contact.id))

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





    

# TODO
# - [ ] Download on demand on last message in chat
# - [ ] change protection (1:1 chat gets guranteed encryption)
# - [ ] Imap sync seen messages - chatlist item should update
# - [ ] multidevice sync (chat visibility; chat muted)
# - [ ] syncing chat visibility and muting across multiple devices
# - [ ] Chatlist correctly updated after AEAP

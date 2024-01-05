import concurrent.futures
import json
import subprocess
from unittest.mock import MagicMock

import pytest
from deltachat_rpc_client import EventType, events
from deltachat_rpc_client.rpc import JsonRpcError


def test_system_info(rpc) -> None:
    system_info = rpc.get_system_info()
    assert "arch" in system_info
    assert "deltachat_core_version" in system_info


def test_sleep(rpc) -> None:
    """Test that long-running task does not block short-running task from completion."""
    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
        sleep_5_future = executor.submit(rpc.sleep, 5.0)
        sleep_3_future = executor.submit(rpc.sleep, 3.0)
        done, pending = concurrent.futures.wait(
            [sleep_5_future, sleep_3_future],
            return_when=concurrent.futures.FIRST_COMPLETED,
        )
        assert sleep_3_future in done
        assert sleep_5_future in pending


def test_email_address_validity(rpc) -> None:
    valid_addresses = [
        "email@example.com",
        "36aa165ae3406424e0c61af17700f397cad3fe8ab83d682d0bddf3338a5dd52e@yggmail@yggmail",
    ]
    invalid_addresses = ["email@", "example.com", "emai221"]

    for addr in valid_addresses:
        assert rpc.check_email_validity(addr)
    for addr in invalid_addresses:
        assert not rpc.check_email_validity(addr)


def test_acfactory(acfactory) -> None:
    account = acfactory.new_configured_account()
    while True:
        event = account.wait_for_event()
        if event.kind == EventType.CONFIGURE_PROGRESS:
            assert event.progress != 0  # Progress 0 indicates error.
            if event.progress == 1000:  # Success
                break
        else:
            print(event)
    print("Successful configuration")


def test_configure_starttls(acfactory) -> None:
    account = acfactory.new_preconfigured_account()

    # Use STARTTLS
    account.set_config("mail_security", "2")
    account.set_config("send_security", "2")
    account.configure()
    assert account.is_configured()


def test_account(acfactory) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()
    alice_chat_bob.send_text("Hello!")

    while True:
        event = bob.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            chat_id = event.chat_id
            msg_id = event.msg_id
            break

    message = bob.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.chat_id == chat_id
    assert snapshot.text == "Hello!"
    bob.mark_seen_messages([message])

    assert alice != bob
    assert repr(alice)
    assert alice.get_info().level
    assert alice.get_size()
    assert alice.is_configured()
    assert not alice.get_avatar()
    assert alice.get_contact_by_addr(bob_addr) == alice_contact_bob
    assert alice.get_contacts()
    assert alice.get_contacts(snapshot=True)
    assert alice.self_contact
    assert alice.get_chatlist()
    assert alice.get_chatlist(snapshot=True)
    assert alice.get_qr_code()
    assert alice.get_fresh_messages()
    assert alice.get_next_messages()

    # Test sending empty message.
    assert len(bob.wait_next_messages()) == 0
    alice_chat_bob.send_text("")
    messages = bob.wait_next_messages()
    assert len(messages) == 1
    message = messages[0]
    snapshot = message.get_snapshot()
    assert snapshot.text == ""
    bob.mark_seen_messages([message])

    group = alice.create_group("test group")
    group.add_contact(alice_contact_bob)
    group_msg = group.send_message(text="hello")
    assert group_msg == alice.get_message_by_id(group_msg.id)
    assert group == alice.get_chat_by_id(group.id)
    alice.delete_messages([group_msg])

    alice.set_config("selfstatus", "test")
    assert alice.get_config("selfstatus") == "test"
    alice.update_config(selfstatus="test2")
    assert alice.get_config("selfstatus") == "test2"

    assert not alice.get_blocked_contacts()
    alice_contact_bob.block()
    blocked_contacts = alice.get_blocked_contacts()
    assert blocked_contacts
    assert blocked_contacts[0].contact == alice_contact_bob

    bob.remove()
    alice.stop_io()


def test_chat(acfactory) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()
    alice_chat_bob.send_text("Hello!")

    event = bob.wait_for_incoming_msg_event()
    chat_id = event.chat_id
    msg_id = event.msg_id
    message = bob.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.chat_id == chat_id
    assert snapshot.text == "Hello!"
    bob_chat_alice = bob.get_chat_by_id(chat_id)

    assert alice_chat_bob != bob_chat_alice
    assert repr(alice_chat_bob)
    alice_chat_bob.delete()
    assert not bob_chat_alice.can_send()
    bob_chat_alice.accept()
    assert bob_chat_alice.can_send()
    bob_chat_alice.block()
    bob_chat_alice = snapshot.sender.create_chat()
    bob_chat_alice.mute()
    bob_chat_alice.unmute()
    bob_chat_alice.pin()
    bob_chat_alice.unpin()
    bob_chat_alice.archive()
    bob_chat_alice.unarchive()
    with pytest.raises(JsonRpcError):  # can't set name for 1:1 chats
        bob_chat_alice.set_name("test")
    bob_chat_alice.set_ephemeral_timer(300)
    bob_chat_alice.get_encryption_info()

    group = alice.create_group("test group")
    group.add_contact(alice_contact_bob)
    group.get_qr_code()

    snapshot = group.get_basic_snapshot()
    assert snapshot.name == "test group"
    group.set_name("new name")
    snapshot = group.get_full_snapshot()
    assert snapshot.name == "new name"

    msg = group.send_message(text="hi")
    assert (msg.get_snapshot()).text == "hi"
    group.forward_messages([msg])

    group.set_draft(text="test draft")
    draft = group.get_draft()
    assert draft.text == "test draft"
    group.remove_draft()
    assert not group.get_draft()

    assert group.get_messages()
    group.get_fresh_message_count()
    group.mark_noticed()
    assert group.get_contacts()
    group.remove_contact(alice_chat_bob)
    group.get_locations()


def test_contact(acfactory) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")

    assert alice_contact_bob == alice.get_contact_by_id(alice_contact_bob.id)
    assert repr(alice_contact_bob)
    alice_contact_bob.block()
    alice_contact_bob.unblock()
    alice_contact_bob.set_name("new name")
    alice_contact_bob.get_encryption_info()
    snapshot = alice_contact_bob.get_snapshot()
    assert snapshot.address == bob_addr
    alice_contact_bob.create_chat()


def test_message(acfactory) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()
    alice_chat_bob.send_text("Hello!")

    event = bob.wait_for_incoming_msg_event()
    chat_id = event.chat_id
    msg_id = event.msg_id

    message = bob.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.chat_id == chat_id
    assert snapshot.text == "Hello!"
    assert not snapshot.is_bot
    assert repr(message)

    with pytest.raises(JsonRpcError):  # chat is not accepted
        snapshot.chat.send_text("hi")
    snapshot.chat.accept()
    snapshot.chat.send_text("hi")

    message.mark_seen()
    message.send_reaction("😎")
    reactions = message.get_reactions()
    assert reactions
    snapshot = message.get_snapshot()
    assert reactions == snapshot.reactions


def test_is_bot(acfactory) -> None:
    """Test that we can recognize messages submitted by bots."""
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()

    # Alice becomes a bot.
    alice.set_config("bot", "1")
    alice_chat_bob.send_text("Hello!")

    while True:
        event = bob.wait_for_event()
        if event.kind == EventType.INCOMING_MSG:
            msg_id = event.msg_id
            message = bob.get_message_by_id(msg_id)
            snapshot = message.get_snapshot()
            assert snapshot.chat_id == event.chat_id
            assert snapshot.text == "Hello!"
            assert snapshot.is_bot
            break


def test_bot(acfactory) -> None:
    mock = MagicMock()
    user = (acfactory.get_online_accounts(1))[0]
    bot = acfactory.new_configured_bot()
    bot2 = acfactory.new_configured_bot()

    assert bot.is_configured()
    assert bot.account.get_config("bot") == "1"

    hook = lambda e: mock.hook(e.msg_id) and None, events.RawEvent(EventType.INCOMING_MSG)
    bot.add_hook(*hook)
    event = acfactory.process_message(from_account=user, to_client=bot, text="Hello!")
    snapshot = bot.account.get_message_by_id(event.msg_id).get_snapshot()
    assert not snapshot.is_bot
    mock.hook.assert_called_once_with(event.msg_id)
    bot.remove_hook(*hook)

    def track(e):
        mock.hook(e.message_snapshot.id)

    mock.hook.reset_mock()
    hook = track, events.NewMessage(r"hello")
    bot.add_hook(*hook)
    bot.add_hook(track, events.NewMessage(command="/help"))
    event = acfactory.process_message(from_account=user, to_client=bot, text="hello")
    mock.hook.assert_called_with(event.msg_id)
    event = acfactory.process_message(from_account=user, to_client=bot, text="hello!")
    mock.hook.assert_called_with(event.msg_id)
    acfactory.process_message(from_account=bot2.account, to_client=bot, text="hello")
    assert len(mock.hook.mock_calls) == 2  # bot messages are ignored between bots
    acfactory.process_message(from_account=user, to_client=bot, text="hey!")
    assert len(mock.hook.mock_calls) == 2
    bot.remove_hook(*hook)

    mock.hook.reset_mock()
    acfactory.process_message(from_account=user, to_client=bot, text="hello")
    event = acfactory.process_message(from_account=user, to_client=bot, text="/help")
    mock.hook.assert_called_once_with(event.msg_id)


def test_wait_next_messages(acfactory) -> None:
    alice = acfactory.new_configured_account()

    # Create a bot account so it does not receive device messages in the beginning.
    bot = acfactory.new_preconfigured_account()
    bot.set_config("bot", "1")
    bot.configure()

    # There are no old messages and the call returns immediately.
    assert not bot.wait_next_messages()

    with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
        # Bot starts waiting for messages.
        next_messages_task = executor.submit(bot.wait_next_messages)

        bot_addr = bot.get_config("addr")
        alice_contact_bot = alice.create_contact(bot_addr, "Bot")
        alice_chat_bot = alice_contact_bot.create_chat()
        alice_chat_bot.send_text("Hello!")

        next_messages = next_messages_task.result()
        assert len(next_messages) == 1
        snapshot = next_messages[0].get_snapshot()
        assert snapshot.text == "Hello!"


def test_import_export_backup(acfactory, tmp_path) -> None:
    alice = acfactory.new_configured_account()
    alice.export_backup(tmp_path)

    files = list(tmp_path.glob("*.tar"))
    alice2 = acfactory.get_unconfigured_account()
    alice2.import_backup(files[0])

    assert alice2.manager.get_system_info()


def test_import_export_keys(acfactory, tmp_path) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")
    alice_contact_bob = alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = alice_contact_bob.create_chat()
    alice_chat_bob.send_text("Hello Bob!")

    snapshot = bob.get_message_by_id(bob.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Hello Bob!"

    # Alice resetups account, but keeps the key.
    alice_keys_path = tmp_path / "alice_keys"
    alice_keys_path.mkdir()
    alice.export_self_keys(alice_keys_path)
    alice = acfactory.resetup_account(alice)
    alice.import_self_keys(alice_keys_path)

    snapshot.chat.accept()
    snapshot.chat.send_text("Hello Alice!")
    snapshot = alice.get_message_by_id(alice.wait_for_incoming_msg_event().msg_id).get_snapshot()
    assert snapshot.text == "Hello Alice!"
    assert snapshot.show_padlock


def test_openrpc_command_line() -> None:
    """Test that "deltachat-rpc-server --openrpc" command returns an OpenRPC specification."""
    out = subprocess.run(["deltachat-rpc-server", "--openrpc"], capture_output=True, check=True).stdout
    openrpc = json.loads(out)
    assert "openrpc" in openrpc
    assert "methods" in openrpc


def test_provider_info(rpc) -> None:
    account_id = rpc.add_account()

    provider_info = rpc.get_provider_info(account_id, "example.org")
    assert provider_info["id"] == "example.com"

    provider_info = rpc.get_provider_info(account_id, "uep7oiw4ahtaizuloith.org")
    assert provider_info is None

    # Test MX record resolution.
    provider_info = rpc.get_provider_info(account_id, "github.com")
    assert provider_info["id"] == "gmail"

    # Disable MX record resolution.
    rpc.set_config(account_id, "socks5_enabled", "1")
    provider_info = rpc.get_provider_info(account_id, "github.com")
    assert provider_info is None


def test_mdn_doesnt_break_autocrypt(acfactory) -> None:
    alice, bob = acfactory.get_online_accounts(2)

    bob_addr = bob.get_config("addr")

    alice_contact_bob = alice.create_contact(bob_addr, "Bob")

    # Bob creates chat manually so chat with Alice is accepted.
    alice_chat_bob = alice_contact_bob.create_chat()

    # Alice sends a message to Bob.
    alice_chat_bob.send_text("Hello Bob!")
    event = bob.wait_for_incoming_msg_event()
    msg_id = event.msg_id
    message = bob.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()

    # Bob sends a message to Alice.
    bob_chat_alice = snapshot.chat
    bob_chat_alice.accept()
    bob_chat_alice.send_text("Hello Alice!")
    event = alice.wait_for_incoming_msg_event()
    msg_id = event.msg_id
    message = alice.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.show_padlock

    # Alice reads Bob's message.
    message.mark_seen()
    while True:
        event = bob.wait_for_event()
        if event.kind == EventType.MSG_READ:
            break

    # Bob sends a message to Alice, it should also be encrypted.
    bob_chat_alice.send_text("Hi Alice!")
    event = alice.wait_for_incoming_msg_event()
    msg_id = event.msg_id
    message = alice.get_message_by_id(msg_id)
    snapshot = message.get_snapshot()
    assert snapshot.show_padlock

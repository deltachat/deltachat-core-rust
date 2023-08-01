import asyncio
import json
import subprocess
from unittest.mock import MagicMock

import pytest
from deltachat_rpc_client import EventType, events
from deltachat_rpc_client.rpc import JsonRpcError


@pytest.mark.asyncio()
async def test_system_info(rpc) -> None:
    system_info = await rpc.get_system_info()
    assert "arch" in system_info
    assert "deltachat_core_version" in system_info


@pytest.mark.asyncio()
async def test_sleep(rpc) -> None:
    """Test that long-running task does not block short-running task from completion."""
    sleep_5_task = asyncio.create_task(rpc.sleep(5.0))
    sleep_3_task = asyncio.create_task(rpc.sleep(3.0))
    done, pending = await asyncio.wait([sleep_5_task, sleep_3_task], return_when=asyncio.FIRST_COMPLETED)
    assert sleep_3_task in done
    assert sleep_5_task in pending
    sleep_5_task.cancel()


@pytest.mark.asyncio()
async def test_email_address_validity(rpc) -> None:
    valid_addresses = [
        "email@example.com",
        "36aa165ae3406424e0c61af17700f397cad3fe8ab83d682d0bddf3338a5dd52e@yggmail@yggmail",
    ]
    invalid_addresses = ["email@", "example.com", "emai221"]

    for addr in valid_addresses:
        assert await rpc.check_email_validity(addr)
    for addr in invalid_addresses:
        assert not await rpc.check_email_validity(addr)


@pytest.mark.asyncio()
async def test_acfactory(acfactory) -> None:
    account = await acfactory.new_configured_account()
    while True:
        event = await account.wait_for_event()
        if event.type == EventType.CONFIGURE_PROGRESS:
            assert event.progress != 0  # Progress 0 indicates error.
            if event.progress == 1000:  # Success
                break
        else:
            print(event)
    print("Successful configuration")


@pytest.mark.asyncio()
async def test_configure_starttls(acfactory) -> None:
    account = await acfactory.new_preconfigured_account()

    # Use STARTTLS
    await account.set_config("mail_security", "2")
    await account.set_config("send_security", "2")
    await account.configure()
    assert await account.is_configured()


@pytest.mark.asyncio()
async def test_account(acfactory) -> None:
    alice, bob = await acfactory.get_online_accounts(2)

    bob_addr = await bob.get_config("addr")
    alice_contact_bob = await alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = await alice_contact_bob.create_chat()
    await alice_chat_bob.send_text("Hello!")

    while True:
        event = await bob.wait_for_event()
        if event.type == EventType.INCOMING_MSG:
            chat_id = event.chat_id
            msg_id = event.msg_id
            break

    message = bob.get_message_by_id(msg_id)
    snapshot = await message.get_snapshot()
    assert snapshot.chat_id == chat_id
    assert snapshot.text == "Hello!"
    await bob.mark_seen_messages([message])

    assert alice != bob
    assert repr(alice)
    assert (await alice.get_info()).level
    assert await alice.get_size()
    assert await alice.is_configured()
    assert not await alice.get_avatar()
    assert await alice.get_contact_by_addr(bob_addr) == alice_contact_bob
    assert await alice.get_contacts()
    assert await alice.get_contacts(snapshot=True)
    assert alice.self_contact
    assert await alice.get_chatlist()
    assert await alice.get_chatlist(snapshot=True)
    assert await alice.get_qr_code()
    assert await alice.get_fresh_messages()
    assert await alice.get_next_messages()

    # Test sending empty message.
    assert len(await bob.wait_next_messages()) == 0
    await alice_chat_bob.send_text("")
    messages = await bob.wait_next_messages()
    assert len(messages) == 1
    message = messages[0]
    snapshot = await message.get_snapshot()
    assert snapshot.text == ""
    await bob.mark_seen_messages([message])

    group = await alice.create_group("test group")
    await group.add_contact(alice_contact_bob)
    group_msg = await group.send_message(text="hello")
    assert group_msg == alice.get_message_by_id(group_msg.id)
    assert group == alice.get_chat_by_id(group.id)
    await alice.delete_messages([group_msg])

    await alice.set_config("selfstatus", "test")
    assert await alice.get_config("selfstatus") == "test"
    await alice.update_config(selfstatus="test2")
    assert await alice.get_config("selfstatus") == "test2"

    assert not await alice.get_blocked_contacts()
    await alice_contact_bob.block()
    blocked_contacts = await alice.get_blocked_contacts()
    assert blocked_contacts
    assert blocked_contacts[0].contact == alice_contact_bob

    await bob.remove()
    await alice.stop_io()


@pytest.mark.asyncio()
async def test_chat(acfactory) -> None:
    alice, bob = await acfactory.get_online_accounts(2)

    bob_addr = await bob.get_config("addr")
    alice_contact_bob = await alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = await alice_contact_bob.create_chat()
    await alice_chat_bob.send_text("Hello!")

    while True:
        event = await bob.wait_for_event()
        if event.type == EventType.INCOMING_MSG:
            chat_id = event.chat_id
            msg_id = event.msg_id
            break
    message = bob.get_message_by_id(msg_id)
    snapshot = await message.get_snapshot()
    assert snapshot.chat_id == chat_id
    assert snapshot.text == "Hello!"
    bob_chat_alice = bob.get_chat_by_id(chat_id)

    assert alice_chat_bob != bob_chat_alice
    assert repr(alice_chat_bob)
    await alice_chat_bob.delete()
    assert not await bob_chat_alice.can_send()
    await bob_chat_alice.accept()
    assert await bob_chat_alice.can_send()
    await bob_chat_alice.block()
    bob_chat_alice = await snapshot.sender.create_chat()
    await bob_chat_alice.mute()
    await bob_chat_alice.unmute()
    await bob_chat_alice.pin()
    await bob_chat_alice.unpin()
    await bob_chat_alice.archive()
    await bob_chat_alice.unarchive()
    with pytest.raises(JsonRpcError):  # can't set name for 1:1 chats
        await bob_chat_alice.set_name("test")
    await bob_chat_alice.set_ephemeral_timer(300)
    await bob_chat_alice.get_encryption_info()

    group = await alice.create_group("test group")
    await group.add_contact(alice_contact_bob)
    await group.get_qr_code()

    snapshot = await group.get_basic_snapshot()
    assert snapshot.name == "test group"
    await group.set_name("new name")
    snapshot = await group.get_full_snapshot()
    assert snapshot.name == "new name"

    msg = await group.send_message(text="hi")
    assert (await msg.get_snapshot()).text == "hi"
    await group.forward_messages([msg])

    await group.set_draft(text="test draft")
    draft = await group.get_draft()
    assert draft.text == "test draft"
    await group.remove_draft()
    assert not await group.get_draft()

    assert await group.get_messages()
    await group.get_fresh_message_count()
    await group.mark_noticed()
    assert await group.get_contacts()
    await group.remove_contact(alice_chat_bob)
    await group.get_locations()


@pytest.mark.asyncio()
async def test_contact(acfactory) -> None:
    alice, bob = await acfactory.get_online_accounts(2)

    bob_addr = await bob.get_config("addr")
    alice_contact_bob = await alice.create_contact(bob_addr, "Bob")

    assert alice_contact_bob == alice.get_contact_by_id(alice_contact_bob.id)
    assert repr(alice_contact_bob)
    await alice_contact_bob.block()
    await alice_contact_bob.unblock()
    await alice_contact_bob.set_name("new name")
    await alice_contact_bob.get_encryption_info()
    snapshot = await alice_contact_bob.get_snapshot()
    assert snapshot.address == bob_addr
    await alice_contact_bob.create_chat()


@pytest.mark.asyncio()
async def test_message(acfactory) -> None:
    alice, bob = await acfactory.get_online_accounts(2)

    bob_addr = await bob.get_config("addr")
    alice_contact_bob = await alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = await alice_contact_bob.create_chat()
    await alice_chat_bob.send_text("Hello!")

    while True:
        event = await bob.wait_for_event()
        if event.type == EventType.INCOMING_MSG:
            chat_id = event.chat_id
            msg_id = event.msg_id
            break

    message = bob.get_message_by_id(msg_id)
    snapshot = await message.get_snapshot()
    assert snapshot.chat_id == chat_id
    assert snapshot.text == "Hello!"
    assert not snapshot.is_bot
    assert repr(message)

    with pytest.raises(JsonRpcError):  # chat is not accepted
        await snapshot.chat.send_text("hi")
    await snapshot.chat.accept()
    await snapshot.chat.send_text("hi")

    await message.mark_seen()
    await message.send_reaction("😎")
    reactions = await message.get_reactions()
    assert reactions
    snapshot = await message.get_snapshot()
    assert reactions == snapshot.reactions


@pytest.mark.asyncio()
async def test_is_bot(acfactory) -> None:
    """Test that we can recognize messages submitted by bots."""
    alice, bob = await acfactory.get_online_accounts(2)

    bob_addr = await bob.get_config("addr")
    alice_contact_bob = await alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = await alice_contact_bob.create_chat()

    # Alice becomes a bot.
    await alice.set_config("bot", "1")
    await alice_chat_bob.send_text("Hello!")

    while True:
        event = await bob.wait_for_event()
        if event.type == EventType.INCOMING_MSG:
            msg_id = event.msg_id
            message = bob.get_message_by_id(msg_id)
            snapshot = await message.get_snapshot()
            assert snapshot.chat_id == event.chat_id
            assert snapshot.text == "Hello!"
            assert snapshot.is_bot
            break


@pytest.mark.asyncio()
async def test_bot(acfactory) -> None:
    mock = MagicMock()
    user = (await acfactory.get_online_accounts(1))[0]
    bot = await acfactory.new_configured_bot()
    bot2 = await acfactory.new_configured_bot()

    assert await bot.is_configured()
    assert await bot.account.get_config("bot") == "1"

    hook = lambda e: mock.hook(e.msg_id) and None, events.RawEvent(EventType.INCOMING_MSG)
    bot.add_hook(*hook)
    event = await acfactory.process_message(from_account=user, to_client=bot, text="Hello!")
    snapshot = await bot.account.get_message_by_id(event.msg_id).get_snapshot()
    assert not snapshot.is_bot
    mock.hook.assert_called_once_with(event.msg_id)
    bot.remove_hook(*hook)

    def track(e):
        mock.hook(e.message_snapshot.id)

    mock.hook.reset_mock()
    hook = track, events.NewMessage(r"hello")
    bot.add_hook(*hook)
    bot.add_hook(track, events.NewMessage(command="/help"))
    event = await acfactory.process_message(from_account=user, to_client=bot, text="hello")
    mock.hook.assert_called_with(event.msg_id)
    event = await acfactory.process_message(from_account=user, to_client=bot, text="hello!")
    mock.hook.assert_called_with(event.msg_id)
    await acfactory.process_message(from_account=bot2.account, to_client=bot, text="hello")
    assert len(mock.hook.mock_calls) == 2  # bot messages are ignored between bots
    await acfactory.process_message(from_account=user, to_client=bot, text="hey!")
    assert len(mock.hook.mock_calls) == 2
    bot.remove_hook(*hook)

    mock.hook.reset_mock()
    await acfactory.process_message(from_account=user, to_client=bot, text="hello")
    event = await acfactory.process_message(from_account=user, to_client=bot, text="/help")
    mock.hook.assert_called_once_with(event.msg_id)


@pytest.mark.asyncio()
async def test_wait_next_messages(acfactory) -> None:
    alice = await acfactory.new_configured_account()

    # Create a bot account so it does not receive device messages in the beginning.
    bot = await acfactory.new_preconfigured_account()
    await bot.set_config("bot", "1")
    await bot.configure()

    # There are no old messages and the call returns immediately.
    assert not await bot.wait_next_messages()

    # Bot starts waiting for messages.
    next_messages_task = asyncio.create_task(bot.wait_next_messages())

    bot_addr = await bot.get_config("addr")
    alice_contact_bot = await alice.create_contact(bot_addr, "Bob")
    alice_chat_bot = await alice_contact_bot.create_chat()
    await alice_chat_bot.send_text("Hello!")

    next_messages = await next_messages_task
    assert len(next_messages) == 1
    snapshot = await next_messages[0].get_snapshot()
    assert snapshot.text == "Hello!"


@pytest.mark.asyncio()
async def test_import_export(acfactory, tmp_path) -> None:
    alice = await acfactory.new_configured_account()
    await alice.export_backup(tmp_path)

    files = list(tmp_path.glob("*.tar"))
    alice2 = await acfactory.get_unconfigured_account()
    await alice2.import_backup(files[0])

    assert await alice2.manager.get_system_info()


def test_openrpc_command_line() -> None:
    """Test that "deltachat-rpc-server --openrpc" command returns an OpenRPC specification."""
    out = subprocess.run(["deltachat-rpc-server", "--openrpc"], capture_output=True).stdout
    openrpc = json.loads(out)
    assert "openrpc" in openrpc
    assert "methods" in openrpc

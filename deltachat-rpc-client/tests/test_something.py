import pytest

from deltachat_rpc_client import AttrDict, EventType, events


@pytest.mark.asyncio
async def test_system_info(rpc) -> None:
    system_info = await rpc.get_system_info()
    assert "arch" in system_info
    assert "deltachat_core_version" in system_info


@pytest.mark.asyncio
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


@pytest.mark.asyncio
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


@pytest.mark.asyncio
async def test_object_account(acfactory) -> None:
    alice, bob = await acfactory.get_online_accounts(2)

    alice_contact_bob = await alice.create_contact(await bob.get_config("addr"), "Bob")
    alice_chat_bob = await alice_contact_bob.create_chat()
    await alice_chat_bob.send_text("Hello!")

    while True:
        event = await bob.wait_for_event()
        if event.type == EventType.INCOMING_MSG:
            chat_id = event.chat_id
            msg_id = event.msg_id
            break

    message = await bob.get_message_by_id(msg_id).get_snapshot()
    assert message.chat_id == chat_id
    assert message.text == "Hello!"


@pytest.mark.asyncio
async def test_bot(acfactory) -> None:
    async def callback(e):
        res.append(e)

    res = []
    bot = await acfactory.new_configured_bot()
    assert await bot.is_configured()
    assert await bot.account.get_config("bot") == "1"

    bot.add_hook(callback, events.RawEvent(EventType.INFO))
    info_event = AttrDict(account=bot.account, type=EventType.INFO, msg="info")
    warn_event = AttrDict(account=bot.account, type=EventType.WARNING, msg="warning")
    await bot._on_event(info_event)
    await bot._on_event(warn_event)
    assert info_event in res
    assert warn_event not in res
    assert len(res) == 1

    res = []
    bot.add_hook(callback, events.NewMessage(r"hello"))
    snapshot1 = AttrDict(text="hello")
    snapshot2 = AttrDict(text="hello, world")
    snapshot3 = AttrDict(text="hey!")
    for snapshot in [snapshot1, snapshot2, snapshot3]:
        await bot._on_event(snapshot, events.NewMessage)
    assert len(res) == 2
    assert snapshot1 in res
    assert snapshot2 in res
    assert snapshot3 not in res

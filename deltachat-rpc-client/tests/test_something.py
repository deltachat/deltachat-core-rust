import pytest


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
        if event["type"] == "ConfigureProgress":
            # Progress 0 indicates error.
            assert event["progress"] != 0

            if event["progress"] == 1000:
                # Success.
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
        if event["type"] == "IncomingMsg":
            chat_id = event["chatId"]
            msg_id = event["msgId"]
            break

    rpc = acfactory.deltachat.rpc
    message = await rpc.get_message(bob.account_id, msg_id)
    assert message["chatId"] == chat_id
    assert message["text"] == "Hello!"

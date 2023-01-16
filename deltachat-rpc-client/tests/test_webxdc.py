import pytest

from deltachat_rpc_client import EventType


@pytest.mark.asyncio
async def test_webxdc(acfactory) -> None:
    alice, bob = await acfactory.get_online_accounts(2)

    bob_addr = await bob.get_config("addr")
    alice_contact_bob = await alice.create_contact(bob_addr, "Bob")
    alice_chat_bob = await alice_contact_bob.create_chat()
    await alice_chat_bob.send_message(
        text="Let's play chess!", file="../test-data/webxdc/chess.xdc"
    )

    while True:
        event = await bob.wait_for_event()
        if event.type == EventType.INCOMING_MSG:
            bob_chat_alice = bob.get_chat_by_id(event.chat_id)
            message = bob.get_message_by_id(event.msg_id)
            break

    webxdc_info = await message.get_webxdc_info()
    assert webxdc_info == {
        "document": None,
        "icon": "icon.png",
        "internetAccess": False,
        "name": "Chess Board",
        "sourceCodeUrl": None,
        "summary": None,
    }

    status_updates = await message.get_webxdc_status_updates()
    assert status_updates == []

    await bob_chat_alice.accept()
    await message.send_webxdc_status_update({"payload": 42}, "")
    await message.send_webxdc_status_update({"payload": "Second update"}, "description")

    status_updates = await message.get_webxdc_status_updates()
    assert status_updates == [
        {"payload": 42, "serial": 1, "max_serial": 2},
        {"payload": "Second update", "serial": 2, "max_serial": 2},
    ]

    status_updates = await message.get_webxdc_status_updates(1)
    assert status_updates == [
        {"payload": "Second update", "serial": 2, "max_serial": 2},
    ]

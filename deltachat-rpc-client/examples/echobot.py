#!/usr/bin/env python3
import asyncio
import logging
import sys

import deltachat_rpc_client as dc


async def main():
    rpc = await dc.start_rpc_server()
    deltachat = dc.Deltachat(rpc)
    system_info = await deltachat.get_system_info()
    logging.info("Running deltachat core %s", system_info["deltachat_core_version"])

    accounts = await deltachat.get_all_accounts()
    account = accounts[0] if accounts else await deltachat.add_account()

    await account.set_config("bot", "1")
    if not await account.is_configured():
        logging.info("Account is not configured, configuring")
        await account.set_config("addr", sys.argv[1])
        await account.set_config("mail_pw", sys.argv[2])
        await account.configure()
        logging.info("Configured")
    else:
        logging.info("Account is already configured")
        await deltachat.start_io()

    async def process_messages():
        fresh_messages = await account.get_fresh_messages()
        fresh_message_snapshot_tasks = [
            message.get_snapshot() for message in fresh_messages
        ]
        fresh_message_snapshots = await asyncio.gather(*fresh_message_snapshot_tasks)
        for snapshot in reversed(fresh_message_snapshots):
            if not snapshot.is_info:
                await snapshot.chat.send_text(snapshot.text)
            await snapshot.message.mark_seen()

    # Process old messages.
    await process_messages()

    while True:
        event = await account.wait_for_event()
        if event["type"] == "Info":
            logging.info("%s", event["msg"])
        elif event["type"] == "Warning":
            logging.warning("%s", event["msg"])
        elif event["type"] == "Error":
            logging.error("%s", event["msg"])
        elif event["type"] == "IncomingMsg":
            logging.info("Got an incoming message")
            await process_messages()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())

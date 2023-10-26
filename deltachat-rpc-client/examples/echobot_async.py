#!/usr/bin/env python3
"""
Example asynchronous echo bot
"""
import asyncio
import logging
import sys

from deltachat_rpc_client import EventType, Rpc, SpecialContactId


async def main():
    async with Rpc() as rpc:
        system_info = await rpc.get_system_info()
        logging.info("Running deltachat core %s", system_info["deltachat_core_version"])

        account_ids = await rpc.get_all_account_ids()
        accid = account_ids[0] if account_ids else await rpc.add_account()

        await rpc.set_config(accid, "bot", "1")
        if not await rpc.is_configured(accid):
            logging.info("Account is not configured, configuring")
            await rpc.set_config(accid, "addr", sys.argv[1])
            await rpc.set_config(accid, "mail_pw", sys.argv[2])
            await rpc.configure(accid)
            logging.info("Configured")
        else:
            logging.info("Account is already configured")
            await rpc.start_io(accid)

        async def process_messages():
            for msgid in await rpc.get_next_msgs(accid):
                msg = await rpc.get_message(accid, msgid)
                if msg["from_id"] != SpecialContactId.SELF and not msg["is_bot"] and not msg["is_info"]:
                    await rpc.misc_send_text_message(accid, msg["chat_id"], msg["text"])
                await rpc.markseen_msgs(accid, [msgid])

        # Process old messages.
        await process_messages()

        while True:
            event = await rpc.wait_for_event(accid)
            if event["kind"] == EventType.INFO:
                logging.info("%s", event["msg"])
            elif event["kind"] == EventType.WARNING:
                logging.warning("%s", event["msg"])
            elif event["kind"] == EventType.ERROR:
                logging.error("%s", event["msg"])
            elif event["kind"] == EventType.INCOMING_MSG:
                logging.info("Got an incoming message (id=%s)", event["msg_id"])
                await process_messages()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())

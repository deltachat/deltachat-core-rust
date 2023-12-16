#!/usr/bin/env python3
"""
Example echo bot without using hooks
"""
import logging
import sys

from deltachat_rpc_client import DeltaChat, EventType, Rpc, SpecialContactId


def main():
    with Rpc() as rpc:
        deltachat = DeltaChat(rpc)
        system_info = deltachat.get_system_info()
        logging.info("Running deltachat core %s", system_info["deltachat_core_version"])

        accounts = deltachat.get_all_accounts()
        account = accounts[0] if accounts else deltachat.add_account()

        account.set_config("bot", "1")
        if not account.is_configured():
            logging.info("Account is not configured, configuring")
            account.set_config("addr", sys.argv[1])
            account.set_config("mail_pw", sys.argv[2])
            account.configure()
            logging.info("Configured")
        else:
            logging.info("Account is already configured")
            deltachat.start_io()

        def process_messages():
            for message in account.get_next_messages():
                snapshot = message.get_snapshot()
                if snapshot.from_id != SpecialContactId.SELF and not snapshot.is_bot and not snapshot.is_info:
                    snapshot.chat.send_text(snapshot.text)
                snapshot.message.mark_seen()

        # Process old messages.
        process_messages()

        while True:
            event = account.wait_for_event()
            if event["type"] == EventType.INFO:
                logging.info("%s", event["msg"])
            elif event["type"] == EventType.WARNING:
                logging.warning("%s", event["msg"])
            elif event["type"] == EventType.ERROR:
                logging.error("%s", event["msg"])
            elif event["type"] == EventType.INCOMING_MSG:
                logging.info("Got an incoming message")
                process_messages()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    main()

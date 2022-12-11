#!/usr/bin/env python3
"""Advanced echo bot example.

it will echo back any message that has non-empty text and also supports the /help command.
"""
import asyncio
import logging
import sys

from deltachat_rpc_client import Bot, DeltaChat, EventType, Rpc, events

hooks = events.HookCollection()


@hooks.on(events.RawEvent)
async def log_event(event):
    if event.type == EventType.INFO:
        logging.info(event.msg)
    elif event.type == EventType.WARNING:
        logging.warning(event.msg)


@hooks.on(events.RawEvent(EventType.ERROR))
async def log_error(event):
    logging.error(event.msg)


@hooks.on(events.NewMessage(func=lambda e: not e.command))
async def echo(event):
    if event.text or event.file:
        await event.chat.send_message(text=event.text, file=event.file)


@hooks.on(events.NewMessage(command="/help"))
async def help_command(event):
    await event.chat.send_text("Send me any message and I will echo it back")


async def main():
    async with Rpc() as rpc:
        deltachat = DeltaChat(rpc)
        system_info = await deltachat.get_system_info()
        logging.info("Running deltachat core %s", system_info.deltachat_core_version)

        accounts = await deltachat.get_all_accounts()
        account = accounts[0] if accounts else await deltachat.add_account()

        bot = Bot(account, hooks)
        if not await bot.is_configured():
            asyncio.create_task(bot.configure(email=sys.argv[1], password=sys.argv[2]))
        await bot.run_forever()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())

#!/usr/bin/env python3
"""Minimal echo bot example.

it will echo back any text send to it, it also will print to console all Delta Chat core events.
Pass --help to the CLI to see available options.
"""
import asyncio

from deltachat_rpc_client import events, run_bot_cli

hooks = events.HookCollection()


@hooks.on(events.RawEvent)
async def log_event(event):
    print(event)


@hooks.on(events.NewMessage)
async def echo(event):
    await event.chat.send_text(event.text)


if __name__ == "__main__":
    asyncio.run(run_bot_cli(hooks))

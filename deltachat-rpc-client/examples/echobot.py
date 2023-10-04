#!/usr/bin/env python3
"""Minimal echo bot example.

it will echo back any text send to it, it also will print to console all Delta Chat core events.
Pass --help to the CLI to see available options.
"""
from deltachat_rpc_client import events, run_bot_cli

hooks = events.HookCollection()


@hooks.on(events.RawEvent)
def log_event(event):
    print(event)


@hooks.on(events.NewMessage)
def echo(event):
    snapshot = event.message_snapshot
    snapshot.chat.send_text(snapshot.text)


if __name__ == "__main__":
    run_bot_cli(hooks)

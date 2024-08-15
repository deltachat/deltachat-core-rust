from __future__ import annotations

import os
import random
from typing import AsyncGenerator, Optional

import pytest

from . import Account, AttrDict, Bot, Chat, Client, DeltaChat, EventType, Message
from ._utils import futuremethod
from .rpc import Rpc


def get_temp_credentials() -> dict:
    domain = os.getenv("CHATMAIL_DOMAIN")
    username = "ci-" + "".join(random.choice("2345789acdefghjkmnpqrstuvwxyz") for i in range(6))
    password = f"{username}${username}"
    addr = f"{username}@{domain}"
    return {"email": addr, "password": password}


class ACFactory:
    def __init__(self, deltachat: DeltaChat) -> None:
        self.deltachat = deltachat

    def get_unconfigured_account(self) -> Account:
        account = self.deltachat.add_account()
        account.set_config("verified_one_on_one_chats", "1")
        return account

    def get_unconfigured_bot(self) -> Bot:
        return Bot(self.get_unconfigured_account())

    def new_preconfigured_account(self) -> Account:
        """Make a new account with configuration options set, but configuration not started."""
        credentials = get_temp_credentials()
        account = self.get_unconfigured_account()
        account.set_config("addr", credentials["email"])
        account.set_config("mail_pw", credentials["password"])
        assert not account.is_configured()
        return account

    @futuremethod
    def new_configured_account(self):
        account = self.new_preconfigured_account()
        yield account.configure.future()
        assert account.is_configured()
        return account

    def new_configured_bot(self) -> Bot:
        credentials = get_temp_credentials()
        bot = self.get_unconfigured_bot()
        bot.configure(credentials["email"], credentials["password"])
        return bot

    @futuremethod
    def get_online_account(self):
        account = yield self.new_configured_account.future()
        account.bring_online()
        return account

    def get_online_accounts(self, num: int) -> list[Account]:
        futures = [self.get_online_account.future() for _ in range(num)]
        return [f() for f in futures]

    def resetup_account(self, ac: Account) -> Account:
        """Resetup account from scratch, losing the encryption key."""
        ac.stop_io()
        ac_clone = self.get_unconfigured_account()
        for i in ["addr", "mail_pw"]:
            ac_clone.set_config(i, ac.get_config(i))
        ac.remove()
        ac_clone.configure()
        return ac_clone

    def get_accepted_chat(self, ac1: Account, ac2: Account) -> Chat:
        ac2.create_chat(ac1)
        return ac1.create_chat(ac2)

    def send_message(
        self,
        to_account: Account,
        from_account: Optional[Account] = None,
        text: Optional[str] = None,
        file: Optional[str] = None,
        group: Optional[str] = None,
    ) -> Message:
        if not from_account:
            from_account = (self.get_online_accounts(1))[0]
        to_contact = from_account.create_contact(to_account.get_config("addr"))
        if group:
            to_chat = from_account.create_group(group)
            to_chat.add_contact(to_contact)
        else:
            to_chat = to_contact.create_chat()
        return to_chat.send_message(text=text, file=file)

    def process_message(
        self,
        to_client: Client,
        from_account: Optional[Account] = None,
        text: Optional[str] = None,
        file: Optional[str] = None,
        group: Optional[str] = None,
    ) -> AttrDict:
        self.send_message(
            to_account=to_client.account,
            from_account=from_account,
            text=text,
            file=file,
            group=group,
        )

        return to_client.run_until(lambda e: e.kind == EventType.INCOMING_MSG)


@pytest.fixture
def rpc(tmp_path) -> AsyncGenerator:
    rpc_server = Rpc(accounts_dir=str(tmp_path / "accounts"))
    with rpc_server:
        yield rpc_server


@pytest.fixture
def acfactory(rpc) -> AsyncGenerator:
    return ACFactory(DeltaChat(rpc))

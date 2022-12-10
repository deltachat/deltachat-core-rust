import json
import os
from typing import AsyncGenerator, List

import aiohttp
import pytest_asyncio

from .account import Account
from .client import Bot
from .deltachat import DeltaChat
from .rpc import Rpc


async def get_temp_credentials() -> dict:
    url = os.getenv("DCC_NEW_TMP_EMAIL")
    assert url, "Failed to get online account, DCC_NEW_TMP_EMAIL is not set"
    async with aiohttp.ClientSession() as session:
        async with session.post(url) as response:
            return json.loads(await response.text())


class ACFactory:
    def __init__(self, deltachat: DeltaChat) -> None:
        self.deltachat = deltachat

    async def get_unconfigured_account(self) -> Account:
        return await self.deltachat.add_account()

    async def get_unconfigured_bot(self) -> Bot:
        return Bot(await self.get_unconfigured_account())

    async def new_configured_account(self) -> Account:
        credentials = await get_temp_credentials()
        account = await self.get_unconfigured_account()
        assert not await account.is_configured()
        await account.set_config("addr", credentials["email"])
        await account.set_config("mail_pw", credentials["password"])
        await account.configure()
        assert await account.is_configured()
        return account

    async def new_configured_bot(self) -> Bot:
        credentials = await get_temp_credentials()
        bot = await self.get_unconfigured_bot()
        await bot.configure(credentials["email"], credentials["password"])
        return bot

    async def get_online_accounts(self, num: int) -> List[Account]:
        accounts = [await self.new_configured_account() for _ in range(num)]
        for account in accounts:
            await account.start_io()
        return accounts


@pytest_asyncio.fixture
async def rpc(tmp_path) -> AsyncGenerator:
    rpc_server = Rpc(accounts_dir=str(tmp_path / "accounts"))
    async with rpc_server:
        yield rpc_server


@pytest_asyncio.fixture
async def acfactory(rpc) -> AsyncGenerator:
    yield ACFactory(DeltaChat(rpc))

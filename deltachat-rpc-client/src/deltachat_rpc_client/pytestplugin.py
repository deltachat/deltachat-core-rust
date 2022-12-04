import asyncio
import json
import os
from typing import AsyncGenerator, List

import aiohttp
import pytest_asyncio

from .account import Account
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

    async def new_configured_account(self) -> Account:
        credentials = await get_temp_credentials()
        account = await self.deltachat.add_account()
        assert not await account.is_configured()
        await account.set_config("addr", credentials["email"])
        await account.set_config("mail_pw", credentials["password"])
        await account.configure()
        assert await account.is_configured()
        return account

    async def get_online_accounts(self, num: int) -> List[Account]:
        accounts = [await self.new_configured_account() for _ in range(num)]
        await self.deltachat.start_io()
        return accounts


@pytest_asyncio.fixture
async def rpc(tmp_path) -> AsyncGenerator:
    env = {**os.environ, "DC_ACCOUNTS_PATH": str(tmp_path / "accounts")}
    rpc_server = Rpc(env=env)
    async with rpc_server as rpc:
        yield rpc


@pytest_asyncio.fixture
async def acfactory(rpc) -> AsyncGenerator:
    yield ACFactory(DeltaChat(rpc))

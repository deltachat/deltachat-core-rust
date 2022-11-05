from .account import Account


class Deltachat:
    """
    Delta Chat account manager.
    This is the root of the object oriented API.
    """

    def __init__(self, rpc):
        self.rpc = rpc

    async def add_account(self):
        account_id = await self.rpc.add_account()
        return Account(self.rpc, account_id)

    async def get_all_accounts(self):
        account_ids = await self.rpc.get_all_account_ids()
        return [Account(self.rpc, account_id) for account_id in account_ids]

    async def start_io(self) -> None:
        await self.rpc.start_io_for_all_accounts()

    async def stop_io(self) -> None:
        await self.rpc.stop_io_for_all_accounts()

    async def maybe_network(self) -> None:
        await self.rpc.maybe_network()

    async def get_system_info(self):
        return await self.rpc.get_system_info()

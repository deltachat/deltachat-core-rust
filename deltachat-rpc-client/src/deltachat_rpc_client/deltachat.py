from typing import TYPE_CHECKING, Dict, List

from ._utils import AttrDict
from .account import Account

if TYPE_CHECKING:
    from .rpc import Rpc


class DeltaChat:
    """
    Delta Chat accounts manager.
    This is the root of the object oriented API.
    """

    def __init__(self, rpc: Rpc) -> None:
        self.rpc = rpc

    async def add_account(self) -> Account:
        """Create a new account database."""
        account_id = await self.rpc.add_account()
        return Account(self, account_id)

    async def get_all_accounts(self) -> List[Account]:
        """Return a list of all available accounts."""
        account_ids = await self.rpc.get_all_account_ids()
        return [Account(self, account_id) for account_id in account_ids]

    async def start_io(self) -> None:
        """Start the I/O of all accounts."""
        await self.rpc.start_io_for_all_accounts()

    async def stop_io(self) -> None:
        """Stop the I/O of all accounts."""
        await self.rpc.stop_io_for_all_accounts()

    async def maybe_network(self) -> None:
        """Indicate that the network likely has come back or just that the network
        conditions might have changed.
        """
        await self.rpc.maybe_network()

    async def get_system_info(self) -> AttrDict:
        """Get information about the Delta Chat core in this system."""
        return AttrDict(await self.rpc.get_system_info())

    async def set_translations(self, translations: Dict[str, str]) -> None:
        """Set stock translation strings."""
        await self.rpc.set_stock_strings(translations)

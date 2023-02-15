from typing import TYPE_CHECKING
from dataclasses import dataclass

from ._utils import AttrDict
from .rpc import Rpc

if TYPE_CHECKING:
    from .account import Account
    from .chat import Chat


@dataclass
class Contact:
    """
    Contact API.

    Essentially a wrapper for RPC, account ID and a contact ID.
    """

    account: "Account"
    id: int

    @property
    def _rpc(self) -> Rpc:
        return self.account._rpc

    async def block(self) -> None:
        """Block contact."""
        await self._rpc.block_contact(self.account.id, self.id)

    async def unblock(self) -> None:
        """Unblock contact."""
        await self._rpc.unblock_contact(self.account.id, self.id)

    async def delete(self) -> None:
        """Delete contact."""
        await self._rpc.delete_contact(self.account.id, self.id)

    async def set_name(self, name: str) -> None:
        """Change the name of this contact."""
        await self._rpc.change_contact_name(self.account.id, self.id, name)

    async def get_encryption_info(self) -> str:
        """Get a multi-line encryption info, containing your fingerprint and
        the fingerprint of the contact.
        """
        return await self._rpc.get_contact_encryption_info(self.account.id, self.id)

    async def get_snapshot(self) -> AttrDict:
        """Return a dictionary with a snapshot of all contact properties."""
        snapshot = AttrDict(await self._rpc.get_contact(self.account.id, self.id))
        snapshot["contact"] = self
        return snapshot

    async def create_chat(self) -> "Chat":
        """Create or get an existing 1:1 chat for this contact."""
        from .chat import Chat

        return Chat(
            self.account,
            await self._rpc.create_chat_by_contact_id(self.account.id, self.id),
        )

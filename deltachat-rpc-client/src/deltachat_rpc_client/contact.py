from typing import TYPE_CHECKING

from ._utils import AttrDict
from .rpc import Rpc

if TYPE_CHECKING:
    from .account import Account
    from .chat import Chat


class Contact:
    """
    Contact API.

    Essentially a wrapper for RPC, account ID and a contact ID.
    """

    def __init__(self, account: "Account", contact_id: int) -> None:
        self.account = account
        self.id = contact_id

    def __eq__(self, other) -> bool:
        if not isinstance(other, Contact):
            return False
        return self.id == other.id and self.account == other.account

    def __ne__(self, other) -> bool:
        return not self == other

    def __repr__(self) -> str:
        return f"<Contact id={self.id} account={self.account.id}>"

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

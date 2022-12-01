from typing import TYPE_CHECKING

from .rpc import Rpc

if TYPE_CHECKING:
    from .chat import Chat


class Contact:
    """
    Contact API.

    Essentially a wrapper for RPC, account ID and a contact ID.
    """

    def __init__(self, rpc: Rpc, account_id: int, contact_id: int) -> None:
        self.rpc = rpc
        self.account_id = account_id
        self.contact_id = contact_id

    async def block(self) -> None:
        """Block contact."""
        await self.rpc.block_contact(self.account_id, self.contact_id)

    async def unblock(self) -> None:
        """Unblock contact."""
        await self.rpc.unblock_contact(self.account_id, self.contact_id)

    async def delete(self) -> None:
        """Delete contact."""
        await self.rpc.delete_contact(self.account_id, self.contact_id)

    async def change_name(self, name: str) -> None:
        await self.rpc.change_contact_name(self.account_id, self.contact_id, name)

    async def get_encryption_info(self) -> str:
        return await self.rpc.get_contact_encryption_info(
            self.account_id, self.contact_id
        )

    async def get_dictionary(self) -> dict:
        """Return a dictionary with a snapshot of all contact properties."""
        return await self.rpc.get_contact(self.account_id, self.contact_id)

    async def create_chat(self) -> "Chat":
        from .chat import Chat

        return Chat(
            self.rpc,
            self.account_id,
            await self.rpc.create_chat_by_contact_id(self.account_id, self.contact_id),
        )

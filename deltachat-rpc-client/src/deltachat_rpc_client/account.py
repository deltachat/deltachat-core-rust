from typing import Optional

from .chat import Chat
from .contact import Contact
from .message import Message


class Account:
    def __init__(self, rpc, account_id):
        self.rpc = rpc
        self.account_id = account_id

    def __repr__(self):
        return "<Account id={}>".format(self.account_id)

    async def wait_for_event(self):
        """Wait until the next event and return it."""
        return await self.rpc.wait_for_event(self.account_id)

    async def remove(self) -> None:
        """Remove the account."""
        await self.rpc.remove_account(self.account_id)

    async def start_io(self) -> None:
        """Start the account I/O."""
        await self.rpc.start_io(self.account_id)

    async def stop_io(self) -> None:
        """Stop the account I/O."""
        await self.rpc.stop_io(self.account_id)

    async def get_info(self):
        return await self.rpc.get_info(self.account_id)

    async def get_file_size(self):
        return await self.rpc.get_account_file_size(self.account_id)

    async def is_configured(self) -> bool:
        """Return True for configured accounts."""
        return await self.rpc.is_configured(self.account_id)

    async def set_config(self, key: str, value: Optional[str]):
        """Set the configuration value key pair."""
        await self.rpc.set_config(self.account_id, key, value)

    async def get_config(self, key: str) -> Optional[str]:
        """Get the configuration value."""
        return await self.rpc.get_config(self.account_id, key)

    async def configure(self):
        """Configure an account."""
        await self.rpc.configure(self.account_id)

    async def create_contact(self, address: str, name: Optional[str]) -> Contact:
        """Create a contact with the given address and, optionally, a name."""
        return Contact(
            self.rpc,
            self.account_id,
            await self.rpc.create_contact(self.account_id, address, name),
        )

    async def secure_join(self, qr: str) -> Chat:
        chat_id = await self.rpc.secure_join(self.account_id, qr)
        return Chat(self.rpc, self.account_id, chat_id)

    async def get_fresh_messages(self):
        """Return the list of fresh messages, newest messages first.

        This call is intended for displaying notifications.
        If you are writing a bot, use get_fresh_messages_in_arrival_order instead,
        to process oldest messages first.
        """
        fresh_msg_ids = await self.rpc.get_fresh_msgs(self.account_id)
        return [Message(self.rpc, self.account_id, msg_id) for msg_id in fresh_msg_ids]

    async def get_fresh_messages_in_arrival_order(self):
        """Return the list of fresh messages sorted in the order of their arrival, with ascending IDs."""
        fresh_msg_ids = sorted(await self.rpc.get_fresh_msgs(self.account_id))
        return [Message(self.rpc, self.account_id, msg_id) for msg_id in fresh_msg_ids]

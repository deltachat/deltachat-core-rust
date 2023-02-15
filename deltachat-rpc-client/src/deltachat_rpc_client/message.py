import json
from typing import TYPE_CHECKING, Union
from dataclasses import dataclass

from ._utils import AttrDict
from .contact import Contact
from .rpc import Rpc

if TYPE_CHECKING:
    from .account import Account


@dataclass
class Message:
    """Delta Chat Message object."""

    account: "Account"
    id: int

    @property
    def _rpc(self) -> Rpc:
        return self.account._rpc

    async def send_reaction(self, *reaction: str):
        """Send a reaction to this message."""
        await self._rpc.send_reaction(self.account.id, self.id, reaction)

    async def get_snapshot(self) -> AttrDict:
        """Get a snapshot with the properties of this message."""
        from .chat import Chat

        snapshot = AttrDict(await self._rpc.get_message(self.account.id, self.id))
        snapshot["chat"] = Chat(self.account, snapshot.chat_id)
        snapshot["sender"] = Contact(self.account, snapshot.from_id)
        snapshot["message"] = self
        return snapshot

    async def mark_seen(self) -> None:
        """Mark the message as seen."""
        await self._rpc.markseen_msgs(self.account.id, [self.id])

    async def send_webxdc_status_update(self, update: Union[dict, str], description: str) -> None:
        """Send a webxdc status update. This message must be a webxdc."""
        if not isinstance(update, str):
            update = json.dumps(update)
        await self._rpc.send_webxdc_status_update(self.account.id, self.id, update, description)

    async def get_webxdc_status_updates(self, last_known_serial: int = 0) -> list:
        return json.loads(await self._rpc.get_webxdc_status_updates(self.account.id, self.id, last_known_serial))

    async def get_webxdc_info(self) -> dict:
        return await self._rpc.get_webxdc_info(self.account.id, self.id)

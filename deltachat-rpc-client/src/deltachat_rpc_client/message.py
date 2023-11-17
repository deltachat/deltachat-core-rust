import json
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Union

from ._utils import AttrDict
from .contact import Contact

if TYPE_CHECKING:
    from .account import Account
    from .rpc import Rpc


@dataclass
class Message:
    """Delta Chat Message object."""

    account: "Account"
    id: int

    @property
    def _rpc(self) -> "Rpc":
        return self.account._rpc

    def send_reaction(self, *reaction: str):
        """Send a reaction to this message."""
        self._rpc.send_reaction(self.account.id, self.id, reaction)

    def get_snapshot(self) -> AttrDict:
        """Get a snapshot with the properties of this message."""
        from .chat import Chat

        snapshot = AttrDict(self._rpc.get_message(self.account.id, self.id))
        snapshot["chat"] = Chat(self.account, snapshot.chat_id)
        snapshot["sender"] = Contact(self.account, snapshot.from_id)
        snapshot["message"] = self
        return snapshot

    def get_reactions(self) -> Optional[AttrDict]:
        """Get message reactions."""
        reactions = self._rpc.get_message_reactions(self.account.id, self.id)
        if reactions:
            return AttrDict(reactions)
        return None

    def get_sender_contact(self) -> Contact:
        from_id = self.get_snapshot().from_id
        return self.account.get_contact_by_id(from_id)

    def mark_seen(self) -> None:
        """Mark the message as seen."""
        self._rpc.markseen_msgs(self.account.id, [self.id])

    def send_webxdc_status_update(self, update: Union[dict, str], description: str) -> None:
        """Send a webxdc status update. This message must be a webxdc."""
        if not isinstance(update, str):
            update = json.dumps(update)
        self._rpc.send_webxdc_status_update(self.account.id, self.id, update, description)

    def get_webxdc_status_updates(self, last_known_serial: int = 0) -> list:
        return json.loads(self._rpc.get_webxdc_status_updates(self.account.id, self.id, last_known_serial))

    def get_webxdc_info(self) -> dict:
        return self._rpc.get_webxdc_info(self.account.id, self.id)

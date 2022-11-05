from dataclasses import dataclass
from typing import Optional

from .chat import Chat
from .contact import Contact


class Message:
    def __init__(self, rpc, account_id, msg_id):
        self.rpc = rpc
        self.account_id = account_id
        self.msg_id = msg_id

    async def send_reaction(self, reactions):
        msg_id = await self.rpc.send_reaction(self.account_id, self.msg_id, reactions)
        return Message(self.rpc, self.account_id, msg_id)

    async def get_snapshot(self):
        message_object = await self.rpc.get_message(self.account_id, self.msg_id)
        return MessageSnapshot(
            message=self,
            chat=Chat(self.rpc, self.account_id, message_object["chatId"]),
            sender=Contact(self.rpc, self.account_id, message_object["fromId"]),
            text=message_object["text"],
            error=message_object.get("error"),
            is_info=message_object["isInfo"],
        )

    async def mark_seen(self) -> None:
        """Mark the message as seen."""
        await self.rpc.markseen_msgs(self.account_id, [self.msg_id])


@dataclass
class MessageSnapshot:
    message: Message
    chat: Chat
    sender: Contact
    text: str
    error: Optional[str]
    is_info: bool

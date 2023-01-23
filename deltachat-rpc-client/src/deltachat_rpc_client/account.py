from typing import TYPE_CHECKING, List, Optional, Tuple, Union

from ._utils import AttrDict
from .chat import Chat
from .const import ChatlistFlag, ContactFlag, SpecialContactId
from .contact import Contact
from .message import Message
from .rpc import Rpc

if TYPE_CHECKING:
    from .deltachat import DeltaChat


class Account:
    """Delta Chat account."""

    def __init__(self, manager: "DeltaChat", account_id: int) -> None:
        self.manager = manager
        self.id = account_id

    @property
    def _rpc(self) -> Rpc:
        return self.manager.rpc

    def __eq__(self, other) -> bool:
        if not isinstance(other, Account):
            return False
        return self.id == other.id and self.manager == other.manager

    def __ne__(self, other) -> bool:
        return not self == other

    def __repr__(self) -> str:
        return f"<Account id={self.id}>"

    async def wait_for_event(self) -> AttrDict:
        """Wait until the next event and return it."""
        return AttrDict(await self._rpc.wait_for_event(self.id))

    async def remove(self) -> None:
        """Remove the account."""
        await self._rpc.remove_account(self.id)

    async def start_io(self) -> None:
        """Start the account I/O."""
        await self._rpc.start_io(self.id)

    async def stop_io(self) -> None:
        """Stop the account I/O."""
        await self._rpc.stop_io(self.id)

    async def get_info(self) -> AttrDict:
        """Return dictionary of this account configuration parameters."""
        return AttrDict(await self._rpc.get_info(self.id))

    async def get_size(self) -> int:
        """Get the combined filesize of an account in bytes."""
        return await self._rpc.get_account_file_size(self.id)

    async def is_configured(self) -> bool:
        """Return True if this account is configured."""
        return await self._rpc.is_configured(self.id)

    async def set_config(self, key: str, value: Optional[str] = None) -> None:
        """Set configuration value."""
        await self._rpc.set_config(self.id, key, value)

    async def get_config(self, key: str) -> Optional[str]:
        """Get configuration value."""
        return await self._rpc.get_config(self.id, key)

    async def update_config(self, **kwargs) -> None:
        """update config values."""
        for key, value in kwargs.items():
            await self.set_config(key, value)

    async def set_avatar(self, img_path: Optional[str] = None) -> None:
        """Set self avatar.

        Passing None will discard the currently set avatar.
        """
        await self.set_config("selfavatar", img_path)

    async def get_avatar(self) -> Optional[str]:
        """Get self avatar."""
        return await self.get_config("selfavatar")

    async def configure(self) -> None:
        """Configure an account."""
        await self._rpc.configure(self.id)

    async def create_contact(self, obj: Union[int, str, Contact], name: Optional[str] = None) -> Contact:
        """Create a new Contact or return an existing one.

        Calling this method will always result in the same
        underlying contact id.  If there already is a Contact
        with that e-mail address, it is unblocked and its display
        name is updated if specified.

        :param obj: email-address or contact id.
        :param name: (optional) display name for this contact.
        """
        if isinstance(obj, int):
            obj = Contact(self, obj)
        if isinstance(obj, Contact):
            obj = (await obj.get_snapshot()).address
        return Contact(self, await self._rpc.create_contact(self.id, obj, name))

    def get_contact_by_id(self, contact_id: int) -> Contact:
        """Return Contact instance for the given contact ID."""
        return Contact(self, contact_id)

    async def get_contact_by_addr(self, address: str) -> Optional[Contact]:
        """Check if an e-mail address belongs to a known and unblocked contact."""
        contact_id = await self._rpc.lookup_contact_id_by_addr(self.id, address)
        return contact_id and Contact(self, contact_id)

    async def get_blocked_contacts(self) -> List[AttrDict]:
        """Return a list with snapshots of all blocked contacts."""
        contacts = await self._rpc.get_blocked_contacts(self.id)
        return [AttrDict(contact=Contact(self, contact["id"]), **contact) for contact in contacts]

    async def get_contacts(
        self,
        query: Optional[str] = None,
        with_self: bool = False,
        verified_only: bool = False,
        snapshot: bool = False,
    ) -> Union[List[Contact], List[AttrDict]]:
        """Get a filtered list of contacts.

        :param query: if a string is specified, only return contacts
                      whose name or e-mail matches query.
        :param with_self: if True the self-contact is also included if it matches the query.
        :param only_verified: if True only return verified contacts.
        :param snapshot: If True return a list of contact snapshots instead of Contact instances.
        """
        flags = 0
        if verified_only:
            flags |= ContactFlag.VERIFIED_ONLY
        if with_self:
            flags |= ContactFlag.ADD_SELF

        if snapshot:
            contacts = await self._rpc.get_contacts(self.id, flags, query)
            return [AttrDict(contact=Contact(self, contact["id"]), **contact) for contact in contacts]
        contacts = await self._rpc.get_contact_ids(self.id, flags, query)
        return [Contact(self, contact_id) for contact_id in contacts]

    @property
    def self_contact(self) -> Contact:
        """This account's identity as a Contact."""
        return Contact(self, SpecialContactId.SELF)

    async def get_chatlist(
        self,
        query: Optional[str] = None,
        contact: Optional[Contact] = None,
        archived_only: bool = False,
        for_forwarding: bool = False,
        no_specials: bool = False,
        alldone_hint: bool = False,
        snapshot: bool = False,
    ) -> Union[List[Chat], List[AttrDict]]:
        """Return list of chats.

        :param query: if a string is specified only chats matching this query are returned.
        :param contact: if a contact is specified only chats including this contact are returned.
        :param archived_only: if True only archived chats are returned.
        :param for_forwarding: if True the chat list is sorted with "Saved messages" at the top
                               and withot "Device chat" and contact requests.
        :param no_specials: if True archive link is not added to the list.
        :param alldone_hint: if True the "all done hint" special chat will be added to the list
                             as needed.
        :param snapshot: If True return a list of chat snapshots instead of Chat instances.
        """
        flags = 0
        if archived_only:
            flags |= ChatlistFlag.ARCHIVED_ONLY
        if for_forwarding:
            flags |= ChatlistFlag.FOR_FORWARDING
        if no_specials:
            flags |= ChatlistFlag.NO_SPECIALS
        if alldone_hint:
            flags |= ChatlistFlag.ADD_ALLDONE_HINT

        entries = await self._rpc.get_chatlist_entries(self.id, flags, query, contact and contact.id)
        if not snapshot:
            return [Chat(self, entry[0]) for entry in entries]

        items = await self._rpc.get_chatlist_items_by_entries(self.id, entries)
        chats = []
        for item in items.values():
            item["chat"] = Chat(self, item["id"])
            chats.append(AttrDict(item))
        return chats

    async def create_group(self, name: str, protect: bool = False) -> Chat:
        """Create a new group chat.

        After creation, the group has only self-contact as member and is in unpromoted state.
        """
        return Chat(self, await self._rpc.create_group_chat(self.id, name, protect))

    def get_chat_by_id(self, chat_id: int) -> Chat:
        """Return the Chat instance with the given ID."""
        return Chat(self, chat_id)

    async def secure_join(self, qrdata: str) -> Chat:
        """Continue a Setup-Contact or Verified-Group-Invite protocol started on
        another device.

        The function returns immediately and the handshake runs in background, sending
        and receiving several messages.
        Subsequent calls of `secure_join()` will abort previous, unfinished handshakes.
        See https://countermitm.readthedocs.io/en/latest/new.html for protocol details.

        :param qrdata: The text of the scanned QR code.
        """
        return Chat(self, await self._rpc.secure_join(self.id, qrdata))

    async def get_qr_code(self) -> Tuple[str, str]:
        """Get Setup-Contact QR Code text and SVG data.

        this data needs to be transferred to another Delta Chat account
        in a second channel, typically used by mobiles with QRcode-show + scan UX.
        """
        return await self._rpc.get_chat_securejoin_qr_code_svg(self.id, None)

    def get_message_by_id(self, msg_id: int) -> Message:
        """Return the Message instance with the given ID."""
        return Message(self, msg_id)

    async def mark_seen_messages(self, messages: List[Message]) -> None:
        """Mark the given set of messages as seen."""
        await self._rpc.markseen_msgs(self.id, [msg.id for msg in messages])

    async def delete_messages(self, messages: List[Message]) -> None:
        """Delete messages (local and remote)."""
        await self._rpc.delete_messages(self.id, [msg.id for msg in messages])

    async def get_fresh_messages(self) -> List[Message]:
        """Return the list of fresh messages, newest messages first.

        This call is intended for displaying notifications.
        If you are writing a bot, use `get_fresh_messages_in_arrival_order()` instead,
        to process oldest messages first.
        """
        fresh_msg_ids = await self._rpc.get_fresh_msgs(self.id)
        return [Message(self, msg_id) for msg_id in fresh_msg_ids]

    async def get_fresh_messages_in_arrival_order(self) -> List[Message]:
        """Return fresh messages list sorted in the order of their arrival, with ascending IDs."""
        fresh_msg_ids = sorted(await self._rpc.get_fresh_msgs(self.id))
        return [Message(self, msg_id) for msg_id in fresh_msg_ids]

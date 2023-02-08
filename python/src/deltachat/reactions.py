"""The Reactions object."""

from .capi import ffi, lib
from .cutil import from_dc_charpointer, iter_array


class Reactions(object):
    """Reactions object.

    You obtain instances of it through :class:`deltachat.message.Message`.
    """

    def __init__(self, account, dc_reactions):
        assert isinstance(account._dc_context, ffi.CData)
        assert isinstance(dc_reactions, ffi.CData)
        assert dc_reactions != ffi.NULL
        self.account = account
        self._dc_reactions = dc_reactions

    def __repr__(self):
        return f"<Reactions dc_reactions={self._dc_reactions}>"

    @classmethod
    def from_msg(cls, msg):
        assert msg.id > 0
        return cls(
            msg.account,
            ffi.gc(lib.dc_get_msg_reactions(msg.account._dc_context, msg.id), lib.dc_reactions_unref),
        )

    def get_contacts(self) -> list:
        """Get list of contacts reacted to the message.

        :returns: list of :class:`deltachat.contact.Contact` objects for this reaction.
        """
        from .contact import Contact

        dc_array = ffi.gc(lib.dc_reactions_get_contacts(self._dc_reactions), lib.dc_array_unref)
        return list(iter_array(dc_array, lambda x: Contact(self.account, x)))

    def get_by_contact(self, contact) -> str:
        """Get a string containing space-separated reactions of a single :class:`deltachat.contact.Contact`."""
        return from_dc_charpointer(lib.dc_reactions_get_by_contact_id(self._dc_reactions, contact.id))

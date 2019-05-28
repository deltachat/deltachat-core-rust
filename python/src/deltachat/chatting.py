""" chatting related objects: Contact, Chat, Message. """

import os

from . import props
from .cutil import as_dc_charpointer, from_dc_charpointer, iter_array
from .capi import lib, ffi
from . import const
import attr
from attr import validators as v
from .message import Message


@attr.s
class Contact(object):
    """ Delta-Chat Contact.

    You obtain instances of it through :class:`deltachat.account.Account`.
    """
    _dc_context = attr.ib(validator=v.instance_of(ffi.CData))
    id = attr.ib(validator=v.instance_of(int))

    @property
    def _dc_contact(self):
        return ffi.gc(
            lib.dc_get_contact(self._dc_context, self.id),
            lib.dc_contact_unref
        )

    @props.with_doc
    def addr(self):
        """ normalized e-mail address for this account. """
        return from_dc_charpointer(lib.dc_contact_get_addr(self._dc_contact))

    @props.with_doc
    def display_name(self):
        """ display name for this contact. """
        return from_dc_charpointer(lib.dc_contact_get_display_name(self._dc_contact))

    def is_blocked(self):
        """ Return True if the contact is blocked. """
        return lib.dc_contact_is_blocked(self._dc_contact)

    def is_verified(self):
        """ Return True if the contact is verified. """
        return lib.dc_contact_is_verified(self._dc_contact)


@attr.s
class Chat(object):
    """ Chat object which manages members and through which you can send and retrieve messages.

    You obtain instances of it through :class:`deltachat.account.Account`.
    """
    _dc_context = attr.ib(validator=v.instance_of(ffi.CData))
    id = attr.ib(validator=v.instance_of(int))

    @property
    def _dc_chat(self):
        return ffi.gc(
            lib.dc_get_chat(self._dc_context, self.id),
            lib.dc_chat_unref
        )

    def delete(self):
        """Delete this chat and all its messages.

        Note:

        - does not delete messages on server
        - the chat or contact is not blocked, new message will arrive
        """
        lib.dc_delete_chat(self._dc_context, self.id)

    # ------  chat status/metadata API ------------------------------

    def is_deaddrop(self):
        """ return true if this chat is a deaddrop chat.

        :returns: True if chat is the deaddrop chat, False otherwise.
        """
        return self.id == const.DC_CHAT_ID_DEADDROP

    def is_promoted(self):
        """ return True if this chat is promoted, i.e.
        the member contacts are aware of their membership,
        have been sent messages.

        :returns: True if chat is promoted, False otherwise.
        """
        return not lib.dc_chat_is_unpromoted(self._dc_chat)

    def get_name(self):
        """ return name of this chat.

        :returns: unicode name
        """
        return from_dc_charpointer(lib.dc_chat_get_name(self._dc_chat))

    def set_name(self, name):
        """ set name of this chat.

        :param: name as a unicode string.
        :returns: None
        """
        name = as_dc_charpointer(name)
        return lib.dc_set_chat_name(self._dc_context, self.id, name)

    def get_type(self):
        """ return type of this chat.

        :returns: one of const.DC_CHAT_TYPE_*
        """
        return lib.dc_chat_get_type(self._dc_chat)
    
    # ------  chat messaging API ------------------------------

    def send_text(self, text):
        """ send a text message and return the resulting Message instance.

        :param msg: unicode text
        :raises ValueError: if message can not be send/chat does not exist.
        :returns: the resulting :class:`deltachat.message.Message` instance
        """
        msg = as_dc_charpointer(text)
        msg_id = lib.dc_send_text_msg(self._dc_context, self.id, msg)
        if msg_id == 0:
            raise ValueError("message could not be send, does chat exist?")
        return Message.from_db(self._dc_context, msg_id)

    def send_file(self, path, mime_type="application/octet-stream"):
        """ send a file and return the resulting Message instance.

        :param path: path to the file.
        :param mime_type: the mime-type of this file, defaults to application/octet-stream.
        :raises ValueError: if message can not be send/chat does not exist.
        :returns: the resulting :class:`deltachat.message.Message` instance
        """
        path = as_dc_charpointer(path)
        mtype = as_dc_charpointer(mime_type)
        msg = Message.new(self._dc_context, "file")
        msg.set_file(path, mtype)
        msg_id = lib.dc_send_msg(self._dc_context, self.id, msg._dc_msg)
        if msg_id == 0:
            raise ValueError("message could not be send, does chat exist?")
        return Message.from_db(self._dc_context, msg_id)

    def send_image(self, path):
        """ send an image message and return the resulting Message instance.

        :param path: path to an image file.
        :raises ValueError: if message can not be send/chat does not exist.
        :returns: the resulting :class:`deltachat.message.Message` instance
        """
        if not os.path.exists(path):
            raise ValueError("path does not exist: {!r}".format(path))
        msg = Message.new(self._dc_context, "image")
        msg.set_file(path)
        msg_id = lib.dc_send_msg(self._dc_context, self.id, msg._dc_msg)
        return Message.from_db(self._dc_context, msg_id)

    def prepare_file(self, path, mime_type=None, view_type="file"):
        """ prepare a message for sending and return the resulting Message instance.

        To actually send the message, call :meth:`send_prepared`.
        The file must be inside the blob directory.

        :param path: path to the file.
        :param mime_type: the mime-type of this file, defaults to auto-detection.
        :param view_type: passed to :meth:`MessageType.new`.
        :raises ValueError: if message can not be prepared/chat does not exist.
        :returns: the resulting :class:`Message` instance
        """
        path = as_dc_charpointer(path)
        mtype = as_dc_charpointer(mime_type)
        msg = Message.new(self._dc_context, view_type)
        msg.set_file(path, mtype)
        msg_id = lib.dc_prepare_msg(self._dc_context, self.id, msg._dc_msg)
        if msg_id == 0:
            raise ValueError("message could not be prepared, does chat exist?")
        return Message.from_db(self._dc_context, msg_id)

    def send_prepared(self, message):
        """ send a previously prepared message.

        :param message: a :class:`Message` instance previously returned by
                        :meth:`prepare_file`.
        :raises ValueError: if message can not be sent.
        :returns: a :class:`deltachat.message.Message` instance with updated state
        """
        msg_id = lib.dc_send_msg(self._dc_context, 0, message._dc_msg)
        if msg_id == 0:
            raise ValueError("message could not be sent")
        return Message.from_db(self._dc_context, msg_id)

    def get_messages(self):
        """ return list of messages in this chat.

        :returns: list of :class:`deltachat.message.Message` objects for this chat.
        """
        dc_array = ffi.gc(
            lib.dc_get_chat_msgs(self._dc_context, self.id, 0, 0),
            lib.dc_array_unref
        )
        return list(iter_array(dc_array, lambda x: Message.from_db(self._dc_context, x)))

    def count_fresh_messages(self):
        """ return number of fresh messages in this chat.

        :returns: number of fresh messages
        """
        return lib.dc_get_fresh_msg_cnt(self._dc_context, self.id)

    def mark_noticed(self):
        """ mark all messages in this chat as noticed.

        Noticed messages are no longer fresh.
        """
        return lib.dc_marknoticed_chat(self._dc_context, self.id)

    # ------  group management API ------------------------------

    def add_contact(self, contact):
        """ add a contact to this chat.

        :params: contact object.
        :raises ValueError: if contact could not be added
        :returns: None
        """
        ret = lib.dc_add_contact_to_chat(self._dc_context, self.id, contact.id)
        if ret != 1:
            raise ValueError("could not add contact {!r} to chat".format(contact))

    def remove_contact(self, contact):
        """ remove a contact from this chat.

        :params: contact object.
        :raises ValueError: if contact could not be removed
        :returns: None
        """
        ret = lib.dc_remove_contact_from_chat(self._dc_context, self.id, contact.id)
        if ret != 1:
            raise ValueError("could not remove contact {!r} from chat".format(contact))

    def get_contacts(self):
        """ get all contacts for this chat.
        :params: contact object.
        :raises ValueError: if contact could not be added
        :returns: list of :class:`deltachat.chatting.Contact` objects for this chat

        """
        dc_array = ffi.gc(
            lib.dc_get_chat_contacts(self._dc_context, self.id),
            lib.dc_array_unref
        )
        return list(iter_array(
            dc_array, lambda id: Contact(self._dc_context, id))
        )

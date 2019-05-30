""" chatting related objects: Contact, Chat, Message. """

import os
from . import props
from .cutil import from_dc_charpointer, as_dc_charpointer
from .capi import lib, ffi
from . import const
from datetime import datetime
import attr
from attr import validators as v


@attr.s
class Message(object):
    """ Message object.

    You obtain instances of it through :class:`deltachat.account.Account` or
    :class:`deltachat.chatting.Chat`.
    """
    _dc_context = attr.ib(validator=v.instance_of(ffi.CData))
    try:
        id = attr.ib(validator=v.instance_of((int, long)))
    except NameError:  # py35
        id = attr.ib(validator=v.instance_of(int))

    @property
    def _dc_msg(self):
        if self.id > 0:
            return ffi.gc(
                lib.dc_get_msg(self._dc_context, self.id),
                lib.dc_msg_unref
            )
        return self._dc_msg_volatile

    @classmethod
    def from_db(cls, _dc_context, id):
        assert id > 0
        return cls(_dc_context, id)

    @classmethod
    def new(cls, dc_context, view_type):
        """ create a non-persistent method. """
        msg = cls(dc_context, 0)
        view_type_code = MessageType.get_typecode(view_type)
        msg._dc_msg_volatile = ffi.gc(
            lib.dc_msg_new(dc_context, view_type_code),
            lib.dc_msg_unref
        )
        return msg

    def get_state(self):
        """ get the message in/out state.

        :returns: :class:`deltachat.message.MessageState`
        """
        return MessageState(self)

    @props.with_doc
    def text(self):
        """unicode text of this messages (might be empty if not a text message). """
        return from_dc_charpointer(lib.dc_msg_get_text(self._dc_msg))

    def set_text(self, text):
        """set text of this message. """
        return lib.dc_msg_set_text(self._dc_msg, as_dc_charpointer(text))

    @props.with_doc
    def filename(self):
        """filename if there was an attachment, otherwise empty string. """
        return from_dc_charpointer(lib.dc_msg_get_file(self._dc_msg))

    def set_file(self, path, mime_type=None):
        """set file for this message. """
        mtype = ffi.NULL if mime_type is None else mime_type
        assert os.path.exists(path)
        lib.dc_msg_set_file(self._dc_msg, as_dc_charpointer(path), mtype)

    @props.with_doc
    def basename(self):
        """basename of the attachment if it exists, otherwise empty string. """
        return from_dc_charpointer(lib.dc_msg_get_filename(self._dc_msg))

    @props.with_doc
    def filemime(self):
        """mime type of the file (if it exists)"""
        return from_dc_charpointer(lib.dc_msg_get_filemime(self._dc_msg))

    @props.with_doc
    def view_type(self):
        """the view type of this message.

        :returns: a :class:`deltachat.message.MessageType` instance.
        """
        return MessageType(lib.dc_msg_get_viewtype(self._dc_msg))

    @props.with_doc
    def time_sent(self):
        """UTC time when the message was sent.

        :returns: naive datetime.datetime() object.
        """
        ts = lib.dc_msg_get_timestamp(self._dc_msg)
        return datetime.utcfromtimestamp(ts)

    @props.with_doc
    def time_received(self):
        """UTC time when the message was received.

        :returns: naive datetime.datetime() object or None if message is an outgoing one.
        """
        ts = lib.dc_msg_get_received_timestamp(self._dc_msg)
        if ts:
            return datetime.utcfromtimestamp(ts)

    def get_mime_headers(self):
        """ return mime-header object for an incoming message.

        This only returns a non-None object if ``save_mime_headers``
        config option was set and the message is incoming.

        :returns: email-mime message object (with headers only, no body).
        """
        import email.parser
        mime_headers = lib.dc_get_mime_headers(self._dc_context, self.id)
        if mime_headers:
            s = ffi.string(mime_headers)
            if isinstance(s, bytes):
                s = s.decode("ascii")
            return email.message_from_string(s)

    @property
    def chat(self):
        """chat this message was posted in.

        :returns: :class:`deltachat.chatting.Chat` object
        """
        from .chatting import Chat
        chat_id = lib.dc_msg_get_chat_id(self._dc_msg)
        return Chat(self._dc_context, chat_id)

    def get_sender_contact(self):
        """return the contact of who wrote the message.

        :returns: :class:`deltachat.chatting.Contact` instance
        """
        from .chatting import Contact
        contact_id = lib.dc_msg_get_from_id(self._dc_msg)
        return Contact(self._dc_context, contact_id)


@attr.s
class MessageType(object):
    """ DeltaChat message type, with is_* methods. """
    _type = attr.ib(validator=v.instance_of(int))
    _mapping = {
            const.DC_MSG_TEXT: 'text',
            const.DC_MSG_IMAGE: 'image',
            const.DC_MSG_GIF: 'gif',
            const.DC_MSG_AUDIO: 'audio',
            const.DC_MSG_VIDEO: 'video',
            const.DC_MSG_FILE: 'file'
    }

    @classmethod
    def get_typecode(cls, view_type):
        for code, value in cls._mapping.items():
            if value == view_type:
                return code
        raise ValueError("message typecode not found for {!r}".format(view_type))

    @props.with_doc
    def name(self):
        """ human readable type name. """
        return self._mapping.get(self._type, "")

    def is_text(self):
        """ return True if it's a text message. """
        return self._type == const.DC_MSG_TEXT

    def is_image(self):
        """ return True if it's an image message. """
        return self._type == const.DC_MSG_IMAGE

    def is_gif(self):
        """ return True if it's a gif message. """
        return self._type == const.DC_MSG_GIF

    def is_audio(self):
        """ return True if it's an audio message. """
        return self._type == const.DC_MSG_AUDIO

    def is_video(self):
        """ return True if it's a video message. """
        return self._type == const.DC_MSG_VIDEO

    def is_file(self):
        """ return True if it's a file message. """
        return self._type == const.DC_MSG_FILE


@attr.s
class MessageState(object):
    """ Current Message In/Out state, updated on each call of is_* methods.
    """
    message = attr.ib(validator=v.instance_of(Message))

    @property
    def _msgstate(self):
        return lib.dc_msg_get_state(self.message._dc_msg)

    def is_in_fresh(self):
        """ return True if Message is incoming fresh message (un-noticed).

        Fresh messages are not noticed nor seen and are typically
        shown in notifications.
        """
        return self._msgstate == const.DC_STATE_IN_FRESH

    def is_in_noticed(self):
        """Return True if Message is incoming and noticed.

        Eg. chat opened but message not yet read - noticed messages
        are not counted as unread but were not marked as read nor resulted in MDNs.
        """
        return self._msgstate == const.DC_STATE_IN_NOTICED

    def is_in_seen(self):
        """Return True if Message is incoming, noticed and has been seen.

        Eg. chat opened but message not yet read - noticed messages
        are not counted as unread but were not marked as read nor resulted in MDNs.
        """
        return self._msgstate == const.DC_STATE_IN_SEEN

    def is_out_preparing(self):
        """Return True if Message is outgoing, but its file is being prepared.
        """
        return self._msgstate == const.DC_STATE_OUT_PREPARING

    def is_out_pending(self):
        """Return True if Message is outgoing, but is pending (no single checkmark).
        """
        return self._msgstate == const.DC_STATE_OUT_PENDING

    def is_out_failed(self):
        """Return True if Message is unrecoverably failed.
        """
        return self._msgstate == const.DC_STATE_OUT_FAILED

    def is_out_delivered(self):
        """Return True if Message was successfully delivered to the server (one checkmark).

        Note, that already delivered messages may get into the state  is_out_failed().
        """
        return self._msgstate == const.DC_STATE_OUT_DELIVERED

    def is_out_mdn_received(self):
        """Return True if message was marked as read by the recipient(s) (two checkmarks;
        this requires goodwill on the receiver's side). If a sent message changes to this
        state, you'll receive the event DC_EVENT_MSG_READ.
        """
        return self._msgstate == const.DC_STATE_OUT_MDN_RCVD

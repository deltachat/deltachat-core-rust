""" Chat and Location related API. """

import mimetypes
import calendar
import json
from datetime import datetime
import os
from .cutil import as_dc_charpointer, from_dc_charpointer, iter_array
from .capi import lib, ffi
from . import const
from .message import Message


class Chat(object):
    """ Chat object which manages members and through which you can send and retrieve messages.

    You obtain instances of it through :class:`deltachat.account.Account`.
    """

    def __init__(self, account, id):
        self.account = account
        self._dc_context = account._dc_context
        self.id = id

    def __eq__(self, other):
        return self.id == getattr(other, "id", None) and \
               self._dc_context == getattr(other, "_dc_context", None)

    def __ne__(self, other):
        return not (self == other)

    def __repr__(self):
        return "<Chat id={} name={} dc_context={}>".format(self.id, self.get_name(), self._dc_context)

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

    def is_verified(self):
        """ return True if this chat is a verified group.

        :returns: True if chat is verified, False otherwise.
        """
        return lib.dc_chat_is_verified(self._dc_chat)

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

    def get_join_qr(self):
        """ get/create Join-Group QR Code as ascii-string.

        this string needs to be transferred to another DC account
        in a second channel (typically used by mobiles with QRcode-show + scan UX)
        where account.join_with_qrcode(qr) needs to be called.
        """
        res = lib.dc_get_securejoin_qr(self._dc_context, self.id)
        return from_dc_charpointer(res)

    # ------  chat messaging API ------------------------------

    def send_msg(self, msg):
        """send a message by using a ready Message object.

        :param msg: a :class:`deltachat.message.Message` instance
           previously returned by
           e.g. :meth:`deltachat.message.Message.new_empty` or
           :meth:`prepare_file`.
        :raises ValueError: if message can not be sent.

        :returns: a :class:`deltachat.message.Message` instance as
           sent out.  This is the same object as was passed in, which
           has been modified with the new state of the core.
        """
        if msg.is_out_preparing():
            assert msg.id != 0
            # get a fresh copy of dc_msg, the core needs it
            msg = Message.from_db(self.account, msg.id)
        sent_id = lib.dc_send_msg(self._dc_context, self.id, msg._dc_msg)
        if sent_id == 0:
            raise ValueError("message could not be sent")
        # modify message in place to avoid bad state for the caller
        msg._dc_msg = Message.from_db(self.account, sent_id)._dc_msg
        return msg

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
        return Message.from_db(self.account, msg_id)

    def send_file(self, path, mime_type="application/octet-stream"):
        """ send a file and return the resulting Message instance.

        :param path: path to the file.
        :param mime_type: the mime-type of this file, defaults to application/octet-stream.
        :raises ValueError: if message can not be send/chat does not exist.
        :returns: the resulting :class:`deltachat.message.Message` instance
        """
        msg = Message.new_empty(self.account, view_type="file")
        msg.set_file(path, mime_type)
        sent_id = lib.dc_send_msg(self._dc_context, self.id, msg._dc_msg)
        if sent_id == 0:
            raise ValueError("message could not be sent")
        return Message.from_db(self.account, sent_id)

    def send_image(self, path):
        """ send an image message and return the resulting Message instance.

        :param path: path to an image file.
        :raises ValueError: if message can not be send/chat does not exist.
        :returns: the resulting :class:`deltachat.message.Message` instance
        """
        mime_type = mimetypes.guess_type(path)[0]
        msg = Message.new_empty(self.account, view_type="image")
        msg.set_file(path, mime_type)
        sent_id = lib.dc_send_msg(self._dc_context, self.id, msg._dc_msg)
        if sent_id == 0:
            raise ValueError("message could not be sent")
        return Message.from_db(self.account, sent_id)

    def prepare_message(self, msg):
        """ create a new prepared message.

        :param msg: the message to be prepared.
        :returns: :class:`deltachat.message.Message` instance.
        """
        msg_id = lib.dc_prepare_msg(self._dc_context, self.id, msg._dc_msg)
        if msg_id == 0:
            raise ValueError("message could not be prepared")
        # invalidate passed in message which is not safe to use anymore
        msg._dc_msg = msg.id = None
        return Message.from_db(self.account, msg_id)

    def prepare_message_file(self, path, mime_type=None, view_type="file"):
        """ prepare a message for sending and return the resulting Message instance.

        To actually send the message, call :meth:`send_prepared`.
        The file must be inside the blob directory.

        :param path: path to the file.
        :param mime_type: the mime-type of this file, defaults to auto-detection.
        :param view_type: "text", "image", "gif", "audio", "video", "file"
        :raises ValueError: if message can not be prepared/chat does not exist.
        :returns: the resulting :class:`Message` instance
        """
        msg = Message.new_empty(self.account, view_type)
        msg.set_file(path, mime_type)
        return self.prepare_message(msg)

    def send_prepared(self, message):
        """ send a previously prepared message.

        :param message: a :class:`Message` instance previously returned by
                        :meth:`prepare_file`.
        :raises ValueError: if message can not be sent.
        :returns: a :class:`deltachat.message.Message` instance as sent out.
        """
        assert message.id != 0 and message.is_out_preparing()
        # get a fresh copy of dc_msg, the core needs it
        msg = Message.from_db(self.account, message.id)

        # pass 0 as chat-id because core-docs say it's ok when out-preparing
        sent_id = lib.dc_send_msg(self._dc_context, 0, msg._dc_msg)
        if sent_id == 0:
            raise ValueError("message could not be sent")
        assert sent_id == msg.id
        # modify message in place to avoid bad state for the caller
        msg._dc_msg = Message.from_db(self.account, sent_id)._dc_msg

    def set_draft(self, message):
        """ set message as draft.

        :param message: a :class:`Message` instance
        :returns: None
        """
        if message is None:
            lib.dc_set_draft(self._dc_context, self.id, ffi.NULL)
        else:
            lib.dc_set_draft(self._dc_context, self.id, message._dc_msg)

    def get_draft(self):
        """ get draft message for this chat.

        :param message: a :class:`Message` instance
        :returns: Message object or None (if no draft available)
        """
        x = lib.dc_get_draft(self._dc_context, self.id)
        if x == ffi.NULL:
            return None
        dc_msg = ffi.gc(x, lib.dc_msg_unref)
        return Message(self.account, dc_msg)

    def get_messages(self):
        """ return list of messages in this chat.

        :returns: list of :class:`deltachat.message.Message` objects for this chat.
        """
        dc_array = ffi.gc(
            lib.dc_get_chat_msgs(self._dc_context, self.id, 0, 0),
            lib.dc_array_unref
        )
        return list(iter_array(dc_array, lambda x: Message.from_db(self.account, x)))

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

    def get_summary(self):
        """ return dictionary with summary information. """
        dc_res = lib.dc_chat_get_info_json(self._dc_context, self.id)
        s = from_dc_charpointer(dc_res)
        return json.loads(s)

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
        :returns: list of :class:`deltachat.contact.Contact` objects for this chat

        """
        from .contact import Contact
        dc_array = ffi.gc(
            lib.dc_get_chat_contacts(self._dc_context, self.id),
            lib.dc_array_unref
        )
        return list(iter_array(
            dc_array, lambda id: Contact(self._dc_context, id))
        )

    def set_profile_image(self, img_path):
        """Set group profile image.

        If the group is already promoted (any message was sent to the group),
        all group members are informed by a special status message that is sent
        automatically by this function.
        :params img_path: path to image object
        :raises ValueError: if profile image could not be set
        :returns: None
        """
        assert os.path.exists(img_path), img_path
        p = as_dc_charpointer(img_path)
        res = lib.dc_set_chat_profile_image(self._dc_context, self.id, p)
        if res != 1:
            raise ValueError("Setting Profile Image {!r} failed".format(p))

    def remove_profile_image(self):
        """remove group profile image.

        If the group is already promoted (any message was sent to the group),
        all group members are informed by a special status message that is sent
        automatically by this function.
        :raises ValueError: if profile image could not be reset
        :returns: None
        """
        res = lib.dc_set_chat_profile_image(self._dc_context, self.id, ffi.NULL)
        if res != 1:
            raise ValueError("Removing Profile Image failed")

    def get_profile_image(self):
        """Get group profile image.

        For groups, this is the image set by any group member using
        set_chat_profile_image(). For normal chats, this is the image
        set by each remote user on their own using dc_set_config(context,
        "selfavatar", image).
        :returns: path to profile image, None if no profile image exists.
        """
        dc_res = lib.dc_chat_get_profile_image(self._dc_chat)
        if dc_res == ffi.NULL:
            return None
        return from_dc_charpointer(dc_res)

    def get_color(self):
        """return the color of the chat.
        :returns: color as 0x00rrggbb
        """
        return lib.dc_chat_get_color(self._dc_chat)

    def get_subtitle(self):
        """return the subtitle of the chat
        :returns: the subtitle
        """
        return from_dc_charpointer(lib.dc_chat_get_subtitle(self._dc_chat))

    # ------  location streaming API ------------------------------

    def is_sending_locations(self):
        """return True if this chat has location-sending enabled currently.
        :returns: True if location sending is enabled.
        """
        return lib.dc_is_sending_locations_to_chat(self._dc_context, self.id)

    def is_archived(self):
        """return True if this chat is archived.
        :returns: True if archived.
        """
        return lib.dc_chat_get_archived(self._dc_chat)

    def enable_sending_locations(self, seconds):
        """enable sending locations for this chat.

        all subsequent messages will carry a location with them.
        """
        lib.dc_send_locations_to_chat(self._dc_context, self.id, seconds)

    def get_locations(self, contact=None, timestamp_from=None, timestamp_to=None):
        """return list of locations for the given contact in the given timespan.

        :param contact: the contact for which locations shall be returned.
        :param timespan_from: a datetime object or None (indicating "since beginning")
        :param timespan_to: a datetime object or None (indicating up till now)
        :returns: list of :class:`deltachat.chat.Location` objects.
        """
        if timestamp_from is None:
            time_from = 0
        else:
            time_from = calendar.timegm(timestamp_from.utctimetuple())
        if timestamp_to is None:
            time_to = 0
        else:
            time_to = calendar.timegm(timestamp_to.utctimetuple())

        if contact is None:
            contact_id = 0
        else:
            contact_id = contact.id

        dc_array = lib.dc_get_locations(self._dc_context, self.id, contact_id, time_from, time_to)
        return [
            Location(
                latitude=lib.dc_array_get_latitude(dc_array, i),
                longitude=lib.dc_array_get_longitude(dc_array, i),
                accuracy=lib.dc_array_get_accuracy(dc_array, i),
                timestamp=datetime.utcfromtimestamp(lib.dc_array_get_timestamp(dc_array, i)))
            for i in range(lib.dc_array_get_cnt(dc_array))
        ]


class Location:
    def __init__(self, latitude, longitude, accuracy, timestamp):
        assert isinstance(timestamp, datetime)
        self.latitude = latitude
        self.longitude = longitude
        self.accuracy = accuracy
        self.timestamp = timestamp

    def __eq__(self, other):
        return self.__dict__ == other.__dict__

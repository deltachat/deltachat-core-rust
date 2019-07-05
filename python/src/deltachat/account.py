""" Account class implementation. """

from __future__ import print_function
import threading
import os
import re
import time
from array import array
try:
    from queue import Queue
except ImportError:
    from Queue import Queue

import deltachat
from . import const
from .capi import ffi, lib
from .cutil import as_dc_charpointer, from_dc_charpointer, iter_array
from .chatting import Contact, Chat, Message


class Account(object):
    """ Each account is tied to a sqlite database file which is fully managed
    by the underlying deltachat c-library.  All public Account methods are
    meant to be memory-safe and return memory-safe objects.
    """
    def __init__(self, db_path, logid=None):
        """ initialize account object.

        :param db_path: a path to the account database. The database
                        will be created if it doesn't exist.
        :param logid: an optional logging prefix that should be used with
                      the default internal logging.
        """
        self._dc_context = ffi.gc(
            lib.dc_context_new(lib.py_dc_callback, ffi.NULL, ffi.NULL),
            _destroy_dc_context,
        )
        if hasattr(db_path, "encode"):
            db_path = db_path.encode("utf8")
        if not lib.dc_open(self._dc_context, db_path, ffi.NULL):
            raise ValueError("Could not dc_open: {}".format(db_path))
        self._evlogger = EventLogger(self._dc_context, logid)
        deltachat.set_context_callback(self._dc_context, self._process_event)
        self._threads = IOThreads(self._dc_context)
        self._configkeys = self.get_config("sys.config_keys").split()
        self._imex_completed = threading.Event()

    def _check_config_key(self, name):
        if name not in self._configkeys:
            raise KeyError("{!r} not a valid config key, existing keys: {!r}".format(
                           name, self._configkeys))

    def get_info(self):
        """ return dictionary of built config parameters. """
        lines = from_dc_charpointer(lib.dc_get_info(self._dc_context))
        d = {}
        for line in lines.split("\n"):
            if not line.strip():
                continue
            key, value = line.split("=", 1)
            d[key.lower()] = value
        return d

    def set_config(self, name, value):
        """ set configuration values.

        :param name: config key name (unicode)
        :param value: value to set (unicode)
        :returns: None
        """
        self._check_config_key(name)
        name = name.encode("utf8")
        value = value.encode("utf8")
        if name == b"addr" and self.is_configured():
            raise ValueError("can not change 'addr' after account is configured.")
        lib.dc_set_config(self._dc_context, name, value)

    def get_config(self, name):
        """ return unicode string value.

        :param name: configuration key to lookup (eg "addr" or "mail_pw")
        :returns: unicode value
        :raises: KeyError if no config value was found.
        """
        if name != "sys.config_keys":
            self._check_config_key(name)
        name = name.encode("utf8")
        res = lib.dc_get_config(self._dc_context, name)
        assert res != ffi.NULL, "config value not found for: {!r}".format(name)
        return from_dc_charpointer(res)

    def configure(self, **kwargs):
        """ set config values and configure this account.

        :param kwargs: name=value config settings for this account.
                       values need to be unicode.
        :returns: None
        """
        for name, value in kwargs.items():
            self.set_config(name, value)
        lib.dc_configure(self._dc_context)

    def is_configured(self):
        """ determine if the account is configured already; an initial connection
        to SMTP/IMAP has been verified.

        :returns: True if account is configured.
        """
        return lib.dc_is_configured(self._dc_context)

    def check_is_configured(self):
        """ Raise ValueError if this account is not configured. """
        if not self.is_configured():
            raise ValueError("need to configure first")

    def get_infostring(self):
        """ return info of the configured account. """
        self.check_is_configured()
        return from_dc_charpointer(lib.dc_get_info(self._dc_context))

    def get_blobdir(self):
        """ return the directory for files.

        All sent files are copied to this directory if necessary.
        Place files there directly to avoid copying.
        """
        return from_dc_charpointer(lib.dc_get_blobdir(self._dc_context))

    def get_self_contact(self):
        """ return this account's identity as a :class:`deltachat.chatting.Contact`.

        :returns: :class:`deltachat.chatting.Contact`
        """
        self.check_is_configured()
        return Contact(self._dc_context, const.DC_CONTACT_ID_SELF)

    def create_message(self, view_type):
        """ create a new non persistent message.

        :param view_type: a string specifying "text", "video",
                          "image", "audio" or "file".
        :returns: :class:`deltachat.message.Message` instance.
        """
        return Message.new(self._dc_context, view_type)

    def create_contact(self, email, name=None):
        """ create a (new) Contact. If there already is a Contact
        with that e-mail address, it is unblocked and its name is
        updated.

        :param email: email-address (text type)
        :param name: display name for this contact (optional)
        :returns: :class:`deltachat.chatting.Contact` instance.
        """
        name = as_dc_charpointer(name)
        email = as_dc_charpointer(email)
        contact_id = lib.dc_create_contact(self._dc_context, name, email)
        assert contact_id > const.DC_CHAT_ID_LAST_SPECIAL
        return Contact(self._dc_context, contact_id)

    def get_contacts(self, query=None, with_self=False, only_verified=False):
        """ get a (filtered) list of contacts.

        :param query: if a string is specified, only return contacts
                      whose name or e-mail matches query.
        :param only_verified: if true only return verified contacts.
        :param with_self: if true the self-contact is also returned.
        :returns: list of :class:`deltachat.message.Message` objects.
        """
        flags = 0
        query = as_dc_charpointer(query)
        if only_verified:
            flags |= const.DC_GCL_VERIFIED_ONLY
        if with_self:
            flags |= const.DC_GCL_ADD_SELF
        dc_array = ffi.gc(
            lib.dc_get_contacts(self._dc_context, flags, query),
            lib.dc_array_unref
        )
        return list(iter_array(dc_array, lambda x: Contact(self._dc_context, x)))

    def create_chat_by_contact(self, contact):
        """ create or get an existing 1:1 chat object for the specified contact or contact id.

        :param contact: chat_id (int) or contact object.
        :returns: a :class:`deltachat.chatting.Chat` object.
        """
        if hasattr(contact, "id"):
            if contact._dc_context != self._dc_context:
                raise ValueError("Contact belongs to a different Account")
            contact_id = contact.id
        else:
            assert isinstance(contact, int)
            contact_id = contact
        chat_id = lib.dc_create_chat_by_contact_id(self._dc_context, contact_id)
        return Chat(self._dc_context, chat_id)

    def create_chat_by_message(self, message):
        """ create or get an existing chat object for the
        the specified message.

        :param message: messsage id or message instance.
        :returns: a :class:`deltachat.chatting.Chat` object.
        """
        if hasattr(message, "id"):
            if self._dc_context != message._dc_context:
                raise ValueError("Message belongs to a different Account")
            msg_id = message.id
        else:
            assert isinstance(message, int)
            msg_id = message
        chat_id = lib.dc_create_chat_by_msg_id(self._dc_context, msg_id)
        return Chat(self._dc_context, chat_id)

    def create_group_chat(self, name, verified=False):
        """ create a new group chat object.

        Chats are unpromoted until the first message is sent.

        :param verified: if true only verified contacts can be added.
        :returns: a :class:`deltachat.chatting.Chat` object.
        """
        bytes_name = name.encode("utf8")
        chat_id = lib.dc_create_group_chat(self._dc_context, verified, bytes_name)
        return Chat(self._dc_context, chat_id)

    def get_chats(self):
        """ return list of chats.

        :returns: a list of :class:`deltachat.chatting.Chat` objects.
        """
        dc_chatlist = ffi.gc(
            lib.dc_get_chatlist(self._dc_context, 0, ffi.NULL, 0),
            lib.dc_chatlist_unref
        )

        assert dc_chatlist != ffi.NULL
        chatlist = []
        for i in range(0, lib.dc_chatlist_get_cnt(dc_chatlist)):
            chat_id = lib.dc_chatlist_get_chat_id(dc_chatlist, i)
            chatlist.append(Chat(self._dc_context, chat_id))
        return chatlist

    def get_deaddrop_chat(self):
        return Chat(self._dc_context, const.DC_CHAT_ID_DEADDROP)

    def get_message_by_id(self, msg_id):
        """ return Message instance. """
        return Message.from_db(self._dc_context, msg_id)

    def mark_seen_messages(self, messages):
        """ mark the given set of messages as seen.

        :param messages: a list of message ids or Message instances.
        """
        arr = array("i")
        for msg in messages:
            msg = getattr(msg, "id", msg)
            arr.append(msg)
        msg_ids = ffi.cast("uint32_t*", ffi.from_buffer(arr))
        lib.dc_markseen_msgs(self._dc_context, msg_ids, len(messages))

    def forward_messages(self, messages, chat):
        """ Forward list of messages to a chat.

        :param messages: list of :class:`deltachat.message.Message` object.
        :param chat: :class:`deltachat.chatting.Chat` object.
        :returns: None
        """
        msg_ids = [msg.id for msg in messages]
        lib.dc_forward_msgs(self._dc_context, msg_ids, len(msg_ids), chat.id)

    def delete_messages(self, messages):
        """ delete messages (local and remote).

        :param messages: list of :class:`deltachat.message.Message` object.
        :returns: None
        """
        msg_ids = [msg.id for msg in messages]
        lib.dc_delete_msgs(self._dc_context, msg_ids, len(msg_ids))

    def export_to_dir(self, backupdir):
        """return after all delta chat state is exported to a new file in
        the specified directory.
        """
        snap_files = os.listdir(backupdir)
        self._imex_completed.clear()
        lib.dc_imex(self._dc_context, 11, as_dc_charpointer(backupdir), ffi.NULL)
        if not self._threads.is_started():
            lib.dc_perform_imap_jobs(self._dc_context)
        self._imex_completed.wait()
        for x in os.listdir(backupdir):
            if x not in snap_files:
                return os.path.join(backupdir, x)

    def import_from_file(self, path):
        """import delta chat state from the specified backup file.

        The account must be in unconfigured state for import to attempted.
        """
        assert not self.is_configured(), "cannot import into configured account"
        self._imex_completed.clear()
        lib.dc_imex(self._dc_context, 12, as_dc_charpointer(path), ffi.NULL)
        if not self._threads.is_started():
            lib.dc_perform_imap_jobs(self._dc_context)
        self._imex_completed.wait()

    def initiate_key_transfer(self):
        """return setup code after a Autocrypt setup message
        has been successfully sent to our own e-mail address ("self-sent message").
        If sending out was unsuccessful, a RuntimeError is raised.
        """
        self.check_is_configured()
        if not self._threads.is_started():
            raise RuntimeError("threads not running, can not send out")
        res = lib.dc_initiate_key_transfer(self._dc_context)
        if res == ffi.NULL:
            raise RuntimeError("could not send out autocrypt setup message")
        return from_dc_charpointer(res)

    def start_threads(self):
        """ start IMAP/SMTP threads (and configure account if it hasn't happened).

        :raises: ValueError if 'addr' or 'mail_pw' are not configured.
        :returns: None
        """
        if not self.is_configured():
            self.configure()
        self._threads.start()

    def stop_threads(self, wait=True):
        """ stop IMAP/SMTP threads. """
        lib.dc_stop_ongoing_process(self._dc_context)
        self._threads.stop(wait=wait)

    def _process_event(self, ctx, evt_name, data1, data2):
        assert ctx == self._dc_context
        self._evlogger(evt_name, data1, data2)
        method = getattr(self, "on_" + evt_name.lower(), None)
        if method is not None:
            method(data1, data2)
        return 0

    def on_dc_event_imex_progress(self, data1, data2):
        if data1 == 1000:
            self._imex_completed.set()


class IOThreads:
    def __init__(self, dc_context):
        self._dc_context = dc_context
        self._thread_quitflag = False
        self._name2thread = {}

    def is_started(self):
        return len(self._name2thread) > 0

    def start(self, imap=True, smtp=True):
        assert not self.is_started()
        if imap:
            self._start_one_thread("imap", self.imap_thread_run)
        if smtp:
            self._start_one_thread("smtp", self.smtp_thread_run)

    def _start_one_thread(self, name, func):
        self._name2thread[name] = t = threading.Thread(target=func, name=name)
        t.setDaemon(1)
        t.start()

    def stop(self, wait=False):
        self._thread_quitflag = True
        lib.dc_interrupt_imap_idle(self._dc_context)
        lib.dc_interrupt_smtp_idle(self._dc_context)
        if wait:
            for name, thread in self._name2thread.items():
                thread.join()

    def imap_thread_run(self):
        while not self._thread_quitflag:
            lib.dc_perform_imap_jobs(self._dc_context)
            lib.dc_perform_imap_fetch(self._dc_context)
            lib.dc_perform_imap_idle(self._dc_context)

    def smtp_thread_run(self):
        while not self._thread_quitflag:
            lib.dc_perform_smtp_jobs(self._dc_context)
            lib.dc_perform_smtp_idle(self._dc_context)


class EventLogger:
    _loglock = threading.RLock()

    def __init__(self, dc_context, logid=None, debug=True):
        self._dc_context = dc_context
        self._event_queue = Queue()
        self._debug = debug
        if logid is None:
            logid = str(self._dc_context).strip(">").split()[-1]
        self.logid = logid
        self._timeout = None
        self.init_time = time.time()

    def __call__(self, evt_name, data1, data2):
        self._log_event(evt_name, data1, data2)
        self._event_queue.put((evt_name, data1, data2))

    def set_timeout(self, timeout):
        self._timeout = timeout

    def get(self, timeout=None, check_error=True):
        timeout = timeout or self._timeout
        ev = self._event_queue.get(timeout=timeout)
        if check_error and ev[0] == "DC_EVENT_ERROR":
            raise ValueError("{}({!r},{!r})".format(*ev))
        return ev

    def get_matching(self, event_name_regex):
        self._log("-- waiting for event with regex: {} --".format(event_name_regex))
        rex = re.compile("(?:{}).*".format(event_name_regex))
        while 1:
            ev = self.get()
            if rex.match(ev[0]):
                return ev

    def get_info_matching(self, regex):
        rex = re.compile("(?:{}).*".format(regex))
        while 1:
            ev = self.get_matching("DC_EVENT_INFO")
            if rex.match(ev[2]):
                return ev

    def _log_event(self, evt_name, data1, data2):
        # don't show events that are anyway empty impls now
        if evt_name == "DC_EVENT_GET_STRING":
            return
        if self._debug:
            evpart = "{}({!r},{!r})".format(evt_name, data1, data2)
            self._log(evpart)

    def _log(self, msg):
        t = threading.currentThread()
        tname = getattr(t, "name", t)
        if tname == "MainThread":
            tname = "MAIN"
        with self._loglock:
            print("{:2.2f} [{}-{}] {}".format(time.time() - self.init_time, tname, self.logid, msg))


def _destroy_dc_context(dc_context, dc_context_unref=lib.dc_context_unref):
    # destructor for dc_context
    dc_context_unref(dc_context)
    try:
        deltachat.clear_context_callback(dc_context)
    except (TypeError, AttributeError):
        # we are deep into Python Interpreter shutdown,
        # so no need to clear the callback context mapping.
        pass

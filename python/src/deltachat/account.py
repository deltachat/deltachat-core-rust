""" Account class implementation. """

from __future__ import print_function
import atexit
import threading
import os
import re
import time
from array import array
try:
    from queue import Queue, Empty
except ImportError:
    from Queue import Queue, Empty

import deltachat
from . import const
from .capi import ffi, lib
from .cutil import as_dc_charpointer, from_dc_charpointer, iter_array, DCLot
from .chat import Chat
from .message import Message
from .contact import Contact


class Account(object):
    """ Each account is tied to a sqlite database file which is fully managed
    by the underlying deltachat core library.  All public Account methods are
    meant to be memory-safe and return memory-safe objects.
    """
    def __init__(self, db_path, logid=None, eventlogging=True, os_name=None, debug=True):
        """ initialize account object.

        :param db_path: a path to the account database. The database
                        will be created if it doesn't exist.
        :param logid: an optional logging prefix that should be used with
                      the default internal logging.
        :param eventlogging: if False no eventlogging and no context callback will be configured
        :param os_name: this will be put to the X-Mailer header in outgoing messages
        :param debug: turn on debug logging for events.
        """
        self._dc_context = ffi.gc(
            lib.dc_context_new(lib.py_dc_callback, ffi.NULL, as_dc_charpointer(os_name)),
            _destroy_dc_context,
        )
        if eventlogging:
            self._evlogger = EventLogger(self._dc_context, logid, debug)
            deltachat.set_context_callback(self._dc_context, self._process_event)
            self._threads = IOThreads(self._dc_context, self._evlogger._log_event)
        else:
            self._threads = IOThreads(self._dc_context)

        if hasattr(db_path, "encode"):
            db_path = db_path.encode("utf8")
        if not lib.dc_open(self._dc_context, db_path, ffi.NULL):
            raise ValueError("Could not dc_open: {}".format(db_path))
        self._configkeys = self.get_config("sys.config_keys").split()
        self._imex_events = Queue()
        atexit.register(self.shutdown)

    # def __del__(self):
    #    self.shutdown()

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

    def set_stock_translation(self, id, string):
        """ set stock translation string.

        :param id: id of stock string (const.DC_STR_*)
        :param value: string to set as new transalation
        :returns: None
        """
        string = string.encode("utf8")
        res = lib.dc_set_stock_translation(self._dc_context, id, string)
        if res == 0:
            raise ValueError("could not set translation string")

    def set_config(self, name, value):
        """ set configuration values.

        :param name: config key name (unicode)
        :param value: value to set (unicode)
        :returns: None
        """
        self._check_config_key(name)
        name = name.encode("utf8")
        if name == b"addr" and self.is_configured():
            raise ValueError("can not change 'addr' after account is configured.")
        if value is not None:
            value = value.encode("utf8")
        else:
            value = ffi.NULL
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

    def set_avatar(self, img_path):
        """Set self avatar.

        :raises ValueError: if profile image could not be set
        :returns: None
        """
        if img_path is None:
            self.set_config("selfavatar", None)
        else:
            assert os.path.exists(img_path), img_path
            self.set_config("selfavatar", img_path)

    def check_is_configured(self):
        """ Raise ValueError if this account is not configured. """
        if not self.is_configured():
            raise ValueError("need to configure first")

    def empty_server_folders(self, inbox=False, mvbox=False):
        """ empty server folders. """
        flags = 0
        if inbox:
            flags |= const.DC_EMPTY_INBOX
        if mvbox:
            flags |= const.DC_EMPTY_MVBOX
        if not flags:
            raise ValueError("no flags set")
        lib.dc_empty_server(self._dc_context, flags)

    def get_infostring(self):
        """ return info of the configured account. """
        self.check_is_configured()
        return from_dc_charpointer(lib.dc_get_info(self._dc_context))

    def get_latest_backupfile(self, backupdir):
        """ return the latest backup file in a given directory.
        """
        res = lib.dc_imex_has_backup(self._dc_context, as_dc_charpointer(backupdir))
        if res == ffi.NULL:
            return None
        return from_dc_charpointer(res)

    def get_blobdir(self):
        """ return the directory for files.

        All sent files are copied to this directory if necessary.
        Place files there directly to avoid copying.
        """
        return from_dc_charpointer(lib.dc_get_blobdir(self._dc_context))

    def get_self_contact(self):
        """ return this account's identity as a :class:`deltachat.contact.Contact`.

        :returns: :class:`deltachat.contact.Contact`
        """
        self.check_is_configured()
        return Contact(self._dc_context, const.DC_CONTACT_ID_SELF)

    def create_contact(self, email, name=None):
        """ create a (new) Contact. If there already is a Contact
        with that e-mail address, it is unblocked and its name is
        updated.

        :param email: email-address (text type)
        :param name: display name for this contact (optional)
        :returns: :class:`deltachat.contact.Contact` instance.
        """
        name = as_dc_charpointer(name)
        email = as_dc_charpointer(email)
        contact_id = lib.dc_create_contact(self._dc_context, name, email)
        assert contact_id > const.DC_CHAT_ID_LAST_SPECIAL
        return Contact(self._dc_context, contact_id)

    def delete_contact(self, contact):
        """ delete a Contact.

        :param contact: contact object obtained
        :returns: True if deletion succeeded (contact was deleted)
        """
        contact_id = contact.id
        assert contact._dc_context == self._dc_context
        assert contact_id > const.DC_CHAT_ID_LAST_SPECIAL
        return bool(lib.dc_delete_contact(self._dc_context, contact_id))

    def get_contacts(self, query=None, with_self=False, only_verified=False):
        """ get a (filtered) list of contacts.

        :param query: if a string is specified, only return contacts
                      whose name or e-mail matches query.
        :param only_verified: if true only return verified contacts.
        :param with_self: if true the self-contact is also returned.
        :returns: list of :class:`deltachat.contact.Contact` objects.
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
        :returns: a :class:`deltachat.chat.Chat` object.
        """
        if hasattr(contact, "id"):
            if contact._dc_context != self._dc_context:
                raise ValueError("Contact belongs to a different Account")
            contact_id = contact.id
        else:
            assert isinstance(contact, int)
            contact_id = contact
        chat_id = lib.dc_create_chat_by_contact_id(self._dc_context, contact_id)
        return Chat(self, chat_id)

    def create_chat_by_message(self, message):
        """ create or get an existing chat object for the
        the specified message.

        :param message: messsage id or message instance.
        :returns: a :class:`deltachat.chat.Chat` object.
        """
        if hasattr(message, "id"):
            if self._dc_context != message._dc_context:
                raise ValueError("Message belongs to a different Account")
            msg_id = message.id
        else:
            assert isinstance(message, int)
            msg_id = message
        chat_id = lib.dc_create_chat_by_msg_id(self._dc_context, msg_id)
        return Chat(self, chat_id)

    def create_group_chat(self, name, verified=False):
        """ create a new group chat object.

        Chats are unpromoted until the first message is sent.

        :param verified: if true only verified contacts can be added.
        :returns: a :class:`deltachat.chat.Chat` object.
        """
        bytes_name = name.encode("utf8")
        chat_id = lib.dc_create_group_chat(self._dc_context, int(verified), bytes_name)
        return Chat(self, chat_id)

    def get_chats(self):
        """ return list of chats.

        :returns: a list of :class:`deltachat.chat.Chat` objects.
        """
        dc_chatlist = ffi.gc(
            lib.dc_get_chatlist(self._dc_context, 0, ffi.NULL, 0),
            lib.dc_chatlist_unref
        )

        assert dc_chatlist != ffi.NULL
        chatlist = []
        for i in range(0, lib.dc_chatlist_get_cnt(dc_chatlist)):
            chat_id = lib.dc_chatlist_get_chat_id(dc_chatlist, i)
            chatlist.append(Chat(self, chat_id))
        return chatlist

    def get_deaddrop_chat(self):
        return Chat(self, const.DC_CHAT_ID_DEADDROP)

    def get_message_by_id(self, msg_id):
        """ return Message instance.
        :param msg_id: integer id of this message.
        :returns: :class:`deltachat.message.Message` instance.
        """
        return Message.from_db(self, msg_id)

    def get_chat_by_id(self, chat_id):
        """ return Chat instance.
        :param chat_id: integer id of this chat.
        :returns: :class:`deltachat.chat.Chat` instance.
        :raises: ValueError if chat does not exist.
        """
        res = lib.dc_get_chat(self._dc_context, chat_id)
        if res == ffi.NULL:
            raise ValueError("cannot get chat with id={}".format(chat_id))
        lib.dc_chat_unref(res)
        return Chat(self, chat_id)

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
        :param chat: :class:`deltachat.chat.Chat` object.
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

    def export_self_keys(self, path):
        """ export public and private keys to the specified directory. """
        return self._export(path, imex_cmd=1)

    def export_all(self, path):
        """return new file containing a backup of all database state
        (chats, contacts, keys, media, ...). The file is created in the
        the `path` directory.
        """
        export_files = self._export(path, 11)
        if len(export_files) != 1:
            raise RuntimeError("found more than one new file")
        return export_files[0]

    def _imex_events_clear(self):
        try:
            while True:
                self._imex_events.get_nowait()
        except Empty:
            pass

    def _export(self, path, imex_cmd):
        self._imex_events_clear()
        lib.dc_imex(self._dc_context, imex_cmd, as_dc_charpointer(path), ffi.NULL)
        if not self._threads.is_started():
            lib.dc_perform_imap_jobs(self._dc_context)
        files_written = []
        while True:
            ev = self._imex_events.get()
            if isinstance(ev, str):
                files_written.append(ev)
            elif isinstance(ev, bool):
                if not ev:
                    raise ValueError("export failed, exp-files: {}".format(files_written))
                return files_written

    def import_self_keys(self, path):
        """ Import private keys found in the `path` directory.
        The last imported key is made the default keys unless its name
        contains the string legacy. Public keys are not imported.
        """
        self._import(path, imex_cmd=2)

    def import_all(self, path):
        """import delta chat state from the specified backup `path` (a file).

        The account must be in unconfigured state for import to attempted.
        """
        assert not self.is_configured(), "cannot import into configured account"
        self._import(path, imex_cmd=12)

    def _import(self, path, imex_cmd):
        self._imex_events_clear()
        lib.dc_imex(self._dc_context, imex_cmd, as_dc_charpointer(path), ffi.NULL)
        if not self._threads.is_started():
            lib.dc_perform_imap_jobs(self._dc_context)
        if not self._imex_events.get():
            raise ValueError("import from path '{}' failed".format(path))

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

    def get_setup_contact_qr(self):
        """ get/create Setup-Contact QR Code as ascii-string.

        this string needs to be transferred to another DC account
        in a second channel (typically used by mobiles with QRcode-show + scan UX)
        where qr_setup_contact(qr) is called.
        """
        res = lib.dc_get_securejoin_qr(self._dc_context, 0)
        return from_dc_charpointer(res)

    def check_qr(self, qr):
        """ check qr code and return :class:`ScannedQRCode` instance representing the result"""
        res = ffi.gc(
            lib.dc_check_qr(self._dc_context, as_dc_charpointer(qr)),
            lib.dc_lot_unref
        )
        lot = DCLot(res)
        if lot.state() == const.DC_QR_ERROR:
            raise ValueError("invalid or unknown QR code: {}".format(lot.text1()))
        return ScannedQRCode(lot)

    def qr_setup_contact(self, qr):
        """ setup contact and return a Chat after contact is established.

        Note that this function may block for a long time as messages are exchanged
        with the emitter of the QR code.  On success a :class:`deltachat.chat.Chat` instance
        is returned.
        :param qr: valid "setup contact" QR code (all other QR codes will result in an exception)
        """
        assert self.check_qr(qr).is_ask_verifycontact()
        chat_id = lib.dc_join_securejoin(self._dc_context, as_dc_charpointer(qr))
        if chat_id == 0:
            raise ValueError("could not setup secure contact")
        return Chat(self, chat_id)

    def qr_join_chat(self, qr):
        """ join a chat group through a QR code.

        Note that this function may block for a long time as messages are exchanged
        with the emitter of the QR code.  On success a :class:`deltachat.chat.Chat` instance
        is returned which is the chat that we just joined.

        :param qr: valid "join-group" QR code (all other QR codes will result in an exception)
        """
        assert self.check_qr(qr).is_ask_verifygroup()
        chat_id = lib.dc_join_securejoin(self._dc_context, as_dc_charpointer(qr))
        if chat_id == 0:
            raise ValueError("could not join group")
        return Chat(self, chat_id)

    def stop_ongoing(self):
        lib.dc_stop_ongoing_process(self._dc_context)

    #
    # meta API for start/stop and event based processing
    #

    def wait_next_incoming_message(self):
        """ wait for and return next incoming message. """
        ev = self._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        return self.get_message_by_id(ev[2])

    def start_threads(self, mvbox=False, sentbox=False):
        """ start IMAP/SMTP threads (and configure account if it hasn't happened).

        :raises: ValueError if 'addr' or 'mail_pw' are not configured.
        :returns: None
        """
        if not self.is_configured():
            self.configure()
        self._threads.start(mvbox=mvbox, sentbox=sentbox)

    def stop_threads(self, wait=True):
        """ stop IMAP/SMTP threads. """
        if self._threads.is_started():
            self.stop_ongoing()
            self._threads.stop(wait=wait)

    def shutdown(self, wait=True):
        """ stop threads and close and remove underlying dc_context and callbacks. """
        if hasattr(self, "_dc_context") and hasattr(self, "_threads"):
            # print("SHUTDOWN", self)
            self.stop_threads(wait=False)
            lib.dc_close(self._dc_context)
            self.stop_threads(wait=wait)  # to wait for threads
            deltachat.clear_context_callback(self._dc_context)
            del self._dc_context
            atexit.unregister(self.shutdown)

    def _process_event(self, ctx, evt_name, data1, data2):
        assert ctx == self._dc_context
        if hasattr(self, "_evlogger"):
            self._evlogger(evt_name, data1, data2)
            method = getattr(self, "on_" + evt_name.lower(), None)
            if method is not None:
                method(data1, data2)
        return 0

    def on_dc_event_imex_progress(self, data1, data2):
        if data1 == 1000:
            self._imex_events.put(True)
        elif data1 == 0:
            self._imex_events.put(False)

    def on_dc_event_imex_file_written(self, data1, data2):
        self._imex_events.put(data1)

    def set_location(self, latitude=0.0, longitude=0.0, accuracy=0.0):
        """set a new location. It effects all chats where we currently
        have enabled location streaming.

        :param latitude: float (use 0.0 if not known)
        :param longitude: float (use 0.0 if not known)
        :param accuracy: float (use 0.0 if not known)
        :raises: ValueError if no chat is currently streaming locations
        :returns: None
        """
        dc_res = lib.dc_set_location(self._dc_context, latitude, longitude, accuracy)
        if dc_res == 0:
            raise ValueError("no chat is streaming locations")


class IOThreads:
    def __init__(self, dc_context, log_event=lambda *args: None):
        self._dc_context = dc_context
        self._thread_quitflag = False
        self._name2thread = {}
        self._log_event = log_event

    def is_started(self):
        return len(self._name2thread) > 0

    def start(self, imap=True, smtp=True, mvbox=False, sentbox=False):
        assert not self.is_started()
        if imap:
            self._start_one_thread("inbox", self.imap_thread_run)
        if mvbox:
            self._start_one_thread("mvbox", self.mvbox_thread_run)
        if sentbox:
            self._start_one_thread("sentbox", self.sentbox_thread_run)
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
        lib.dc_interrupt_mvbox_idle(self._dc_context)
        lib.dc_interrupt_sentbox_idle(self._dc_context)
        if wait:
            for name, thread in self._name2thread.items():
                thread.join()

    def imap_thread_run(self):
        self._log_event("py-bindings-info", 0, "INBOX THREAD START")
        while not self._thread_quitflag:
            lib.dc_perform_imap_jobs(self._dc_context)
            if not self._thread_quitflag:
                lib.dc_perform_imap_fetch(self._dc_context)
            if not self._thread_quitflag:
                lib.dc_perform_imap_idle(self._dc_context)
        self._log_event("py-bindings-info", 0, "INBOX THREAD FINISHED")

    def mvbox_thread_run(self):
        self._log_event("py-bindings-info", 0, "MVBOX THREAD START")
        while not self._thread_quitflag:
            lib.dc_perform_mvbox_jobs(self._dc_context)
            lib.dc_perform_mvbox_fetch(self._dc_context)
            lib.dc_perform_mvbox_idle(self._dc_context)
        self._log_event("py-bindings-info", 0, "MVBOX THREAD FINISHED")

    def sentbox_thread_run(self):
        self._log_event("py-bindings-info", 0, "SENTBOX THREAD START")
        while not self._thread_quitflag:
            lib.dc_perform_sentbox_jobs(self._dc_context)
            lib.dc_perform_sentbox_fetch(self._dc_context)
            lib.dc_perform_sentbox_idle(self._dc_context)
        self._log_event("py-bindings-info", 0, "SENTBOX THREAD FINISHED")

    def smtp_thread_run(self):
        self._log_event("py-bindings-info", 0, "SMTP THREAD START")
        while not self._thread_quitflag:
            lib.dc_perform_smtp_jobs(self._dc_context)
            lib.dc_perform_smtp_idle(self._dc_context)
        self._log_event("py-bindings-info", 0, "SMTP THREAD FINISHED")


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

    def consume_events(self, check_error=True):
        while not self._event_queue.empty():
            self.get()

    def get(self, timeout=None, check_error=True):
        timeout = timeout or self._timeout
        ev = self._event_queue.get(timeout=timeout)
        if check_error and ev[0] == "DC_EVENT_ERROR":
            raise ValueError("{}({!r},{!r})".format(*ev))
        return ev

    def ensure_event_not_queued(self, event_name_regex):
        __tracebackhide__ = True
        rex = re.compile("(?:{}).*".format(event_name_regex))
        while 1:
            try:
                ev = self._event_queue.get(False)
            except Empty:
                break
            else:
                assert not rex.match(ev[0]), "event found {}".format(ev)

    def get_matching(self, event_name_regex, check_error=True, timeout=None):
        self._log("-- waiting for event with regex: {} --".format(event_name_regex))
        rex = re.compile("(?:{}).*".format(event_name_regex))
        while 1:
            ev = self.get(timeout=timeout, check_error=check_error)
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


class ScannedQRCode:
    def __init__(self, dc_lot):
        self._dc_lot = dc_lot

    def is_ask_verifycontact(self):
        return self._dc_lot.state() == const.DC_QR_ASK_VERIFYCONTACT

    def is_ask_verifygroup(self):
        return self._dc_lot.state() == const.DC_QR_ASK_VERIFYGROUP

    @property
    def contact_id(self):
        return self._dc_lot.id()

import threading
import time
import re
from queue import Queue, Empty

import deltachat
from .hookspec import account_hookimpl
from contextlib import contextmanager
from .capi import ffi, lib
from .message import map_system_message


class FFIEvent:
    def __init__(self, name, data1, data2):
        self.name = name
        self.data1 = data1
        self.data2 = data2

    def __str__(self):
        return "{name} data1={data1} data2={data2}".format(**self.__dict__)


class FFIEventLogger:
    """ If you register an instance of this logger with an Account
    you'll get all ffi-events printed.
    """
    # to prevent garbled logging
    _loglock = threading.RLock()

    def __init__(self, account, logid):
        """
        :param logid: an optional logging prefix that should be used with
                      the default internal logging.
        """
        self.account = account
        self.logid = logid
        self.init_time = time.time()

    @account_hookimpl
    def ac_process_ffi_event(self, ffi_event):
        self.account.log(str(ffi_event))

    @account_hookimpl
    def ac_log_line(self, message):
        t = threading.currentThread()
        tname = getattr(t, "name", t)
        if tname == "MainThread":
            tname = "MAIN"
        elapsed = time.time() - self.init_time
        locname = tname
        if self.logid:
            locname += "-" + self.logid
        s = "{:2.2f} [{}] {}".format(elapsed, locname, message)
        with self._loglock:
            print(s, flush=True)


class FFIEventTracker:
    def __init__(self, account, timeout=None):
        self.account = account
        self._timeout = timeout
        self._event_queue = Queue()

    @account_hookimpl
    def ac_process_ffi_event(self, ffi_event):
        self._event_queue.put(ffi_event)

    def set_timeout(self, timeout):
        self._timeout = timeout

    def consume_events(self, check_error=True):
        while not self._event_queue.empty():
            self.get(check_error=check_error)

    def get(self, timeout=None, check_error=True):
        timeout = timeout if timeout is not None else self._timeout
        ev = self._event_queue.get(timeout=timeout)
        if check_error and ev.name == "DC_EVENT_ERROR":
            raise ValueError(str(ev))
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
                assert not rex.match(ev.name), "event found {}".format(ev)

    def wait_securejoin_inviter_progress(self, target):
        while 1:
            event = self.get_matching("DC_EVENT_SECUREJOIN_INVITER_PROGRESS")
            if event.data2 >= target:
                print("** SECUREJOINT-INVITER PROGRESS {}".format(target), self.account)
                break

    def get_matching(self, event_name_regex, check_error=True, timeout=None):
        self.account.log("-- waiting for event with regex: {} --".format(event_name_regex))
        rex = re.compile("(?:{}).*".format(event_name_regex))
        while 1:
            ev = self.get(timeout=timeout, check_error=check_error)
            if rex.match(ev.name):
                return ev

    def get_info_matching(self, regex):
        rex = re.compile("(?:{}).*".format(regex))
        while 1:
            ev = self.get_matching("DC_EVENT_INFO")
            if rex.match(ev.data2):
                return ev

    def wait_next_incoming_message(self):
        """ wait for and return next incoming message. """
        ev = self.get_matching("DC_EVENT_INCOMING_MSG")
        return self.account.get_message_by_id(ev.data2)

    def wait_next_messages_changed(self):
        """ wait for and return next message-changed message or None
        if the event contains no msgid"""
        ev = self.get_matching("DC_EVENT_MSGS_CHANGED")
        if ev.data2 > 0:
            return self.account.get_message_by_id(ev.data2)


class EventThread(threading.Thread):
    """ Event Thread for an account.

    With each Account init this callback thread is started.
    """
    def __init__(self, account):
        self.account = account
        self._dc_context = account._dc_context
        self._thread_quitflag = False
        super(EventThread, self).__init__(name="events")
        self.start()

    @contextmanager
    def log_execution(self, message):
        self.account.log(message + " START")
        yield
        self.account.log(message + " FINISHED")

    def stop(self, wait=False):
        self._thread_quitflag = True

        if wait:
            if self == threading.current_thread():
                # we are in the callback thread and thus cannot
                # wait for the thread-loop to finish.
                return
            self.join()

    def run(self):
        """ get and run events until shutdown. """
        with self.log_execution("EVENT THREAD"):
            self._inner_run()

    def _inner_run(self):
        while lib.dc_is_open(self._dc_context) and not self._thread_quitflag:
            event = lib.dc_get_next_event(self._dc_context)
            if event == ffi.NULL:
                break
            evt = lib.dc_event_get_id(event)
            data1 = lib.dc_event_get_data1(event)
            data2 = lib.dc_event_get_data2(event)
            # the following code relates to the deltachat/_build.py's helper
            # function which provides us signature info of an event call
            evt_name = deltachat.get_dc_event_name(evt)
            event_sig_types = lib.dc_get_event_signature_types(evt)
            if data1 and event_sig_types & 1:
                data1 = ffi.string(ffi.gc(ffi.cast('char*', data1), lib.dc_str_unref)).decode("utf8")
            if data2 and event_sig_types & 2:
                data2 = ffi.string(ffi.gc(ffi.cast('char*', data2), lib.dc_str_unref)).decode("utf8")
            try:
                if isinstance(data2, bytes):
                    data2 = data2.decode("utf8")
            except UnicodeDecodeError:
                # XXX ignoring the decode error is not quite correct but for now
                # i don't want to hunt down encoding problems in the c lib
                pass

            lib.dc_event_unref(event)
            ffi_event = FFIEvent(name=evt_name, data1=data1, data2=data2)
            self.account._pm.hook.ac_process_ffi_event(account=self, ffi_event=ffi_event)
            for name, kwargs in self._map_ffi_event(ffi_event):
                # self.account.log("calling hook name={} kwargs={}".format(name, kwargs))
                hook = getattr(self.account._pm.hook, name)
                try:
                    hook(**kwargs)
                except Exception:
                    # don't bother logging this error
                    # if dc_close() was concurrently called
                    # (note: core API starts failing after that)
                    if not self._thread_quitflag:
                        raise

    def _map_ffi_event(self, ffi_event):
        name = ffi_event.name
        account = self.account
        if name == "DC_EVENT_CONFIGURE_PROGRESS":
            data1 = ffi_event.data1
            if data1 == 0 or data1 == 1000:
                success = data1 == 1000
                yield "ac_configure_completed", dict(success=success)
        elif name == "DC_EVENT_INCOMING_MSG":
            msg = account.get_message_by_id(ffi_event.data2)
            yield map_system_message(msg) or ("ac_incoming_message", dict(message=msg))
        elif name == "DC_EVENT_MSGS_CHANGED":
            if ffi_event.data2 != 0:
                msg = account.get_message_by_id(ffi_event.data2)
                if msg.is_outgoing():
                    res = map_system_message(msg)
                    if res and res[0].startswith("ac_member"):
                        yield res
                    yield "ac_outgoing_message", dict(message=msg)
                elif msg.is_in_fresh():
                    yield map_system_message(msg) or ("ac_incoming_message", dict(message=msg))
        elif name == "DC_EVENT_MSG_DELIVERED":
            msg = account.get_message_by_id(ffi_event.data2)
            yield "ac_message_delivered", dict(message=msg)
        elif name == "DC_EVENT_CHAT_MODIFIED":
            chat = account.get_chat_by_id(ffi_event.data1)
            yield "ac_chat_modified", dict(chat=chat)

import threading
import time
import re
import os
from queue import Queue, Empty

import deltachat
from .hookspec import account_hookimpl
from contextlib import contextmanager
from .capi import ffi, lib
from .message import map_system_message
from .cutil import from_optional_dc_charpointer


class FFIEvent:
    def __init__(self, name: str, data1, data2):
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

    def __init__(self, account) -> None:
        self.account = account
        self.logid = self.account.get_config("displayname")
        self.init_time = time.time()

    @account_hookimpl
    def ac_process_ffi_event(self, ffi_event: FFIEvent) -> None:
        self.account.log(str(ffi_event))

    @account_hookimpl
    def ac_log_line(self, message):
        t = threading.current_thread()
        tname = getattr(t, "name", t)
        if tname == "MainThread":
            tname = "MAIN"
        elapsed = time.time() - self.init_time
        locname = tname
        if self.logid:
            locname += "-" + self.logid
        s = "{:2.2f} [{}] {}".format(elapsed, locname, message)

        if os.name == "posix":
            WARN = '\033[93m'
            ERROR = '\033[91m'
            ENDC = '\033[0m'
            if message.startswith("DC_EVENT_WARNING"):
                s = WARN + s + ENDC
            if message.startswith("DC_EVENT_ERROR"):
                s = ERROR + s + ENDC
        with self._loglock:
            print(s, flush=True)


class FFIEventTracker:
    def __init__(self, account, timeout=None):
        self.account = account
        self._timeout = timeout
        self._event_queue = Queue()

    @account_hookimpl
    def ac_process_ffi_event(self, ffi_event: FFIEvent):
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
            raise ValueError("unexpected event: {}".format(ev))
        return ev

    def iter_events(self, timeout=None, check_error=True):
        while 1:
            yield self.get(timeout=timeout, check_error=check_error)

    def get_matching(self, event_name_regex, check_error=True, timeout=None):
        rex = re.compile("(?:{}).*".format(event_name_regex))
        for ev in self.iter_events(timeout=timeout, check_error=check_error):
            if rex.match(ev.name):
                return ev

    def get_info_contains(self, regex: str) -> FFIEvent:
        rex = re.compile(regex)
        while 1:
            ev = self.get_matching("DC_EVENT_INFO")
            if rex.search(ev.data2):
                return ev

    def get_info_regex_groups(self, regex, check_error=True):
        rex = re.compile(regex)
        while 1:
            ev = self.get_matching("DC_EVENT_INFO", check_error=check_error)
            m = rex.match(ev.data2)
            if m is not None:
                return m.groups()

    def wait_for_connectivity(self, connectivity):
        """Wait for the specified connectivity.
        This only works reliably if the connectivity doesn't change
        again too quickly, otherwise we might miss it."""
        while 1:
            if self.account.get_connectivity() == connectivity:
                return
            self.get_matching("DC_EVENT_CONNECTIVITY_CHANGED")

    def wait_for_connectivity_change(self, previous, expected_next):
        """Wait until the connectivity changes to `expected_next`.
        Fails the test if it changes to something else."""
        while 1:
            current = self.account.get_connectivity()
            if current == expected_next:
                return
            elif current != previous:
                raise Exception("Expected connectivity " + str(expected_next) + " but got " + str(current))

            self.get_matching("DC_EVENT_CONNECTIVITY_CHANGED")

    def wait_for_all_work_done(self):
        while 1:
            if self.account.all_work_done():
                return
            self.get_matching("DC_EVENT_CONNECTIVITY_CHANGED")

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

    def wait_all_initial_fetches(self):
        """Has to be called after start_io() to wait for fetch_existing_msgs to run
        so that new messages are not mistaken for old ones:
        - ac1 and ac2 are created
        - ac1 sends a message to ac2
        - ac2 is still running FetchExsistingMsgs job and thinks it's an existing, old message
        - therefore no DC_EVENT_INCOMING_MSG is sent"""
        self.get_info_contains("Done fetching existing messages")

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
        return None

    def wait_msg_delivered(self, msg):
        ev = self.get_matching("DC_EVENT_MSG_DELIVERED")
        assert ev.data1 == msg.chat.id
        assert ev.data2 == msg.id
        assert msg.is_out_delivered()


class EventThread(threading.Thread):
    """ Event Thread for an account.

    With each Account init this callback thread is started.
    """
    def __init__(self, account) -> None:
        self.account = account
        super(EventThread, self).__init__(name="events")
        self.daemon = True
        self._marked_for_shutdown = False
        self.start()

    @contextmanager
    def log_execution(self, message):
        self.account.log(message + " START")
        yield
        self.account.log(message + " FINISHED")

    def mark_shutdown(self) -> None:
        self._marked_for_shutdown = True

    def wait(self, timeout=None) -> None:
        if self == threading.current_thread():
            # we are in the callback thread and thus cannot
            # wait for the thread-loop to finish.
            return
        self.join(timeout=timeout)

    def run(self) -> None:
        """ get and run events until shutdown. """
        with self.log_execution("EVENT THREAD"):
            self._inner_run()

    def _inner_run(self):
        event_emitter = ffi.gc(
            lib.dc_get_event_emitter(self.account._dc_context),
            lib.dc_event_emitter_unref,
        )
        while not self._marked_for_shutdown:
            event = lib.dc_get_next_event(event_emitter)
            if event == ffi.NULL:
                break
            if self._marked_for_shutdown:
                break
            evt = lib.dc_event_get_id(event)
            data1 = lib.dc_event_get_data1_int(event)
            # the following code relates to the deltachat/_build.py's helper
            # function which provides us signature info of an event call
            evt_name = deltachat.get_dc_event_name(evt)
            if lib.dc_event_has_string_data(evt):
                data2 = from_optional_dc_charpointer(lib.dc_event_get_data2_str(event))
            else:
                data2 = lib.dc_event_get_data2_int(event)

            lib.dc_event_unref(event)
            ffi_event = FFIEvent(name=evt_name, data1=data1, data2=data2)
            try:
                self.account._pm.hook.ac_process_ffi_event(account=self, ffi_event=ffi_event)
                for name, kwargs in self._map_ffi_event(ffi_event):
                    self.account.log("calling hook name={} kwargs={}".format(name, kwargs))
                    hook = getattr(self.account._pm.hook, name)
                    hook(**kwargs)
            except Exception:
                if self.account._dc_context is not None:
                    raise

    def _map_ffi_event(self, ffi_event: FFIEvent):
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

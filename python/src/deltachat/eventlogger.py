import deltachat
import threading
import time
import re
from queue import Queue, Empty
from .hookspec import account_hookimpl, global_hookimpl


@global_hookimpl
def dc_account_init(account):
    # send all FFI events for this account to a plugin hook
    def _ll_event(ctx, evt_name, data1, data2):
        assert ctx == account._dc_context
        ffi_event = FFIEvent(name=evt_name, data1=data1, data2=data2)
        account._pm.hook.ac_process_ffi_event(
            account=account, ffi_event=ffi_event
        )
    deltachat.set_context_callback(account._dc_context, _ll_event)


@global_hookimpl
def dc_account_after_shutdown(dc_context):
    deltachat.clear_context_callback(dc_context)


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
        self._log_event(ffi_event)

    def _log_event(self, ffi_event):
        # don't show events that are anyway empty impls now
        if ffi_event.name == "DC_EVENT_GET_STRING":
            return
        self.account.ac_log_line(str(ffi_event))

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

    def get_matching(self, event_name_regex, check_error=True, timeout=None):
        self.account.ac_log_line("-- waiting for event with regex: {} --".format(event_name_regex))
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

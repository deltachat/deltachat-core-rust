import threading
import re
import time
from queue import Queue, Empty
from .hookspec import account_hookimpl


class EventLogger:
    _loglock = threading.RLock()

    def __init__(self, account, logid=None, debug=True):
        self.account = account
        self._event_queue = Queue()
        self._debug = debug
        if logid is None:
            logid = str(self.account._dc_context).strip(">").split()[-1]
        self.logid = logid
        self._timeout = None
        self.init_time = time.time()

    @account_hookimpl
    def process_low_level_event(self, event_name, data1, data2):
        self._log_event(event_name, data1, data2)
        self._event_queue.put((event_name, data1, data2))

    def set_timeout(self, timeout):
        self._timeout = timeout

    def consume_events(self, check_error=True):
        while not self._event_queue.empty():
            self.get(check_error=check_error)

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

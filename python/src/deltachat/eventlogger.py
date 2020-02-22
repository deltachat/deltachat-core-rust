import threading
import time
from .hookspec import account_hookimpl


class EventLogger:
    _loglock = threading.RLock()

    def __init__(self, account, logid=None, debug=True):
        self.account = account
        self._debug = debug
        if logid is None:
            logid = str(self.account._dc_context).strip(">").split()[-1]
        self.logid = logid
        self.init_time = time.time()

    @account_hookimpl
    def process_low_level_event(self, event_name, data1, data2):
        self._log_event(event_name, data1, data2)

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

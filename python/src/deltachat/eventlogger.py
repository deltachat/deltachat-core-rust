import threading
import time
from .hookspec import account_hookimpl, global_hookimpl


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
    def process_low_level_event(self, event_name, data1, data2):
        self._log_event(event_name, data1, data2)

    def _log_event(self, evt_name, data1, data2):
        # don't show events that are anyway empty impls now
        if evt_name == "DC_EVENT_GET_STRING":
            return
        evpart = "{}({!r},{!r})".format(evt_name, data1, data2)
        self.account.log_line(evpart)

    @account_hookimpl
    def log_line(self, message):
        t = threading.currentThread()
        tname = getattr(t, "name", t)
        if tname == "MainThread":
            tname = "MAIN"
        with self._loglock:
            print("{:2.2f} [{}-{}] {}".format(
                time.time() - self.init_time,
                tname,
                self.logid,
                message))

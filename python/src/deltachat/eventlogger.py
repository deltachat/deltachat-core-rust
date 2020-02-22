import threading
import time
from .hookspec import account_hookimpl


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
    def process_ffi_event(self, ffi_event):
        self._log_event(ffi_event)

    def _log_event(self, ffi_event):
        # don't show events that are anyway empty impls now
        if ffi_event.name == "DC_EVENT_GET_STRING":
            return
        self.account.log_line(str(ffi_event))

    @account_hookimpl
    def log_line(self, message):
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
            print(s)

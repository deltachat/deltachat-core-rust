import deltachat
import threading
import time
from .hookspec import account_hookimpl, global_hookimpl


@global_hookimpl
def account_init(account):
    # send all FFI events for this account to a plugin hook
    def _ll_event(ctx, evt_name, data1, data2):
        assert ctx == account._dc_context
        ffi_event = FFIEvent(name=evt_name, data1=data1, data2=data2)
        account._pm.hook.process_ffi_event(
            account=account, ffi_event=ffi_event
        )
    deltachat.set_context_callback(account._dc_context, _ll_event)


@global_hookimpl
def account_after_shutdown(dc_context):
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

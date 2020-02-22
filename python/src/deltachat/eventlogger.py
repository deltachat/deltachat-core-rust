import time
from .hookspec import account_hookimpl, global_hookimpl


@global_hookimpl
def at_account_init(account, logid):
    account._evlogger = account.add_account_plugin(EventLogger(account, logid=logid))


class EventLogger:
    def __init__(self, account, logid=None):
        self.account = account
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
        evpart = "{}({!r},{!r})".format(evt_name, data1, data2)
        self.account.log_line(evpart)

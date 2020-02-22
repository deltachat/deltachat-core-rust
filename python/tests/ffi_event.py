import re
from queue import Queue, Empty
from deltachat.hookspec import account_hookimpl


class FFIEventTracker:
    def __init__(self, account, timeout=None):
        self.account = account
        self._timeout = timeout
        self._event_queue = Queue()

    @account_hookimpl
    def process_low_level_event(self, event_name, data1, data2):
        self._event_queue.put((event_name, data1, data2))

    def set_timeout(self, timeout):
        self._timeout = timeout

    def consume_events(self, check_error=True):
        while not self._event_queue.empty():
            self.get(check_error=check_error)

    def get(self, timeout=None, check_error=True):
        timeout = timeout if timeout is not None else self._timeout
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
        self.account._evlogger._log("-- waiting for event with regex: {} --".format(event_name_regex))
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

    def wait_next_incoming_message(self):
        """ wait for and return next incoming message. """
        ev = self.get_matching("DC_EVENT_INCOMING_MSG")
        return self.account.get_message_by_id(ev[2])

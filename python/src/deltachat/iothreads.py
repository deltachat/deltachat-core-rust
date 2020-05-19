
import threading
import time

from contextlib import contextmanager

from .capi import ffi, lib
import deltachat
from .eventlogger import FFIEvent

class IOThreads:
    def __init__(self, account):
        self.account = account
        self._dc_context = account._dc_context
        self._thread_quitflag = False
        self._name2thread = {}

    def is_started(self):
        return len(self._name2thread) > 0

    def start(self):
        assert not self.is_started()

        self._start_one_thread("cb", self.cb_thread_run)

    def _start_one_thread(self, name, func):
        self._name2thread[name] = t = threading.Thread(target=func, name=name)
        t.setDaemon(1)
        t.start()

    @contextmanager
    def log_execution(self, message):
        self.account.ac_log_line(message + " START")
        yield
        self.account.ac_log_line(message + " FINISHED")

    def stop(self, wait=False):
        self._thread_quitflag = True

        # Workaround for a race condition. Make sure that thread is
        # not in between checking for quitflag and entering idle.
        time.sleep(0.5)

        if wait:
            for name, thread in self._name2thread.items():
                if thread != threading.currentThread():
                    thread.join()

    def cb_thread_run(self):
        with self.log_execution("CALLBACK THREAD START"):
            it = self.account.iter_events()
            while not self._thread_quitflag:
                try:
                    ev = next(it)
                except StopIteration:
                    break
                print("{}", ev)
                self.account.ac_log_line("calling hook name={} kwargs={}".format(ev.name, ev.kwargs))
                ev.call_hook()



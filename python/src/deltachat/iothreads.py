
import threading
import time

from contextlib import contextmanager

from .capi import lib


class IOThreads:
    def __init__(self, account):
        self.account = account
        self._dc_context = account._dc_context
        self._thread_quitflag = False
        self._name2thread = {}

    def is_started(self):
        return len(self._name2thread) > 0

    def start(self, callback_thread):
        assert not self.is_started()
        self._start_one_thread("inbox", self.imap_thread_run)
        self._start_one_thread("smtp", self.smtp_thread_run)

        if callback_thread:
            self._start_one_thread("cb", self.cb_thread_run)

        if int(self.account.get_config("mvbox_watch")):
            self._start_one_thread("mvbox", self.mvbox_thread_run)

        if int(self.account.get_config("sentbox_watch")):
            self._start_one_thread("sentbox", self.sentbox_thread_run)

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

        lib.dc_interrupt_imap_idle(self._dc_context)
        lib.dc_interrupt_smtp_idle(self._dc_context)
        if "mvbox" in self._name2thread:
            lib.dc_interrupt_mvbox_idle(self._dc_context)
        if "sentbox" in self._name2thread:
            lib.dc_interrupt_sentbox_idle(self._dc_context)
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
                self.account.ac_log_line("calling hook name={} kwargs={}".format(ev.name, ev.kwargs))
                ev.call_hook()

    def imap_thread_run(self):
        with self.log_execution("INBOX THREAD START"):
            while not self._thread_quitflag:
                lib.dc_perform_imap_jobs(self._dc_context)
                if not self._thread_quitflag:
                    lib.dc_perform_imap_fetch(self._dc_context)
                if not self._thread_quitflag:
                    lib.dc_perform_imap_idle(self._dc_context)

    def mvbox_thread_run(self):
        with self.log_execution("MVBOX THREAD"):
            while not self._thread_quitflag:
                lib.dc_perform_mvbox_jobs(self._dc_context)
                if not self._thread_quitflag:
                    lib.dc_perform_mvbox_fetch(self._dc_context)
                if not self._thread_quitflag:
                    lib.dc_perform_mvbox_idle(self._dc_context)

    def sentbox_thread_run(self):
        with self.log_execution("SENTBOX THREAD"):
            while not self._thread_quitflag:
                lib.dc_perform_sentbox_jobs(self._dc_context)
                if not self._thread_quitflag:
                    lib.dc_perform_sentbox_fetch(self._dc_context)
                if not self._thread_quitflag:
                    lib.dc_perform_sentbox_idle(self._dc_context)

    def smtp_thread_run(self):
        with self.log_execution("SMTP THREAD"):
            while not self._thread_quitflag:
                lib.dc_perform_smtp_jobs(self._dc_context)
                if not self._thread_quitflag:
                    lib.dc_perform_smtp_idle(self._dc_context)

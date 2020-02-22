
from queue import Queue
from threading import Event

from .hookspec import account_hookimpl


class ImexFailed(RuntimeError):
    """ Exception for signalling that import/export operations failed."""


class ImexTracker:
    def __init__(self):
        self._imex_events = Queue()

    @account_hookimpl
    def process_low_level_event(self, event_name, data1, data2):
        if event_name == "DC_EVENT_IMEX_PROGRESS":
            self._imex_events.put(data1)
        elif event_name == "DC_EVENT_IMEX_FILE_WRITTEN":
            self._imex_events.put(data1)

    def wait_finish(self, progress_timeout=60):
        """ Return list of written files, raise ValueError if ExportFailed. """
        files_written = []
        while True:
            ev = self._imex_events.get(timeout=progress_timeout)
            if isinstance(ev, str):
                files_written.append(ev)
            elif ev == 0:
                raise ImexFailed("export failed, exp-files: {}".format(files_written))
            elif ev == 1000:
                return files_written


class ConfigureFailed(RuntimeError):
    """ Exception for signalling that configuration failed."""


class ConfigureTracker:
    def __init__(self):
        self._configure_events = Queue()
        self._smtp_finished = Event()
        self._imap_finished = Event()
        self._ffi_events = []

    @account_hookimpl
    def process_low_level_event(self, event_name, data1, data2):
        self._ffi_events.append((event_name, data1, data2))
        if event_name == "DC_EVENT_SMTP_CONNECTED":
            self._smtp_finished.set()
        elif event_name == "DC_EVENT_IMAP_CONNECTED":
            self._imap_finished.set()

    @account_hookimpl
    def configure_completed(self, success):
        self._configure_events.put(success)

    def wait_smtp_connected(self):
        """ wait until smtp is configured. """
        self._smtp_finished.wait()

    def wait_imap_connected(self):
        """ wait until smtp is configured. """
        self._imap_finished.wait()

    def wait_finish(self):
        """ wait until configure is completed.

        Raise Exception if Configure failed
        """
        if not self._configure_events.get():
            content = "\n".join("{}: {} {}".format(*args) for args in self._ffi_events)
            raise ConfigureFailed(content)

from queue import Queue
from threading import Event

from .hookspec import Global, account_hookimpl


class ImexFailed(RuntimeError):
    """Exception for signalling that import/export operations failed."""


class ImexTracker:
    def __init__(self):
        self._imex_events = Queue()

    @account_hookimpl
    def ac_process_ffi_event(self, ffi_event):
        if ffi_event.name == "DC_EVENT_IMEX_PROGRESS":
            self._imex_events.put(ffi_event.data1)
        elif ffi_event.name == "DC_EVENT_IMEX_FILE_WRITTEN":
            self._imex_events.put(ffi_event.data2)

    def wait_progress(self, target_progress, progress_upper_limit=1000, progress_timeout=60):
        while True:
            ev = self._imex_events.get(timeout=progress_timeout)
            if isinstance(ev, int) and ev >= target_progress:
                assert ev <= progress_upper_limit, (
                    str(ev) + " exceeded upper progress limit " + str(progress_upper_limit)
                )
                return ev
            if ev == 0:
                return None

    def wait_finish(self, progress_timeout=60):
        """Return list of written files, raise ValueError if ExportFailed."""
        files_written = []
        while True:
            ev = self._imex_events.get(timeout=progress_timeout)
            if isinstance(ev, str):
                files_written.append(ev)
            elif ev == 0:
                raise ImexFailed(f"export failed, exp-files: {files_written}")
            elif ev == 1000:
                return files_written


class ConfigureFailed(RuntimeError):
    """Exception for signalling that configuration failed."""


class ConfigureTracker:
    ConfigureFailed = ConfigureFailed

    def __init__(self, account):
        self.account = account
        self._configure_events = Queue()
        self._smtp_finished = Event()
        self._imap_finished = Event()
        self._ffi_events = []
        self._progress = Queue()
        self._gm = Global._get_plugin_manager()

    @account_hookimpl
    def ac_process_ffi_event(self, ffi_event):
        self._ffi_events.append(ffi_event)
        if ffi_event.name == "DC_EVENT_SMTP_CONNECTED":
            self._smtp_finished.set()
        elif ffi_event.name == "DC_EVENT_IMAP_CONNECTED":
            self._imap_finished.set()
        elif ffi_event.name == "DC_EVENT_CONFIGURE_PROGRESS":
            self._progress.put(ffi_event.data1)

    @account_hookimpl
    def ac_configure_completed(self, success):
        if success:
            self._gm.hook.dc_account_extra_configure(account=self.account)
        self._configure_events.put(success)
        self.account.remove_account_plugin(self)

    def wait_smtp_connected(self):
        """Wait until SMTP is configured."""
        self._smtp_finished.wait()

    def wait_imap_connected(self):
        """Wait until IMAP is configured."""
        self._imap_finished.wait()

    def wait_progress(self, data1=None):
        while 1:
            evdata = self._progress.get()
            if data1 is None or evdata == data1:
                break

    def wait_finish(self, timeout=None):
        """
        Wait until configure is completed.

        Raise Exception if Configure failed
        """
        if not self._configure_events.get(timeout=timeout):
            content = "\n".join(map(str, self._ffi_events))
            raise ConfigureFailed(content)

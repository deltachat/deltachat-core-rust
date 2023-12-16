import os
import threading
import time
from queue import Empty, Queue

import pytest

import deltachat


def test_db_busy_error(acfactory):
    starttime = time.time()
    log_lock = threading.RLock()

    def log(string):
        with log_lock:
            print(f"{time.time() - starttime:3.2f} {string}")

    # make a number of accounts
    accounts = acfactory.get_many_online_accounts(3)
    log("created %s accounts" % len(accounts))

    # put a bigfile into each account
    for acc in accounts:
        acc.bigfile = os.path.join(acc.get_blobdir(), "bigfile")
        with open(acc.bigfile, "wb") as f:
            f.write(b"01234567890" * 1000_000)
    log("created %s bigfiles" % len(accounts))

    contact_addrs = [acc.get_self_contact().addr for acc in accounts]
    chat = accounts[0].create_group_chat("stress-group")
    for addr in contact_addrs[1:]:
        chat.add_contact(chat.account.create_contact(addr))

    # setup auto-responder bots which report back failures/actions
    report_queue = Queue()

    def report_func(replier, report_type, *report_args):
        report_queue.put((replier, report_type, report_args))

    # each replier receives all events and sends report events to receive_queue
    repliers = []
    for acc in accounts:
        replier = AutoReplier(acc, log=log, num_send=500, num_bigfiles=5, report_func=report_func)
        acc.add_account_plugin(replier)
        repliers.append(replier)

    # kick off message sending
    # after which repliers will reply to each other
    chat.send_text("hello")

    alive_count = len(accounts)
    while alive_count > 0:
        try:
            replier, report_type, report_args = report_queue.get(timeout=10)
        except Empty:
            log("timeout waiting for next event")
            pytest.fail("timeout exceeded")
        if report_type == ReportType.exit:
            replier.log("EXIT")
        elif report_type == ReportType.ffi_error:
            replier.log(f"ERROR: {report_args[0]}")
        elif report_type == ReportType.message_echo:
            continue
        else:
            raise ValueError(f"{addr} unknown report type {report_type}, args={report_args}")
        alive_count -= 1
        replier.log("shutting down")
        replier.account.shutdown()
        replier.log(f"shut down complete, remaining={alive_count}")


class ReportType:
    exit = "exit"
    ffi_error = "ffi-error"
    message_echo = "message-echo"


class AutoReplier:
    def __init__(self, account, log, num_send, num_bigfiles, report_func) -> None:
        self.account = account
        self._log = log
        self.report_func = report_func
        self.num_send = num_send
        self.num_bigfiles = num_bigfiles
        self.current_sent = 0
        self.addr = self.account.get_self_contact().addr

        self._thread = threading.Thread(name=f"Stats{self.account}", target=self.thread_stats)
        self._thread.setDaemon(True)
        self._thread.start()

    def log(self, message) -> None:
        self._log(f"{self.addr} {message}")

    def thread_stats(self):
        # XXX later use, for now we just quit
        return
        while 1:
            time.sleep(1.0)
            break

    @deltachat.account_hookimpl
    def ac_incoming_message(self, message):
        if self.current_sent >= self.num_send:
            self.report_func(self, ReportType.exit)
            return
        message.create_chat()
        message.mark_seen()
        self.log(f"incoming message: {message}")

        self.current_sent += 1
        # we are still alive, let's send a reply
        if self.num_bigfiles and self.current_sent % (self.num_send / self.num_bigfiles) == 0:
            message.chat.send_text(f"send big file as reply to: {message.text}")
            msg = message.chat.send_file(self.account.bigfile)
        else:
            msg = message.chat.send_text(f"got message id {message.id}, small text reply")
            assert msg.text
        self.log(f"message-sent: {msg}")
        self.report_func(self, ReportType.message_echo)
        if self.current_sent >= self.num_send:
            self.report_func(self, ReportType.exit)
            return

    @deltachat.account_hookimpl
    def ac_process_ffi_event(self, ffi_event):
        self.log(ffi_event)
        if ffi_event.name == "DC_EVENT_ERROR":
            self.report_func(self, ReportType.ffi_error, ffi_event)

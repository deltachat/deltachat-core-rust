"""
Internal Python-level IMAP handling used by the testplugin
and for cleaning up inbox/mvbox for each test function run.
"""

import imaplib
import io
import pathlib
import ssl
from contextlib import contextmanager
from typing import List

from imap_tools import (
    AND,
    Header,
    MailBox,
    MailBoxTls,
    MailMessage,
    MailMessageFlags,
    errors,
)

from deltachat import Account, const

FLAGS = b"FLAGS"
FETCH = b"FETCH"
ALL = "1:*"


class DirectImap:
    def __init__(self, account: Account) -> None:
        self.account = account
        self.logid = account.get_config("displayname") or id(account)
        self._idling = False
        self.connect()

    def connect(self):
        host = self.account.get_config("configured_mail_server")
        port = int(self.account.get_config("configured_mail_port"))
        security = int(self.account.get_config("configured_mail_security"))

        user = self.account.get_config("addr")
        pw = self.account.get_config("mail_pw")

        if security == const.DC_SOCKET_PLAIN:
            ssl_context = None
        else:
            ssl_context = ssl.create_default_context()

            # don't check if certificate hostname doesn't match target hostname
            ssl_context.check_hostname = False

            # don't check if the certificate is trusted by a certificate authority
            ssl_context.verify_mode = ssl.CERT_NONE

        if security == const.DC_SOCKET_STARTTLS:
            self.conn = MailBoxTls(host, port, ssl_context=ssl_context)
        elif security == const.DC_SOCKET_PLAIN or security == const.DC_SOCKET_SSL:
            self.conn = MailBox(host, port, ssl_context=ssl_context)
        self.conn.login(user, pw)

        self.select_folder("INBOX")

    def shutdown(self):
        try:
            self.conn.logout()
        except (OSError, imaplib.IMAP4.abort):
            print("Could not logout direct_imap conn")

    def create_folder(self, foldername):
        try:
            self.conn.folder.create(foldername)
        except errors.MailboxFolderCreateError as e:
            print("Can't create", foldername, "probably it already exists:", str(e))

    def select_folder(self, foldername: str) -> tuple:
        assert not self._idling
        return self.conn.folder.set(foldername)

    def select_config_folder(self, config_name: str):
        """Return info about selected folder if it is
        configured, otherwise None.
        """
        if "_" not in config_name:
            config_name = f"configured_{config_name}_folder"
        foldername = self.account.get_config(config_name)
        if foldername:
            return self.select_folder(foldername)
        return None

    def list_folders(self) -> List[str]:
        """return list of all existing folder names."""
        assert not self._idling
        return [folder.name for folder in self.conn.folder.list()]

    def delete(self, uid_list: str, expunge=True):
        """delete a range of messages (imap-syntax).
        If expunge is true, perform the expunge-operation
        to make sure the messages are really gone and not
        just flagged as deleted.
        """
        self.conn.client.uid("STORE", uid_list, "+FLAGS", r"(\Deleted)")
        if expunge:
            self.conn.expunge()

    def get_all_messages(self) -> List[MailMessage]:
        assert not self._idling
        return list(self.conn.fetch())

    def get_unread_messages(self) -> List[str]:
        assert not self._idling
        return [msg.uid for msg in self.conn.fetch(AND(seen=False))]

    def mark_all_read(self):
        messages = self.get_unread_messages()
        if messages:
            res = self.conn.flag(messages, MailMessageFlags.SEEN, True)
            print("marked seen:", messages, res)

    def get_unread_cnt(self) -> int:
        return len(self.get_unread_messages())

    def dump_imap_structures(self, dir, logfile):
        assert not self._idling
        stream = io.StringIO()

        def log(*args, **kwargs):
            kwargs["file"] = stream
            print(*args, **kwargs)

        empty_folders = []
        for imapfolder in self.list_folders():
            self.select_folder(imapfolder)
            messages = list(self.get_all_messages())
            if not messages:
                empty_folders.append(imapfolder)
                continue

            log("---------", imapfolder, len(messages), "messages ---------")
            # get message content without auto-marking it as seen
            # fetching 'RFC822' would mark it as seen.
            for msg in self.conn.fetch(mark_seen=False):
                body = getattr(msg.obj, "text", None)
                if not body:
                    body = getattr(msg.obj, "html", None)
                if not body:
                    log("Message", msg.uid, "has empty body")
                    continue

                path = pathlib.Path(str(dir)).joinpath("IMAP", self.logid, imapfolder)
                path.mkdir(parents=True, exist_ok=True)
                fn = path.joinpath(str(msg.uid))
                fn.write_bytes(body)
                log("Message", msg.uid, fn)
                log(
                    "Message",
                    msg.uid,
                    msg.flags,
                    "Message-Id:",
                    msg.obj.get("Message-Id"),
                )

        if empty_folders:
            log("--------- EMPTY FOLDERS:", empty_folders)

        print(stream.getvalue(), file=logfile)

    @contextmanager
    def idle(self):
        """return Idle ContextManager."""
        idle_manager = IdleManager(self)
        try:
            yield idle_manager
        finally:
            idle_manager.done()

    def append(self, folder: str, msg: str):
        """Upload a message to *folder*.
        Trailing whitespace or a linebreak at the beginning will be removed automatically.
        """
        if msg.startswith("\n"):
            msg = msg[1:]
        msg = "\n".join([s.lstrip() for s in msg.splitlines()])
        self.conn.append(bytes(msg, encoding="ascii"), folder)

    def get_uid_by_message_id(self, message_id) -> str:
        msgs = [msg.uid for msg in self.conn.fetch(AND(header=Header("MESSAGE-ID", message_id)))]
        if len(msgs) == 0:
            raise Exception("Did not find message " + message_id + ", maybe you forgot to select the correct folder?")
        return msgs[0]


class IdleManager:
    def __init__(self, direct_imap):
        self.direct_imap = direct_imap
        self.log = direct_imap.account.log
        # fetch latest messages before starting idle so that it only
        # returns messages that arrive anew
        self.direct_imap.conn.fetch("1:*")
        self.direct_imap.conn.idle.start()

    def check(self, timeout=None) -> List[bytes]:
        """(blocking) wait for next idle message from server."""
        self.log("imap-direct: calling idle_check")
        res = self.direct_imap.conn.idle.poll(timeout=timeout)
        self.log(f"imap-direct: idle_check returned {res!r}")
        return res

    def wait_for_new_message(self, timeout=None) -> bytes:
        while 1:
            for item in self.check(timeout=timeout):
                if b"EXISTS" in item or b"RECENT" in item:
                    return item

    def wait_for_seen(self, timeout=None) -> int:
        """Return first message with SEEN flag from a running idle-stream."""
        while 1:
            for item in self.check(timeout=timeout):
                if FETCH in item:
                    self.log(str(item))
                    if FLAGS in item and rb"\Seen" in item:
                        return int(item.split(b" ")[1])

    def done(self):
        """send idle-done to server if we are currently in idle mode."""
        return self.direct_imap.conn.idle.stop()

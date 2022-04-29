"""
Internal Python-level IMAP handling used by the testplugin
and for cleaning up inbox/mvbox for each test function run.
"""

import io
import ssl
import pathlib
from imap_tools import MailBox, MailBoxTls, errors, AND, Header, MailMessageFlags, MailMessage
import imaplib
import deltachat
from deltachat import const, Account
from typing import List


FLAGS = b'FLAGS'
FETCH = b'FETCH'
ALL = "1:*"


@deltachat.global_hookimpl
def dc_account_extra_configure(account: Account):
    """ Reset the account (we reuse accounts across tests)
    and make 'account.direct_imap' available for direct IMAP ops.
    """
    try:

        if not hasattr(account, "direct_imap"):
            imap = DirectImap(account)

            for folder in imap.list_folders():
                if folder.lower() == "inbox" or folder.lower() == "deltachat":
                    assert imap.select_folder(folder)
                    imap.delete(ALL, expunge=True)
                else:
                    imap.conn.folder.delete(folder)
                    # We just deleted the folder, so we have to make DC forget about it, too
                    if account.get_config("configured_sentbox_folder") == folder:
                        account.set_config("configured_sentbox_folder", None)

            setattr(account, "direct_imap", imap)

    except Exception as e:
        # Uncaught exceptions here would lead to a timeout without any note written to the log
        # start with DC_EVENT_WARNING so that the line is printed in yellow and won't be overlooked when reading
        account.log("DC_EVENT_WARNING =================== DIRECT_IMAP CAN'T RESET ACCOUNT: ===================")
        account.log("DC_EVENT_WARNING =================== " + str(e) + " ===================")


@deltachat.global_hookimpl
def dc_account_after_shutdown(account):
    """ shutdown the imap connection if there is one. """
    imap = getattr(account, "direct_imap", None)
    if imap is not None:
        imap.shutdown()
        del account.direct_imap


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
            self.idle_done()
        except (OSError, imaplib.IMAP4.abort):
            pass
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
        """ Return info about selected folder if it is
        configured, otherwise None. """
        if "_" not in config_name:
            config_name = "configured_{}_folder".format(config_name)
        foldername = self.account.get_config(config_name)
        if foldername:
            return self.select_folder(foldername)

    def list_folders(self) -> List[str]:
        """ return list of all existing folder names"""
        assert not self._idling
        return [folder.name for folder in self.conn.folder.list()]

    def delete(self, uid_list: str, expunge=True):
        """ delete a range of messages (imap-syntax).
        If expunge is true, perform the expunge-operation
        to make sure the messages are really gone and not
        just flagged as deleted.
        """
        self.conn.client.uid('STORE', uid_list, '+FLAGS', r'(\Deleted)')
        if expunge:
            self.conn.expunge()

    def get_all_messages(self) -> List[MailMessage]:
        assert not self._idling
        return [mail for mail in self.conn.fetch()]

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
                log("Message", msg.uid, msg.flags, "Message-Id:", msg.obj.get("Message-Id"))

        if empty_folders:
            log("--------- EMPTY FOLDERS:", empty_folders)

        print(stream.getvalue(), file=logfile)

    def idle_start(self):
        """ switch this connection to idle mode. non-blocking. """
        assert not self._idling
        res = self.conn.idle.start()
        self._idling = True
        return res

    def idle_check(self, terminate=False, timeout=None) -> List[bytes]:
        """ (blocking) wait for next idle message from server. """
        assert self._idling
        self.account.log("imap-direct: calling idle_check")
        res = self.conn.idle.poll(timeout=timeout)
        if terminate:
            self.idle_done()
        self.account.log("imap-direct: idle_check returned {!r}".format(res))
        return res

    def idle_wait_for_new_message(self, terminate=False, timeout=None) -> bytes:
        while 1:
            for item in self.idle_check(timeout=timeout):
                if b'EXISTS' in item or b'RECENT' in item:
                    if terminate:
                        self.idle_done()
                    return item

    def idle_wait_for_seen(self, terminate=False, timeout=None) -> int:
        """ Return first message with SEEN flag from a running idle-stream.
        """
        while 1:
            for item in self.idle_check(timeout=timeout):
                if FETCH in item:
                    self.account.log(str(item))
                    if FLAGS in item and rb'\Seen' in item:
                        if terminate:
                            self.idle_done()
                        return int(item.split(b' ')[1])

    def idle_done(self):
        """ send idle-done to server if we are currently in idle mode. """
        if self._idling:
            res = self.conn.idle.stop()
            self._idling = False
            return res

    def append(self, folder: str, msg: str):
        """Upload a message to *folder*.
        Trailing whitespace or a linebreak at the beginning will be removed automatically.
        """
        if msg.startswith("\n"):
            msg = msg[1:]
        msg = '\n'.join([s.lstrip() for s in msg.splitlines()])
        self.conn.append(bytes(msg, encoding='ascii'), folder)

    def get_uid_by_message_id(self, message_id) -> str:
        msgs = [msg.uid for msg in self.conn.fetch(AND(header=Header('MESSAGE-ID', message_id)))]
        if len(msgs) == 0:
            raise Exception("Did not find message " + message_id + ", maybe you forgot to select the correct folder?")
        return msgs[0]

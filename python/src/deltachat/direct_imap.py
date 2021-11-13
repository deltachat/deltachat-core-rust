"""
Internal Python-level IMAP handling used by the testplugin
and for cleaning up inbox/mvbox for each test function run.
"""

import io
import email
import ssl
import pathlib
from imapclient import IMAPClient
from imapclient.exceptions import IMAPClientError
import imaplib
import deltachat
from deltachat import const, Account


SEEN = b'\\Seen'
DELETED = b'\\Deleted'
FLAGS = b'FLAGS'
FETCH = b'FETCH'
ALL = "1:*"


@deltachat.global_hookimpl
def dc_account_extra_configure(account):
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
                    imap.conn.delete_folder(folder)
                    # We just deleted the folder, so we have to make DC forget about it, too
                    if account.get_config("configured_sentbox_folder") == folder:
                        account.set_config("configured_sentbox_folder", None)
                    if account.get_config("configured_spam_folder") == folder:
                        account.set_config("configured_spam_folder", None)

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
            self.conn = IMAPClient(host, port, ssl=False)
            self.conn.starttls(ssl_context)
        elif security == const.DC_SOCKET_PLAIN:
            self.conn = IMAPClient(host, port, ssl=False)
        elif security == const.DC_SOCKET_SSL:
            self.conn = IMAPClient(host, port, ssl_context=ssl_context)
        self.conn.login(user, pw)

        self.select_folder("INBOX")

    def shutdown(self):
        try:
            self.conn.idle_done()
        except (OSError, IMAPClientError):
            pass
        try:
            self.conn.logout()
        except (OSError, IMAPClientError):
            print("Could not logout direct_imap conn")

    def create_folder(self, foldername):
        try:
            self.conn.create_folder(foldername)
        except imaplib.IMAP4.error as e:
            print("Can't create", foldername, "probably it already exists:", str(e))

    def select_folder(self, foldername):
        assert not self._idling
        return self.conn.select_folder(foldername)

    def select_config_folder(self, config_name):
        """ Return info about selected folder if it is
        configured, otherwise None. """
        if "_" not in config_name:
            config_name = "configured_{}_folder".format(config_name)
        foldername = self.account.get_config(config_name)
        if foldername:
            return self.select_folder(foldername)

    def list_folders(self):
        """ return list of all existing folder names"""
        assert not self._idling
        folders = []
        for meta, sep, foldername in self.conn.list_folders():
            folders.append(foldername)
        return folders

    def delete(self, range, expunge=True):
        """ delete a range of messages (imap-syntax).
        If expunge is true, perform the expunge-operation
        to make sure the messages are really gone and not
        just flagged as deleted.
        """
        self.conn.set_flags(range, [DELETED])
        if expunge:
            self.conn.expunge()

    def get_all_messages(self):
        assert not self._idling

        # Flush unsolicited responses. IMAPClient has problems
        # dealing with them: https://github.com/mjs/imapclient/issues/334
        # When this NOOP was introduced, next FETCH returned empty
        # result instead of a single message, even though IMAP server
        # can only return more untagged responses than required, not
        # less.
        self.conn.noop()

        return self.conn.fetch(ALL, [FLAGS])

    def get_unread_messages(self):
        assert not self._idling
        res = self.conn.fetch(ALL, [FLAGS])
        return [uid for uid in res
                if SEEN not in res[uid][FLAGS]]

    def mark_all_read(self):
        messages = self.get_unread_messages()
        if messages:
            res = self.conn.set_flags(messages, [SEEN])
            print("marked seen:", messages, res)

    def get_unread_cnt(self):
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
            requested = [b'BODY.PEEK[]', FLAGS]
            for uid, data in self.conn.fetch(messages, requested).items():
                body_bytes = data[b'BODY[]']
                if not body_bytes:
                    log("Message", uid, "has empty body")
                    continue

                flags = data[FLAGS]
                path = pathlib.Path(str(dir)).joinpath("IMAP", self.logid, imapfolder)
                path.mkdir(parents=True, exist_ok=True)
                fn = path.joinpath(str(uid))
                fn.write_bytes(body_bytes)
                log("Message", uid, fn)
                email_message = email.message_from_bytes(body_bytes)
                log("Message", uid, flags, "Message-Id:", email_message.get("Message-Id"))

        if empty_folders:
            log("--------- EMPTY FOLDERS:", empty_folders)

        print(stream.getvalue(), file=logfile)

    def idle_start(self):
        """ switch this connection to idle mode. non-blocking. """
        assert not self._idling
        res = self.conn.idle()
        self._idling = True
        return res

    def idle_check(self, terminate=False):
        """ (blocking) wait for next idle message from server. """
        assert self._idling
        self.account.log("imap-direct: calling idle_check")
        res = self.conn.idle_check(timeout=30)
        if len(res) == 0:
            raise TimeoutError
        if terminate:
            self.idle_done()
        self.account.log("imap-direct: idle_check returned {!r}".format(res))
        return res

    def idle_wait_for_seen(self):
        """ Return first message with SEEN flag
        from a running idle-stream REtiurn.
        """
        while 1:
            for item in self.idle_check():
                if item[1] == FETCH:
                    if item[2][0] == FLAGS:
                        if SEEN in item[2][1]:
                            return item[0]

    def idle_done(self):
        """ send idle-done to server if we are currently in idle mode. """
        if self._idling:
            res = self.conn.idle_done()
            self._idling = False
            return res

    def append(self, folder, msg):
        """Upload a message to *folder*.
        Trailing whitespace or a linebreak at the beginning will be removed automatically.
        """
        if msg.startswith("\n"):
            msg = msg[1:]
        msg = '\n'.join([s.lstrip() for s in msg.splitlines()])
        self.conn.append(folder, msg)

    def get_uid_by_message_id(self, message_id):
        msgs = self.conn.search(['HEADER', 'MESSAGE-ID', message_id])
        if len(msgs) == 0:
            raise Exception("Did not find message " + message_id + ", maybe you forgot to select the correct folder?")
        return msgs[0]

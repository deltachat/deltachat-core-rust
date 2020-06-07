import io
import email
import ssl
import pathlib
from imapclient import IMAPClient
from imapclient.exceptions import IMAPClientError


class ImapConn:
    def __init__(self, account):
        self.account = account
        self.connect()

    def connect(self):
        ssl_context = ssl.create_default_context()

        # don't check if certificate hostname doesn't match target hostname
        ssl_context.check_hostname = False

        # don't check if the certificate is trusted by a certificate authority
        ssl_context.verify_mode = ssl.CERT_NONE

        host = self.account.get_config("configured_mail_server")
        user = self.account.get_config("addr")
        pw = self.account.get_config("mail_pw")
        self.conn = IMAPClient(host, ssl_context=ssl_context)
        self.conn.login(user, pw)

        self._original_msg_count = {}
        self.select_folder("INBOX")

    def shutdown(self):
        try:
            self.conn.logout()
        except (OSError, IMAPClientError):
            print("Could not logout direct_imap conn")

    def select_folder(self, foldername):
        res = self.conn.select_folder(foldername)
        self.foldername = foldername
        msg_count = res[b'UIDNEXT'] - 1
        # memorize initial message count on first select
        self._original_msg_count.setdefault(foldername, msg_count)
        return res

    def select_config_folder(self, config_name):
        if "_" not in config_name:
            config_name = "configured_{}_folder".format(config_name)
        foldername = self.account.get_config(config_name)
        return self.select_folder(foldername)

    def list_folders(self):
        folders = []
        for meta, sep, foldername in self.conn.list_folders():
            folders.append(foldername)
        return folders

    def get_unread_messages(self):
        return self.conn.search("UNSEEN")

    def mark_all_read(self):
        messages = self.get_unread_messages()
        if messages:
            res = self.conn.set_flags(messages, ['\\SEEN'])
            print("marked seen:", messages, res)

    def get_unread_cnt(self):
        return len(self.get_unread_messages())

    def get_new_email_cnt(self):
        return self.get_unread_cnt() - self._original_msg_count[self.foldername]

    def dump_account_info(self, logfile):
        def log(*args, **kwargs):
            kwargs["file"] = logfile
            print(*args, **kwargs)

        cursor = 0
        for name, val in self.account.get_info().items():
            entry = "{}={}".format(name.upper(), val)
            if cursor + len(entry) > 80:
                log("")
                cursor = 0
            log(entry, end=" ")
            cursor += len(entry) + 1
        log("")

    def dump_imap_structures(self, dir, logfile):
        stream = io.StringIO()

        def log(*args, **kwargs):
            kwargs["file"] = stream
            print(*args, **kwargs)

        acinfo = self.account.logid + "-" + self.account.get_config("addr")

        empty_folders = []
        for imapfolder in self.list_folders():
            self.select_folder(imapfolder)
            messages = self.conn.search('ALL')
            if not messages:
                empty_folders.append(imapfolder)
                continue

            log("---------", imapfolder, len(messages), "messages ---------")
            for uid, data in self.conn.fetch(messages, [b'RFC822', b'FLAGS']).items():
                body_bytes = data[b'RFC822']
                flags = data[b'FLAGS']
                path = pathlib.Path(str(dir)).joinpath("IMAP", acinfo, imapfolder)
                path.mkdir(parents=True, exist_ok=True)
                fn = path.joinpath(str(uid))
                fn.write_bytes(body_bytes)
                log("Message", uid, "saved as", fn)
                email_message = email.message_from_bytes(body_bytes)
                log("Message", uid, flags, "Message-Id:", email_message.get("Message-Id"))

        if empty_folders:
            log("--------- EMPTY FOLDERS:", empty_folders)

        print(stream.getvalue(), file=logfile)

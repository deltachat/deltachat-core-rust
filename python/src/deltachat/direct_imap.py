import imaplib
import pathlib
from . import Account


def db_folder_attr(name):
    def fget(s):
        return s.db_folder.get(name, 1)

    def fset(s, val):
        s.db_folder[name] = val
    return property(fget, fset, None, None)


class ImapConn:
    def __init__(self, account):
        self.account = account
        self.conn_info = (account.get_config("configured_mail_server"),
                          account.get_config("addr"),
                          account.get_config("mail_pw"))

        host, user, pw = self.conn_info
        self.connection = imaplib.IMAP4_SSL(host)
        self.connection.login(user, pw)
        self._original_msg_count = {}
        self.select_folder("INBOX")

    def shutdown(self):
        try:
            self.connection.close()
        except Exception:
            pass
        try:
            self.connection.logout()
        except Exception:
            print("Could not logout direct_imap conn")

    def select_folder(self, foldername):
        status, messages = self.connection.select(foldername)
        if status != "OK":
            raise ConnectionError("Could not select {}: status={} message={}".format(
                                  foldername, status, messages))
        print("selected", foldername, messages)
        self.foldername = foldername
        try:
            msg_count = int(messages[0])
        except IndexError:
            msg_count = 0

        # memorize initial message count on first select
        self._original_msg_count.setdefault(foldername, msg_count)
        return messages

    def select_config_folder(self, config_name):
        if "_" not in config_name:
            config_name = "configured_{}_folder".format(config_name)
        foldername = self.account.get_config(config_name)
        return self.select_folder(foldername)

    def mark_all_read(self):
#        result, data = self.connection.uid('search', None, "(UNSEEN)")
        result, data = self.connection.search(None, 'UnSeen')
        try:
            mails_uid = data[0].split()
            print("New mails")

#            self.connection.store(data[0].replace(' ',','),'+FLAGS','\Seen')
            for e_id in mails_uid:
                self.connection.store(e_id, '+FLAGS', '\\Seen')
                print("marked:", e_id)

            return True
        except IndexError:
            print("No unread")
            return False

    def get_unread_cnt(self):
#        result, data = self.connection.uid('search', None, "(UNSEEN)")
        result, data = self.connection.search(None, 'UnSeen')
        try:
            mails_uid = data[0].split()

            return len(mails_uid)
        except IndexError:
            return 0

    def get_new_email_cnt(self):
        messages = self.select_folder(self.foldername)
        try:
            return int(messages[0]) - self._original_msg_count[self.foldername]
        except IndexError:
            return 0


def print_imap_structure(database, dir="."):
    print_imap_structure_ac(Account(database), dir)


def print_imap_structure_ac(ac, dir="."):
    acinfo = ac.logid + "-" + ac.get_config("addr")
    print("================= ACCOUNT", acinfo, "=================")
    print("----------------- CONFIG: -----------------")
    print(ac.get_info())

    for imapfolder in [INBOX, MVBOX, SENT, MVBOX_FALLBBACK]:
        try:
            imap = make_direct_imap(ac, imapfolder)
            c = imap.connection
            typ, data = c.search(None, 'ALL')
            c._get_tagged_response
            print("-----------------", imapfolder, "-----------------")
            for num in data[0].split():
                typ, data = c.fetch(num, '(RFC822)')
                body = data[0][1]

                typ, data = c.fetch(num, '(UID FLAGS)')
                info = data[0]

                path = pathlib.Path(dir).joinpath("IMAP-MESSAGES", acinfo, imapfolder)
                path.mkdir(parents=True, exist_ok=True)
                file = path.joinpath(str(info).replace("b'", "").replace("'", "").replace("\\", ""))
                file.write_bytes(body)
                print("Message", info, "saved as", file)
        except Exception:
            pass

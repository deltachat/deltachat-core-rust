import os
import threading
import click
import ssl
import atexit
import email
import contextlib
import time
import imaplib
from subprocess import call

INBOX = "Inbox"
SENT = "Sent"
MVBOX = "DeltaChat"
DC_CONSTANT_MSG_MOVESTATE_PENDING = 1
DC_CONSTANT_MSG_MOVESTATE_STAY = 2
DC_CONSTANT_MSG_MOVESTATE_MOVING = 3


def db_folder_attr(name):
    def fget(s):
        return s.db_folder.get(name, 1)

    def fset(s, val):
        s.db_folder[name] = val
    return property(fget, fset, None, None)

class ImapConn():
    def __init__(self, foldername, conn_info):
        self.foldername = foldername
        host, user, pw = conn_info

        self.connection = imaplib.IMAP4_SSL(host)
        self.connection.login(user, pw)
        messages = self._reselect_folder()
        try:
            self.original_msg_count = messages[0]
        except IndexError:
            self.original_msg_count = 0

    def mark_all_read(self):
        self._reselect_folder()
#        result, data = self.connection.uid('search', None, "(UNSEEN)")
        result, data = self.connection.search(None, 'UnSeen')
        try:
            mails_uid = data[0].split()
            newest_mail = mails_uid[0]
            print("New mails")

#            self.connection.store(data[0].replace(' ',','),'+FLAGS','\Seen')
            for e_id in mails_uid:
                self.connection.store(e_id, '+FLAGS', '\Seen')
                print("marked:",e_id)

            return True
        except IndexError:
            print("No unread")
            return False

    def get_unread_cnt(self):
        self._reselect_folder()
#        result, data = self.connection.uid('search', None, "(UNSEEN)")
        result, data = self.connection.search(None, 'UnSeen')
        try:
            mails_uid = data[0].split()

            return len(mails_uid)
        except IndexError:
            return 0

    def get_new_email_cnt(self):
        messages = self._reselect_folder()
        try:
            return messages[0] - self.original_msg_count
        except IndexError:
            return 0

    def _reselect_folder(self):
        status, messages = self.connection.select(self.foldername)
        if status != "OK":
            print("Incorrect mail box " + status + str(messages))
            raise ConnectionError
        print("(Re-)Selected mailbox: " + status + " " + str(messages))
        return messages

    def __del__(self):
        self.connection.shutdown()

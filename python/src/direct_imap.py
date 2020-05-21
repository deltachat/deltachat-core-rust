import os
import threading
import click
import ssl
import atexit
import email
from imapclient import IMAPClient
from imapclient.exceptions import IMAPClientError
import contextlib
import time
from persistentdict import PersistentDict


INBOX = "INBOX"
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


lock_log = threading.RLock()
started = time.time()


class ImapConn(object):
    def __init__(self, db, foldername, conn_info):
        self.db = db
        self.foldername = foldername
        self._thread = None
        self.MHOST, self.MUSER, self.MPASSWORD = conn_info
        self.event_initial_polling_complete = threading.Event()
        self.pending_imap_jobs = False

        # persistent database state below
        self.db_folder = self.db.setdefault(foldername, {})
        self.db_messages = self.db.setdefault(":message-full", {})

    last_sync_uid = db_folder_attr("last_sync_uid")

    @contextlib.contextmanager
    def wlog(self, msg):
        t = time.time() - started
        with lock_log:
            print("%03.2f [%s] %s -->" % (t, self.foldername, msg))
            t0 = time.time()
        yield
        t1 = time.time()
        with lock_log:
            print("%03.2f [%s] ... finish %s (%3.2f secs)" % (t1-started, self.foldername, msg, t1-t0))

    def log(self, *msgs):
        t = time.time() - started
        bmsg = "%03.2f [%s]" % (t, self.foldername)
        with lock_log:
            print(bmsg, *msgs)

    def connect(self):
        with self.wlog("IMAP_CONNECT {}: {}".format(self.MUSER, self.MPASSWORD)):
            ssl_context = ssl.create_default_context()

            # don't check if certificate hostname doesn't match target hostname
            ssl_context.check_hostname = False

            # don't check if the certificate is trusted by a certificate authority
            ssl_context.verify_mode = ssl.CERT_NONE
            self.conn = IMAPClient(self.MHOST, ssl_context=ssl_context)
            self.conn.login(self.MUSER, self.MPASSWORD)
            self.log(self.conn.welcome)
            try:
                self.select_info = self.conn.select_folder(self.foldername)
            except IMAPClientError:
                self.ensure_folder_exists()
                self.select_info = self.conn.select_folder(self.foldername)

            self.log('folder has %d messages' % self.select_info[b'EXISTS'])
            self.log('capabilities', self.conn.capabilities())

    def ensure_folder_exists(self):
        with self.wlog("ensure_folder_exists: {}".format(self.foldername)):
            try:
                resp = self.conn.create_folder(self.foldername)
            except IMAPClientError as e:
                if "ALREADYEXISTS" in str(e):
                    return
                print("EXCEPTION:" + str(e))
            else:
                print("Server sent:", resp if resp else "nothing")

    def move(self, messages):
        self.log("IMAP_MOVE to {}: {}".format(MVBOX, messages))
        try:
            resp = self.conn.move(messages, MVBOX)
        except IMAPClientError as e:
            if "EXPUNGEISSUED" in str(e):
                self.log("IMAP_MOVE errored with EXPUNGEISSUED, probably another client moved it")
            else:
                self.log("IMAP_MOVE {} successfully completed.".format(messages))

    def perform_imap_idle(self):
        if self.pending_imap_jobs:
            self.log("perform_imap_idle skipped because jobs are pending")
            return
        with self.wlog("IMAP_IDLE()"):
            res = self.conn.idle()
            interrupted = False
            while not interrupted:
                # Wait for up to 30 seconds for an IDLE response
                responses = self.conn.idle_check(timeout=30)
                self.log("Server sent:", responses if responses else "nothing")
                for resp in responses:
                    if resp[1] == b"EXISTS":
                        # we ignore what is returned and just let
                        # perform_imap_fetch look since lastseen
                        # id = resp[0]
                        interrupted = True
            resp = self.conn.idle_done()

    def perform_imap_fetch(self):
        range = "%s:*" % (self.last_sync_uid + 1,)
        with self.wlog("IMAP_PERFORM_FETCH %s" % (range,)):
            requested_fields = [
                b"RFC822.SIZE", b'FLAGS',
                b"BODY.PEEK[HEADER.FIELDS (FROM TO CC DATE CHAT-VERSION MESSAGE-ID IN-REPLY-TO)]"
            ]
            resp = self.conn.fetch(range, requested_fields)
            timestamp_fetch = time.time()
            for uid in sorted(resp):  # get lower uids first
                if uid < self.last_sync_uid:
                    self.log("IMAP-ODDITY: ignoring bogus uid %s, it is lower than min-requested %s" % (
                             uid, self.last_sync_uid))
                    continue
                data = resp[uid]
                headers = data[requested_fields[-1].replace(b'.PEEK', b'')]
                msg_headers = email.message_from_bytes(headers)
                message_id = normalized_messageid(msg_headers)
                chat_version = msg_headers.get("Chat-Version")
                in_reply_to = msg_headers.get("In-Reply-To", "").lower()

                if not self.has_message(normalized_messageid(msg_headers)):
                    self.log('fetching body of ID %d: %d bytes, message-id=%s '
                             'in-reply-to=%s chat-version=%s' % (
                                 uid, data[b'RFC822.SIZE'], message_id, in_reply_to, chat_version,))
                    fetchbody_resp = self.conn.fetch(uid, [b'BODY.PEEK[]'])
                    msg = email.message_from_bytes(fetchbody_resp[uid][b'BODY[]'])
                    msg.fetch_retrieve_time = timestamp_fetch
                    msg.foldername = self.foldername
                    msg.uid = uid
                    msg.move_state = DC_CONSTANT_MSG_MOVESTATE_PENDING
                    self.store_message(message_id, msg)
                else:
                    msg = self.get_message_from_db(message_id)
                    self.log('fetching-from-db: ID %s message-id=%s' % (uid, message_id))
                    if msg.foldername != self.foldername:
                        self.log("detected moved message", message_id)
                        msg.foldername = self.foldername
                        msg.move_state = DC_CONSTANT_MSG_MOVESTATE_STAY

                if self.foldername in (INBOX, SENT):
                    if self.resolve_move_status(msg) != DC_CONSTANT_MSG_MOVESTATE_PENDING:
                        # see if there are pending messages which have a in-reply-to
                        # to our currnet msg
                        # NOTE: should be one sql-statement to find the
                        # possibly multiple messages that waited on us
                        for dbmid, dbmsg in self.db_messages.items():
                            if dbmsg.move_state == DC_CONSTANT_MSG_MOVESTATE_PENDING:
                                if dbmsg.get("In-Reply-To", "").lower() == message_id:
                                    self.log("resolving pending message", dbmid)
                                    # resolving the dependent message must work now
                                    res = self.resolve_move_status(dbmsg)
                                    assert res != DC_CONSTANT_MSG_MOVESTATE_PENDING, (dbmid, res)

                if not self.has_message(message_id):
                    self.store_message(message_id, msg)

                self.last_sync_uid = max(uid, self.last_sync_uid)

        self.log("last-sync-uid after fetch:", self.last_sync_uid)
        self.db.sync()

    def resolve_move_status(self, msg):
        """ Return move-state after this message's next move-state is determined (i.e. it is not PENDING)"""
        message_id = normalized_messageid(msg)
        if msg.move_state == DC_CONSTANT_MSG_MOVESTATE_PENDING:
            res = self.determine_next_move_state(msg)
            if res == DC_CONSTANT_MSG_MOVESTATE_MOVING:
                self.schedule_move(msg)
                msg.move_state = DC_CONSTANT_MSG_MOVESTATE_MOVING
            elif res == DC_CONSTANT_MSG_MOVESTATE_STAY:
                self.log("STAY uid=%s message-id=%s" % (msg.uid, message_id))
                msg.move_state = DC_CONSTANT_MSG_MOVESTATE_STAY
            else:
                self.log("PENDING uid=%s message-id=%s in-reply-to=%s" % (
                         msg.uid, message_id, msg["In-Reply-To"]))
        return msg.move_state

    def determine_next_move_state(self, msg):
        """ Return the next move state for this message.
        Only call this function if the message is pending.
        This function works with the DB, does not perform any IMAP commands.
        """
        self.log("shall_move %s " % (normalized_messageid(msg)))
        assert self.foldername in (INBOX, SENT)
        assert msg.move_state == DC_CONSTANT_MSG_MOVESTATE_PENDING
        if msg.foldername == MVBOX:
            self.log("is already in mvbox, next state is STAY %s" % (normalized_messageid(msg)))
            return DC_CONSTANT_MSG_MOVESTATE_STAY
        last_dc_count = 0
        while 1:
            last_dc_count = (last_dc_count + 1) if is_dc_message(msg) else 0
            in_reply_to = normalized_messageid(msg.get("In-Reply-To", ""))
            if not in_reply_to:
                type_msg = "DC" if last_dc_count else "CLEAR"
                self.log("detected thread-start %s message" % type_msg, normalized_messageid(msg))
                if last_dc_count > 0:
                    return DC_CONSTANT_MSG_MOVESTATE_MOVING
                else:
                    return DC_CONSTANT_MSG_MOVESTATE_STAY

            newmsg = self.get_message_from_db(in_reply_to)
            if not newmsg:
                self.log("failed to fetch from db:", in_reply_to)
                # we don't have the parent message ... maybe because
                # it hasn't arrived (yet), was deleted or we failed to
                # scan/fetch it:
                if last_dc_count >= 4:
                    self.log("no thread-start found, but last 4 messages were DC")
                    return DC_CONSTANT_MSG_MOVESTATE_MOVING
                else:
                    self.log("pending: missing parent, last_dc_count=%x" % (last_dc_count, ))
                    return DC_CONSTANT_MSG_MOVESTATE_PENDING
            elif newmsg.move_state == DC_CONSTANT_MSG_MOVESTATE_MOVING:
                self.log("parent was a moved message")
                return DC_CONSTANT_MSG_MOVESTATE_MOVING
            else:
                msg = newmsg
        assert 0, "should never arrive here"

    def schedule_move(self, msg):
        message_id = normalized_messageid(msg)
        assert msg.foldername != MVBOX
        self.log("scheduling move message-id=%s" % (message_id))
        self.pending_imap_jobs = True

    def has_message(self, message_id):
        assert isinstance(message_id, str)
        return message_id in self.db_messages

    def get_message_from_db(self, message_id):
        return self.db_messages.get(normalized_messageid(message_id))

    def store_message(self, message_id, msg):
        mid2 = normalized_messageid(msg)
        message_id = normalized_messageid(message_id)
        assert message_id == mid2
        assert message_id not in self.db_messages, message_id
        assert msg.foldername in (MVBOX, SENT, INBOX)
        self.db_messages[message_id] = msg
        self.log("stored new message message-id=%s" % (message_id,))

    def forget_about_too_old_pending_messages(self):
        # some housekeeping but not sure if neccessary
        # because the involved sql-statements
        # probably don't care if there are some foreever-pending messages
        now = time.time()
        for dbmid, dbmsg in self.db_messages.items():
            if dbmsg.move_state == DC_CONSTANT_MSG_MOVESTATE_PENDING:
                delay = now - dbmsg.fetch_retrieve_time
                if delay > self.pendingtimeout:
                    dbmsg.move_state = DC_CONSTANT_MSG_MOVESTATE_STAY
                    self.log("pendingtimeout: message now set to stay", dbmid)

    def perform_imap_jobs(self):
        with self.wlog("perform_imap_jobs()"):
            if self.foldername in (INBOX, SENT):
                to_move_uids = []
                to_move_msgs = []

                # determine all uids of messages that are to be moved
                for dbmid, dbmsg in self.db_messages.items():
                    if dbmsg.move_state == DC_CONSTANT_MSG_MOVESTATE_MOVING:
                        if dbmsg.uid > 0:  # else it's already moved?
                            to_move_uids.append(dbmsg.uid)
                            to_move_msgs.append(dbmsg)
                if to_move_uids:
                    self.move(to_move_uids)
                # now that we moved let's invalidate "uid" because it's
                # not there anyore in thie folder
                for dbmsg in to_move_msgs:
                    dbmsg.uid = 0
            self.pending_imap_jobs = False

    def _run_in_thread(self):
        self.connect()
        if self.foldername == INBOX:
            # INBOX loop should wait until MVBOX polled once
            mvbox.event_initial_polling_complete.wait()
        now = time.time()
        while True:
            self.perform_imap_jobs()
            self.perform_imap_fetch()
            if self.foldername == MVBOX:
                # signal that MVBOX has polled once
                self.event_initial_polling_complete.set()
            elif self.foldername == INBOX:
                # it's not clear we need to do this housekeeping
                # (depends on the SQL statements)
                self.forget_about_too_old_pending_messages()
            self.perform_imap_idle()

    def start_thread_loop(self):
        assert not self._thread
        self._thread = t = threading.Thread(target=self._run_in_thread)
        t.start()


def repr_msg(msg):
    res = ["message-id: " + str(msg["message-id"]),
           "foldername: " + msg.foldername,
           "uid: " + str(msg.uid),
           ]
    return "\n".join(res)


def is_dc_message(msg):
    return msg and msg.get("Chat-Version")


def normalized_messageid(msg):
    if isinstance(msg, str):
        return msg.lower()
    return msg["Message-ID"].lower()


@click.command(context_settings=dict(help_option_names=["-h", "--help"]))
@click.option("--pendingtimeout", type=int, default=3600,
              help="(default 3600) seconds which a message is still considered for moving "
                   "even though it has no determined thread-start message")
@click.option("--basedir", type=click.Path(),
              default=click.get_app_dir("imap_move_chats"),
              help="directory where database files are stored")
@click.option("-n", "--name", type=str, default=None,
              help="database name (by default derived from login-user)")
@click.argument("imaphost", type=str, required=True)
@click.argument("login-user", type=str, required=True)
@click.argument("login-password", type=str, required=True)
@click.pass_context
def main(context, basedir, name, imaphost, login_user, login_password, pendingtimeout):
    global mvbox
    if not os.path.exists(basedir):
        os.makedirs(basedir)
    if name is None:
        name = login_user
    dbpath = os.path.join(basedir, name) + ".db"
    print("Using dbfile:", dbpath)
    db = PersistentDict(dbpath)
    conn_info = (imaphost, login_user, login_password)
    inbox = ImapConn(db, INBOX, conn_info=conn_info)
    inbox.connect()
    assert 0
    sent = ImapConn(db, SENT, conn_info=conn_info)
    inbox.pendingtimeout = pendingtimeout
    mvbox = ImapConn(db, MVBOX, conn_info=conn_info)
    mvbox.start_thread_loop()
    inbox.start_thread_loop()
    sent.start_thread_loop()


if __name__ == "__main__":
    main()

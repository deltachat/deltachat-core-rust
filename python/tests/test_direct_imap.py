import time
import sys


def test_basic_message_seen(acfactory, tmpdir):
    ac1, ac2 = acfactory.get_two_online_accounts()
    chat12 = acfactory.get_chat(ac1, ac2)

    chat12.send_text("hello")
    msg = ac2._evtracker.wait_next_incoming_message()

    # imap2.dump_imap_structures(tmpdir, logfile=sys.stdout)

    imap2 = acfactory.new_imap_conn(ac2)
    assert imap2.get_unread_cnt() == 1
    imap2.mark_all_read()
    assert imap2.get_unread_cnt() == 0
    imap2.shutdown()


class TestDirectImap:
    def test_mark_read_on_server(self, acfactory, lp):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account(mvbox=True, move=True)

        ac1.wait_configure_finish()
        ac1.start_io()
        ac2.wait_configure_finish()
        ac2.start_io()

        imap2 = acfactory.new_imap_conn(ac2, config_folder="mvbox")
        imap2.mark_all_read()
        assert imap2.get_unread_cnt() == 0

        chat, chat_on_ac2 = acfactory.get_chats(ac1, ac2)

        chat.send_text("Text message")

        incoming_on_ac2 = ac2._evtracker.wait_next_incoming_message()
        lp.sec("Incoming: "+incoming_on_ac2.text)

        assert list(ac2.get_fresh_messages())

        for i in range(0, 20):
            if imap2.get_unread_cnt() == 1:
                break
            time.sleep(1)  # We might need to wait because Imaplib is slower than DC-Core
        assert imap2.get_unread_cnt() == 1

        chat_on_ac2.mark_noticed()
        incoming_on_ac2.mark_seen()
        ac2._evtracker.wait_next_messages_changed()

        assert not list(ac2.get_fresh_messages())

        # The new messages should be seen now.
        for i in range(0, 20):
            if imap2.get_unread_cnt() == 0:
                break
            time.sleep(1)  # We might need to wait because Imaplib is slower than DC-Core
        assert imap2.get_unread_cnt() == 0

    def test_mark_bcc_read_on_server(self, acfactory, lp):
        ac1 = acfactory.get_online_configuring_account(mvbox=True, move=True)
        ac2 = acfactory.get_online_configuring_account()

        ac1.wait_configure_finish()
        ac1.start_io()
        ac2.wait_configure_finish()
        ac2.start_io()

        imap1 = acfactory.new_imap_conn(ac1, config_folder="mvbox")
        imap1.mark_all_read()
        assert imap1.get_unread_cnt() == 0

        chat = acfactory.get_chat(ac1, ac2)

        ac1.set_config("bcc_self", "1")
        chat.send_text("Text message")

        ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

        for i in range(0, 20):
            if imap1.get_new_email_cnt() == 1:
                break
            time.sleep(1)  # We might need to wait because Imaplib is slower than DC-Core
        assert imap1.get_new_email_cnt() == 1

        for i in range(0, 20):
            if imap1.get_unread_cnt() == 0:
                break
            time.sleep(1)  # We might need to wait because Imaplib is slower than DC-Core

        assert imap1.get_unread_cnt() == 0

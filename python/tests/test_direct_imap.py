import sys

from deltachat.direct_imap import SEEN, FLAGS, FETCH


def test_basic_imap_api(acfactory, tmpdir):
    ac1, ac2 = acfactory.get_two_online_accounts()
    chat12 = acfactory.get_chat(ac1, ac2)

    imap2 = acfactory.new_imap_conn(ac2)

    imap2.idle()
    chat12.send_text("hello")
    ac2._evtracker.wait_next_incoming_message()

    imap2.idle_check(terminate=True)
    assert imap2.get_unread_cnt() == 1
    imap2.mark_all_read()
    assert imap2.get_unread_cnt() == 0

    imap2.dump_imap_structures(tmpdir, logfile=sys.stdout)
    imap2.shutdown()


class TestDirectImap:
    def test_mark_read_on_server(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts(move=False)

        imap1 = acfactory.new_imap_conn(ac1, config_folder="inbox")
        assert imap1.get_unread_cnt() == 0
        chat12, chat21 = acfactory.get_chats(ac1, ac2)

        # send a message and check IMAP read flag
        imap1.idle()
        chat21.send_text("Text message")

        msg_in = ac1._evtracker.wait_next_incoming_message()
        assert list(ac1.get_fresh_messages())

        imap1.idle_check()
        msg_in.mark_seen()
        imap1.idle_check(terminate=True)
        assert imap1.get_unread_cnt() == 0

    def test_mark_bcc_read_on_server(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts(move=True)

        imap1_mvbox = acfactory.new_imap_conn(ac1, config_folder="mvbox")

        chat = acfactory.get_chat(ac1, ac2)
        ac1.set_config("bcc_self", "1")
        # wait for seen/read message to appear in mvbox
        imap1_mvbox.idle()
        chat.send_text("Text message")
        ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

        while 1:
            res = imap1_mvbox.idle_check()
            for item in res:
                uid = item[0]
                if item[1] == FETCH:
                    if item[2][0] == FLAGS and SEEN in item[2][1]:
                        return

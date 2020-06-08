import sys

from deltachat.direct_imap import SEEN, FLAGS, FETCH


def test_basic_imap_api(acfactory, tmpdir):
    ac1, ac2 = acfactory.get_two_online_accounts()
    chat12 = acfactory.get_chat(ac1, ac2)

    imap2 = ac2.direct_imap

    ac2.direct_imap.idle_start()
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

        chat12, chat21 = acfactory.get_chats(ac1, ac2)

        # send a message and check IMAP read flag
        ac1.direct_imap.idle_start()
        chat21.send_text("Text message")

        msg_in = ac1._evtracker.wait_next_incoming_message()
        assert list(ac1.get_fresh_messages())

        ac1.direct_imap.idle_check()
        msg_in.mark_seen()
        ac1.direct_imap.idle_check(terminate=True)
        assert ac1.direct_imap.get_unread_cnt() == 0

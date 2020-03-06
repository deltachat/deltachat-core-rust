
import threading
import pytest
import py
import echo_and_quit
import group_tracking


@pytest.fixture(scope='session')
def datadir():
    """The py.path.local object of the test-data/ directory."""
    for path in reversed(py.path.local(__file__).parts()):
        datadir = path.join('test-data')
        if datadir.isdir():
            return datadir
    else:
        pytest.skip('test-data directory not found')


def test_echo_quit_plugin(acfactory):
    bot_ac, bot_cfg = acfactory.get_online_config()

    def run_bot():
        print("*"*20 + " starting bot")
        print("*"*20 + " bot_ac.dbpath", bot_ac.db_path)
        echo_and_quit.main([
            "echo",
            "--show-ffi",
            "--db", bot_ac.db_path,
            "--email", bot_cfg["addr"],
            "--password", bot_cfg["mail_pw"],
        ])

    t = threading.Thread(target=run_bot)
    t.start()

    ac1 = acfactory.get_one_online_account()
    bot_contact = ac1.create_contact(bot_cfg["addr"])
    ch1 = ac1.create_chat_by_contact(bot_contact)
    ch1.send_text("hello")
    reply = ac1._evtracker.wait_next_incoming_message()
    assert "hello" in reply.text
    ch1.send_text("/quit")
    t.join()


@pytest.mark.skip(reason="not implemented")
def test_group_tracking_plugin(acfactory):
    bot_ac, bot_cfg = acfactory.get_online_config()

    def run_bot():
        print("*"*20 + " starting bot")
        print("*"*20 + " bot_ac.dbpath", bot_ac.db_path)
        group_tracking.main([
            "group-tracking",
            "--show-ffi", bot_ac.db_path,
            "--db", bot_ac.db_path,
            "--email", bot_cfg["addr"],
            "--password", bot_cfg["mail_pw"],
        ])

    t = threading.Thread(target=run_bot)
    t.setDaemon(1)
    t.start()

    ac1 = acfactory.get_one_online_account()
    bot_contact = ac1.create_contact(bot_cfg["addr"])
    ch1 = ac1.create_chat_by_contact(bot_contact)
    ch1.send_text("hello")
    ch1.add_contact(ac1.create_contact("x@example.org"))

    # XXX wait for bot to receive things
    t.join()


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
    botproc = acfactory.run_bot_process(echo_and_quit)

    ac1 = acfactory.get_one_online_account()
    bot_contact = ac1.create_contact(botproc.addr)
    ch1 = ac1.create_chat_by_contact(bot_contact)
    ch1.send_text("hello")
    reply = ac1._evtracker.wait_next_incoming_message()
    assert "hello" in reply.text
    assert reply.chat == ch1
    ch1.send_text("/quit")
    botproc.wait()


@pytest.mark.skip(reason="botproc-matching not implementing")
def test_group_tracking_plugin(acfactory):
    botproc = acfactory.run_bot_process(group_tracking)

    ac1 = acfactory.get_one_online_account()
    bot_contact = ac1.create_contact(botproc.addr)
    ch1 = ac1.create_group_chat("bot test group")
    ch1.add_contact(bot_contact)
    ch1.send_text("hello")
    ch1.add_contact(ac1.create_contact("x@example.org"))

    botproc.fnmatch_lines("""
        *member_added x@example.org*
    """)

    botproc.kill()

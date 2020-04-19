
import pytest
import py
import echo_and_quit
import group_tracking
from deltachat.eventlogger import FFIEventLogger


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


def test_group_tracking_plugin(acfactory, lp):
    lp.sec("creating one group-tracking bot and two temp accounts")
    botproc = acfactory.run_bot_process(group_tracking, ffi=False)

    ac1, ac2 = acfactory.get_two_online_accounts(quiet=True)

    botproc.fnmatch_lines("""
        *ac_configure_completed*
    """)
    ac1.add_account_plugin(FFIEventLogger(ac1, "ac1"))
    ac2.add_account_plugin(FFIEventLogger(ac2, "ac2"))

    lp.sec("creating bot test group with bot")
    bot_contact = ac1.create_contact(botproc.addr)
    ch = ac1.create_group_chat("bot test group")
    ch.add_contact(bot_contact)
    ch.send_text("hello")

    botproc.fnmatch_lines("""
        *ac_chat_modified*bot test group*
    """.format(ac1.get_config("addr")))

    lp.sec("adding third member {}".format(ac2.get_config("addr")))
    contact3 = ac1.create_contact(ac2.get_config("addr"))
    ch.add_contact(contact3)

    reply = ac1._evtracker.wait_next_incoming_message()
    assert "hello" in reply.text

    lp.sec("now looking at what the bot received")
    botproc.fnmatch_lines("""
        *ac_member_added {}*
    """.format(contact3.addr))

    lp.sec("contact successfully added, now removing")
    ch.remove_contact(contact3)
    botproc.fnmatch_lines("""
        *ac_member_removed {}*
    """.format(contact3.addr))

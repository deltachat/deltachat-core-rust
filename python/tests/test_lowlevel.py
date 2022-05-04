from __future__ import print_function

from queue import Queue
from deltachat import capi, cutil, const
from deltachat import register_global_plugin
from deltachat.hookspec import global_hookimpl
from deltachat.capi import ffi
from deltachat.capi import lib
from deltachat.testplugin import ACSetup
# from deltachat.account import EventLogger


class TestACSetup:
    def test_basic_states(self, acfactory, monkeypatch):
        pc = ACSetup(init_time=0.0)
        acc = acfactory.get_unconfigured_account()
        monkeypatch.setattr(acc, "configure", lambda **kwargs: None)
        pc.start_configure(acc)
        assert pc._account2state[acc] == pc.CONFIGURING
        pc._configured_events.put((acc, True))
        monkeypatch.setattr(pc, "init_direct_imap", lambda *args, **kwargs: None)
        pc.wait_one_configured(acc)
        assert pc._account2state[acc] == pc.CONFIGURED
        monkeypatch.setattr(pc, "_onconfigure_start_io", lambda *args, **kwargs: None)
        pc.bring_online()
        assert pc._account2state[acc] == pc.IDLEREADY

    def test_two_accounts_one_waited_all_started(self, monkeypatch, acfactory):
        pc = ACSetup(init_time=0.0)
        monkeypatch.setattr(pc, "init_direct_imap", lambda *args, **kwargs: None)
        monkeypatch.setattr(pc, "_onconfigure_start_io", lambda *args, **kwargs: None)
        ac1 = acfactory.get_unconfigured_account()
        monkeypatch.setattr(ac1, "configure", lambda **kwargs: None)
        pc.start_configure(ac1)
        ac2 = acfactory.get_unconfigured_account()
        monkeypatch.setattr(ac2, "configure", lambda **kwargs: None)
        pc.start_configure(ac2)
        assert pc._account2state[ac1] == pc.CONFIGURING
        assert pc._account2state[ac2] == pc.CONFIGURING
        pc._configured_events.put((ac1, True))
        pc.wait_one_configured(ac1)
        assert pc._account2state[ac1] == pc.CONFIGURED
        assert pc._account2state[ac2] == pc.CONFIGURING
        pc._configured_events.put((ac2, True))
        pc.bring_online()
        assert pc._account2state[ac1] == pc.IDLEREADY
        assert pc._account2state[ac2] == pc.IDLEREADY


def test_empty_context():
    ctx = capi.lib.dc_context_new(capi.ffi.NULL, capi.ffi.NULL, capi.ffi.NULL)
    capi.lib.dc_context_unref(ctx)


def test_dc_close_events(tmpdir, acfactory):
    ac1 = acfactory.get_unconfigured_account()

    # register after_shutdown function
    shutdowns = Queue()

    class ShutdownPlugin:
        @global_hookimpl
        def dc_account_after_shutdown(self, account):
            assert account._dc_context is None
            shutdowns.put(account)
    register_global_plugin(ShutdownPlugin())
    assert hasattr(ac1, "_dc_context")
    ac1.shutdown()
    shutdowns.get(timeout=2)


def test_wrong_db(tmpdir):
    p = tmpdir.join("hello.db")
    # write an invalid database file
    p.write("x123" * 10)

    context = lib.dc_context_new(ffi.NULL, p.strpath.encode("ascii"), ffi.NULL)
    assert not lib.dc_context_is_open(context)


def test_empty_blobdir(tmpdir):
    db_fname = tmpdir.join("hello.db")
    # Apparently some client code expects this to be the same as passing NULL.
    ctx = ffi.gc(
        lib.dc_context_new(ffi.NULL, db_fname.strpath.encode("ascii"), b""),
        lib.dc_context_unref,
    )
    assert ctx != ffi.NULL


def test_event_defines():
    assert const.DC_EVENT_INFO == 100
    assert const.DC_CONTACT_ID_SELF


def test_sig():
    sig = capi.lib.dc_event_has_string_data
    assert not sig(const.DC_EVENT_MSGS_CHANGED)
    assert sig(const.DC_EVENT_INFO)
    assert sig(const.DC_EVENT_WARNING)
    assert sig(const.DC_EVENT_ERROR)
    assert sig(const.DC_EVENT_SMTP_CONNECTED)
    assert sig(const.DC_EVENT_IMAP_CONNECTED)
    assert sig(const.DC_EVENT_SMTP_MESSAGE_SENT)
    assert sig(const.DC_EVENT_IMEX_FILE_WRITTEN)


def test_markseen_invalid_message_ids(acfactory):
    ac1 = acfactory.get_pseudo_configured_account()
    contact1 = ac1.create_contact("some1@example.com", name="some1")
    chat = contact1.create_chat()
    chat.send_text("one messae")
    ac1._evtracker.get_matching("DC_EVENT_MSGS_CHANGED")
    msg_ids = [9]
    lib.dc_markseen_msgs(ac1._dc_context, msg_ids, len(msg_ids))
    ac1._evtracker.ensure_event_not_queued("DC_EVENT_WARNING|DC_EVENT_ERROR")


def test_get_special_message_id_returns_empty_message(acfactory):
    ac1 = acfactory.get_pseudo_configured_account()
    for i in range(1, 10):
        msg = ac1.get_message_by_id(i)
        assert msg.id == 0


def test_provider_info_none():
    ctx = ffi.gc(
        lib.dc_context_new(ffi.NULL, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    assert lib.dc_provider_new_from_email(ctx, cutil.as_dc_charpointer("email@unexistent.no")) == ffi.NULL


def test_get_info_open(tmpdir):
    db_fname = tmpdir.join("test.db")
    ctx = ffi.gc(
        lib.dc_context_new(ffi.NULL, db_fname.strpath.encode("ascii"), ffi.NULL),
        lib.dc_context_unref,
    )
    info = cutil.from_dc_charpointer(lib.dc_get_info(ctx))
    assert 'deltachat_core_version' in info
    assert 'database_dir' in info


def test_logged_hook_failure(acfactory):
    ac1 = acfactory.get_pseudo_configured_account()
    cap = []
    ac1.log = cap.append
    with ac1._event_thread.swallow_and_log_exception("some"):
        0/0
    assert cap
    assert "some" in str(cap)
    assert "ZeroDivisionError" in str(cap)
    assert "Traceback" in str(cap)

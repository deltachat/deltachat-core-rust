from __future__ import print_function

from queue import Queue
from deltachat import capi, cutil, const
from deltachat import register_global_plugin
from deltachat.hookspec import global_hookimpl
from deltachat.capi import ffi
from deltachat.capi import lib
# from deltachat.account import EventLogger


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

    assert ffi.NULL == lib.dc_context_new(ffi.NULL, p.strpath.encode("ascii"), ffi.NULL)


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
    ac1 = acfactory.get_configured_offline_account()

    contact1 = ac1.create_contact("some1@example.com", name="some1")
    chat = contact1.create_chat()
    chat.send_text("one messae")
    ac1._evtracker.get_matching("DC_EVENT_MSGS_CHANGED")
    msg_ids = [9]
    lib.dc_markseen_msgs(ac1._dc_context, msg_ids, len(msg_ids))
    ac1._evtracker.ensure_event_not_queued("DC_EVENT_WARNING|DC_EVENT_ERROR")


def test_get_special_message_id_returns_empty_message(acfactory):
    ac1 = acfactory.get_configured_offline_account()
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

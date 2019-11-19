from __future__ import print_function
from deltachat import capi, cutil, const, set_context_callback, clear_context_callback
from deltachat.capi import ffi
from deltachat.capi import lib
from deltachat.account import EventLogger


def test_empty_context():
    ctx = capi.lib.dc_context_new(capi.ffi.NULL, capi.ffi.NULL, capi.ffi.NULL)
    capi.lib.dc_close(ctx)


def test_callback_None2int():
    ctx = capi.lib.dc_context_new(capi.lib.py_dc_callback, ffi.NULL, ffi.NULL)
    set_context_callback(ctx, lambda *args: None)
    capi.lib.dc_close(ctx)
    clear_context_callback(ctx)


def test_dc_close_events(tmpdir):
    ctx = ffi.gc(
        capi.lib.dc_context_new(capi.lib.py_dc_callback, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    evlog = EventLogger(ctx)
    evlog.set_timeout(5)
    set_context_callback(
        ctx,
        lambda ctx, evt_name, data1, data2: evlog(evt_name, data1, data2)
    )
    p = tmpdir.join("hello.db")
    lib.dc_open(ctx, p.strpath.encode("ascii"), ffi.NULL)
    capi.lib.dc_close(ctx)

    def find(info_string):
        while 1:
            ev = evlog.get_matching("DC_EVENT_INFO", check_error=False)
            data2 = ev[2]
            if info_string in data2:
                return
            else:
                print("skipping event", *ev)

    find("disconnecting inbox-thread")
    find("disconnecting sentbox-thread")
    find("disconnecting mvbox-thread")
    find("disconnecting SMTP")
    find("Database closed")


def test_wrong_db(tmpdir):
    dc_context = ffi.gc(
        lib.dc_context_new(lib.py_dc_callback, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    p = tmpdir.join("hello.db")
    # write an invalid database file
    p.write("x123" * 10)
    assert not lib.dc_open(dc_context, p.strpath.encode("ascii"), ffi.NULL)


def test_empty_blobdir(tmpdir):
    # Apparently some client code expects this to be the same as passing NULL.
    ctx = ffi.gc(
        lib.dc_context_new(lib.py_dc_callback, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    db_fname = tmpdir.join("hello.db")
    assert lib.dc_open(ctx, db_fname.strpath.encode("ascii"), b"")


def test_event_defines():
    assert const.DC_EVENT_INFO == 100
    assert const.DC_CONTACT_ID_SELF


def test_sig():
    sig = capi.lib.dc_get_event_signature_types
    assert sig(const.DC_EVENT_INFO) == 2
    assert sig(const.DC_EVENT_WARNING) == 2
    assert sig(const.DC_EVENT_ERROR) == 2
    assert sig(const.DC_EVENT_SMTP_CONNECTED) == 2
    assert sig(const.DC_EVENT_IMAP_CONNECTED) == 2
    assert sig(const.DC_EVENT_SMTP_MESSAGE_SENT) == 2


def test_markseen_invalid_message_ids(acfactory):
    ac1 = acfactory.get_configured_offline_account()
    contact1 = ac1.create_contact(email="some1@example.com", name="some1")
    chat = ac1.create_chat_by_contact(contact1)
    chat.send_text("one messae")
    ac1._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
    msg_ids = [9]
    lib.dc_markseen_msgs(ac1._dc_context, msg_ids, len(msg_ids))
    ac1._evlogger.ensure_event_not_queued("DC_EVENT_WARNING|DC_EVENT_ERROR")


def test_get_special_message_id_returns_empty_message(acfactory):
    ac1 = acfactory.get_configured_offline_account()
    for i in range(1, 10):
        msg = ac1.get_message_by_id(i)
        assert msg.id == 0


def test_provider_info():
    provider = lib.dc_provider_new_from_email(cutil.as_dc_charpointer("ex@example.com"))
    assert cutil.from_dc_charpointer(
        lib.dc_provider_get_overview_page(provider)
    ) == "https://providers.delta.chat/example.com"
    assert cutil.from_dc_charpointer(lib.dc_provider_get_name(provider)) == "Example"
    assert cutil.from_dc_charpointer(lib.dc_provider_get_markdown(provider)) == "\n..."
    assert cutil.from_dc_charpointer(lib.dc_provider_get_status_date(provider)) == "2018-09"
    assert lib.dc_provider_get_status(provider) == const.DC_PROVIDER_STATUS_PREPARATION


def test_provider_info_none():
    assert lib.dc_provider_new_from_email(cutil.as_dc_charpointer("email@unexistent.no")) == ffi.NULL


def test_get_info_closed():
    ctx = ffi.gc(
        lib.dc_context_new(lib.py_dc_callback, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    info = cutil.from_dc_charpointer(lib.dc_get_info(ctx))
    assert 'deltachat_core_version' in info
    assert 'database_dir' not in info


def test_get_info_open(tmpdir):
    ctx = ffi.gc(
        lib.dc_context_new(lib.py_dc_callback, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    db_fname = tmpdir.join("test.db")
    lib.dc_open(ctx, db_fname.strpath.encode("ascii"), ffi.NULL)
    info = cutil.from_dc_charpointer(lib.dc_get_info(ctx))
    assert 'deltachat_core_version' in info
    assert 'database_dir' in info


def test_is_open_closed():
    ctx = ffi.gc(
        lib.dc_context_new(lib.py_dc_callback, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    assert lib.dc_is_open(ctx) == 0


def test_is_open_actually_open(tmpdir):
    ctx = ffi.gc(
        lib.dc_context_new(lib.py_dc_callback, ffi.NULL, ffi.NULL),
        lib.dc_context_unref,
    )
    db_fname = tmpdir.join("test.db")
    lib.dc_open(ctx, db_fname.strpath.encode("ascii"), ffi.NULL)
    assert lib.dc_is_open(ctx) == 1

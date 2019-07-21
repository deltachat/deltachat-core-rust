from __future__ import print_function
from deltachat import capi, const, set_context_callback, clear_context_callback
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


def test_dc_close_events():
    ctx = capi.lib.dc_context_new(capi.lib.py_dc_callback, ffi.NULL, ffi.NULL)
    evlog = EventLogger(ctx)
    evlog.set_timeout(5)
    set_context_callback(ctx, lambda ctx, evt_name, data1, data2: evlog(evt_name, data1, data2))
    capi.lib.dc_close(ctx)

    def find(info_string):
        while 1:
            ev = evlog.get_matching("DC_EVENT_INFO", check_error=False)
            data2 = ev[2]
            if info_string in data2:
                return
            else:
                print("skipping event", *ev)

    find("disconnecting INBOX-watch")
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

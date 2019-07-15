from __future__ import print_function
import pytest
from deltachat import capi, Account, const, set_context_callback
from deltachat.capi import ffi
from deltachat.cutil import as_dc_charpointer
from deltachat.account import EventLogger


def test_empty_context():
    ctx = capi.lib.dc_context_new(capi.ffi.NULL, capi.ffi.NULL, capi.ffi.NULL)
    capi.lib.dc_close(ctx)

def test_dc_close_events():
    ctx = capi.lib.dc_context_new(capi.lib.py_dc_callback, ffi.NULL, ffi.NULL)
    evlog = EventLogger(ctx)
    evlog.set_timeout(5)
    set_context_callback(ctx, lambda ctx, evt_name, data1, data2: evlog(evt_name, data1, data2))
    capi.lib.dc_close(ctx)
    # test that we get events from dc_close
    print(evlog.get_matching("DC_EVENT_INFO", check_error=False))
    print(evlog.get_matching("DC_EVENT_INFO", check_error=False))
    print(evlog.get_matching("DC_EVENT_INFO", check_error=False))
    print(evlog.get_matching("DC_EVENT_INFO", check_error=False))


def test_wrong_db(tmpdir):
    tmpdir.join("hello.db").write("123")
    with pytest.raises(ValueError):
        Account(db_path=tmpdir.strpath)


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

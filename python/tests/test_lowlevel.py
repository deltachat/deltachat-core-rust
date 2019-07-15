from __future__ import print_function
import pytest
from deltachat import capi, Account, const, set_context_callback
from deltachat.cutil import as_dc_charpointer
from queue import Queue


def test_empty_context():
    ctx = capi.lib.dc_context_new(capi.ffi.NULL, capi.ffi.NULL, capi.ffi.NULL)
    capi.lib.dc_close(ctx)

def test_set_context():
    ctx = capi.lib.dc_context_new(capi.ffi.NULL, capi.ffi.NULL, capi.ffi.NULL)

    q = Queue()
    set_context_callback(ctx, lambda *args: q.put(args))

    name = as_dc_charpointer("ein")
    email = as_dc_charpointer("ein@kontakt.org")
    contact_id = capi.lib.dc_create_contact(ctx, name, email)
    capi.lib.dc_close(ctx)
    q.get(timeout=10)


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

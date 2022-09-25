from __future__ import print_function

import os
import time
from datetime import datetime, timedelta, timezone

import pytest

from deltachat import Account, const
from deltachat.capi import ffi, lib
from deltachat.cutil import iter_array
from deltachat.hookspec import account_hookimpl
from deltachat.message import Message
from deltachat.tracker import ImexFailed


@pytest.mark.parametrize(
    "msgtext,res",
    [
        (
            "Member Me (tmp1@x.org) removed by tmp2@x.org.",
            ("removed", "tmp1@x.org", "tmp2@x.org"),
        ),
        (
            "Member With space (tmp1@x.org) removed by tmp2@x.org.",
            ("removed", "tmp1@x.org", "tmp2@x.org"),
        ),
        (
            "Member With space (tmp1@x.org) removed by Another member (tmp2@x.org).",
            ("removed", "tmp1@x.org", "tmp2@x.org"),
        ),
        (
            "Member With space (tmp1@x.org) removed by me",
            ("removed", "tmp1@x.org", "me"),
        ),
        (
            "Group left by some one (tmp1@x.org).",
            ("removed", "tmp1@x.org", "tmp1@x.org"),
        ),
        ("Group left by tmp1@x.org.", ("removed", "tmp1@x.org", "tmp1@x.org")),
        (
            "Member tmp1@x.org added by tmp2@x.org.",
            ("added", "tmp1@x.org", "tmp2@x.org"),
        ),
        ("Member nothing bla bla", None),
        ("Another unknown system message", None),
    ],
)
def test_parse_system_add_remove(msgtext, res):
    from deltachat.message import parse_system_add_remove

    out = parse_system_add_remove(msgtext)
    assert out == res


class TestOfflineAccountBasic:
    def test_wrong_db(self, tmpdir):
        p = tmpdir.join("hello.db")
        p.write("123")
        account = Account(p.strpath)
        assert not account.is_open()

    def test_os_name(self, tmpdir):
        p = tmpdir.join("hello.db")
        # we can't easily test if os_name is used in X-Mailer
        # outgoing messages without a full Online test
        # but we at least check Account accepts the arg
        ac1 = Account(p.strpath, os_name="solarpunk")
        ac1.get_info()

    def test_preconfigure_keypair(self, acfactory, data):
        ac = acfactory.get_unconfigured_account()
        alice_public = data.read_path("key/alice-public.asc")
        alice_secret = data.read_path("key/alice-secret.asc")
        assert alice_public and alice_secret
        ac._preconfigure_keypair("alice@example.org", alice_public, alice_secret)

    def test_getinfo(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        d = ac1.get_info()
        assert d["arch"]
        assert d["number_of_chats"] == "0"
        assert d["bcc_self"] == "1"

    def test_is_not_configured(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        assert not ac1.is_configured()
        with pytest.raises(ValueError):
            ac1.check_is_configured()

    def test_wrong_config_keys(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        with pytest.raises(KeyError):
            ac1.set_config("lqkwje", "value")
        with pytest.raises(KeyError):
            ac1.get_config("lqkwje")

    def test_set_config_int_conversion(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        ac1.set_config("mvbox_move", False)
        assert ac1.get_config("mvbox_move") == "0"
        ac1.set_config("mvbox_move", True)
        assert ac1.get_config("mvbox_move") == "1"
        ac1.set_config("mvbox_move", 0)
        assert ac1.get_config("mvbox_move") == "0"
        ac1.set_config("mvbox_move", 1)
        assert ac1.get_config("mvbox_move") == "1"

    def test_update_config(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        ac1.update_config(dict(mvbox_move=False))
        assert ac1.get_config("mvbox_move") == "0"

    def test_has_savemime(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        assert "save_mime_headers" in ac1.get_config("sys.config_keys").split()

    def test_has_bccself(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        assert "bcc_self" in ac1.get_config("sys.config_keys").split()
        assert ac1.get_config("bcc_self") == "1"

    def test_selfcontact_if_unconfigured(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        assert not ac1.get_self_contact().addr

    def test_selfcontact_configured(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        me = ac1.get_self_contact()
        assert me.display_name
        assert me.addr

    def test_get_config_fails(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        with pytest.raises(KeyError):
            ac1.get_config("123123")

    def test_empty_group_bcc_self_enabled(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        ac1.set_config("bcc_self", "1")
        chat = ac1.create_group_chat(name="group1")
        msg = chat.send_text("msg1")
        assert msg in chat.get_messages()

    def test_empty_group_bcc_self_disabled(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        ac1.set_config("bcc_self", "0")
        chat = ac1.create_group_chat(name="group1")
        msg = chat.send_text("msg1")
        assert msg in chat.get_messages()


class TestOfflineContact:
    def test_contact_attr(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        contact1 = ac1.create_contact("some1@example.org", name="some1")
        contact2 = ac1.create_contact("some1@example.org", name="some1")
        contact3 = None
        str(contact1)
        repr(contact1)
        assert contact1 == contact2
        assert contact1 != contact3
        assert contact1.id
        assert contact1.addr == "some1@example.org"
        assert contact1.display_name == "some1"
        assert not contact1.is_blocked()
        assert not contact1.is_verified()

    def test_get_blocked(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        contact1 = ac1.create_contact("some1@example.org", name="some1")
        contact2 = ac1.create_contact("some2@example.org", name="some2")
        ac1.create_contact("some3@example.org", name="some3")
        assert ac1.get_blocked_contacts() == []
        contact1.block()
        assert ac1.get_blocked_contacts() == [contact1]
        contact2.block()
        blocked = ac1.get_blocked_contacts()
        assert len(blocked) == 2 and contact1 in blocked and contact2 in blocked
        contact2.unblock()
        assert ac1.get_blocked_contacts() == [contact1]

    def test_create_self_contact(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        contact1 = ac1.create_contact(ac1.get_config("addr"))
        assert contact1.id == 1

    def test_get_contacts_and_delete(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        contact1 = ac1.create_contact("some1@example.org", name="some1")
        contacts = ac1.get_contacts()
        assert len(contacts) == 1
        assert contact1 in contacts

        assert not ac1.get_contacts(query="some2")
        assert ac1.get_contacts(query="some1")
        assert not ac1.get_contacts(only_verified=True)
        assert len(ac1.get_contacts(with_self=True)) == 2

        assert ac1.delete_contact(contact1)
        assert contact1 not in ac1.get_contacts()

    def test_get_contacts_and_delete_fails(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        contact1 = ac1.create_contact("some1@example.com", name="some1")
        msg = contact1.create_chat().send_text("one message")
        assert not ac1.delete_contact(contact1)
        assert not msg.filemime

    def test_create_chat_flexibility(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        ac2 = acfactory.get_pseudo_configured_account()
        chat1 = ac1.create_chat(ac2)
        chat2 = ac1.create_chat(ac2.get_self_contact().addr)
        assert chat1 == chat2
        ac3 = acfactory.get_unconfigured_account()
        with pytest.raises(ValueError):
            ac1.create_chat(ac3)

    def test_contact_rename(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        contact = ac1.create_contact("some1@example.com", name="some1")
        chat = ac1.create_chat(contact)
        assert chat.get_name() == "some1"
        ac1.create_contact("some1@example.com", name="renamed")
        ev = ac1._evtracker.get_matching("DC_EVENT_CHAT_MODIFIED")
        assert ev.data1 == chat.id
        assert chat.get_name() == "renamed"


class TestOfflineChat:
    @pytest.fixture
    def ac1(self, acfactory):
        return acfactory.get_pseudo_configured_account()

    @pytest.fixture
    def chat1(self, ac1):
        return ac1.create_contact("some1@example.org", name="some1").create_chat()

    def test_display(self, chat1):
        str(chat1)
        repr(chat1)

    def test_is_group(self, chat1):
        assert not chat1.is_group()

    def test_chat_by_id(self, chat1):
        chat2 = chat1.account.get_chat_by_id(chat1.id)
        assert chat2 == chat1
        with pytest.raises(ValueError):
            chat1.account.get_chat_by_id(123123)

    def test_chat_idempotent(self, chat1, ac1):
        contact1 = chat1.get_contacts()[0]
        chat2 = contact1.create_chat()
        chat3 = None
        assert chat2.id == chat1.id
        assert chat2.get_name() == chat1.get_name()
        assert chat1 == chat2
        assert not (chat1 != chat2)
        assert chat1 != chat3

        for ichat in ac1.get_chats():
            if ichat.id == chat1.id:
                break
        else:
            pytest.fail("could not find chat")

    def test_group_chat_add_second_account(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        ac2 = acfactory.get_pseudo_configured_account()
        chat = ac1.create_group_chat(name="title1")
        with pytest.raises(ValueError):
            chat.add_contact(ac2.get_self_contact())
        contact = chat.add_contact(ac2)
        assert contact.addr == ac2.get_config("addr")
        assert contact.name == ac2.get_config("displayname")
        assert contact.account == ac1
        chat.remove_contact(ac2)

    def test_group_chat_creation(self, ac1):
        contact1 = ac1.create_contact("some1@example.org", name="some1")
        contact2 = ac1.create_contact("some2@example.org", name="some2")
        chat = ac1.create_group_chat(name="title1", contacts=[contact1, contact2])
        assert chat.get_name() == "title1"
        assert contact1 in chat.get_contacts()
        assert contact2 in chat.get_contacts()
        assert not chat.is_promoted()
        chat.set_name("title2")
        assert chat.get_name() == "title2"

        d = chat.get_summary()
        print(d)
        assert d["id"] == chat.id
        assert d["type"] == chat.get_type()
        assert d["name"] == chat.get_name()
        assert d["archived"] == chat.is_archived()
        # assert d["param"] == chat.param
        assert d["color"] == chat.get_color()
        assert d["profile_image"] == "" if chat.get_profile_image() is None else chat.get_profile_image()
        assert d["draft"] == "" if chat.get_draft() is None else chat.get_draft()

    def test_group_chat_creation_with_translation(self, ac1):
        ac1.set_stock_translation(const.DC_STR_GROUP_NAME_CHANGED_BY_YOU, "abc %1$s xyz %2$s")
        ac1._evtracker.consume_events()
        with pytest.raises(ValueError):
            ac1.set_stock_translation(const.DC_STR_FILE, "xyz %1$s")
        ac1._evtracker.get_matching("DC_EVENT_WARNING")
        with pytest.raises(ValueError):
            ac1.set_stock_translation(const.DC_STR_CONTACT_NOT_VERIFIED, "xyz %2$s")
        ac1._evtracker.get_matching("DC_EVENT_WARNING")
        with pytest.raises(ValueError):
            ac1.set_stock_translation(500, "xyz %1$s")
        ac1._evtracker.get_matching("DC_EVENT_WARNING")
        chat = ac1.create_group_chat(name="homework", contacts=[])
        assert chat.get_name() == "homework"
        chat.send_text("Now we have a group for homework")
        assert chat.is_promoted()
        chat.set_name("Homework")
        assert chat.get_messages()[-1].text == "abc homework xyz Homework"

    @pytest.mark.parametrize("verified", [True, False])
    def test_group_chat_qr(self, acfactory, ac1, verified):
        ac2 = acfactory.get_pseudo_configured_account()
        chat = ac1.create_group_chat(name="title1", verified=verified)
        assert chat.is_group()
        qr = chat.get_join_qr()
        assert ac2.check_qr(qr).is_ask_verifygroup

    def test_removing_blocked_user_from_group(self, ac1, lp):
        """
        Test that blocked contact is not unblocked when removed from a group.
        See https://github.com/deltachat/deltachat-core-rust/issues/2030
        """
        lp.sec("Create a group chat with a contact")
        contact = ac1.create_contact("some1@example.org")
        group = ac1.create_group_chat("title", contacts=[contact])
        group.send_text("First group message")

        lp.sec("ac1 blocks contact")
        contact.block()
        assert contact.is_blocked()

        lp.sec("ac1 removes contact from their group")
        group.remove_contact(contact)
        assert contact.is_blocked()

        lp.sec("ac1 adding blocked contact unblocks it")
        group.add_contact(contact)
        assert not contact.is_blocked()

    def test_get_set_profile_image_simple(self, ac1, data):
        chat = ac1.create_group_chat(name="title1")
        p = data.get_path("d.png")
        chat.set_profile_image(p)
        p2 = chat.get_profile_image()
        assert open(p, "rb").read() == open(p2, "rb").read()
        chat.remove_profile_image()
        assert chat.get_profile_image() is None

    def test_mute(self, ac1):
        chat = ac1.create_group_chat(name="title1")
        assert not chat.is_muted()
        assert chat.get_mute_duration() == 0
        chat.mute()
        assert chat.is_muted()
        assert chat.get_mute_duration() == -1
        chat.unmute()
        assert not chat.is_muted()
        chat.mute(50)
        assert chat.is_muted()
        assert chat.get_mute_duration() <= 50
        with pytest.raises(ValueError):
            chat.mute(-51)

        # Regression test, this caused Rust panic previously
        chat.mute(2**63 - 1)
        assert chat.is_muted()
        assert chat.get_mute_duration() == -1

    def test_delete_and_send_fails(self, ac1, chat1):
        chat1.delete()
        ac1._evtracker.wait_next_messages_changed()
        with pytest.raises(ValueError):
            chat1.send_text("msg1")

    def test_prepare_message_and_send(self, ac1, chat1):
        msg = chat1.prepare_message(Message.new_empty(chat1.account, "text"))
        msg.set_text("hello world")
        assert msg.text == "hello world"
        assert msg.id > 0
        chat1.send_prepared(msg)
        assert "Sent" in msg.get_message_info()
        str(msg)
        repr(msg)
        assert msg == ac1.get_message_by_id(msg.id)

    def test_prepare_file(self, ac1, chat1):
        blobdir = ac1.get_blobdir()
        p = os.path.join(blobdir, "somedata.txt")
        with open(p, "w") as f:
            f.write("some data")
        message = chat1.prepare_message_file(p)
        assert message.id > 0
        message.set_text("hello world")
        assert message.is_out_preparing()
        assert message.text == "hello world"
        chat1.send_prepared(message)
        assert "Sent" in message.get_message_info()

    def test_message_eq_contains(self, chat1):
        msg = chat1.send_text("msg1")
        msg2 = None
        assert msg != msg2
        assert msg in chat1.get_messages()
        assert not (msg not in chat1.get_messages())
        str(msg)
        repr(msg)

    def test_message_send_text(self, chat1):
        msg = chat1.send_text("msg1")
        assert msg
        assert msg.is_text()
        assert not msg.is_audio()
        assert not msg.is_video()
        assert not msg.is_gif()
        assert not msg.is_file()
        assert not msg.is_image()

        assert not msg.is_in_fresh()
        assert not msg.is_in_noticed()
        assert not msg.is_in_seen()
        assert msg.is_out_pending()
        assert not msg.is_out_failed()
        assert not msg.is_out_delivered()
        assert not msg.is_out_mdn_received()

    def test_message_image(self, chat1, data, lp):
        with pytest.raises(ValueError):
            chat1.send_image(path="notexists")
        fn = data.get_path("d.png")
        lp.sec("sending image")
        chat1.account._evtracker.consume_events()
        msg = chat1.send_image(fn)
        chat1.account._evtracker.get_matching("DC_EVENT_NEW_BLOB_FILE")
        assert msg.is_image()
        assert msg
        assert msg.id > 0
        assert os.path.exists(msg.filename)
        assert msg.filemime == "image/png"

    @pytest.mark.parametrize(
        "typein,typeout",
        [
            (None, "application/octet-stream"),
            ("text/plain", "text/plain"),
            ("image/png", "image/png"),
        ],
    )
    def test_message_file(self, ac1, chat1, data, lp, typein, typeout):
        lp.sec("sending file")
        fn = data.get_path("r.txt")
        msg = chat1.send_file(fn, typein)
        assert msg
        assert msg.id > 0
        assert msg.is_file()
        assert os.path.exists(msg.filename)
        assert msg.filename.endswith(msg.basename)
        assert msg.filemime == typeout
        msg2 = chat1.send_file(fn, typein)
        assert msg2 != msg
        assert msg2.filename != msg.filename

    def test_create_contact(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        email = "hello <hello@example.org>"
        contact1 = ac1.create_contact(email)
        assert contact1.addr == "hello@example.org"
        assert contact1.name == "hello"
        contact1 = ac1.create_contact(email, name="world")
        assert contact1.name == "world"
        contact2 = ac1.create_contact("display1 <x@example.org>", "real")
        assert contact2.name == "real"

    def test_create_chat_simple(self, acfactory):
        ac1 = acfactory.get_pseudo_configured_account()
        contact1 = ac1.create_contact("some1@example.org", name="some1")
        contact1.create_chat().send_text("hello")

    def test_chat_message_distinctions(self, ac1, chat1):
        past1s = datetime.now(timezone.utc) - timedelta(seconds=1)
        msg = chat1.send_text("msg1")
        ts = msg.time_sent
        assert msg.time_received is None
        assert ts.strftime("Y")
        assert past1s < ts
        contact = msg.get_sender_contact()
        assert contact == ac1.get_self_contact()

    def test_import_export_on_unencrypted_acct(self, acfactory, tmpdir):
        backupdir = tmpdir.mkdir("backup")
        ac1 = acfactory.get_pseudo_configured_account()
        chat = ac1.create_contact("some1 <some1@example.org>").create_chat()
        # send a text message
        msg = chat.send_text("msg1")
        # send a binary file
        bin = tmpdir.join("some.bin")
        with bin.open("w") as f:
            f.write("\00123" * 10000)
        msg = chat.send_file(bin.strpath)
        contact = msg.get_sender_contact()
        assert contact == ac1.get_self_contact()
        assert not backupdir.listdir()
        ac1.stop_io()
        path = ac1.export_all(backupdir.strpath)
        assert os.path.exists(path)
        ac2 = acfactory.get_unconfigured_account()
        ac2.import_all(path)
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@example.org"
        chat2 = contact2.create_chat()
        messages = chat2.get_messages()
        assert len(messages) == 2
        assert messages[0].text == "msg1"
        assert os.path.exists(messages[1].filename)

    def test_import_export_on_encrypted_acct(self, acfactory, tmpdir):
        passphrase1 = "passphrase1"
        passphrase2 = "passphrase2"
        backupdir = tmpdir.mkdir("backup")
        ac1 = acfactory.get_pseudo_configured_account(passphrase=passphrase1)

        chat = ac1.create_contact("some1 <some1@example.org>").create_chat()
        # send a text message
        msg = chat.send_text("msg1")
        # send a binary file
        bin = tmpdir.join("some.bin")
        with bin.open("w") as f:
            f.write("\00123" * 10000)
        msg = chat.send_file(bin.strpath)
        contact = msg.get_sender_contact()
        assert contact == ac1.get_self_contact()

        assert not backupdir.listdir()
        ac1.stop_io()

        path = ac1.export_all(backupdir.strpath)
        assert os.path.exists(path)

        ac2 = acfactory.get_unconfigured_account(closed=True)
        ac2.open(passphrase2)
        ac2.import_all(path)

        # check data integrity
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@example.org"
        chat2 = contact2.create_chat()
        messages = chat2.get_messages()
        assert len(messages) == 2
        assert messages[0].text == "msg1"
        assert os.path.exists(messages[1].filename)

        ac2.shutdown()

        # check that passphrase is not lost after import:
        ac2 = Account(ac2.db_path, logging=ac2._logging, closed=True)
        ac2.open(passphrase2)

        # check data integrity
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@example.org"
        chat2 = contact2.create_chat()
        messages = chat2.get_messages()
        assert len(messages) == 2
        assert messages[0].text == "msg1"
        assert os.path.exists(messages[1].filename)

    def test_import_export_with_passphrase(self, acfactory, tmpdir):
        passphrase = "test_passphrase"
        wrong_passphrase = "wrong_passprase"
        backupdir = tmpdir.mkdir("backup")
        ac1 = acfactory.get_pseudo_configured_account()

        chat = ac1.create_contact("some1 <some1@example.org>").create_chat()
        # send a text message
        msg = chat.send_text("msg1")
        # send a binary file
        bin = tmpdir.join("some.bin")
        with bin.open("w") as f:
            f.write("\00123" * 10000)
        msg = chat.send_file(bin.strpath)
        contact = msg.get_sender_contact()
        assert contact == ac1.get_self_contact()

        assert not backupdir.listdir()
        ac1.stop_io()

        path = ac1.export_all(backupdir.strpath, passphrase)
        assert os.path.exists(path)

        ac2 = acfactory.get_unconfigured_account()
        with pytest.raises(ImexFailed):
            ac2.import_all(path, wrong_passphrase)
        ac2.import_all(path, passphrase)

        # check data integrity
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@example.org"
        chat2 = contact2.create_chat()
        messages = chat2.get_messages()
        assert len(messages) == 2
        assert messages[0].text == "msg1"
        assert os.path.exists(messages[1].filename)

    def test_import_encrypted_bak_into_encrypted_acct(self, acfactory, tmpdir):
        """
        Test that account passphrase isn't lost if backup failed to be imported.
        See https://github.com/deltachat/deltachat-core-rust/issues/3379
        """
        acct_passphrase = "passphrase1"
        bak_passphrase = "passphrase2"
        wrong_passphrase = "wrong_passprase"
        backupdir = tmpdir.mkdir("backup")

        ac1 = acfactory.get_pseudo_configured_account()
        chat = ac1.create_contact("some1 <some1@example.org>").create_chat()
        # send a text message
        msg = chat.send_text("msg1")
        # send a binary file
        bin = tmpdir.join("some.bin")
        with bin.open("w") as f:
            f.write("\00123" * 10000)
        msg = chat.send_file(bin.strpath)
        contact = msg.get_sender_contact()
        assert contact == ac1.get_self_contact()

        assert not backupdir.listdir()
        ac1.stop_io()

        path = ac1.export_all(backupdir.strpath, bak_passphrase)
        assert os.path.exists(path)

        ac2 = acfactory.get_unconfigured_account(closed=True)
        ac2.open(acct_passphrase)
        with pytest.raises(ImexFailed):
            ac2.import_all(path, wrong_passphrase)
        ac2.import_all(path, bak_passphrase)

        # check data integrity
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@example.org"
        chat2 = contact2.create_chat()
        messages = chat2.get_messages()
        assert len(messages) == 2
        assert messages[0].text == "msg1"
        assert os.path.exists(messages[1].filename)

        ac2.shutdown()

        # check that passphrase is not lost after import
        ac2 = Account(ac2.db_path, logging=ac2._logging, closed=True)
        ac2.open(acct_passphrase)

        # check data integrity
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@example.org"
        chat2 = contact2.create_chat()
        messages = chat2.get_messages()
        assert len(messages) == 2
        assert messages[0].text == "msg1"
        assert os.path.exists(messages[1].filename)

    def test_set_get_draft(self, chat1):
        msg = Message.new_empty(chat1.account, "text")
        msg1 = chat1.prepare_message(msg)
        msg1.set_text("hello")
        chat1.set_draft(msg1)
        msg1.set_text("obsolete")
        msg2 = chat1.get_draft()
        assert msg2.text == "hello"
        chat1.set_draft(None)
        assert chat1.get_draft() is None

    def test_qr_setup_contact(self, acfactory, lp):
        ac1 = acfactory.get_pseudo_configured_account()
        ac2 = acfactory.get_pseudo_configured_account()
        qr = ac1.get_setup_contact_qr()
        assert qr.startswith("OPENPGP4FPR:")
        res = ac2.check_qr(qr)
        assert res.is_ask_verifycontact()
        assert not res.is_ask_verifygroup()
        assert res.contact_id == 10

    def test_quote(self, chat1):
        """Offline quoting test"""
        msg = Message.new_empty(chat1.account, "text")
        msg.set_text("Multi\nline\nmessage")
        assert msg.quoted_text is None

        # Prepare message to assign it a Message-Id.
        # Messages without Message-Id cannot be quoted.
        msg = chat1.prepare_message(msg)

        reply_msg = Message.new_empty(chat1.account, "text")
        reply_msg.set_text("reply")
        reply_msg.quote = msg
        assert reply_msg.quoted_text == "Multi\nline\nmessage"

    def test_group_chat_many_members_add_remove(self, ac1, lp):
        lp.sec("ac1: creating group chat with 10 other members")
        chat = ac1.create_group_chat(name="title1")
        # promote chat
        chat.send_text("hello")
        assert chat.is_promoted()

        # activate local plugin
        in_list = []

        class InPlugin:
            @account_hookimpl
            def ac_member_added(self, chat, contact, actor):
                in_list.append(("added", chat, contact, actor))

            @account_hookimpl
            def ac_member_removed(self, chat, contact, actor):
                in_list.append(("removed", chat, contact, actor))

        ac1.add_account_plugin(InPlugin())

        # perform add contact many times
        contacts = []
        for i in range(10):
            lp.sec("create contact")
            contact = ac1.create_contact("some{}@example.org".format(i))
            contacts.append(contact)
            lp.sec("add contact")
            chat.add_contact(contact)

        assert chat.num_contacts() == 11

        # let's make sure the events perform plugin hooks
        def wait_events(cond):
            now = time.time()
            while time.time() < now + 5:
                if cond():
                    break
                time.sleep(0.1)
            else:
                pytest.fail("failed to get events")

        wait_events(lambda: len(in_list) == 10)

        assert len(in_list) == 10
        chat_contacts = chat.get_contacts()
        for in_cmd, in_chat, in_contact, in_actor in in_list:
            assert in_cmd == "added"
            assert in_chat == chat
            assert in_contact in chat_contacts
            assert in_actor is None
            chat_contacts.remove(in_contact)

        assert chat_contacts[0].id == 1  # self contact

        in_list[:] = []

        lp.sec("ac1: removing two contacts and checking things are right")
        chat.remove_contact(contacts[9])
        chat.remove_contact(contacts[3])
        assert chat.num_contacts() == 9

        wait_events(lambda: len(in_list) == 2)
        assert len(in_list) == 2
        assert in_list[0][0] == "removed"
        assert in_list[0][1] == chat
        assert in_list[0][2] == contacts[9]
        assert in_list[1][0] == "removed"
        assert in_list[1][1] == chat
        assert in_list[1][2] == contacts[3]

    def test_audit_log_view_without_daymarker(self, ac1, lp):
        lp.sec("ac1: test audit log (show only system messages)")
        chat = ac1.create_group_chat(name="audit log sample data")
        # promote chat
        chat.send_text("hello")
        assert chat.is_promoted()

        lp.sec("create test data")
        chat.add_contact(ac1.create_contact("some-1@example.org"))
        chat.set_name("audit log test group")
        chat.send_text("a message in between")

        lp.sec("check message count of all messages")
        assert len(chat.get_messages()) == 4

        lp.sec("check message count of only system messages (without daymarkers)")
        dc_array = ffi.gc(
            lib.dc_get_chat_msgs(ac1._dc_context, chat.id, const.DC_GCM_INFO_ONLY, 0),
            lib.dc_array_unref,
        )
        assert len(list(iter_array(dc_array, lambda x: x))) == 2

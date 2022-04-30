from __future__ import print_function
import pytest
import os
import sys
import queue
import time
from deltachat import const, Account
from deltachat.message import Message
from deltachat.tracker import ImexTracker
from deltachat.hookspec import account_hookimpl
from deltachat.capi import ffi, lib
from deltachat.cutil import iter_array
from datetime import datetime, timedelta, timezone
from imap_tools import AND, U


@pytest.mark.parametrize("msgtext,res", [
    ("Member Me (tmp1@x.org) removed by tmp2@x.org.",
        ("removed", "tmp1@x.org", "tmp2@x.org")),
    ("Member With space (tmp1@x.org) removed by tmp2@x.org.",
        ("removed", "tmp1@x.org", "tmp2@x.org")),
    ("Member With space (tmp1@x.org) removed by Another member (tmp2@x.org).",
        ("removed", "tmp1@x.org", "tmp2@x.org")),
    ("Member With space (tmp1@x.org) removed by me",
        ("removed", "tmp1@x.org", "me")),
    ("Group left by some one (tmp1@x.org).",
        ("removed", "tmp1@x.org", "tmp1@x.org")),
    ("Group left by tmp1@x.org.",
        ("removed", "tmp1@x.org", "tmp1@x.org")),
    ("Member tmp1@x.org added by tmp2@x.org.", ("added", "tmp1@x.org", "tmp2@x.org")),
    ("Member nothing bla bla", None),
    ("Another unknown system message", None),
])
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
        assert d["bcc_self"] == "0"

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

    def test_has_savemime(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        assert "save_mime_headers" in ac1.get_config("sys.config_keys").split()

    def test_has_bccself(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        assert "bcc_self" in ac1.get_config("sys.config_keys").split()
        assert ac1.get_config("bcc_self") == "0"

    def test_selfcontact_if_unconfigured(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        assert not ac1.get_self_contact().addr

    def test_selfcontact_configured(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        me = ac1.get_self_contact()
        assert me.display_name
        assert me.addr

    def test_get_config_fails(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        with pytest.raises(KeyError):
            ac1.get_config("123123")

    def test_empty_group_bcc_self_enabled(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        ac1.set_config("bcc_self", "1")
        chat = ac1.create_group_chat(name="group1")
        msg = chat.send_text("msg1")
        assert msg in chat.get_messages()

    def test_empty_group_bcc_self_disabled(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        ac1.set_config("bcc_self", "0")
        chat = ac1.create_group_chat(name="group1")
        msg = chat.send_text("msg1")
        assert msg in chat.get_messages()


class TestOfflineContact:
    def test_contact_attr(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@example.org", name="some1")
        contact2 = ac1.create_contact("some1@example.org", name="some1")
        str(contact1)
        repr(contact1)
        assert contact1 == contact2
        assert contact1.id
        assert contact1.addr == "some1@example.org"
        assert contact1.display_name == "some1"
        assert not contact1.is_blocked()
        assert not contact1.is_verified()

    def test_get_blocked(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
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
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact(ac1.get_config("addr"))
        assert contact1.id == 1

    def test_get_contacts_and_delete(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
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
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@example.com", name="some1")
        msg = contact1.create_chat().send_text("one message")
        assert not ac1.delete_contact(contact1)
        assert not msg.filemime

    def test_create_chat_flexibility(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        ac2 = acfactory.get_configured_offline_account()
        chat1 = ac1.create_chat(ac2)
        chat2 = ac1.create_chat(ac2.get_self_contact().addr)
        assert chat1 == chat2
        ac3 = acfactory.get_unconfigured_account()
        with pytest.raises(ValueError):
            ac1.create_chat(ac3)

    def test_contact_rename(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
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
        return acfactory.get_configured_offline_account()

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
        assert chat2.id == chat1.id
        assert chat2.get_name() == chat1.get_name()
        assert chat1 == chat2
        assert not (chat1 != chat2)

        for ichat in ac1.get_chats():
            if ichat.id == chat1.id:
                break
        else:
            pytest.fail("could not find chat")

    def test_group_chat_add_second_account(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        ac2 = acfactory.get_configured_offline_account()
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
        ac1.set_stock_translation(const.DC_STR_MSGGRPNAME, "abc %1$s xyz %2$s")
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
        assert chat.get_messages()[-1].text == "abc homework xyz Homework by me."

    @pytest.mark.parametrize("verified", [True, False])
    def test_group_chat_qr(self, acfactory, ac1, verified):
        ac2 = acfactory.get_configured_offline_account()
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

    @pytest.mark.parametrize("typein,typeout", [
            (None, "application/octet-stream"),
            ("text/plain", "text/plain"),
            ("image/png", "image/png"),
    ])
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
        ac1 = acfactory.get_configured_offline_account()
        email = "hello <hello@example.org>"
        contact1 = ac1.create_contact(email)
        assert contact1.addr == "hello@example.org"
        assert contact1.name == "hello"
        contact1 = ac1.create_contact(email, name="world")
        assert contact1.name == "world"
        contact2 = ac1.create_contact("display1 <x@example.org>", "real")
        assert contact2.name == "real"

    def test_create_chat_simple(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
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

    def test_set_config_after_configure_is_forbidden(self, ac1):
        assert ac1.get_config("mail_pw")
        assert ac1.is_configured()
        with pytest.raises(ValueError):
            ac1.set_config("addr", "123@example.org")

    def test_import_export_one_contact(self, acfactory, tmpdir):
        backupdir = tmpdir.mkdir("backup")
        ac1 = acfactory.get_configured_offline_account()
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
        ac1 = acfactory.get_configured_offline_account()
        ac2 = acfactory.get_configured_offline_account()
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
            lib.dc_array_unref
        )
        assert len(list(iter_array(dc_array, lambda x: x))) == 2


def test_basic_imap_api(acfactory, tmpdir):
    ac1, ac2 = acfactory.get_two_online_accounts()
    chat12 = acfactory.get_accepted_chat(ac1, ac2)

    imap2 = ac2.direct_imap

    imap2.idle_start()
    chat12.send_text("hello")
    ac2._evtracker.wait_next_incoming_message()

    imap2.idle_check(terminate=True)
    assert imap2.get_unread_cnt() == 1
    imap2.mark_all_read()
    assert imap2.get_unread_cnt() == 0

    imap2.dump_imap_structures(tmpdir, logfile=sys.stdout)
    imap2.shutdown()


class TestOnlineAccount:
    @pytest.mark.ignored
    def test_configure_generate_key(self, acfactory, lp):
        # A slow test which will generate new keys.
        ac1 = acfactory.get_online_configuring_account(
            pre_generated_key=False,
            config={"key_gen_type": str(const.DC_KEY_GEN_RSA2048)}
        )
        ac2 = acfactory.get_online_configuring_account(
            pre_generated_key=False,
            config={"key_gen_type": str(const.DC_KEY_GEN_ED25519)}
        )
        acfactory.wait_configure_and_start_io()
        chat = acfactory.get_accepted_chat(ac1, ac2)

        lp.sec("ac1: send unencrypted message to ac2")
        chat.send_text("message1")
        lp.sec("ac2: waiting for message from ac1")
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        msg_in = ac2.get_message_by_id(ev.data2)
        assert msg_in.text == "message1"
        assert not msg_in.is_encrypted()

        lp.sec("ac2: send encrypted message to ac1")
        msg_in.chat.send_text("message2")
        lp.sec("ac1: waiting for message from ac2")
        ev = ac1._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        msg2_in = ac1.get_message_by_id(ev.data2)
        assert msg2_in.text == "message2"
        assert msg2_in.is_encrypted()

        lp.sec("ac1: send encrypted message to ac2")
        msg2_in.chat.send_text("message3")
        lp.sec("ac2: waiting for message from ac1")
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        msg3_in = ac1.get_message_by_id(ev.data2)
        assert msg3_in.text == "message3"
        assert msg3_in.is_encrypted()

    def test_configure_canceled(self, acfactory):
        ac1 = acfactory.get_online_configuring_account()
        ac1._configtracker.wait_progress()
        ac1.stop_ongoing()
        try:
            ac1._configtracker.wait_finish()
        except Exception:
            pass

    def test_export_import_self_keys(self, acfactory, tmpdir, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        dir = tmpdir.mkdir("exportdir")
        export_files = ac1.export_self_keys(dir.strpath)
        assert len(export_files) == 2
        for x in export_files:
            assert x.startswith(dir.strpath)
        key_id, = ac1._evtracker.get_info_regex_groups(r".*xporting.*KeyId\((.*)\).*")
        ac1._evtracker.consume_events()

        lp.sec("exported keys (private and public)")
        for name in os.listdir(dir.strpath):
            lp.indent(dir.strpath + os.sep + name)
        lp.sec("importing into existing account")
        ac2.import_self_keys(dir.strpath)
        key_id2, = ac2._evtracker.get_info_regex_groups(
            r".*stored.*KeyId\((.*)\).*", check_error=False)
        assert key_id2 == key_id

    def test_one_account_send_bcc_setting(self, acfactory, lp):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()

        # Clone the first account: we will test if sent messages
        # are copied to it via BCC.
        ac1_clone = acfactory.clone_online_account(ac1)

        acfactory.wait_configure_and_start_io()

        chat = acfactory.get_accepted_chat(ac1, ac2)

        self_addr = ac1.get_config("addr")
        other_addr = ac2.get_config("addr")

        lp.sec("send out message without bcc to ourselves")
        ac1.set_config("bcc_self", "0")
        msg_out = chat.send_text("message1")
        assert not msg_out.is_forwarded()

        # wait for send out (no BCC)
        ev = ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")
        assert ac1.get_config("bcc_self") == "0"

        # make sure we are not sending message to ourselves
        assert self_addr not in ev.data2
        assert other_addr in ev.data2

        lp.sec("ac1: setting bcc_self=1")
        ac1.set_config("bcc_self", "1")

        lp.sec("send out message with bcc to ourselves")
        ac1.direct_imap.idle_start()
        msg_out = chat.send_text("message2")

        # wait for send out (BCC)
        ev = ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")
        assert ac1.get_config("bcc_self") == "1"

        # now make sure we are sending message to ourselves too
        assert self_addr in ev.data2
        assert other_addr in ev.data2
        assert ac1.direct_imap.idle_wait_for_seen()

        # Second client receives only second message, but not the first
        ev_msg = ac1_clone._evtracker.wait_next_messages_changed()
        assert ev_msg.text == msg_out.text

    def test_send_file_twice_unicode_filename_mangling(self, tmpdir, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = acfactory.get_accepted_chat(ac1, ac2)

        basename = "somedäüta.html.zip"
        p = os.path.join(tmpdir.strpath, basename)
        with open(p, "w") as f:
            f.write("some data")

        def send_and_receive_message():
            lp.sec("ac1: prepare and send attachment + text to ac2")
            msg1 = Message.new_empty(ac1, "file")
            msg1.set_text("withfile")
            msg1.set_file(p)
            chat.send_msg(msg1)

            lp.sec("ac2: receive message")
            ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
            assert ev.data2 > const.DC_CHAT_ID_LAST_SPECIAL
            return ac2.get_message_by_id(ev.data2)

        msg = send_and_receive_message()
        assert msg.text == "withfile"
        assert open(msg.filename).read() == "some data"
        assert msg.filename.endswith(basename)

        msg2 = send_and_receive_message()
        assert msg2.text == "withfile"
        assert open(msg2.filename).read() == "some data"
        assert msg2.filename.endswith("html.zip")
        assert msg.filename != msg2.filename

    def test_send_file_html_attachment(self, tmpdir, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = acfactory.get_accepted_chat(ac1, ac2)

        basename = "test.html"
        content = "<html><body>text</body>data"

        p = os.path.join(tmpdir.strpath, basename)
        with open(p, "w") as f:
            # write wrong html to see if core tries to parse it
            # (it shouldn't as it's a file attachment)
            f.write(content)

        lp.sec("ac1: prepare and send attachment + text to ac2")
        chat.send_file(p, mime_type="text/html")

        lp.sec("ac2: receive message")
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        assert ev.data2 > const.DC_CHAT_ID_LAST_SPECIAL
        msg = ac2.get_message_by_id(ev.data2)

        assert open(msg.filename).read() == content
        assert msg.filename.endswith(basename)

    def test_html_message(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = acfactory.get_accepted_chat(ac1, ac2)
        html_text = "<p>hello HTML world</p>"

        lp.sec("ac1: prepare and send text message to ac2")
        msg1 = chat.send_text("message0")
        assert not msg1.has_html()
        assert msg1.html == ""

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "message0"
        assert not msg2.has_html()
        assert msg2.html == ""

        lp.sec("ac1: prepare and send HTML+text message to ac2")
        msg1 = Message.new_empty(ac1, "text")
        msg1.set_text("message1")
        msg1.set_html(html_text)
        msg1 = chat.send_msg(msg1)
        assert msg1.has_html()
        assert html_text in msg1.html

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "message1"
        assert msg2.has_html()
        assert html_text in msg2.html

        lp.sec("ac1: prepare and send HTML-only message to ac2")
        msg1 = Message.new_empty(ac1, "text")
        msg1.set_html(html_text)
        msg1 = chat.send_msg(msg1)

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert "<p>" not in msg2.text
        assert "hello HTML world" in msg2.text
        assert msg2.has_html()
        assert html_text in msg2.html

    def test_mvbox_sentbox_threads(self, acfactory, lp):
        lp.sec("ac1: start with mvbox thread")
        ac1 = acfactory.get_online_configuring_account(move=True, sentbox=True)

        lp.sec("ac2: start without mvbox/sentbox threads")
        ac2 = acfactory.get_online_configuring_account()

        lp.sec("ac2 and ac1: waiting for configuration")
        acfactory.wait_configure_and_start_io()

        lp.sec("ac1: send message and wait for ac2 to receive it")
        acfactory.get_accepted_chat(ac1, ac2).send_text("message1")
        assert ac2._evtracker.wait_next_incoming_message().text == "message1"

    def test_move_works(self, acfactory):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account(move=True)
        acfactory.wait_configure_and_start_io()
        chat = acfactory.get_accepted_chat(ac1, ac2)
        chat.send_text("message1")

        # Message is moved to the movebox
        ac2._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")

        # Message is downloaded
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        assert ev.data2 > const.DC_CHAT_ID_LAST_SPECIAL

    def test_move_works_on_self_sent(self, acfactory):
        ac1 = acfactory.get_online_configuring_account(move=True)
        ac2 = acfactory.get_online_configuring_account()
        acfactory.wait_configure_and_start_io()
        ac1.set_config("bcc_self", "1")

        chat = acfactory.get_accepted_chat(ac1, ac2)
        chat.send_text("message1")
        ac1._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")
        chat.send_text("message2")
        ac1._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")
        chat.send_text("message3")
        ac1._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")

    def test_forward_messages(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = ac1.create_chat(ac2)

        lp.sec("ac1: send message to ac2")
        msg_out = chat.send_text("message2")

        lp.sec("ac2: wait for receive")
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev.data2 == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.text == "message2"

        lp.sec("ac2: check that the message arrived in a chat")
        chat2 = msg_in.chat
        assert msg_in in chat2.get_messages()
        assert not msg_in.is_forwarded()
        assert chat2.is_contact_request()

        lp.sec("ac2: create new chat and forward message to it")
        chat3 = ac2.create_group_chat("newgroup")
        assert not chat3.is_promoted()
        ac2.forward_messages([msg_in], chat3)

        lp.sec("ac2: check new chat has a forwarded message")
        assert chat3.is_promoted()
        messages = chat3.get_messages()
        msg = messages[-1]
        assert msg.is_forwarded()
        ac2.delete_messages(messages)
        assert not chat3.get_messages()

    def test_forward_own_message(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = acfactory.get_accepted_chat(ac1, ac2)

        lp.sec("sending message")
        msg_out = chat.send_text("message2")

        lp.sec("receiving message")
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        msg_in = ac2.get_message_by_id(ev.data2)
        assert msg_in.text == "message2"
        assert not msg_in.is_forwarded()

        lp.sec("ac1: creating group chat, and forward own message")
        group = ac1.create_group_chat("newgroup2")
        group.add_contact(ac2)
        ac1.forward_messages([msg_out], group)

        # wait for other account to receive
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        msg_in = ac2.get_message_by_id(ev.data2)
        assert msg_in.text == "message2"
        assert msg_in.is_forwarded()

    def test_send_self_message(self, acfactory, lp):
        ac1 = acfactory.get_one_online_account(move=True)
        lp.sec("ac1: create self chat")
        chat = ac1.get_self_contact().create_chat()
        chat.send_text("hello")
        ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

    def test_send_dot(self, acfactory, lp):
        """Test that a single dot is properly escaped in SMTP protocol"""
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = acfactory.get_accepted_chat(ac1, ac2)

        lp.sec("sending message")
        msg_out = chat.send_text(".")

        lp.sec("receiving message")
        msg_in = ac2._evtracker.wait_next_incoming_message()
        assert msg_in.text == msg_out.text

    def test_send_and_receive_message_markseen(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        # make DC's life harder wrt to encodings
        ac1.set_config("displayname", "ä name")

        lp.sec("ac1: create chat with ac2")
        chat = ac1.create_chat(ac2)

        lp.sec("sending text message from ac1 to ac2")
        msg1 = chat.send_text("message1")
        ac1._evtracker.wait_msg_delivered(msg1)

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "message1"
        assert not msg2.is_forwarded()
        assert msg2.get_sender_contact().display_name == ac1.get_config("displayname")

        lp.sec("check the message arrived in contact request chat")
        chat2 = msg2.chat
        assert msg2 in chat2.get_messages()
        assert chat2.is_contact_request()
        assert chat2.count_fresh_messages() == 1
        # Like it or not, this assert is flaky
        # assert msg2.time_received >= msg1.time_sent

        lp.sec("create new chat with contact and verify it's proper")
        chat2b = msg2.create_chat()
        assert not chat2b.is_contact_request()
        assert chat2b.count_fresh_messages() == 1

        lp.sec("mark chat as noticed")
        chat2b.mark_noticed()
        assert chat2b.count_fresh_messages() == 0

        ac2._evtracker.consume_events()

        lp.sec("sending a second message from ac1 to ac2")
        msg3 = chat.send_text("message2")

        lp.sec("wait for ac2 to receive second message")
        msg4 = ac2._evtracker.wait_next_incoming_message()

        lp.sec("mark messages as seen on ac2, wait for changes on ac1")
        ac2.direct_imap.idle_start()
        ac1.direct_imap.idle_start()

        ac2.mark_seen_messages([msg2, msg4])
        ev = ac2._evtracker.get_matching("DC_EVENT_MSGS_NOTICED")
        assert msg2.chat.id == msg4.chat.id
        assert ev.data1 == msg2.chat.id
        assert ev.data2 == 0

        ac2.direct_imap.idle_wait_for_new_message(terminate=True)
        lp.step("1")
        for i in range(2):
            ev = ac1._evtracker.get_matching("DC_EVENT_MSG_READ")
            assert ev.data1 > const.DC_CHAT_ID_LAST_SPECIAL
            assert ev.data2 > const.DC_MSG_ID_LAST_SPECIAL
        lp.step("2")
        ac1.direct_imap.idle_wait_for_seen()  # Check that ac1 marks the read receipt as read

        assert msg1.is_out_mdn_received()
        assert msg3.is_out_mdn_received()

        lp.sec("try check that a second call to mark_seen doesn't happen")
        ac2._evtracker.consume_events()
        msg2.mark_seen()
        try:
            ac2._evtracker.get_matching("DC_EVENT_MSG_READ", timeout=0.01)
        except queue.Empty:
            pass  # mark_seen_messages() has generated events before it returns

    def test_moved_markseen(self, acfactory, lp):
        """Test that message already moved to DeltaChat folder is marked as seen."""
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account(move=True)
        acfactory.wait_configure_and_start_io([ac1, ac2])

        ac2.stop_io()
        ac2.direct_imap.idle_start()

        ac1.create_chat(ac2).send_text("Hello!")

        ac2.direct_imap.idle_wait_for_new_message(terminate=True)

        # Emulate moving of the message to DeltaChat folder by Sieve rule.
        # mailcow server contains this rule by default.
        ac2.direct_imap.conn.move(["*"], "DeltaChat")

        ac2.direct_imap.select_folder("DeltaChat")
        ac2.direct_imap.idle_start()
        ac2.start_io()
        msg = ac2._evtracker.wait_next_incoming_message()

        # Accept the contact request.
        msg.chat.accept()
        ac2.mark_seen_messages([msg])
        uid = ac2.direct_imap.idle_wait_for_seen(terminate=True)

        assert len([a for a in ac2.direct_imap.conn.fetch(AND(seen=True, uid=U(uid, "*")))]) == 1

    def test_multidevice_sync_seen(self, acfactory, lp):
        """Test that message marked as seen on one device is marked as seen on another."""
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()
        ac1_clone = acfactory.clone_online_account(ac1)
        acfactory.wait_configure_and_start_io()

        ac1.set_config("bcc_self", "1")
        ac1_clone.set_config("bcc_self", "1")

        ac1_chat = ac1.create_chat(ac2)
        ac1_clone_chat = ac1_clone.create_chat(ac2)
        ac2_chat = ac2.create_chat(ac1)

        lp.sec("Send a message from ac2 to ac1 and check that it's 'fresh'")
        ac2_chat.send_text("Hi")
        ac1_message = ac1._evtracker.wait_next_incoming_message()
        ac1_clone_message = ac1_clone._evtracker.wait_next_incoming_message()
        assert ac1_chat.count_fresh_messages() == 1
        assert ac1_clone_chat.count_fresh_messages() == 1
        assert ac1_message.is_in_fresh
        assert ac1_clone_message.is_in_fresh

        lp.sec("ac1 marks message as seen on the first device")
        ac1.mark_seen_messages([ac1_message])
        assert ac1_message.is_in_seen

        lp.sec("ac1 clone detects that message is marked as seen")
        ev = ac1_clone._evtracker.get_matching("DC_EVENT_MSGS_NOTICED")
        assert ev.data1 == ac1_clone_chat.id
        assert ac1_clone_message.is_in_seen

        lp.sec("Send an ephemeral message from ac2 to ac1")
        ac2_chat.set_ephemeral_timer(60)
        ac1._evtracker.get_matching("DC_EVENT_CHAT_EPHEMERAL_TIMER_MODIFIED")
        ac1._evtracker.wait_next_incoming_message()
        ac1_clone._evtracker.get_matching("DC_EVENT_CHAT_EPHEMERAL_TIMER_MODIFIED")
        ac1_clone._evtracker.wait_next_incoming_message()

        ac2_chat.send_text("Foobar")
        ac1_message = ac1._evtracker.wait_next_incoming_message()
        ac1_clone_message = ac1_clone._evtracker.wait_next_incoming_message()
        assert "Ephemeral timer: 60\n" in ac1_message.get_message_info()
        assert "Expires: " not in ac1_clone_message.get_message_info()
        assert "Ephemeral timer: 60\n" in ac1_message.get_message_info()
        assert "Expires: " not in ac1_clone_message.get_message_info()

        ac1.mark_seen_messages([ac1_message])
        assert ac1_message.is_in_seen
        assert "Expires: " in ac1_message.get_message_info()
        ev = ac1_clone._evtracker.get_matching("DC_EVENT_MSGS_NOTICED")
        assert ev.data1 == ac1_clone_chat.id
        assert ac1_clone_message.is_in_seen
        # Test that the timer is started on the second device after synchronizing the seen status.
        assert "Expires: " in ac1_clone_message.get_message_info()

    def test_message_override_sender_name(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = acfactory.get_accepted_chat(ac1, ac2)
        overridden_name = "someone else"

        ac1.set_config("displayname", "ac1")

        lp.sec("sending text message with overridden name from ac1 to ac2")
        msg1 = Message.new_empty(ac1, "text")
        msg1.set_override_sender_name(overridden_name)
        msg1.set_text("message1")
        msg1 = chat.send_msg(msg1)
        assert msg1.override_sender_name == overridden_name

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "message1"
        assert msg2.get_sender_contact().name == ac1.get_config("displayname")
        assert msg2.override_sender_name == overridden_name

        lp.sec("sending normal text message from ac1 to ac2")
        msg1 = Message.new_empty(ac1, "text")
        msg1.set_text("message2")
        msg1 = chat.send_msg(msg1)
        assert not msg1.override_sender_name

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "message2"
        assert msg2.get_sender_contact().name == ac1.get_config("displayname")
        assert not msg2.override_sender_name

    @pytest.mark.parametrize("mvbox_move", [True, False])
    def test_markseen_message_and_mdn(self, acfactory, mvbox_move):
        # Please only change this test if you are very sure that it will still catch the issues it catches now.
        # We had so many problems with markseen, if in doubt, rather create another test, it can't harm.
        ac1 = acfactory.get_online_configuring_account(move=mvbox_move)
        ac2 = acfactory.get_online_configuring_account(move=mvbox_move)

        acfactory.wait_configure_and_start_io()
        # Do not send BCC to self, we only want to test MDN on ac1.
        ac1.set_config("bcc_self", "0")

        folder = "mvbox" if mvbox_move else "inbox"
        ac1.direct_imap.select_config_folder(folder)
        ac2.direct_imap.select_config_folder(folder)
        ac1.direct_imap.idle_start()
        ac2.direct_imap.idle_start()

        acfactory.get_accepted_chat(ac1, ac2).send_text("hi")
        msg = ac2._evtracker.wait_next_incoming_message()

        ac2.mark_seen_messages([msg])

        ac1.direct_imap.idle_wait_for_seen()  # Check that the mdn is marked as seen
        ac2.direct_imap.idle_wait_for_seen()  # Check that the original message is marked as seen
        ac1.direct_imap.idle_done()
        ac2.direct_imap.idle_done()

    def test_reply_privately(self, acfactory):
        ac1, ac2 = acfactory.get_two_online_accounts()

        group1 = ac1.create_group_chat("group")
        group1.add_contact(ac2)
        group1.send_text("hello")

        msg2 = ac2._evtracker.wait_next_incoming_message()
        group2 = msg2.create_chat()
        assert group2.get_name() == group1.get_name()

        msg_reply = Message.new_empty(ac2, "text")
        msg_reply.set_text("message reply")
        msg_reply.quote = msg2

        private_chat1 = ac1.create_chat(ac2)
        private_chat2 = ac2.create_chat(ac1)
        private_chat2.send_msg(msg_reply)

        msg_reply1 = ac1._evtracker.wait_next_incoming_message()
        assert msg_reply1.quoted_text == "hello"
        assert not msg_reply1.chat.is_group()
        assert msg_reply1.chat.id == private_chat1.id

    def test_mdn_asymmetric(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts(move=True)

        lp.sec("ac1: create chat with ac2")
        chat = ac1.create_chat(ac2)
        ac2.create_chat(ac1)

        # make sure mdns are enabled (usually enabled by default already)
        ac1.set_config("mdns_enabled", "1")
        ac2.set_config("mdns_enabled", "1")

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")

        assert len(chat.get_messages()) == 1

        lp.sec("disable ac1 MDNs")
        ac1.set_config("mdns_enabled", "0")

        lp.sec("wait for ac2 to receive message")
        msg = ac2._evtracker.wait_next_incoming_message()

        assert len(msg.chat.get_messages()) == 1

        ac1.direct_imap.select_config_folder("mvbox")
        ac1.direct_imap.idle_start()

        lp.sec("ac2: mark incoming message as seen")
        ac2.mark_seen_messages([msg])

        lp.sec("ac1: waiting for incoming activity")
        # MDN should be moved even though MDNs are already disabled
        ac1._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")

        assert len(chat.get_messages()) == 1

        # Wait for the message to be marked as seen on IMAP.
        assert ac1.direct_imap.idle_wait_for_seen()

        # MDN is received even though MDNs are already disabled
        assert msg_out.is_out_mdn_received()

    def test_send_and_receive_will_encrypt_decrypt(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat = ac1.create_chat(ac2)

        lp.sec("sending text message from ac1 to ac2")
        chat.send_text("message1")

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "message1"

        lp.sec("create new chat with contact and send back (encrypted) message")
        chat2b = msg2.create_chat()
        chat2b.send_text("message-back")

        lp.sec("wait for ac1 to receive message")
        msg3 = ac1._evtracker.wait_next_incoming_message()
        assert msg3.text == "message-back"
        assert msg3.is_encrypted() and msg3.is_in_fresh()

        # test get_fresh_messages
        fresh_msgs = list(ac1.get_fresh_messages())
        assert len(fresh_msgs) == 1
        assert fresh_msgs[0] == msg3
        msg3.mark_seen()
        assert not list(ac1.get_fresh_messages())

        # Test that we do not gossip peer keys in 1-to-1 chat,
        # as it makes no sense to gossip to peers their own keys.
        # Gossip is only sent in encrypted messages,
        # and we sent encrypted msg_back right above.
        assert chat2b.get_summary()["gossiped_timestamp"] == 0

        lp.sec("create group chat with two members, one of which has no encrypt state")
        chat = ac1.create_group_chat("encryption test")
        chat.add_contact(ac2)
        chat.add_contact(ac1.create_contact("notexisting@testrun.org"))
        msg = chat.send_text("test not encrypt")
        assert not msg.is_encrypted()
        ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

    def test_gossip_optimization(self, acfactory, lp):
        """Test that gossip timestamp is updated when someone else sends gossip,
        so we don't have to send gossip ourselves.
        """
        ac1, ac2, ac3 = acfactory.get_many_online_accounts(3)

        acfactory.introduce_each_other([ac1, ac2])
        acfactory.introduce_each_other([ac2, ac3])

        lp.sec("ac1 creates a group chat with ac2")
        group_chat = ac1.create_group_chat("hello")
        group_chat.add_contact(ac2)
        msg = group_chat.send_text("hi")

        # No Autocrypt gossip was sent yet.
        gossiped_timestamp = msg.chat.get_summary()["gossiped_timestamp"]
        assert gossiped_timestamp == 0

        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.is_encrypted()
        assert msg.text == "hi"

        lp.sec("ac2 adds ac3 to the group")
        msg.chat.add_contact(ac3)

        lp.sec("ac1 receives message from ac2 and updates gossip timestamp")
        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.is_encrypted()

        # ac1 has updated the gossip timestamp even though no gossip was sent by ac1.
        # ac1 does not need to send gossip because ac2 already did it.
        gossiped_timestamp = msg.chat.get_summary()["gossiped_timestamp"]
        assert gossiped_timestamp == int(msg.time_sent.timestamp())

    def test_gossip_encryption_preference(self, acfactory, lp):
        """Test that encryption preference of group members is gossiped to new members.
        This is a Delta Chat extension to Autocrypt 1.1.0, which Autocrypt-Gossip headers
        SHOULD NOT contain encryption preference.
        """
        ac1, ac2, ac3 = acfactory.get_many_online_accounts(3)

        lp.sec("ac1 learns that ac2 prefers encryption")
        ac1.create_chat(ac2)
        msg = ac2.create_chat(ac1).send_text("first message")
        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.text == "first message"
        assert not msg.is_encrypted()
        res = "{} End-to-end encryption preferred.".format(ac2.get_config('addr'))
        assert msg.chat.get_encryption_info() == res
        lp.sec("ac2 learns that ac3 prefers encryption")
        ac2.create_chat(ac3)
        msg = ac3.create_chat(ac2).send_text("I prefer encryption")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "I prefer encryption"
        assert not msg.is_encrypted()

        lp.sec("ac3 does not know that ac1 prefers encryption")
        ac1.create_chat(ac3)
        chat = ac3.create_chat(ac1)
        res = "{} No encryption.".format(ac1.get_config('addr'))
        assert chat.get_encryption_info() == res
        msg = chat.send_text("not encrypted")
        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.text == "not encrypted"
        assert not msg.is_encrypted()

        lp.sec("ac1 creates a group chat with ac2")
        group_chat = ac1.create_group_chat("hello")
        group_chat.add_contact(ac2)
        encryption_info = group_chat.get_encryption_info()
        res = "{} End-to-end encryption preferred.".format(ac2.get_config("addr"))
        assert encryption_info == res
        msg = group_chat.send_text("hi")

        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.is_encrypted()
        assert msg.text == "hi"

        lp.sec("ac2 adds ac3 to the group")
        msg.chat.add_contact(ac3)
        assert msg.is_encrypted()

        lp.sec("ac3 learns that ac1 prefers encryption")
        msg = ac3._evtracker.wait_next_incoming_message()
        encryption_info = msg.chat.get_encryption_info().splitlines()
        res = "{} End-to-end encryption preferred.".format(ac1.get_config("addr"))
        assert res in encryption_info
        res = "{} End-to-end encryption preferred.".format(ac2.get_config("addr"))
        assert res in encryption_info
        msg = chat.send_text("encrypted")
        assert msg.is_encrypted()

    def test_send_first_message_as_long_unicode_with_cr(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        ac2.set_config("save_mime_headers", "1")

        lp.sec("ac1: create chat with ac2")
        chat = acfactory.get_accepted_chat(ac1, ac2)

        lp.sec("sending multi-line non-unicode message from ac1 to ac2")
        text1 = (
            "hello\nworld\nthis is a very long message that should be"
            + " wrapped using format=flowed and unwrapped on the receiver"
        )
        msg_out = chat.send_text(text1)
        assert not msg_out.is_encrypted()

        lp.sec("wait for ac2 to receive multi-line non-unicode message")
        msg_in = ac2._evtracker.wait_next_incoming_message()
        assert msg_in.text == text1

        lp.sec("sending multi-line unicode text message from ac1 to ac2")
        text2 = "äalis\nthis is ßßÄ"
        msg_out = chat.send_text(text2)
        assert not msg_out.is_encrypted()

        lp.sec("wait for ac2 to receive multi-line unicode message")
        msg_in = ac2._evtracker.wait_next_incoming_message()
        assert msg_in.text == text2
        assert ac1.get_config("addr") in [x.addr for x in msg_in.chat.get_contacts()]

    def test_no_draft_if_cant_send(self, acfactory):
        """Tests that no quote can be set if the user can't send to this chat"""
        ac1 = acfactory.get_one_online_account()
        device_chat = ac1.get_device_chat()
        msg = Message.new_empty(ac1, "text")
        device_chat.set_draft(msg)

        assert not device_chat.can_send()
        assert device_chat.get_draft() is None

    def test_dont_show_emails(self, acfactory, lp):
        """Most mailboxes have a "Drafts" folder where constantly new emails appear but we don't actually want to show them.
        So: If it's outgoing AND there is no Received header AND it's not in the sentbox, then ignore the email.

        If the draft email is sent out later (i.e. moved to "Sent"), it must be shown.

        Also, test that unknown emails in the Spam folder are not shown."""
        ac1 = acfactory.get_online_configuring_account()
        ac1.set_config("show_emails", "2")
        ac1.create_contact("alice@example.org").create_chat()

        acfactory.wait_configure(ac1)
        ac1.direct_imap.create_folder("Drafts")
        ac1.direct_imap.create_folder("Sent")
        ac1.direct_imap.create_folder("Spam")
        ac1.direct_imap.create_folder("Junk")

        acfactory.wait_configure_and_start_io()
        # Wait until each folder was selected once and we are IDLEing again:
        ac1._evtracker.get_info_contains("INBOX: Idle entering wait-on-remote state")
        ac1.stop_io()

        ac1.direct_imap.append("Drafts", """
            From: ac1 <{}>
            Subject: subj
            To: alice@example.org
            Message-ID: <aepiors@example.org>
            Content-Type: text/plain; charset=utf-8

            message in Drafts that is moved to Sent later
        """.format(ac1.get_config("configured_addr")))
        ac1.direct_imap.append("Sent", """
            From: ac1 <{}>
            Subject: subj
            To: alice@example.org
            Message-ID: <hsabaeni@example.org>
            Content-Type: text/plain; charset=utf-8

            message in Sent
        """.format(ac1.get_config("configured_addr")))
        ac1.direct_imap.append("Spam", """
            From: unknown.address@junk.org
            Subject: subj
            To: {}
            Message-ID: <spam.message@junk.org>
            Content-Type: text/plain; charset=utf-8

            Unknown message in Spam
        """.format(ac1.get_config("configured_addr")))
        ac1.direct_imap.append("Junk", """
            From: unknown.address@junk.org
            Subject: subj
            To: {}
            Message-ID: <spam.message@junk.org>
            Content-Type: text/plain; charset=utf-8

            Unknown message in Junk
        """.format(ac1.get_config("configured_addr")))

        ac1.set_config("scan_all_folders_debounce_secs", "0")
        lp.sec("All prepared, now let DC find the message")
        ac1.start_io()

        msg = ac1._evtracker.wait_next_messages_changed()

        # Wait until each folder was scanned, this is necessary for this test to test what it should test:
        ac1._evtracker.get_info_contains("INBOX: Idle entering wait-on-remote state")

        assert msg.text == "subj – message in Sent"
        assert len(msg.chat.get_messages()) == 1

        assert not any("unknown.address" in c.get_name() for c in ac1.get_chats())
        ac1.direct_imap.select_folder("Spam")
        assert ac1.direct_imap.get_uid_by_message_id("spam.message@junk.org")

        ac1.stop_io()
        lp.sec("'Send out' the draft, i.e. move it to the Sent folder, and wait for DC to display it this time")
        ac1.direct_imap.select_folder("Drafts")
        uid = ac1.direct_imap.get_uid_by_message_id("aepiors@example.org")
        ac1.direct_imap.conn.move(uid, "Sent")

        ac1.start_io()
        msg2 = ac1._evtracker.wait_next_messages_changed()

        assert msg2.text == "subj – message in Drafts that is moved to Sent later"
        assert len(msg.chat.get_messages()) == 2

    def test_no_old_msg_is_fresh(self, acfactory, lp):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()
        ac1_clone = acfactory.clone_online_account(ac1)
        acfactory.wait_configure_and_start_io()

        ac1.set_config("e2ee_enabled", "0")
        ac1_clone.set_config("e2ee_enabled", "0")
        ac2.set_config("e2ee_enabled", "0")

        ac1_clone.set_config("bcc_self", "1")

        ac1.create_chat(ac2)
        ac1_clone.create_chat(ac2)

        lp.sec("Send a first message from ac2 to ac1 and check that it's 'fresh'")
        first_msg_id = ac2.create_chat(ac1).send_text("Hi")
        ac1._evtracker.wait_next_incoming_message()
        assert ac1.create_chat(ac2).count_fresh_messages() == 1
        assert len(list(ac1.get_fresh_messages())) == 1

        lp.sec("Send a message from ac1_clone to ac2 and check that ac1 marks the first message as 'noticed'")
        ac1_clone.create_chat(ac2).send_text("Hi back")
        ev = ac1._evtracker.get_matching("DC_EVENT_MSGS_NOTICED")

        assert ev.data1 == first_msg_id.chat.id
        assert ac1.create_chat(ac2).count_fresh_messages() == 0
        assert len(list(ac1.get_fresh_messages())) == 0

    def test_prefer_encrypt(self, acfactory, lp):
        """Test quorum rule for encryption preference in 1:1 and group chat."""
        ac1, ac2, ac3 = acfactory.get_many_online_accounts(3)
        ac1.set_config("e2ee_enabled", "0")
        ac2.set_config("e2ee_enabled", "1")
        ac3.set_config("e2ee_enabled", "0")

        # Make sure we do not send a copy to ourselves. This is to
        # test that we count own preference even when we are not in
        # the recipient list.
        ac1.set_config("bcc_self", "0")
        ac2.set_config("bcc_self", "0")
        ac3.set_config("bcc_self", "0")

        acfactory.introduce_each_other([ac1, ac2, ac3])

        lp.sec("ac1: sending message to ac2")
        chat1 = ac1.create_chat(ac2)
        msg1 = chat1.send_text("message1")
        assert not msg1.is_encrypted()
        ac2._evtracker.wait_next_incoming_message()

        lp.sec("ac2: sending message to ac1")
        chat2 = ac2.create_chat(ac1)
        msg2 = chat2.send_text("message2")
        assert not msg2.is_encrypted()
        ac1._evtracker.wait_next_incoming_message()

        lp.sec("ac1: sending message to group chat with ac2 and ac3")
        group = ac1.create_group_chat("hello")
        group.add_contact(ac2)
        group.add_contact(ac3)
        msg3 = group.send_text("message3")
        assert not msg3.is_encrypted()
        ac2._evtracker.wait_next_incoming_message()
        ac3._evtracker.wait_next_incoming_message()

        lp.sec("ac3: start preferring encryption and inform ac1")
        ac3.set_config("e2ee_enabled", "1")
        chat3 = ac3.create_chat(ac1)
        msg4 = chat3.send_text("message4")
        # ac1 still does not prefer encryption
        assert not msg4.is_encrypted()
        ac1._evtracker.wait_next_incoming_message()

        lp.sec("ac1: sending another message to group chat with ac2 and ac3")
        msg5 = group.send_text("message5")
        # Majority prefers encryption now
        assert msg5.is_encrypted()

    def test_bot(self, acfactory, lp):
        """Test that bot messages can be identified as such"""
        ac1, ac2 = acfactory.get_two_online_accounts()
        ac1.set_config("bot", "0")
        ac2.set_config("bot", "1")

        lp.sec("ac1: create chat with ac2")
        chat = acfactory.get_accepted_chat(ac1, ac2)

        lp.sec("sending a message from ac1 to ac2")
        text1 = "hello"
        chat.send_text(text1)

        lp.sec("wait for ac2 to receive a message")
        msg_in = ac2._evtracker.wait_next_incoming_message()
        assert msg_in.text == text1
        assert not msg_in.is_bot()

        lp.sec("sending a message from ac2 to ac1")
        text2 = "reply"
        msg_in.chat.send_text(text2)

        lp.sec("wait for ac1 to receive a message")
        msg_in = ac1._evtracker.wait_next_incoming_message()
        assert msg_in.text == text2
        assert msg_in.is_bot()

    def test_quote_encrypted(self, acfactory, lp):
        """Test that replies to encrypted messages with quotes are encrypted."""
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat = ac1.create_chat(ac2)

        lp.sec("sending text message from ac1 to ac2")
        msg1 = chat.send_text("message1")
        assert not msg1.is_encrypted()

        lp.sec("wait for ac2 to receive message")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "message1"
        assert not msg2.is_encrypted()

        lp.sec("create new chat with contact and send back (encrypted) message")
        msg2.create_chat().send_text("message-back")

        lp.sec("wait for ac1 to receive message")
        msg3 = ac1._evtracker.wait_next_incoming_message()
        assert msg3.text == "message-back"
        assert msg3.is_encrypted()

        lp.sec("ac1: e2ee_enabled=0 and see if reply is encrypted")
        print("ac1: e2ee_enabled={}".format(ac1.get_config("e2ee_enabled")))
        print("ac2: e2ee_enabled={}".format(ac2.get_config("e2ee_enabled")))
        ac1.set_config("e2ee_enabled", "0")

        for quoted_msg in msg1, msg3:
            # Save the draft with a quote.
            # It should be encrypted if quoted message is encrypted.
            msg_draft = Message.new_empty(ac1, "text")
            msg_draft.set_text("message reply")
            msg_draft.quote = quoted_msg
            chat.set_draft(msg_draft)

            # Get the draft, prepare and send it.
            msg_draft = chat.get_draft()
            msg_out = chat.prepare_message(msg_draft)
            chat.send_prepared(msg_out)

            chat.set_draft(None)
            assert chat.get_draft() is None

            msg_in = ac2._evtracker.wait_next_incoming_message()
            assert msg_in.text == "message reply"
            assert msg_in.quoted_text == quoted_msg.text
            assert msg_in.is_encrypted() == quoted_msg.is_encrypted()

    def test_quote_attachment(self, tmpdir, acfactory, lp):
        """Test that replies with an attachment and a quote are received correctly."""
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1 creates chat with ac2")
        chat1 = ac1.create_chat(ac2)

        lp.sec("ac1 sends text message to ac2")
        chat1.send_text("hi")

        lp.sec("ac2 receives contact request from ac1")
        received_message = ac2._evtracker.wait_next_incoming_message()
        assert received_message.text == "hi"

        basename = "attachment.txt"
        p = os.path.join(tmpdir.strpath, basename)
        with open(p, "w") as f:
            f.write("data to send")

        lp.sec("ac2 sends a reply to ac1")
        chat2 = received_message.create_chat()
        reply = Message.new_empty(ac2, "file")
        reply.set_text("message reply")
        reply.set_file(p)
        reply.quote = received_message
        chat2.send_msg(reply)

        lp.sec("ac1 receives a reply from ac2")
        received_reply = ac1._evtracker.wait_next_incoming_message()
        assert received_reply.text == "message reply"
        assert received_reply.quoted_text == received_message.text
        assert open(received_reply.filename).read() == "data to send"

    def test_saved_mime_on_received_message(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("configure ac2 to save mime headers, create ac1/ac2 chat")
        ac2.set_config("save_mime_headers", "1")
        chat = ac1.create_chat(ac2)

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")
        ac1._evtracker.wait_msg_delivered(msg_out)
        assert msg_out.get_mime_headers() is None

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG")
        in_id = ev.data2
        mime = ac2.get_message_by_id(in_id).get_mime_headers()
        assert mime.get_all("From")
        assert mime.get_all("Received")

    def test_send_mark_seen_clean_incoming_events(self, acfactory, lp, data):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = acfactory.get_accepted_chat(ac1, ac2)

        message_queue = queue.Queue()

        class InPlugin:
            @account_hookimpl
            def ac_incoming_message(self, message):
                message_queue.put(message)

        ac1.add_account_plugin(InPlugin())

        lp.sec("sending one message from ac1 to ac2")
        chat.send_text("hello")

        lp.sec("ac2: waiting to receive")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"

        lp.sec("ac2: mark seen {}".format(msg))
        msg.mark_seen()

        for ev in ac1._evtracker.iter_events():
            if ev.name == "DC_EVENT_INCOMING_MSG":
                pytest.fail("MDN arrived as regular incoming message")
            elif ev.name == "DC_EVENT_MSG_READ":
                break

    def test_send_and_receive_image(self, acfactory, lp, data):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = ac1.create_chat(ac2)

        message_queue = queue.Queue()

        class InPlugin:
            @account_hookimpl
            def ac_incoming_message(self, message):
                message_queue.put(message)

        delivered = queue.Queue()
        out = queue.Queue()

        class OutPlugin:
            @account_hookimpl
            def ac_message_delivered(self, message):
                delivered.put(message)

            @account_hookimpl
            def ac_outgoing_message(self, message):
                out.put(message)

        ac1.add_account_plugin(OutPlugin())
        ac2.add_account_plugin(InPlugin())

        lp.sec("sending image message from ac1 to ac2")
        path = data.get_path("d.png")
        msg_out = chat.send_image(path)
        ac1._evtracker.wait_msg_delivered(msg_out)
        m = out.get()
        assert m == msg_out
        m = delivered.get()
        assert m == msg_out

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evtracker.get_matching("DC_EVENT_MSGS_CHANGED|DC_EVENT_INCOMING_MSG")
        assert ev.data2 == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.is_image()
        assert os.path.exists(msg_in.filename)
        assert os.stat(msg_in.filename).st_size == os.stat(path).st_size
        m = message_queue.get()
        assert m == msg_in

    def test_import_export_online_all(self, acfactory, tmpdir, data, lp):
        ac1 = acfactory.get_one_online_account()

        lp.sec("create some chat content")
        chat1 = ac1.create_contact("some1@example.org", name="some1").create_chat()
        chat1.send_text("msg1")
        assert len(ac1.get_contacts(query="some1")) == 1

        original_image_path = data.get_path("d.png")
        chat1.send_image(original_image_path)

        # Add another 100KB file that ensures that the progress is smooth enough
        path = tmpdir.join("attachment.txt")
        with open(path, "w") as file:
            file.truncate(100000)
        chat1.send_file(path.strpath)

        def assert_account_is_proper(ac):
            contacts = ac.get_contacts(query="some1")
            assert len(contacts) == 1
            contact2 = contacts[0]
            assert contact2.addr == "some1@example.org"
            chat2 = contact2.create_chat()
            messages = chat2.get_messages()
            assert len(messages) == 3
            assert messages[0].text == "msg1"
            assert messages[1].filemime == "image/png"
            assert os.stat(messages[1].filename).st_size == os.stat(original_image_path).st_size
            ac.set_config("displayname", "new displayname")
            assert ac.get_config("displayname") == "new displayname"

        assert_account_is_proper(ac1)

        backupdir = tmpdir.mkdir("backup")

        lp.sec("export all to {}".format(backupdir))
        with ac1.temp_plugin(ImexTracker()) as imex_tracker:

            ac1.stop_io()
            ac1.imex(backupdir.strpath, const.DC_IMEX_EXPORT_BACKUP)

            # check progress events for export
            assert imex_tracker.wait_progress(1, progress_upper_limit=249)
            assert imex_tracker.wait_progress(250, progress_upper_limit=499)
            assert imex_tracker.wait_progress(500, progress_upper_limit=749)
            assert imex_tracker.wait_progress(750, progress_upper_limit=999)

            paths = imex_tracker.wait_finish()
            assert len(paths) == 1
            path = paths[0]
            assert os.path.exists(path)
            ac1.start_io()

        lp.sec("get fresh empty account")
        ac2 = acfactory.get_unconfigured_account()

        lp.sec("get latest backup file")
        path2 = ac2.get_latest_backupfile(backupdir.strpath)
        assert path2 == path

        lp.sec("import backup and check it's proper")
        with ac2.temp_plugin(ImexTracker()) as imex_tracker:
            ac2.import_all(path)

            # check progress events for import
            assert imex_tracker.wait_progress(1, progress_upper_limit=249)
            assert imex_tracker.wait_progress(500, progress_upper_limit=749)
            assert imex_tracker.wait_progress(750, progress_upper_limit=999)
            assert imex_tracker.wait_progress(1000)

        assert_account_is_proper(ac1)
        assert_account_is_proper(ac2)

        lp.sec("Second-time export all to {}".format(backupdir))
        ac1.stop_io()
        path2 = ac1.export_all(backupdir.strpath)
        assert os.path.exists(path2)
        assert path2 != path
        assert ac2.get_latest_backupfile(backupdir.strpath) == path2

    def test_ac_setup_message(self, acfactory, lp):
        # note that the receiving account needs to be configured and running
        # before ther setup message is send. DC does not read old messages
        # as of Jul2019
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.clone_online_account(ac1)
        acfactory.wait_configure_and_start_io()

        lp.sec("trigger ac setup message and return setupcode")
        assert ac1.get_info()["fingerprint"] != ac2.get_info()["fingerprint"]
        setup_code = ac1.initiate_key_transfer()
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        msg = ac2.get_message_by_id(ev.data2)
        assert msg.is_setup_message()
        assert msg.get_setupcodebegin() == setup_code[:2]
        lp.sec("try a bad setup code")
        with pytest.raises(ValueError):
            msg.continue_key_transfer(str(reversed(setup_code)))
        lp.sec("try a good setup code")
        print("*************** Incoming ASM File at: ", msg.filename)
        print("*************** Setup Code: ", setup_code)
        msg.continue_key_transfer(setup_code)
        assert ac1.get_info()["fingerprint"] == ac2.get_info()["fingerprint"]

    def test_ac_setup_message_twice(self, acfactory, lp):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.clone_online_account(ac1)
        acfactory.wait_configure_and_start_io()

        lp.sec("trigger ac setup message but ignore")
        assert ac1.get_info()["fingerprint"] != ac2.get_info()["fingerprint"]
        ac1.initiate_key_transfer()
        ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")

        lp.sec("trigger second ac setup message, wait for receive ")
        setup_code2 = ac1.initiate_key_transfer()
        ev = ac2._evtracker.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        msg = ac2.get_message_by_id(ev.data2)
        assert msg.is_setup_message()
        assert msg.get_setupcodebegin() == setup_code2[:2]

        lp.sec("process second setup message")
        msg.continue_key_transfer(setup_code2)
        assert ac1.get_info()["fingerprint"] == ac2.get_info()["fingerprint"]

    def test_qr_setup_contact(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        lp.sec("ac1: create QR code and let ac2 scan it, starting the securejoin")
        qr = ac1.get_setup_contact_qr()

        lp.sec("ac2: start QR-code based setup contact protocol")
        ch = ac2.qr_setup_contact(qr)
        assert ch.id >= 10
        ac1._evtracker.wait_securejoin_inviter_progress(1000)

    def test_qr_join_chat(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        lp.sec("ac1: create QR code and let ac2 scan it, starting the securejoin")
        chat = ac1.create_group_chat("hello")
        qr = chat.get_join_qr()
        lp.sec("ac2: start QR-code based join-group protocol")
        ch = ac2.qr_join_chat(qr)
        lp.sec("ac2: qr_join_chat() returned")
        assert ch.id >= 10
        # check that at least some of the handshake messages are deleted
        ac1._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_DELETED")
        ac2._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_DELETED")
        ac1._evtracker.wait_securejoin_inviter_progress(1000)

    def test_qr_verified_group_and_chatting(self, acfactory, lp):
        ac1, ac2, ac3 = acfactory.get_many_online_accounts(3)
        lp.sec("ac1: create verified-group QR, ac2 scans and joins")
        chat1 = ac1.create_group_chat("hello", verified=True)
        assert chat1.is_protected()
        qr = chat1.get_join_qr()
        lp.sec("ac2: start QR-code based join-group protocol")
        chat2 = ac2.qr_join_chat(qr)
        assert chat2.id >= 10
        ac1._evtracker.wait_securejoin_inviter_progress(1000)

        lp.sec("ac2: read member added message")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.is_encrypted()
        assert "added" in msg.text.lower()

        lp.sec("ac1: send message")
        msg_out = chat1.send_text("hello")
        assert msg_out.is_encrypted()

        lp.sec("ac2: read message and check it's verified chat")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"
        assert msg.chat.is_protected()
        assert msg.is_encrypted()

        lp.sec("ac2: send message and let ac1 read it")
        chat2.send_text("world")
        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.text == "world"
        assert msg.is_encrypted()

        lp.sec("ac1: create QR code and let ac3 scan it, starting the securejoin")
        qr = ac1.get_setup_contact_qr()

        lp.sec("ac3: start QR-code based setup contact protocol")
        ch = ac3.qr_setup_contact(qr)
        assert ch.id >= 10
        ac1._evtracker.wait_securejoin_inviter_progress(1000)

        lp.sec("ac1: add ac3 to verified group")
        chat1.add_contact(ac3)
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.is_encrypted()
        assert msg.is_system_message()
        assert not msg.error

        lp.sec("ac2: send message and let ac3 read it")
        chat2.send_text("hi")
        # Skip system message about added member
        ac3._evtracker.wait_next_incoming_message()
        msg = ac3._evtracker.wait_next_incoming_message()
        assert msg.text == "hi"
        assert msg.is_encrypted()

    def test_set_get_contact_avatar(self, acfactory, data, lp):
        lp.sec("configuring ac1 and ac2")
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("set ac1 and ac2 profile images")
        p = data.get_path("d.png")
        ac1.set_avatar(p)
        ac2.set_avatar(p)

        lp.sec("ac1: send message to ac2")
        ac1.create_chat(ac2).send_text("with avatar!")

        lp.sec("ac2: wait for receiving message and avatar from ac1")
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.chat.is_contact_request()
        received_path = msg2.get_sender_contact().get_profile_image()
        assert open(received_path, "rb").read() == open(p, "rb").read()

        lp.sec("ac2: send back message")
        msg3 = msg2.create_chat().send_text("yes, i received your avatar -- how do you like mine?")
        assert msg3.is_encrypted()

        lp.sec("ac1: wait for receiving message and avatar from ac2")
        msg4 = ac1._evtracker.wait_next_incoming_message()
        received_path = msg4.get_sender_contact().get_profile_image()
        assert received_path is not None, "did not get avatar through encrypted message"
        assert open(received_path, "rb").read() == open(p, "rb").read()

        ac2._evtracker.consume_events()
        ac1._evtracker.consume_events()

        lp.sec("ac1: delete profile image from chat, and send message to ac2")
        ac1.set_avatar(None)
        msg5 = ac1.create_chat(ac2).send_text("removing my avatar")
        assert msg5.is_encrypted()

        lp.sec("ac2: wait for message along with avatar deletion of ac1")
        msg6 = ac2._evtracker.wait_next_incoming_message()
        assert msg6.get_sender_contact().get_profile_image() is None

    def test_add_remove_member_remote_events(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        ac1_addr = ac1.get_config("addr")
        # activate local plugin for ac2
        in_list = queue.Queue()

        class EventHolder:
            def __init__(self, **kwargs):
                self.__dict__.update(kwargs)

        class InPlugin:
            @account_hookimpl
            def ac_incoming_message(self, message):
                # we immediately accept the sender because
                # otherwise we won't see member_added contacts
                message.create_chat()

            @account_hookimpl
            def ac_chat_modified(self, chat):
                in_list.put(EventHolder(action="chat-modified", chat=chat))

            @account_hookimpl
            def ac_member_added(self, chat, contact, message):
                in_list.put(EventHolder(action="added", chat=chat, contact=contact, message=message))

            @account_hookimpl
            def ac_member_removed(self, chat, contact, message):
                in_list.put(EventHolder(action="removed", chat=chat, contact=contact, message=message))

        ac2.add_account_plugin(InPlugin())

        lp.sec("ac1: create group chat with ac2")
        chat = ac1.create_group_chat("hello", contacts=[ac2])

        lp.sec("ac1: send a message to group chat to promote the group")
        chat.send_text("afterwards promoted")
        ev = in_list.get()
        assert ev.action == "chat-modified"
        assert chat.is_promoted()
        assert sorted(x.addr for x in chat.get_contacts()) == \
            sorted(x.addr for x in ev.chat.get_contacts())

        lp.sec("ac1: add address2")
        # note that if the above create_chat() would not
        # happen we would not receive a proper member_added event
        contact2 = chat.add_contact("devnull@testrun.org")
        ev = in_list.get()
        assert ev.action == "chat-modified"
        ev = in_list.get()
        assert ev.action == "chat-modified"
        ev = in_list.get()
        assert ev.action == "added"
        assert ev.message.get_sender_contact().addr == ac1_addr
        assert ev.contact.addr == "devnull@testrun.org"

        lp.sec("ac1: remove address2")
        chat.remove_contact(contact2)
        ev = in_list.get()
        assert ev.action == "chat-modified"
        ev = in_list.get()
        assert ev.action == "removed"
        assert ev.contact.addr == contact2.addr
        assert ev.message.get_sender_contact().addr == ac1_addr

        lp.sec("ac1: remove ac2 contact from chat")
        chat.remove_contact(ac2)
        ev = in_list.get()
        assert ev.action == "chat-modified"
        ev = in_list.get()
        assert ev.action == "removed"
        assert ev.message.get_sender_contact().addr == ac1_addr

    def test_system_group_msg_from_blocked_user(self, acfactory, lp):
        """
        Tests that a blocked user removes you from a group.
        The message has to be fetched even though the user is blocked
        to avoid inconsistent group state.
        Also tests blocking in general.
        """
        lp.sec("Create a group chat with ac1 and ac2")
        (ac1, ac2) = acfactory.get_two_online_accounts()
        acfactory.introduce_each_other((ac1, ac2))
        chat_on_ac1 = ac1.create_group_chat("title", contacts=[ac2])
        chat_on_ac1.send_text("First group message")
        chat_on_ac2 = ac2._evtracker.wait_next_incoming_message().chat

        lp.sec("ac1 blocks ac2")
        contact = ac1.create_contact(ac2)
        contact.block()
        assert contact.is_blocked()
        ev = ac1._evtracker.get_matching("DC_EVENT_CONTACTS_CHANGED")
        assert ev.data1 == contact.id

        lp.sec("ac2 sends a message to ac1 that does not arrive because it is blocked")
        ac2.create_chat(ac1).send_text("This will not arrive!")

        lp.sec("ac2 sends a group message to ac1 that arrives")
        # Groups would be hardly usable otherwise: If you have blocked some
        # users, they write messages and you only see replies to them without context
        chat_on_ac2.send_text("This will arrive")
        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.text == "This will arrive"
        message_texts = [m.text for m in chat_on_ac1.get_messages()]
        assert len(message_texts) == 2
        assert "First group message" in message_texts
        assert "This will arrive" in message_texts

        lp.sec("ac2 removes ac1 from their group")
        assert ac1.get_self_contact() in chat_on_ac1.get_contacts()
        assert contact.is_blocked()
        chat_on_ac2.remove_contact(ac1)
        ac1._evtracker.get_matching("DC_EVENT_CHAT_MODIFIED")
        assert not ac1.get_self_contact() in chat_on_ac1.get_contacts()

    def test_set_get_group_image(self, acfactory, data, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("create unpromoted group chat")
        chat = ac1.create_group_chat("hello")
        p = data.get_path("d.png")

        lp.sec("ac1: set profile image on unpromoted chat")
        chat.set_profile_image(p)
        ac1._evtracker.get_matching("DC_EVENT_CHAT_MODIFIED")
        assert not chat.is_promoted()

        lp.sec("ac1: send text to promote chat (XXX without contact added)")
        # XXX first promote the chat before adding contact
        # because DC does not send out profile images for unpromoted chats
        # otherwise
        chat.send_text("ac1: initial message to promote chat (workaround)")
        assert chat.is_promoted()
        assert chat.get_profile_image()

        lp.sec("ac2: check that initial message arrived")
        ac2.create_contact(ac1).create_chat()
        ac2._evtracker.get_matching("DC_EVENT_MSGS_CHANGED")

        lp.sec("ac1: add ac2 to promoted group chat")
        chat.add_contact(ac2)  # sends one message

        lp.sec("ac1: send a first message to ac2")
        chat.send_text("hi")  # sends another message
        assert chat.is_promoted()

        lp.sec("ac2: wait for receiving message from ac1")
        msg1 = ac2._evtracker.wait_next_incoming_message()
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg1.text == "hi" or msg2.text == "hi"
        assert msg1.chat.id == msg2.chat.id

        lp.sec("ac2: see if chat now has got the profile image")
        p2 = msg1.chat.get_profile_image()
        assert p2 is not None
        assert open(p2, "rb").read() == open(p, "rb").read()

        ac2._evtracker.consume_events()
        ac1._evtracker.consume_events()

        lp.sec("ac2: delete profile image from chat")
        msg1.chat.remove_profile_image()
        msg_back = ac1._evtracker.wait_next_incoming_message()
        assert msg_back.chat == chat
        assert chat.get_profile_image() is None

    def test_connectivity(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        ac1.set_config("scan_all_folders_debounce_secs", "0")

        ac1._evtracker.wait_for_connectivity(const.DC_CONNECTIVITY_CONNECTED)

        lp.sec("Test stop_io() and start_io()")
        ac1.stop_io()
        ac1._evtracker.wait_for_connectivity(const.DC_CONNECTIVITY_NOT_CONNECTED)

        ac1.start_io()
        ac1._evtracker.wait_for_connectivity(const.DC_CONNECTIVITY_CONNECTING)
        ac1._evtracker.wait_for_connectivity_change(const.DC_CONNECTIVITY_CONNECTING, const.DC_CONNECTIVITY_CONNECTED)

        lp.sec("Test that after calling start_io(), maybe_network() and waiting for `all_work_done()`, " +
               "all messages are fetched")

        ac1.direct_imap.select_config_folder("inbox")
        ac1.direct_imap.idle_start()
        ac2.create_chat(ac1).send_text("Hi")

        ac1.direct_imap.idle_wait_for_new_message(terminate=True)
        ac1.maybe_network()

        ac1._evtracker.wait_for_all_work_done()
        msgs = ac1.create_chat(ac2).get_messages()
        assert len(msgs) == 1
        assert msgs[0].text == "Hi"

        lp.sec("Test that the connectivity changes to WORKING while new messages are fetched")

        ac2.create_chat(ac1).send_text("Hi 2")

        ac1._evtracker.wait_for_connectivity_change(const.DC_CONNECTIVITY_CONNECTED, const.DC_CONNECTIVITY_WORKING)
        ac1._evtracker.wait_for_connectivity_change(const.DC_CONNECTIVITY_WORKING, const.DC_CONNECTIVITY_CONNECTED)

        msgs = ac1.create_chat(ac2).get_messages()
        assert len(msgs) == 2
        assert msgs[1].text == "Hi 2"

        lp.sec("Test that the connectivity doesn't flicker to WORKING if there are no new messages")

        ac1.maybe_network()
        while 1:
            assert ac1.get_connectivity() == const.DC_CONNECTIVITY_CONNECTED
            if ac1.all_work_done():
                break
            ac1._evtracker.get_matching("DC_EVENT_CONNECTIVITY_CHANGED")

        lp.sec("Test that the connectivity doesn't flicker to WORKING if the sender of the message is blocked")
        ac1.create_contact(ac2).block()

        ac1.direct_imap.select_config_folder("inbox")
        ac1.direct_imap.idle_start()
        ac2.create_chat(ac1).send_text("Hi")

        ac1.direct_imap.idle_wait_for_new_message(terminate=True)
        ac1.maybe_network()

        while 1:
            assert ac1.get_connectivity() == const.DC_CONNECTIVITY_CONNECTED
            if ac1.all_work_done():
                break
            ac1._evtracker.get_matching("DC_EVENT_CONNECTIVITY_CHANGED")

        lp.sec("Test that the connectivity is NOT_CONNECTED if the password is wrong")

        ac1.set_config("configured_mail_pw", "abc")
        ac1.stop_io()
        ac1._evtracker.wait_for_connectivity(const.DC_CONNECTIVITY_NOT_CONNECTED)
        ac1.start_io()
        ac1._evtracker.wait_for_connectivity(const.DC_CONNECTIVITY_CONNECTING)
        ac1._evtracker.wait_for_connectivity(const.DC_CONNECTIVITY_NOT_CONNECTED)

    def test_fetch_deleted_msg(self, acfactory, lp):
        """This is a regression test: Messages with \\Deleted flag were downloaded again and again,
        hundreds of times, because uid_next was not updated.

        See https://github.com/deltachat/deltachat-core-rust/issues/2429.
        """
        ac1 = acfactory.get_one_online_account()
        ac1.stop_io()

        ac1.direct_imap.append("INBOX", """
            From: alice <alice@example.org>
            Subject: subj
            To: bob@example.com
            Chat-Version: 1.0
            Message-ID: <aepiors@example.org>
            Content-Type: text/plain; charset=utf-8

            Deleted message
        """)
        ac1.direct_imap.delete("1:*", expunge=False)
        ac1.start_io()

        for ev in ac1._evtracker.iter_events():
            if ev.name == "DC_EVENT_MSGS_CHANGED":
                pytest.fail("A deleted message was shown to the user")

            if ev.name == "DC_EVENT_INFO" and "1 mails read from" in ev.data2:
                break

        # The message was downloaded once, now check that it's not downloaded again

        for ev in ac1._evtracker.iter_events():
            if ev.name == "DC_EVENT_INFO" and "1 mails read from" in ev.data2:
                pytest.fail("The same email was read twice")

            if ev.name == "DC_EVENT_MSGS_CHANGED":
                pytest.fail("A deleted message was shown to the user")

            if ev.name == "DC_EVENT_INFO" and "INBOX: Idle entering wait-on-remote state" in ev.data2:
                break  # DC is done with reading messages

    def test_send_receive_locations(self, acfactory, lp):
        now = datetime.now(timezone.utc)
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat1 = ac1.create_chat(ac2)
        chat2 = ac2.create_chat(ac1)

        assert not chat1.is_sending_locations()
        with pytest.raises(ValueError):
            ac1.set_location(latitude=0.0, longitude=10.0)

        ac1._evtracker.consume_events()
        ac2._evtracker.consume_events()

        lp.sec("ac1: enable location sending in chat")
        chat1.enable_sending_locations(seconds=100)
        assert chat1.is_sending_locations()
        ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

        ac1.set_location(latitude=2.0, longitude=3.0, accuracy=0.5)
        ac1._evtracker.get_matching("DC_EVENT_LOCATION_CHANGED")
        chat1.send_text("🍞")
        ac1._evtracker.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

        lp.sec("ac2: wait for incoming location message")

        # currently core emits location changed before event_incoming message
        ac2._evtracker.get_matching("DC_EVENT_LOCATION_CHANGED")

        locations = chat2.get_locations()
        assert len(locations) == 1
        assert locations[0].latitude == 2.0
        assert locations[0].longitude == 3.0
        assert locations[0].accuracy == 0.5
        assert locations[0].timestamp > now
        assert locations[0].marker == "🍞"

        contact = ac2.create_contact(ac1)
        locations2 = chat2.get_locations(contact=contact)
        assert len(locations2) == 1
        assert locations2 == locations

        contact = ac2.create_contact("nonexisting@example.org")
        locations3 = chat2.get_locations(contact=contact)
        assert not locations3

    def test_undecipherable_group(self, acfactory, lp):
        """Test how group messages that cannot be decrypted are
        handled.

        Group name is encrypted and plaintext subject is set to "..." in
        this case, so we should assign the messages to existing chat
        instead of creating a new one. Since there is no existing group
        chat, the messages should be assigned to 1-1 chat with the sender
        of the message.
        """

        lp.sec("creating and configuring three accounts")
        ac1, ac2, ac3 = acfactory.get_many_online_accounts(3)

        acfactory.introduce_each_other([ac1, ac2, ac3])

        lp.sec("ac3 reinstalls DC and generates a new key")
        ac3.stop_io()
        ac4 = acfactory.clone_online_account(ac3, pre_generated_key=False)
        ac4._configtracker.wait_finish()
        # Create contacts to make sure incoming messages are not treated as contact requests
        chat41 = ac4.create_chat(ac1)
        chat42 = ac4.create_chat(ac2)
        ac4.start_io()
        ac4._evtracker.wait_all_initial_fetches()

        lp.sec("ac1: creating group chat with 2 other members")
        chat = ac1.create_group_chat("title", contacts=[ac2, ac3])

        lp.sec("ac1: send message to new group chat")
        msg = chat.send_text("hello")

        lp.sec("ac2: checking that the chat arrived correctly")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"
        assert msg.is_encrypted(), "Message is not encrypted"

        # ac4 cannot decrypt the message.
        # Error message should be assigned to the chat with ac1.
        lp.sec("ac4: checking that message is assigned to the sender chat")
        error_msg = ac4._evtracker.wait_next_incoming_message()
        assert error_msg.error  # There is an error decrypting the message
        assert error_msg.chat == chat41

        lp.sec("ac2: sending a reply to the chat")
        msg.chat.send_text("reply")
        reply = ac1._evtracker.wait_next_incoming_message()
        assert reply.text == "reply"
        assert reply.is_encrypted(), "Reply is not encrypted"

        lp.sec("ac4: checking that reply is assigned to ac2 chat")
        error_reply = ac4._evtracker.wait_next_incoming_message()
        assert error_reply.error  # There is an error decrypting the message
        assert error_reply.chat == chat42

        # Test that ac4 replies to error messages don't appear in the
        # group chat on ac1 and ac2.
        lp.sec("ac4: replying to ac1 and ac2")

        # Otherwise reply becomes a contact request.
        chat41.send_text("I can't decrypt your message, ac1!")
        chat42.send_text("I can't decrypt your message, ac2!")

        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.error is None
        assert msg.text == "I can't decrypt your message, ac1!"
        assert msg.is_encrypted(), "Message is not encrypted"
        assert msg.chat == ac1.create_chat(ac3)

        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.error is None
        assert msg.text == "I can't decrypt your message, ac2!"
        assert msg.is_encrypted(), "Message is not encrypted"
        assert msg.chat == ac2.create_chat(ac4)

    def test_immediate_autodelete(self, acfactory, lp):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account(move=False, sentbox=False)

        # "1" means delete immediately, while "0" means do not delete
        ac2.set_config("delete_server_after", "1")

        acfactory.wait_configure_and_start_io()

        lp.sec("ac1: create chat with ac2")
        chat1 = ac1.create_chat(ac2)
        ac2.create_chat(ac1)

        lp.sec("ac1: send message to ac2")
        sent_msg = chat1.send_text("hello")

        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"

        lp.sec("ac2: wait for close/expunge on autodelete")
        ac2._evtracker.get_info_contains("close/expunge succeeded")

        lp.sec("ac2: check that message was autodeleted on server")
        assert len(ac2.direct_imap.get_all_messages()) == 0

        lp.sec("ac2: Mark deleted message as seen and check that read receipt arrives")
        msg.mark_seen()
        ev = ac1._evtracker.get_matching("DC_EVENT_MSG_READ")
        assert ev.data1 == chat1.id
        assert ev.data2 == sent_msg.id

    def test_ephemeral_timer(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat1 = ac1.create_chat(ac2)
        chat2 = ac2.create_chat(ac1)

        lp.sec("ac1: set ephemeral timer to 60")
        chat1.set_ephemeral_timer(60)

        lp.sec("ac1: check that ephemeral timer is set for chat")
        assert chat1.get_ephemeral_timer() == 60
        chat1_summary = chat1.get_summary()
        assert chat1_summary["ephemeral_timer"] == {'Enabled': {'duration': 60}}

        lp.sec("ac2: receive system message about ephemeral timer modification")
        ac2._evtracker.get_matching("DC_EVENT_CHAT_EPHEMERAL_TIMER_MODIFIED")
        system_message1 = ac2._evtracker.wait_next_incoming_message()
        assert chat2.get_ephemeral_timer() == 60
        assert system_message1.is_system_message()

        # Disabled until markers are implemented
        # assert "Ephemeral timer: 60\n" in system_message1.get_message_info()

        lp.sec("ac2: send message to ac1")
        sent_message = chat2.send_text("message")
        assert sent_message.ephemeral_timer == 60
        assert "Ephemeral timer: 60\n" in sent_message.get_message_info()

        # Timer is started immediately for sent messages
        assert sent_message.ephemeral_timestamp is not None
        assert "Expires: " in sent_message.get_message_info()

        lp.sec("ac1: waiting for message from ac2")
        text_message = ac1._evtracker.wait_next_incoming_message()
        assert text_message.text == "message"
        assert text_message.ephemeral_timer == 60
        assert "Ephemeral timer: 60\n" in text_message.get_message_info()

        # Timer should not start until message is displayed
        assert text_message.ephemeral_timestamp is None
        assert "Expires: " not in text_message.get_message_info()
        text_message.mark_seen()
        text_message = ac1.get_message_by_id(text_message.id)
        assert text_message.ephemeral_timestamp is not None
        assert "Expires: " in text_message.get_message_info()

        lp.sec("ac2: set ephemeral timer to 0")
        chat2.set_ephemeral_timer(0)
        ac2._evtracker.get_matching("DC_EVENT_CHAT_EPHEMERAL_TIMER_MODIFIED")

        lp.sec("ac1: receive system message about ephemeral timer modification")
        ac1._evtracker.get_matching("DC_EVENT_CHAT_EPHEMERAL_TIMER_MODIFIED")
        system_message2 = ac1._evtracker.wait_next_incoming_message()
        assert system_message2.ephemeral_timer is None
        assert "Ephemeral timer: " not in system_message2.get_message_info()
        assert chat1.get_ephemeral_timer() == 0

    def test_delete_multiple_messages(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat12 = acfactory.get_accepted_chat(ac1, ac2)

        lp.sec("ac1: sending seven messages")
        texts = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh"]
        for text in texts:
            chat12.send_text(text)

        lp.sec("ac2: waiting for all messages on the other side")
        to_delete = []
        for text in texts:
            msg = ac2._evtracker.wait_next_incoming_message()
            assert msg.text in texts
            if text != "third":
                to_delete.append(msg)

        lp.sec("ac2: deleting all messages except third")
        assert len(to_delete) == len(texts) - 1
        ac2.delete_messages(to_delete)
        ac2._evtracker.get_matching("DC_EVENT_IMAP_MESSAGE_DELETED")

        ac2._evtracker.get_info_contains("close/expunge succeeded")

        lp.sec("ac2: test that only one message is left")
        ac2.direct_imap.select_config_folder("inbox")
        assert len(ac2.direct_imap.get_all_messages()) == 1

    def test_configure_error_msgs(self, acfactory):
        ac1, configdict = acfactory.get_online_config()
        ac1.update_config(configdict)
        ac1.set_config("mail_pw", "abc")  # Wrong mail pw
        ac1.configure()
        while True:
            ev = ac1._evtracker.get_matching("DC_EVENT_CONFIGURE_PROGRESS")
            if ev.data1 == 0:
                break
        # Password is wrong so it definitely has to say something about "password"
        assert "password" in ev.data2

        ac2, configdict = acfactory.get_online_config()
        ac2.update_config(configdict)
        ac2.set_config("addr", "abc@def.invalid")  # mail server can't be reached
        ac2.configure()
        while True:
            ev = ac2._evtracker.get_matching("DC_EVENT_CONFIGURE_PROGRESS")
            if ev.data1 == 0:
                break
        # Can't connect so it probably should say something about "internet"
        # again, should not repeat itself
        # If this fails then probably `e.msg.to_lowercase().contains("could not resolve")`
        # in configure/mod.rs returned false because the error message was changed
        # (i.e. did not contain "could not resolve" anymore)
        assert (ev.data2.count("internet") + ev.data2.count("network")) == 1
        # Should mention that it can't connect:
        assert ev.data2.count("connect") == 1
        # The users do not know what "configuration" is
        assert "configuration" not in ev.data2.lower()

    def test_name_changes(self, acfactory):
        ac1, ac2 = acfactory.get_two_online_accounts()
        ac1.set_config("displayname", "Account 1")

        # Similar to acfactory.get_accepted_chat, but without setting the contact name.
        ac2.create_contact(ac1.get_config("addr")).create_chat()
        chat12 = ac1.create_contact(ac2.get_config("addr")).create_chat()
        contact = None

        def update_name():
            """Send a message from ac1 to ac2 to update the name"""
            nonlocal contact
            chat12.send_text("Hello")
            msg = ac2._evtracker.wait_next_incoming_message()
            contact = msg.get_sender_contact()
            return contact.name

        assert update_name() == "Account 1"

        ac1.set_config("displayname", "Account 1 revision 2")
        assert update_name() == "Account 1 revision 2"

        # Explicitly rename contact on ac2 to "Renamed"
        ac2.create_contact(contact, name="Renamed")
        assert contact.name == "Renamed"
        ev = ac2._evtracker.get_matching("DC_EVENT_CONTACTS_CHANGED")
        assert ev.data1 == contact.id

        # ac1 also renames itself into "Renamed"
        assert update_name() == "Renamed"
        ac1.set_config("displayname", "Renamed")
        assert update_name() == "Renamed"

        # Contact name was set to "Renamed" explicitly before,
        # so it should not be changed.
        ac1.set_config("displayname", "Renamed again")
        updated_name = update_name()
        assert updated_name == "Renamed"

    def test_status(self, acfactory):
        """Test that status is transferred over the network."""
        ac1, ac2 = acfactory.get_two_online_accounts()

        chat12 = acfactory.get_accepted_chat(ac1, ac2)
        ac1.set_config("selfstatus", "New status")
        chat12.send_text("hi")
        msg_received = ac2._evtracker.wait_next_incoming_message()
        assert msg_received.text == "hi"
        assert msg_received.get_sender_contact().status == "New status"

        # Send a reply from ac2 to ac1 so ac1 can send a read receipt.
        reply_msg = msg_received.chat.send_text("reply")
        reply_msg_received = ac1._evtracker.wait_next_incoming_message()
        assert reply_msg_received.text == "reply"

        # Send read receipt from ac1 to ac2.
        # It does not contain the signature.
        ac1.mark_seen_messages([reply_msg_received])
        ev = ac2._evtracker.get_matching("DC_EVENT_MSG_READ")
        assert ev.data1 == reply_msg.chat.id
        assert ev.data2 == reply_msg.id
        assert reply_msg.is_out_mdn_received()

        # Test that the status is not cleared as a result of receiving a read receipt.
        assert msg_received.get_sender_contact().status == "New status"

        ac1.set_config("selfstatus", "")
        chat12.send_text("hello")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"
        assert msg.get_sender_contact().status == ""

    def test_group_quote(self, acfactory, lp):
        """Test quoting in a group with a new member who have not seen the quoted message."""
        ac1, ac2, ac3 = accounts = acfactory.get_many_online_accounts(3)
        acfactory.introduce_each_other(accounts)
        chat = ac1.create_group_chat(name="quote group")
        chat.add_contact(ac2)

        lp.sec("ac1: sending message")
        out_msg = chat.send_text("hello")

        lp.sec("ac2: receiving message")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"

        chat.add_contact(ac3)
        ac2._evtracker.wait_next_incoming_message()
        ac3._evtracker.wait_next_incoming_message()

        lp.sec("ac2: sending reply with a quote")
        reply_msg = Message.new_empty(msg.chat.account, "text")
        reply_msg.set_text("reply")
        reply_msg.quote = msg
        reply_msg = msg.chat.prepare_message(reply_msg)
        assert reply_msg.quoted_text == "hello"
        msg.chat.send_prepared(reply_msg)

        lp.sec("ac3: receiving reply")
        received_reply = ac3._evtracker.wait_next_incoming_message()
        assert received_reply.text == "reply"
        assert received_reply.quoted_text == "hello"
        # ac3 was not in the group and has not received quoted message
        assert received_reply.quote is None

        lp.sec("ac1: receiving reply")
        received_reply = ac1._evtracker.wait_next_incoming_message()
        assert received_reply.text == "reply"
        assert received_reply.quoted_text == "hello"
        assert received_reply.quote.id == out_msg.id

    @pytest.mark.parametrize("folder,move,expected_destination,", [
        ("xyz", False, "xyz"),  # Test that emails are recognized in a random folder but not moved
        ("xyz", True, "DeltaChat"),  # ...emails are found in a random folder and moved to DeltaChat
        ("Spam", False, "INBOX"),  # ...emails are moved from the spam folder to the Inbox
    ])
    # Testrun.org does not support the CREATE-SPECIAL-USE capability, which means that we can't create a folder with
    # the "\Junk" flag (see https://tools.ietf.org/html/rfc6154). So, we can't test spam folder detection by flag.
    def test_scan_folders(self, acfactory, lp, folder, move, expected_destination):
        """Delta Chat periodically scans all folders for new messages to make sure we don't miss any."""
        variant = folder + "-" + str(move) + "-" + expected_destination
        lp.sec("Testing variant " + variant)
        ac1 = acfactory.get_online_configuring_account(move=move)
        ac2 = acfactory.get_online_configuring_account()

        acfactory.wait_configure(ac1)
        ac1.direct_imap.create_folder(folder)

        acfactory.wait_configure_and_start_io()
        # Wait until each folder was selected once and we are IDLEing:
        ac1._evtracker.get_info_contains("INBOX: Idle entering wait-on-remote state")
        ac1.stop_io()

        # Send a message to ac1 and move it to the mvbox:
        ac1.direct_imap.select_config_folder("inbox")
        ac1.direct_imap.idle_start()
        acfactory.get_accepted_chat(ac2, ac1).send_text("hello")
        ac1.direct_imap.idle_wait_for_new_message(terminate=True)
        ac1.direct_imap.conn.move(["*"], folder)  # "*" means "biggest UID in mailbox"

        lp.sec("Everything prepared, now see if DeltaChat finds the message (" + variant + ")")
        ac1.set_config("scan_all_folders_debounce_secs", "0")
        ac1.start_io()
        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"

        # The message has been downloaded, which means it has reached its destination.
        ac1.direct_imap.select_folder(expected_destination)
        assert len(ac1.direct_imap.get_all_messages()) == 1
        if folder != expected_destination:
            ac1.direct_imap.select_folder(folder)
            assert len(ac1.direct_imap.get_all_messages()) == 0

    @pytest.mark.parametrize("mvbox_move", [False, True])
    def test_fetch_existing(self, acfactory, lp, mvbox_move):
        """Delta Chat reads the recipients from old emails sent by the user and adds them as contacts.
        This way, we can already offer them some email addresses they can write to.

        Also, the newest existing emails from each folder are fetched during onboarding.

        Additionally tests that bcc_self messages moved to the mvbox/sentbox are marked as read."""

        def assert_folders_configured(ac):
            """There was a bug that scan_folders() set the configured folders to None under some circumstances.
            So, check that they are still configured:"""
            assert ac.get_config("configured_sentbox_folder") == "Sent"
            if mvbox_move:
                assert ac.get_config("configured_mvbox_folder")

        ac1 = acfactory.get_online_configuring_account(move=mvbox_move)
        ac2 = acfactory.get_online_configuring_account()

        acfactory.wait_configure(ac1)

        ac1.direct_imap.create_folder("Sent")
        ac1.set_config("sentbox_watch", "1")

        # We need to reconfigure to find the new "Sent" folder.
        # `scan_folders()`, which runs automatically shortly after `start_io()` is invoked,
        # would also find the "Sent" folder, but it would be too late:
        # The sentbox thread, started by `start_io()`, would have seen that there is no
        # ConfiguredSentboxFolder and do nothing.
        ac1._configtracker = ac1.configure(reconfigure=True)
        acfactory.wait_configure_and_start_io()
        assert_folders_configured(ac1)

        assert ac1.direct_imap.select_config_folder("mvbox" if mvbox_move else "inbox")
        ac1.direct_imap.idle_start()

        lp.sec("send out message with bcc to ourselves")
        ac1.set_config("bcc_self", "1")
        chat = acfactory.get_accepted_chat(ac1, ac2)
        chat.send_text("message text")
        assert_folders_configured(ac1)

        lp.sec("wait until the bcc_self message arrives in correct folder and is marked seen")
        assert ac1.direct_imap.idle_wait_for_seen()
        assert_folders_configured(ac1)

        lp.sec("create a cloned ac1 and fetch contact history during configure")
        ac1_clone = acfactory.clone_online_account(ac1)
        ac1_clone.set_config("fetch_existing_msgs", "1")
        ac1_clone._configtracker.wait_finish()
        ac1_clone.start_io()
        assert_folders_configured(ac1_clone)

        lp.sec("check that ac2 contact was fetchted during configure")
        ac1_clone._evtracker.get_matching("DC_EVENT_CONTACTS_CHANGED")
        ac2_addr = ac2.get_config("addr")
        assert any(c.addr == ac2_addr for c in ac1_clone.get_contacts())
        assert_folders_configured(ac1_clone)

        lp.sec("check that messages changed events arrive for the correct message")
        msg = ac1_clone._evtracker.wait_next_messages_changed()
        assert msg.text == "message text"
        assert_folders_configured(ac1)
        assert_folders_configured(ac1_clone)

    def test_fetch_existing_msgs_group_and_single(self, acfactory, lp):
        """There was a bug concerning fetch-existing-msgs:

        A sent a message to you, adding you to a group. This created a contact request.
        You wrote a message to A, creating a chat.
        ...but the group stayed blocked.
        So, after fetch-existing-msgs you have one contact request and one chat with the same person.

        See https://github.com/deltachat/deltachat-core-rust/issues/2097"""
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()

        acfactory.wait_configure_and_start_io()

        lp.sec("receive a message")
        ac2.create_group_chat("group name", contacts=[ac1]).send_text("incoming, unencrypted group message")
        ac1._evtracker.wait_next_incoming_message()

        lp.sec("send out message with bcc to ourselves")
        ac1.direct_imap.idle_start()
        ac1.set_config("bcc_self", "1")
        ac1.create_chat(ac2).send_text("outgoing, encrypted direct message, creating a chat")

        # now wait until the bcc_self message arrives
        assert ac1.direct_imap.idle_wait_for_seen()

        lp.sec("Clone online account and let it fetch the existing messages")
        ac1_clone = acfactory.clone_online_account(ac1)
        ac1_clone.set_config("fetch_existing_msgs", "1")
        ac1_clone._configtracker.wait_finish()

        ac1_clone.start_io()
        ac1_clone._evtracker.wait_all_initial_fetches()
        chats = ac1_clone.get_chats()
        assert len(chats) == 4  # two newly created chats + self-chat + device-chat
        group_chat = [c for c in chats if c.get_name() == "group name"][0]
        assert group_chat.is_group()
        private_chat = [c for c in chats if c.get_name() == "ac2"][0]
        assert not private_chat.is_group()

        group_messages = group_chat.get_messages()
        assert len(group_messages) == 1
        assert group_messages[0].text == "incoming, unencrypted group message"
        private_messages = private_chat.get_messages()
        # We can't decrypt the message in this chat, so the chat is empty:
        assert len(private_messages) == 0

    def test_delete_deltachat_folder(self, acfactory):
        """Test that DeltaChat folder is recreated if user deletes it manually."""
        ac1 = acfactory.get_online_configuring_account(move=True)
        ac2 = acfactory.get_online_configuring_account()
        acfactory.wait_configure(ac1)

        ac1.direct_imap.conn.folder.delete("DeltaChat")
        assert "DeltaChat" not in ac1.direct_imap.list_folders()
        acfactory.wait_configure_and_start_io()

        ac2.create_chat(ac1).send_text("hello")
        msg = ac1._evtracker.wait_next_incoming_message()
        assert msg.text == "hello"

        assert "DeltaChat" in ac1.direct_imap.list_folders()


class TestGroupStressTests:
    def test_group_many_members_add_leave_remove(self, acfactory, lp):
        accounts = acfactory.get_many_online_accounts(5)
        acfactory.introduce_each_other(accounts)
        ac1, ac5 = accounts.pop(), accounts.pop()

        lp.sec("ac1: creating group chat with 3 other members")
        chat = ac1.create_group_chat("title1", contacts=accounts)

        lp.sec("ac1: send message to new group chat")
        msg1 = chat.send_text("hello")
        assert msg1.is_encrypted()
        gossiped_timestamp = chat.get_summary()["gossiped_timestamp"]
        assert gossiped_timestamp > 0

        assert chat.num_contacts() == 3 + 1

        lp.sec("ac2: checking that the chat arrived correctly")
        ac2 = accounts[0]
        msg2 = ac2._evtracker.wait_next_incoming_message()
        assert msg2.text == "hello"
        print("chat is", msg2.chat)
        assert msg2.chat.num_contacts() == 4

        lp.sec("ac3: checking that 'ac4' is a known contact")
        ac3 = accounts[1]
        msg3 = ac3._evtracker.wait_next_incoming_message()
        assert msg3.text == "hello"
        ac3_contacts = ac3.get_contacts()
        assert len(ac3_contacts) == 4
        ac4_contacts = ac3.get_contacts(query=accounts[2].get_config("addr"))
        assert len(ac4_contacts) == 1

        lp.sec("ac2: removing one contact")
        to_remove = ac2.create_contact(accounts[-1])
        msg2.chat.remove_contact(to_remove)

        lp.sec("ac1: receiving system message about contact removal")
        sysmsg = ac1._evtracker.wait_next_incoming_message()
        assert to_remove.addr in sysmsg.text
        assert sysmsg.chat.num_contacts() == 3

        # Receiving message about removed contact does not reset gossip
        assert chat.get_summary()["gossiped_timestamp"] == gossiped_timestamp

        lp.sec("ac1: sending another message to the chat")
        chat.send_text("hello2")
        msg = ac2._evtracker.wait_next_incoming_message()
        assert msg.text == "hello2"
        assert chat.get_summary()["gossiped_timestamp"] == gossiped_timestamp

        lp.sec("ac1: adding fifth member to the chat")
        chat.add_contact(ac5)
        # Adding contact to chat resets gossiped_timestamp
        assert chat.get_summary()["gossiped_timestamp"] >= gossiped_timestamp

        lp.sec("ac2: receiving system message about contact addition")
        sysmsg = ac2._evtracker.wait_next_incoming_message()
        assert ac5.addr in sysmsg.text
        assert sysmsg.chat.num_contacts() == 4

        lp.sec("ac5: waiting for message about addition to the chat")
        sysmsg = ac5._evtracker.wait_next_incoming_message()
        msg = sysmsg.chat.send_text("hello!")
        # Message should be encrypted because keys of other members are gossiped
        assert msg.is_encrypted()

    def test_synchronize_member_list_on_group_rejoin(self, acfactory, lp):
        """
        Test that user recreates group member list when it joins the group again.
        ac1 creates a group with two other accounts: ac2 and ac3
        Then it removes ac2, removes ac3 and adds ac2 back.
        ac2 did not see that ac3 is removed, so it should rebuild member list from scratch.
        """
        lp.sec("setting up accounts, accepted with each other")
        accounts = acfactory.get_many_online_accounts(3)
        acfactory.introduce_each_other(accounts)
        ac1, ac2, ac3 = accounts

        lp.sec("ac1: creating group chat with 2 other members")
        chat = ac1.create_group_chat("title1", contacts=[ac2, ac3])
        assert not chat.is_promoted()

        lp.sec("ac1: send message to new group chat")
        msg = chat.send_text("hello")
        assert chat.is_promoted() and msg.is_encrypted()

        assert chat.num_contacts() == 3

        lp.sec("checking that the chat arrived correctly")
        for ac in accounts[1:]:
            msg = ac._evtracker.wait_next_incoming_message()
            assert msg.text == "hello"
            print("chat is", msg.chat)
            assert msg.chat.num_contacts() == 3

        lp.sec("ac1: removing ac2")
        chat.remove_contact(ac2)

        lp.sec("ac2: wait for a message about removal from the chat")
        msg = ac2._evtracker.wait_next_incoming_message()

        lp.sec("ac1: removing ac3")
        chat.remove_contact(ac3)

        lp.sec("ac1: adding ac2 back")
        # Group is promoted, message is sent automatically
        assert chat.is_promoted()
        chat.add_contact(ac2)

        lp.sec("ac2: check that ac3 is removed")
        msg = ac2._evtracker.wait_next_incoming_message()

        assert chat.num_contacts() == 2
        assert msg.chat.num_contacts() == 2
        acfactory.dump_imap_summary(sys.stdout)


class TestOnlineConfigureFails:
    def test_invalid_password(self, acfactory):
        ac1, configdict = acfactory.get_online_config()
        ac1.update_config(dict(addr=configdict["addr"], mail_pw="123"))
        configtracker = ac1.configure()
        configtracker.wait_progress(500)
        configtracker.wait_progress(0)

    def test_invalid_user(self, acfactory):
        ac1, configdict = acfactory.get_online_config()
        ac1.update_config(dict(addr="x" + configdict["addr"], mail_pw=configdict["mail_pw"]))
        configtracker = ac1.configure()
        configtracker.wait_progress(500)
        configtracker.wait_progress(0)

    def test_invalid_domain(self, acfactory):
        ac1, configdict = acfactory.get_online_config()
        ac1.update_config((dict(addr=configdict["addr"] + "x", mail_pw=configdict["mail_pw"])))
        configtracker = ac1.configure()
        configtracker.wait_progress(500)
        configtracker.wait_progress(0)

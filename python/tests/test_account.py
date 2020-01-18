from __future__ import print_function
import pytest
import os
import queue
import time
from deltachat import const, Account
from deltachat.message import Message
from datetime import datetime, timedelta
from conftest import wait_configuration_progress, wait_successful_IMAP_SMTP_connection, wait_securejoin_inviter_progress


class TestOfflineAccountBasic:
    def test_wrong_db(self, tmpdir):
        p = tmpdir.join("hello.db")
        p.write("123")
        with pytest.raises(ValueError):
            Account(p.strpath)

    def test_os_name(self, tmpdir):
        p = tmpdir.join("hello.db")
        # we can't easily test if os_name is used in X-Mailer
        # outgoing messages without a full Online test
        # but we at least check Account accepts the arg
        ac1 = Account(p.strpath, os_name="solarpunk")
        ac1.get_info()

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
        with pytest.raises(ValueError):
            ac1.get_self_contact()

    def test_get_info(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        out = ac1.get_infostring()
        assert "number_of_chats=0" in out

    def test_selfcontact_configured(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        me = ac1.get_self_contact()
        assert me.display_name
        assert me.addr

    def test_get_config_fails(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        with pytest.raises(KeyError):
            ac1.get_config("123123")


class TestOfflineContact:
    def test_contact_attr(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact(email="some1@hello.com", name="some1")
        contact2 = ac1.create_contact(email="some1@hello.com", name="some1")
        str(contact1)
        repr(contact1)
        assert contact1 == contact2
        assert contact1.id
        assert contact1.addr == "some1@hello.com"
        assert contact1.display_name == "some1"
        assert not contact1.is_blocked()
        assert not contact1.is_verified()

    def test_get_contacts_and_delete(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact(email="some1@hello.com", name="some1")
        contacts = ac1.get_contacts()
        assert len(contacts) == 1
        assert contact1 in contacts

        assert not ac1.get_contacts(query="some2")
        assert ac1.get_contacts(query="some1")
        assert not ac1.get_contacts(only_verified=True)
        contacts = ac1.get_contacts(with_self=True)
        assert len(contacts) == 2

        assert ac1.delete_contact(contact1)
        assert contact1 not in ac1.get_contacts()

    def test_get_contacts_and_delete_fails(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact(email="some1@example.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        msg = chat.send_text("one message")
        assert not ac1.delete_contact(contact1)
        assert not msg.filemime


class TestOfflineChat:
    @pytest.fixture
    def ac1(self, acfactory):
        return acfactory.get_configured_offline_account()

    @pytest.fixture
    def chat1(self, ac1):
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        assert chat.id > const.DC_CHAT_ID_LAST_SPECIAL, chat.id
        return chat

    def test_display(self, chat1):
        str(chat1)
        repr(chat1)

    def test_chat_by_id(self, chat1):
        chat2 = chat1.account.get_chat_by_id(chat1.id)
        assert chat2 == chat1
        with pytest.raises(ValueError):
            chat1.account.get_chat_by_id(123123)

    def test_chat_idempotent(self, chat1, ac1):
        contact1 = chat1.get_contacts()[0]
        chat2 = ac1.create_chat_by_contact(contact1.id)
        assert chat2.id == chat1.id
        assert chat2.get_name() == chat1.get_name()
        assert chat1 == chat2
        assert not (chat1 != chat2)

        for ichat in ac1.get_chats():
            if ichat.id == chat1.id:
                break
        else:
            pytest.fail("could not find chat")

    def test_group_chat_creation(self, ac1):
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        contact2 = ac1.create_contact("some2@hello.com", name="some2")
        chat = ac1.create_group_chat(name="title1")
        chat.add_contact(contact1)
        chat.add_contact(contact2)
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
        assert d["subtitle"] == chat.get_subtitle()
        assert d["draft"] == "" if chat.get_draft() is None else chat.get_draft()

    def test_group_chat_creation_with_translation(self, ac1):
        ac1.set_stock_translation(const.DC_STR_NEWGROUPDRAFT, "xyz %1$s")
        ac1._evlogger.consume_events()
        with pytest.raises(ValueError):
            ac1.set_stock_translation(const.DC_STR_NEWGROUPDRAFT, "xyz %2$s")
        ac1._evlogger.get_matching("DC_EVENT_WARNING")
        with pytest.raises(ValueError):
            ac1.set_stock_translation(500, "xyz %1$s")
        ac1._evlogger.get_matching("DC_EVENT_WARNING")
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        contact2 = ac1.create_contact("some2@hello.com", name="some2")
        chat = ac1.create_group_chat(name="title1")
        chat.add_contact(contact1)
        chat.add_contact(contact2)
        assert chat.get_name() == "title1"
        assert contact1 in chat.get_contacts()
        assert contact2 in chat.get_contacts()
        assert not chat.is_promoted()
        msg = chat.get_draft()
        assert msg.text == "xyz title1"

    @pytest.mark.parametrize("verified", [True, False])
    def test_group_chat_qr(self, acfactory, ac1, verified):
        ac2 = acfactory.get_configured_offline_account()
        chat = ac1.create_group_chat(name="title1", verified=verified)
        qr = chat.get_join_qr()
        assert ac2.check_qr(qr).is_ask_verifygroup

    def test_get_set_profile_image_simple(self, ac1, data):
        chat = ac1.create_group_chat(name="title1")
        p = data.get_path("d.png")
        chat.set_profile_image(p)
        p2 = chat.get_profile_image()
        assert open(p, "rb").read() == open(p2, "rb").read()
        chat.remove_profile_image()
        assert chat.get_profile_image() is None

    def test_delete_and_send_fails(self, ac1, chat1):
        chat1.delete()
        ac1._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
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

    def test_create_chat_by_message_id(self, ac1, chat1):
        msg = chat1.send_text("msg1")
        assert chat1 == ac1.create_chat_by_message(msg)
        assert chat1 == ac1.create_chat_by_message(msg.id)

    def test_message_image(self, chat1, data, lp):
        with pytest.raises(ValueError):
            chat1.send_image(path="notexists")
        fn = data.get_path("d.png")
        lp.sec("sending image")
        chat1.account._evlogger.consume_events()
        msg = chat1.send_image(fn)
        chat1.account._evlogger.get_matching("DC_EVENT_NEW_BLOB_FILE")
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

    def test_create_chat_mismatch(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        ac2 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        with pytest.raises(ValueError):
            ac2.create_chat_by_contact(contact1)
        chat1 = ac1.create_chat_by_contact(contact1)
        msg = chat1.send_text("hello")
        with pytest.raises(ValueError):
            ac2.create_chat_by_message(msg)

    def test_chat_message_distinctions(self, ac1, chat1):
        past1s = datetime.utcnow() - timedelta(seconds=1)
        msg = chat1.send_text("msg1")
        ts = msg.time_sent
        assert msg.time_received is None
        assert ts.strftime("Y")
        assert past1s < ts
        contact = msg.get_sender_contact()
        assert contact == ac1.get_self_contact()

    def test_basic_configure_ok_addr_setting_forbidden(self, ac1):
        assert ac1.get_config("mail_pw")
        assert ac1.is_configured()
        with pytest.raises(ValueError):
            ac1.set_config("addr", "123@example.org")
        with pytest.raises(ValueError):
            ac1.configure(addr="123@example.org")

    def test_import_export_one_contact(self, acfactory, tmpdir):
        backupdir = tmpdir.mkdir("backup")
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
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

        path = ac1.export_all(backupdir.strpath)
        assert os.path.exists(path)
        ac2 = acfactory.get_unconfigured_account()
        ac2.import_all(path)
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@hello.com"
        chat2 = ac2.create_chat_by_contact(contact2)
        messages = chat2.get_messages()
        assert len(messages) == 2
        assert messages[0].text == "msg1"
        assert os.path.exists(messages[1].filename)

    def test_ac_setup_message_fails(self, ac1):
        with pytest.raises(RuntimeError):
            ac1.initiate_key_transfer()

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

    def test_group_chat_many_members_add_remove(self, ac1, lp):
        lp.sec("ac1: creating group chat with 10 other members")
        chat = ac1.create_group_chat(name="title1")
        contacts = []
        for i in range(10):
            contact = ac1.create_contact("some{}@example.org".format(i))
            contacts.append(contact)
            chat.add_contact(contact)

        num_contacts = len(chat.get_contacts())
        assert num_contacts == 11

        lp.sec("ac1: removing two contacts and checking things are right")
        chat.remove_contact(contacts[9])
        chat.remove_contact(contacts[3])
        assert len(chat.get_contacts()) == 9


class TestOnlineAccount:
    def get_chat(self, ac1, ac2, both_created=False):
        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)
        assert chat.id > const.DC_CHAT_ID_LAST_SPECIAL
        if both_created:
            ac2.create_chat_by_contact(ac2.create_contact(email=ac1.get_config("addr")))
        return chat

    def test_configure_canceled(self, acfactory):
        ac1 = acfactory.get_online_configuring_account()
        wait_configuration_progress(ac1, 200)
        ac1.stop_ongoing()
        wait_configuration_progress(ac1, 0, 0)

    def test_export_import_self_keys(self, acfactory, tmpdir):
        ac1, ac2 = acfactory.get_two_online_accounts()
        dir = tmpdir.mkdir("exportdir")
        export_files = ac1.export_self_keys(dir.strpath)
        assert len(export_files) == 2
        for x in export_files:
            assert x.startswith(dir.strpath)
        ac1._evlogger.consume_events()
        ac1.import_self_keys(dir.strpath)

    def test_one_account_send_bcc_setting(self, acfactory, lp):
        ac1 = acfactory.get_online_configuring_account()
        ac2_config = acfactory.peek_online_config()
        c2 = ac1.create_contact(email=ac2_config["addr"])
        chat = ac1.create_chat_by_contact(c2)
        assert chat.id > const.DC_CHAT_ID_LAST_SPECIAL
        wait_successful_IMAP_SMTP_connection(ac1)
        wait_configuration_progress(ac1, 1000)

        lp.sec("ac1: setting bcc_self=1")
        ac1.set_config("bcc_self", "1")

        lp.sec("send out message with bcc to ourselves")
        msg_out = chat.send_text("message2")
        ev = ac1._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        # wait for send out (BCC)
        assert ac1.get_config("bcc_self") == "1"
        self_addr = ac1.get_config("addr")
        ev = ac1._evlogger.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")
        assert self_addr in ev[2]
        ev = ac1._evlogger.get_matching("DC_EVENT_DELETED_BLOB_FILE")

        ac1._evlogger.consume_events()
        lp.sec("send out message without bcc")
        ac1.set_config("bcc_self", "0")
        msg_out = chat.send_text("message3")
        assert not msg_out.is_forwarded()
        ev = ac1._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        ev = ac1._evlogger.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")
        assert self_addr not in ev[2]
        ev = ac1._evlogger.get_matching("DC_EVENT_DELETED_BLOB_FILE")

    def test_send_file_twice_unicode_filename_mangling(self, tmpdir, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = self.get_chat(ac1, ac2)

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
            ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
            assert ev[2] > const.DC_CHAT_ID_LAST_SPECIAL
            return ac2.get_message_by_id(ev[2])

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
        chat = self.get_chat(ac1, ac2)

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
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev[2] > const.DC_CHAT_ID_LAST_SPECIAL
        msg = ac2.get_message_by_id(ev[2])

        assert open(msg.filename).read() == content
        assert msg.filename.endswith(basename)

    def test_mvbox_sentbox_threads(self, acfactory, lp):
        lp.sec("ac1: start with mvbox thread")
        ac1 = acfactory.get_online_configuring_account(mvbox=True, sentbox=True)

        lp.sec("ac2: start without mvbox/sentbox threads")
        ac2 = acfactory.get_online_configuring_account()

        lp.sec("ac2: waiting for configuration")
        wait_configuration_progress(ac2, 1000)

        lp.sec("ac1: waiting for configuration")
        wait_configuration_progress(ac1, 1000)

        lp.sec("ac1: send message and wait for ac2 to receive it")
        chat = self.get_chat(ac1, ac2)
        chat.send_text("message1")
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev[2] > const.DC_CHAT_ID_LAST_SPECIAL
        lp.sec("test finished")

    def test_move_works(self, acfactory):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account(mvbox=True)
        wait_configuration_progress(ac2, 1000)
        wait_configuration_progress(ac1, 1000)
        chat = self.get_chat(ac1, ac2)
        chat.send_text("message1")
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev[2] > const.DC_CHAT_ID_LAST_SPECIAL
        ev = ac2._evlogger.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")

    def test_move_works_on_self_sent(self, acfactory):
        ac1 = acfactory.get_online_configuring_account(mvbox=True)
        ac1.set_config("bcc_self", "1")
        ac2 = acfactory.get_online_configuring_account()
        wait_configuration_progress(ac2, 1000)
        wait_configuration_progress(ac1, 1000)
        chat = self.get_chat(ac1, ac2)
        chat.send_text("message1")
        chat.send_text("message2")
        chat.send_text("message3")
        ac1._evlogger.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")
        ac1._evlogger.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")
        ac1._evlogger.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")

    def test_forward_messages(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = self.get_chat(ac1, ac2)

        lp.sec("ac1: send message to ac2")
        msg_out = chat.send_text("message2")

        lp.sec("ac2: wait for receive")
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.text == "message2"

        lp.sec("ac2: check that the message arrive in deaddrop")
        chat2 = msg_in.chat
        assert msg_in in chat2.get_messages()
        assert not msg_in.is_forwarded()
        assert chat2.is_deaddrop()
        assert chat2 == ac2.get_deaddrop_chat()

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
        chat = self.get_chat(ac1, ac2, both_created=True)

        lp.sec("sending message")
        msg_out = chat.send_text("message2")

        lp.sec("receiving message")
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        msg_in = ac2.get_message_by_id(ev[2])
        assert msg_in.text == "message2"
        assert not msg_in.is_forwarded()

        lp.sec("ac1: creating group chat, and forward own message")
        group = ac1.create_group_chat("newgroup2")
        group.add_contact(ac1.create_contact(ac2.get_config("addr")))
        ac1.forward_messages([msg_out], group)

        # wait for other account to receive
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        msg_in = ac2.get_message_by_id(ev[2])
        assert msg_in.text == "message2"
        assert msg_in.is_forwarded()

    def test_send_self_message_and_empty_folder(self, acfactory, lp):
        ac1 = acfactory.get_one_online_account()
        lp.sec("ac1: create self chat")
        chat = ac1.create_chat_by_contact(ac1.get_self_contact())
        chat.send_text("hello")
        ac1._evlogger.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")
        ac1.empty_server_folders(inbox=True, mvbox=True)
        ev = ac1._evlogger.get_matching("DC_EVENT_IMAP_FOLDER_EMPTIED")
        assert ev[2] == "DeltaChat"
        ev = ac1._evlogger.get_matching("DC_EVENT_IMAP_FOLDER_EMPTIED")
        assert ev[2] == "INBOX"

    def test_send_and_receive_message_markseen(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        # make DC's life harder wrt to encodings
        ac1.set_config("displayname", "ä name")

        lp.sec("ac1: create chat with ac2")
        chat = self.get_chat(ac1, ac2)

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")
        ev = ac1._evlogger.get_matching("DC_EVENT_MSG_DELIVERED")
        evt_name, data1, data2 = ev
        assert data1 == chat.id
        assert data2 == msg_out.id
        assert msg_out.is_out_delivered()

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.text == "message1"
        assert not msg_in.is_forwarded()
        assert msg_in.get_sender_contact().display_name == ac1.get_config("displayname")

        lp.sec("check the message arrived in contact-requets/deaddrop")
        chat2 = msg_in.chat
        assert msg_in in chat2.get_messages()
        assert chat2.is_deaddrop()
        assert chat2.count_fresh_messages() == 0
        assert msg_in.time_received > msg_in.time_sent

        lp.sec("create new chat with contact and verify it's proper")
        chat2b = ac2.create_chat_by_message(msg_in)
        assert not chat2b.is_deaddrop()
        assert chat2b.count_fresh_messages() == 1

        lp.sec("mark chat as noticed")
        chat2b.mark_noticed()
        assert chat2b.count_fresh_messages() == 0

        ac2._evlogger.consume_events()

        lp.sec("sending a second message from ac1 to ac2")
        msg_out2 = chat.send_text("message2")

        lp.sec("wait for ac2 to receive second message")
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        assert ev[2] == msg_out2.id
        msg_in2 = ac2.get_message_by_id(msg_out2.id)

        lp.sec("mark messages as seen on ac2, wait for changes on ac1")
        ac2.mark_seen_messages([msg_in, msg_in2])
        lp.step("1")
        for i in range(2):
            ev = ac1._evlogger.get_matching("DC_EVENT_MSG_READ")
            assert ev[1] > const.DC_CHAT_ID_LAST_SPECIAL
            assert ev[2] > const.DC_MSG_ID_LAST_SPECIAL
        lp.step("2")
        assert msg_out.is_out_mdn_received()
        assert msg_out2.is_out_mdn_received()

        lp.sec("check that a second call to mark_seen does not create change or smtp job")
        ac2._evlogger.consume_events()
        ac2.mark_seen_messages([msg_in])
        try:
            ac2._evlogger.get_matching("DC_EVENT_MSG_READ", timeout=0.01)
        except queue.Empty:
            pass  # mark_seen_messages() has generated events before it returns

    def test_mdn_asymetric(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat = self.get_chat(ac1, ac2, both_created=True)

        # make sure mdns are enabled (usually enabled by default already)
        ac1.set_config("mdns_enabled", "1")
        ac2.set_config("mdns_enabled", "1")

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")

        assert len(chat.get_messages()) == 1

        lp.sec("disable ac1 MDNs")
        ac1.set_config("mdns_enabled", "0")

        lp.sec("wait for ac2 to receive message")
        msg = ac2.wait_next_incoming_message()

        assert len(msg.chat.get_messages()) == 1

        lp.sec("ac2: mark incoming message as seen")
        ac2.mark_seen_messages([msg])

        lp.sec("ac1: waiting for incoming activity")
        # MDN should be moved even though MDNs are already disabled
        ac1._evlogger.get_matching("DC_EVENT_IMAP_MESSAGE_MOVED")

        assert len(chat.get_messages()) == 1

        # MDN is received even though MDNs are already disabled
        assert msg_out.is_out_mdn_received()

    def test_send_and_receive_will_encrypt_decrypt(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat = self.get_chat(ac1, ac2)

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")
        assert not msg_out.is_encrypted()

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.text == "message1"

        lp.sec("create new chat with contact and send back (encrypted) message")
        chat2b = ac2.create_chat_by_message(msg_in)
        chat2b.send_text("message-back")

        lp.sec("wait for ac1 to receive message")
        ev = ac1._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        assert ev[1] == chat.id
        assert ev[2] > msg_out.id
        msg_back = ac1.get_message_by_id(ev[2])
        assert msg_back.text == "message-back"
        assert msg_back.is_encrypted()

        # Test that we do not gossip peer keys in 1-to-1 chat,
        # as it makes no sense to gossip to peers their own keys.
        # Gossip is only sent in encrypted messages,
        # and we sent encrypted msg_back right above.
        assert chat2b.get_summary()["gossiped_timestamp"] == 0

        lp.sec("create group chat with two members, one of which has no encrypt state")
        chat = ac1.create_group_chat("encryption test")
        chat.add_contact(ac1.create_contact(ac2.get_config("addr")))
        chat.add_contact(ac1.create_contact("notexisting@testrun.org"))
        msg = chat.send_text("test not encrypt")
        ev = ac1._evlogger.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")
        assert not msg.is_encrypted()

    def test_send_first_message_as_long_unicode_with_cr(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        ac2.set_config("save_mime_headers", "1")

        lp.sec("ac1: create chat with ac2")
        chat = self.get_chat(ac1, ac2, both_created=True)

        lp.sec("sending multi-line non-unicode message from ac1 to ac2")
        text1 = "hello\nworld"
        msg_out = chat.send_text(text1)
        assert not msg_out.is_encrypted()

        lp.sec("sending multi-line unicode text message from ac1 to ac2")
        text2 = "äalis\nthis is ßßÄ"
        msg_out = chat.send_text(text2)
        assert not msg_out.is_encrypted()

        lp.sec("wait for ac2 to receive multi-line non-unicode message")
        msg_in = ac2.wait_next_incoming_message()
        assert msg_in.text == text1

        lp.sec("wait for ac2 to receive multi-line unicode message")
        msg_in = ac2.wait_next_incoming_message()
        assert msg_in.text == text2
        assert ac1.get_config("addr") in msg_in.chat.get_name()

    def test_reply_encrypted(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat = self.get_chat(ac1, ac2)

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")
        assert not msg_out.is_encrypted()

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.text == "message1"
        assert not msg_in.is_encrypted()

        lp.sec("create new chat with contact and send back (encrypted) message")
        chat2b = ac2.create_chat_by_message(msg_in)
        chat2b.send_text("message-back")

        lp.sec("wait for ac1 to receive message")
        ev = ac1._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        assert ev[1] == chat.id
        msg_back = ac1.get_message_by_id(ev[2])
        assert msg_back.text == "message-back"
        assert msg_back.is_encrypted()

        lp.sec("ac1: e2ee_enabled=0 and see if reply is encrypted")
        print("ac1: e2ee_enabled={}".format(ac1.get_config("e2ee_enabled")))
        print("ac2: e2ee_enabled={}".format(ac2.get_config("e2ee_enabled")))
        ac1.set_config("e2ee_enabled", "0")

        # Set unprepared and unencrypted draft to test that it is not
        # taken into account when determining whether last message is
        # encrypted.
        msg_draft = Message.new_empty(ac1, "text")
        msg_draft.set_text("message2 -- should be encrypted")
        chat.set_draft(msg_draft)

        # Get the draft, prepare and send it.
        msg_draft = chat.get_draft()
        msg_out = chat.prepare_message(msg_draft)
        chat.send_prepared(msg_out)

        chat.set_draft(None)
        assert chat.get_draft() is None

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        msg_in = ac2.get_message_by_id(ev[2])
        assert msg_in.text == "message2 -- should be encrypted"
        assert msg_in.is_encrypted()

    def test_saved_mime_on_received_message(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("configure ac2 to save mime headers, create ac1/ac2 chat")
        ac2.set_config("save_mime_headers", "1")
        chat = self.get_chat(ac1, ac2)

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")
        ac1._evlogger.get_matching("DC_EVENT_MSG_DELIVERED")
        assert msg_out.get_mime_headers() is None

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        in_id = ev[2]
        mime = ac2.get_message_by_id(in_id).get_mime_headers()
        assert mime.get_all("From")
        assert mime.get_all("Received")

    def test_send_and_receive_image(self, acfactory, lp, data):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = self.get_chat(ac1, ac2)

        lp.sec("sending image message from ac1 to ac2")
        path = data.get_path("d.png")
        msg_out = chat.send_image(path)
        ev = ac1._evlogger.get_matching("DC_EVENT_MSG_DELIVERED")
        evt_name, data1, data2 = ev
        assert data1 == chat.id
        assert data2 == msg_out.id
        assert msg_out.is_out_delivered()

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.is_image()
        assert os.path.exists(msg_in.filename)
        assert os.stat(msg_in.filename).st_size == os.stat(path).st_size

    def test_import_export_online_all(self, acfactory, tmpdir, lp):
        ac1 = acfactory.get_online_configuring_account()
        wait_configuration_progress(ac1, 1000)

        lp.sec("create some chat content")
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        chat.send_text("msg1")
        backupdir = tmpdir.mkdir("backup")

        lp.sec("export all to {}".format(backupdir))
        path = ac1.export_all(backupdir.strpath)
        assert os.path.exists(path)
        t = time.time()

        lp.sec("get fresh empty account")
        ac2 = acfactory.get_unconfigured_account()

        lp.sec("get latest backup file")
        path2 = ac2.get_latest_backupfile(backupdir.strpath)
        assert path2 == path

        lp.sec("import backup and check it's proper")
        ac2.import_all(path)
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@hello.com"
        chat2 = ac2.create_chat_by_contact(contact2)
        messages = chat2.get_messages()
        assert len(messages) == 1
        assert messages[0].text == "msg1"

        # wait until a second passed since last backup
        # because get_latest_backupfile() shall return the latest backup
        # from a UI it's unlikely anyone manages to export two
        # backups in one second.
        time.sleep(max(0, 1 - (time.time() - t)))
        lp.sec("Second-time export all to {}".format(backupdir))
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
        wait_configuration_progress(ac2, 1000)
        wait_configuration_progress(ac1, 1000)
        lp.sec("trigger ac setup message and return setupcode")
        assert ac1.get_info()["fingerprint"] != ac2.get_info()["fingerprint"]
        setup_code = ac1.initiate_key_transfer()
        ac2._evlogger.set_timeout(30)
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        msg = ac2.get_message_by_id(ev[2])
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
        ac2._evlogger.set_timeout(30)
        wait_configuration_progress(ac2, 1000)
        wait_configuration_progress(ac1, 1000)

        lp.sec("trigger ac setup message but ignore")
        assert ac1.get_info()["fingerprint"] != ac2.get_info()["fingerprint"]
        ac1.initiate_key_transfer()
        ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")

        lp.sec("trigger second ac setup message, wait for receive ")
        setup_code2 = ac1.initiate_key_transfer()
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        msg = ac2.get_message_by_id(ev[2])
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
        wait_securejoin_inviter_progress(ac1, 1000)

    def test_qr_join_chat(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        lp.sec("ac1: create QR code and let ac2 scan it, starting the securejoin")
        chat = ac1.create_group_chat("hello")
        qr = chat.get_join_qr()
        lp.sec("ac2: start QR-code based join-group protocol")
        ch = ac2.qr_join_chat(qr)
        assert ch.id >= 10
        # check that at least some of the handshake messages are deleted
        ac1._evlogger.get_matching("DC_EVENT_IMAP_MESSAGE_DELETED")
        ac2._evlogger.get_matching("DC_EVENT_IMAP_MESSAGE_DELETED")
        wait_securejoin_inviter_progress(ac1, 1000)
        ac1._evlogger.get_matching("DC_EVENT_SECUREJOIN_MEMBER_ADDED")

    def test_qr_verified_group_and_chatting(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        lp.sec("ac1: create verified-group QR, ac2 scans and joins")
        chat1 = ac1.create_group_chat("hello", verified=True)
        assert chat1.is_verified()
        qr = chat1.get_join_qr()
        lp.sec("ac2: start QR-code based join-group protocol")
        chat2 = ac2.qr_join_chat(qr)
        assert chat2.id >= 10
        wait_securejoin_inviter_progress(ac1, 1000)
        ac1._evlogger.get_matching("DC_EVENT_SECUREJOIN_MEMBER_ADDED")

        lp.sec("ac2: read member added message")
        msg = ac2.wait_next_incoming_message()
        assert msg.is_encrypted()
        assert "added" in msg.text.lower()

        lp.sec("ac1: send message")
        msg_out = chat1.send_text("hello")
        assert msg_out.is_encrypted()

        lp.sec("ac2: read message and check it's verified chat")
        msg = ac2.wait_next_incoming_message()
        assert msg.text == "hello"
        assert msg.chat.is_verified()
        assert msg.is_encrypted()

        lp.sec("ac2: send message and let ac1 read it")
        chat2.send_text("world")
        msg = ac1.wait_next_incoming_message()
        assert msg.text == "world"
        assert msg.is_encrypted()

    def test_set_get_contact_avatar(self, acfactory, data, lp):
        lp.sec("configuring ac1 and ac2")
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: set own profile image")
        p = data.get_path("d.png")
        ac1.set_avatar(p)

        lp.sec("ac1: create 1:1 chat with ac2")
        chat = self.get_chat(ac1, ac2, both_created=True)

        msg = chat.send_text("hi -- do you see my brand new avatar?")
        assert not msg.is_encrypted()

        lp.sec("ac2: wait for receiving message and avatar from ac1")
        msg1 = ac2.wait_next_incoming_message()
        assert not msg1.chat.is_deaddrop()
        received_path = msg1.get_sender_contact().get_profile_image()
        assert open(received_path, "rb").read() == open(p, "rb").read()

        lp.sec("ac2: set own profile image")
        p = data.get_path("d.png")
        ac2.set_avatar(p)

        lp.sec("ac2: send back message")
        m = msg1.chat.send_text("yes, i received your avatar -- how do you like mine?")
        assert m.is_encrypted()

        lp.sec("ac1: wait for receiving message and avatar from ac2")
        msg2 = ac1.wait_next_incoming_message()
        received_path = msg2.get_sender_contact().get_profile_image()
        assert received_path is not None, "did not get avatar through encrypted message"
        assert open(received_path, "rb").read() == open(p, "rb").read()

        ac2._evlogger.consume_events()
        ac1._evlogger.consume_events()

        # XXX not sure if the following is correct / possible. you may remove it
        lp.sec("ac1: delete profile image from chat, and send message to ac2")
        ac1.set_avatar(None)
        m = msg2.chat.send_text("i don't like my avatar anymore and removed it")
        assert m.is_encrypted()

        lp.sec("ac2: wait for message along with avatar deletion of ac1")
        msg3 = ac2.wait_next_incoming_message()
        assert msg3.get_sender_contact().get_profile_image() is None

    def test_set_get_group_image(self, acfactory, data, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("create unpromoted group chat")
        chat = ac1.create_group_chat("hello")
        p = data.get_path("d.png")

        lp.sec("ac1: set profile image on unpromoted chat")
        chat.set_profile_image(p)
        ac1._evlogger.get_matching("DC_EVENT_CHAT_MODIFIED")
        assert not chat.is_promoted()

        lp.sec("ac1: send text to promote chat (XXX without contact added)")
        # XXX first promote the chat before adding contact
        # because DC does not send out profile images for unpromoted chats
        # otherwise
        chat.send_text("ac1: initial message to promote chat (workaround)")
        assert chat.is_promoted()

        lp.sec("ac2: add ac1 to a chat so the message does not land in DEADDROP")
        c1 = ac2.create_contact(email=ac1.get_config("addr"))
        ac2.create_chat_by_contact(c1)
        ev = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")

        lp.sec("ac1: add ac2 to promoted group chat")
        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat.add_contact(c2)

        lp.sec("ac1: send a first message to ac2")
        chat.send_text("hi")
        assert chat.is_promoted()

        lp.sec("ac2: wait for receiving message from ac1")
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        msg_in = ac2.get_message_by_id(ev[2])
        assert not msg_in.chat.is_deaddrop()

        lp.sec("ac2: create chat and read profile image")
        chat2 = ac2.create_chat_by_message(msg_in)
        p2 = chat2.get_profile_image()
        assert p2 is not None
        assert open(p2, "rb").read() == open(p, "rb").read()

        ac2._evlogger.consume_events()
        ac1._evlogger.consume_events()
        lp.sec("ac2: delete profile image from chat")
        chat2.remove_profile_image()
        ev = ac1._evlogger.get_matching("DC_EVENT_INCOMING_MSG")
        assert ev[1] == chat.id
        chat1b = ac1.create_chat_by_message(ev[2])
        assert chat1b.get_profile_image() is None
        assert chat.get_profile_image() is None

    def test_send_receive_locations(self, acfactory, lp):
        now = datetime.utcnow()
        ac1, ac2 = acfactory.get_two_online_accounts()

        lp.sec("ac1: create chat with ac2")
        chat1 = self.get_chat(ac1, ac2)
        chat2 = self.get_chat(ac2, ac1)

        assert not chat1.is_sending_locations()
        with pytest.raises(ValueError):
            ac1.set_location(latitude=0.0, longitude=10.0)

        ac1._evlogger.consume_events()
        ac2._evlogger.consume_events()

        lp.sec("ac1: enable location sending in chat")
        chat1.enable_sending_locations(seconds=100)
        assert chat1.is_sending_locations()
        ac1._evlogger.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

        ac1.set_location(latitude=2.0, longitude=3.0, accuracy=0.5)
        ac1._evlogger.get_matching("DC_EVENT_LOCATION_CHANGED")
        chat1.send_text("hello")
        ac1._evlogger.get_matching("DC_EVENT_SMTP_MESSAGE_SENT")

        lp.sec("ac2: wait for incoming location message")
        ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG")  # "enabled-location streaming"

        # currently core emits location changed before event_incoming message
        ac2._evlogger.get_matching("DC_EVENT_LOCATION_CHANGED")
        ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG")  # text message with location

        locations = chat2.get_locations()
        assert len(locations) == 1
        assert locations[0].latitude == 2.0
        assert locations[0].longitude == 3.0
        assert locations[0].accuracy == 0.5
        assert locations[0].timestamp > now

        contact = ac2.create_contact(ac1.get_config("addr"))
        locations2 = chat2.get_locations(contact=contact)
        assert len(locations2) == 1
        assert locations2 == locations

        contact = ac2.create_contact("nonexisting@example.org")
        locations3 = chat2.get_locations(contact=contact)
        assert not locations3


class TestGroupStressTests:
    def test_group_many_members_add_leave_remove(self, acfactory, lp):
        lp.sec("creating and configuring five accounts")
        accounts = [acfactory.get_online_configuring_account() for i in range(5)]
        for acc in accounts:
            wait_configuration_progress(acc, 1000)
        ac1 = accounts.pop()

        lp.sec("ac1: setting up contacts with 4 other members")
        contacts = []
        for acc, name in zip(accounts, list("äöüsr")):
            contact = ac1.create_contact(acc.get_config("addr"), name=name)
            contacts.append(contact)

            # make sure we accept the "hi" message
            ac1.create_chat_by_contact(contact)

            # make sure the other side accepts our messages
            c1 = acc.create_contact(ac1.get_config("addr"), "ä member")
            chat1 = acc.create_chat_by_contact(c1)

            # send a message to get the contact key via autocrypt header
            chat1.send_text("hi")
            msg = ac1.wait_next_incoming_message()
            assert msg.text == "hi"

        # Save fifth account for later
        ac5 = accounts.pop()
        contact5 = contacts.pop()

        lp.sec("ac1: creating group chat with 3 other members")
        chat = ac1.create_group_chat("title1")
        for contact in contacts:
            chat.add_contact(contact)
        assert not chat.is_promoted()

        lp.sec("ac1: send mesage to new group chat")
        msg = chat.send_text("hello")
        assert chat.is_promoted()
        assert msg.is_encrypted()

        gossiped_timestamp = chat.get_summary()["gossiped_timestamp"]
        assert gossiped_timestamp > 0

        num_contacts = len(chat.get_contacts())
        assert num_contacts == 3 + 1

        lp.sec("ac2: checking that the chat arrived correctly")
        ac2 = accounts[0]
        msg = ac2.wait_next_incoming_message()
        assert msg.text == "hello"
        print("chat is", msg.chat)
        assert len(msg.chat.get_contacts()) == 4

        lp.sec("ac3: checking that 'ac4' is a known contact")
        ac3 = accounts[1]
        msg3 = ac3.wait_next_incoming_message()
        assert msg3.text == "hello"
        ac3_contacts = ac3.get_contacts()
        assert len(ac3_contacts) == 3
        ac4_contacts = ac3.get_contacts(query=accounts[2].get_config("addr"))
        assert len(ac4_contacts) == 1

        lp.sec("ac2: removing one contact")
        to_remove = contacts[-1]
        msg.chat.remove_contact(to_remove)

        lp.sec("ac1: receiving system message about contact removal")
        sysmsg = ac1.wait_next_incoming_message()
        assert to_remove.addr in sysmsg.text
        assert len(sysmsg.chat.get_contacts()) == 3

        # Receiving message about removed contact does not reset gossip
        assert chat.get_summary()["gossiped_timestamp"] == gossiped_timestamp

        lp.sec("ac1: sending another message to the chat")
        chat.send_text("hello2")
        msg = ac2.wait_next_incoming_message()
        assert msg.text == "hello2"
        assert chat.get_summary()["gossiped_timestamp"] == gossiped_timestamp

        lp.sec("ac1: adding fifth member to the chat")
        chat.add_contact(contact5)
        # Additng contact to chat resets gossiped_timestamp
        assert chat.get_summary()["gossiped_timestamp"] >= gossiped_timestamp

        lp.sec("ac2: receiving system message about contact addition")
        sysmsg = ac2.wait_next_incoming_message()
        assert contact5.addr in sysmsg.text
        assert len(sysmsg.chat.get_contacts()) == 4

        lp.sec("ac5: waiting for message about addition to the chat")
        sysmsg = ac5.wait_next_incoming_message()
        msg = sysmsg.chat.send_text("hello!")
        # Message should be encrypted because keys of other members are gossiped
        assert msg.is_encrypted()


class TestOnlineConfigureFails:
    def test_invalid_password(self, acfactory):
        ac1, configdict = acfactory.get_online_config()
        ac1.configure(addr=configdict["addr"], mail_pw="123")
        ac1.start_threads()
        wait_configuration_progress(ac1, 500)
        ev1 = ac1._evlogger.get_matching("DC_EVENT_ERROR_NETWORK")
        assert "cannot login" in ev1[2].lower()
        wait_configuration_progress(ac1, 0, 0)

    def test_invalid_user(self, acfactory):
        ac1, configdict = acfactory.get_online_config()
        ac1.configure(addr="x" + configdict["addr"], mail_pw=configdict["mail_pw"])
        ac1.start_threads()
        wait_configuration_progress(ac1, 500)
        ev1 = ac1._evlogger.get_matching("DC_EVENT_ERROR_NETWORK")
        assert "cannot login" in ev1[2].lower()
        wait_configuration_progress(ac1, 0, 0)

    def test_invalid_domain(self, acfactory):
        ac1, configdict = acfactory.get_online_config()
        ac1.configure(addr=configdict["addr"] + "x", mail_pw=configdict["mail_pw"])
        ac1.start_threads()
        wait_configuration_progress(ac1, 500)
        ev1 = ac1._evlogger.get_matching("DC_EVENT_ERROR_NETWORK")
        assert "could not connect" in ev1[2].lower()
        wait_configuration_progress(ac1, 0, 0)

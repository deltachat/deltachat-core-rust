from __future__ import print_function
import pytest
import os
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

    def test_getinfo(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        d = ac1.get_info()
        assert d["arch"]
        assert d["number_of_chats"] == "0"

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
        chat.send_text("one messae")
        assert not ac1.delete_contact(contact1)


class TestOfflineChat:
    @pytest.fixture
    def ac1(self, acfactory):
        return acfactory.get_configured_offline_account()

    @pytest.fixture
    def chat1(self, ac1):
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        assert chat.id >= const.DC_CHAT_ID_LAST_SPECIAL, chat.id
        return chat

    def test_display(self, chat1):
        str(chat1)
        repr(chat1)

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
        msg = chat1.send_image(fn)
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

        path = ac1.export_to_dir(backupdir.strpath)
        assert os.path.exists(path)
        ac2 = acfactory.get_unconfigured_account()
        ac2.import_from_file(path)
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


class TestOnlineAccount:
    def get_chat(self, ac1, ac2):
        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)
        assert chat.id >= const.DC_CHAT_ID_LAST_SPECIAL
        return chat

    def test_one_account_send(self, acfactory):
        ac1 = acfactory.get_online_configuring_account()
        c2 = ac1.create_contact(email=ac1.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)
        assert chat.id >= const.DC_CHAT_ID_LAST_SPECIAL
        wait_successful_IMAP_SMTP_connection(ac1)
        wait_configuration_progress(ac1, 1000)

        msg_out = chat.send_text("message2")
        # wait for own account to receive
        ev = ac1._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev[1] == msg_out.id

    def test_two_accounts_send_receive(self, acfactory):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = self.get_chat(ac1, ac2)

        msg_out = chat.send_text("message1")

        # wait for other account to receive
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.text == "message1"

    def test_forward_messages(self, acfactory):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = self.get_chat(ac1, ac2)

        msg_out = chat.send_text("message2")

        # wait for other account to receive
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.text == "message2"

        # check the message arrived in contact-requests/deaddrop
        chat2 = msg_in.chat
        assert msg_in in chat2.get_messages()
        assert chat2.is_deaddrop()
        assert chat2 == ac2.get_deaddrop_chat()
        chat3 = ac2.create_group_chat("newgroup")
        assert not chat3.is_promoted()
        ac2.forward_messages([msg_in], chat3)
        assert chat3.is_promoted()
        messages = chat3.get_messages()
        ac2.delete_messages(messages)
        assert not chat3.get_messages()

    def test_send_and_receive_message(self, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

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

        lp.sec("mark message as seen on ac2, wait for changes on ac1")
        ac2.mark_seen_messages([msg_in])
        lp.step("1")
        ev = ac1._evlogger.get_matching("DC_EVENT_MSG_READ")
        assert ev[1] >= const.DC_CHAT_ID_LAST_SPECIAL
        assert ev[2] >= const.DC_MSG_ID_LAST_SPECIAL
        lp.step("2")
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

    def test_import_export_online(self, acfactory, tmpdir):
        ac1 = acfactory.get_online_configuring_account()
        wait_configuration_progress(ac1, 1000)

        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        chat.send_text("msg1")
        backupdir = tmpdir.mkdir("backup")
        path = ac1.export_to_dir(backupdir.strpath)
        assert os.path.exists(path)

        ac2 = acfactory.get_unconfigured_account()
        ac2.import_from_file(path)
        contacts = ac2.get_contacts(query="some1")
        assert len(contacts) == 1
        contact2 = contacts[0]
        assert contact2.addr == "some1@hello.com"
        chat2 = ac2.create_chat_by_contact(contact2)
        messages = chat2.get_messages()
        assert len(messages) == 1
        assert messages[0].text == "msg1"

    def test_ac_setup_message(self, acfactory):
        # note that the receiving account needs to be configured and running
        # before ther setup message is send. DC does not read old messages
        # as of Jul2019
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.clone_online_account(ac1)
        wait_configuration_progress(ac2, 1000)
        wait_configuration_progress(ac1, 1000)
        assert ac1.get_info()["fingerprint"] != ac2.get_info()["fingerprint"]
        setup_code = ac1.initiate_key_transfer()
        ac2._evlogger.set_timeout(30)
        ev = ac2._evlogger.get_matching("DC_EVENT_INCOMING_MSG|DC_EVENT_MSGS_CHANGED")
        msg = ac2.get_message_by_id(ev[2])
        assert msg.is_setup_message()
        # first try a bad setup code
        with pytest.raises(ValueError):
            msg.continue_key_transfer(str(reversed(setup_code)))
        print("*************** Incoming ASM File at: ", msg.filename)
        print("*************** Setup Code: ", setup_code)
        msg.continue_key_transfer(setup_code)
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
        wait_securejoin_inviter_progress(ac1, 1000)

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

    def test_set_get_profile_image(self, acfactory, data, lp):
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

from __future__ import print_function
import pytest
import os
from deltachat import const
from datetime import datetime, timedelta
from conftest import wait_configuration_progress, wait_successful_IMAP_SMTP_connection


class TestOfflineAccount:
    def test_getinfo(self, acfactory):
        ac1 = acfactory.get_unconfigured_account()
        d = ac1.get_info()
        assert d["compile_date"]
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

    def test_contact_attr(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact(email="some1@hello.com", name="some1")
        assert contact1.id
        assert contact1.addr == "some1@hello.com"
        assert contact1.display_name == "some1"
        assert not contact1.is_blocked()
        assert not contact1.is_verified()

    def test_get_contacts(self, acfactory):
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

    def test_chat(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        assert chat.id >= const.DC_CHAT_ID_LAST_SPECIAL, chat.id

        chat2 = ac1.create_chat_by_contact(contact1.id)
        assert chat2.id == chat.id
        assert chat2.get_name() == chat.get_name()
        assert chat == chat2
        assert not (chat != chat2)

        for ichat in ac1.get_chats():
            if ichat.id == chat.id:
                break
        else:
            pytest.fail("could not find chat")

    def test_group_chat_creation(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
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

    def test_delete_and_send_fails(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        chat.delete()
        ac1._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        with pytest.raises(ValueError):
            chat.send_text("msg1")

    def test_create_message(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        message = ac1.create_message("text")
        assert message.id == 0
        assert message._dc_msg is message._dc_msg
        message.set_text("hello")
        assert message.text == "hello"
        assert message.id == 0

    def test_message(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        msg = chat.send_text("msg1")
        assert msg
        assert msg.view_type.is_text()
        assert msg.view_type.name == "text"
        assert not msg.view_type.is_audio()
        assert not msg.view_type.is_video()
        assert not msg.view_type.is_gif()
        assert not msg.view_type.is_file()
        assert not msg.view_type.is_image()
        msg_state = msg.get_state()
        assert not msg_state.is_in_fresh()
        assert not msg_state.is_in_noticed()
        assert not msg_state.is_in_seen()
        assert msg_state.is_out_pending()
        assert not msg_state.is_out_failed()
        assert not msg_state.is_out_delivered()
        assert not msg_state.is_out_mdn_received()

    def test_message_image(self, acfactory, data, lp):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        with pytest.raises(ValueError):
            chat.send_image(path="notexists")
        fn = data.get_path("d.png")
        lp.sec("sending image")
        msg = chat.send_image(fn)
        assert msg.view_type.name == "image"
        assert msg
        assert msg.id > 0
        assert os.path.exists(msg.filename)
        assert msg.filemime == "image/png"

    @pytest.mark.parametrize("typein,typeout", [
            (None, "application/octet-stream"),
            ("text/plain", "text/plain"),
            ("image/png", "image/png"),
    ])
    def test_message_file(self, acfactory, data, lp, typein, typeout):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        lp.sec("sending file")
        fn = data.get_path("r.txt")
        msg = chat.send_file(fn, typein)
        assert msg
        assert msg.id > 0
        assert msg.view_type.name == "file"
        assert msg.view_type.is_file()
        assert os.path.exists(msg.filename)
        assert msg.filename.endswith(msg.basename)
        assert msg.filemime == typeout

    def test_chat_message_distinctions(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        contact1 = ac1.create_contact("some1@hello.com", name="some1")
        chat = ac1.create_chat_by_contact(contact1)
        past1s = datetime.utcnow() - timedelta(seconds=1)
        msg = chat.send_text("msg1")
        ts = msg.time_sent
        assert msg.time_received is None
        assert ts.strftime("Y")
        assert past1s < ts
        contact = msg.get_sender_contact()
        assert contact == ac1.get_self_contact()

    def test_basic_configure_ok_addr_setting_forbidden(self, acfactory):
        ac1 = acfactory.get_configured_offline_account()
        assert ac1.get_config("mail_pw")
        assert ac1.is_configured()
        with pytest.raises(ValueError):
            ac1.set_config("addr", "123@example.org")
        with pytest.raises(ValueError):
            ac1.configure(addr="123@example.org")


class TestOnlineAccount:
    def test_forward_messages(self, acfactory):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()
        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)
        assert chat.id >= const.DC_CHAT_ID_LAST_SPECIAL
        wait_successful_IMAP_SMTP_connection(ac1)
        wait_configuration_progress(ac1, 1000)
        wait_successful_IMAP_SMTP_connection(ac2)
        wait_configuration_progress(ac2, 1000)

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
        lp.sec("starting accounts, waiting for configuration")
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()
        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)
        assert chat.id >= const.DC_CHAT_ID_LAST_SPECIAL

        wait_configuration_progress(ac1, 1000)
        wait_configuration_progress(ac2, 1000)

        lp.sec("sending text message from ac1 to ac2")
        msg_out = chat.send_text("message1")
        ev = ac1._evlogger.get_matching("DC_EVENT_MSG_DELIVERED")
        evt_name, data1, data2 = ev
        assert data1 == chat.id
        assert data2 == msg_out.id
        assert msg_out.get_state().is_out_delivered()

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
        ac1._evlogger.get_matching("DC_EVENT_MSG_READ")
        lp.step("2")
        # ac1._evlogger.get_info_matching("Message marked as seen")
        assert msg_out.get_state().is_out_mdn_received()

    def test_saved_mime_on_received_message(self, acfactory, lp):
        lp.sec("starting accounts, waiting for configuration")
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()
        ac2.set_config("save_mime_headers", "1")
        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)
        wait_configuration_progress(ac1, 1000)
        wait_configuration_progress(ac2, 1000)
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
        lp.sec("starting accounts, waiting for configuration")
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()
        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)

        wait_configuration_progress(ac1, 1000)
        wait_configuration_progress(ac2, 1000)

        lp.sec("sending image message from ac1 to ac2")
        path = data.get_path("d.png")
        msg_out = chat.send_image(path)
        ev = ac1._evlogger.get_matching("DC_EVENT_MSG_DELIVERED")
        evt_name, data1, data2 = ev
        assert data1 == chat.id
        assert data2 == msg_out.id
        assert msg_out.get_state().is_out_delivered()

        lp.sec("wait for ac2 to receive message")
        ev = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev[2] == msg_out.id
        msg_in = ac2.get_message_by_id(msg_out.id)
        assert msg_in.view_type.is_image()
        assert os.path.exists(msg_in.filename)
        assert os.stat(msg_in.filename).st_size == os.stat(path).st_size

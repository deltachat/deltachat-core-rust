from __future__ import print_function
from filecmp import cmp
from deltachat import const
from conftest import wait_configuration_progress, wait_msgs_changed


class TestOnlineInCreation:
    def test_forward_increation(self, acfactory, data, lp):
        ac1 = acfactory.get_online_configuring_account()
        ac2 = acfactory.get_online_configuring_account()
        wait_configuration_progress(ac1, 1000)
        wait_configuration_progress(ac2, 1000)

        c2 = ac1.create_contact(email=ac2.get_config("addr"))
        chat = ac1.create_chat_by_contact(c2)
        assert chat.id >= const.DC_CHAT_ID_LAST_SPECIAL
        wait_msgs_changed(ac1, 0, 0)  # why no chat id?

        lp.sec("create a message with a file in creation")
        path = data.get_path("d.png")
        prepared_original = chat.prepare_message_file(path)
        assert prepared_original.is_out_preparing()
        wait_msgs_changed(ac1, chat.id, prepared_original.id)

        lp.sec("forward the message while still in creation")
        chat2 = ac1.create_group_chat("newgroup")
        chat2.add_contact(c2)
        wait_msgs_changed(ac1, 0, 0)  # why not chat id?
        ac1.forward_messages([prepared_original], chat2)
        # XXX there might be two EVENT_MSGS_CHANGED and only one of them
        # is the one caused by forwarding
        forwarded_id = wait_msgs_changed(ac1, chat2.id)
        if forwarded_id == 0:
            forwarded_id = wait_msgs_changed(ac1, chat2.id)
            assert forwarded_id
        forwarded_msg = ac1.get_message_by_id(forwarded_id)
        assert forwarded_msg.is_out_preparing()

        lp.sec("finish creating the file and send it")
        assert prepared_original.is_out_preparing()
        chat.send_prepared(prepared_original)
        assert prepared_original.is_out_pending() or prepared_original.is_out_delivered()
        wait_msgs_changed(ac1, chat.id, prepared_original.id)

        lp.sec("expect the forwarded message to be sent now too")
        wait_msgs_changed(ac1, chat2.id, forwarded_id)
        fwd_msg = ac1.get_message_by_id(forwarded_id)
        assert fwd_msg.is_out_pending() or fwd_msg.is_out_delivered()

        lp.sec("wait for the messages to be delivered to SMTP")
        ev = ac1._evlogger.get_matching("DC_EVENT_MSG_DELIVERED")
        assert ev[1] == chat.id
        assert ev[2] == prepared_original.id
        ev = ac1._evlogger.get_matching("DC_EVENT_MSG_DELIVERED")
        assert ev[1] == chat2.id
        assert ev[2] == forwarded_id

        lp.sec("wait1 for original or forwarded messages to arrive")
        ev1 = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev1[1] >= const.DC_CHAT_ID_LAST_SPECIAL
        received_original = ac2.get_message_by_id(ev1[2])
        assert cmp(received_original.filename, path, False)

        lp.sec("wait2 for original or forwarded messages to arrive")
        ev2 = ac2._evlogger.get_matching("DC_EVENT_MSGS_CHANGED")
        assert ev2[1] >= const.DC_CHAT_ID_LAST_SPECIAL
        assert ev2[1] != ev1[1]
        received_copy = ac2.get_message_by_id(ev2[2])
        assert cmp(received_copy.filename, path, False)

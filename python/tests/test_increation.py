from __future__ import print_function

import os.path
import shutil

import pytest
from filecmp import cmp


def wait_msg_delivered(account, msg_list):
    """ wait for one or more MSG_DELIVERED events to match msg_list contents. """
    msg_list = list(msg_list)
    while msg_list:
        ev = account._evtracker.get_matching("DC_EVENT_MSG_DELIVERED")
        msg_list.remove((ev.data1, ev.data2))


def wait_msgs_changed(account, msgs_list):
    """ wait for one or more MSGS_CHANGED events to match msgs_list contents. """
    account.log("waiting for msgs_list={}".format(msgs_list))
    msgs_list = list(msgs_list)
    while msgs_list:
        ev = account._evtracker.get_matching("DC_EVENT_MSGS_CHANGED")
        for i, (data1, data2) in enumerate(msgs_list):
            if ev.data1 == data1:
                if data2 is None or ev.data2 == data2:
                    del msgs_list[i]
                    break
        else:
            account.log("waiting mismatch data1={} data2={}".format(data1, data2))
    return ev.data1, ev.data2


class TestOnlineInCreation:
    def test_increation_not_blobdir(self, tmpdir, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = ac1.create_chat(ac2)

        lp.sec("Creating in-creation file outside of blobdir")
        assert tmpdir.strpath != ac1.get_blobdir()
        src = tmpdir.join('file.txt').ensure(file=1)
        with pytest.raises(Exception):
            chat.prepare_message_file(src.strpath)

    def test_no_increation_copies_to_blobdir(self, tmpdir, acfactory, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()
        chat = ac1.create_chat(ac2)

        lp.sec("Creating file outside of blobdir")
        assert tmpdir.strpath != ac1.get_blobdir()
        src = tmpdir.join('file.txt')
        src.write("hello there\n")
        chat.send_file(src.strpath)

        blob_src = os.path.join(ac1.get_blobdir(), 'file.txt')
        assert os.path.exists(blob_src), "file.txt not copied to blobdir"

    def test_forward_increation(self, acfactory, data, lp):
        ac1, ac2 = acfactory.get_two_online_accounts()

        chat = ac1.create_chat(ac2)
        wait_msgs_changed(ac1, [(0, 0)])  # why no chat id?

        lp.sec("create a message with a file in creation")
        orig = data.get_path("d.png")
        path = os.path.join(ac1.get_blobdir(), 'd.png')
        with open(path, "x") as fp:
            fp.write("preparing")
        prepared_original = chat.prepare_message_file(path)
        assert prepared_original.is_out_preparing()
        wait_msgs_changed(ac1, [(chat.id, prepared_original.id)])

        lp.sec("forward the message while still in creation")
        chat2 = ac1.create_group_chat("newgroup")
        chat2.add_contact(ac2)
        wait_msgs_changed(ac1, [(0, 0)])  # why not chat id?
        ac1.forward_messages([prepared_original], chat2)
        # XXX there might be two EVENT_MSGS_CHANGED and only one of them
        # is the one caused by forwarding
        _, forwarded_id = wait_msgs_changed(ac1, [(chat2.id, None)])
        forwarded_msg = ac1.get_message_by_id(forwarded_id)
        assert forwarded_msg.is_out_preparing()

        lp.sec("finish creating the file and send it")
        assert prepared_original.is_out_preparing()
        shutil.copyfile(orig, path)
        chat.send_prepared(prepared_original)
        assert prepared_original.is_out_pending() or prepared_original.is_out_delivered()

        lp.sec("check that both forwarded and original message are proper.")
        wait_msgs_changed(ac1, [(chat2.id, forwarded_id), (chat.id, prepared_original.id)])

        fwd_msg = ac1.get_message_by_id(forwarded_id)
        assert fwd_msg.is_out_pending() or fwd_msg.is_out_delivered()

        lp.sec("wait for both messages to be delivered to SMTP")
        wait_msg_delivered(ac1, [
            (chat2.id, forwarded_id),
            (chat.id, prepared_original.id)
        ])

        lp.sec("wait1 for original or forwarded messages to arrive")
        received_original = ac2._evtracker.wait_next_incoming_message()
        assert cmp(received_original.filename, orig, shallow=False)

        lp.sec("wait2 for original or forwarded messages to arrive")
        received_copy = ac2._evtracker.wait_next_incoming_message()
        assert received_copy.id != received_original.id
        assert cmp(received_copy.filename, orig, shallow=False)

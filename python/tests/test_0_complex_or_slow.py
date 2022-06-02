import sys

import pytest


class TestGroupStressTests:
    def test_group_many_members_add_leave_remove(self, acfactory, lp):
        accounts = acfactory.get_online_accounts(5)
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
        assert ac5.get_config("configured_addr") in sysmsg.text
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
        accounts = acfactory.get_online_accounts(3)
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


def test_qr_verified_group_and_chatting(acfactory, lp):
    ac1, ac2, ac3 = acfactory.get_online_accounts(3)
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


@pytest.mark.parametrize("mvbox_move", [False, True])
def test_fetch_existing(acfactory, lp, mvbox_move):
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

    ac1 = acfactory.new_online_configuring_account(mvbox_move=mvbox_move)
    ac2 = acfactory.new_online_configuring_account()
    acfactory.wait_configured(ac1)
    ac1.direct_imap.create_folder("Sent")
    ac1.set_config("sentbox_watch", "1")

    # We need to reconfigure to find the new "Sent" folder.
    # `scan_folders()`, which runs automatically shortly after `start_io()` is invoked,
    # would also find the "Sent" folder, but it would be too late:
    # The sentbox thread, started by `start_io()`, would have seen that there is no
    # ConfiguredSentboxFolder and do nothing.
    acfactory._acsetup.start_configure(ac1, reconfigure=True)
    acfactory.bring_accounts_online()
    assert_folders_configured(ac1)

    assert ac1.direct_imap.select_config_folder("mvbox" if mvbox_move else "inbox")
    with ac1.direct_imap.idle() as idle1:
        lp.sec("send out message with bcc to ourselves")
        ac1.set_config("bcc_self", "1")
        chat = acfactory.get_accepted_chat(ac1, ac2)
        chat.send_text("message text")
        assert_folders_configured(ac1)

        lp.sec("wait until the bcc_self message arrives in correct folder and is marked seen")
        assert idle1.wait_for_seen()
    assert_folders_configured(ac1)

    lp.sec("create a cloned ac1 and fetch contact history during configure")
    ac1_clone = acfactory.new_online_configuring_account(cloned_from=ac1)
    ac1_clone.set_config("fetch_existing_msgs", "1")
    acfactory.wait_configured(ac1_clone)
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


def test_fetch_existing_msgs_group_and_single(acfactory, lp):
    """There was a bug concerning fetch-existing-msgs:

    A sent a message to you, adding you to a group. This created a contact request.
    You wrote a message to A, creating a chat.
    ...but the group stayed blocked.
    So, after fetch-existing-msgs you have one contact request and one chat with the same person.

    See https://github.com/deltachat/deltachat-core-rust/issues/2097"""
    ac1 = acfactory.new_online_configuring_account()
    ac2 = acfactory.new_online_configuring_account()

    acfactory.bring_accounts_online()

    lp.sec("receive a message")
    ac2.create_group_chat("group name", contacts=[ac1]).send_text("incoming, unencrypted group message")
    ac1._evtracker.wait_next_incoming_message()

    lp.sec("send out message with bcc to ourselves")
    with ac1.direct_imap.idle() as idle1:
        ac1.set_config("bcc_self", "1")
        ac1_ac2_chat = ac1.create_chat(ac2)
        ac1_ac2_chat.send_text("outgoing, encrypted direct message, creating a chat")
        # wait until the bcc_self message arrives
        assert idle1.wait_for_seen()

    lp.sec("Clone online account and let it fetch the existing messages")
    ac1_clone = acfactory.new_online_configuring_account(cloned_from=ac1)
    ac1_clone.set_config("fetch_existing_msgs", "1")
    acfactory.wait_configured(ac1_clone)

    ac1_clone.start_io()
    ac1_clone._evtracker.wait_idle_inbox_ready()

    chats = ac1_clone.get_chats()
    assert len(chats) == 4  # two newly created chats + self-chat + device-chat
    group_chat = [c for c in chats if c.get_name() == "group name"][0]
    assert group_chat.is_group()
    (private_chat,) = [c for c in chats if c.get_name() == ac1_ac2_chat.get_name()]
    assert not private_chat.is_group()

    group_messages = group_chat.get_messages()
    assert len(group_messages) == 1
    assert group_messages[0].text == "incoming, unencrypted group message"
    private_messages = private_chat.get_messages()
    # We can't decrypt the message in this chat, so the chat is empty:
    assert len(private_messages) == 0


def test_undecipherable_group(acfactory, lp):
    """Test how group messages that cannot be decrypted are
    handled.

    Group name is encrypted and plaintext subject is set to "..." in
    this case, so we should assign the messages to existing chat
    instead of creating a new one. Since there is no existing group
    chat, the messages should be assigned to 1-1 chat with the sender
    of the message.
    """

    lp.sec("creating and configuring three accounts")
    ac1, ac2, ac3 = acfactory.get_online_accounts(3)

    acfactory.introduce_each_other([ac1, ac2, ac3])

    lp.sec("ac3 reinstalls DC and generates a new key")
    ac3.stop_io()
    acfactory.remove_preconfigured_keys()
    ac4 = acfactory.new_online_configuring_account(cloned_from=ac3)
    acfactory.wait_configured(ac4)
    # Create contacts to make sure incoming messages are not treated as contact requests
    chat41 = ac4.create_chat(ac1)
    chat42 = ac4.create_chat(ac2)
    ac4.start_io()
    ac4._evtracker.wait_idle_inbox_ready()

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


def test_ephemeral_timer(acfactory, lp):
    ac1, ac2 = acfactory.get_online_accounts(2)

    lp.sec("ac1: create chat with ac2")
    chat1 = ac1.create_chat(ac2)
    chat2 = ac2.create_chat(ac1)

    lp.sec("ac1: set ephemeral timer to 60")
    chat1.set_ephemeral_timer(60)

    lp.sec("ac1: check that ephemeral timer is set for chat")
    assert chat1.get_ephemeral_timer() == 60
    chat1_summary = chat1.get_summary()
    assert chat1_summary["ephemeral_timer"] == {"Enabled": {"duration": 60}}

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


def test_multidevice_sync_seen(acfactory, lp):
    """Test that message marked as seen on one device is marked as seen on another."""
    ac1 = acfactory.new_online_configuring_account()
    ac2 = acfactory.new_online_configuring_account()
    ac1_clone = acfactory.new_online_configuring_account(cloned_from=ac1)
    acfactory.bring_accounts_online()

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

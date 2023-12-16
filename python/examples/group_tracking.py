# content of group_tracking.py

from deltachat import account_hookimpl, run_cmdline


class GroupTrackingPlugin:
    @account_hookimpl
    def ac_incoming_message(self, message):
        print("process_incoming message", message)
        if message.text.strip() == "/quit":
            message.account.shutdown()
        else:
            # unconditionally accept the chat
            message.create_chat()
            addr = message.get_sender_contact().addr
            text = message.text
            message.chat.send_text(f"echoing from {addr}:\n{text}")

    @account_hookimpl
    def ac_outgoing_message(self, message):
        print("ac_outgoing_message:", message)

    @account_hookimpl
    def ac_configure_completed(self, success):
        print("ac_configure_completed:", success)

    @account_hookimpl
    def ac_chat_modified(self, chat):
        print("ac_chat_modified:", chat.id, chat.get_name())
        for member in chat.get_contacts():
            print(f"chat member: {member.addr}")

    @account_hookimpl
    def ac_member_added(self, chat, contact, actor, message):
        print(f"ac_member_added {contact.addr} to chat {chat.id} from {actor or message.get_sender_contact().addr}")
        for member in chat.get_contacts():
            print(f"chat member: {member.addr}")

    @account_hookimpl
    def ac_member_removed(self, chat, contact, actor, message):
        print(f"ac_member_removed {contact.addr} from chat {chat.id} by {actor or message.get_sender_contact().addr}")


def main(argv=None):
    run_cmdline(argv=argv, account_plugins=[GroupTrackingPlugin()])


if __name__ == "__main__":
    main()

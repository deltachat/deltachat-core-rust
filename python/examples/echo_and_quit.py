
# content of echo_and_quit.py

from deltachat import account_hookimpl, run_cmdline


class EchoPlugin:
    @account_hookimpl
    def ac_incoming_message(self, message):
        print("process_incoming message", message)
        if message.text.strip() == "/quit":
            message.account.shutdown()
        else:
            # unconditionally accept the chat
            message.create_chat()
            addr = message.get_sender_contact().addr
            if message.is_system_message():
                message.chat.send_text("echoing system message from {}:\n{}".format(addr, message))
            else:
                text = message.text
                message.chat.send_text("echoing from {}:\n{}".format(addr, text))

    @account_hookimpl
    def ac_message_delivered(self, message):
        print("ac_message_delivered", message)


def main(argv=None):
    run_cmdline(argv=argv, account_plugins=[EchoPlugin()])


if __name__ == "__main__":
    main()

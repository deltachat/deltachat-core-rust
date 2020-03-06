
# content of echo_and_quit.py

import deltachat


class EchoPlugin:
    @deltachat.hookspec.account_hookimpl
    def process_incoming_message(self, message):
        print("process_incoming message", message)
        if message.text.strip() == "/quit":
            message.account.shutdown()
        else:
            ch = message.get_sender_chat()
            addr = message.get_sender_contact().addr
            text = message.text
            ch.send_text("echoing from {}:\n{}".format(addr, text))

    @deltachat.hookspec.account_hookimpl
    def process_message_delivered(self, message):
        print("process_message_delivered", message)


def main(argv=None):
    deltachat.run_cmdline(argv=argv, account_plugins=[EchoPlugin()])


if __name__ == "__main__":
    main()

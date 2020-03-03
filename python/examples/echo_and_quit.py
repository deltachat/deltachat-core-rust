
# content of echo_and_quit.py

import sys
import optparse
import deltachat


class SimpleEchoPlugin:
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


def main(argv):
    p = optparse.OptionParser("simple-echo")
    p.add_option("-l", action="store_true", help="show ffi")
    p.add_option("--db", type="str", help="database file")
    p.add_option("--email", type="str", help="email address")
    p.add_option("--password", type="str", help="password")

    opts, posargs = p.parse_args(argv)

    assert opts.db, "you must specify --db"
    ac = deltachat.Account(opts.db)

    if opts.l:
        log = deltachat.eventlogger.FFIEventLogger(ac, "echo")
        ac.add_account_plugin(log)

    if not ac.is_configured():
        assert opts.email and opts.password, (
            "you must specify --email and --password"
        )
        ac.set_config("addr", opts.email)
        ac.set_config("mail_pw", opts.password)
        ac.set_config("mvbox_watch", "0")
        ac.set_config("sentbox_watch", "0")

    ac.add_account_plugin(SimpleEchoPlugin())

    # start IO threads and configure if neccessary
    ac.start()

    print("{}: waiting for message".format(ac.get_config("addr")))

    ac.wait_shutdown()


if __name__ == "__main__":
    main(sys.argv)

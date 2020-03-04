
# content of group_tracking.py

import sys
import optparse
import deltachat


class GroupTrackingPlugin:
    @deltachat.hookspec.account_hookimpl
    def process_incoming_message(self, message):
        print("*** process_incoming_message addr={} msg={!r}".format(
              message.get_sender_contact().addr, message.text))
        for member in message.chat.get_contacts():
            print("chat member: {}".format(member.addr))

    @deltachat.hookspec.account_hookimpl
    def member_added(self, chat, contact):
        print("*** member_added", contact.addr, "from", chat)
        for member in chat.get_contacts():
            print("chat member: {}".format(member.addr))

    @deltachat.hookspec.account_hookimpl
    def member_removed(self, chat, contact):
        print("*** member_removed", contact.addr, "from", chat)


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
        log = deltachat.eventlogger.FFIEventLogger(ac, "group-tracking")
        ac.add_account_plugin(log)

    if not ac.is_configured():
        assert opts.email and opts.password, (
            "you must specify --email and --password"
        )
        ac.set_config("addr", opts.email)
        ac.set_config("mail_pw", opts.password)
        ac.set_config("mvbox_watch", "0")
        ac.set_config("sentbox_watch", "0")

    ac.add_account_plugin(GroupTrackingPlugin())

    # start IO threads and configure if neccessary
    ac.start()

    print("{}: waiting for message".format(ac.get_config("addr")))

    ac.wait_shutdown()


if __name__ == "__main__":
    main(sys.argv)

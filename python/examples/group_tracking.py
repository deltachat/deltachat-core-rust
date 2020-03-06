
# content of group_tracking.py

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


def main(argv=None):
    deltachat.run_cmdline(argv=argv, account_plugins=[GroupTrackingPlugin()])


if __name__ == "__main__":
    main()

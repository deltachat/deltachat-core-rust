
# instantiate and configure deltachat account
import deltachat
ac = deltachat.Account("/tmp/db")

# to see low-level events in the console uncomment the following line
# ac.add_account_plugin(deltachat.eventlogger.FFIEventLogger(ac, ""))

if not ac.is_configured():
    ac.set_config("addr", "tmpy.94mtm@testrun.org")
    ac.set_config("mail_pw", "5CbD6VnjD/li")
    ac.set_config("mvbox_watch", "0")
    ac.set_config("sentbox_watch", "0")

class MyPlugin:
    @deltachat.hookspec.account_hookimpl
    def process_incoming_message(self, message):
        print("process_incoming message", message)
        if message.text.strip() == "/quit":
            print("shutting down")
            ac.shutdown()
        else:
            ch = ac.create_chat_by_contact(message.get_sender_contact())
            ch.send_text("echoing {}".format(message.text))

    @deltachat.hookspec.account_hookimpl
    def process_message_delivered(self, message):
        print("process_message_delivered", message)

ac.add_account_plugin(MyPlugin())

# start IO threads and perform configuration
ac.start()

print("waiting for /quit or other message on {}".format(ac.get_config("addr")))

ac.wait_shutdown()

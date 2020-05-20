
import deltachat
import os
import shutil
from deltachat.capi import lib

try:
    os.remove("/tmp/db")
except:
    pass
try:
    shutil.rmtree("/tmp/db-blobs")
except:
    pass


acc = deltachat.Account("/tmp/db", logging=True)
acc.set_config("addr", "tmp.hjfcq@five.chat")
acc.set_config("mail_pw", "aihWNtLuRJgV")
acc.start()  # lib.dc_configure + lib.dc_context_run
assert acc.is_configured()
acc.stop_scheduler()

run = 0
while 1:
    print("****** starting scheduler")
    acc.start()
    import time ; time.sleep(0.5)
    print("******* stopping scheduler")
    acc.stop_scheduler()
    print("******* waiting", run)
    import time ; time.sleep(1.0)
    run += 1

contact = acc.create_contact("holger@deltachat.de")
chat = acc.create_chat_by_contact(contact)
chat.send_text("hello")

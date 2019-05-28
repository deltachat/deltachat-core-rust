import sys
import re
import os
from os.path import dirname, abspath
from os.path import join as joinpath

# the following const are generated from deltachat.h
# this works well when you in a git-checkout
# run "python deltachat/const.py" to regenerate events
# begin const generated
DC_GCL_ARCHIVED_ONLY = 0x01
DC_GCL_NO_SPECIALS = 0x02
DC_GCL_ADD_ALLDONE_HINT = 0x04
DC_GCL_VERIFIED_ONLY = 0x01
DC_GCL_ADD_SELF = 0x02
DC_CHAT_ID_DEADDROP = 1
DC_CHAT_ID_TRASH = 3
DC_CHAT_ID_MSGS_IN_CREATION = 4
DC_CHAT_ID_STARRED = 5
DC_CHAT_ID_ARCHIVED_LINK = 6
DC_CHAT_ID_ALLDONE_HINT = 7
DC_CHAT_ID_LAST_SPECIAL = 9
DC_CHAT_TYPE_UNDEFINED = 0
DC_CHAT_TYPE_SINGLE = 100
DC_CHAT_TYPE_GROUP = 120
DC_CHAT_TYPE_VERIFIED_GROUP = 130
DC_MSG_ID_MARKER1 = 1
DC_MSG_ID_DAYMARKER = 9
DC_MSG_ID_LAST_SPECIAL = 9
DC_STATE_UNDEFINED = 0
DC_STATE_IN_FRESH = 10
DC_STATE_IN_NOTICED = 13
DC_STATE_IN_SEEN = 16
DC_STATE_OUT_PREPARING = 18
DC_STATE_OUT_DRAFT = 19
DC_STATE_OUT_PENDING = 20
DC_STATE_OUT_FAILED = 24
DC_STATE_OUT_DELIVERED = 26
DC_STATE_OUT_MDN_RCVD = 28
DC_CONTACT_ID_SELF = 1
DC_CONTACT_ID_DEVICE = 2
DC_CONTACT_ID_LAST_SPECIAL = 9
DC_MSG_TEXT = 10
DC_MSG_IMAGE = 20
DC_MSG_GIF = 21
DC_MSG_AUDIO = 40
DC_MSG_VOICE = 41
DC_MSG_VIDEO = 50
DC_MSG_FILE = 60
DC_EVENT_INFO = 100
DC_EVENT_SMTP_CONNECTED = 101
DC_EVENT_IMAP_CONNECTED = 102
DC_EVENT_SMTP_MESSAGE_SENT = 103
DC_EVENT_WARNING = 300
DC_EVENT_ERROR = 400
DC_EVENT_ERROR_NETWORK = 401
DC_EVENT_ERROR_SELF_NOT_IN_GROUP = 410
DC_EVENT_MSGS_CHANGED = 2000
DC_EVENT_INCOMING_MSG = 2005
DC_EVENT_MSG_DELIVERED = 2010
DC_EVENT_MSG_FAILED = 2012
DC_EVENT_MSG_READ = 2015
DC_EVENT_CHAT_MODIFIED = 2020
DC_EVENT_CONTACTS_CHANGED = 2030
DC_EVENT_LOCATION_CHANGED = 2035
DC_EVENT_CONFIGURE_PROGRESS = 2041
DC_EVENT_IMEX_PROGRESS = 2051
DC_EVENT_IMEX_FILE_WRITTEN = 2052
DC_EVENT_SECUREJOIN_INVITER_PROGRESS = 2060
DC_EVENT_SECUREJOIN_JOINER_PROGRESS = 2061
DC_EVENT_GET_STRING = 2091
DC_EVENT_HTTP_GET = 2100
DC_EVENT_HTTP_POST = 2110
DC_EVENT_FILE_COPIED = 2055
DC_EVENT_IS_OFFLINE = 2081
# end const generated


def read_event_defines(f):
    rex = re.compile(r'#define\s+((?:DC_EVENT_|DC_MSG|DC_STATE_|DC_CONTACT_ID_|DC_GCL|DC_CHAT)\S+)\s+([x\d]+).*')
    for line in f:
        m = rex.match(line)
        if m:
            yield m.groups()


if __name__ == "__main__":
    here = abspath(__file__).rstrip("oc")
    here_dir = dirname(here)
    if len(sys.argv) >= 2:
        deltah = sys.argv[1]
    else:
        deltah = joinpath(dirname(dirname(dirname(here_dir))), "src", "deltachat.h")
    assert os.path.exists(deltah)

    lines = []
    skip_to_end = False
    for orig_line in open(here):
        if skip_to_end:
            if not orig_line.startswith("# end const"):
                continue
            skip_to_end = False
        lines.append(orig_line)
        if orig_line.startswith("# begin const"):
            with open(deltah) as f:
                for name, item in read_event_defines(f):
                    lines.append("{} = {}\n".format(name, item))
            skip_to_end = True

    tmpname = here + ".tmp"
    with open(tmpname, "w") as f:
        f.write("".join(lines))
    os.rename(tmpname, here)

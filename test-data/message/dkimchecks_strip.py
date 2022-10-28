# Use with dkimchecks_strip.sh

import sys

inheader = False
for l in sys.stdin:
    if inheader and (l.startswith(" ") or l.startswith("\t")):
        print(l, end='')
        continue
    else:
        inheader = False
    if l.startswith("Authentication-Results:") or l.startswith("From:") or l.startswith("To:") or l.startswith("ARC-Authentication-Results"):
        print(l, end='')
        inheader=True

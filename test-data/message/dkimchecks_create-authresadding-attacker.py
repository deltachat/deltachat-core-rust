# This is a small script which helped me write the atuhresadding-attacker@example.com emails
# I still did quite some things manually.
# cd dkimchecks-2022-09-28; for d in *; do cd $d ; python3 ../../create-forged-authres-added.py >forged-authres-added@example.com; cd $HOME/deltachat-android/jni/deltachat-core-rust/test-data/message/dkimchecks-2022-09-28; done

with open("nami.lefherz@delta.blinzeln.de", "r") as f:
    inheader = False
    for l in f:
        if inheader and l.startswith(" "):
            print(l, end='')
            continue
        else:
            inheader=False
        if l.startswith("Authentication-Results: secure-mailgate.com"):
            print(f"Authentication-Results: aaa.com; dkim=pass header.i=@example.com")
        elif l.startswith("Authentication-Results:") and not l.startswith("Authentication-Results: secure-mailgate.com"):
            print(l, end='')
            inheader=True
        if l.startswith("From:"):
            print("From: forged-authres-added@example.com");
        if l.startswith("Authentication-Results-Original"):
            print("TO BE DELETED")
    print(f"Authentication-Results: aaa.com; dkim=pass header.i=@example.com")

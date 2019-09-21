#!/usr/bin/env python3


from pathlib import Path
import os
import re

if __name__ == "__main__":
    if Path('src/top_evil_rs.py').exists():
        os.chdir('src')
    filestats = []
    for fn in Path(".").glob("**/*.rs"):
        s = fn.read_text()
        s = re.sub(r"(?m)///.*$", "", s)  # remove comments
        unsafe = s.count("unsafe")
        free = s.count("free(")
        gotoblocks = s.count("ok_to_continue") + s.count('OK_TO_CONTINUE')
        chars = s.count("c_char") + s.count("CStr")
        filestats.append((fn, unsafe, free, gotoblocks, chars))

    sum_unsafe, sum_free, sum_gotoblocks, sum_chars = 0, 0, 0, 0

    for fn, unsafe, free, gotoblocks, chars in reversed(sorted(filestats, key=lambda x: sum(x[1:]))):
        print("{0: <25} unsafe: {1: >3} free: {2: >3} ok_to_cont: {3: >3} chars: {4: >3}".format(str(fn), unsafe, free, gotoblocks, chars))
        sum_unsafe += unsafe
        sum_free += free
        sum_gotoblocks += gotoblocks
        sum_chars += chars


    print()
    print("total unsafe:", sum_unsafe)
    print("total free:", sum_free)
    print("total ok_to_continue:", sum_gotoblocks)
    print("total c_chars:", sum_chars)

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
        unsafe_fn = s.count("unsafe fn")
        chars = s.count("c_char") + s.count("CStr")
        filestats.append((fn, unsafe, free, unsafe_fn, chars))

    sum_unsafe, sum_free, sum_unsafe_fn, sum_chars = 0, 0, 0, 0

    for fn, unsafe, free, unsafe_fn, chars in reversed(sorted(filestats, key=lambda x: sum(x[1:]))):
        if unsafe + free + unsafe_fn + chars == 0:
            continue
        print("{0: <25} unsafe: {1: >3} free: {2: >3} unsafe-fn: {3: >3} chars: {4: >3}".format(str(fn), unsafe, free, unsafe_fn, chars))
        sum_unsafe += unsafe
        sum_free += free
        sum_unsafe_fn += unsafe_fn
        sum_chars += chars


    print()
    print("total unsafe:", sum_unsafe)
    print("total free:", sum_free)
    print("total unsafe-fn:", sum_unsafe_fn)
    print("total c_chars:", sum_chars)

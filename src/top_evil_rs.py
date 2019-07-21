#!/usr/bin/env python3


import os

if __name__ == "__main__":
    filestats = []
    for fn in os.listdir():
        if fn.endswith(".rs"):
            s = open(fn).read()
            unsafe = s.count("unsafe")
            free = s.count("free(")
            gotoblocks = s.count("current_block =")
            filestats.append((fn, unsafe, free, gotoblocks))

    sum_unsafe, sum_free, sum_gotoblocks = 0, 0, 0

    for fn, unsafe, free, gotoblocks in reversed(sorted(filestats, key=lambda x: sum(x[1:]))):
        print("{0: <30} unsafe: {1: >3} free: {2: >3} goto-blocks: {3: >3}".format(fn, unsafe, free, gotoblocks))
        sum_unsafe += unsafe
        sum_free += free
        sum_gotoblocks += gotoblocks


    print()
    print("total unsafe:", sum_unsafe)
    print("total free:", sum_free)
    print("total gotoblocks:", sum_gotoblocks)


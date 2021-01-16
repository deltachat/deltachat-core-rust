#!/usr/bin/env python3

"""
 setup a python binding development in-place install with cargo debug symbols.
"""

import os
import subprocess
import sys

if __name__ == "__main__":
    target = os.environ.get("DCC_RS_TARGET")
    if target is None:
        os.environ["DCC_RS_TARGET"] = target = "debug"
    if "DCC_RS_DEV" not in os.environ:
        dn = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        os.environ["DCC_RS_DEV"] = dn

    cmd = ["cargo", "build", "-p", "deltachat_ffi"]

    if target == 'release':
        os.environ["CARGO_PROFILE_RELEASE_LTO"] = "on"
        cmd.append("--release")

    print("running:", " ".join(cmd))
    subprocess.check_call(cmd)
    subprocess.check_call("rm -rf build/ src/deltachat/*.so" , shell=True)

    if len(sys.argv) <= 1 or sys.argv[1] != "onlybuild":
        subprocess.check_call([
            sys.executable, "-m", "pip", "install", "-e", "."
        ])

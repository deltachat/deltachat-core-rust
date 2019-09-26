#!/usr/bin/env python

"""
 setup a python binding development in-place install with cargo debug symbols.
"""

import os
import subprocess
import sys

if __name__ == "__main__":
    os.environ["DCC_RS_TARGET"] = target = "release"
    if "DCC_RS_DEV" not in os.environ:
        dn = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        os.environ["DCC_RS_DEV"] = dn

    os.environ["RUSTFLAGS"] = "-g"
    subprocess.check_call([
        "cargo", "build", "-p", "deltachat_ffi", "--" + target
    ])
    subprocess.check_call("rm -rf build/ src/deltachat/*.so" , shell=True)

    subprocess.check_call([
        sys.executable, "-m", "pip", "install", "-e", "."
    ])

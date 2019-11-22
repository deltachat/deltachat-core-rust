#!/usr/bin/env python

"""
 setup a python binding development in-place install with cargo debug symbols.
"""

import os
import subprocess
import sys

if __name__ == "__main__":
    target = os.environ.get("DCC_RS_TARGET")
    if target is None:
        os.environ["DCC_RS_TARGET"] = target = "release"
    if "DCC_RS_DEV" not in os.environ:
        dn = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        os.environ["DCC_RS_DEV"] = dn

    # build the core library  in release + debug mode because
    # as of Nov 2019 rPGP generates RSA keys which take
    # prohibitively long for non-release installs
    os.environ["RUSTFLAGS"] = "-g"
    subprocess.check_call([
        "cargo", "build", "-p", "deltachat_ffi", "--" + target
    ])
    subprocess.check_call("rm -rf build/ src/deltachat/*.so" , shell=True)

    if len(sys.argv) <= 1 or sys.argv[1] != "onlybuild":
        subprocess.check_call([
            sys.executable, "-m", "pip", "install", "-e", "."
        ])

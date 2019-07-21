#!/usr/bin/env python

"""
 setup a python binding development in-place install with cargo debug symbols.
"""

import os
import subprocess

if __name__ == "__main__":
    os.environ["DCC_RS_TARGET"] = target = "release"

    toml = os.path.join(os.getcwd(), "..", "Cargo.toml")
    assert os.path.exists(toml)
    with open(toml) as f:
        s = orig = f.read()
    s += "\n"
    s += "[profile.release]\n"
    s += "debug = true\n"
    with open(toml, "w") as f:
        f.write(s)
    print("temporarily modifying Cargo.toml to provide release build with debug symbols ")
    try:
        subprocess.check_call([
            "cargo", "build", "-p", "deltachat_ffi", "--" + target
        ])
    finally:
        with open(toml, "w") as f:
            f.write(orig)
        print("\nreseted Cargo.toml to previous original state")

    subprocess.check_call("rm -rf build/ src/deltachat/*.so" , shell=True)

    subprocess.check_call([
        "pip", "install", "-e", "."
    ])

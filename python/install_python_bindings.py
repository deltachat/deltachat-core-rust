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
        os.environ["DCC_RS_TARGET"] = target = "debug"
    if "DCC_RS_DEV" not in os.environ:
        dn = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        os.environ["DCC_RS_DEV"] = dn

    cmd = ["cargo", "build", "-p", "deltachat_ffi"]

    if target == 'release':
        extra = " -C lto=on -C embed-bitcode=yes"
        os.environ["RUSTFLAGS"] = os.environ.get("RUSTFLAGS", "") + extra
        cmd.append("--release")

    print("running:", " ".join(cmd))

    if "dontblock" in sys.argv:
        # Unfortunately, the CI sometimes does nothing, simply stating "Blocking waiting for file lock on build directory".
        process = subprocess.Popen(cmd, stderr=subprocess.PIPE)
        for line in process.stderr:
            l = line.decode("utf-8")
            print(l, end='')
            if "waiting for file lock on build directory" in l:
                print("Stopping build, cleaning up and retrying")
                process.terminate()
                try:
                    subprocess.check_call("cargo clean", shell=True)
                except subprocess.CalledProcessError as e:
                    print(e)
                subprocess.check_call(cmd)
        process.wait()

    else:
        subprocess.check_call(cmd)

    subprocess.check_call("rm -rf build/ src/deltachat/*.so" , shell=True)

    if len(sys.argv) <= 1 or sys.argv[1] != "onlybuild":
        subprocess.check_call([
            sys.executable, "-m", "pip", "install", "-e", "."
        ])

#!/usr/bin/env python

import os
import sys
import re
import pathlib
import subprocess

rex = re.compile(r'version = "(\S+)"')

def read_toml_version(relpath):
    p = pathlib.Path(relpath)
    assert p.exists()
    for line in open(str(p)):
        m = rex.match(line)
        if m is not None:
            return m.group(1)
    raise ValueError("no version found in {}".format(relpath))

def replace_toml_version(relpath, newversion):
    p = pathlib.Path(relpath)
    assert p.exists()
    tmp_path = str(p) + "_tmp"
    with open(tmp_path, "w") as f:
        for line in open(str(p)):
            m = rex.match(line)
            if m is not None:
                f.write('version = "{}"\n'.format(newversion))
            else:
                f.write(line)
    os.rename(tmp_path, str(p))

if __name__ == "__main__":

    if len(sys.argv) < 2:
        for x in ("Cargo.toml", "deltachat-ffi/Cargo.toml"):
            print("{}: {}".format(x, read_toml_version(x)))
        raise SystemExit("need argument: new version, example 1.0.0-beta.27")
    newversion = sys.argv[1]
    if newversion.count(".") < 2:
        raise SystemExit("need at least two dots in version")

    core_toml = read_toml_version("Cargo.toml")
    ffi_toml = read_toml_version("deltachat-ffi/Cargo.toml")
    assert core_toml == ffi_toml, (core_toml, ffi_toml)

    for line in open("CHANGELOG.md"):
        ## 1.0.0-beta5
        if line.startswith("## "):
            if line[2:].strip().startswith(newversion):
                break
    else:
        raise SystemExit("CHANGELOG.md contains no entry for version: {}".format(newversion))

    replace_toml_version("Cargo.toml", newversion)
    replace_toml_version("deltachat-ffi/Cargo.toml", newversion)

    subprocess.call(["cargo", "check"])
    subprocess.call(["git", "add", "-u"])
    # subprocess.call(["cargo", "update", "-p", "deltachat"])

    print("after commit make sure to: ")
    print("")
    print("   git tag {}".format(newversion))
    print("")

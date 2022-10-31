#!/usr/bin/env python3

import os
import json
import re
import pathlib
import subprocess
from argparse import ArgumentParser

rex = re.compile(r'version = "(\S+)"')


def regex_matches(relpath, regex=rex):
    p = pathlib.Path(relpath)
    assert p.exists()
    for line in open(str(p)):
        m = regex.match(line)
        if m is not None:
            return m


def read_toml_version(relpath):
    res = regex_matches(relpath, rex)
    if res is not None:
        return res.group(1)
    raise ValueError("no version found in {}".format(relpath))


def replace_toml_version(relpath, newversion):
    p = pathlib.Path(relpath)
    assert p.exists()
    tmp_path = str(p) + "_tmp"
    with open(tmp_path, "w") as f:
        for line in open(str(p)):
            m = rex.match(line)
            if m is not None:
                print("{}: set version={}".format(relpath, newversion))
                f.write('version = "{}"\n'.format(newversion))
            else:
                f.write(line)
    os.rename(tmp_path, str(p))


def read_json_version(relpath):
    p = pathlib.Path(relpath)
    assert p.exists()
    with open(p, "r") as f:
        json_data = json.loads(f.read())
    return json_data["version"]


def update_package_json(relpath, newversion):
    p = pathlib.Path(relpath)
    assert p.exists()
    with open(p, "r") as f:
        json_data = json.loads(f.read())
    json_data["version"] = newversion
    with open(p, "w") as f:
        f.write(json.dumps(json_data, sort_keys=True, indent=2))


def main():
    parser = ArgumentParser(prog="set_core_version")
    parser.add_argument("newversion")

    json_list = ["package.json",  "deltachat-jsonrpc/typescript/package.json"]
    toml_list = [
        "Cargo.toml",
        "deltachat-ffi/Cargo.toml",
        "deltachat-jsonrpc/Cargo.toml",
        "deltachat-rpc-server/Cargo.toml",
    ]
    try:
        opts = parser.parse_args()
    except SystemExit:
        print()
        for x in toml_list:
            print("{}: {}".format(x, read_toml_version(x)))
        for x in json_list:
            print("{}: {}".format(x, read_json_version(x)))
        print()
        raise SystemExit("need argument: new version, example: 1.25.0")

    newversion = opts.newversion
    if newversion.count(".") < 2:
        raise SystemExit("need at least two dots in version")

    core_toml = read_toml_version("Cargo.toml")
    ffi_toml = read_toml_version("deltachat-ffi/Cargo.toml")
    assert core_toml == ffi_toml, (core_toml, ffi_toml)

    if "alpha" not in newversion:
        for line in open("CHANGELOG.md"):
            ## 1.25.0
            if line.startswith("## "):
                if line[2:].strip().startswith(newversion):
                    break
        else:
            raise SystemExit("CHANGELOG.md contains no entry for version: {}".format(newversion))

    for toml_filename in toml_list:
        replace_toml_version(toml_filename, newversion)
    
    for json_filename in json_list:
        update_package_json(json_filename, newversion)


    print("running cargo check")
    subprocess.call(["cargo", "check"])

    print("adding changes to git index")
    subprocess.call(["git", "add", "-u"])
    # subprocess.call(["cargo", "update", "-p", "deltachat"])

    print("after commit, on master make sure to: ")
    print("")
    print("   git tag -a {}".format(newversion))
    print("   git push origin {}".format(newversion))
    print("   git tag -a py-{}".format(newversion))
    print("   git push origin py-{}".format(newversion))
    print("")


if __name__ == "__main__":
    main()


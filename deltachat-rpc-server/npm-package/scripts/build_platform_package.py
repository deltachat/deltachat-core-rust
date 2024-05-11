#!/usr/bin/env python3
import subprocess
from sys import argv
from os import path, makedirs, chdir
from shutil import copy
from src.make_package import write_package_json

# ensure correct working directory
chdir(path.join(path.dirname(path.abspath(__file__)), "../"))

if len(argv) < 2:
    print("First argument should be target architecture as required by cargo")
    exit(1)

target = argv[1].strip()

output = subprocess.run(["rustup", "target", "list"], capture_output=True)
available_targets = output.stdout.decode("utf-8")

if available_targets.find(target) == -1:
    print("target", target, "is not known")
    exit(1)

if available_targets.find(target + " (installed)") == -1:
    print(
        "target ", target, " is not installed, run 'rustup target add " + target + "'"
    )
    exit(1)

subprocess.run(
    ["cargo", "build", "--release", "-p", "deltachat-rpc-server", "--target", target]
)

newpath = "platform_package"
if not path.exists(newpath):
    makedirs(newpath)

# make new folder

platform_path = "platform_package/" + target
if not path.exists(platform_path):
    makedirs(platform_path)

# copy binary it over


def binary_path(binary_name):
    return "../../target/" + target + "/release/" + binary_name


my_binary_name = "deltachat-rpc-server"

if not path.isfile(binary_path("deltachat-rpc-server")):
    my_binary_name = "deltachat-rpc-server.exe"
    if not path.isfile(binary_path("deltachat-rpc-server.exe")):
        print("Did not find the build")
        exit(1)

my_binary_path = binary_path(my_binary_name)

copy(my_binary_path, platform_path + "/" + my_binary_name)

# make a package.json for it

write_package_json(platform_path, target, my_binary_name)

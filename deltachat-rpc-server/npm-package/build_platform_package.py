import subprocess
from sys import argv
from os import path, makedirs
import json
import tomllib
from shutil import copy

if len(argv) < 2:
    print("First argument should be target architecture as required by cargo")
    exit(1)

target = argv[1].strip()

output = subprocess.run(["rustup","target","list"], capture_output=True)
available_targets = output.stdout.decode("utf-8")

if available_targets.find(target) == -1:
    print("target", target, "is not known")
    exit(1)

if available_targets.find(target + " (installed)") == -1:
    print("target ", target, " is not installed, run 'rustup target add "+target+"'")
    exit(1)

subprocess.run([
    "cargo",
    "build",
    "--release",
    "-p",
    "deltachat-rpc-server",
    "--target",
    target
])

newpath = r'platform_package' 
if not path.exists(newpath):
    makedirs(newpath)

# make new folder

platform_path = 'platform_package/' + target
if not path.exists(platform_path):
    makedirs(platform_path)

# copy binary it over

def binary_path(binary_name):
    return "../../target/"+target+"/release/"+binary_name

my_binary_name = "deltachat-rpc-server"

if not path.isfile(binary_path("deltachat-rpc-server")):
    my_binary_name = "deltachat-rpc-server.exe"
    if not path.isfile(binary_path("deltachat-rpc-server.exe")):
        print("Did not find the build")
        exit(1)

my_binary_path = binary_path(my_binary_name)

copy(my_binary_path, platform_path + "/" + my_binary_name)

# read version
tomlfile = open("../Cargo.toml", 'rb')
version = tomllib.load(tomlfile)['package']['version']

# make a package.json for it
[cpu_arch, vendor, os] = target.split("-")

def convert_cpu_arch_to_npm_cpu_arch(arch):
    if arch == "x86_64":
        return "x64"
    if arch == "i686":
        return "i32"
    if arch == "aarch64":
        return "arm64"
    print("architecture might not be known by nodejs, please make sure it can be returned by 'process.arch':", arch)
    return arch

def convert_os_to_npm_os(os):
    if os == "windows":
        return "win32"
    if os == "darwin" or os == "linux":
        return os
    print("architecture might not be known by nodejs, please make sure it can be returned by 'process.platform':", os)
    return os

package_json = dict({
    "name": "@deltachat/stdio-rpc-server-" + convert_os_to_npm_os(os) + "-" + convert_cpu_arch_to_npm_cpu_arch(cpu_arch),
    "version": version,
    "os": [convert_os_to_npm_os(os)],
    "cpu": [convert_cpu_arch_to_npm_cpu_arch(cpu_arch)],
    "main": my_binary_name,
    "license": "MPL-2.0"
})

file = open(platform_path + "/package.json", 'w')
file.write(json.dumps(package_json, indent=4))
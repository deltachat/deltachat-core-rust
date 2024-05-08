import subprocess
from sys import argv
from os import path, makedirs, chdir
import json
from shutil import copy
from src.make_package import write_package_json

# ensure correct working directory
chdir(path.join(path.dirname(path.abspath(__file__)), "../"))

if len(argv) < 3:
    print("First argument should be target architecture as required by cargo")
    print("Second argument should be the location of th built binary (binary_path)")
    exit(1)

target = argv[1].strip()
binary_path = argv[2].strip()

output = subprocess.run(["rustup","target","list"], capture_output=True)
available_targets = output.stdout.decode("utf-8")

if available_targets.find(target) == -1:
    print("target", target, "is not known / not valid")
    exit(1)


newpath = r'platform_package' 
if not path.exists(newpath):
    makedirs(newpath)

# make new folder

platform_path = 'platform_package/' + target
if not path.exists(platform_path):
    makedirs(platform_path)

# copy binary it over

my_binary_name = path.basename(binary_path)
copy(binary_path, platform_path + "/" + my_binary_name)

# make a package.json for it

write_package_json(platform_path, target, my_binary_name)
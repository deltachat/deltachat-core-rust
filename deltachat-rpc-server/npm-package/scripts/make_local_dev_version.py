# This script is for making a version of the npm packet that you can install locally

import subprocess
from sys import argv
from os import path, makedirs, chdir
import re
import json
import tomllib
from shutil import copy, rmtree

# ensure correct working directory
chdir(path.join(path.dirname(path.abspath(__file__)), "../"))

# get host target with "rustc -vV"
output = subprocess.run(["rustc", "-vV"], capture_output=True)
host_target = re.search('host: ([-\\w]*)', output.stdout.decode("utf-8")).group(1)
print("host target to build for is:", host_target)

# clean platform_package folder
newpath = r'platform_package' 
if not path.exists(newpath):
    makedirs(newpath)
else:
    rmtree(path.join(path.dirname(path.abspath(__file__)), "../platform_package/"))
    makedirs(newpath)

# run build_platform_package.py with the host's target to build it
subprocess.run(["python", "scripts/build_platform_package.py", host_target], capture_output=False, check=True)

# run update_optional_dependencies_and_version.js to adjust the package / make it installable locally
subprocess.run(["node", "scripts/update_optional_dependencies_and_version.js", "--local"], capture_output=False, check=True)

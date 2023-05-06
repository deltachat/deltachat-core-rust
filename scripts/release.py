#!/usr/bin/env python3

import sys
import subprocess
import set_core_version
from argparse import ArgumentParser


# update the version
parser = ArgumentParser(prog="release")
parser.add_argument("newversion")

try:
    newversion = parser.parse_args().newversion
except SystemExit:
    newversion = None
set_core_version.set_version(newversion)

tag = "v" + newversion

# TODO would be nice to automatically checkout the correct branch

# update the changelog
print(f"Updating CHANGELOG.md using git cliff --unreleased --tag {newversion} --prepend CHANGELOG.md")
changelog = subprocess.run(["git", "cliff", "--unreleased", "--tag", newversion, "--prepend", "CHANGELOG.md"]).stdout.strip()
subprocess.run(["git", "add", "-A"])
subprocess.run(["git", "commit", "-m", f"chore(release): prepare for {tag}"])
subprocess.run(["git", "show"])

# create a tag
subprocess.run(
    [
        "git", "tag", "-a", tag, "-m", f"Release {tag}", "-m", changelog
    ]
)
subprocess.run(["git", "tag", "-v", tag])

print("Done!")
print(f"Now push the commit (git push origin {tag}).")

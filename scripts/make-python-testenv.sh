#!/usr/bin/env bash
#
# Script to create or update a python development environment.
# It rebuilds the core and bindings as needed.
#
# After running the script, you can either
# run `pytest` directly with `venv/bin/pytest python/`
# or activate the environment with `. venv/bin/activate`
# and run `pytest` from there.
set -euo pipefail

export DCC_RS_TARGET=debug
export DCC_RS_DEV="$PWD"
cargo build -p deltachat_ffi --features jsonrpc

tox -c python -e py --devenv venv
env/bin/pip install --upgrade pip

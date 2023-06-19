#!/usr/bin/env bash
#
# Script to create or update a python development environment.
# It rebuilds the core and bindings as needed.
#
# After running the script, you can either
# run `pytest` directly with `env/bin/pytest python/`
# or activate the environment with `. env/bin/activacte`
# and run `pytest` from there.
set -euo pipefail

export DCC_RS_TARGET=debug
export DCC_RS_DEV="$PWD"
cargo build -p deltachat_ffi --features jsonrpc

if test -d env; then
	env/bin/pip install -e python --force-reinstall
else
	tox -e py --devenv env
	env/bin/pip install --upgrade pip
fi

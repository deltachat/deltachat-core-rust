#!/usr/bin/env bash
set -euo pipefail

export DCC_RS_TARGET=debug
export DCC_RS_DEV="$PWD"
cargo build -p deltachat_ffi --features jsonrpc

python3 -m venv venv
venv/bin/pip install ./python
venv/bin/pip install ./deltachat-rpc-client
venv/bin/pip install sphinx breathe sphinx_rtd_theme
venv/bin/pip install ./deltachat-rpc-client
venv/bin/sphinx-build -b html -a python/doc/ dist/html

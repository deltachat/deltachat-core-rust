#!/usr/bin/env bash
set -euo pipefail
cargo install --locked --path deltachat-rpc-server/ --root "$PWD/venv" --debug
PATH="$PWD/venv/bin:$PATH" tox -c deltachat-rpc-client

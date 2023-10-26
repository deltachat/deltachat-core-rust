#!/usr/bin/env bash
set -euo pipefail
cargo install --path deltachat-rpc-server/ --root "$PWD/venv"
PATH="$PWD/venv/bin:$PATH" tox -c deltachat-rpc-client

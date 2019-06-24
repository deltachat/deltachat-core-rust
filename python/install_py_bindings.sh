#!/usr/bin/env bash

set -ex

cargo build -p deltachat_ffi --release
DCC_RS_DEV=`pwd`/.. pip install -e .

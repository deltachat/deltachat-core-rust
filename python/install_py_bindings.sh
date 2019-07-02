#!/usr/bin/env bash

set -ex

cargo build -p deltachat_ffi --release
rm -rf build/ 
DCC_RS_DEV=`pwd`/.. pip install -e .

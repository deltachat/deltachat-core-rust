#!/usr/bin/env bash

set -ex

cargo build -p deltachat_ffi 
rm -rf build/ src/deltachat/*.so
DCC_RS_DEV=`pwd`/.. pip install -e .

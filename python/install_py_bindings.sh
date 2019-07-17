#!/usr/bin/env bash

set -ex

export DCC_RS_TARGET=release

cargo build -p deltachat_ffi --${DCC_RS_TARGET}
rm -rf build/ src/deltachat/*.so
DCC_RS_DEV=`pwd`/.. pip install -e .

#!/usr/bin/env bash

set -ex

cd ..
cargo build -p deltachat_ffi --release
cd python

export CFLAGS=-I../deltachat-ffi
# the followine line results in "libdeltachat.so" not found
# export LDFLAGS='-Wl,-rpath=$ORIGIN/../target/release -Wl,--enable-new-dtags'
pip install -e .
export LD_LIBRARY_PATH=../target/release
python -c "import deltachat"

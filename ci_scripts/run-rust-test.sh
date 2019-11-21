#!/usr/bin/env bash

set -ex

#export RUST_TEST_THREADS=1
export RUST_BACKTRACE=1
export RUSTFLAGS='--deny warnings'
export OPT="--target=$TARGET"
export OPT_RELEASE="--release ${OPT}"
export OPT_FFI_RELEASE="--manifest-path=deltachat-ffi/Cargo.toml --release"

# Select cargo command: use cross by default
export CARGO_CMD=cross

# On Appveyor (windows) and Travis (x86_64-unknown-linux-gnu and apple) native targets we use cargo (no need to cross-compile):
if [[ $TARGET = *"windows"* ]] || [[ $TARGET == "x86_64-unknown-linux-gnu" ]] || [[ $TARGET = *"apple"* ]]; then
    export CARGO_CMD=cargo
fi

# Install cross if necessary:
if [[ $CARGO_CMD == "cross" ]]; then
    cargo install --git https://github.com/dignifiedquire/cross --rev fix-tty --force
fi

# Make sure TARGET is installed when using cargo:
if [[ $CARGO_CMD == "cargo" ]]; then
    rustup target add $TARGET || true
fi

# If the build should not run tests, just check that the code builds:
if [[ $NORUN == "1" ]]; then
    export CARGO_SUBCMD="build"
else
    export CARGO_SUBCMD="test --all"
    export OPT="${OPT} "
    export OPT_RELEASE="${OPT_RELEASE} "
    export OPT_RELEASE_IGNORED="${OPT_RELEASE} -- --ignored"
fi

# Run all the test configurations 
# RUSTC_WRAPPER=SCCACHE seems to destroy parallelism / prolong the test
unset RUSTC_WRAPPER
$CARGO_CMD $CARGO_SUBCMD $OPT
$CARGO_CMD $CARGO_SUBCMD $OPT_RELEASE
$CARGO_CMD $CARGO_SUBCMD $OPT_RELEASE_IGNORED

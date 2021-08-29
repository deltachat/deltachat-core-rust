#!/bin/sh
set -eu

if ! command -v grcov >/dev/null; then
    echo >&2 '`grcov` not found. Check README at https://github.com/mozilla/grcov for setup instructions.'
    echo >&2 'Run `cargo install grcov` to build `grcov` from source.'
    exit 1
fi

# Allow `-Z` flags without using nightly Rust.
export RUSTC_BOOTSTRAP=1

# We are using `-Zprofile` instead of source-based coverage [1]
# (`-Zinstrument-coverage`) due to a bug resulting in empty reports [2].
#
# [1] https://blog.rust-lang.org/inside-rust/2020/11/12/source-based-code-coverage.html
# [2] https://github.com/mozilla/grcov/issues/595

export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
cargo clean
cargo build
cargo test

grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./coverage/

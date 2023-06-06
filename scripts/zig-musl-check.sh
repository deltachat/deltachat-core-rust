#!/bin/sh
#
# Run `cargo check` with musl libc.
# This requires `zig` to compile vendored openssl.

set -x
set -e

unset RUSTFLAGS

# Pin Rust version to avoid uncontrolled changes in the compiler and linker flags.
export RUSTUP_TOOLCHAIN=1.70.0

ZIG_VERSION=0.11.0-dev.2213+515e1c93e

# Download Zig
rm -fr "$ZIG_VERSION" "zig-linux-x86_64-$ZIG_VERSION.tar.xz"
wget "https://ziglang.org/builds/zig-linux-x86_64-$ZIG_VERSION.tar.xz"
tar xf "zig-linux-x86_64-$ZIG_VERSION.tar.xz"
export PATH="$PWD/zig-linux-x86_64-$ZIG_VERSION:$PATH"

rustup target add x86_64-unknown-linux-musl
CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="x86_64-linux-musl" \
cargo check --release --target x86_64-unknown-linux-musl -p deltachat_ffi --features jsonrpc

#!/bin/sh
#
# Build statically linked deltachat-rpc-server using zig.

set -x
set -e

unset RUSTFLAGS

# Pin Rust version to avoid uncontrolled changes in the compiler and linker flags.
export RUSTUP_TOOLCHAIN=1.72.0

rustup target add i686-unknown-linux-musl
CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_I686_UNKNOWN_LINUX_MUSL_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="x86-linux-musl" \
cargo build --release --target i686-unknown-linux-musl -p deltachat-rpc-server --features vendored

rustup target add armv7-unknown-linux-musleabihf
CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="arm-linux-musleabihf" \
ZIG_CPU="generic+v7a+vfp3-d32+thumb2-neon" \
cargo build --release --target armv7-unknown-linux-musleabihf -p deltachat-rpc-server --features vendored

rustup target add x86_64-unknown-linux-musl
CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="x86_64-linux-musl" \
cargo build --release --target x86_64-unknown-linux-musl -p deltachat-rpc-server --features vendored

rustup target add aarch64-unknown-linux-musl
CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="aarch64-linux-musl" \
cargo build --release --target aarch64-unknown-linux-musl -p deltachat-rpc-server --features vendored

mkdir -p dist
cp target/x86_64-unknown-linux-musl/release/deltachat-rpc-server dist/deltachat-rpc-server-x86_64-linux
cp target/i686-unknown-linux-musl/release/deltachat-rpc-server dist/deltachat-rpc-server-i686-linux
cp target/aarch64-unknown-linux-musl/release/deltachat-rpc-server dist/deltachat-rpc-server-aarch64-linux
cp target/armv7-unknown-linux-musleabihf/release/deltachat-rpc-server dist/deltachat-rpc-server-armv7-linux

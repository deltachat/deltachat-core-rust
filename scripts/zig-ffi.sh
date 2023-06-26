#!/bin/sh
#
# Build FFI library using zig.

set -x
set -e

export RUSTFLAGS="-Clinker-plugin-lto"
export CFLAGS="-fno-unwind-tables -fno-exceptions -fno-asynchronous-unwind-tables -fomit-frame-pointer"

build() {
	cargo build --release --target $1 -p deltachat_ffi --features vendored
}

CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_I686_UNKNOWN_LINUX_MUSL_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="x86-linux-musl" \
build i686-unknown-linux-musl

CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="arm-linux-musleabihf" \
ZIG_CPU="generic+v7a+vfp3-d32+thumb2-neon" \
build armv7-unknown-linux-musleabihf

CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="x86_64-linux-musl" \
build x86_64-unknown-linux-musl

CC="$PWD/scripts/zig-cc" \
TARGET_CC="$PWD/scripts/zig-cc" \
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$PWD/scripts/zig-cc" \
LD="$PWD/scripts/zig-cc" \
ZIG_TARGET="aarch64-linux-musl" \
build aarch64-unknown-linux-musl

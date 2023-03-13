#!/bin/sh
#
# Build statically linked deltachat-rpc-server using cargo-zigbuild.

set -x
set -e

unset RUSTFLAGS

ZIG_VERSION=0.11.0-dev.1935+1d96a17af

# Download Zig
rm -fr "$ZIG_VERSION" "ZIG_VERSION.tar.xz"
wget "https://ziglang.org/builds/zig-linux-x86_64-$ZIG_VERSION.tar.xz"
tar xf "zig-linux-x86_64-$ZIG_VERSION.tar.xz"
export PATH="$PWD/zig-linux-x86_64-$ZIG_VERSION:$PATH"

cargo install cargo-zigbuild

for TARGET in aarch64-unknown-linux-musl armv7-unknown-linux-musleabihf; do
	rustup target add "$TARGET"
	cargo zigbuild --release --target "$TARGET" -p deltachat-rpc-server --features vendored
done

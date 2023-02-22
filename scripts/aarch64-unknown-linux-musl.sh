#!/bin/sh
#
# Build statically linked deltachat-rpc-server for aarch64-unknown-linux-musl.

set -x
set -e

# Download Zig
rm -fr zig-linux-x86_64-0.10.1 zig-linux-x86_64-0.10.1.tar.xz
wget https://ziglang.org/download/0.10.1/zig-linux-x86_64-0.10.1.tar.xz
tar xf zig-linux-x86_64-0.10.1.tar.xz
export PATH="$PATH:$PWD/zig-linux-x86_64-0.10.1"

cargo install cargo-zigbuild

rustup target add aarch64-unknown-linux-musl

cargo zigbuild --release --target aarch64-unknown-linux-musl -p deltachat-rpc-server --features vendored

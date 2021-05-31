#!/bin/bash

set -e -x

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain "1.50.0-$(uname -m)-unknown-linux-gnu" -y
export PATH=/root/.cargo/bin:$PATH
rustc --version

# remove some 300-400 MB that we don't need for automated builds
rm -rf "/root/.rustup/toolchains/1.50.0-$(uname -m)-unknown-linux-gnu/share"

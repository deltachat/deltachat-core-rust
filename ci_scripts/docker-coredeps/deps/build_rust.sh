#!/bin/bash

set -e -x

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.49.0-x86_64-unknown-linux-gnu -y
export PATH=/root/.cargo/bin:$PATH
rustc --version

# remove some 300-400 MB that we don't need for automated builds
rm -rf /root/.rustup/toolchains/1.49.0-x86_64-unknown-linux-gnu/share

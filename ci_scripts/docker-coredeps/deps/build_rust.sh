#!/bin/bash

set -e -x

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly-2019-09-12 -y
export PATH=/root/.cargo/bin:$PATH
rustc --version

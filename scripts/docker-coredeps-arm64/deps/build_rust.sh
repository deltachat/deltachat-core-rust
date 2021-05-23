#!/bin/bash

set -e -x

# Install Rust
#
# Path from https://forge.rust-lang.org/infra/other-installation-methods.html
#
# Avoid using rustup here as it depends on reading /proc/self/exe and
# has problems running under QEMU.
curl "https://static.rust-lang.org/dist/rust-1.50.0-$(uname -m)-unknown-linux-gnu.tar.gz" | tar xz
cd "rust-1.50.0-$(uname -m)-unknown-linux-gnu"
./install.sh --prefix=/usr --components=rustc,cargo,"rust-std-$(uname -m)-unknown-linux-gnu"
rustc --version
cd ..
rm -fr "rust-1.50.0-$(uname -m)-unknown-linux-gnu"

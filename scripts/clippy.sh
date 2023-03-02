#!/bin/sh
# Run clippy for all Rust code in the project.
cargo clippy --workspace --all-targets -- -D warnings

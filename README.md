# Delta Chat Rust

> Project porting deltachat-core to rust

[![CircleCI](https://circleci.com/gh/deltachat/deltachat-core-rust.svg?style=svg)](https://circleci.com/gh/deltachat/deltachat-core-rust)

Current commit on deltachat/deltachat-core: `12ef73c8e76185f9b78e844ea673025f56a959ab`.

## Development

```sh
# run example
$ cargo run --example simple
# build header file
$ cargo build -p deltachat_ffi --release
$ cat deltachat-ffi/deltachat.h
# run tests
$ cargo test --all
```

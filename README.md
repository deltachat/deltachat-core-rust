# Delta Chat Rust

> Project porting deltachat-core to rust


[![CircleCI build status][circle-shield]][circle] [![Appveyor build status][appveyor-shield]][appveyor]

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

[circle-shield]: https://img.shields.io/circleci/project/github/deltachat/deltachat-core-rust/master.svg?style=flat-square
[circle]: https://circleci.com/gh/deltachat/deltachat-core-rust/
[appveyor-shield]: https://ci.appveyor.com/api/projects/status/lqpegel3ld4ipxj8/branch/master?style=flat-square
[appveyor]: https://ci.appveyor.com/project/dignifiedquire/deltachat-core-rust/branch/master

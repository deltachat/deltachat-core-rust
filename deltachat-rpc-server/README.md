# Delta Chat RPC server

This program provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface to DeltaChat
over standard I/O.

## Install

To download binary pre-builds check the [releases page](https://github.com/deltachat/deltachat-core-rust/releases).
Rename the downloaded binary to `deltachat-rpc-server` and add it to your `PATH`.

To install from source run:

```sh
cargo install --git https://github.com/deltachat/deltachat-core-rust/ deltachat-rpc-server
```

The `deltachat-rpc-server` executable will be installed into `$HOME/.cargo/bin` that should be available
in your `PATH`.

## Usage

To use just run `deltachat-rpc-server` command. The accounts folder will be created in the current
working directory unless `DC_ACCOUNTS_PATH` is set:

```sh
export DC_ACCOUNTS_PATH=$HOME/delta/
deltachat-rpc-server
```

The common use case for this program is to create bindings to use Delta Chat core from programming
languages other than Rust, for example:

1. Python: https://pypi.org/project/deltachat-rpc-client/
2. Go: https://github.com/deltachat/deltachat-rpc-client-go/

Run `deltachat-rpc-server --version` to check the version of the server.
Run `deltachat-rpc-server --openrpc` to get [OpenRPC](https://open-rpc.org/) specification of the provided JSON-RPC API.

## Usage with `tokio-console`

When built with `RUSTFLAGS="--cfg tokio_unstable"`, console-subscriber is enabled.
That means that you can use [`tokio-console`](https://github.com/tokio-rs/console) to inspect active Tokio tasks.
You can install it via `cargo install tokio-console`.

```sh
RUSTFLAGS="--cfg tokio_unstable" cargo run
```

### Usage in deltachat-desktop:

Follow steps from `deltachat-desktop/docs/UPDATE_CORE.md`, but run the `make_local_dev_version` script with the `tokio_unstable` rustflag:

```sh
RUSTFLAGS="--cfg tokio_unstable" python3 deltachat-rpc-server/npm-package/scripts/make_local_dev_version.py
```

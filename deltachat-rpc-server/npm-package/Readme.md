## npm backage for deltachat-rpc-server

This is the successor of `deltachat-node`,
it does not use NAPI bindings but instead uses stdio executables
to let you talk to core over jsonrpc over stdio.
This simplifies cross-compilation and even reduces binary size (no CFFI layer and no NAPI layer).

## How to use on an unsupported platform

<!-- todo instructions, will uses an env var for pointing to `deltachat-rpx-server` binary -->

## How does it work when you install it

NPM automatically installs platform dependent optional dependencies when `os` and `cpu` fields are set correctly.

references:
- https://napi.rs/docs/deep-dive/release#3-the-native-addon-for-different-platforms-is-distributed-through-different-npm-packages, [webarchive version](https://web.archive.org/web/20240309234250/https://napi.rs/docs/deep-dive/release#3-the-native-addon-for-different-platforms-is-distributed-through-different-npm-packages)
- https://docs.npmjs.com/cli/v6/configuring-npm/package-json#cpu
- https://docs.npmjs.com/cli/v6/configuring-npm/package-json#os

When you import this package it searches for the rpc server in the following locations and order:
1. `DELTA_CHAT_RPC_SERVER` environment variable
2. prebuilds in npm packages
3. in PATH, but there an additional version check is performed

## How do you built this package in CI

- To build platform packages, run the `build_platform_package.py` script:
    ```
    python3 build_platform_package.py <cargo-target>
    # example
    python3 build_platform_package.py x86_64-apple-darwin
    ```
- Then pass it as an artifact to the last CI action that publishes the main package.
- upload all packages from `deltachat-rpc-server/npm-package/platform_package`.
- then publish `deltachat-rpc-server/npm-package`,
  - this will run `update_optional_dependencie_and_version.js` (in the `prepack` script),
    which puts all platform packages into `optionalDependencies` and updates the `version` in `package.json`

## Thanks to nlnet

The initial work on this package was funded by nlnet as part of the [Delta Tauri](https://nlnet.nl/project/DeltaTauri/) Project.
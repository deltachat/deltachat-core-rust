//@ts-check
import {ENV_VAR_NAME} from "./const"


const cargoInstallCommand = "cargo install --git https://github.com/deltachat/deltachat-core-rust deltachat-rpc-server"

export function NPM_NOT_FOUND_SUPPORTED_PLATFORM_ERROR (package_name) {

    return `deltachat-rpc-server not found:

- Install it with "npm i ${package_name}"
- or download/compile deltachat-rpc-server for your platform and
 - either put it into your PATH (for example with "${cargoInstallCommand}")
 - or set the "${ENV_VAR_NAME}" env var to the path to deltachat-rpc-server"`
}

export function NPM_NOT_FOUND_UNSUPPORTED_PLATFORM_ERROR () {

    return `deltachat-rpc-server not found:

Unfortunately no prebuild is available for your system, so you need to provide deltachat-rpc-server yourself.

- Download or Compile deltachat-rpc-server for your platform and
 - either put it into your PATH (for example with "${cargoInstallCommand}")
 - or set the "${ENV_VAR_NAME}" env var to the path to deltachat-rpc-server"`
}
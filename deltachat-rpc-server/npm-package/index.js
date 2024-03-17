//@ts-check
import os from "os"
import { join } from "path"
import process from "process"
import { NPM_NOT_FOUND_SUPPORTED_PLATFORM_ERROR, NPM_NOT_FOUND_UNSUPPORTED_PLATFORM_ERROR } from "./src/errors"

const ENV_VAR_NAME = "DELTA_CHAT_RPC_SERVER"

// find the rpc server
// - [ ] env var
// - [ ] in npm packages
// - [ ] in PATH -> but there we need extra version check

// exports
// - [ ] expose from where the rpc server was loaded (env_var, prebuild or npm package)
// - [ ] a raw starter that has a stdin/out handle thingie like desktop uses
// - [ ] a function that already wraps the stdio handle from aboe into the deltachat jsonrpc bindings


function findRpcServerInNodeModules() {
    const arch = os.arch()
    const operating_system = process.platform

    const package_name = `@deltachat/stdio-rpc-server-${operating_system}-${arch}`

    try {
        return join(require.resolve(package_name), "deltachat-rpc-server")
    } catch (error) {
        console.debug("findRpcServerInNodeModules", error)
        if(
            Object.keys(require("./package.json").optionalDependencies).includes(package_name)
        ) {
            throw new Error(NPM_NOT_FOUND_SUPPORTED_PLATFORM_ERROR())
        } else {
            throw new Error(NPM_NOT_FOUND_UNSUPPORTED_PLATFORM_ERROR())
        }
    }
}


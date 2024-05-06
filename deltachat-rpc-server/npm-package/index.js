//@ts-check
import { execFile } from "child_process";
import { stat, readdir } from "fs/promises";
import os from "os";
import { join } from "path";
import { basename } from "path/posix";
import process from "process";
import { promisify } from "node:util";
import {
  ENV_VAR_NAME,
  PATH_EXECUTABLE_NAME,
  SKIP_SEARCH_IN_PATH,
} from "./src/const.js";
import {
  ENV_VAR_LOCATION_NOT_FOUND,
  NPM_NOT_FOUND_SUPPORTED_PLATFORM_ERROR,
  NPM_NOT_FOUND_UNSUPPORTED_PLATFORM_ERROR,
} from "./src/errors.js";

// Because this is not compiled by typescript, nodejs needs this stuff (` assert { type: "json" };`)
import package_json from "./package.json" assert { type: "json" };
import { createRequire } from "node:module";

const { resolve } = createRequire(import.meta.url);

// find the rpc server
// - [X] env var
// - [X] in npm packages
// - [X] in PATH -> but there we need extra version check

// exports
// - [ ] expose from where the rpc server was loaded (env_var, prebuild or npm package)
// - [ ] a raw starter that has a stdin/out handle thingie like desktop uses
// - [ ] a function that already wraps the stdio handle from aboe into the deltachat jsonrpc bindings

function findRPCServerInNodeModules() {
  const arch = os.arch();
  const operating_system = process.platform;
  const package_name = `@deltachat/stdio-rpc-server-${operating_system}-${arch}`;
  try {
    return resolve(package_name);
  } catch (error) {
    console.debug("findRpcServerInNodeModules", error);
    if (Object.keys(package_json.optionalDependencies).includes(package_name)) {
      throw new Error(NPM_NOT_FOUND_SUPPORTED_PLATFORM_ERROR(package_name));
    } else {
      throw new Error(NPM_NOT_FOUND_UNSUPPORTED_PLATFORM_ERROR());
    }
  }
}

export default async function findRPCServer(
  options = { skipSearchInPath: false, disableEnvPath: false }
) {
    const { skipSearchInPath, disableEnvPath } = options
  // 1. check if it is set as env var
  if (process.env[ENV_VAR_NAME] && !disableEnvPath) {
    try {
      if (!(await stat(process.env[ENV_VAR_NAME])).isFile()) {
        throw new Error(
          `expected ${ENV_VAR_NAME} to point to the deltachat-rpc-server executable`
        );
      }
    } catch (error) {
      throw new Error(ENV_VAR_LOCATION_NOT_FOUND());
    }
    return process.env[ENV_VAR_NAME];
  }

  // 2. check if it can be found in PATH
  if (!process.env[SKIP_SEARCH_IN_PATH] && !skipSearchInPath) {
    const path_dirs = process.env["PATH"].split(/:|;/);
    // check cargo dir first
    const cargo_dirs = path_dirs.filter((p) => p.endsWith(".cargo/bin"));
    const findExecutable = async (directory) => {
      const files = await readdir(directory);
      const file = files.find((p) =>
        basename(p).includes(PATH_EXECUTABLE_NAME)
      );
      if (file) {
        return join(directory, file);
      } else {
        throw null;
      }
    };
    const executable_search = // TODO make code simpler to read
      (await Promise.allSettled(cargo_dirs.map(findExecutable))).find(
        ({ status }) => status === "fulfilled"
      ) ||
      (await Promise.allSettled(path_dirs.map(findExecutable))).find(
        ({ status }) => status === "fulfilled"
      );
    // TODO maybe we could the system do this stuff automatically
    // by just trying to execute it and then use "which" (unix) or "where" (windows) to get the path to the executable
    if (executable_search.status === "fulfilled") {
      const executable = executable_search.value;
      // test if it is the right version
      try {
        // for some unknown reason it is in stderr and not in stdout
        const { stderr } = await promisify(execFile)(executable, ["--version"]);
        const version = stderr.slice(0,stderr.indexOf('\n'))
        if (package_json.version !== version) {
          throw new Error(
            `version mismatch: (npm package: ${package_json.version})  (installed ${PATH_EXECUTABLE_NAME} version: ${version})`
          );
        } else {
          return executable;
        }
      } catch (error) {
        console.error(
          "Found executable in PATH, but there was an error: " + error
        );
        console.error("So falling back to using prebuild...");
      }
    }
  }
  // 3. check for prebuilds

  return findRPCServerInNodeModules();
}

// TODO script for local development (build for current platform and change link in package.json to be local)
// TODO script to build prebuild for current platform
// TODO disable PATH search in desktop (hardening, so it can not be easily replaced in production)

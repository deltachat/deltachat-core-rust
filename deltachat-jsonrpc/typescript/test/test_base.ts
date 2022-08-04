import { tmpdir } from "os";
import { join, resolve } from "path";
import { mkdtemp, rm } from "fs/promises";
import { existsSync } from "fs";
import { spawn, exec } from "child_process";
import fetch from "node-fetch";

export const RPC_SERVER_PORT = 20808;

export type RpcServerHandle = {
  url: string,
  close: () => Promise<void>
}

export async function startServer(port: number = RPC_SERVER_PORT): Promise<RpcServerHandle> {
  const tmpDir = await mkdtemp(join(tmpdir(), "deltachat-jsonrpc-test"));

  const pathToServerBinary = resolve(join(await getTargetDir(), "debug/deltachat-jsonrpc-server"));
  console.log('using server binary: ' + pathToServerBinary);

  if (!existsSync(pathToServerBinary)) {
    throw new Error(
      "server executable does not exist, you need to build it first" +
        "\nserver executable not found at " +
        pathToServerBinary
    );
  }

  const server = spawn(pathToServerBinary, {
    cwd: tmpDir,
    env: {
      RUST_LOG: process.env.RUST_LOG || "info",
      DC_PORT: '' + port
    },
  });
  let shouldClose = false;

  server.on("exit", () => {
    if (shouldClose) {
      return;
    }
    throw new Error("Server quit");
  });

  server.stderr.pipe(process.stderr);
  server.stdout.pipe(process.stdout)

  const url = `ws://localhost:${port}/ws`

  return {
    url,
    close: async () => {
      shouldClose = true;
      if (!server.kill()) {
        console.log("server termination failed");
      }
      await rm(tmpDir, { recursive: true });
    },
  };
}

export async function createTempUser(url: string) {
  const response = await fetch(url, {
    method: "POST", 
    headers: {
      "cache-control": "no-cache",
    },
  });
  if (!response.ok) throw new Error('Received invalid response')
  return response.json(); 
}

function getTargetDir(): Promise<string> {
  return new Promise((resolve, reject) => {
    exec(
      "cargo metadata --no-deps --format-version 1",
      (error, stdout, _stderr) => {
        if (error) {
          console.log("error", error);
          reject(error);
        } else {
          try {
            const json = JSON.parse(stdout);
            resolve(json.target_directory);
          } catch (error) {
            console.log("json error", error);
            reject(error);
          }
        }
      }
    );
  });
}


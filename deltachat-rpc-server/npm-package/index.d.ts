import { StdioDeltaChat } from "@deltachat/jsonrpc-client";

export interface SearchOptions {
  /** whether to disable looking for deltachat-rpc-server inside of $PATH */
  skipSearchInPath: boolean;

  /** whether to disable the DELTA_CHAT_RPC_SERVER environment variable */
  disableEnvPath: boolean;
}

/**
 * 
 * @returns absolute path to deltachat-rpc-server binary
 * @throws when it is not found
 */
export function getRPCServerPath(
  options?: Partial<SearchOptions>
): Promise<string>;



export type DeltaChatOverJsonRpcServer = StdioDeltaChat & {
    readonly pathToServerBinary: string;
};

export interface StartOptions {
  /** whether to disable outputting stderr to the parent process's stderr */
  muteStdErr: boolean;
}

/**
 * 
 * @param directory directory for accounts folder
 * @param options 
 */
export function startDeltaChat(directory: string, options?: Partial<SearchOptions & StartOptions> ): Promise<DeltaChatOverJsonRpcServer>


export namespace FnTypes {
    export type getRPCServerPath = typeof getRPCServerPath
    export type startDeltaChat = typeof startDeltaChat
}
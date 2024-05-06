
export interface SearchOptions {
    /** whether to disable looking for deltachat-rpc-server inside of $PATH */
    skipSearchInPath: boolean

    /** whether to disable the DELTA_CHAT_RPC_SERVER environment variable */
    disableEnvPath: boolean
}

export default function getRPCServerPath(options: Partial<SearchOptions>): Promise<string> 
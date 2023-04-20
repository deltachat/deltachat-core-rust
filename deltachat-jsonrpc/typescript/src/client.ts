import * as T from "../generated/types.js";
import * as RPC from "../generated/jsonrpc.js";
import { RawClient } from "../generated/client.js";
import { WebsocketTransport, BaseTransport, Request } from "yerpc";

type DCWireEvent<T extends Event> = {
  event: T;
  contextId: number;
};

type Events = { ALL: (accountId: number, event: Event) => void } & {
  [Property in Event["type"]]: (
    accountId: number,
    event: Extract<Event, { type: Property }>
  ) => void;
};

type ContextEvents = { ALL: (event: Event) => void } & {
  [Property in Event["type"]]: (
    event: Extract<Event, { type: Property }>
  ) => void;
};

export type DcEvent = Event;
export type DcEventType<T extends Event["type"]> = Extract<Event, { type: T }>;

export class BaseDeltaChat<Transport extends BaseTransport<any>> {
  rpc: RawClient;
  account?: T.Account;
  constructor(public transport: Transport) {
    this.rpc = new RawClient(this.transport);
  }

  async listAccounts(): Promise<T.Account[]> {
    return await this.rpc.getAllAccounts();
  }
}

export type Opts = {
  url: string;
};

export const DEFAULT_OPTS: Opts = {
  url: "ws://localhost:20808/ws",
};
export class DeltaChat extends BaseDeltaChat<WebsocketTransport> {
  opts: Opts;
  close() {
    this.transport.close();
  }
  constructor(opts?: Opts | string) {
    if (typeof opts === "string") opts = { url: opts };
    if (opts) opts = { ...DEFAULT_OPTS, ...opts };
    else opts = { ...DEFAULT_OPTS };
    const transport = new WebsocketTransport(opts.url);
    super(transport);
    this.opts = opts;
  }
}

export class StdioDeltaChat extends BaseDeltaChat<StdioTransport> {
  close() {}
  constructor(input: any, output: any) {
    const transport = new StdioTransport(input, output);
    super(transport);
  }
}

export class StdioTransport extends BaseTransport {
  constructor(public input: any, public output: any) {
    super();

    var buffer = "";
    this.output.on("data", (data: any) => {
      buffer += data.toString();
      while (buffer.includes("\n")) {
        const n = buffer.indexOf("\n");
        const line = buffer.substring(0, n);
        const message = JSON.parse(line);
        this._onmessage(message);
        buffer = buffer.substring(n + 1);
      }
    });
  }

  _send(message: any): void {
    const serialized = JSON.stringify(message);
    this.input.write(serialized + "\n");
  }
}

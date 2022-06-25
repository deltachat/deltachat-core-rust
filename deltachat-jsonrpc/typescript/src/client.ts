import * as T from "../generated/types.js";
import * as RPC from "../generated/jsonrpc.js";
import { RawClient } from "../generated/client.js";
import { EventTypeName } from "../generated/events.js";
import { WebsocketTransport, BaseTransport, Request } from "yerpc";
import { TinyEmitter } from "tiny-emitter";

export type DeltachatEvent = {
  id: EventTypeName;
  contextId: number;
  field1: any;
  field2: any;
};
export type Events = Record<
  EventTypeName | "ALL",
  (event: DeltachatEvent) => void
>;

export class BaseDeltachat<
  Transport extends BaseTransport
> extends TinyEmitter<Events> {
  rpc: RawClient;
  account?: T.Account;
  constructor(protected transport: Transport) {
    super();
    this.rpc = new RawClient(this.transport);
    this.transport.on("request", (request: Request) => {
      const method = request.method;
      if (method === "event") {
        const event = request.params! as DeltachatEvent;
        this.emit(event.id, event);
        this.emit("ALL", event);

        if (this.contextEmitters[event.contextId]) {
          this.contextEmitters[event.contextId].emit(event.id, event);
          this.contextEmitters[event.contextId].emit("ALL", event);
        }
      }
    });
  }

  async listAccounts(): Promise<T.Account[]> {
    return await this.rpc.getAllAccounts();
  }

  private contextEmitters: TinyEmitter<Events>[] = [];

  getContextEvents(account_id: number) {
    if (this.contextEmitters[account_id]) {
      return this.contextEmitters[account_id];
    } else {
      this.contextEmitters[account_id] = new TinyEmitter();
      return this.contextEmitters[account_id];
    }
  }
}

export type Opts = {
  url: string;
};

export const DEFAULT_OPTS: Opts = {
  url: "ws://localhost:20808/ws",
};
export class Deltachat extends BaseDeltachat<WebsocketTransport> {
  opts: Opts;
  close() {
    this.transport._socket.close();
  }
  constructor(opts: Opts | string | undefined) {
    if (typeof opts === "string") opts = { url: opts };
    if (opts) opts = { ...DEFAULT_OPTS, ...opts };
    else opts = { ...DEFAULT_OPTS };
    super(new WebsocketTransport(opts.url));
    this.opts = opts;
  }
}

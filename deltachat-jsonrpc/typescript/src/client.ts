import * as T from "../generated/types.js";
import * as RPC from "../generated/jsonrpc.js";
import { RawClient } from "../generated/client.js";
import { WebsocketTransport, BaseTransport, Request } from "yerpc";
import { eventIdToName } from "./events.js";
import { TinyEmitter } from "tiny-emitter";

export type EventNames = ReturnType<typeof eventIdToName> | "ALL";
export type WireEvent = {
  id: number;
  contextId: number;
  field1: any;
  field2: any;
};
export type DeltachatEvent = WireEvent & { name: EventNames };
export type Events = Record<EventNames, (event: DeltachatEvent) => void>;

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
        const params = request.params! as WireEvent;
        const name = eventIdToName(params.id);
        const event = { name, ...params };
        this.emit(name, event);
        this.emit("ALL", event);

        if (this.contextEmitters[params.contextId]) {
          this.contextEmitters[params.contextId].emit(name, event);
          this.contextEmitters[params.contextId].emit("ALL", event);
        }
      }
    });
  }

  async selectAccount(id: number) {
    await this.rpc.selectAccount(id);
    this.account = await this.rpc.getAccountInfo(id);
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

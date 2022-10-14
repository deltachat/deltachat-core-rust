import * as T from "../generated/types.js";
import * as RPC from "../generated/jsonrpc.js";
import { RawClient } from "../generated/client.js";
import { Event } from "../generated/events.js";
import { WebsocketTransport, BaseTransport, Request } from "yerpc";
import { TinyEmitter } from "tiny-emitter";

type DCWireEvent<T extends Event> = {
  event: T;
  contextId: number;
};
// export type Events = Record<
//   Event["type"] | "ALL",
//   (event: DeltaChatEvent<Event>) => void
// >;

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

export class BaseDeltaChat<
  Transport extends BaseTransport<any>
> extends TinyEmitter<Events> {
  rpc: RawClient;
  account?: T.Account;
  private contextEmitters: TinyEmitter<ContextEvents>[] = [];
  constructor(public transport: Transport) {
    super();
    this.rpc = new RawClient(this.transport);
    this.transport.on("request", (request: Request) => {
      const method = request.method;
      if (method === "event") {
        const event = request.params! as DCWireEvent<Event>;
        this.emit(event.event.type, event.contextId, event.event as any);
        this.emit("ALL", event.contextId, event.event as any);

        if (this.contextEmitters[event.contextId]) {
          this.contextEmitters[event.contextId].emit(
            event.event.type,
            event.event as any
          );
          this.contextEmitters[event.contextId].emit("ALL", event.event);
        }
      }
    });
  }

  async listAccounts(): Promise<T.Account[]> {
    return await this.rpc.getAllAccounts();
  }

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

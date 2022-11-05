# Delta Chat RPC python client

RPC client connects to standalone Delta Chat RPC server `deltachat-rpc-server`
and provides asynchronous interface to it.

## Getting started

To use Delta Chat RPC client, first build a `deltachat-rpc-server` with `cargo build -p deltachat-rpc-server`.
Install it anywhere in your `PATH`.

## Testing

1. Build `deltachat-rpc-server` with `cargo build -p deltachat-rpc-server`.
2. Run `tox`.

Additional arguments to `tox` are passed to pytest, e.g. `tox -- -s` does not capture test output.

## Using in REPL

It is recommended to use IPython, because it supports using `await` directly
from the REPL.

```
PATH="../target/debug:$PATH" ipython
...
In  [1]: from deltachat_rpc_client import *
In  [2]: dc = Deltachat(await start_rpc_server())
In  [3]: await dc.get_all_accounts()
Out [3]: []
In  [4]: alice = await dc.add_account()
In  [5]: (await alice.get_info())["journal_mode"]
Out [5]: 'wal'
```

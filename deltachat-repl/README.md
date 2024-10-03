# Delta Chat REPL

This is a simple [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop) frontend build on top of delta chat core.
It's purpose is to help with quick testing during development, it is not meant for end users.

Dependencies:
- if you want to use `getqr` you need `qrencode` (macOS: `brew install qrencode`)

## Usage

```
cargo run <path to deltachat db>
```

Type in `help` to learn about what comands are available.

## Usage with `tokio-console`

Tokio is the async runtime that delta chat core uses.
Core uses tokio tasks, which is something similar to a thread.
`tokio-console` is like a task manager for these tokio-tasks.

Examples of tasks:
- The event loop in the repl tool which processes events received from core
- The repl loop itself which waits for and executes user commands
- The imap task that manages imap connection in core

```
RUSTFLAGS="--cfg tokio_unstable" cargo run <path to deltachat db>
```

Then in a new console window start [`tokio-console`](https://github.com/tokio-rs/console).
You can install it via `cargo install tokio-console`.

### Quick Example

An example session in the repl tool.

```
RUSTFLAGS="--cfg tokio_unstable" cargo run test-db/db
setqr dcaccount:https://nine.testrun.org/new
configure
connect
listchats
getqr
```

If it crashes you can just start it again and use the openpgp4fpr url instead of scanning the code from the terminal.
Or install `qrencode` to fix the crash and run `getqr` again.

Use the qrcode/openpgp4fpr link to setup the contact on deltachat.
Then write a message to that new contact, after that we can accept the chat in the repl tool and send a reply:

```
listchats
accept 12
chat 12
send hi!
chat
```
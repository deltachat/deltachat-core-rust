# Delta Chat REPL

This is a [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop) frontend built on top of Delta Chat core.
Its purpose is to help with testing during development, it is not meant for end users.

Dependencies:
- If you want to use `getqr` you need `qrencode` (To install, use your system's package manager)

## Usage

```
cargo run <path to deltachat db>
```

Type in `help` to learn about what commands are available.

## Usage with `tokio-console`

Tokio is an async runtime that Delta Chat core uses.
Core uses Tokio tasks, which are something similar to threads.
`tokio-console` is like a task manager for these Tokio tasks.

Examples of tasks:
- The event loop in the REPL tool which processes events received from core
- The REPL loop itself which waits for and executes user commands
- The IMAP task that manages IMAP connection in core

```
RUSTFLAGS="--cfg tokio_unstable" cargo run <path to deltachat db>
```

Then in a new console window start [`tokio-console`](https://github.com/tokio-rs/console).
You can install it via `cargo install --locked tokio-console`.

### Example

An example session in the REPL tool:

```
RUSTFLAGS="--cfg tokio_unstable" cargo run test-db/db
setqr dcaccount:https://nine.testrun.org/new
configure
connect
listchats
getqr
```

If it crashes you can just start it again and use the openpgp4fpr URL instead of scanning the code from the terminal.
Or install `qrencode` to fix the crash and run `getqr` again.

Use the qrcode/openpgp4fpr link to setup the contact on Delta Chat.
Then write a message to that new contact, after that you can accept the chat in the REPL tool and send a reply:

```
listchats
accept 12
chat 12
send hi!
chat
```
# Delta Chat Rust

> Deltachat-core written in Rust 

[![CircleCI build status][circle-shield]][circle] [![Appveyor build status][appveyor-shield]][appveyor]

## Installing Rust and Cargo

To download and install the official compiler for the Rust programming language, and the Cargo package manager, run the command in your user environment:

```
curl https://sh.rustup.rs -sSf | sh
```

## Using the CLI client

Compile and run Delta Chat Core command line utility, using `cargo`:

```
cargo run --example repl -- ~/deltachat-db
```
where ~/deltachat-db is the database file. Delta Chat will create it if it does not exist.

Configure your account (if not already configured):

```
Delta Chat Core is awaiting your commands.
> set addr your@email.org
> set mail_pw yourpassword
> configure
```

Connect to your mail server (if already configured):

```
> connect
```

Create a contact:

```
> addcontact yourfriends@email.org
Command executed successfully.
```

List contacts:

```
> listcontacts
Contact#10: <name unset> <yourfriends@email.org>
Contact#1: Me √√ <your@email.org>
```

Create a chat with your friend and send a message:

```
> createchat 10
Single#10 created successfully.
> chat 10
Single#10: yourfriends@email.org [yourfriends@email.org]
> send hi
Message sent.
```

If `yourfriend@email.org` uses DeltaChat, but does not receive message just
sent, it is advisable to check `Spam` folder. It is known that at least
`gmx.com` treat such test messages as spam, unless told otherwise with web
interface.

List messages when inside a chat:

```
> chat
```

For more commands type:

```
> help
```

## Development

```sh
# run tests
$ cargo test --all
# build c-ffi
$ cargo build -p deltachat_ffi --release
```

## Debugging environment variables 

- `DCC_IMAP_DEBUG`: if set IMAP protocol commands and responses will be
  printed

- `DCC_MIME_DEBUG`: if set outgoing and incoming message will be printed 



### Expensive tests

Some tests are expensive and marked with `#[ignore]`, to run these
use the `--ignored` argument to the test binary (not to cargo itself):
```sh
$ cargo test -- --ignored
```

## Features

- `vendored`: When using Openssl for TLS, this bundles a vendored version.
- `nightly`: Enable nightly only performance and security related features.

[circle-shield]: https://img.shields.io/circleci/project/github/deltachat/deltachat-core-rust/master.svg?style=flat-square
[circle]: https://circleci.com/gh/deltachat/deltachat-core-rust/
[appveyor-shield]: https://ci.appveyor.com/api/projects/status/lqpegel3ld4ipxj8/branch/master?style=flat-square
[appveyor]: https://ci.appveyor.com/project/dignifiedquire/deltachat-core-rust/branch/master

## Language bindings and frontend projects

Language bindings are available for:

- [C](https://c.delta.chat)
- [Node.js](https://www.npmjs.com/package/deltachat-node)
- [Python](https://py.delta.chat)
- [Go](https://github.com/hugot/go-deltachat/)
- **Java** and **Swift** (contained in the Android/iOS repos)

The following "frontend" projects make use of the Rust-library
or its language bindings:

- [Android](https://github.com/deltachat/deltachat-android)
- [iOS](https://github.com/deltachat/deltachat-ios)
- [Desktop](https://github.com/deltachat/deltachat-desktop)
- [Pidgin](https://code.ur.gs/lupine/purple-plugin-delta/)
- several **Bots**

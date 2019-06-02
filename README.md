# Delta Chat Rust

> Project porting deltachat-core to rust

[![CircleCI build status][circle-shield]][circle] [![Appveyor build status][appveyor-shield]][appveyor]

Current commit on deltachat/deltachat-core: `12ef73c8e76185f9b78e844ea673025f56a959ab`.

## Using the CLI client

Run using `cargo`:

```
cargo run --example repl -- /path/to/db
```

Configure your account (if not already configured):

```
Delta Chat Core is awaiting your commands.
> set addr your@email.org
> set mail_pw yourpassword
> configure
```

If you're already configured it's enough to `> connect`.

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

For more commands type `> help`.

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

[circle-shield]: https://img.shields.io/circleci/project/github/deltachat/deltachat-core-rust/master.svg?style=flat-square
[circle]: https://circleci.com/gh/deltachat/deltachat-core-rust/
[appveyor-shield]: https://ci.appveyor.com/api/projects/status/lqpegel3ld4ipxj8/branch/master?style=flat-square
[appveyor]: https://ci.appveyor.com/project/dignifiedquire/deltachat-core-rust/branch/master

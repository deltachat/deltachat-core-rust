# deltachat-jsonrpc

## Build Requirements

- Linux or Mac, scrips make use of features like `>` pipes and `&&` (maybe the newer versions of powershell support them, but I didn't try that.)
- rust (installed via rustup)

## Start the webserver

The webserver is an example usage. Goal of it is to be usable both as example and as base for deltachat-kaiOS.

```sh
RUST_LOG=info cargo run --features webserver
```

## Generate Typescript Bindings

```sh
cd typescript
npm i
npm run build
```

## Run the development example

Mac

```sh
alias firefox=/Applications/Firefox.app/Contents/MacOS/firefox
npm run example:build && firefox --devtools $(pwd)/example/browser-example.html
```

Linux:

```sh
npm run example:run
```

## Compiling server for kaiOS or android:

```sh
cross build --features=webserver --target armv7-linux-androideabi --release
```

## Run the tests

### Rust tests

```
cargo test --features=webserver
```

### Typescript

```
cd typescript
npm run test
```

For the online tests to run you need a test account token for a mailadm instance,
you can use docker to spin up a local instance: https://github.com/deltachat/docker-mailadm

> set the env var `DCC_NEW_TMP_EMAIL` to your mailadm token: example:
> `DCC_NEW_TMP_EMAIL=https://testrun.org/new_email?t=1h_195dksa6544`

If your test fail with server shutdown at the start, then you might have a process from a last run still running probably and you need to kill that process manually to continue.

#### Test Coverage

You can test coverage with `npm run coverage`, but you need to have `DCC_NEW_TMP_EMAIL` set, otherwise the result will be useless because some functions can only be tested with the online tests.

> If you are offline and want to see the coverage results anyway (even though they are NOT correct), you can bypass the error with `COVERAGE_OFFLINE=1 npm run coverage`

Open `coverage/index.html` for a detailed report.
`bindings.ts` is probably the most interesting file for coverage, because it describes the api functions.

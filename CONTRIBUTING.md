# Contributing guidelines

## Reporting bugs

If you found a bug, [report it on GitHub](https://github.com/deltachat/deltachat-core-rust/issues).
If the bug you found is specific to
[Android](https://github.com/deltachat/deltachat-android/issues),
[iOS](https://github.com/deltachat/deltachat-ios/issues) or
[Desktop](https://github.com/deltachat/deltachat-desktop/issues),
report it to the corresponding repository.

## Proposing features

If you have a feature request, create a new topic on the [forum](https://support.delta.chat/).

## Contributing code

If you want to contribute a code, [open a Pull Request](https://github.com/deltachat/deltachat-core-rust/pulls).

If you have write access to the repository,
push a branch named `<username>/<feature>`
so it is clear who is responsible for the branch,
and open a PR proposing to merge the change.
Otherwise fork the repository and create a branch in your fork.

You can find the list of good first issues
and a link to this guide
on the contributing page: <https://github.com/deltachat/deltachat-core-rust/contribute>

### Coding conventions

We format the code using `rustfmt`. Run `cargo fmt` prior to committing the code.
Run `scripts/clippy.sh` to check the code for common mistakes with [Clippy].

### SQL

Multi-line SQL statements should be formatted using string literals,
for example
```
    sql.execute(
        "CREATE TABLE messages (
id INTEGER PRIMARY KEY AUTOINCREMENT,
text TEXT DEFAULT '' NOT NULL -- message text
) STRICT",
    )
    .await?;
```

Do not use macros like [`concat!`](https://doc.rust-lang.org/std/macro.concat.html)
or [`indoc!](https://docs.rs/indoc).
Do not escape newlines like this:
```
    sql.execute(
        "CREATE TABLE messages ( \
id INTEGER PRIMARY KEY AUTOINCREMENT, \
text TEXT DEFAULT '' NOT NULL \
) STRICT",
    )
    .await?;
```
Escaping newlines
is prone to errors like this if space before backslash is missing:
```
"SELECT foo\
 FROM bar"
```
Literal above results in `SELECT fooFROM bar` string.
This style also does not allow using `--` comments.

---

Declare new SQL tables with [`STRICT`](https://sqlite.org/stricttables.html) keyword
to make SQLite check column types.

Declare primary keys with [`AUTOINCREMENT`](https://www.sqlite.org/autoinc.html) keyword.
This avoids reuse of the row IDs and can avoid dangerous bugs
like forwarding wrong message because the message was deleted
and another message took its row ID.

Declare all new columns as `NOT NULL`
and set the `DEFAULT` value if it is optional so the column can be skipped in `INSERT` statements.
Dealing with `NULL` values both in SQL and in Rust is tricky and we try to avoid it.
If column is already declared without `NOT NULL`, use `IFNULL` function to provide default value when selecting it.
Use `HAVING COUNT(*) > 0` clause
to [prevent aggregate functions such as `MIN` and `MAX` from returning `NULL`](https://stackoverflow.com/questions/66527856/aggregate-functions-max-etc-return-null-instead-of-no-rows).

Don't delete unused columns too early, but maybe after several months/releases, unused columns are
still used by older versions, so deleting them breaks downgrading the core or importing a backup in
an older version. Also don't change the column type, consider adding a new column with another name
instead. Finally, never change column semantics, this is especially dangerous because the `STRICT`
keyword doesn't help here.

### Commit messages

Commit messages follow the [Conventional Commits] notation.
We use [git-cliff] to generate the changelog from commit messages before the release.

With **`git cliff --unreleased`**, you can check how the changelog entry for your commit will look.

The following prefix types are used:
- `feat`: Features, e.g. "feat: Pause IO for BackupProvider". If you are unsure what's the category of your commit, you can often just use `feat`.
- `fix`: Bug fixes, e.g. "fix: delete `smtp` rows when message sending is cancelled"
- `api`: API changes, e.g. "api(rust): add `get_msg_read_receipts(context, msg_id)`"
- `refactor`: Refactorings, e.g. "refactor: iterate over `msg_ids` without `.iter()`"
- `perf`: Performance improvements, e.g. "perf: improve SQLite performance with `PRAGMA synchronous=normal`"
- `test`: Test changes and improvements to the testing framework.
- `build`: Build system and tool configuration changes, e.g. "build(git-cliff): put "ci" commits into "CI" section of changelog"
- `ci`: CI configuration changes, e.g. "ci: limit artifact retention time for `libdeltachat.a` to 1 day"
- `docs`: Documentation changes, e.g. "docs: add contributing guidelines"
- `chore`: miscellaneous tasks, e.g. "chore: add `.DS_Store` to `.gitignore`"

Release preparation commits are marked as "chore(release): prepare for vX.Y.Z".

If you intend to squash merge the PR from the web interface,
make sure the PR title follows the conventional commits notation
as it will end up being a commit title.
Otherwise make sure each commit title follows the conventional commit notation.

#### Breaking Changes

Use a `!` to mark breaking changes, e.g. "api!: Remove `dc_chat_can_send`".

Alternatively, breaking changes can go into the commit description, e.g.:

```
fix: Fix race condition and db corruption when a message was received during backup

BREAKING CHANGE: You have to call `dc_stop_io()`/`dc_start_io()` before/after `dc_imex(DC_IMEX_EXPORT_BACKUP)`
```

#### Multiple Changes in one PR

If you have multiple changes in one PR, create multiple conventional commits, and then do a rebase merge. Otherwise, you should usually do a squash merge.

[Clippy]: https://doc.rust-lang.org/clippy/
[Conventional Commits]: https://www.conventionalcommits.org/
[git-cliff]: https://git-cliff.org/

### Errors

Delta Chat core mostly uses [`anyhow`](https://docs.rs/anyhow/) errors.
When using [`Context`](https://docs.rs/anyhow/latest/anyhow/trait.Context.html),
capitalize it but do not add a full stop as the contexts will be separated by `:`.
For example:
```
.with_context(|| format!("Unable to trash message {msg_id}"))
```

All errors should be handled in one of these ways:
- With `if let Err() =` (incl. logging them into `warn!()`/`err!()`).
- With `.log_err().ok()`.
- Bubbled up with `?`.

`backtrace` feature is enabled for `anyhow` crate
and `debug = 1` option is set in the test profile.
This allows to run `RUST_BACKTRACE=1 cargo test`
and get a backtrace with line numbers in resultified tests
which return `anyhow::Result`.

### Logging

For logging, use `info!`, `warn!` and `error!` macros.
Log messages should be capitalized and have a full stop in the end. For example:
```
info!(context, "Ignoring addition of {added_addr:?} to {chat_id}.");
```

Format anyhow errors with `{:#}` to print all the contexts like this:
```
error!(context, "Failed to set selfavatar timestamp: {err:#}.");
```

### Reviewing

Once a PR has an approval and passes CI, it can be merged.

PRs from a branch created in the main repository, i.e. authored by those who have write access, are merged by their authors.
This is to ensure that PRs are merged as intended by the author,
e.g. as a squash merge, by rebasing from the web interface or manually from the command line.

If you do not have access to the repository and created a PR from a fork,
ask the maintainers to merge the PR and say how it should be merged.

## Other ways to contribute

For other ways to contribute, refer to the [website](https://delta.chat/en/contribute).

# Releasing a new version of DeltaChat core

For example, to release version 1.116.0 of the core, do the following steps.

1. Resolve all [blocker issues](https://github.com/deltachat/deltachat-core-rust/labels/blocker).

2. Run `npm run build:core:constants` in the root of the repository
   and commit generated `node/constants.js`, `node/events.js` and `node/lib/constants.js`.

3. Update the changelog: `git cliff --unreleased --tag 1.116.0 --prepend CHANGELOG.md` or `git cliff -u -t 1.116.0 -p CHANGELOG.md`.

4. add a link to compare previous with current version to the end of CHANGELOG.md:
  `[1.116.0]: https://github.com/deltachat/deltachat-core-rust/compare/v1.115.2...v1.116.0`

5. Update the version by running `scripts/set_core_version.py 1.116.0`.

6. Commit the changes as `chore(release): prepare for 1.116.0`.
   Optionally, use a separate branch like `prep-1.116.0` for this commit and open a PR for review.

7. Tag the release: `git tag -a v1.116.0`.

8. Push the release tag: `git push origin v1.116.0`.

9. Create a GitHub release: `gh release create v1.116.0 -n ''`.

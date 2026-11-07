# commit-msg-lint

A Rust CLI that validates commit messages against the Conventional Commits
specification using a TOML configuration.

```sh
cargo install --path .
printf 'feat: add parser\n' | commit-msg-lint
commit-msg-lint --edit .git/COMMIT_EDITMSG
```

With `--strict`, warnings result in exit code 1. Use `--config <path>` to specify a configuration
file. If `--config` is omitted, the CLI uses Git commands to find the repository root and reads
`commit-msg-lint.toml` from there. See the
[requirements](docs/requirements/commit-message-linter.md) for the configuration format and details
about each Rule.

Display Column measurements use `unicode-width` 0.2.2 (Unicode 17.0.0). When updating the Unicode
data, update this information and the conformance tests, and document the change in the release
notes.

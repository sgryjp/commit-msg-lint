# commit-msg-lint

A Rust CLI that checks commit messages using Conventional Commit Rules and a TOML configuration.

## Design philosophy

- Measure display width using East Asian character widths, not character counts.
- Separate recommended limits from required ones with warnings and errors, giving agents discretion
  to preserve meaning.

## Install and use

From a checkout with the Rust toolchain installed:

```sh
cargo install --path .
printf 'feat: add parser\n' | commit-msg-lint
commit-msg-lint --edit .git/COMMIT_EDITMSG
```

By default, the CLI reads `commit-msg-lint.toml` from the Git repository root, if present. Use
`--config <path>` to select another file. With `--strict`, warnings result in exit code 1.

To use the CLI from an existing `commit-msg` Git hook, call:

```sh
commit-msg-lint --edit "$1"
```

The CLI does not install or manage hooks. It checks a single message from a file or standard input;
Git history inspection and custom Rules are not supported.

## Web demo

[Try the web demo](https://sgryjp.github.io/commit-msg-lint/) to check a commit message with an
optional TOML configuration, without installing the CLI. The linter runs in your browser; the
message and configuration are not sent to a server.

## Reference

- [CLI](docs/cli.md): options, input, diagnostics, and exit codes
- [Configuration](docs/configuration.md): file format, defaults, and Rule settings
- [Message format](docs/message-format.md): parsing and Rule behavior

For development and documentation checks, see [Contributing](CONTRIBUTING.md).

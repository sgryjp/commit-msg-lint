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
Git history inspection and custom Rules are not supported. Not every Git commit path invokes the
hook. For agent integrations, `commit-msg-lint --help` points to `--help-for-agents`, which prints
[agent usage guidance](docs/agent-usage.md) without reading input or configuration.

By default, the CLI excludes Git comment lines and cuts the message at a scissors line before
linting, for both standard input and `--edit`. This helps with editor and verbose commit messages,
but can exclude content that Git retains, including comments in default `-m` / `-F` commits.
The [CLI reference](docs/cli.md#message-filtering) explains the independent environment controls,
the Git prefix lookup, and how to inspect a completed message as supplied. The public library and
Web demo check the supplied message without this filtering.

## Web demo

[Try the web demo](https://sgryjp.github.io/commit-msg-lint/) to check a commit message with an
optional TOML configuration, without installing the CLI. The linter runs in your browser; the
message and configuration are not sent to a server. The demo checks the supplied message without
Git comment removal or scissors truncation.

## Reference

- [CLI](docs/cli.md): options, input, diagnostics, and exit codes
- [Configuration](docs/configuration.md): file format, defaults, and Rule settings
- [Message format](docs/message-format.md): parsing and Rule behavior

For development and documentation checks, see [Contributing](CONTRIBUTING.md).

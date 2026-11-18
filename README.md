# commit-msg-lint

A Rust CLI that checks commit messages using Conventional Commit Rules and a JSONC configuration.

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

By default, the CLI reads `commit-msg-lint.jsonc` from the Git repository root, if present. Use
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

By default, recognized Git-generated and hosting-service Header forms are silently ignored in
entirety, including width and severity checks. Matching is text-based, so it can also exclude
manually written messages. Configure or disable these patterns as described in
[ignored messages](docs/configuration.md#ignored-messages).

## pre-commit

This repository defines a `commit-msg` hook for [pre-commit](https://pre-commit.com/). Add it to
your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/sgryjp/commit-msg-lint
    rev: <tag-or-commit-containing-this-hook>
    hooks:
      - id: commit-msg-lint
```

Replace the placeholder with a release tag or commit SHA whose tree contains
`.pre-commit-hooks.yaml`; the placeholder is not an installable revision. Install the Git hook with:

```sh
pre-commit install --hook-type commit-msg
```

This installs the `commit-msg` stage, which a plain `pre-commit install` does not enable. On its
first run, pre-commit creates a Rust environment and builds the CLI. If `cargo --version` succeeds,
pre-commit uses the system toolchain; otherwise, it bootstraps Rust through rustup and installs the
stable toolchain by default. This may download rustup and Rust components, so bootstrapping requires
network access. Cargo may also need network access to fetch uncached crate dependencies for the
build. Restricted or offline environments must provide the needed toolchain and dependencies or
allow those downloads. The Rust setup runs `cargo install --bins` without `--locked`, so Cargo may
resolve dependencies without following the repository's `Cargo.lock`.

Warnings succeed by default, in keeping with the distinction between recommended and required
limits. The hook displays diagnostics even when it succeeds, so these warnings remain visible. To
make warnings fail, override the hook arguments in your configuration:

```yaml
      - id: commit-msg-lint
        args: [--strict, --edit]
```

The hook definition keeps its command in `entry: commit-msg-lint` and supplies `--edit` through
`args`. pre-commit does not allow users to override a remote hook's `entry`; overriding `args`
keeps the message-file argument after the CLI options, so strict mode works with the same hook ID.

The CLI's default comment and scissors filtering is an approximation, not Git's final message
cleanup. For example, the CLI removes comment lines by default, while Git's default cleanup for
`-m` and `-F` commits retains them; Git may also remove whitespace that the CLI checks. See the
[message filtering details](docs/cli.md#message-filtering) before relying on either behavior.

By default, recognized generated messages—including merge, revert, and autosquash subjects—are
ignored in full, even with `--strict`. Matching is based on text, so a manually written message can
also be ignored. See [ignored messages](docs/configuration.md#ignored-messages) for the patterns and
limits.

## Web demo

[Try the web demo](https://sgryjp.github.io/commit-msg-lint/) to check a commit message with an
optional JSONC configuration, without installing the CLI. The linter runs in your browser; the
message and configuration are not sent to a server. The demo checks the supplied message without
Git comment removal or scissors truncation.

## Reference

- [CLI](docs/cli.md): options, input, diagnostics, and exit codes
- [Configuration](docs/configuration.md): file format, defaults, and Rule settings
- [Message format](docs/message-format.md): parsing and Rule behavior

For development and documentation checks, see [Contributing](CONTRIBUTING.md).

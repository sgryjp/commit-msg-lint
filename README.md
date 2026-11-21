# commit-msg-lint

A Rust CLI that checks a single commit message or a message stored in one Git commit against
Conventional Commit Rules, with configurable limits and severities. [Try the browser demo](https://sgryjp.github.io/commit-msg-lint/) without
installing anything: it checks in-browser without the CLI's comment or scissors filtering. The
message and configuration are not sent to a server.

## Design philosophy

- Measure display width using East Asian character widths, not character counts. This better
  reflects commit-log display, although terminal and font rendering can differ
  ([ADR](docs/adr/0001-display-column-measurement.md)).
- Separate recommended limits from required ones with warnings and errors, giving agents discretion
  to preserve meaning.
- Approximate Git cleanup by default for editor and verbose messages, accepting that the CLI can
  check different content from what Git retains
  ([ADR, in Japanese](docs/adr/0002-default-message-filtering.md)).

## Install and check a message

From a checkout with the Rust toolchain installed, build and install the CLI:

```sh
cargo install --path .
printf 'feat: add parser\n' | commit-msg-lint
commit-msg-lint --edit .git/COMMIT_EDITMSG
commit-msg-lint --commits HEAD
```

For example, `printf 'feat:subject\n' | commit-msg-lint` reports:

```text
<stdin>:1: error[header-format] header does not match the Conventional Commit format
```

See [diagnostics and exit codes](docs/cli.md#diagnostics-and-exit-codes).

## Git hook integrations

The CLI does not install hooks, and not every commit path runs `commit-msg`. These consumer
recipes are separate from this repository's [development hooks](CONTRIBUTING.md#git-hooks).

### pre-commit

Add the hook to `.pre-commit-config.yaml`, using a release tag or commit containing
`.pre-commit-hooks.yaml`. This example uses tag `0.0.2`:

```yaml
repos:
  - repo: https://github.com/sgryjp/commit-msg-lint
    rev: 0.0.2
    hooks:
      - id: commit-msg-lint
```

Install the `commit-msg` stage explicitly; plain `pre-commit install` installs a different stage:

```sh
pre-commit install --hook-type commit-msg
```

The first run builds the CLI and may need network access. See the
[Git hook guide](docs/git-hooks.md) for prerequisites, offline use, strict mode, and
troubleshooting.

### Existing commit-msg hook

In an existing hook, pass Git's message file argument to the installed CLI:

```sh
commit-msg-lint --edit "$1"
```

### Config-based hook

For a lightweight local trial with Git 2.54 or later, first install the CLI and ensure it is on
Git's hook `PATH`. Run once from the repository where you want to check messages:

```sh
git config set --local hook.commit-msg-lint.command 'commit-msg-lint --edit'
git config set --local --append hook.commit-msg-lint.event commit-msg
```

Git supplies the message filename to the configured command. Local configuration is not shared
with other clones. Configured hooks run alongside an existing hook, so a hook that also runs this
linter can cause duplicate checks. See the [Git hook guide](docs/git-hooks.md#config-based-hooks)
for inspection and removal.

## Configure the linter

The built-in defaults apply when no configuration is found; `commit-msg-lint.jsonc` at the Git
repository root overrides selected settings. Use `--config <path>` to select a file explicitly.
From the repository root, generate editable defaults or a schema matching the installed CLI with:

```sh
commit-msg-lint --print-default-config > commit-msg-lint.jsonc
commit-msg-lint --print-config-schema > commit-msg-lint.schema.json
```

Redirection overwrites the destination files. See the
[configuration reference](docs/configuration.md) for discovery and schema compatibility, and the
[Rule defaults](docs/configuration.md#built-in-rules). The
[JSONC decision record](docs/adr/0003-jsonc-configuration.md) explains the format choice.

## Caveats and current limitations

- Warnings can print while the command succeeds; `--strict` makes them fail. See
  [exit codes](docs/cli.md#diagnostics-and-exit-codes).
- CLI comment and scissors filtering is enabled by default and approximates Git cleanup, so the
  checked content can differ from what Git retains. See [filtering](docs/cli.md#message-filtering).
- By default, recognized generated-message Headers bypass all Rules, even with `--strict`; matching
  is based on text, not provenance. See [ignored messages](docs/configuration.md#ignored-messages).
- History or range linting, custom Rules, and hook installation are not currently supported.

## Using from agents

Run `commit-msg-lint --help-for-agents` or read the [agent usage guide](docs/agent-usage.md).

## Reference

- [CLI](docs/cli.md): options, [filtering](docs/cli.md#message-filtering), and
  [diagnostics and exit codes](docs/cli.md#diagnostics-and-exit-codes)
- [Configuration](docs/configuration.md): [defaults](docs/configuration.md#built-in-rules),
  [ignored messages](docs/configuration.md#ignored-messages), and Rule settings
- [Message format](docs/message-format.md): parsing and Rule behavior
- [Agent usage](docs/agent-usage.md) and [Git hooks](docs/git-hooks.md)
- [Glossary](GLOSSARY.md) and [decision records](docs/adr/)

## Development

See [Contributing](CONTRIBUTING.md) for development and validation, and the
[Changelog](CHANGELOG.md) for release history. Distribution channels, platform guarantees, and
library API support remain open maintainer decisions; see [TODO](TODO.md).

## License

Licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.

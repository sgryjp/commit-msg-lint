# commit-msg-lint

Yet another Git commit message linter focusing on:

1. Warnings for recommended limits, errors for hard limits: room for agents to preserve meaning
2. EAW-aware line-width checks (East Asian Width)

See the [design principles](docs/design-principles.md).

It checks commit messages against Conventional Commit by default.
[Try the browser demo](https://sgryjp.github.io/commit-msg-lint/);
it does not send your message or configuration to a server.

## Install the CLI

From a checkout with the Rust toolchain installed, build and install the CLI:

```sh
cargo install --path .
```

## Set up a Git hook

Choose a setup based on your environment:

- If you use pre-commit, [add the pre-commit hook](#pre-commit). It builds the CLI on first run,
  so you can skip the CLI installation above.
- To try the linter locally with Git 2.54 or later, [set up a config-based hook](#config-based-hook).
  This requires the installed CLI.
- If you already have a `commit-msg` hook, [add the linter to it](#existing-commit-msg-hook).
  This also requires the installed CLI.

### pre-commit

Add the hook to `.pre-commit-config.yaml`, using a release tag or commit containing
`.pre-commit-hooks.yaml`. This example uses tag `0.0.3`:

```yaml
repos:
  - repo: https://github.com/sgryjp/commit-msg-lint
    rev: 0.0.3
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

### Existing commit-msg hook

In an existing hook, pass Git's message file argument to the installed CLI:

```sh
commit-msg-lint --edit "$1"
```

## Configure the linter

The built-in defaults apply when no configuration is found; `commit-msg-lint.jsonc` at the Git
repository root overrides selected settings. Use `--config <path>` to select a file explicitly.
From the repository root, generate editable defaults or a schema matching the installed CLI with:

```sh
commit-msg-lint --print-default-config > commit-msg-lint.jsonc
commit-msg-lint --print-config-schema > commit-msg-lint.schema.json
```

Redirection overwrites the destination files. See the
[configuration reference](docs/references/configuration.md) for discovery and schema compatibility, and the
[Rule defaults](docs/references/lint-rules.md#built-in-rules). The
[JSONC decision record](docs/adr/0003-jsonc-configuration.md) explains the format choice.

## Caveats and current limitations

- Lint Rule warnings can print while the command succeeds; `--strict` makes them fail. See
  [exit codes](docs/references/cli.md#diagnostics-and-exit-codes).
- History input accepts one Git revision or explicit two-dot and three-dot ranges, and rejects
  shallow repositories. The [history input guide](docs/references/cli.md#history-input) explains Git selection
  semantics and CI history requirements.
- For standard input and `--edit`, CLI comment and scissors filtering is enabled by default and
  approximates Git cleanup, so the checked content can differ from what Git retains. See
  [filtering](docs/references/cli.md#message-filtering). History input checks Git's display-oriented messages
  without those filters; see the [history input limitations](docs/references/cli.md#history-input).
- By default, recognized generated-message Headers bypass all Rules, even with `--strict`; matching
  is based on text, not provenance. See [ignored messages](docs/references/configuration.md#ignored-messages).
- Custom Rules and hook installation are not currently supported.

## Using from agents

Run `commit-msg-lint --help-for-agents` or read the [agent usage guide](docs/agent-usage.md).

## Reference

- [CLI](docs/references/cli.md): options, [history input](docs/references/cli.md#history-input),
  [filtering](docs/references/cli.md#message-filtering), and
  [diagnostics and exit codes](docs/references/cli.md#diagnostics-and-exit-codes)
- [Configuration](docs/references/configuration.md): discovery, schema, and
  [ignored messages](docs/references/configuration.md#ignored-messages)
- [Lint Rules](docs/references/lint-rules.md): defaults, severity overrides, and width thresholds
- [Message format](docs/references/message-format.md): parsing and Rule behavior
- [Agent usage](docs/agent-usage.md) and [Git hooks](docs/git-hooks.md)
- [Glossary](GLOSSARY.md) and [decision records](docs/adr/)

## Development

See [Contributing](CONTRIBUTING.md) for development and validation, and the
[Changelog](CHANGELOG.md) for release history. Distribution channels, platform guarantees, and
library API support remain open maintainer decisions; see [TODO](TODO.md).

## License

Licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.

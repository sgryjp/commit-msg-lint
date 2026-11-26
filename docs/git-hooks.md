# Git hook integration guide

[English](git-hooks.md) | [日本語](git-hooks.ja.md)

For short installation recipes, see [Git hook integrations](../README.md#git-hook-integrations).
These are consumer integrations; this repository's own development hooks are described in
[Contributing](../CONTRIBUTING.md#git-hooks).

## pre-commit setup and operation

Install [pre-commit](https://pre-commit.com/) and use a revision containing
`.pre-commit-hooks.yaml`. Install the `commit-msg` stage explicitly: a plain `pre-commit install`
installs the `pre-commit` stage instead. The [README recipe](../README.md#pre-commit) shows both
configuration and installation.

On first run, pre-commit creates a Rust environment and builds the CLI. If `cargo --version`
succeeds, it uses the system toolchain; otherwise, it bootstraps Rust through rustup and installs
the stable toolchain by default. Bootstrapping may download rustup and Rust components; Cargo may
also need network access for uncached dependencies. Offline or restricted environments must supply
the toolchain and dependencies or allow these downloads. The Rust setup runs `cargo install --bins`
without `--locked`, so Cargo may resolve dependencies without following the repository's
`Cargo.lock`.

The hook prints diagnostics even when it succeeds. To make warnings fail, override its arguments:

```yaml
      - id: commit-msg-lint
        args: [--strict, --edit]
```

Include `--edit` when overriding `args`: the manifest supplies it there, and pre-commit appends the
message filename. For example, test the configured hook with an existing message file:

```sh
pre-commit run commit-msg-lint --hook-stage commit-msg --commit-msg-filename message.txt
```

If the hook does not run, check that the `commit-msg` stage is installed and that the Git operation
invokes it; `--no-verify` bypasses the hook. If setup fails, check the toolchain and network
requirements above. If the command cannot be found in a manually configured hook, ensure it is
installed and available on the hook's `PATH`.

## Config-based hooks

The [README recipe](../README.md#config-based-hook) registers a local hook. Inspect it with
`git hook list --show-scope commit-msg`. To remove the trial, run the following from the repository,
provided the named section contains no other settings you want to keep:

```sh
git config --local --remove-section hook.commit-msg-lint
```

For strict mode, set its command to `commit-msg-lint --strict --edit` instead. For an existing
hook script, use `commit-msg-lint --strict --edit "$1"`. See the
[Git hook documentation](https://git-scm.com/docs/git-hook) for configuration details.

## Aligning with Git cleanup

The hook receives a message before Git's final cleanup. The CLI's filtering is an approximation,
not a reproduction of Git cleanup. Align the Git commit options and linter settings when content
must be retained. The [CLI filtering reference](cli.md#message-filtering) is authoritative for
controls, prefix lookup, their limits, and an example that disables both filtering operations while
committing a verbatim message. Windows hook behavior and Git for Windows CRLF scissors behavior
have not been verified.

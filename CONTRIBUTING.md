# Contributing

This repository validates a Rust CLI and Markdown documents. For agent-specific workflows, see
[AGENTS.md](AGENTS.md), which is the authoritative guide.

## Required tools

- Stable Rust toolchain, including `cargo`
- Python 3
- cjkfmt 0.0.10
- pre-commit 4.4.0 or later, or a compatible current version of prek

Install cjkfmt with:

```sh
cargo install --locked cjkfmt --version 0.0.10
```

Install either pre-commit or prek as your hook runner. Use the installation method appropriate for
your environment, and make sure you have the latest release.

```sh
python -m pip install --user --upgrade pre-commit
# or
python -m pip install --user --upgrade prek
```

## Development and validation

Run all Rust tests:

```sh
cargo test --locked
```

Run the fixture-discovery check:

```sh
python scripts/check_fixture_discovery.py
```

On Windows, you may need to use the Python launcher:

```powershell
python scripts/check_fixture_discovery.py
```

## Markdown documents

The user-facing references describe the current implementation. When changing CLI behavior,
configuration, or Rules, update the relevant documents and examples alongside the code and tests.
When updating the `unicode-width` crate or Unicode data, update the documented versions and
conformance tests, and describe the change in the release notes.

Tracked `.md` and `.markdown` files must be no more than 100 columns wide and must include a
half-width space between Japanese text and adjacent ASCII letters or digits. Check for violations by
running all files through your hook runner:

```sh
pre-commit run --all-files
# or
prek run --all-files
```

The hooks do not modify files. When editing Markdown, format the target files explicitly and review
the diff before running the checks. If automatic formatting leaves diagnostics, fix them by hand
while preserving the Markdown structure and meaning.

```sh
cjkfmt format --write README.md docs/example.md
cjkfmt check README.md docs/example.md
```

## Git hooks

Enable Markdown checks before commits and commit-message checks:

```sh
pre-commit install --hook-type pre-commit --hook-type commit-msg
# or
prek install --hook-type pre-commit --hook-type commit-msg
```

The `commit-msg` hook runs this repository's `commit-msg-lint` with `cargo run --locked`. A commit
fails if the message has errors. Warnings alone do not cause failure.

## Updating cjkfmt

When updating cjkfmt, update the hook release, the version of the binary installed in CI, and the
version listed in this document in the same change. Run the checks on all Markdown files both before
and after the update.

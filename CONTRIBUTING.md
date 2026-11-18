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
cargo test --locked --workspace
```

Run the fixture-discovery check:

```sh
python scripts/check_fixture_discovery.py
```

On Windows, you may need to use the Python launcher:

```powershell
python scripts/check_fixture_discovery.py
```

### Configuration schema

The generated `site/commit-msg-lint.schema.json` is published on GitHub Pages and tracks `main`.
Run the schema synchronization tests with:

```sh
python -B -m unittest discover -s tests -p test_sync_config_schema.py
```

When configuration behavior changes, regenerate and check it with the pre-commit hooks:

```sh
pre-commit run --all-files
# or
prek run --all-files
```

The schema hook always runs without filenames. If the generated schema differs, the hook updates
the file and fails so that you can review it. It also fails if the file is missing, deleted, or
untracked, even if an untracked file already has the expected content. Review the schema, stage it
with `git add site/commit-msg-lint.schema.json`, and rerun the checks. If generation fails, the
existing file is preserved; fix the reported error and rerun the hook.

## Web demo

To build and preview the WebAssembly demo locally, install the WebAssembly target and `wasm-pack`:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-pack
wasm-pack build web --target web --release --out-dir ../site/pkg
```

Serve the generated files over HTTP rather than opening `site/index.html` directly:

```sh
cd site
python -m http.server 8000
```

Open <http://localhost:8000/> in a browser. The generated `site/pkg/` directory is ignored by Git.

Run the focused Web UI tests with Node.js (no additional packages are needed):

```sh
node --test tests/site-ui.test.cjs
```

These tests use a simulated DOM and WASM API. To also exercise the built WebAssembly module with
that simulated DOM, run:

```sh
wasm-pack build web --target web --dev --out-dir /tmp/commit-msg-lint-web-pkg
WASM_PKG_DIR=/tmp/commit-msg-lint-web-pkg node --test web/tests/site-wasm.test.cjs
```

Preview the built demo in a browser to check the actual browser integration.

The [Pages workflow](.github/workflows/pages.yml) builds and deploys the demo on pushes to `main`
and can also be run manually. The repository's GitHub Pages source must be set to GitHub Actions.

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

The Markdown hook does not modify files; the configuration schema hook may update its generated
file as described above. When editing Markdown, format the target files explicitly and review the
diff before running the checks. If automatic formatting leaves diagnostics, fix them by hand while
preserving the Markdown structure and meaning.

```sh
cjkfmt format --write README.md docs/example.md
cjkfmt check README.md docs/example.md
```

## Git hooks

Enable the pre-commit checks, including schema synchronization, and commit-message checks:

```sh
pre-commit install --hook-type pre-commit --hook-type commit-msg
# or
prek install --hook-type pre-commit --hook-type commit-msg
```

The `commit-msg` hook runs this repository's `commit-msg-lint` with `cargo run --locked`. A commit
fails if the message has errors. Warnings alone do not cause failure.

## Releases

1. Prepare a release change:
   - Update the CLI version in `Cargo.toml` and the non-published Web package version in
     `web/Cargo.toml`. Run `cargo check --workspace` to update `Cargo.lock` and review the diff.
   - Move the relevant `CHANGELOG.md` entries from Unreleased to a dated release section. Review
     the changes since the previous release for user-visible additions, fixes, and behavior changes,
     and update the comparison links. Keep an Unreleased section for future changes.
2. Run the checks described above, including `cargo test --locked --workspace` and
   `pre-commit run --all-files` (or `prek run --all-files`). Confirm that
   `cargo run --locked -- --version` prints the new CLI version.
3. Merge the release change into `main` and wait for the Test workflow to pass on the exact commit
   to be released. The release workflow does not wait for tests.
4. Create an annotated tag without a `v` prefix, matching the version, and push it. Replace the
   version and commit below with the release version and verified commit:

   ```sh
   git tag -a 0.0.2 <verified-commit> -m "Release 0.0.2"
   git push origin 0.0.2
   ```

5. Check that the [release workflow](.github/workflows/release.yml) succeeded and that the GitHub
   Release points to the intended tag. The workflow generates release notes from GitHub, not from
   `CHANGELOG.md`; review the notes against the changelog and edit them if needed. Verify that the
   pre-commit hook can be installed using the new tag.

The workflow runs on tag pushes and does not publish packages or attach build artifacts. Tags pushed
before the workflow is available do not trigger it retroactively. The Web demo and configuration
schema on GitHub Pages track `main`, not the release tag.

## Updating cjkfmt

When updating cjkfmt, update the hook release, the version of the binary installed in CI, and the
version listed in this document in the same change. Run the checks on all Markdown files both before
and after the update.

# Contributing

[日本語](CONTRIBUTING.ja.md)

This repository validates a Rust CLI and Markdown documents. For agent-specific workflows, see
[AGENTS.md](AGENTS.md), which is the authoritative guide.

## Required tools

- Stable Rust toolchain, including `cargo`
- Python 3
- pre-commit 4.4.0 or later, or a compatible current version of prek

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

Write user-facing documentation for developers familiar with Git and the command line. Avoid
explaining basic concepts unnecessarily, but explain this tool's behavior, limitations, and
important differences from Git. Use clear, concise language without omitting important conditions.

The user-facing references describe the current implementation. When changing CLI behavior,
configuration, or Rules, update the relevant documents and examples alongside the code and tests.
When updating the `unicode-width` crate or Unicode data, update the documented versions and
conformance tests, and describe the change in the release notes.

The design documents under `docs/design/` describe the current implementation. When changing the
implementation they cover, update them in the same change as the code.

### Translations

Where a document has English and Japanese versions, the English version is authoritative. Japanese
translations use the `.ja.md` suffix, and the pairs reviewed for synchronization are listed in
`docs/translation-sync.json`. Existing Japanese-only design and research documents are not included
in the paired translation review. `docs/agents/` is outside translation maintenance; its existing
Japanese copies are not maintained.

Contributors may update either language without updating the other. The translations can therefore
lag behind. Maintainers periodically review and reconcile the paired documents; see the manual
review procedure below.

### Markdown lint

Tracked Markdown files are checked with rumdl using `.rumdl.toml`. The line-length rule uses a
limit of 120 display columns, with exceptions for some long lines. The CJK spacing rule checks for
spaces between CJK text and adjacent ASCII letters or digits. It also checks the outside of links
and inline code, even when their text is Japanese; use `<!-- rumdl-disable-line MD089 -->` for such
cases rather than adding unnecessary spaces. Run all checks through your hook runner:

```sh
pre-commit run --all-files
# or
prek run --all-files
```

The Markdown hook does not modify files; the configuration schema hook may update its generated
file as described above. To check or fix specific Markdown files directly, install rumdl separately:

```sh
cargo install --locked rumdl --version 0.3.5
rumdl check README.md docs/cli.md
rumdl check --fix README.md docs/cli.md
```

The hook installs its own copy, so a separate installation is optional. Review the diff after fixing
and address any remaining diagnostics by hand, preserving the Markdown structure and meaning.

### Manual translation review

The maintainer can review translations at any time; this is not a CI or contribution requirement.
The review needs [uv](https://docs.astral.sh/uv/getting-started/installation/), Python 3.11 or later,
Git, a complete non-shallow history, and a clean working tree without untracked files. `uv run`
installs the script's pinned Pydantic AI dependency and can obtain a suitable Python. Ignored files
are allowed. The configured baseline for each pair is a previously reviewed commit; the initial
baseline predates the first translations so that the initial additions are included.

When updating translations, keep commands, identifiers, and actual diagnostic output unchanged;
explanatory comments may be translated. Prefer links to the corresponding Japanese document from
a Japanese translation when available.

1. Select an ID from `docs/translation-sync.json` and preview the evidence:

   ```sh
   uv run scripts/check_translation_sync.py review --pair readme --print-prompt
   ```

2. Choose a provider and model explicitly, set its credentials, and save the SHA printed in
   `Reviewing through`. For example, set `OPENAI_API_KEY`, replace `YOUR_MODEL` with an available
   model name, and run:

   ```sh
   uv run scripts/check_translation_sync.py review --pair readme --model openai:YOUR_MODEL
   ```

   `--model anthropic:<model>` (with `ANTHROPIC_API_KEY`) and `--model google:<model>`
   (with `GOOGLE_API_KEY`) are also supported. `--model github-copilot:<model>` uses Pydantic AI's
   [Copilot integration](https://ai.pydantic.dev/models/github-copilot/). It accepts
   `GITHUB_COPILOT_API_KEY`, `GITHUB_COPILOT_API_TOKEN`, or `COPILOT_GITHUB_TOKEN`; `GH_TOKEN` is
   not read automatically. Check token and model eligibility, provider terms, organizational policy,
   and charges before using it. There is no automatic cross-provider fallback. Other
   [Pydantic AI providers](https://ai.pydantic.dev/models/overview/)
   can be selected by their `provider:model` name; they may require an extra dependency, for example
   `uv run --with 'pydantic-ai-slim[EXTRA]==2.76.0' scripts/check_translation_sync.py ...`.

   The command sends both versions and their diffs, commit messages, the changed-path list, and
   diffs and full current contents of changed non-Markdown tracked text files to the selected
   provider. It omits the contents and diffs of other Markdown documents but lists their paths.
   Check that sharing this evidence is appropriate first, including provider data policies. No
   function tools are registered, so the model cannot inspect unchanged source files or omitted
   documents. A structured output tool returns the report; it is not a shell or repository tool.
   Omitted binary or other files are identified in the prompt. Large histories can exceed the
   model's context. The per-pair timeout defaults to 180 seconds; change it with `--timeout`.
   A timeout does not guarantee that remote generation or charges stop. Each pair is limited to
   one Pydantic AI model request, but provider SDKs or transports may retry it. Repeat `--pair` to
   select more pairs, or use `--all` to review all pairs; each pair resends its evidence.
3. Check the report and relevant current implementation yourself, make and commit any corrections,
   and only then record the reviewed SHA for the pairs you actually reviewed:

   ```sh
   uv run scripts/check_translation_sync.py acknowledge --pair readme --reviewed-through YOUR_REVIEWED_SHA
   ```

   Replace `YOUR_REVIEWED_SHA` with the captured full SHA. Repeat `--pair` to acknowledge several
   pairs at the same SHA. Review and commit the resulting
   `docs/translation-sync.json` change. Commits after that SHA remain in the next review, including
   any corrections made after the review began. A moved or deleted document needs its mapping
   updated; missing history or a non-ancestor checkpoint is reported as an error.

Model findings are suggestions, not proof of synchronization. Untrusted repository content can
mislead the model, and even a valid report with no findings can miss discrepancies. No checkpoint
is advanced automatically. Errors, invalid, truncated or explicitly incomplete reports are not
successful reviews; do not acknowledge an incomplete review. Run the offline tests with:

```sh
uv run --python 3.11 --with-requirements scripts/check_translation_sync.py \
  python -B -m unittest discover -s tests -p test_check_translation_sync.py
python -B -m unittest discover -s tests -p test_translation_sync_scope.py
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

## Updating rumdl

When updating rumdl, update the hook release and the version in this document together. Run the
checks on all Markdown files before and after the update.

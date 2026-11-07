# CLI fixture cases

Each directory directly under `tests/cases/` is one independently named CLI
integration test. The test runner discovers these directories at build time.
Each case must contain:

- `message.txt` — exact bytes sent to standard input. The same bytes are sent
to standard input when the case uses `--edit`; the CLI must then read the
message from the path named in `args`.
- `case.toml` — strict case metadata with required `args` (an array of strings)
and `exit_code` (an integer from 0 through 255). Unknown and missing fields
are errors.
- `stdout.txt` and `stderr.txt` — expected output bytes, including final
newlines where applicable. Empty output is represented by an empty file.

Example `case.toml`:

```toml
args = ["--config", "config.toml", "--edit", "message.txt"]
exit_code = 0
```

Any other files in the case directory are copied into an isolated temporary
working directory, so configuration paths and `--edit` paths should be
relative. The runner executes `CARGO_BIN_EXE_commit-msg-lint` from a temporary
directory outside the repository, with an isolated home and Git environment.
Git remains available so the CLI's default repository-configuration discovery
is exercised without inheriting a parent repository or user configuration. The
runner caps discovery with `GIT_CEILING_DIRECTORIES`, passing the parent path
without quoting. Git splits this list on `;` on Windows and `:` on Unix. If the
parent path contains that separator, the runner uses an isolated nonexistent
`GIT_DIR` so Git reports that no repository was found. It never forces
`--config`.

The tests compare exit status, stdout, and stderr byte-for-byte; they do not
normalize line endings or whitespace. `.gitattributes` checks out fixture
`.txt` and `.toml` files with LF line endings on every platform. To create a
fixture that intentionally contains actual CRLF bytes, add a more specific
`-text` rule for each affected file in the repository `.gitattributes`, after
the general fixture rules. For example:

```gitattributes
tests/cases/crlf-input/message.txt -text
tests/cases/crlf-input/stderr.txt -text
```

Do not use `-text` for ordinary text fixtures: Windows checkout settings could
otherwise change their bytes and invalidate exact comparisons. Cases must be
flat directories of regular files, with no nested directories or symlinks.

To add a case, create a directory with a unique descriptive name, add the four
required files, and add any optional configuration/input files beside them.
The build script watches the case root and each case directory so additions,
removals, and file updates regenerate the test list. Run all cases with:

```sh
cargo test --test fixtures
```

Run one case by its generated test name (the case directory name is converted
to an identifier by replacing punctuation with underscores):

```sh
cargo test --test fixtures -- case_success
```

Use `cargo test --test fixtures -- --list` to inspect the discovered test names.
A malformed fixture fails explicitly with its case name; it is never silently
skipped.

To verify that additions and removals trigger Cargo's incremental test
regeneration, run this standalone check (Python 3 and Cargo are required):

```sh
python3 scripts/check_fixture_discovery.py
```

On Windows, use `python scripts/check_fixture_discovery.py` instead. The check
creates a uniquely named valid case, asks Cargo to list and run that individual
generated test, removes only the case it created in a `finally` cleanup, and
asks Cargo to confirm the test is gone. Run it outside another Cargo test
process; it invokes Cargo itself. CI runs this check on Linux and Windows after
`cargo test --locked`.

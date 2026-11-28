# CLI reference

```text
commit-msg-lint [OPTIONS]
```

The CLI checks commit messages from standard input, a file, one Git commit, or an explicit Git
commit range against configured Rules, or prints information with a standalone option. It does not
modify messages or install Git hooks.

## Options

| Option | Description |
| --- | --- |
| `--edit <path>` | Read the message from a file instead of standard input. |
| `--commits <revision-or-range>` | Lint one commit or commits selected by an explicit Git range. Conflicts with `--edit`. |
| `--config <path>` | Use the specified configuration file instead of discovering one. |
| `--strict` | Treat Lint Rule warnings as failures. |
| `--print-default-config` | Print built-in defaults as editable JSON to stdout and exit. |
| `--print-config-schema` | Print the configuration JSON Schema to stdout and exit. |
| `--help-for-agents` | Print [agent guidance](../agent-usage.md) to stdout and exit successfully. |
| `-h`, `--help` | Display command help and exit successfully. |
| `-V`, `--version` | Print the program name and version to standard output and exit successfully. |

Options can appear in any order and each may be used once. `--edit` and `--commits` are
mutually exclusive. `--commits` ignores standard input and can be combined with `--config` and
`--strict`. `--help-for-agents`, `--print-default-config`, and `--print-config-schema` must each be
used alone; combining any of them with another argument, including `--help` or `--version`, is an
argument error with exit code `2`. These standalone options do not read standard input, invoke Git,
discover or load configuration, or read the filtering environment variables. `--help-for-agents`
prints the Markdown bundled at build time. `--print-default-config` prints the built-in defaults,
not the current configuration. `--print-config-schema` prints a schema matching this CLI, which may
differ from the latest public schema. See
[editor schema and compatibility](configuration.md#editor-schema-and-compatibility). Relative
paths passed to `--edit` and `--config` are resolved from the current working directory.

## Examples

Read from standard input:

```sh
printf 'feat: add parser\n' | commit-msg-lint
```

Read a message file, optionally with an explicit configuration or strict mode:

```sh
commit-msg-lint --edit .git/COMMIT_EDITMSG
commit-msg-lint --config config/lint.jsonc --strict --edit message.txt
```

Lint only the commit named by a Git revision; its ancestors are not included:

```sh
commit-msg-lint --commits HEAD
commit-msg-lint --config config/lint.jsonc --strict --commits v1.2.3
```

For an existing Git `commit-msg` hook, pass the hook's first argument:

```sh
commit-msg-lint --edit "$1"
```

To lint a stored commit or range, see [History input](#history-input). The CLI does not install
the hook, and not every Git commit path invokes it. Git passes the hook its message before final
cleanup. The same hook command works with the CLI's default [message filtering](#message-filtering).
For setup options and troubleshooting, see the [Git hook integration guide](../git-hooks.md).

## Input

For linting, without `--edit` or `--commits`, the CLI reads standard input until EOF.
With `--edit`, it reads the entire file and ignores standard input. With
`--commits <revision-or-range>`, the CLI reads one commit or the commits selected by an explicit
two-dot or three-dot range. A single revision resolves to exactly one commit and does not
walk its ancestors. Branches, tags, and full object IDs are accepted when they resolve to commits.
Invalid, unavailable, and non-commit revisions or range endpoints are operational errors. A
revision endpoint starting with `-` is rejected to prevent option injection. `--commits` conflicts
with `--edit` and does not read standard input.

Standard input and file messages must be valid UTF-8; the CLI does not convert them based on Git's
`i18n.commitEncoding` setting. For history input, the CLI asks Git to output messages in UTF-8,
using each commit's encoding header when Git can convert it. If Git's output is not valid UTF-8,
the CLI reports an operational error identifying the commit. Git may truncate a history message
at an embedded NUL; see [History input](#history-input). CRLF is normalized to LF for linting, and
one final newline is excluded from the message content. Filtering runs before normalization for
stdin and file input: in particular, a scissors line ending in CRLF does not match, as described
below.
With the built-in defaults, an empty message fails the `header-format` Rule.
By default, recognized Git-generated and hosting-service Header forms silently bypass all Rules.
See [ignored messages](configuration.md#ignored-messages) for matching boundaries, configuration,
and the consequences for width and severity checks. A matching Header does not prove provenance.
Some Git operations, including revert and cherry-pick without editing, may not invoke `commit-msg`;
inspect the resulting message separately if it must be checked.

## History input

Single-commit and explicit range checks are available.

Use `--commits` to check a stored Git commit or a range. The argument must be one revision that
resolves to a commit, or a range with two explicit endpoints separated by `..` or `...`. Revisions
can be commit IDs or refs such as branch and tag names. A single revision selects exactly that
commit, not its ancestors. An argument that resolves as a single commit is treated as one revision,
even if it contains `..`. Otherwise, both endpoints must resolve to commits. If more than one split
into valid endpoints is possible, the range is rejected; use unambiguous revisions or commit IDs.
A two-dot range `A..B` selects commits
reachable from `B` but not `A`; a three-dot range `A...B` selects the symmetric difference: commits
reachable from either endpoint but not both. For example:

```sh
commit-msg-lint --commits HEAD
commit-msg-lint --commits v1.2.0..HEAD
commit-msg-lint --commits "$BASE_SHA...$HEAD_SHA"
```

These are Git's revision-selection semantics. The selected commits are reported in reversed
topological order, so parents precede descendants; Git determines the order of unrelated commits.
Within each commit, diagnostics retain the usual line and Rule ID ordering. Every history
diagnostic identifies the complete commit object ID and the one-based line in Git's output message;
the object ID length depends on the repository's hash format. A successful run with no diagnostics
prints nothing. Lint diagnostics and operational errors go to standard error.

History linting uses the same discovered configuration as other CLI input, or the file selected by
`--config`. The configured Lint Rules, severities, Display Column measurement, and Ignored Message
policy all apply. `--strict` promotes Lint Rule warnings to failure, but a message ignored by the
configured policy remains excluded from every Rule, including in strict mode. See the
[configuration reference](configuration.md), including [Ignored Messages](configuration.md#ignored-messages).

Unlike standard input and `--edit`, history input checks the message Git outputs for display without
comment-line removal or scissors truncation. The filtering environment variables are not read, and
Git's comment prefix is not resolved; even invalid values for those variables do not affect history
checks. The CLI requests UTF-8 output from Git, overriding `i18n.logOutputEncoding`. Git uses a
commit's encoding header when converting the message, but conversion can fail or leave invalid bytes
unchanged. Output that is not valid UTF-8 is an operational error identifying the affected commit,
not a lint diagnostic; the CLI does not replace invalid bytes. Valid messages use the linter's
existing unfiltered message and line handling; see [message format](message-format.md#input-and-lines).

History input is not a byte-exact audit of stored commit objects. Git's message output stops at an
embedded NUL: the NUL and any content after it are not checked or diagnosed. See the
[decision record](../adr/0004-git-history-input.md) for the trade-offs.

An invalid, unavailable, or non-commit revision is an operational error, not an empty selection. A
valid range that selects no commits in a non-shallow repository succeeds without lint diagnostics.
The CLI rejects history input in a shallow repository with an operational error, including a single
commit or an empty range. It does not fetch, deepen, or unshallow the repository. History checks do
not print summary counts.

For CI, obtain a non-shallow checkout and the history needed for the selected range before running
the linter. For example, GitHub Actions can fetch full history with `actions/checkout`'s
`fetch-depth: 0`. The linter will not fetch it automatically. A shallow clone can resolve both
endpoints and still omit commits; a non-shallow repository alone does not guarantee that every
required object is available.

For a push, `"$BEFORE_SHA..$AFTER_SHA"` checks commits reachable from the new tip but not the old
tip, when both revisions are available. For a pull request, `"$BASE_SHA..$HEAD_SHA"` checks commits
reachable from the head but not the base. Choose revisions that match the event and commit set you
intend to inspect: a three-dot range is Git's symmetric difference, not a guarantee of the exact
commit list shown by every hosting service. History linting checks the messages Git outputs for the
selected commits; it does not check a future squash-merge message. A squash merge creates a new commit with its own
message, which must be checked separately after it exists.

## Message filtering

For standard input and `--edit`, the CLI excludes comment lines and truncates at scissors by
default. It does not rewrite the input file. History input checks Git's message output without these
filters and does not read or validate the filtering environment variables or resolve a Git comment
prefix. The public `lint(input, config)` library API and Web demo also check the original input
without this filtering. See the [design](../design/commit-message-parsing-and-linting.md) and
[decision record](../adr/0002-default-message-filtering.md) for the rationale and full constraints.

| Environment variable | Effect | Default |
| --- | --- | --- |
| `COMMIT_MSG_LINT_STRIP_COMMENTS` | Exclude lines starting with the comment prefix. | Enabled |
| `COMMIT_MSG_LINT_CUT_SCISSORS` | Exclude scissors and subsequent lines. | Enabled |

Each variable is independent: an unspecified variable stays enabled. Accepted true values are `1`,
`true`, `yes`, and `on`; false values are `0`, `false`, `no`, and `off`. Letter case is ignored.
Empty values, leading or trailing whitespace, other values, and non-Unicode values are
configuration errors with exit code `2`. These controls are not JSONC settings or CLI options.

When either operation is enabled, the CLI uses Git in the current repository context to read
`core.commentChar` and `core.commentString` in configuration order, using the last value. Git
resolves includes, worktrees, and inherited Git environment variables. If neither key is set, or
the current directory is outside a repository, the prefix is `#`. A case-insensitive `auto`,
an empty value, a value containing a newline, malformed Git output, or a required Git launch or
configuration lookup failure results in exit code `2`. Use a fixed prefix in Git or disable
both operations in that case. Values are not trimmed or inferred from the `--edit` path. When
both operations are disabled, Git is not called for prefix resolution, although configuration
discovery can still call Git unless `--config` is specified.

The inherited `GIT_CONFIG` variable can make `git config` read a different file from the one
`git commit` uses, so the prefixes may differ. Unrecognized or multiline Git trace output during
repository discovery can also cause a configuration error rather than the outside-repository
fallback. In either case, adjust the Git environment or disable both filtering operations.

Scissors truncation runs first, followed by comment removal. A comment line must start with the
prefix; for prefix `#`, `#note` matches but ` #note` does not. Multi-character prefixes are
supported. Git versions before 2.45 handle multi-character prefixes and aliases differently; full
compatibility with them is not guaranteed. A scissors line must match the following entire line
and end in LF in the original input:

```text
<prefix> ------------------------ >8 ------------------------
```

Indented lines, extra characters, CRLF endings, and a final line without LF do not match scissors.
If comment removal is enabled, a non-matching scissors-like line may still be removed as a comment,
without truncating what follows. Ordinary CRLF lines are normalized for linting. Windows-specific
behavior, including Git for Windows behavior with CRLF scissors lines, has not been verified; the
LF-only condition is fixed by this design, not a claim about Git for Windows.

The filtering is a deliberate approximation, not a reproduction of Git cleanup. In particular,
Git's default cleanup for `-m` and `-F` retains comments, whereas the CLI default excludes
them. A matching scissors line also truncates lint input even when Git would retain it. Conversely,
Git may clean up whitespace that the linter still checks. Callers must align their Git options and
lint policy when content is intended to be retained.

Use both variables to check a completed message as supplied:

```sh
COMMIT_MSG_LINT_STRIP_COMMENTS=false \
COMMIT_MSG_LINT_CUT_SCISSORS=false \
commit-msg-lint --edit message.txt
```

For an agent committing a completed message verbatim through the existing hook, combine the
controls with Git's cleanup option (POSIX shell):

```sh
COMMIT_MSG_LINT_STRIP_COMMENTS=false \
COMMIT_MSG_LINT_CUT_SCISSORS=false \
git -c commit.verbose=false commit --cleanup=verbatim -F message.txt
```

The variables affect only the linter; `--cleanup=verbatim` and disabling Git's verbose setting are
separate Git choices.

## Diagnostics and exit codes

Lint diagnostics are written to standard error in this format:

```text
<source>:<line>: <severity>[<rule-id>] <message>
```

`<source>` is `<stdin>`, the path passed to `--edit` (with non-UTF-8 path characters replaced),
or the complete commit object ID for history input. Line numbers start at 1 and refer to the
original message in standard-input or file mode, even when lines are filtered out, or to Git's
output message in history mode. Diagnostics for each message are sorted by line number and then Rule
ID. For example,
`printf 'feat:subject\n' | commit-msg-lint` with the built-in defaults produces:

```text
<stdin>:1: error[header-format] header does not match the Conventional Commit format
```

A successful lint run with no diagnostics prints nothing. Warnings are still printed to standard
error when they do not cause failure, so exit code `0` does not mean there were
no warnings.
`--strict` promotes Lint Rule warnings to exit code `1` only when explicitly supplied; the CLI
does not force strict mode for agents. A shallow repository is an operational error for history
input regardless of strict mode. If a Git, read, or decoding error occurs after diagnostics have
been emitted, those diagnostics remain visible and the command returns `2`. Lint diagnostics and
command errors are written to standard error.

| Exit code | Meaning |
| --- | --- |
| `0` | No lint errors; warnings are allowed unless strict. Informational options and valid empty ranges in non-shallow repositories succeed. |
| `1` | At least one error diagnostic, or any Lint Rule warning with `--strict`. |
| `2` | Argument, configuration, shallow repository, Git, temporary file, message reading, or UTF-8 decoding error. |

## Configuration discovery

For linting, including history input, `--config` selects a file explicitly and bypasses discovery.
Otherwise, the CLI runs Git to find the root of the repository containing the current working
directory and looks only for `commit-msg-lint.jsonc` there. If there is no repository or no root
configuration file, ordinary message input uses the built-in defaults; history input still requires
a Git repository. An explicit file is parsed as JSONC regardless of its extension. Git must be
runnable for discovery; other discovery failures, unreadable files, and invalid configuration are
errors rather than reasons to use defaults. Configuration is loaded before the message is read.

See the [configuration reference](configuration.md) for the format and settings, and the
[Lint Rules reference](lint-rules.md) for Rule defaults and overrides.

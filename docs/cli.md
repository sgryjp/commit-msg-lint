# CLI reference

```text
commit-msg-lint [OPTIONS]
```

The CLI checks one commit message against its configured Rules. It does not modify the message or
install Git hooks.

## Options

| Option | Description |
| --- | --- |
| `--edit <path>` | Read the message from a file instead of standard input. |
| `--config <path>` | Use the specified configuration file instead of discovering one. |
| `--strict` | Treat warnings as failures for the exit code, without changing their severity. |
| `-h`, `--help` | Display command help and exit successfully. |
| `-V`, `--version` | Print the program name and version to standard output and exit successfully. |

Options can appear in any order and each may be used once. Relative paths passed to `--edit` and
`--config` are resolved from the current working directory.

## Examples

Read from standard input:

```sh
printf 'feat: add parser\n' | commit-msg-lint
```

Read a message file, optionally with an explicit configuration or strict mode:

```sh
commit-msg-lint --edit .git/COMMIT_EDITMSG
commit-msg-lint --config config/lint.toml --strict --edit message.txt
```

For an existing Git `commit-msg` hook, pass the hook's first argument:

```sh
commit-msg-lint --edit "$1"
```

The CLI does not install the hook, and not every Git commit path invokes it. Git passes the hook
its message before final cleanup. The same hook command works with the CLI's default
[message filtering](#message-filtering).

## Input

Without `--edit`, the CLI reads standard input until EOF. With `--edit`, it reads the entire file
and ignores standard input. Input must be valid UTF-8. Ordinary LF and CRLF line endings are treated
alike; one final newline is excluded from the message content. Comment and scissors filtering runs
before this normalization, as described below. With the built-in defaults, an empty message fails
the `header-format` Rule. Special Headers such as Git-generated merge or revert messages have no
exemption from the configured Rules.

## Message filtering

For both standard input and `--edit`, the CLI excludes comment lines and truncates at scissors by
default. It does not rewrite the input file. The public `lint(input, config)` library API and Web
demo check the original input without this filtering. See the
[design](design/commit-message-parsing-and-linting.md) and
[decision record](adr/0002-default-message-filtering.md) for the rationale and full constraints.

| Environment variable | Effect | Default |
| --- | --- | --- |
| `COMMIT_MSG_LINT_STRIP_COMMENTS` | Exclude lines starting with the comment prefix. | Enabled |
| `COMMIT_MSG_LINT_CUT_SCISSORS` | Exclude scissors and subsequent lines. | Enabled |

Each variable is independent: an unspecified variable stays enabled. Accepted true values are `1`,
`true`, `yes`, and `on`; false values are `0`, `false`, `no`, and `off`. Letter case is ignored.
Empty values, leading or trailing whitespace, other values, and non-Unicode values are
configuration errors with exit code `2`. These controls are not TOML settings or CLI options.

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

`<source>` is `<stdin>` or the path passed to `--edit`, with non-UTF-8 path characters replaced.
Line numbers start at 1 and refer to the original input, even when lines are filtered out.
Diagnostics are sorted by line number and then Rule ID. For example,
`printf 'feat:subject\n' | commit-msg-lint` with the built-in defaults produces:

```text
<stdin>:1: error[header-format] header does not match the Conventional Commit format
```

A successful lint run with no diagnostics prints nothing. Warnings are still printed when they do
not cause failure. Lint diagnostics and command errors are written to standard error.

| Exit code | Meaning |
| --- | --- |
| `0` | No errors, and no warnings in strict mode. Help and version also exit with `0`. |
| `1` | At least one error diagnostic, or any warning with `--strict`. |
| `2` | Argument, configuration, Git, file reading, or UTF-8 decoding error. |

## Configuration discovery

`--config` selects a file explicitly and bypasses discovery. Otherwise, the CLI runs Git to find
the root of the repository containing the current working directory and looks only for
`commit-msg-lint.toml` there. If there is no repository or no root configuration file, it uses the
built-in defaults. Git must be runnable for discovery; other discovery failures, unreadable files,
and invalid configuration are errors rather than reasons to use defaults. Configuration is loaded
before the message is read.

See the [configuration reference](configuration.md) for the format, defaults, and Rule settings.

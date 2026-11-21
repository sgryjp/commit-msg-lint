# Using `commit-msg-lint` from an agent

Use `commit-msg-lint` to check one complete commit message or commits selected by an explicit Git
revision range. See the [CLI history input guide](cli.md#history-input) for the supported syntax and
Git selection semantics. Run `commit-msg-lint --help-for-agents` to print this guidance. This option
prints the bundled Markdown without reading standard input, invoking Git, discovering or loading
configuration, or reading message-filter environment variables. Do not combine it with other
arguments.

## Provide the message

By default, the linter reads the complete message from standard input until EOF:

```sh
printf '%s\n' 'feat: add parser' | commit-msg-lint
```

Use `--edit <path>` to read a complete message from a file instead. In that mode, standard input is
ignored:

```sh
commit-msg-lint --edit message.txt
```

The CLI does not modify the message. Diagnostics go to standard error and include the source and
original line number. Exit code `0` means there are no lint errors; warnings can still be printed to
standard error with exit code `0`. Exit code `1` means a lint error was found, or a Lint Rule warning
was promoted to failure by `--strict`. A shallow-range warning alone does not fail the command.
Exit code `2` means an argument, configuration, Git, file-read, or input-decoding error occurred.
`--strict` is opt-in; the linter does not infer that warnings should fail based on the caller.

## Check a completed message as supplied

The CLI's default filtering is intended for messages still being edited: comment lines and a
scissors section are excluded. The two environment variables below control those operations
independently, and each defaults to enabled:

- `COMMIT_MSG_LINT_STRIP_COMMENTS`
- `COMMIT_MSG_LINT_CUT_SCISSORS`

Set both to a false value to inspect the complete input without filtering. For example, this POSIX
shell command sends the completed message directly through standard input:

```sh
printf '%s\n' 'feat: add parser' '' '# retained text' | \
  COMMIT_MSG_LINT_STRIP_COMMENTS=false \
  COMMIT_MSG_LINT_CUT_SCISSORS=false \
  commit-msg-lint
```

The variables affect only the linter's input selection. They do not configure Git's cleanup. Git's
cleanup mode does not automatically change either variable, and setting the variables does not
change Git's cleanup. In particular, Git's default cleanup for `-m` and `-F` can retain comments
that the linter excludes by default.

## Existing `commit-msg` hooks

If an existing hook invokes `commit-msg-lint`, environment variables set on the `git commit` command
are inherited by Git and then by the hook and linter, unless the hook explicitly removes or replaces
them. This lets a caller align both sides when committing a completed message file verbatim:

```sh
COMMIT_MSG_LINT_STRIP_COMMENTS=false \
COMMIT_MSG_LINT_CUT_SCISSORS=false \
git -c commit.verbose=false commit --cleanup=verbatim -F message.txt
```

This POSIX shell example disables filtering in the linter and separately asks Git to preserve the
message verbatim and not add a verbose diff. Neither choice implies the other. The hook still has to
be installed and invoked by the Git commit path; `commit-msg-lint` does not install or manage hooks.

See the [CLI reference](cli.md) for all options, filtering details, and configuration behavior.

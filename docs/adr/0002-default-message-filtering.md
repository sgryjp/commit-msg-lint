# Enable Comment Filtering and Scissors Truncation by Default in the CLI

[日本語](0002-default-message-filtering.ja.md)

The CLI enables comment filtering and scissors truncation by default for both standard input and
`--edit`. Per-commit changes are specified through two independent environment variables. Neither
rewriting shared configuration nor a dedicated hook mode is required. The public
`lint(input, config)` and the web demo continue to lint the original text.
**Status: implemented, under verification.** Windows-specific behavior and actual hook tests with
Git for Windows have not yet been verified.

A `commit-msg` hook cannot determine the effective cleanup mode or whether verbose was specified.
In particular, disabling scissors truncation by default could cause even the diff from
`git commit -v` to be linted. The owner prioritized reducing surprising false rejections during
normal commits, including verbose commits. In exchange, if an exact scissors line appears in the
body under conditions where Git would not truncate, violations after it may be missed. Intentionally
including an exact scissors line in the body was judged rare, and the same defaults apply to
standard input and `--edit`. However, comment lines that Git retains by default with `-m` / `-F`
are also excluded from linting. Exact reproduction of Git's final result is not guaranteed; callers
are responsible for aligning Git's options and the linting policy.

A hook-only mode that keeps original-text linting as the default was also considered, but the
decision favors requiring no extra option for hooks, the central use case. When linting a completed
message with the CLI, disable both operations with environment variables. The variables control only
what is linted; they do not change Git's behavior or the input file.

The existing policies of measuring display width and separating warnings from errors to leave room
for agent judgment remain in place. See the
[preprocessing and linting design](../design/commit-message-parsing-and-linting.md) for details
of the specification and constraints.

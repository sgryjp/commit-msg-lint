# Enable comment stripping and scissors truncation by default in the CLI

The CLI enables comment stripping and scissors truncation by default for both standard input and
`--edit` input. Two independent environment variables allow per-commit overrides. This does not
require changing shared configuration or using a dedicated hook mode. The public
`lint(input, config)` API and the web demo continue to lint the original input.
**Status: Implemented; under validation.** Windows-specific behavior and actual hook testing with
Git for Windows have not been verified.

The `commit-msg` hook cannot determine the effective cleanup mode or whether verbose mode was
specified. In particular, if scissors truncation is disabled by default, the diff from
`git commit -v` may also be linted. The owner prioritized reducing surprising false rejections
for normal commits, including verbose commits. The trade-off is that if the body contains an exact
scissors line under conditions where Git does not truncate it, violations after that line may go
undetected. Since intentionally including an exact scissors line in the body is considered rare,
the same defaults apply to standard input and `--edit`. However, with `-m` / `-F`, comment lines
that Git retains are also excluded from linting by default. The CLI does not guarantee an exact
reproduction of Git's final result; callers are responsible for aligning Git's settings with the
linting policy.

A dedicated hook mode that preserves linting of the original input by default was considered, but
the decision was not to require extra configuration for the primary hook use case. When using the
CLI to lint an already completed message, disable both operations through their environment
variables. These variables do not change Git's behavior or the input file; they only control what
is linted.

The existing policies of measuring display width and separating warnings from errors to preserve
room for agent judgment remain unchanged. See
[Preprocessing and Linting Design](../design/commit-message-parsing-and-linting.md) <!-- rumdl-disable-line MD089 -->
for details on the specification and constraints.

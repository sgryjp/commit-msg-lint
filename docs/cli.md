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

## Input

Without `--edit`, the CLI reads standard input until EOF. With `--edit`, it reads the entire file
and ignores standard input. Input must be valid UTF-8. LF and CRLF line endings are treated alike;
one final newline is excluded from the message content. Other content is checked as supplied,
including Git comment or scissors lines. With the built-in defaults, an empty message fails the
`header-format` Rule.

## Diagnostics and exit codes

Lint diagnostics are written to standard error in this format:

```text
<source>:<line>: <severity>[<rule-id>] <message>
```

`<source>` is `<stdin>` or the path passed to `--edit`, with non-UTF-8 path characters replaced.
Line numbers start at 1. Diagnostics are sorted by line number and then Rule ID. For example,
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
| `2` | Argument, configuration, Git discovery, file reading, or UTF-8 decoding error. |

## Configuration discovery

`--config` selects a file explicitly and bypasses discovery. Otherwise, the CLI runs Git to find
the root of the repository containing the current working directory and looks only for
`commit-msg-lint.toml` there. If there is no repository or no root configuration file, it uses the
built-in defaults. Git must be runnable for discovery; other discovery failures, unreadable files,
and invalid configuration are errors rather than reasons to use defaults. Configuration is loaded
before the message is read.

See the [configuration reference](configuration.md) for the format, defaults, and Rule settings.

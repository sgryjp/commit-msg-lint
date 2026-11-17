# TODO

These items capture limitations and questions for evaluation, not commitments to implement every
feature. Group them by area; prioritize individual items when they are triaged.

## Display width

- [ ] Evaluate how to communicate differences between calculated display columns and actual
  rendering, including terminal, font, emoji, and combining-character behavior.

## Message compatibility

- [ ] Verify the
  [message-filtering implementation](docs/design/commit-message-parsing-and-linting.md) on Windows,
  including actual Git for Windows hook and CRLF scissors behavior.
- [ ] Evaluate an explicit comment-prefix override through an environment variable or CLI option,
  including stable agent-driven commits and filtering in environments without Git, such as WASM.
  Keep a built-in default prefix (`#`) available independently of Git configuration lookup.

## Integration and extensibility

- [ ] After addressing Git message cleanup, document hook setup for private trials with Git 2.54+
  config-based hooks and for shared use with pre-commit and prek. Evaluate providing a pre-commit
  hook manifest.
- [ ] Evaluate support for custom Rules or plugins.
- [ ] Evaluate direct linting of Git history and commit ranges.
- [ ] Evaluate hook installation and management.

## Agent workflows

- [ ] Evaluate a machine-readable diagnostic format such as JSON.
- [ ] Evaluate whether to provide repair suggestions or automatic fixes, preserving agent discretion
  over warnings and the meaning of messages.
- [ ] Document or evaluate an agent workflow that reads warnings even when the exit code is zero.

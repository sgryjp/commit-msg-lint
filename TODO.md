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

- [ ] Decide which installation and distribution channels to support beyond building from a
  checkout and the existing pre-commit hook.
- [ ] Decide what platform guarantees to offer, including Windows hook behavior.
- [ ] Decide whether the Rust library API is a supported, stable embedding interface.
- [ ] Evaluate support for custom Rules or plugins.
- [ ] Evaluate hook installation and management.

## Agent workflows

- [ ] Evaluate a machine-readable diagnostic format such as JSON.
- [ ] Evaluate whether to provide repair suggestions or automatic fixes, preserving agent discretion
  over warnings and the meaning of messages.
- [ ] Document or evaluate an agent workflow that reads warnings even when the exit code is zero.

# TODO

These items capture limitations and questions for evaluation, not commitments to implement every
feature. Group them by area; prioritize individual items when they are triaged.

## Display width

- [ ] Evaluate how to communicate differences between calculated display columns and actual
  rendering, including terminal, font, emoji, and combining-character behavior.

## Message compatibility

- [ ] Evaluate the independent Header and Footer parsing rules against Conventional Commits and
  Git trailer behavior, and identify compatibility gaps worth addressing.
- [ ] Evaluate handling Git comment and scissors lines in `commit-msg` hooks; currently they are
  checked as input rather than removed.

## Integration and extensibility

- [ ] Evaluate support for custom Rules or plugins.
- [ ] Evaluate direct linting of Git history and commit ranges.
- [ ] Evaluate hook installation and management.

## Agent workflows

- [ ] Evaluate a machine-readable diagnostic format such as JSON.
- [ ] Evaluate whether to provide repair suggestions or automatic fixes, preserving agent discretion
  over warnings and the meaning of messages.
- [ ] Document or evaluate an agent workflow that reads warnings even when the exit code is zero.

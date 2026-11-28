# Lint Rules reference

Lint Rules check commit messages against the built-in policy.
Use the `rules` object in the [configuration](configuration.md) to override individual Rules.
For message structure and detailed Rule behavior, see the [message format reference](message-format.md).

## Rule settings

Use an object named for the Rule ID inside `rules` to override a built-in Rule:

```jsonc
{
  "version": 1,
  "rules": {
    "type-case": { "enabled": true, "severity": "warning" },
  },
}
```

- `enabled` is a boolean. A disabled Rule produces no diagnostics.
- `severity` is either `"warning"` or `"error"`. Warnings normally leave the exit code at `0`;
  errors result in exit code `1`. With `--strict`, warnings also result in exit code `1` without
  changing their reported severity.
- `thresholds` is supported only by `header-max-width`, `body-max-width`, and
  `footer-max-width`; see [width thresholds](#width-thresholds).

Unspecified Rule settings retain their defaults. Rule configuration is validated even for a
disabled Rule. Header component Rules (`type-enum`, `type-case`, `scope-required`, `scope-enum`,
`subject-empty`, and `subject-full-stop`) run only when the Header can be parsed.

## Width thresholds

A width Rule's `thresholds` array replaces its default thresholds. Each entry must have an integer
`limit` and a `severity`:

```jsonc
{
  "version": 1,
  "rules": {
    "body-max-width": {
      "thresholds": [
        { "limit": 60, "severity": "warning" },
        { "limit": 80, "severity": "error" },
      ],
    },
  },
}
```

Limits are non-negative and must be in strictly increasing order. The array must not be empty.
Severities must stay the same or increase from `warning` to `error` as limits increase. A line
violates a threshold only when its width is greater than its limit. When multiple thresholds are
exceeded, the Rule reports one diagnostic using the severity of the highest exceeded threshold.

`header-max-width` must use `thresholds` to change severity; it does not accept a Rule-level
`severity`. `body-max-width` and `footer-max-width` also accept a Rule-level `severity`, which
changes the severity of their default thresholds. A Rule object cannot specify both `severity` and
`thresholds`. Other Rules do not accept `thresholds`.

## Built-in Rules

All Rules are enabled with `error` severity by default, except as noted below.

| Rule ID | Default | What it checks |
| --- | --- | --- |
| `header-format` | error | Header syntax: Type, optional Scope and `!`, and the `: ` separator. |
| `type-enum` | error | Type is in the allowed `types` list, ignoring ASCII case. |
| `type-case` | error | Type contains no uppercase ASCII letters. |
| `scope-required` | disabled | A Scope is present. |
| `scope-enum` | disabled | A present Scope matches an entry in `allowed-scopes`. |
| `subject-empty` | error | Subject is not empty. |
| `header-trim` | error | Header has no leading or trailing whitespace. |
| `subject-full-stop` | error | Subject does not end in `.`. |
| `header-max-width` | warning above 50; error above 72 | Header width in Display Columns. |
| `section-separator` | error | Exactly one empty line separates Header, Body, and Footer. |
| `body-max-width` | error above 72 | Width of each Body line in Display Columns. |
| `footer-max-width` | error above 72 | Width of each Footer line in Display Columns. |
| `trailer-format` | error | A Footer candidate starts with a valid Trailer. |
| `line-trailing-whitespace` | error | Body and Footer lines have no trailing whitespace. |
| `control-character` | error | Lines contain no forbidden control or bidirectional characters. |

For CLI options and diagnostics, see the [CLI reference](cli.md).

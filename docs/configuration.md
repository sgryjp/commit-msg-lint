# Configuration reference

`commit-msg-lint` uses JSONC to override its built-in configuration. Unspecified settings retain
their built-in defaults; a configuration file does not replace the entire profile. The CLI and Web
demo use the same configuration format; an empty configuration in the Web demo uses the defaults.

## Generating a configuration file

At the Git repository root, generate an editable configuration from the built-in defaults:

```sh
commit-msg-lint --print-default-config > commit-msg-lint.jsonc
```

Shell redirection overwrites an existing `commit-msg-lint.jsonc`. The output is editable,
pretty-printed JSON without comments or trailing commas. Customize the generated file as needed.

## Finding a configuration file

Use `--config <path>` to select a file explicitly. The path is relative to the current working
directory unless it is absolute. An explicit path is used instead of automatic discovery and must
refer to a readable UTF-8 file. Its contents are parsed as JSONC regardless of the file extension.

Without `--config`, the CLI uses Git to find the root of the repository containing the current
working directory and looks only for `commit-msg-lint.jsonc` at that root. Files in subdirectories
are not searched. If the file is absent or the current directory is outside a Git repository, the
built-in defaults are used. Other discovery failures, including failure to run Git, are errors.

JSONC accepts JSON with `//` line comments, `/* ... */` block comments, and trailing commas in
objects and arrays. JSON5 extensions such as unquoted keys and single-quoted strings are not
supported. Unknown or duplicate object keys, including Rule IDs, and explicit `null` values are
rejected; omit optional fields to retain their defaults instead. Invalid JSONC, an unreadable file,
and invalid values or combinations are configuration errors. They do not fall back to defaults.
Configuration errors exit with code `2`.

The [CLI's comment and scissors filtering controls](cli.md#message-filtering) are independent
per-invocation environment variables, not settings in the shared JSONC configuration. Explicit
`--config` bypasses configuration discovery, not necessarily the separate Git lookup for the
comment prefix.

## Example

```jsonc
{
  "version": 1,
  "conventional": {
    "types": ["feat", "fix", "docs", "refactor", "test"],
    "scope-required": true,
    "allowed-scopes": ["cli", "parser", "docs"],
  },
  "display-width": {
    "ambiguous-width": 2,
  },
  "rules": {
    // Warn at 51 columns and error at 73 columns.
    "header-max-width": {
      "thresholds": [
        { "limit": 50, "severity": "warning" },
        { "limit": 72, "severity": "error" },
      ],
    },
    "subject-full-stop": {
      "enabled": false,
    },
  },
}
```

The only required key is the top-level integer `version`, which must be `1`. All other objects and
keys are optional. The minimal configuration file is:

```json
{ "version": 1 }
```

## Conventional Commit settings

The optional `conventional` object contains:

- `types`: An array of strings that replaces the default allowed Type list. The default is
  `["build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style", "test"]`.
  The `type-enum` Rule compares Types using ASCII case-insensitive matching. The separate
  `type-case` Rule enforces lowercase. An empty array is invalid while `type-enum` is enabled.
- `scope-required`: A boolean, defaulting to `false`. When `true`, it enables the
  `scope-required` Rule.
- `allowed-scopes`: An array of strings. By default it is unspecified and Scopes are unrestricted.
  Setting it enables the `scope-enum` Rule, which compares Scopes exactly and case-sensitively.
  An empty array permits no Scope. `"scope-required": true` cannot be combined with an empty
  `allowed-scopes` array, even if a Rule override disables either Rule.

Rule-level `enabled` settings are applied after these settings, so they can override the Rule
states implied by `scope-required` and `allowed-scopes`. Enabling `scope-enum` explicitly requires
`allowed-scopes` to be specified, including when the array is empty.

## Display width settings

The optional `display-width` object has one key:

- `ambiguous-width`: Either `1` (the default) or `2`. This selects the normal or CJK width
  calculation, respectively, for characters with ambiguous display width.

Width Rules measure Display Columns rather than bytes or characters. The implementation uses
`unicode-width` 0.2.2 with Unicode 17.0.0 data.

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
  `footer-max-width`; see below.

Unspecified Rule settings retain their defaults. Rule configuration is validated even for a
disabled Rule. Header component Rules (`type-enum`, `type-case`, `scope-required`, `scope-enum`,
`subject-empty`, and `subject-full-stop`) run only when the Header can be parsed.

### Width thresholds

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

### Built-in Rules

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

For the message structure and detailed Rule behavior, see the
[message format reference](message-format.md). For CLI options and diagnostics, see the
[CLI reference](cli.md).

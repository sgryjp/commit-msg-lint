# Message format reference

`commit-msg-lint` checks a UTF-8 commit message as a Header followed by optional Body and Footer
lines. This reference describes the current implementation, including its Footer heuristic; it is
not a general-purpose Conventional Commits or Git trailer parser. See the
[configuration reference](configuration.md) for all settings, default Types, Rule defaults, and
severity overrides.

For example, with the built-in defaults:

```text
feat(parser)!: support multiple inputs

Explain the change and its motivation.

BREAKING CHANGE: callers must provide a list
Refs #123
```

## Input and lines

The CLI reads UTF-8 from standard input or the file specified by `--edit`. Invalid UTF-8 is an
input error. For linting, every CRLF pair is converted to LF, and at most one final LF is removed.
The remaining text is split on LF; the first line is always the Header, even for empty input. A
standalone CR is not normalized and is checked as a forbidden control character. No other trimming,
comment removal, Git cleanup, or Unicode normalization is performed, and the input file is not
rewritten.

An **empty line** in the section rules has zero characters. A line containing spaces or other
whitespace is not empty. Line numbers in diagnostics are one-based after CRLF normalization.

## Header

A parseable Header has this shape:

```text
<type>[(<scope>)][!]: <subject>
```

The brackets denote optional parts, not literal characters. The parser applies these constraints:

- Type starts with an ASCII letter; subsequent characters may be ASCII letters, digits, or `-`.
- If present, Scope is nonempty and contains no Unicode whitespace or `(`. The first `)` closes
  it. Scope can contain Unicode and other non-whitespace characters.
- An optional `!` follows the Type or closing `)`. The parser does not otherwise check its meaning
  or require a `BREAKING CHANGE` Trailer.
- The separator is exactly a colon followed by one ASCII space. The Subject may be empty, but it
  cannot start with another ASCII space. Other characters, including non-ASCII whitespace, are
  not rejected by the Header parser itself.

For a successfully parsed Header, the component Rules check that the Type is in the configured
list using ASCII case-insensitive comparison (`type-enum`), that it contains no ASCII uppercase
letters (`type-case`), that Scope is present if required (`scope-required`), and that a present
Scope matches an allowed Scope exactly and case-sensitively (`scope-enum`). They also check that
Subject is not empty (`subject-empty`) and does not end in a literal ASCII period
(`subject-full-stop`). The Scope Rules are disabled by default; see the configuration reference
for how to enable them.

`header-trim` checks the whole Header for leading or trailing Unicode whitespace, and
`header-max-width` measures the whole Header. These checks and `control-character` run even when
the Header cannot be parsed. If parsing fails, `header-format` is reported when enabled, and none
of the component Rules run, even if `header-format` is disabled. Other lines are still checked.

For example, `FIX:bad.` fails `header-format`; it does not also report `type-case` or
`subject-full-stop`. `feat: ` parses with an empty Subject, so it reports both `subject-empty` and
`header-trim` under the defaults. `feat:  change` fails `header-format` because of the second
ASCII space.

## Sections and separators

After the Header, the linter finds the first nonempty line. If one exists, it must be line 3:
exactly one empty line must separate it from the Header. Otherwise, `section-separator` is
reported on that first nonempty line. If there is no nonempty line after the Header, no section
separator is required or checked, even if multiple empty lines follow the Header.

The first nonempty line and everything after it are classified as Body or Footer using the
heuristic below. If a Footer starts after the first nonempty line, the consecutive empty lines
immediately before its start must number exactly one. Otherwise, `section-separator` is reported
on the first Footer line. If the Footer starts at the first nonempty line, there is no separate
Body-to-Footer separator check. Empty lines at the end of the message are not required or limited
by these section checks.

## Footer and Trailer detection

The linter scans from the first nonempty line after the Header and chooses the first line that is
either:

1. A valid Trailer, anywhere in the scan, including immediately after a Body line; or
2. A Trailer candidate that is the first nonempty line after the Header or immediately follows an
   empty line.

That line starts the Footer; every later line belongs to the Footer. Lines before it are Body. If
no line qualifies, all content after the Header is Body. This classification is independent of
whether `trailer-format`, `section-separator`, or the width Rules are enabled.

To recognize a Trailer, the linter first identifies a token and the text immediately after it:

- If the line starts with the exact, case-sensitive text `BREAKING CHANGE`, that entire prefix is
  the token, even if more token-like characters immediately follow it.
- Otherwise, the token is the nonempty initial run of characters before the first Unicode
  whitespace character, `:`, or `#`. It can contain Unicode and punctuation; it is not restricted
  to ASCII letters or hyphens.

A **valid Trailer** has either `: ` (colon and one ASCII space) or ` #` (one ASCII space and a
hash) immediately after the token, followed by a value that is not empty after trimming Unicode
whitespace. The value may contain additional spaces or other characters. Examples include
`Refs: issue 123`, `Refs #123`, `レビュー: 完了`, and `BREAKING CHANGE: reason`.

A **Trailer candidate** has a nonempty token followed immediately by `:` or `#`, or by one or
more ASCII spaces and then `:` or `#`. It need not have a valid separator or value. For example,
`Refs:#123`, `Refs#123`, `Refs : value`, and `Refs: ` are candidates but not valid Trailers.
Whitespace other than ASCII space between the token and delimiter does not make a candidate.
The special `BREAKING CHANGE` prefix is case-sensitive: for example,
`BREAKING CHANGELOG: value` is not recognized as a Trailer, because the text after that prefix
starts with `LOG`, not a delimiter.

If the first Footer line is an invalid candidate, `trailer-format` is reported on that line.
Later lines are not required to be valid Trailers: they can be continuation text, empty lines, or
malformed Trailer-like lines, even before a valid Trailer appears. They still receive the
applicable width, trailing-whitespace, and control-character checks. In particular:

```text
feat: change

Body text
Refs:#123
```

Here `Refs:#123` remains Body because it is an invalid candidate immediately after a nonempty
Body line. Replacing it with `Refs: value` starts a Footer there and reports a missing
Body-to-Footer separator. Inserting an empty line before `Refs:#123` instead starts a Footer and
reports `trailer-format`. Once a Footer starts, even a later `Refs:#123` is not independently
validated as a Trailer.

## Whitespace and controls

`line-trailing-whitespace` checks every line after the Header for trailing Unicode whitespace,
including whitespace-only lines. The Header uses `header-trim` instead. A line may produce both a
whitespace diagnostic and a control-character diagnostic.

`control-character` checks every line for C0 controls `U+0000`–`U+001F`, DEL and C1 controls
`U+007F`–`U+009F`, bidirectional formatting controls `U+202A`–`U+202E`, and bidirectional isolate
controls `U+2066`–`U+2069`. Tabs and standalone CRs are included. LF line
separators and CRs consumed as part of CRLF are not present in the lines being checked. The Rule
reports at most once per line, even if that line contains multiple forbidden characters.

## Display width and diagnostics

Width Rules measure each whole line in Display Columns using the `unicode-width` 0.2.2 string
algorithm with Unicode 17.0.0 data, not bytes, Unicode scalar values, or grapheme counts. For
example, `あ` is two columns. Ambiguous-width characters use one column by default; configuration
can select the CJK calculation instead. Actual terminal rendering can differ.

By default, Header width above 50 columns is a warning and above 72 is an error; Body and Footer
lines above 72 columns are errors. A width equal to a limit is allowed. Each width Rule reports at
most one diagnostic per line, using the severity of the highest exceeded threshold. Footer width
applies to every line from the detected Footer start onward, not only valid Trailers; Body width
applies to preceding Body lines. See the configuration reference for threshold overrides and the
other Rule defaults.

Diagnostics are sorted by line number and then Rule ID. By default, enabled non-width Rules report
errors. Warnings do not fail the CLI unless `--strict` is used; errors fail it regardless. See the
configuration reference for enabling, disabling, and changing the severity of Rules.
